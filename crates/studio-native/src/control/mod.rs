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
//! - Every action and its effect go to the **event trace**, with the agent's
//!   reason (`why`), its goal and who held the window; the control an agent
//!   acts on is marked on screen with who did what, and the title bar's
//!   agents chip shows the latest action, its goal, the decision and how it
//!   ended. **Pause** holds actions (observing is never held), between the
//!   characters of typed text too; **Step** lets one action or one typed
//!   character through; **Resume** goes on; **Stop** ends the action in
//!   progress at once, drops those waiting and refuses agents' actions
//!   until Resume. Each is the Operator's.
//! - **A test instance** (`--test-instance`: started by the Orchestrator,
//!   with its own app data and no work of the Operator's): agents may also
//!   use the Conversation (focus the composer, type, send, answer, stop the
//!   Assistant's turn, expand cards, scroll) and undo and redo, so
//!   exploration can test them as a user would. Everything else that is the
//!   Operator's stays refused there too.
//! - **One agent acts in a window at a time** (C-54): the first to act holds
//!   it until it releases it (`release`) or has been idle for [`IDLE`];
//!   another agent's action meanwhile is refused with who holds it. Observing
//!   and waiting are never refused, and the Operator's own input never meets
//!   a hold. The Orchestrator's own steps by rule in an instance it
//!   supervises through the endpoint (agent [`SUPERVISOR`], such as
//!   cancelling a dialog in its way) are the supervisor's, like the
//!   Operator's: they neither take the window nor are refused by a hold.
//! - **Observer mode** (C-54): at the speed `control.speed` (Settings, or
//!   `--control-speed`), typing appears character by character, the target
//!   is shown before a click, and clicks and scrolls are drawn ([`Speed`]).
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
/// How long an agent keeps the window after its last action ended (ROADMAP
/// §4.16): long enough for a reasoning model to think between two actions
/// (seconds to tens of seconds a step), short enough that an agent that
/// ended without releasing it does not keep others out for long.
pub const IDLE: Duration = Duration::from_secs(30);
/// The name under which the Orchestrator's own steps by rule act in an
/// instance it supervises through the endpoint.
pub const SUPERVISOR: &str = "orchestrator";
/// How long a click's ripple and a scroll's arrow are drawn.
const EFFECT: Duration = Duration::from_millis(700);

/// How fast agents' actions are carried out and shown (observer mode,
/// C-54): `control.speed` in Settings, or `--control-speed` for a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    /// At once, as before observer mode (for tests and tools).
    Instant,
    /// A typed character a frame; clicks and scrolls drawn.
    Fast,
    /// About twelve characters a second, and the target shown for a moment
    /// before a click, so the Operator can follow.
    Observe,
}

impl Speed {
    pub const NAMES: [&'static str; 3] = ["instant", "fast", "observe"];

    pub fn from_name(name: &str) -> Option<Speed> {
        match name {
            "instant" => Some(Speed::Instant),
            "fast" => Some(Speed::Fast),
            "observe" => Some(Speed::Observe),
            _ => None,
        }
    }

    /// The time between two typed characters; `None` types text at once.
    fn typing(self) -> Option<Duration> {
        match self {
            Speed::Instant => None,
            Speed::Fast => Some(Duration::ZERO),
            Speed::Observe => Some(Duration::from_millis(83)),
        }
    }

    /// How long the target is shown before the pointer presses it.
    fn dwell(self) -> Duration {
        match self {
            Speed::Observe => Duration::from_millis(300),
            _ => Duration::ZERO,
        }
    }
}

/// Why an action was not carried out, for clients to tell apart without
/// reading the words: every refused action's answer carries its `kind`
/// beside its `error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The Operator's own (§3 roles).
    OperatorOwn,
    /// Observed in another instance, project, build or session, or on a
    /// screen that changed since.
    Stale,
    /// The control is no longer on screen.
    Gone,
    Disabled,
    /// The command is not available now (the observation says why).
    Unavailable,
    /// Another agent holds the window.
    Held,
    /// The Operator stopped agents (or the agent's own turn).
    Stopped,
    /// Nobody waits for the answer any more (it was held past its deadline).
    Expired,
    /// The request cannot be carried out as written: no such action,
    /// operation, key, element or project, or not a field.
    Invalid,
    /// A wait whose condition did not come about in time.
    Timeout,
    /// It started, and a step could not be completed (the focus moved, the
    /// project did not open, the control stayed out of view).
    Failed,
}

impl Refusal {
    pub fn name(self) -> &'static str {
        match self {
            Refusal::OperatorOwn => "operator-own",
            Refusal::Stale => "stale",
            Refusal::Gone => "gone",
            Refusal::Disabled => "disabled",
            Refusal::Unavailable => "unavailable",
            Refusal::Held => "held",
            Refusal::Stopped => "stopped",
            Refusal::Expired => "expired",
            Refusal::Invalid => "invalid",
            Refusal::Timeout => "timeout",
            Refusal::Failed => "failed",
        }
    }

    /// The answer: not done, of this kind, and why.
    pub fn answer(self, error: impl Into<String>) -> Value {
        json!({ "ok": false, "kind": self.name(), "error": error.into() })
    }
}

/// A step that could not be carried out, and why.
type Failure = (Refusal, String);

/// Who asked for an action, and for what: the agent, the reason it gave
/// (its decision) and the goal it serves.
#[derive(Clone, Debug, Default, PartialEq)]
struct Who {
    agent: String,
    why: String,
    goal: String,
}

impl Who {
    fn of(body: &Value) -> Who {
        let text = |field: &str| body[field].as_str().unwrap_or_default().trim().to_string();
        Who {
            agent: body["agent"].as_str().unwrap_or("an agent").to_string(),
            why: text("why"),
            goal: text("goal"),
        }
    }
}

/// The agent holding the window, since its last action.
#[derive(Clone, Debug)]
struct Lease {
    agent: String,
    last: Instant,
}

/// What the agents chip shows of an action, while it is carried out and
/// for a few seconds after: who, its goal, its decision and how it ended.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub agent: String,
    pub goal: String,
    pub why: String,
    pub what: String,
    /// It ended (done, or refused); else it is being carried out.
    pub ended: bool,
    /// Why it was refused.
    pub refused: Option<String>,
    pub at: Instant,
}

/// Where an agent's click or scroll went, drawn for a moment.
#[derive(Clone, Debug)]
pub struct Effect {
    pub at: gpui::Point<gpui::Pixels>,
    /// None for a click; the distance for a scroll (down positive).
    pub scroll: Option<f32>,
    pub started: Instant,
}

impl Effect {
    /// How far through its time it is, from 0 to 1.
    pub fn progress(&self, now: Instant) -> f32 {
        (now.saturating_duration_since(self.started).as_secs_f32() / EFFECT.as_secs_f32()).min(1.0)
    }
}

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
    /// It came through the local endpoint, from the instance's supervisor.
    pub endpoint: bool,
}

impl Request {
    pub fn new(body: Value, reply: Reply, within: Duration) -> Request {
        Request {
            body,
            reply,
            deadline: Instant::now() + within,
            endpoint: false,
        }
    }
}

/// The gate for agents' actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Run,
    Pause,
    Step,
    /// The Operator stopped agents: their actions are refused until Resume.
    Stop,
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
    /// The reason the agent gave: its decision.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub why: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub goal: String,
    /// Who held the window then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
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
    /// The Conversation's turn alone is over (none is running).
    pub conversation_idle: bool,
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
    /// Where agents' clicks and scrolls went, while they are drawn.
    pub effects: Vec<Effect>,
    /// The latest action, for the title bar.
    pub activity: Option<(String, Instant)>,
    /// How the latest action ended, for the agents chip.
    pub last: Option<Outcome>,
    /// The agent holding the window.
    lease: Option<Lease>,
    /// The model's digest at a revision (worked out once per revision).
    digest: Option<(u64, String)>,
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
            effects: Vec::new(),
            activity: None,
            last: None,
            lease: None,
            digest: None,
            endpoint: None,
        }
    }
}

impl ControlState {
    /// Remembers that `agent` typed into the panel field `id`. A dialog's
    /// fields are read when its confirm is pressed, and the palette's text
    /// only filters it (what runs is chosen by the Enter or click that
    /// follows, whoever's it is), so neither is remembered.
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

    /// Another project: no field holds what an agent typed, and the model's
    /// digest is worked out again.
    pub fn another_project(&mut self) {
        self.typed.clear();
        self.digest = None;
    }

    /// The digest of the model's printed text (what History saves): the
    /// first 16 hex digits of its SHA-256, worked out only when the model's
    /// revision changed since it was last asked for.
    fn digest(&mut self, project: &agq_system_state::Project) -> String {
        let revision = project.state().revision();
        if let Some((at, digest)) = &self.digest
            && *at == revision
        {
            return digest.clone();
        }
        let mut text = String::new();
        for source in agq_language::print(project.state().tree()) {
            text.push_str(&source.path);
            text.push('\0');
            text.push_str(&source.text);
            text.push('\0');
        }
        let digest: String = agq_simulation::digest::text_digest(&text)
            .chars()
            .take(16)
            .collect();
        self.digest = Some((revision, digest.clone()));
        digest
    }

    /// The agent holding the window now: one whose action is in progress,
    /// or whose last action ended less than [`IDLE`] ago.
    pub fn holder(&self) -> Option<&str> {
        self.holder_at(Instant::now())
    }

    fn holder_at(&self, now: Instant) -> Option<&str> {
        let lease = self.lease.as_ref()?;
        let acting = self
            .active
            .as_ref()
            .is_some_and(|active| active.who.agent == lease.agent);
        (acting || now.saturating_duration_since(lease.last) < IDLE).then_some(lease.agent.as_str())
    }

    /// `agent` takes the window (or keeps it), unless another agent holds
    /// it: then why not.
    fn hold(&mut self, agent: &str, now: Instant) -> Result<(), String> {
        if let Some(holder) = self.holder_at(now)
            && holder != agent
        {
            return Err(format!(
                "the window is in use by {holder}; act in your own test instance, or wait"
            ));
        }
        self.lease = Some(Lease {
            agent: agent.to_string(),
            last: now,
        });
        Ok(())
    }

    /// `agent` lets the window go: whether it held it.
    pub fn release(&mut self, agent: &str) -> bool {
        let held = self.lease.as_ref().is_some_and(|l| l.agent == agent);
        if held {
            self.lease = None;
        }
        held
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
    /// waits (fail closed when its turn is stopped); it no longer holds the
    /// window.
    pub fn cancel(&mut self, agent: &str, why: &str) {
        let mut kept = VecDeque::new();
        for request in self.waiting.drain(..) {
            if request.body["op"] == "act" && request.body["agent"] == agent {
                request.reply.send(Refusal::Stopped.answer(why));
            } else {
                kept.push_back(request);
            }
        }
        self.waiting = kept;
        let (cancelled, waits): (Vec<Waiting>, Vec<Waiting>) =
            self.waits.drain(..).partition(|w| w.who.agent == agent);
        self.waits = waits;
        for wait in cancelled {
            wait.request.reply.send(Refusal::Stopped.answer(why));
        }
        self.release(agent);
    }

    /// Actions waiting, an action in progress, or a recent one (still drawn
    /// in the window).
    pub fn busy(&self) -> bool {
        self.active.is_some()
            || self.waiting.iter().any(|r| r.body["op"] == "act")
            || self
                .activity
                .as_ref()
                .is_some_and(|(_, at)| at.elapsed() < ACTIVITY)
            || self.outcome().is_some()
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

    /// How the latest action ended, while it is recent.
    pub fn outcome(&self) -> Option<&Outcome> {
        self.last.as_ref().filter(|o| o.at.elapsed() < ACTIVITY)
    }

    /// What the agents chip shows: the action being carried out, else how
    /// the latest one ended, while it is recent.
    pub fn shown(&self) -> Option<Outcome> {
        match &self.active {
            Some(active) => Some(Outcome {
                agent: active.who.agent.clone(),
                goal: active.who.goal.clone(),
                why: active.who.why.clone(),
                what: active.what.clone(),
                ended: false,
                refused: None,
                at: active.started,
            }),
            None => self.outcome().cloned(),
        }
    }

    /// Clicks and scrolls still to draw.
    pub fn live_effects(&mut self) -> &[Effect] {
        self.effects.retain(|e| e.started.elapsed() < EFFECT);
        &self.effects
    }

    fn record(&mut self, who: &Who, what: &str, ok: bool, detail: Value) {
        let event = Event {
            seq: self.next_seq,
            at: self.started.elapsed().as_millis() as u64,
            agent: who.agent.clone(),
            what: what.to_string(),
            ok,
            detail,
            why: who.why.clone(),
            goal: who.goal.clone(),
            holder: self.holder().map(str::to_string),
        };
        self.next_seq += 1;
        self.trace.push_back(event);
        while self.trace.len() > TRACE {
            self.trace.pop_front();
        }
    }

    /// An action ended (done or refused): the trace, the window's hold (its
    /// idle time starts now) and the agents chip say so.
    fn ended(&mut self, who: &Who, what: &str, answer: &Value) {
        let now = Instant::now();
        if let Some(lease) = self.lease.as_mut().filter(|l| l.agent == who.agent) {
            lease.last = now;
        }
        let ok = answer["ok"] != false;
        self.record(who, what, ok, answer.clone());
        self.last = Some(Outcome {
            agent: who.agent.clone(),
            goal: who.goal.clone(),
            why: who.why.clone(),
            what: what.to_string(),
            ended: true,
            refused: (!ok).then(|| {
                let error = answer["error"].as_str().unwrap_or("refused");
                error.strip_prefix("refused: ").unwrap_or(error).to_string()
            }),
            at: now,
        });
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

/// What agents may also do in a test instance, which holds no work of the
/// Operator's: use the Conversation, and undo and redo.
const TEST_INSTANCE_COMMANDS: [CommandId; 4] = [
    CommandId::Undo,
    CommandId::Redo,
    CommandId::AskAssistant,
    CommandId::InsertSelection,
];

/// The Conversation's controls that stay the Operator's in a test instance
/// too: steering the Assistant's turn (Pause, Step and Resume are the
/// supervisor's), retrying or editing a message, a new conversation, and
/// the model (as Settings).
const CONVERSATION_OPERATORS: [&str; 6] = [
    "pause",
    "resume",
    "step",
    "retry",
    "new-conversation",
    "model-picker",
];

/// Whether `id` is the Operator's to use here.
pub fn operators_command(studio: &Studio, id: CommandId) -> bool {
    OPERATORS_COMMANDS.contains(&id)
        && !(studio.args.test_instance && TEST_INSTANCE_COMMANDS.contains(&id))
}

/// Whether the region `region` is the Operator's here.
fn operators_region(studio: &Studio, region: &str) -> bool {
    OPERATORS_REGIONS.contains(&region) && !(studio.args.test_instance && region == "conversation")
}

/// The agents chip: pausing, stopping and resuming agents is the Operator's.
const OPERATORS_CONTROLS: [&str; 4] = [
    "agents-pause",
    "agents-step",
    "agents-resume",
    "agents-stop",
];

/// Buttons outside the Operator's regions that do what one of the
/// Operator's commands does (its effect is refused where it happens).
const BUTTONS_FOR_COMMANDS: [(&str, CommandId); 2] = [
    ("inspector-lock", CommandId::Lock),
    ("trust-local", CommandId::TrustLocal),
];

/// The command a control runs, by its control id: a palette row or a menu
/// item (`palette-lock`, `menu-lock`), or a button that does what a command
/// does.
fn runs_command(id: &str) -> Option<CommandId> {
    BUTTONS_FOR_COMMANDS
        .iter()
        .find(|(button, _)| *button == id)
        .map(|(_, command)| *command)
        .or_else(|| {
            id.strip_prefix("palette-")
                .or_else(|| id.strip_prefix("menu-"))
                .and_then(command_by_name)
        })
}

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
        operators_region(studio, d.region).then(|| {
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
        Action::Command(id) if operators_command(studio, *id) => {
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
            // The window's own buttons answer the operating system's
            // pointer, which an agent's input does not use.
            if d.control.id.starts_with("window-") {
                return Some("minimizing, maximizing and closing the window are the Operator's".into());
            }
            if let Some(id) = runs_command(&d.control.id)
                && operators_command(studio, id)
            {
                return Some(format!("`{}` is the Operator's to use", command_name(id)));
            }
            if d.region == "conversation"
                && (CONVERSATION_OPERATORS.contains(&d.control.id.as_ref())
                    || d.control.id.starts_with("edit-")
                    || d.control.id.starts_with("model-"))
            {
                return Some(format!(
                    "`{}` in the Conversation is the Operator's",
                    d.control.label
                ));
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
                operators_command(studio, id)
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

/// Whether agents may not act on the control `d` now, by the same rules
/// that refuse an agent's click on it (or fill, for a field).
fn operator_only(studio: &Studio, d: &Drawn) -> bool {
    let action = if d.control.role == "field" {
        Action::Fill(d.control.id.to_string(), String::new())
    } else {
        Action::Click(d.control.id.to_string())
    };
    operators_only(studio, &action, std::slice::from_ref(d)).is_some()
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

fn control_json(studio: &Studio, d: &Drawn) -> Value {
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
    if operator_only(studio, d) {
        v["operatorOnly"] = json!(true);
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

/// `text`'s first `limit` characters, and an ellipsis when there were more.
fn bounded(text: &str, limit: usize) -> String {
    let mut short: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        short.push('…');
    }
    short
}

/// The tool calls of the turn running, or else of the last one (since the
/// last message to the Assistant): each tool and how it stands, a failure
/// with the start of its message; the last 12, and how many earlier ones
/// were left out.
fn turn_tools(panel: &crate::conversation::ConversationPanel) -> (Vec<Value>, usize) {
    const SHOWN: usize = 12;
    let entries = &panel.conversation.entries;
    let since = entries
        .iter()
        .rposition(|e| matches!(e, agq_assistant::Entry::Operator { .. }))
        .map_or(0, |at| at + 1);
    let mut calls: Vec<(&str, &str)> = Vec::new();
    for entry in &entries[since..] {
        if let agq_assistant::Entry::Assistant { parts, .. } = entry {
            for part in parts {
                if let agq_providers::AssistantPart::ToolCall { id, name, .. } = part {
                    calls.push((id, name));
                }
            }
        }
    }
    for live in &panel.live {
        if let crate::conversation::Live::Tool { id, name, .. } = live {
            calls.push((id, name));
        }
    }
    let omitted = calls.len().saturating_sub(SHOWN);
    let listed: Vec<Value> = calls[omitted..]
        .iter()
        .map(|(id, name)| match panel.results.get(*id) {
            Some(result) if result.is_error => json!({
                "tool": name, "state": "failed", "error": bounded(&result.content, 160),
            }),
            Some(_) => json!({ "tool": name, "state": "done" }),
            None if panel.running() => json!({ "tool": name, "state": "running" }),
            None => json!({ "tool": name, "state": "not run" }),
        })
        .collect();
    (listed, omitted)
}

/// The Conversation's notices since the last message (errors, a stop, a
/// steering note): the last five, bounded.
fn notices(panel: &crate::conversation::ConversationPanel) -> Vec<String> {
    let entries = &panel.conversation.entries;
    let since = entries
        .iter()
        .rposition(|e| matches!(e, agq_assistant::Entry::Operator { .. }))
        .map_or(0, |at| at + 1);
    let all: Vec<String> = entries[since..]
        .iter()
        .filter_map(|e| match e {
            agq_assistant::Entry::Notice { text } => Some(bounded(text, 300)),
            _ => None,
        })
        .collect();
    all[all.len().saturating_sub(5)..].to_vec()
}

/// Why the Conversation's last turn failed (its last notice), or why the
/// conversation could not be read or saved; none when nothing went wrong.
/// A missing key is `keyMissing`.
fn conversation_error(panel: &crate::conversation::ConversationPanel) -> Option<String> {
    if panel.can_retry()
        && let Some(last) = notices(panel).pop()
    {
        return Some(last);
    }
    panel
        .read_error
        .as_deref()
        .or(panel.save_error.as_deref())
        .map(|e| bounded(e, 300))
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
        .map(|d| control_json(studio, d))
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
            if operators_only(studio, &Action::Command(c.id), &[]).is_some() {
                v["operatorOnly"] = json!(true);
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
    let (tool_calls, tools_omitted) = turn_tools(panel);
    let last_message = panel
        .conversation
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            agq_assistant::Entry::Operator { text } => Some(bounded(text, 200)),
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
            "lastMessage": last_message,
            "lastReply": last_reply,
            // The current or last turn's tool calls, and what stands in
            // the way (no key, an error), so an agent that asked can tell
            // whether it was answered.
            "toolCalls": tool_calls,
            "toolCallsOmitted": tools_omitted,
            "notices": notices(panel),
            "error": conversation_error(panel),
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
            // The agent holding the window, if any.
            "holder": studio.control.holder(),
            "speed": format!("{:?}", studio.control_speed()).to_lowercase(),
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

/// A whole number, as the tools' input check reads `integer`: `7`, or `7.0`.
fn whole(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|n| n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64)
            .map(|n| n as u64)
    })
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
                conversation_idle: until["conversationIdle"] == true,
            };
            let ms = whole(&action["timeoutMs"]).unwrap_or(10_000).min(600_000);
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
    let Some(seen) = whole(&body["observed"]) else {
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
    if condition.conversation_idle && studio.conversation.running() {
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
    /// One character into the field `field`, which must still have the
    /// focus: observer mode types text a character at a time, and Pause
    /// holds before each.
    Char {
        field: String,
        c: char,
    },
    /// Frames for the Studio to show the effect.
    Settle(u32),
}

struct Active {
    request: Request,
    who: Who,
    what: String,
    steps: VecDeque<Step>,
    started: Instant,
    speed: Speed,
    /// The next step waits until then (observer mode's pace).
    not_before: Option<Instant>,
    /// It started on a Step: its first typed character goes through too.
    stepped: bool,
}

struct Waiting {
    request: Request,
    who: Who,
    what: String,
    condition: Condition,
    until: Instant,
}

/// What the workspace does with the control interface each tick: takes new
/// requests, answers observations at once, and carries out one step of one
/// action (held while paused, between typed characters too).
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
            let who = Who::of(&request.body);
            let answer = Refusal::Expired.answer("the request waited past its deadline (agents were paused) and was dropped; observe again");
            studio
                .control
                .ended(&who, "a request that waited too long", &answer);
            request.reply.send(answer);
        }
        // Stopped by the Operator: actions are refused (waiting for a
        // condition is observing, and goes on).
        if studio.control.gate == Gate::Stop {
            let (stopped, kept): (Vec<Request>, Vec<Request>) =
                studio.control.waiting.drain(..).partition(|r| {
                    r.body["op"] == "act" && r.body["action"]["kind"] != "wait"
                });
            studio.control.waiting = kept.into();
            for request in stopped {
                let who = Who::of(&request.body);
                let what = parse_action(&request.body)
                    .map(|action| describe(&action))
                    .unwrap_or_else(|_| "an action".into());
                let answer = Refusal::Stopped.answer("refused: stopped by the Operator");
                studio.control.ended(&who, &what, &answer);
                request.reply.send(answer);
            }
        }
        // Waits, beside any action.
        let waits = std::mem::take(&mut studio.control.waits);
        for wait in waits {
            if holds(studio, &wait.condition, &wait.who.agent) {
                let answer = json!({
                    "ok": true,
                    "did": wait.what,
                    "screen": screen(studio),
                    "screenRevision": studio.control.screen_revision,
                    "dialog": dialog_kind(studio),
                    "status": studio.status,
                });
                studio.control.record(&wait.who, &wait.what, true, answer.clone());
                wait.request.reply.send(answer);
            } else if now >= wait.until {
                let answer = Refusal::Timeout.answer(format!(
                    "the wait timed out (screen {}, dialog {}, status “{}”)",
                    screen(studio),
                    dialog_kind(studio).unwrap_or_else(|| "none".into()),
                    studio.status
                ));
                studio.control.record(&wait.who, &wait.what, false, answer.clone());
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
        let gate = studio.read(cx).control.gate;
        if gate == Gate::Stop {
            // Ended at once. A press is released away from its target, so
            // no button stays down and the press does not become a click.
            if let Some(Step::Release(_)) = active.steps.front() {
                input::click(
                    gpui::point(gpui::px(-1.0), gpui::px(-1.0)),
                    false,
                    window,
                    cx,
                );
            }
            let answer = Refusal::Stopped.answer("refused: stopped by the Operator");
            studio.update(cx, |studio, _| {
                studio.control.acting = None;
                studio.control.refused = None;
                studio.control.ended(&active.who, &active.what, &answer);
            });
            active.request.reply.send(answer);
            return changed;
        }
        // Observer mode's pace.
        let early = active.not_before.is_some_and(|at| Instant::now() < at);
        // Pause holds before a typed character; Step lets one through.
        let held = !early
            && matches!(active.steps.front(), Some(Step::Char { .. }))
            && match gate {
                Gate::Pause => !std::mem::take(&mut active.stepped),
                Gate::Step => {
                    studio.update(cx, |studio, _| studio.control.gate = Gate::Pause);
                    false
                }
                Gate::Run | Gate::Stop => false,
            };
        if early || held {
            studio.update(cx, |studio, _| studio.control.active = Some(active));
            return changed;
        }
        match active.steps.pop_front() {
            None => {
                let summary = studio.update(cx, |studio, _| {
                    studio.control.acting = None;
                    let answer = match studio.control.refused.take() {
                        Some(reason) => Refusal::OperatorOwn.answer(format!("refused: {reason}")),
                        None => json!({
                            "ok": true,
                            "did": active.what,
                            "screen": screen(studio),
                            "screenRevision": studio.control.screen_revision,
                            "dialog": dialog_kind(studio),
                            "status": studio.status,
                            "selection": studio.selection.elements(&studio.scene).into_iter().map(|id| element_name(studio, id)).collect::<Vec<_>>(),
                            "tookMs": active.started.elapsed().as_millis() as u64,
                        }),
                    };
                    studio.control.ended(&active.who, &active.what, &answer);
                    answer
                });
                active.request.reply.send(summary);
            }
            Some(step) => {
                if matches!(step, Step::Char { .. }) {
                    active.stepped = false;
                }
                let outcome = run_step(studio, step, &mut active, window, cx);
                match outcome {
                    Ok(()) => studio.update(cx, |studio, _| studio.control.active = Some(active)),
                    Err((kind, error)) => {
                        let answer = kind.answer(error);
                        studio.update(cx, |studio, _| {
                            studio.control.acting = None;
                            studio.control.refused = None;
                            studio.control.ended(&active.who, &active.what, &answer)
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
                    if let Some(active) = studio.control.active.as_mut() {
                        active.stepped = true;
                    }
                }
            }),
            Err(refused) => {
                let (request, answer, who, what) = *refused;
                studio.update(cx, |studio, _| studio.control.ended(&who, &what, &answer));
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
            let digest = studio
                .project
                .as_ref()
                .map(|project| studio.control.digest(project));
            let mut v = observe(
                studio,
                window,
                body["detail"] == "full",
                body["region"].as_str(),
            );
            if let Some(digest) = digest {
                v["project"]["digest"] = json!(digest);
            }
            v["ok"] = json!(true);
            v
        }
        "events" => {
            let since = whole(&body["since"]).unwrap_or(0);
            json!({ "ok": true, "events": studio.control.events(since) })
        }
        // The agent lets the window go before its idle time is up.
        "release" => {
            let who = Who::of(body);
            match studio.control.holder() {
                Some(holder) if holder != who.agent => Refusal::Held.answer(format!(
                    "the window is held by {holder}, not by {}",
                    who.agent
                )),
                _ => {
                    let released = studio.control.release(&who.agent);
                    if released {
                        studio
                            .control
                            .record(&who, "release the window", true, json!({}));
                    }
                    json!({ "ok": true, "released": released })
                }
            }
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
            Some("stop") => {
                studio.stop_agents();
                json!({ "ok": true, "gate": "stop" })
            }
            _ => Refusal::Invalid.answer("`mode` is pause, step, run or stop"),
        },
        other => Refusal::Invalid.answer(format!(
            "there is no operation `{other}`: hello, observe, act, release, events or gate"
        )),
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

type Refused = Box<(Request, Value, Who, String)>;

/// Checks an action and plans its steps. A stale or impossible action is
/// refused before anything happens, and so is one by an agent while
/// another holds the window.
fn start(studio: &mut Studio, request: Request) -> Result<(), Refused> {
    let who = Who::of(&request.body);
    let refuse = |request: Request, (kind, error): Failure, what: String| {
        Err(Box::new((request, kind.answer(error), who.clone(), what)))
    };
    let action = match parse_action(&request.body) {
        Ok(action) => action,
        Err(error) => {
            let what = "an action that could not be read".into();
            return refuse(request, (Refusal::Invalid, error), what);
        }
    };
    let what = describe(&action);
    // One agent at a time; the supervisor's own steps by rule and waiting
    // (which only observes) neither take the window nor are refused.
    let supervisor = request.endpoint && who.agent == SUPERVISOR;
    let holds_window = !supervisor && !matches!(action, Action::Wait(..));
    let now = Instant::now();
    if holds_window
        && let Some(holder) = studio.control.holder_at(now)
        && holder != who.agent
    {
        let error = format!(
            "refused: the window is in use by {holder}; act in your own test instance, or wait"
        );
        return refuse(request, (Refusal::Held, error), what);
    }
    if let Some(reason) = stale(studio, &request.body) {
        return refuse(request, (Refusal::Stale, reason), what);
    }
    let drawn = target::drawn();
    if let Some(reason) = operators_only(studio, &action, &drawn) {
        let error = format!("refused: {reason}");
        return refuse(request, (Refusal::OperatorOwn, error), what);
    }
    let find = |name: &str| -> Result<&Drawn, Failure> {
        match control_named(&drawn, name) {
            None => Err((
                Refusal::Gone,
                format!("stale: no control `{name}` is on screen now; observe again"),
            )),
            Some(d) if !d.control.enabled => Err((
                Refusal::Disabled,
                format!("`{}` is disabled now", d.control.label),
            )),
            Some(d) => Ok(d),
        }
    };
    let mut steps = VecDeque::new();
    let mut marked = None;
    match &action {
        Action::Command(id) => {
            if let Some(reason) = unavailable(*id, &studio.context()) {
                let error = format!("`{}` is not available: {reason}", command_name(*id));
                return refuse(request, (Refusal::Unavailable, error), what);
            }
            steps.push_back(Step::Command(*id));
        }
        Action::Click(name) => match find(name) {
            Ok(d) => {
                marked = Some(d.shown.unwrap_or(d.clip));
                steps.push_back(Step::Reveal(name.clone(), 6));
            }
            Err(failure) => return refuse(request, failure, what),
        },
        Action::Fill(name, text) => match find(name) {
            Ok(d) if d.control.role != "field" => {
                let error = format!(
                    "`{}` is a {}, not a field: click it instead",
                    d.control.label, d.control.role
                );
                return refuse(request, (Refusal::Invalid, error), what);
            }
            Ok(d) => {
                marked = Some(d.shown.unwrap_or(d.clip));
                steps.push_back(Step::Reveal(name.clone(), 6));
                steps.push_back(Step::Fill(name.clone(), text.clone(), 10));
            }
            Err(failure) => return refuse(request, failure, what),
        },
        Action::Scroll(name, dy) => match find(name) {
            Ok(d) => steps.push_back(Step::Scroll(d.shown.unwrap_or(d.clip).center(), *dy)),
            Err(failure) => return refuse(request, failure, what),
        },
        Action::Key(keys) => {
            for key in keys {
                if gpui::Keystroke::parse(key).is_err() {
                    let error = format!(
                        "`{key}` is not a key (GPUI syntax: ctrl-s, escape, enter, shift-1)"
                    );
                    return refuse(request, (Refusal::Invalid, error), what);
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
                    let error = format!("there is no element `{name}` in the open model");
                    return refuse(request, (Refusal::Invalid, error), what);
                }
            }
        }
        Action::OpenProject(folder) => {
            if who.agent == "Assistant" {
                let error = "refused: opening another project ends your own turn; ask the Operator to open it".into();
                return refuse(request, (Refusal::OperatorOwn, error), what);
            }
            if let Some(reason) = unavailable(CommandId::OpenProject, &studio.context()) {
                let error = format!("a project cannot be opened now: {reason}");
                return refuse(request, (Refusal::Unavailable, error), what);
            }
            if !folder.join("model").is_dir() {
                let error = format!(
                    "{} is not an Agentique project (no model folder)",
                    folder.display()
                );
                return refuse(request, (Refusal::Invalid, error), what);
            }
            steps.push_back(Step::Open(folder.clone()));
        }
        Action::Wait(condition, timeout) => {
            // Beside any action, until it holds or times out.
            studio.control.waits.push(Waiting {
                request,
                who,
                what,
                condition: condition.clone(),
                until: Instant::now() + *timeout,
            });
            return Ok(());
        }
    }
    steps.push_back(Step::Settle(3));
    let shown = match &who.why {
        why if why.is_empty() => format!("{}: {what}", who.agent),
        why => format!("{}: {what} — {why}", who.agent),
    };
    if let Some(bounds) = marked {
        studio.control.marks.push(Mark {
            bounds,
            label: shown.clone(),
            until: Instant::now() + MARK,
        });
    }
    if holds_window {
        // Free (checked above), or already this agent's.
        let _ = studio.control.hold(&who.agent, now);
    }
    studio.control.activity = Some((shown, Instant::now()));
    studio.mark(crate::studio::Dirty::STATUS | crate::studio::Dirty::OVERLAY);
    studio.control.refused = None;
    let speed = studio.control_speed();
    studio.control.active = Some(Active {
        request,
        who,
        what,
        steps,
        started: Instant::now(),
        speed,
        not_before: None,
        stepped: false,
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
) -> Result<(), Failure> {
    // A dialog asking for the Operator's approval may have opened since the
    // action started: its own controls are not an agent's either.
    if matches!(
        step,
        Step::Press(..)
            | Step::Release(..)
            | Step::Keys(..)
            | Step::Type(..)
            | Step::Fill(..)
            | Step::Char { .. }
    ) && let Some(kind) = approval(studio.read(cx))
    {
        return Err((
            Refusal::OperatorOwn,
            format!("refused: the {kind} dialog opened; it asks for the Operator's own approval"),
        ));
    }
    let agent = active.who.agent.clone();
    studio.update(cx, |studio, _| studio.control.acting = Some(agent));
    let result = carry_out(studio, step, active, window, cx);
    let clear = studio.clone();
    cx.defer(move |cx| clear.update(cx, |studio, _| studio.control.acting = None));
    result
}

/// The text's characters as steps, before the action's next steps, at the
/// action's pace; or, at `instant`, typed now.
fn type_into(
    field: &str,
    text: &str,
    active: &mut Active,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    if active.speed.typing().is_none() {
        input::type_text(text, window, cx);
        return;
    }
    for c in text.chars().rev() {
        active.steps.push_front(Step::Char {
            field: field.to_string(),
            c,
        });
    }
}

fn carry_out(
    studio: &gpui::Entity<Studio>,
    step: Step,
    active: &mut Active,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> Result<(), Failure> {
    let press = |key: &str, window: &mut gpui::Window, cx: &mut gpui::App| {
        input::press(key, window, cx).map_err(|error| (Refusal::Invalid, error))
    };
    use crate::workspace::StudioExt;
    let drawn_effect = |studio: &gpui::Entity<Studio>, cx: &mut gpui::App, effect: Effect| {
        studio.update(cx, |studio, _| studio.control.effects.push(effect))
    };
    match step {
        Step::Press(at) => {
            input::click(at, true, window, cx);
            if active.speed != Speed::Instant {
                drawn_effect(
                    studio,
                    cx,
                    Effect {
                        at,
                        scroll: None,
                        started: Instant::now(),
                    },
                );
            }
        }
        Step::Release(at) => input::click(at, false, window, cx),
        Step::Keys(keys) => {
            for key in keys {
                press(&key, window, cx)?;
            }
        }
        Step::Scroll(at, dy) => {
            input::scroll(at, dy, window, cx);
            if active.speed != Speed::Instant {
                drawn_effect(
                    studio,
                    cx,
                    Effect {
                        at,
                        scroll: Some(dy),
                        started: Instant::now(),
                    },
                );
            }
        }
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
                return Err((
                    Refusal::Failed,
                    format!("the project did not open: {}", studio.read(cx).status),
                ));
            }
        }
        Step::Reveal(name, tries) => {
            let drawn = target::drawn();
            let Some(d) = control_named(&drawn, &name) else {
                return Err((Refusal::Gone, format!("`{name}` is no longer on screen")));
            };
            match d.shown {
                Some(shown) => {
                    let at = shown.center();
                    active.steps.push_front(Step::Release(at));
                    active.steps.push_front(Step::Press(at));
                    // Observer mode: the target, ringed and labelled, a
                    // moment before the press.
                    let dwell = active.speed.dwell();
                    if !dwell.is_zero() {
                        active.not_before = Some(Instant::now() + dwell);
                    }
                    studio.update(cx, |studio, _| {
                        let label = studio
                            .control
                            .activity
                            .as_ref()
                            .map(|(text, _)| text.clone())
                            .unwrap_or_default();
                        studio.control.marks.retain(|m| m.label != label);
                        studio.control.marks.push(Mark {
                            bounds: shown,
                            label,
                            until: Instant::now() + dwell + MARK,
                        });
                    });
                }
                None if tries > 0 => {
                    // Scroll its panel by the distance to it, as the
                    // Operator would, and look again next frame.
                    let dy = f32::from(d.bounds.center().y - d.clip.center().y);
                    input::scroll(d.clip.center(), dy, window, cx);
                    if active.speed != Speed::Instant {
                        drawn_effect(
                            studio,
                            cx,
                            Effect {
                                at: d.clip.center(),
                                scroll: Some(dy),
                                started: Instant::now(),
                            },
                        );
                    }
                    window.refresh();
                    active.steps.push_front(Step::Reveal(name, tries - 1));
                }
                None => {
                    return Err((
                        Refusal::Failed,
                        format!(
                            "`{name}` stays outside the visible part of its panel; scroll it into view"
                        ),
                    ));
                }
            }
        }
        Step::Fill(name, text, tries) => {
            let drawn = target::drawn();
            let field = drawn.iter().find(|d| {
                d.control.focused
                    && d.control.role == "field"
                    && (d.control.id == name.as_str() || d.control.label == name.as_str())
            });
            let Some(field) = field else {
                if tries == 0 {
                    return Err((
                        Refusal::Failed,
                        format!("`{name}` did not take the focus, so nothing was typed"),
                    ));
                }
                window.refresh();
                active.steps.push_front(Step::Fill(name, text, tries - 1));
                return Ok(());
            };
            let (id, region) = (field.control.id.to_string(), field.region);
            press("ctrl-a", window, cx)?;
            press("backspace", window, cx)?;
            type_into(&id, &text, active, window, cx);
            let agent = active.who.agent.clone();
            studio.update(cx, |studio, _| {
                studio.control.mark_typed(&id, region, &agent)
            });
        }
        Step::Type(text) => {
            let drawn = target::drawn();
            let Some(field) = drawn.iter().rev().find(|d| d.control.focused) else {
                return Err((
                    Refusal::Failed,
                    "no field has the focus, so nothing was typed".into(),
                ));
            };
            if field.control.role != "field" {
                let error = format!("`{}` is not a field", field.control.label);
                return Err((Refusal::Invalid, error));
            }
            if operators_region(studio.read(cx), field.region) {
                let error = format!(
                    "`{}` is not a field an agent may type into",
                    field.control.label
                );
                return Err((Refusal::OperatorOwn, error));
            }
            let id = field.control.id.to_string();
            let region = field.region;
            type_into(&id, &text, active, window, cx);
            let agent = active.who.agent.clone();
            studio.update(cx, |studio, _| {
                studio.control.mark_typed(&id, region, &agent)
            });
        }
        Step::Char { field, c } => {
            // Only into the field the text was meant for: the Operator may
            // have moved the focus meanwhile.
            let drawn = target::drawn();
            let focused = drawn.iter().rev().find(|d| d.control.focused);
            let Some(d) = focused.filter(|d| d.control.role == "field" && d.control.id == field)
            else {
                let left = 1 + active
                    .steps
                    .iter()
                    .filter(|s| matches!(s, Step::Char { .. }))
                    .count();
                return Err((
                    Refusal::Failed,
                    format!(
                        "the focus left `{field}` while typing, so the last {left} character(s) were not typed"
                    ),
                ));
            };
            let bounds = d.shown.unwrap_or(d.bounds);
            if c == '\n' {
                press("enter", window, cx)?;
            } else {
                input::type_char(c, window, cx);
            }
            // The field typed into stays ringed while the text goes in.
            let pace = active.speed.typing().unwrap_or_default();
            studio.update(cx, |studio, _| {
                let label = studio
                    .control
                    .activity
                    .as_ref()
                    .map(|(text, _)| text.clone())
                    .unwrap_or_default();
                studio.control.marks.retain(|m| m.label != label);
                studio.control.marks.push(Mark {
                    bounds,
                    label,
                    until: Instant::now() + pace + MARK,
                });
            });
            if !pace.is_zero() {
                active.not_before = Some(Instant::now() + pace);
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

    /// [`Studio::refused_to_agents`], in the Operator's own window only: in
    /// a test instance an agent may do `what` (the Conversation, undo).
    pub fn refused_in_operators_window(&mut self, what: &str) -> bool {
        !self.args.test_instance && self.refused_to_agents(what)
    }

    /// How fast agents' actions are carried out and shown: this process's
    /// `--control-speed`, else the setting.
    pub fn control_speed(&self) -> Speed {
        self.args
            .control_speed
            .as_deref()
            .and_then(Speed::from_name)
            .or_else(|| Speed::from_name(&self.settings.text("control.speed")))
            .unwrap_or(Speed::Observe)
    }

    /// Pauses agents (C-53): the control interface holds their next action
    /// (or typed character), and the Assistant's running turn holds at its
    /// next tool call.
    pub fn pause_agents(&mut self) {
        if self.refused_to_agents("pausing agents") {
            return;
        }
        self.control.gate = Gate::Pause;
        self.pause_assistant();
        self.status = "Agents pause at their next action".into();
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Lets one action, or one typed character (and one tool call), through,
    /// then holds again.
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

    /// Lets agents go on (after a pause or a stop).
    pub fn resume_agents(&mut self) {
        if self.refused_to_agents("resuming agents") {
            return;
        }
        if self.control.gate == Gate::Stop {
            self.status = "Agents may act again".into();
        }
        self.control.gate = Gate::Run;
        self.resume_assistant();
        self.mark(crate::studio::Dirty::STATUS);
    }

    /// Stops agents' actions (C-54): the one in progress ends at once, those
    /// waiting are dropped, and further ones are refused until Resume.
    pub fn stop_agents(&mut self) {
        if self.refused_to_agents("stopping agents") {
            return;
        }
        self.control.gate = Gate::Stop;
        self.status = "Agents stopped: their actions are refused until you resume them".into();
        self.mark(crate::studio::Dirty::STATUS | crate::studio::Dirty::OVERLAY);
    }

    /// Whether agents are held (or will be at their next action), or stopped.
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
            layer: 0,
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
        app.control.another_project();
        assert!(!app.control.begin_typed_commit(Some("Guard")));
        // A dialog's fields are read when its confirm is pressed: not marked.
        app.control.mark_typed("Name", "dialog", "evaluator");
        assert!(!app.control.begin_typed_commit(Some("Name")));
    }

    /// An action request as the endpoint or a tool sends it, against the
    /// Studio's first screen.
    fn act(app: &Studio, agent: &str, action: Value, endpoint: bool) -> Request {
        let (reply, _) = std::sync::mpsc::channel();
        let mut request = Request::new(
            json!({
                "op": "act", "agent": agent, "why": "a reason", "goal": "a goal",
                "expect": { "instance": app.control.instance },
                "observed": app.control.screen_revision,
                "action": action,
            }),
            Reply::Channel(reply),
            Duration::from_secs(5),
        );
        request.endpoint = endpoint;
        request
    }

    fn refusal(result: Result<(), Refused>) -> Option<String> {
        result
            .err()
            .map(|refused| refused.1["error"].as_str().unwrap_or_default().to_string())
    }

    #[test]
    fn one_agent_holds_the_window_until_it_releases_it_or_is_idle() {
        let mut control = ControlState::default();
        let t0 = Instant::now();
        assert_eq!(control.hold("explorer", t0), Ok(()));
        assert_eq!(control.holder_at(t0), Some("explorer"));
        // Another agent meanwhile is refused with who holds the window.
        let refused = control.hold("evaluator", t0 + Duration::from_secs(5));
        assert_eq!(
            refused,
            Err("the window is in use by explorer; act in your own test instance, or wait".into())
        );
        // The holder acting again keeps it, and its idle time starts again.
        assert_eq!(
            control.hold("explorer", t0 + Duration::from_secs(20)),
            Ok(())
        );
        assert_eq!(
            control.holder_at(t0 + Duration::from_secs(45)),
            Some("explorer")
        );
        // Idle for IDLE: the window is free.
        let later = t0 + Duration::from_secs(20) + IDLE;
        assert_eq!(control.holder_at(later), None);
        assert_eq!(control.hold("evaluator", later), Ok(()));
        // Only the holder releases it.
        assert!(!control.release("explorer"));
        assert!(control.release("evaluator"));
        assert_eq!(control.holder_at(later), None);
    }

    #[test]
    fn an_agent_is_refused_while_another_holds_the_window_but_waiting_and_the_supervisor_are_not() {
        let (mut app, _folder) = crate::edit::app_tests::studio("window-hold");
        let graph = json!({ "kind": "command", "id": "graph" });
        let request = act(&app, "explorer", graph.clone(), true);
        assert_eq!(refusal(start(&mut app, request)), None);
        assert_eq!(app.control.holder(), Some("explorer"));
        // The explorer's action ends; it still holds the window.
        app.control.active = None;
        assert_eq!(app.control.holder(), Some("explorer"));
        let request = act(&app, "evaluator", graph.clone(), true);
        let refused = refusal(start(&mut app, request)).expect("refused");
        assert_eq!(
            refused,
            "refused: the window is in use by explorer; act in your own test instance, or wait"
        );
        // Waiting only observes.
        let wait = json!({ "kind": "wait", "until": { "dialog": null }, "timeoutMs": 10 });
        let request = act(&app, "evaluator", wait, true);
        assert_eq!(refusal(start(&mut app, request)), None);
        // The supervisor's own step by rule, through the endpoint, neither
        // waits for the window nor takes it; the same name from inside the
        // Studio is an agent like any other.
        let request = act(&app, SUPERVISOR, graph.clone(), true);
        assert_eq!(refusal(start(&mut app, request)), None);
        app.control.active = None;
        assert_eq!(app.control.holder(), Some("explorer"));
        let request = act(&app, SUPERVISOR, graph.clone(), false);
        assert!(refusal(start(&mut app, request)).is_some());
        // An agent whose turn is stopped lets the window go.
        app.control.cancel("explorer", "stopped");
        assert_eq!(app.control.holder(), None);
        let request = act(&app, "evaluator", graph, true);
        assert_eq!(refusal(start(&mut app, request)), None);
        // How an action ended goes to the trace, with the reason, the goal
        // and who held the window, and to the agents chip.
        app.control.ended(
            &Who {
                agent: "evaluator".into(),
                why: "a reason".into(),
                goal: "a goal".into(),
            },
            "command graph",
            &json!({ "ok": false, "error": "refused: the window is in use by explorer" }),
        );
        let event = serde_json::to_value(app.control.events(0).last().unwrap()).unwrap();
        assert_eq!(event["why"], "a reason");
        assert_eq!(event["goal"], "a goal");
        assert_eq!(event["holder"], "evaluator");
        assert_eq!(event["ok"], false);
        let outcome = app.control.outcome().expect("the chip shows how it ended");
        assert_eq!(
            outcome.refused.as_deref(),
            Some("the window is in use by explorer")
        );
    }

    /// Every refused action says its kind beside its words, so clients do
    /// not depend on the wording.
    #[test]
    fn a_refused_action_says_its_kind() {
        let (mut app, _folder) = crate::edit::app_tests::studio("refusal-kinds");
        let kind = |app: &mut Studio, agent: &str, action: Value, stale: bool| {
            let mut request = act(app, agent, action, true);
            if stale {
                request.body["observed"] = json!(9999);
            }
            let refused = start(app, request).expect_err("refused");
            app.control.active = None;
            refused.1["kind"].as_str().unwrap_or_default().to_string()
        };
        let command = |id: &str| json!({ "kind": "command", "id": id });
        assert_eq!(
            kind(&mut app, "a", json!({ "kind": "teleport" }), false),
            "invalid"
        );
        assert_eq!(kind(&mut app, "a", command("fit"), true), "stale");
        assert_eq!(kind(&mut app, "a", command("lock"), false), "operator-own");
        assert_eq!(
            kind(&mut app, "a", command("zoom-to-selection"), false),
            "unavailable"
        );
        assert_eq!(
            kind(
                &mut app,
                "a",
                json!({ "kind": "click", "control": "nowhere" }),
                false
            ),
            "gone"
        );
        let fit = act(&app, "a", command("fit"), true);
        assert!(start(&mut app, fit).is_ok());
        app.control.active = None;
        assert_eq!(kind(&mut app, "b", command("fit"), false), "held");
        assert_eq!(Refusal::Stopped.answer("x")["kind"], "stopped");
        assert_eq!(Refusal::Expired.answer("x")["ok"], false);
    }

    #[test]
    fn stop_is_the_operators_and_lasts_until_resume() {
        let (mut app, _folder) = crate::edit::app_tests::studio("stop-agents");
        app.control.acting = Some("explorer".into());
        app.stop_agents();
        assert_eq!(app.control.gate, Gate::Run, "an agent cannot stop agents");
        app.control.acting = None;
        app.control.refused = None;
        app.stop_agents();
        assert_eq!(app.control.gate, Gate::Stop);
        assert!(app.agents_paused());
        app.control.acting = Some("explorer".into());
        app.resume_agents();
        assert_eq!(app.control.gate, Gate::Stop, "nor resume them");
        app.control.acting = None;
        app.resume_agents();
        assert_eq!(app.control.gate, Gate::Run);
        let chip = [drawn("agents-stop", "button", "title", false)];
        assert!(operators_only(&app, &Action::Click("agents-stop".into()), &chip).is_some());
    }

    #[test]
    fn observed_controls_and_commands_say_whether_agents_may_act_on_them() {
        let (app, _folder) = crate::edit::app_tests::studio("operator-only");
        let only = |id: &str, role: &'static str, region: &'static str| {
            operator_only(&app, &drawn(id, role, region, false))
        };
        assert!(!only("dialog-confirm", "button", "dialog"));
        assert!(!only("palette-graph", "option", "palette"));
        assert!(!only("palette-search", "field", "palette"));
        assert!(!only("Name", "field", "inspector"));
        assert!(only("palette-lock", "option", "palette"));
        assert!(only("menu-lock", "item", target::ROOT));
        assert!(only("settings-save", "button", "settings"));
        assert!(
            only("Automatic", "item", "conversation"),
            "a menu the Conversation opened"
        );
        assert!(only("objective-intent", "field", "inspector"));
        assert!(only("objective-merge", "switch", "inspector"));
        assert!(only("window-close", "button", "title"));
        assert!(only("agents-pause", "button", "title"));
        assert!(only("inspector-lock", "button", "inspector"));
        assert!(only("trust-local", "button", "inspector"));
        assert!(!only("status-problems", "button", "status"));
        let command = |id| operators_only(&app, &Action::Command(id), &[]).is_some();
        assert!(command(CommandId::Lock));
        assert!(command(CommandId::Undo));
        assert!(!command(CommandId::Graph));
        // One source of truth: the same rule refuses the click.
        let palette = [drawn("palette-lock", "option", "palette", false)];
        let refused = operators_only(&app, &Action::Click("palette-lock".into()), &palette);
        assert_eq!(refused.as_deref(), Some("`lock` is the Operator's to use"));
    }

    #[test]
    fn the_models_digest_follows_its_text_and_comes_back_with_undo() {
        let (mut app, _folder) = crate::edit::app_tests::studio("digest");
        let digest = |app: &mut Studio| {
            let project = app.project.as_ref().unwrap();
            app.control.digest(project)
        };
        let before = digest(&mut app);
        assert_eq!(before.len(), 16);
        assert_eq!(digest(&mut app), before, "stable");
        crate::edit::app_tests::part(&mut app, "api");
        let after = digest(&mut app);
        assert_ne!(after, before);
        app.execute(CommandId::Undo);
        assert_eq!(digest(&mut app), before, "undo restores the text exactly");
        app.execute(CommandId::Redo);
        assert_eq!(digest(&mut app), after);
        // Worked out once per revision; another project starts afresh.
        app.control.digest = Some((u64::MAX, "stale".into()));
        app.control.another_project();
        assert_eq!(app.control.digest, None);
    }

    /// The digest's cost at 10k elements (W12.2): worked out once per model
    /// revision, so observing stays cheap.
    #[test]
    #[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
    fn the_models_digest_is_worked_out_once_per_revision_at_ten_thousand_elements() {
        let (mut app, folder) = crate::edit::app_tests::studio("digest-10k");
        let project = folder.0.join("P");
        std::fs::write(
            project.join("model").join("P.sysml"),
            crate::edit::app_tests::large_model(850),
        )
        .unwrap();
        app.open_project(&project);
        let elements = app.project.as_ref().unwrap().state().tree().len();
        assert!(elements >= 10_000, "{elements} elements");
        let project = app.project.as_ref().unwrap();
        let started = Instant::now();
        app.control.digest(project);
        let first = started.elapsed();
        let started = Instant::now();
        app.control.digest(project);
        let again = started.elapsed();
        println!(
            "the model's digest at {elements} elements: {first:?} after a change, {again:?} otherwise"
        );
        assert!(again < Duration::from_millis(1), "{again:?}");
    }

    #[test]
    fn speeds_and_whole_numbers_are_read_as_declared() {
        assert_eq!(Speed::from_name("observe"), Some(Speed::Observe));
        assert_eq!(Speed::from_name("slow"), None);
        for name in Speed::NAMES {
            assert!(Speed::from_name(name).is_some(), "{name}");
            assert!(
                crate::settings::allowed(
                    crate::settings::setting("control.speed").unwrap(),
                    &json!(name)
                ),
                "{name} is a choice of the setting"
            );
        }
        // Observe: about twelve characters a second, and a moment on the
        // target; instant types at once.
        let per_second = 1000 / Speed::Observe.typing().unwrap().as_millis();
        assert!((11..=13).contains(&per_second), "{per_second}");
        assert_eq!(Speed::Observe.dwell(), Duration::from_millis(300));
        assert_eq!(Speed::Fast.typing(), Some(Duration::ZERO));
        assert_eq!(Speed::Instant.typing(), None);
        let (app, _folder) = crate::edit::app_tests::studio("speed");
        assert_eq!(app.control_speed(), Speed::Observe, "the default");
        assert_eq!(whole(&json!(7)), Some(7));
        assert_eq!(whole(&json!(7.0)), Some(7));
        assert_eq!(whole(&json!(7.5)), None);
        assert_eq!(whole(&json!("7")), None);
    }

    #[test]
    fn in_a_test_instance_agents_may_also_use_the_conversation_and_undo() {
        let (mut app, _folder) = crate::edit::app_tests::studio("test-instance");
        let composer = [drawn("Message", "field", "conversation", true)];
        let send = [drawn("send", "button", "conversation", false)];
        let fill = Action::Fill("Message".into(), "Add a cache".into());
        let typed = Action::Type("Add a cache".into());
        let enter = Action::Key(vec!["enter".into()]);
        let undo = Action::Command(CommandId::Undo);
        // The Operator's own window: all of it is the Operator's.
        assert!(operators_only(&app, &fill, &composer).is_some());
        assert!(operators_only(&app, &typed, &composer).is_some());
        assert!(operators_only(&app, &undo, &composer).is_some());
        assert!(operator_only(&app, &send[0]));
        // A test instance: the Conversation and undo are the agents' too,
        // and the observation says so.
        app.args.test_instance = true;
        assert_eq!(operators_only(&app, &fill, &composer), None);
        assert_eq!(operators_only(&app, &typed, &composer), None);
        assert_eq!(operators_only(&app, &enter, &composer), None);
        assert_eq!(operators_only(&app, &undo, &composer), None);
        assert_eq!(
            operators_only(&app, &Action::Command(CommandId::Redo), &composer),
            None
        );
        assert!(!operator_only(&app, &send[0]));
        assert!(!operator_only(
            &app,
            &drawn("stop", "button", "conversation", false)
        ));
        assert!(!operator_only(
            &app,
            &drawn("tool-t1", "item", "conversation", false)
        ));
        // What stays the Operator's there too.
        for (id, role, region) in [
            ("pause", "button", "conversation"),
            ("step", "button", "conversation"),
            ("retry", "button", "conversation"),
            ("edit-4", "button", "conversation"),
            ("new-conversation", "button", "conversation"),
            ("model-picker", "button", "conversation"),
            ("model-deepseek", "item", "conversation"),
            ("settings-save", "button", "settings"),
            ("agents-pause", "button", "title"),
            ("agents-stop", "button", "title"),
            ("objective-intent", "field", "inspector"),
        ] {
            assert!(operator_only(&app, &drawn(id, role, region, false)), "{id}");
        }
        for id in [
            CommandId::Lock,
            CommandId::TrustLocal,
            CommandId::NewConversation,
            CommandId::Theme,
        ] {
            assert!(
                operators_only(&app, &Action::Command(id), &[]).is_some(),
                "{id:?}"
            );
        }
        // Where the effects happen, too: an agent's step may send, and undo.
        let api = crate::edit::app_tests::part(&mut app, "api");
        let exists = |app: &Studio| {
            app.project
                .as_ref()
                .unwrap()
                .state()
                .tree()
                .get(api)
                .is_some()
        };
        app.control.acting = Some("explorer".into());
        app.execute(CommandId::Undo);
        assert!(app.control.refused.take().is_none());
        assert!(!exists(&app), "undone");
        app.execute(CommandId::Redo);
        assert!(exists(&app), "redone");
        app.conversation.input = "Add a cache".into();
        app.conversation.key_missing = Some("needs a key".into());
        app.send_message();
        assert!(app.control.refused.take().is_none(), "not refused");
        app.operation(
            "Lock api",
            agq_system_state::Operation::Lock { element: api },
        );
        assert!(
            app.control.refused.take().is_some(),
            "locking stays the Operator's"
        );
        app.control.acting = None;
    }

    #[test]
    fn the_conversation_says_what_the_turn_did_in_a_few_lines() {
        let (mut app, _folder) = crate::edit::app_tests::studio("turn-tools");
        use agq_providers::AssistantPart;
        let call = |id: &str, name: &str| AssistantPart::ToolCall {
            id: id.into(),
            name: name.into(),
            input: json!({}),
        };
        let entries = &mut app.conversation.conversation.entries;
        entries.push(agq_assistant::Entry::Operator {
            text: "An earlier request".into(),
        });
        entries.push(agq_assistant::Entry::Assistant {
            model: None,
            parts: vec![call("old", "read_model")],
        });
        entries.push(agq_assistant::Entry::Operator {
            text: "Add a cache in front of the link store".into(),
        });
        entries.push(agq_assistant::Entry::Assistant {
            model: None,
            parts: vec![call("t1", "read_model"), call("t2", "apply_changes")],
        });
        entries.push(agq_assistant::Entry::Notice {
            text: "The provider refused the request: rate limited".into(),
        });
        app.conversation.results.insert(
            "t1".into(),
            agq_assistant::ToolResult::answer("the outline"),
        );
        app.conversation.results.insert(
            "t2".into(),
            agq_assistant::ToolResult::error("`Cache` is not a type".repeat(20)),
        );
        let panel = &app.conversation;
        let (tools, omitted) = turn_tools(panel);
        assert_eq!(tools[0], json!({ "tool": "read_model", "state": "done" }));
        assert_eq!(tools[1]["state"], "failed");
        assert!(tools[1]["error"].as_str().unwrap().chars().count() <= 161);
        assert_eq!((tools.len(), omitted), (2, 0), "only the last turn's");
        assert_eq!(
            notices(panel),
            vec!["The provider refused the request: rate limited".to_string()]
        );
        assert_eq!(conversation_error(panel), None, "no turn failed");
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
