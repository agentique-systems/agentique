//! The control interface (C-53, ROADMAP §4.16): agents operate the real,
//! visible Studio without computer vision.
//!
//! - **Observation** ([`observe`]): the Studio's identity and revision, what
//!   is on screen (screen, view, panels, dialog, palette, selection, status,
//!   problems, the conversation, tasks, builds), every drawn control with its
//!   id, role, label, value, state and bounds (`ui::target`), the cards in
//!   view, and the commands with whether they are available and why not.
//! - **Actions** ([`Action`]): commands through the Studio's own command
//!   dispatch; click, fill, key, type and scroll as input to the window at a
//!   control's bounds, so hit testing, focus, bindings and rendering run as
//!   for the Operator; selecting an element; opening a project; waiting for a
//!   condition. An action names the identity and the screen it was observed
//!   against; one for another instance, project, build or session, a screen
//!   that changed since (its screen, dialog, palette, project, settings or
//!   selection), or a control that is gone or disabled is refused as stale,
//!   and nothing happens.
//! - **The Operator's own** (§3 roles): an agent cannot answer a dialog that
//!   asks for the Operator's approval (integrating a task, changing locked
//!   elements, trusted-local execution, a paid live run, starting an
//!   implementation) or the Operator's own question, act in Settings or in
//!   the Conversation (the Operator's voice and answers), lock or unlock,
//!   undo, change appearance, or pause and resume agents. This is enforced
//!   where each effect happens ([`Studio::refused_to_agents`]), whatever
//!   route reached it (a click, keys, the palette, focus and Space), while a
//!   step of an agent's action is carried out; requests are also refused up
//!   front with the reason where that can be told. A change an agent's
//!   action makes is recorded as the Assistant's, with the agent named.
//! - Every action and its effect go to the **event trace**; the control an
//!   agent acts on is marked on screen with who did what, and the title bar
//!   shows the latest action. **Pause** holds actions (observing is never
//!   held), **Step** lets one through, **Resume** goes on.
//! - Requests come from Agentique's own tools (the Assistant in this
//!   Studio) and from the local endpoint ([`server`]), which the Orchestrator
//!   uses to drive a test instance (the endpoint's holder supervises that
//!   instance: its `gate` operation is the supervisor's, not an agent's
//!   action). They are carried out on the UI thread,
//!   one action at a time, in the workspace's tick; waits run beside them.
//!   A request not carried out before its deadline is dropped.

pub mod input;
pub mod server;

use crate::commands::{COMMANDS, CommandId, unavailable};
use crate::studio::Studio;
use crate::ui::target::{self, Drawn};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

/// How many events the trace keeps.
const TRACE: usize = 500;
/// How long a mark stays on the control an agent acted on.
const MARK: Duration = Duration::from_millis(1400);
/// How long the title bar shows the latest action.
const ACTIVITY: Duration = Duration::from_secs(6);
/// Controls and cards listed in one observation, at most.
const LISTED: usize = 400;
/// How long one of the Assistant's requests may wait (while agents are
/// paused, say) before it is dropped.
pub const TOOL_WAIT: Duration = Duration::from_secs(15 * 60);

/// Where a request's answer goes.
pub enum Reply {
    /// The local endpoint: the answer as JSON.
    Channel(Sender<Value>),
    /// One of Agentique's tools: the answer as the tool's result.
    Tool(Sender<agq_assistant::ToolResult>),
}

impl Reply {
    fn send(self, answer: Value) {
        match self {
            Reply::Channel(sender) => {
                let _ = sender.send(answer);
            }
            Reply::Tool(sender) => {
                let ok = answer["ok"] != false;
                let text = fitted(answer, agq_assistant::tools::RESULT_LIMIT);
                let _ = sender.send(if ok {
                    agq_assistant::ToolResult::answer(text)
                } else {
                    agq_assistant::ToolResult::error(text)
                });
            }
        }
    }
}

/// `answer` as compact JSON within `limit` characters: an observation too
/// long loses its last controls (and says how many), not its end.
pub fn fitted(mut answer: Value, limit: usize) -> String {
    let mut text = answer.to_string();
    let mut omitted = 0;
    let mut cards = 0;
    while text.chars().count() > limit {
        if let Some(controls) = answer["controls"].as_array_mut().filter(|c| !c.is_empty()) {
            let drop = (controls.len() / 4).max(1);
            controls.truncate(controls.len() - drop);
            omitted += drop;
            answer["controlsOmitted"] = json!(omitted);
        } else if let Some(list) = answer["cards"].as_array_mut().filter(|c| !c.is_empty()) {
            let drop = (list.len() / 4).max(1);
            list.truncate(list.len() - drop);
            cards += drop;
            answer["cardsOmitted"] = json!(cards);
        } else {
            return json!({ "ok": answer["ok"], "error": "the answer is too long to send; observe with less detail or a region" }).to_string();
        }
        text = answer.to_string();
    }
    text
}

/// One request: what was asked, by whom, where the answer goes, and until
/// when it may be carried out (after that, nobody waits for the answer).
pub struct Request {
    pub body: Value,
    pub reply: Reply,
    pub deadline: Instant,
}

impl Request {
    pub fn new(body: Value, reply: Reply, within: Duration) -> Request {
        Request {
            body,
            reply,
            deadline: Instant::now() + within,
        }
    }
}

/// The pause gate for agents' actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Run,
    Pause,
    Step,
}

/// A mark on a control an agent acted on.
#[derive(Clone, Debug)]
pub struct Mark {
    pub bounds: gpui::Bounds<gpui::Pixels>,
    pub label: String,
    pub until: Instant,
}

/// One event of the trace.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Event {
    pub seq: u64,
    /// Milliseconds since this Studio started.
    pub at: u64,
    pub agent: String,
    pub what: String,
    pub ok: bool,
    pub detail: Value,
}

/// An action an agent asked for.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Command(CommandId),
    Click(String),
    Fill(String, String),
    Key(Vec<String>),
    Type(String),
    Scroll(String, f32),
    Select(String),
    OpenProject(std::path::PathBuf),
    Wait(Condition, Duration),
}

/// What a wait waits for: every field given must hold.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Condition {
    /// The dialog's kind, or `Some(None)` for no dialog.
    pub dialog: Option<Option<String>>,
    pub screen: Option<String>,
    /// A control is on screen (and enabled, when `enabled`).
    pub control: Option<String>,
    pub enabled: bool,
    pub status_contains: Option<String>,
    /// The Assistant and every job are idle.
    pub idle: bool,
}

/// The control interface's state in the Studio.
pub struct ControlState {
    /// This Studio process, for stale requests: random at start.
    pub instance: String,
    started: Instant,
    /// Changes when the screen's shape changes (screen, dialog, palette,
    /// project, settings): an action observed before that is stale.
    pub screen_revision: u64,
    shape: String,
    sender: Sender<Request>,
    requests: Receiver<Request>,
    waiting: VecDeque<Request>,
    active: Option<Active>,
    /// Waits in progress (they hold up no other action).
    waits: Vec<Waiting>,
    /// The agent whose action's step is being carried out (until the
    /// effects that step dispatched have run): a change it makes is the
    /// Assistant's, naming it, and the Operator's own effects are refused.
    pub acting: Option<String>,
    /// What was refused during the action in progress.
    refused: Option<String>,
    /// Panel fields an agent typed into, by control id, and who: the
    /// field's commit, whenever it happens (Enter in the agent's step, or
    /// the focus leaving later), is that agent's change.
    typed: std::collections::BTreeMap<String, String>,
    /// This build's commit, read once.
    commit: Option<Option<String>>,
    pub gate: Gate,
    trace: VecDeque<Event>,
    next_seq: u64,
    pub marks: Vec<Mark>,
    /// The latest action, for the title bar.
    pub activity: Option<(String, Instant)>,
    pub endpoint: Option<server::Endpoint>,
}

impl Default for ControlState {
    fn default() -> Self {
        let (sender, requests) = std::sync::mpsc::channel();
        ControlState {
            instance: server::random_hex(8),
            started: Instant::now(),
            screen_revision: 1,
            shape: String::new(),
            sender,
            requests,
            waiting: VecDeque::new(),
            active: None,
            waits: Vec::new(),
            acting: None,
            refused: None,
            typed: Default::default(),
            commit: None,
            gate: Gate::Run,
            trace: VecDeque::new(),
            next_seq: 1,
            marks: Vec::new(),
            activity: None,
            endpoint: None,
        }
    }
}

impl ControlState {
    /// Remembers that `agent` typed into the panel field `id` (a dialog's
    /// fields are read when its confirm is pressed).
    fn mark_typed(&mut self, id: &str, region: &str, agent: &str) {
        if !matches!(region, "dialog" | "palette") {
            self.typed.insert(id.to_string(), agent.to_string());
        }
    }

    /// A panel field commits: if an agent typed into it and no step is
    /// running, the commit is carried out as that agent's (returns whether
    /// it was). Call [`ControlState::end_typed_commit`] after.
    pub fn begin_typed_commit(&mut self, field: Option<&str>) -> bool {
        let Some(agent) = field.and_then(|f| self.typed.remove(f)) else {
            return false;
        };
        if self.acting.is_some() {
            return false;
        }
        self.acting = Some(agent);
        true
    }

    pub fn end_typed_commit(&mut self, began: bool) {
        if began {
            self.acting = None;
        }
    }

    /// The field was loaded with other text: what an agent typed there is
    /// gone, and its next commit is whoever's types next.
    pub fn forget_typed(&mut self, field: &str) {
        self.typed.remove(field);
    }

    /// Another project: no field holds what an agent typed.
    pub fn forget_all_typed(&mut self) {
        self.typed.clear();
    }

    /// Removes the endpoint's file (the Studio is closing): nobody connects
    /// to a Studio that is gone.
    pub fn close_endpoint(&mut self) {
        if let Some(endpoint) = self.endpoint.take() {
            let _ = std::fs::remove_file(&endpoint.file);
        }
    }

    /// Where requests are sent (the local endpoint's threads).
    pub fn sender(&self) -> Sender<Request> {
        self.sender.clone()
    }

    /// A request from inside the Studio (one of Agentique's tools).
    pub fn submit(&mut self, request: Request) {
        self.waiting.push_back(request);
    }

    /// Drops the actions `agent` asked for that have not started, and its
    /// waits (fail closed when its turn is stopped).
    pub fn cancel(&mut self, agent: &str, why: &str) {
        let mut kept = VecDeque::new();
        for request in self.waiting.drain(..) {
            if request.body["op"] == "act" && request.body["agent"] == agent {
                request.reply.send(json!({ "ok": false, "error": why }));
            } else {
                kept.push_back(request);
            }
        }
        self.waiting = kept;
        let (cancelled, waits): (Vec<Waiting>, Vec<Waiting>) =
            self.waits.drain(..).partition(|w| w.agent == agent);
        self.waits = waits;
        for wait in cancelled {
            wait.request
                .reply
                .send(json!({ "ok": false, "error": why }));
        }
    }

    /// Actions waiting, an action in progress, or a recent one.
    pub fn busy(&self) -> bool {
        self.active.is_some()
            || self.waiting.iter().any(|r| r.body["op"] == "act")
            || self
                .activity
                .as_ref()
                .is_some_and(|(_, at)| at.elapsed() < ACTIVITY)
    }

    /// Actions held at the gate.
    pub fn held(&self) -> usize {
        self.waiting
            .iter()
            .filter(|r| r.body["op"] == "act")
            .count()
    }

    /// The latest action, while it is recent.
    pub fn activity(&self) -> Option<&str> {
        self.activity
            .as_ref()
            .filter(|(_, at)| at.elapsed() < ACTIVITY)
            .map(|(text, _)| text.as_str())
    }

    fn record(&mut self, agent: &str, what: &str, ok: bool, detail: Value) {
        let event = Event {
            seq: self.next_seq,
            at: self.started.elapsed().as_millis() as u64,
            agent: agent.to_string(),
            what: what.to_string(),
            ok,
            detail,
        };
        self.next_seq += 1;
        self.trace.push_back(event);
        while self.trace.len() > TRACE {
            self.trace.pop_front();
        }
    }

    /// Events after `since`.
    pub fn events(&self, since: u64) -> Vec<Event> {
        self.trace
            .iter()
            .filter(|e| e.seq > since)
            .cloned()
            .collect()
    }

    /// Marks still to show.
    pub fn live_marks(&mut self) -> &[Mark] {
        let now = Instant::now();
        self.marks.retain(|m| m.until > now);
        &self.marks
    }
}

/// What the Studio runs as: its version, and the build it runs from.
pub fn identity(studio: &Studio) -> Value {
    let build = studio.running_build();
    let commit = studio.control.commit.clone().flatten();
    json!({
        "instance": studio.control.instance,
        "version": env!("CARGO_PKG_VERSION"),
        "build": build,
        "commit": commit,
        "project": studio.project.as_ref().map(|p| p.folder().display().to_string()),
        "session": studio.session_path.display().to_string(),
        "endpoint": studio.control.endpoint.as_ref().map(|e| e.port),
    })
}

/// A command's id in requests: its name in kebab case (`CreatePart` is
/// `create-part`).
pub fn command_name(id: CommandId) -> String {
    let debug = format!("{id:?}");
    let mut name = String::new();
    for (i, c) in debug.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            name.push('-');
        }
        name.extend(c.to_lowercase());
    }
    name
}

pub fn command_by_name(name: &str) -> Option<CommandId> {
    COMMANDS
        .iter()
        .map(|c| c.id)
        .find(|id| command_name(*id) == name)
}

/// The screen's shape: what an action's place and effect depend on (a
/// command acts on the selection).
fn shape(studio: &Studio) -> String {
    format!(
        "{}|{}|{}|{:?}|{}|{:?}",
        screen(studio),
        dialog_kind(studio).unwrap_or_default(),
        studio.palette.is_some(),
        studio.project.as_ref().map(|p| p.folder().to_path_buf()),
        studio.settings_section_name(),
        studio.selection.elements(&studio.scene),
    )
}

/// The open dialog, when it asks for the Operator's own approval.
pub fn approval(studio: &Studio) -> Option<&'static str> {
    use crate::edit::Dialog;
    match studio.dialog.as_ref()? {
        Dialog::Confirm { locked, .. } if !locked.is_empty() => Some("lock confirmation"),
        Dialog::Confirm { change, .. } if change.actor == agq_system_state::Actor::Operator => {
            Some("the Operator's own question")
        }
        Dialog::ReviewTask { .. } => Some("task review"),
        Dialog::TrustLocal => Some("trusted-local execution"),
        Dialog::ConfirmLive => Some("live run"),
        Dialog::Implement { .. } => Some("implementation"),
        _ => None,
    }
}

/// Commands that are the Operator's: locking, trust, the Operator's own
/// voice in the Conversation, undoing (the Operator's changes too), and the
/// appearance (Settings).
pub const OPERATORS_COMMANDS: [CommandId; 10] = [
    CommandId::Lock,
    CommandId::TrustLocal,
    CommandId::AskAssistant,
    CommandId::InsertSelection,
    CommandId::NewConversation,
    CommandId::Undo,
    CommandId::Redo,
    CommandId::Theme,
    CommandId::Contrast,
    CommandId::ReducedMotion,
];

/// Regions whose controls are the Operator's: Settings (keys, autonomy,
/// trust, builds) and the Conversation (messages, answers to the Assistant).
const OPERATORS_REGIONS: [&str; 2] = ["settings", "conversation"];

/// The agents chip: pausing and resuming agents is the Operator's.
const OPERATORS_CONTROLS: [&str; 3] = ["agents-pause", "agents-step", "agents-resume"];

/// The command a key is bound to, if any, comparing keystrokes as GPUI
/// reads them (so `secondary-l` and `l->l` are the keys they press).
fn bound(key: &str) -> Option<CommandId> {
    let pressed = gpui::Keystroke::parse(key).ok()?;
    let same = |binding: &str| {
        gpui::Keystroke::parse(binding)
            .is_ok_and(|b| b.modifiers == pressed.modifiers && b.key == pressed.key)
    };
    COMMANDS
        .iter()
        .find(|c| c.key.is_some_and(same))
        .map(|c| c.id)
        .or_else(|| {
            crate::commands::ALIASES
                .iter()
                .find(|(k, _)| same(k))
                .map(|(_, id)| *id)
        })
}

/// Why `agent` may not do `action`: it is the Operator's (§3 roles).
fn operators_only(studio: &Studio, action: &Action, drawn: &[Drawn]) -> Option<String> {
    if !matches!(action, Action::Wait(..) | Action::Scroll(..))
        && let Some(kind) = approval(studio)
    {
        return Some(format!(
            "the {kind} dialog asks for the Operator's own approval; an agent cannot answer it (wait until the Operator has)"
        ));
    }
    let focused = drawn.iter().rev().find(|d| d.control.focused);
    let operators_region = |d: &Drawn| {
        OPERATORS_REGIONS.contains(&d.region).then(|| {
            format!(
                "the {} is the Operator's; an agent does not act there",
                if d.region == "settings" {
                    "Settings screen"
                } else {
                    "Conversation"
                }
            )
        })
    };
    match action {
        Action::Command(id) if OPERATORS_COMMANDS.contains(id) => {
            Some(format!("`{}` is the Operator's to use", command_name(*id)))
        }
        Action::Click(name) | Action::Fill(name, _) | Action::Scroll(name, _) => {
            let d = control_named(drawn, name)?;
            if OPERATORS_CONTROLS.contains(&d.control.id.as_ref()) {
                return Some("pausing and resuming agents is the Operator's".into());
            }
            // The system's folder picker is a window of its own that no
            // observation shows and no action reaches.
            if d.control.id.starts_with("browse-") {
                return Some("it opens the system's folder picker, which an agent cannot see or operate; fill the folder field instead".into());
            }
            if d.control.id.starts_with("objective-") {
                return Some("objectives are the Operator's to start, steer and stop".into());
            }
            if matches!(action, Action::Scroll(..)) {
                return None;
            }
            operators_region(d)
        }
        Action::Key(keys) => {
            if screen(studio) == "settings" {
                // Leaving is all an agent may do there.
                return (keys.iter().any(|k| k != "escape")).then(|| {
                    "the Settings screen is the Operator's; an agent may only leave it (escape)".into()
                });
            }
            if let Some(reason) = focused.and_then(operators_region) {
                return Some(reason);
            }
            keys.iter().find_map(|key| {
                let id = bound(key)?;
                OPERATORS_COMMANDS
                    .contains(&id)
                    .then(|| format!("`{key}` is the shortcut of `{}`, which is the Operator's", command_name(id)))
            })
        }
        Action::Type(_) => match focused {
            None => Some("no field has the focus, so the text would go to the Studio's shortcuts; fill a field instead".into()),
            Some(d) if d.control.role != "field" => Some(format!("`{}` is not a field", d.control.label)),
            Some(d) => operators_region(d),
        },
        _ => None,
    }
}

fn screen(studio: &Studio) -> &'static str {
    if studio.fixture.as_deref() == Some("components") {
        "gallery"
    } else if studio.settings_open {
        "settings"
    } else if studio.project.is_none() && studio.fixture.is_none() {
        "welcome"
    } else {
        "surface"
    }
}

fn dialog_kind(studio: &Studio) -> Option<String> {
    use crate::edit::Dialog;
    studio.dialog.as_ref().map(|d| {
        match d {
            Dialog::NewProject { .. } => "NewProject",
            Dialog::OpenProject { .. } => "OpenProject",
            Dialog::Create { .. } => "Create",
            Dialog::Rename { .. } => "Rename",
            Dialog::Checkpoint { .. } => "Checkpoint",
            Dialog::MoveTo { .. } => "MoveTo",
            Dialog::Confirm { .. } => "Confirm",
            Dialog::Specialize { .. } => "Specialize",
            Dialog::ExtractBlock { .. } => "ExtractBlock",
            Dialog::SaveToLibrary { .. } => "SaveToLibrary",
            Dialog::Override { .. } => "Override",
            Dialog::LibraryConflict { .. } => "LibraryConflict",
            Dialog::NewScenario { .. } => "NewScenario",
            Dialog::TrustLocal => "TrustLocal",
            Dialog::ConfirmLive => "ConfirmLive",
            Dialog::Implement { .. } => "Implement",
            Dialog::ReviewTask { .. } => "ReviewTask",
            Dialog::LinkCode { .. } => "LinkCode",
        }
        .to_string()
    })
}

fn element_name(studio: &Studio, id: agq_language::ElementId) -> String {
    studio
        .project
        .as_ref()
        .map(|p| p.state().tree().qualified_name(id))
        .unwrap_or_default()
}

fn bounds(b: gpui::Bounds<gpui::Pixels>) -> Value {
    json!([
        f32::from(b.origin.x).round(),
        f32::from(b.origin.y).round(),
        f32::from(b.size.width).round(),
        f32::from(b.size.height).round()
    ])
}

fn control_json(d: &Drawn) -> Value {
    let c = &d.control;
    let mut v = json!({
        "id": c.id.to_string(),
        "role": c.role,
        "label": c.label.to_string(),
        "region": d.region,
        "bounds": bounds(d.shown.unwrap_or(d.bounds)),
    });
    if d.shown.is_none() {
        // Outside the visible part of its panel: a click scrolls to it.
        v["hidden"] = json!(true);
    }
    if let Some(value) = &c.value {
        v["value"] = json!(value.to_string());
    }
    if !c.enabled {
        v["enabled"] = json!(false);
    }
    if c.selected {
        v["selected"] = json!(true);
    }
    if c.focused {
        v["focused"] = json!(true);
    }
    v
}

/// Cards in view on the Surface, with their bounds in the window.
fn cards(studio: &Studio) -> Vec<Value> {
    let Some(viewport) = target::find("Viewport") else {
        return Vec::new();
    };
    let at = |p: agq_studio_scene::Point| {
        let local = studio.camera.world_to_screen(p);
        gpui::point(
            viewport.origin.x + gpui::px(local.x),
            viewport.origin.y + gpui::px(local.y),
        )
    };
    studio
        .scene
        .nodes
        .iter()
        .filter_map(|node| {
            let shown = gpui::Bounds::from_corners(at(node.bounds.min), at(node.bounds.max))
                .intersect(&viewport);
            (shown.size.width > gpui::px(6.0) && shown.size.height > gpui::px(6.0)).then(|| {
                json!({
                    "element": element_name(studio, node.id()),
                    "category": format!("{:?}", node.category).to_lowercase(),
                    "bounds": bounds(shown),
                })
            })
        })
        .take(LISTED)
        .collect()
}

/// The observation: what an agent needs to act in the Studio as it is now.
/// `full` adds every command (with why not, when unavailable) and the cards
/// in view.
pub fn observe(studio: &Studio, window: &gpui::Window, full: bool, region: Option<&str>) -> Value {
    let drawn = target::drawn();
    let controls: Vec<Value> = drawn
        .iter()
        .filter(|d| d.control.role != "area")
        .filter(|d| region.is_none_or(|r| d.region == r))
        .take(LISTED)
        .map(control_json)
        .collect();
    let context = studio.context();
    let commands: Vec<Value> = COMMANDS
        .iter()
        .filter_map(|c| {
            let why = unavailable(c.id, &context);
            if !full && why.is_some() {
                return None;
            }
            let mut v =
                json!({ "id": command_name(c.id), "label": c.label, "shortcut": c.shortcut });
            if let Some(why) = why {
                v["available"] = json!(false);
                v["why"] = json!(why);
            }
            Some(v)
        })
        .collect();
    let selection: Vec<String> = studio
        .selection
        .elements(&studio.scene)
        .into_iter()
        .map(|id| element_name(studio, id))
        .collect();
    let problems: Vec<Value> = studio
        .problems
        .iter()
        .flat_map(|(id, list)| {
            list.iter().map(move |p| (id, p))
        })
        .take(20)
        .map(|(id, problem)| json!({ "element": element_name(studio, *id), "problem": format!("{problem}") }))
        .collect();
    let panel = &studio.conversation;
    let last_reply = panel
        .conversation
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            agq_assistant::Entry::Assistant { parts, .. } => parts.iter().find_map(|p| match p {
                agq_providers::AssistantPart::Text { text } if !text.trim().is_empty() => {
                    Some(text.chars().take(600).collect::<String>())
                }
                _ => None,
            }),
            _ => None,
        });
    let waiting = panel.waiting.as_ref().map(|w| match &w.kind {
        crate::conversation::WaitingFor::Question { question, options } => {
            json!({ "kind": "question", "question": question, "options": options })
        }
        crate::conversation::WaitingFor::SaveToLibrary { question, .. } => {
            json!({ "kind": "question", "question": question })
        }
        _ => json!({ "kind": "confirmation" }),
    });
    let viewport = window.viewport_size();
    let mut observation = json!({
        "identity": identity(studio),
        "screen": screen(studio),
        "screenRevision": studio.control.screen_revision,
        "window": {
            "width": f32::from(viewport.width).round(),
            "height": f32::from(viewport.height).round(),
            "active": window.is_window_active(),
        },
        "project": studio.project.as_ref().map(|p| json!({
            "folder": p.folder().display().to_string(),
            "revision": p.state().revision(),
            "saved": studio.saved,
        })),
        "view": format!("{:?}", studio.view).to_lowercase(),
        "panels": {
            "left": (!studio.outline_hidden && !studio.panels_hidden).then(|| format!("{:?}", studio.left).to_lowercase()),
            "right": (!studio.inspector_hidden && !studio.panels_hidden).then(|| format!("{:?}", studio.panel).to_lowercase()),
            "conversation": studio.conversation.shown && !studio.panels_hidden,
        },
        "settings": studio.settings_open.then(|| studio.settings_section_name()),
        "dialog": dialog_kind(studio),
        // A dialog asking for the Operator's approval: no agent answers it.
        "approval": approval(studio),
        "palette": studio.palette.as_ref().map(|m| format!("{m:?}")),
        "selection": selection,
        "status": studio.status,
        "problems": problems,
        "conversation": {
            "runtime": panel.model_name,
            "running": panel.running(),
            "phase": panel.phase,
            "waiting": waiting,
            "entries": panel.conversation.entries.len(),
            "lastReply": last_reply,
            "keyMissing": panel.key_missing,
        },
        "task": studio.implementation.task.as_ref().map(|t| json!({
            "job": t.job, "title": t.title, "phase": t.phase, "progress": t.progress,
        })),
        "builds": {
            "working": studio.develop.work.is_some(),
            "message": studio.develop.message,
        },
        "agents": {
            "gate": format!("{:?}", studio.control.gate).to_lowercase(),
            "held": studio.control.held(),
            "lastEvent": studio.control.next_seq - 1,
        },
        "commands": commands,
        "controls": controls,
    });
    if full {
        observation["cards"] = json!(cards(studio));
    }
    observation
}

/// Reads an action from a request body.
pub fn parse_action(body: &Value) -> Result<Action, String> {
    let action = &body["action"];
    let text = |field: &str| -> Result<String, String> {
        action[field]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("the action needs `{field}`"))
    };
    match action["kind"].as_str().unwrap_or_default() {
        "command" => {
            let name = text("id")?;
            command_by_name(&name)
                .map(Action::Command)
                .ok_or_else(|| format!("there is no command `{name}`; the observation lists them"))
        }
        "click" => Ok(Action::Click(text("control")?)),
        "fill" => Ok(Action::Fill(text("control")?, text("text")?)),
        "key" => Ok(Action::Key(
            text("keys")?
                .split_whitespace()
                .map(str::to_string)
                .collect(),
        )),
        "type" => Ok(Action::Type(text("text")?)),
        "scroll" => Ok(Action::Scroll(
            text("control")?,
            action["dy"].as_f64().unwrap_or(200.0) as f32,
        )),
        "select" => Ok(Action::Select(text("element")?)),
        "open_project" => Ok(Action::OpenProject(text("folder")?.into())),
        "wait" => {
            let until = &action["until"];
            let condition = Condition {
                dialog: until
                    .get("dialog")
                    .map(|d| d.as_str().map(str::to_string)),
                screen: until["screen"].as_str().map(str::to_string),
                control: until["control"].as_str().map(str::to_string),
                enabled: until["enabled"] == true,
                status_contains: until["statusContains"].as_str().map(str::to_string),
                idle: until["idle"] == true,
            };
            let ms = action["timeoutMs"].as_u64().unwrap_or(10_000).min(600_000);
            Ok(Action::Wait(condition, Duration::from_millis(ms)))
        }
        "" => Err("the action needs a `kind`: command, click, fill, key, type, scroll, select, open_project or wait".into()),
        other => Err(format!("there is no action `{other}`")),
    }
}

/// Why an action observed against `body`'s identity and screen cannot be
/// carried out now: another instance, project or build, or a screen that
/// changed since.
pub fn stale(studio: &Studio, body: &Value) -> Option<String> {
    let expect = &body["expect"];
    let instance = expect["instance"].as_str();
    if instance != Some(studio.control.instance.as_str()) {
        return Some(match instance {
            None => "stale: say which Studio you observed (`expect.instance` from the observation)"
                .into(),
            Some(other) => format!(
                "stale: observed in Studio instance {other}, but this is {}; observe again",
                studio.control.instance
            ),
        });
    }
    if let Some(project) = expect.get("project") {
        let now = studio
            .project
            .as_ref()
            .map(|p| p.folder().display().to_string());
        if project.as_str().map(str::to_string) != now {
            return Some(format!(
                "stale: observed with project {project}, but {} is open now",
                now.as_deref().unwrap_or("no project")
            ));
        }
    }
    if let Some(session) = expect.get("session") {
        let now = studio.session_path.display().to_string();
        if session.as_str() != Some(now.as_str()) {
            return Some(format!(
                "stale: observed with session {session}, but this is {now}"
            ));
        }
    }
    if let Some(build) = expect.get("build") {
        let now = studio.running_build();
        if build.as_str().map(str::to_string) != now {
            return Some(format!(
                "stale: observed in build {build}, but this is {}",
                now.as_deref().unwrap_or("a development build")
            ));
        }
    }
    let Some(seen) = body["observed"].as_u64() else {
        return Some(
            "stale: say which observation you acted on (`observed`: its screenRevision)".into(),
        );
    };
    if seen != studio.control.screen_revision {
        return Some(format!(
            "stale: the screen changed since you observed it (now: {}{}); observe again",
            screen(studio),
            dialog_kind(studio)
                .map(|d| format!(", dialog {d}"))
                .unwrap_or_default()
        ));
    }
    None
}

/// A control on screen by id, or else by label: the topmost visible one,
/// else one its panel hides.
fn control_named<'a>(drawn: &'a [Drawn], name: &str) -> Option<&'a Drawn> {
    let by = |visible: bool, id: bool| {
        drawn.iter().rev().find(move |d| {
            d.shown.is_some() == visible
                && if id {
                    d.control.id == name
                } else {
                    d.control.label == name
                }
        })
    };
    by(true, true)
        .or_else(|| by(true, false))
        .or_else(|| by(false, true))
        .or_else(|| by(false, false))
}

fn holds(studio: &Studio, condition: &Condition, agent: &str) -> bool {
    if let Some(dialog) = &condition.dialog
        && dialog_kind(studio) != *dialog
    {
        return false;
    }
    if let Some(wanted) = &condition.screen
        && screen(studio) != wanted
    {
        return false;
    }
    if let Some(name) = &condition.control {
        let drawn = target::drawn();
        match control_named(&drawn, name) {
            Some(d) if d.shown.is_some() && (!condition.enabled || d.control.enabled) => {}
            _ => return false,
        }
    }
    if let Some(text) = &condition.status_contains
        && !studio.status.contains(text.as_str())
    {
        return false;
    }
    if condition.idle
        && ((agent != "Assistant" && studio.conversation.running())
            || studio.runs.running()
            || studio.implementation.task.is_some()
            || studio.implementation.checking()
            || studio.develop.work.is_some())
    {
        return false;
    }
    true
}

/// One step of an action in progress, one per tick.
enum Step {
    /// The pointer goes there and presses (first tick), then releases.
    Press(gpui::Point<gpui::Pixels>),
    Release(gpui::Point<gpui::Pixels>),
    Keys(Vec<String>),
    Scroll(gpui::Point<gpui::Pixels>, f32),
    Command(CommandId),
    Select(agq_language::ElementId),
    Open(std::path::PathBuf),
    /// The control is clicked where it shows; its panel scrolls to it first
    /// when it is outside the visible part (tried for a few frames).
    Reveal(String, u32),
    /// Once the field clicked has the focus (tried for a few frames): its
    /// text replaced; the field keeps the focus, so Enter reaches it (or its
    /// dialog) next.
    Fill(String, String, u32),
    /// Text into the field that has the focus, checked when it is typed.
    Type(String),
    /// Frames for the Studio to show the effect.
    Settle(u32),
}

struct Active {
    request: Request,
    agent: String,
    what: String,
    steps: VecDeque<Step>,
    started: Instant,
}

struct Waiting {
    request: Request,
    agent: String,
    what: String,
    condition: Condition,
    until: Instant,
}

/// What the workspace does with the control interface each tick: takes new
/// requests, answers observations at once, and carries out one step of one
/// action (held while paused).
pub fn tick(studio: &gpui::Entity<Studio>, window: &mut gpui::Window, cx: &mut gpui::App) -> bool {
    use crate::workspace::StudioExt;
    let mut changed = false;
    // The screen's shape, for stale actions.
    studio.update(cx, |studio, _| {
        let now = shape(studio);
        if now != studio.control.shape {
            studio.control.shape = now;
            studio.control.screen_revision += 1;
        }
        while let Ok(request) = studio.control.requests.try_recv() {
            studio.control.waiting.push_back(request);
        }
        // Nobody waits for these any more: dropped, not carried out late.
        let now = Instant::now();
        let (expired, waiting): (Vec<Request>, Vec<Request>) = studio
            .control
            .waiting
            .drain(..)
            .partition(|r| r.deadline <= now);
        studio.control.waiting = waiting.into();
        for request in expired {
            let agent = request.body["agent"].as_str().unwrap_or("an agent").to_string();
            let answer = json!({ "ok": false, "error": "the request waited past its deadline (agents were paused) and was dropped; observe again" });
            studio.control.record(&agent, "a request that waited too long", false, answer.clone());
            request.reply.send(answer);
        }
        // Waits, beside any action.
        let waits = std::mem::take(&mut studio.control.waits);
        for wait in waits {
            if holds(studio, &wait.condition, &wait.agent) {
                let answer = json!({
                    "ok": true,
                    "did": wait.what,
                    "screen": screen(studio),
                    "screenRevision": studio.control.screen_revision,
                    "dialog": dialog_kind(studio),
                    "status": studio.status,
                });
                studio.control.record(&wait.agent, &wait.what, true, answer.clone());
                wait.request.reply.send(answer);
            } else if now >= wait.until {
                let answer = json!({ "ok": false, "error": format!(
                    "the wait timed out (screen {}, dialog {}, status “{}”)",
                    screen(studio),
                    dialog_kind(studio).unwrap_or_else(|| "none".into()),
                    studio.status
                )});
                studio.control.record(&wait.agent, &wait.what, false, answer.clone());
                wait.request.reply.send(answer);
            } else {
                studio.control.waits.push(wait);
            }
        }
    });
    // Answers that never wait.
    loop {
        let next = studio.update(cx, |studio, _| {
            let at = studio
                .control
                .waiting
                .iter()
                .position(|r| r.body["op"] != "act")?;
            studio.control.waiting.remove(at)
        });
        let Some(request) = next else { break };
        let answer = studio.update(cx, |studio, _| answer(studio, &request.body, window));
        request.reply.send(answer);
    }
    // The action in progress, one step a tick.
    let active = studio.update(cx, |studio, _| studio.control.active.take());
    if let Some(mut active) = active {
        changed = true;
        match active.steps.pop_front() {
            None => {
                let summary = studio.update(cx, |studio, _| {
                    studio.control.acting = None;
                    if let Some(reason) = studio.control.refused.take() {
                        let answer = json!({ "ok": false, "error": format!("refused: {reason}") });
                        studio.control.record(&active.agent, &active.what, false, answer.clone());
                        return answer;
                    }
                    let summary = json!({
                        "ok": true,
                        "did": active.what,
                        "screen": screen(studio),
                        "screenRevision": studio.control.screen_revision,
                        "dialog": dialog_kind(studio),
                        "status": studio.status,
                        "selection": studio.selection.elements(&studio.scene).into_iter().map(|id| element_name(studio, id)).collect::<Vec<_>>(),
                        "tookMs": active.started.elapsed().as_millis() as u64,
                    });
                    studio
                        .control
                        .record(&active.agent, &active.what, true, summary.clone());
                    summary
                });
                active.request.reply.send(summary);
            }
            Some(step) => {
                let outcome = run_step(studio, step, &mut active, window, cx);
                match outcome {
                    Ok(()) => studio.update(cx, |studio, _| studio.control.active = Some(active)),
                    Err(error) => {
                        let answer = json!({ "ok": false, "error": error });
                        studio.update(cx, |studio, _| {
                            studio.control.acting = None;
                            studio.control.refused = None;
                            studio.control.record(
                                &active.agent,
                                &active.what,
                                false,
                                answer.clone(),
                            )
                        });
                        active.request.reply.send(answer);
                    }
                }
            }
        }
        return changed;
    }
    // The next action, unless the gate holds it; a Step is used by an
    // action that starts, not by one refused.
    let next = studio.update(cx, |studio, _| {
        if studio.control.gate == Gate::Pause {
            return None;
        }
        studio.control.waiting.pop_front()
    });
    if let Some(request) = next {
        changed = true;
        let started = studio.act(cx, |studio| start(studio, request));
        match started {
            Ok(()) => studio.update(cx, |studio, _| {
                if studio.control.gate == Gate::Step {
                    studio.control.gate = Gate::Pause;
                }
            }),
            Err(refused) => {
                let (request, answer, agent, what) = *refused;
                studio.update(cx, |studio, _| {
                    studio.control.record(&agent, &what, false, answer.clone())
                });
                request.reply.send(answer);
            }
        }
    }
    changed
}

/// Answers a request that does not act.
fn answer(studio: &mut Studio, body: &Value, window: &gpui::Window) -> Value {
    cache_commit(studio);
    match body["op"].as_str().unwrap_or_default() {
        "hello" => json!({ "ok": true, "identity": identity(studio) }),
        "observe" => {
            let mut v = observe(
                studio,
                window,
                body["detail"] == "full",
                body["region"].as_str(),
            );
            v["ok"] = json!(true);
            v
        }
        "events" => {
            let since = body["since"].as_u64().unwrap_or(0);
            json!({ "ok": true, "events": studio.control.events(since) })
        }
        "gate" => match body["mode"].as_str() {
            Some("pause") => {
                studio.pause_agents();
                json!({ "ok": true, "gate": "pause" })
            }
            Some("step") => {
                studio.step_agents();
                json!({ "ok": true, "gate": "step" })
            }
            Some("run") => {
                studio.resume_agents();
                json!({ "ok": true, "gate": "run" })
            }
            _ => json!({ "ok": false, "error": "`mode` is pause, step or run" }),
        },
        other => {
            json!({ "ok": false, "error": format!("there is no operation `{other}`: hello, observe, act, events or gate") })
        }
    }
}

/// Reads this build's commit once (its manifest in the builds folder).
fn cache_commit(studio: &mut Studio) {
    if studio.control.commit.is_none() {
        let commit = studio.running_build().and_then(|id| {
            agq_launcher::Manifest::load(&studio.builds_root().join(id))
                .ok()
                .map(|m| m.commit)
                .filter(|c| !c.is_empty())
        });
        studio.control.commit = Some(commit);
    }
}

type Refused = Box<(Request, Value, String, String)>;

/// Checks an action and plans its steps. A stale or impossible action is
/// refused before anything happens.
fn start(studio: &mut Studio, request: Request) -> Result<(), Refused> {
    let agent = request.body["agent"]
        .as_str()
        .unwrap_or("an agent")
        .to_string();
    let why = request.body["why"].as_str().unwrap_or_default().to_string();
    let refuse = |request: Request, error: String, what: String| {
        Err(Box::new((
            request,
            json!({ "ok": false, "error": error }),
            agent.clone(),
            what,
        )))
    };
    let action = match parse_action(&request.body) {
        Ok(action) => action,
        Err(error) => return refuse(request, error, "an action that could not be read".into()),
    };
    let what = describe(&action);
    if let Some(reason) = stale(studio, &request.body) {
        return refuse(request, reason, what);
    }
    let drawn = target::drawn();
    if let Some(reason) = operators_only(studio, &action, &drawn) {
        return refuse(request, format!("refused: {reason}"), what);
    }
    let find = |name: &str| -> Result<&Drawn, String> {
        match control_named(&drawn, name) {
            None => Err(format!(
                "stale: no control `{name}` is on screen now; observe again"
            )),
            Some(d) if !d.control.enabled => Err(format!("`{}` is disabled now", d.control.label)),
            Some(d) => Ok(d),
        }
    };
    let mut steps = VecDeque::new();
    let mut marked = None;
    match &action {
        Action::Command(id) => {
            if let Some(reason) = unavailable(*id, &studio.context()) {
                return refuse(
                    request,
                    format!("`{}` is not available: {reason}", command_name(*id)),
                    what,
                );
            }
            steps.push_back(Step::Command(*id));
        }
        Action::Click(name) => match find(name) {
            Ok(d) => {
                marked = Some((d.shown.unwrap_or(d.clip), d.control.label.to_string()));
                steps.push_back(Step::Reveal(name.clone(), 6));
            }
            Err(error) => return refuse(request, error, what),
        },
        Action::Fill(name, text) => match find(name) {
            Ok(d) if d.control.role != "field" => {
                let error = format!(
                    "`{}` is a {}, not a field: click it instead",
                    d.control.label, d.control.role
                );
                return refuse(request, error, what);
            }
            Ok(d) => {
                marked = Some((d.shown.unwrap_or(d.clip), d.control.label.to_string()));
                steps.push_back(Step::Reveal(name.clone(), 6));
                steps.push_back(Step::Fill(name.clone(), text.clone(), 10));
            }
            Err(error) => return refuse(request, error, what),
        },
        Action::Scroll(name, dy) => match find(name) {
            Ok(d) => steps.push_back(Step::Scroll(d.shown.unwrap_or(d.clip).center(), *dy)),
            Err(error) => return refuse(request, error, what),
        },
        Action::Key(keys) => {
            for key in keys {
                if gpui::Keystroke::parse(key).is_err() {
                    return refuse(
                        request,
                        format!(
                            "`{key}` is not a key (GPUI syntax: ctrl-s, escape, enter, shift-1)"
                        ),
                        what,
                    );
                }
            }
            steps.push_back(Step::Keys(keys.clone()));
        }
        Action::Type(text) => steps.push_back(Step::Type(text.clone())),
        Action::Select(name) => {
            let id = studio
                .project
                .as_ref()
                .and_then(|p| p.state().tree().find(name));
            match id {
                Some(id) => steps.push_back(Step::Select(id)),
                None => {
                    return refuse(
                        request,
                        format!("there is no element `{name}` in the open model"),
                        what,
                    );
                }
            }
        }
        Action::OpenProject(folder) => {
            if agent == "Assistant" {
                return refuse(
                    request,
                    "refused: opening another project ends your own turn; ask the Operator to open it".into(),
                    what,
                );
            }
            if let Some(reason) = unavailable(CommandId::OpenProject, &studio.context()) {
                return refuse(
                    request,
                    format!("a project cannot be opened now: {reason}"),
                    what,
                );
            }
            if !folder.join("model").is_dir() {
                return refuse(
                    request,
                    format!(
                        "{} is not an Agentique project (no model folder)",
                        folder.display()
                    ),
                    what,
                );
            }
            steps.push_back(Step::Open(folder.clone()));
        }
        Action::Wait(condition, timeout) => {
            // Beside any action, until it holds or times out.
            studio.control.waits.push(Waiting {
                request,
                agent,
                what,
                condition: condition.clone(),
                until: Instant::now() + *timeout,
            });
            return Ok(());
        }
    }
    steps.push_back(Step::Settle(3));
    let shown = match &why {
        why if why.is_empty() => format!("{agent}: {what}"),
        why => format!("{agent}: {what} — {why}"),
    };
    if let Some((bounds, _)) = marked {
        studio.control.marks.push(Mark {
            bounds,
            label: shown.clone(),
            until: Instant::now() + MARK,
        });
    }
    studio.control.activity = Some((shown, Instant::now()));
    studio.mark(crate::studio::Dirty::STATUS | crate::studio::Dirty::OVERLAY);
    studio.control.refused = None;
    studio.control.active = Some(Active {
        request,
        agent,
        what,
        steps,
        started: Instant::now(),
    });
    Ok(())
}

fn describe(action: &Action) -> String {
    match action {
        Action::Command(id) => format!("command {}", command_name(*id)),
        Action::Click(name) => format!("click “{name}”"),
        Action::Fill(name, text) => format!(
            "fill “{name}” with “{}”",
            text.chars().take(60).collect::<String>()
        ),
        Action::Key(keys) => format!("press {}", keys.join(" ")),
        Action::Type(text) => format!("type “{}”", text.chars().take(60).collect::<String>()),
        Action::Scroll(name, dy) => format!("scroll “{name}” by {dy}"),
        Action::Select(name) => format!("select {name}"),
        Action::OpenProject(folder) => format!("open project {}", folder.display()),
        Action::Wait(..) => "wait".into(),
    }
}

/// Carries out one step with the agent named as acting until the effects
/// it dispatched have run (GPUI runs deferred work in order, so the clearing
/// queued last comes after them).
fn run_step(
    studio: &gpui::Entity<Studio>,
    step: Step,
    active: &mut Active,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> Result<(), String> {
    // A dialog asking for the Operator's approval may have opened since the
    // action started: its own controls are not an agent's either.
    if matches!(
        step,
        Step::Press(..) | Step::Release(..) | Step::Keys(..) | Step::Type(..) | Step::Fill(..)
    ) && let Some(kind) = approval(studio.read(cx))
    {
        return Err(format!(
            "refused: the {kind} dialog opened; it asks for the Operator's own approval"
        ));
    }
    let agent = active.agent.clone();
    studio.update(cx, |studio, _| studio.control.acting = Some(agent));
    let result = carry_out(studio, step, active, window, cx);
    let clear = studio.clone();
    cx.defer(move |cx| clear.update(cx, |studio, _| studio.control.acting = None));
    result
}

fn carry_out(
    studio: &gpui::Entity<Studio>,
    step: Step,
    active: &mut Active,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> Result<(), String> {
    use crate::workspace::StudioExt;
    match step {
        Step::Press(at) => input::click(at, true, window, cx),
        Step::Release(at) => input::click(at, false, window, cx),
        Step::Keys(keys) => {
            for key in keys {
                input::press(&key, window, cx)?;
            }
        }
        Step::Scroll(at, dy) => input::scroll(at, dy, window, cx),
        Step::Command(id) => {
            window.dispatch_action(Box::new(crate::commands::Run(id)), cx);
        }
        Step::Select(id) => studio.act(cx, |studio| studio.reveal(id)),
        Step::Open(folder) => {
            studio.act(cx, |studio| studio.open_project(&folder));
            if studio
                .read(cx)
                .project
                .as_ref()
                .map(|p| p.folder().to_path_buf())
                != Some(folder.clone())
            {
                return Err(format!(
                    "the project did not open: {}",
                    studio.read(cx).status
                ));
            }
        }
        Step::Reveal(name, tries) => {
            let drawn = target::drawn();
            let Some(d) = control_named(&drawn, &name) else {
                return Err(format!("`{name}` is no longer on screen"));
            };
            match d.shown {
                Some(shown) => {
                    let at = shown.center();
                    active.steps.push_front(Step::Release(at));
                    active.steps.push_front(Step::Press(at));
                }
                None if tries > 0 => {
                    // Scroll its panel by the distance to it, as the
                    // Operator would, and look again next frame.
                    let dy = f32::from(d.bounds.center().y - d.clip.center().y);
                    input::scroll(d.clip.center(), dy, window, cx);
                    window.refresh();
                    active.steps.push_front(Step::Reveal(name, tries - 1));
                }
                None => {
                    return Err(format!(
                        "`{name}` stays outside the visible part of its panel; scroll it into view"
                    ));
                }
            }
        }
        Step::Fill(name, text, tries) => {
            let drawn = target::drawn();
            let focused = drawn.iter().any(|d| {
                d.control.focused
                    && d.control.role == "field"
                    && (d.control.id == name.as_str() || d.control.label == name.as_str())
            });
            if !focused {
                if tries == 0 {
                    return Err(format!(
                        "`{name}` did not take the focus, so nothing was typed"
                    ));
                }
                window.refresh();
                active.steps.push_front(Step::Fill(name, text, tries - 1));
                return Ok(());
            }
            let region = drawn
                .iter()
                .find(|d| d.control.focused && d.control.role == "field")
                .map(|d| d.region)
                .unwrap_or(target::ROOT);
            input::press("ctrl-a", window, cx)?;
            input::press("backspace", window, cx)?;
            input::type_text(&text, window, cx);
            let agent = active.agent.clone();
            studio.update(cx, |studio, _| {
                studio.control.mark_typed(&name, region, &agent)
            });
        }
        Step::Type(text) => {
            let drawn = target::drawn();
            let Some(field) = drawn.iter().rev().find(|d| d.control.focused) else {
                return Err("no field has the focus, so nothing was typed".into());
            };
            if field.control.role != "field" || OPERATORS_REGIONS.contains(&field.region) {
                return Err(format!(
                    "`{}` is not a field an agent may type into",
                    field.control.label
                ));
            }
            let id = field.control.id.to_string();
            let region = field.region;
            input::type_text(&text, window, cx);
            let agent = active.agent.clone();
            studio.update(cx, |studio, _| {
                studio.control.mark_typed(&id, region, &agent)
            });
        }
        Step::Settle(frames) => {
            if frames > 1 {
                active.steps.push_front(Step::Settle(frames - 1));
            }
            window.refresh();
        }
    }
    Ok(())
}

impl Studio {
    /// Whether an agent's action is being carried out now; then `what` is
    /// refused as the Operator's own: the status says so and the action's
    /// answer reports it (C-53, §3 roles).
    pub fn refused_to_agents(&mut self, what: &str) -> bool {
        let Some(agent) = self.control.acting.clone() else {
            return false;
        };
        let reason = format!("{what} is the Operator's own; {agent} cannot do it");
        self.status = format!("Refused: {reason}");
        self.control.refused = Some(reason);
        self.mark(crate::studio::Dirty::STATUS);
        true
    }

    /// Pauses agents (C-53): the control interface holds their next action,
    /// and the Assistant's running turn holds at its next tool call.
    pub fn pause_agents(&mut self) {
        if self.refused_to_agents("pausing agents") {
            return;
        }
        self.control.gate = Gate::Pause;
        self.pause_assistant();
        self.status = "Agents pause at their next action".into();
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Lets one action (and one tool call) through, then holds again.
    pub fn step_agents(&mut self) {
        if self.refused_to_agents("stepping agents") {
            return;
        }
        self.control.gate = Gate::Step;
        if self.assistant_paused() {
            self.step_assistant();
        }
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Lets agents go on.
    pub fn resume_agents(&mut self) {
        if self.refused_to_agents("resuming agents") {
            return;
        }
        self.control.gate = Gate::Run;
        self.resume_assistant();
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Whether agents are held (or will be at their next action).
    pub fn agents_paused(&self) -> bool {
        self.control.gate != Gate::Run || self.assistant_paused()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_stable_names_both_ways() {
        assert_eq!(command_name(CommandId::CreatePart), "create-part");
        assert_eq!(
            command_name(CommandId::ZoomToSelection),
            "zoom-to-selection"
        );
        for command in COMMANDS {
            assert_eq!(command_by_name(&command_name(command.id)), Some(command.id));
        }
        assert_eq!(command_by_name("delete-everything"), None);
    }

    #[test]
    fn actions_are_read_strictly() {
        let read = |action: Value| parse_action(&json!({ "action": action }));
        assert_eq!(
            read(json!({"kind": "command", "id": "checkpoint"})),
            Ok(Action::Command(CommandId::Checkpoint))
        );
        assert_eq!(
            read(json!({"kind": "click", "control": "dialog-confirm"})),
            Ok(Action::Click("dialog-confirm".into()))
        );
        assert_eq!(
            read(json!({"kind": "key", "keys": "ctrl-s escape"})),
            Ok(Action::Key(vec!["ctrl-s".into(), "escape".into()]))
        );
        assert!(read(json!({"kind": "command", "id": "launch-missiles"})).is_err());
        assert!(read(json!({"kind": "teleport"})).is_err());
        assert!(read(json!({"kind": "click"})).is_err());
        assert!(read(json!({})).is_err());
        let wait = read(json!({"kind": "wait", "until": {"dialog": null, "statusContains": "Saved"}, "timeoutMs": 900000})).unwrap();
        match wait {
            Action::Wait(condition, timeout) => {
                assert_eq!(condition.dialog, Some(None));
                assert_eq!(condition.status_contains.as_deref(), Some("Saved"));
                assert_eq!(timeout, Duration::from_secs(600), "at most ten minutes");
            }
            other => panic!("{other:?}"),
        }
    }

    fn drawn(id: &str, role: &'static str, region: &'static str, focused: bool) -> Drawn {
        Drawn {
            control: target::Control::new(role, id.to_string()).focused(focused),
            bounds: gpui::Bounds::default(),
            shown: None,
            clip: gpui::Bounds::default(),
            region,
            order: 0,
        }
    }

    #[test]
    fn the_operators_own_controls_are_refused_to_agents() {
        let (mut app, _folder) = crate::edit::app_tests::studio("operators-own");
        let surface = [drawn("dialog-confirm", "button", "dialog", false)];
        // Ordinary actions pass.
        assert_eq!(
            operators_only(&app, &Action::Command(CommandId::CreatePart), &surface),
            None
        );
        assert_eq!(
            operators_only(&app, &Action::Click("dialog-confirm".into()), &surface),
            None
        );
        // Locking, trust and the Operator's voice are the Operator's, and so
        // are their shortcuts.
        for id in [
            CommandId::Lock,
            CommandId::TrustLocal,
            CommandId::AskAssistant,
        ] {
            assert!(
                operators_only(&app, &Action::Command(id), &surface).is_some(),
                "{id:?}"
            );
        }
        assert!(operators_only(&app, &Action::Key(vec!["l".into()]), &surface).is_some());
        assert!(operators_only(&app, &Action::Key(vec!["ctrl-l".into()]), &surface).is_some());
        // The agents chip, Settings and the Conversation.
        let chip = [drawn("agents-resume", "button", "title", false)];
        assert!(operators_only(&app, &Action::Click("agents-resume".into()), &chip).is_some());
        let settings = [drawn("build-use-confirm", "button", "settings", false)];
        assert!(
            operators_only(&app, &Action::Click("build-use-confirm".into()), &settings).is_some()
        );
        let composer = [drawn("Message", "field", "conversation", true)];
        assert!(
            operators_only(
                &app,
                &Action::Fill("Message".into(), "hi".into()),
                &composer
            )
            .is_some()
        );
        assert!(operators_only(&app, &Action::Type("hi".into()), &composer).is_some());
        assert!(operators_only(&app, &Action::Key(vec!["enter".into()]), &composer).is_some());
        // Text goes only to a focused field.
        assert!(operators_only(&app, &Action::Type("p".into()), &surface).is_some());
        let field = [drawn("Checkpoint message", "field", "dialog", true)];
        assert_eq!(
            operators_only(&app, &Action::Type("first".into()), &field),
            None
        );
        // A dialog asking for the Operator's approval: nothing but waiting.
        let api = crate::edit::app_tests::part(&mut app, "api");
        app.operation(
            "Lock api",
            agq_system_state::Operation::Lock { element: api },
        );
        app.rename(api, "gateway");
        assert!(approval(&app).is_some());
        for action in [
            Action::Click("dialog-confirm".into()),
            Action::Key(vec!["enter".into()]),
            Action::Command(CommandId::Checkpoint),
        ] {
            let refused = operators_only(&app, &action, &surface);
            assert!(
                refused.as_deref().is_some_and(|r| r.contains("approval")),
                "{action:?}: {refused:?}"
            );
        }
        let wait = Action::Wait(Condition::default(), Duration::from_secs(1));
        assert_eq!(operators_only(&app, &wait, &surface), None);
    }

    #[test]
    fn the_operators_effects_are_refused_while_an_agents_step_runs_whatever_the_route() {
        let (mut app, _folder) = crate::edit::app_tests::studio("operators-effects");
        let api = crate::edit::app_tests::part(&mut app, "api");
        app.operation(
            "Lock api",
            agq_system_state::Operation::Lock { element: api },
        );
        // The Operator's change to the locked part asks for confirmation.
        app.rename(api, "gateway");
        assert!(approval(&app).is_some());
        app.control.acting = Some("evaluator".into());
        // An Enter chain, Space on a focused button or the palette all end
        // in these effects; each is refused while the agent's step runs.
        app.answer(true);
        assert!(
            approval(&app).is_some(),
            "the lock confirmation is still open"
        );
        assert!(app.control.refused.take().is_some());
        app.dialog = None;
        let locked_before = app.project.as_ref().unwrap().state().locks().clone();
        app.operation(
            "Unlock api",
            agq_system_state::Operation::Unlock { element: api },
        );
        assert_eq!(
            app.project.as_ref().unwrap().state().locks(),
            &locked_before
        );
        app.execute(CommandId::Lock);
        app.execute(CommandId::Undo);
        app.execute(CommandId::NewConversation);
        assert!(app.control.refused.take().is_some());
        app.control.gate = Gate::Pause;
        app.resume_agents();
        assert_eq!(
            app.control.gate,
            Gate::Pause,
            "an agent cannot resume itself"
        );
        assert!(app.use_build("some-build").is_err());
        app.conversation.input = "do as I say".into();
        app.send_message();
        assert!(
            app.conversation.conversation.entries.is_empty(),
            "nothing said as the Operator"
        );
        app.control.acting = None;
        app.control.refused = None;
        // The Operator's own commands still work.
        app.resume_agents();
        assert_eq!(app.control.gate, Gate::Run);
    }

    #[test]
    fn a_change_an_agents_action_makes_is_the_assistants_naming_the_agent() {
        let (mut app, _folder) = crate::edit::app_tests::studio("agent-author");
        app.control.acting = Some("evaluator".into());
        let api = crate::edit::app_tests::part(&mut app, "api");
        app.control.acting = None;
        assert_eq!(app.highlights[&api].1, agq_system_state::Actor::Assistant);
        assert!(app.status.contains("(by evaluator)"), "{}", app.status);
        // The Operator's own change stays the Operator's.
        let db = crate::edit::app_tests::part(&mut app, "db");
        assert_eq!(app.highlights[&db].1, agq_system_state::Actor::Operator);
    }

    #[test]
    fn a_panel_field_an_agent_typed_into_commits_as_its_change_whoever_ends_the_edit() {
        let (mut app, _folder) = crate::edit::app_tests::studio("typed-field");
        // The agent typed into the Inspector's Value field; the commit comes
        // later (the focus leaves at the next frame, or the Operator clicks
        // away), outside the agent's step.
        app.control.mark_typed("Value", "inspector", "evaluator");
        let began = app.control.begin_typed_commit(Some("Value"));
        assert!(began);
        let api = crate::edit::app_tests::part(&mut app, "api");
        app.control.end_typed_commit(began);
        assert_eq!(app.highlights[&api].1, agq_system_state::Actor::Assistant);
        assert!(app.status.contains("(by evaluator)"), "{}", app.status);
        // Used once: the field's next commit is the Operator's.
        assert!(!app.control.begin_typed_commit(Some("Value")));
        let db = crate::edit::app_tests::part(&mut app, "db");
        assert_eq!(app.highlights[&db].1, agq_system_state::Actor::Operator);
        // Loading other text into the field, or another project, forgets it.
        app.control.mark_typed("Value", "inspector", "evaluator");
        app.control.forget_typed("Value");
        assert!(!app.control.begin_typed_commit(Some("Value")));
        app.control.mark_typed("Guard", "inspector", "evaluator");
        app.control.forget_all_typed();
        assert!(!app.control.begin_typed_commit(Some("Guard")));
        // A dialog's fields are read when its confirm is pressed: not marked.
        app.control.mark_typed("Name", "dialog", "evaluator");
        assert!(!app.control.begin_typed_commit(Some("Name")));
    }

    #[test]
    fn an_answer_too_long_loses_controls_not_its_end() {
        let controls: Vec<Value> = (0..400)
            .map(|i| json!({ "id": format!("control-{i}"), "label": "x".repeat(40) }))
            .collect();
        let answer =
            json!({ "ok": true, "commands": [{ "id": "checkpoint" }], "controls": controls });
        let text = fitted(answer, 4000);
        assert!(text.chars().count() <= 4000);
        let back: Value = serde_json::from_str(&text).expect("still JSON");
        assert!(back["controlsOmitted"].as_u64().unwrap() > 0);
        assert_eq!(back["commands"][0]["id"], "checkpoint");
    }
}
