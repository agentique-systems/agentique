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
//!   against; one for another instance, project or build, a screen that
//!   changed since, or a control that is gone or disabled is refused as
//!   stale, and nothing happens.
//! - Every action and its effect go to the **event trace**; the control an
//!   agent acts on is marked on screen with who did what, and the title bar
//!   shows the latest action. **Pause** holds actions (observing is never
//!   held), **Step** lets one through, **Resume** goes on.
//! - Requests come from Agentique's own tools (the Assistant in this
//!   Studio) and from the local endpoint ([`server`]), which the Orchestrator
//!   uses to drive a test instance. They are carried out on the UI thread,
//!   one action at a time, in the workspace's tick.

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
                let mut text = serde_json::to_string_pretty(&answer).unwrap_or_default();
                if text.chars().count() > agq_assistant::tools::RESULT_LIMIT {
                    text = text
                        .chars()
                        .take(agq_assistant::tools::RESULT_LIMIT)
                        .collect::<String>()
                        + "\n… (cut: ask for less detail)";
                }
                let _ = sender.send(if ok {
                    agq_assistant::ToolResult::answer(text)
                } else {
                    agq_assistant::ToolResult::error(text)
                });
            }
        }
    }
}

/// One request: what was asked, by whom, and where the answer goes.
pub struct Request {
    pub body: Value,
    pub reply: Reply,
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

    /// Drops the actions `agent` asked for that have not started (fail
    /// closed when its turn is stopped).
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
    json!({
        "instance": studio.control.instance,
        "version": env!("CARGO_PKG_VERSION"),
        "build": studio.running_build(),
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

/// The screen's shape: what an action's place on screen depends on.
fn shape(studio: &Studio) -> String {
    format!(
        "{}|{}|{}|{:?}|{}",
        screen(studio),
        dialog_kind(studio).unwrap_or_default(),
        studio.palette.is_some(),
        studio.project.as_ref().map(|p| p.folder().to_path_buf()),
        studio.settings_section_name()
    )
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
        "bounds": bounds(d.bounds),
    });
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
pub fn observe(studio: &Studio, window: &gpui::Window, full: bool) -> Value {
    let drawn = target::drawn();
    let controls: Vec<Value> = drawn
        .iter()
        .filter(|d| d.control.role != "area")
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
        "controls": controls,
        "commands": commands,
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
    if let Some(build) = expect.get("build") {
        let now = studio.running_build();
        if build.as_str().map(str::to_string) != now {
            return Some(format!(
                "stale: observed in build {build}, but this is {}",
                now.as_deref().unwrap_or("a development build")
            ));
        }
    }
    if let Some(seen) = body["observed"].as_u64()
        && seen < studio.control.screen_revision
    {
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

/// A control on screen by id, or else by label (the topmost of those).
fn control_named<'a>(drawn: &'a [Drawn], name: &str) -> Option<&'a Drawn> {
    drawn
        .iter()
        .rev()
        .find(|d| d.control.id == name)
        .or_else(|| drawn.iter().rev().find(|d| d.control.label == name))
}

fn holds(studio: &Studio, condition: &Condition) -> bool {
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
            Some(d) if !condition.enabled || d.control.enabled => {}
            _ => return false,
        }
    }
    if let Some(text) = &condition.status_contains
        && !studio.status.contains(text.as_str())
    {
        return false;
    }
    if condition.idle
        && (studio.conversation.running()
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
    Text(String),
    Scroll(gpui::Point<gpui::Pixels>, f32),
    Command(CommandId),
    Select(agq_language::ElementId),
    Open(std::path::PathBuf),
    Wait(Condition, Instant),
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
    // The next action, unless the gate holds it.
    let next = studio.update(cx, |studio, _| {
        if studio.control.gate == Gate::Pause {
            return None;
        }
        let request = studio.control.waiting.pop_front()?;
        if studio.control.gate == Gate::Step {
            studio.control.gate = Gate::Pause;
        }
        Some(request)
    });
    if let Some(request) = next {
        changed = true;
        let started = studio.act(cx, |studio| start(studio, request));
        if let Err(refused) = started {
            let (request, answer, agent, what) = *refused;
            studio.update(cx, |studio, _| {
                studio.control.record(&agent, &what, false, answer.clone())
            });
            request.reply.send(answer);
        }
    }
    changed
}

/// Answers a request that does not act.
fn answer(studio: &mut Studio, body: &Value, window: &gpui::Window) -> Value {
    match body["op"].as_str().unwrap_or_default() {
        "hello" => json!({ "ok": true, "identity": identity(studio) }),
        "observe" => {
            let mut v = observe(studio, window, body["detail"] == "full");
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
                let at = d.bounds.center();
                marked = Some((d.bounds, d.control.label.to_string()));
                steps.push_back(Step::Press(at));
                steps.push_back(Step::Release(at));
            }
            Err(error) => return refuse(request, error, what),
        },
        Action::Fill(name, text) => match find(name) {
            Ok(d) => {
                let at = d.bounds.center();
                marked = Some((d.bounds, d.control.label.to_string()));
                steps.push_back(Step::Press(at));
                steps.push_back(Step::Release(at));
                steps.push_back(Step::Keys(vec!["ctrl-a".into(), "backspace".into()]));
                steps.push_back(Step::Text(text.clone()));
            }
            Err(error) => return refuse(request, error, what),
        },
        Action::Scroll(name, dy) => match find(name) {
            Ok(d) => steps.push_back(Step::Scroll(d.bounds.center(), *dy)),
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
        Action::Type(text) => steps.push_back(Step::Text(text.clone())),
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
            steps.push_back(Step::Wait(condition.clone(), Instant::now() + *timeout));
        }
    }
    if !matches!(action, Action::Wait(..)) {
        steps.push_back(Step::Settle(3));
    }
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

fn run_step(
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
        Step::Text(text) => input::type_text(&text, window, cx),
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
        Step::Wait(condition, deadline) => {
            if !holds(studio.read(cx), &condition) {
                if Instant::now() >= deadline {
                    return Err(format!(
                        "the wait timed out (screen {}, dialog {}, status “{}”)",
                        screen(studio.read(cx)),
                        dialog_kind(studio.read(cx)).unwrap_or_else(|| "none".into()),
                        studio.read(cx).status
                    ));
                }
                active.steps.push_front(Step::Wait(condition, deadline));
            }
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
    /// Pauses agents (C-53): the control interface holds their next action,
    /// and the Assistant's running turn holds at its next tool call.
    pub fn pause_agents(&mut self) {
        self.control.gate = Gate::Pause;
        self.pause_assistant();
        self.status = "Agents pause at their next action".into();
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Lets one action (and one tool call) through, then holds again.
    pub fn step_agents(&mut self) {
        self.control.gate = Gate::Step;
        if self.assistant_paused() {
            self.step_assistant();
        }
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Lets agents go on.
    pub fn resume_agents(&mut self) {
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
}
