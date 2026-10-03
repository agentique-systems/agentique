//! Scenarios and runs in the Studio (ROADMAP §4.14, W7.4): the scenarios of
//! the open project, runs on background threads (model execution, replay,
//! walkthrough, implementation, live evaluation), the results kept in the
//! project's app data, the playback cursor through a trace, and the marks
//! the Surface draws for it. Nothing here waits on the UI thread: a run's
//! result is taken by `poll_runs` on the workspace's tick.
use crate::studio::{Dirty, Panel, Studio};
use agq_language::{
    Direction, Element, ElementId, ElementKind, Expression, Literal, Parent, QualifiedName,
    Reference, Semantics, Step, Tree,
};
use agq_simulation::digest::model_digest;
use agq_simulation::store::RunSummary;
use agq_simulation::{
    Answers, BackgroundRun, EventKind, Freshness, Mode, Recordings, Request, RunResult, RunStatus,
    RunStore, StopReason, TraceEvent, compile, freshness,
};
use agq_system_state::{Actor, Change, Operation};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Which trace events the timeline lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TraceFilter {
    #[default]
    All,
    Messages,
    States,
    Agents,
    Checks,
}

impl TraceFilter {
    pub const ALL: [TraceFilter; 5] = [
        TraceFilter::All,
        TraceFilter::Messages,
        TraceFilter::States,
        TraceFilter::Agents,
        TraceFilter::Checks,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TraceFilter::All => "All",
            TraceFilter::Messages => "Messages",
            TraceFilter::States => "States",
            TraceFilter::Agents => "Agents",
            TraceFilter::Checks => "Checks",
        }
    }

    pub fn shows(self, event: &TraceEvent) -> bool {
        use EventKind::*;
        match self {
            TraceFilter::All => true,
            TraceFilter::Messages => matches!(event.kind, Sent | Received | Output | Step),
            TraceFilter::States => {
                matches!(event.kind, StateEntered | Transition | Assigned | Timer)
            }
            TraceFilter::Agents => matches!(
                event.kind,
                AgentCalled | AgentAnswered | AgentFailed | Fallback | StandIn
            ),
            TraceFilter::Checks => matches!(event.kind, Check | Stopped),
        }
    }
}

/// How the Surface marks an element: for the trace position of a current
/// result, or for drift found by current implementation checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mark {
    /// Touched earlier in the run.
    Visited,
    /// Part of the event at the cursor.
    Current,
    /// Where the run stopped.
    Failed,
    /// The linked code disagrees with the model here.
    Drift,
}

/// A scenario as the Scenarios list shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioRow {
    pub id: ElementId,
    pub name: String,
    pub qualified_name: String,
    /// The subject's type, such as `UrlShortenerService`.
    pub subject: String,
    pub doc: Option<String>,
    /// The newest result per mode, and whether it is current.
    pub latest: Vec<(Mode, RunSummary, bool)>,
}

/// A run in progress.
pub struct ActiveRun {
    pub scenario: ElementId,
    pub mode: Mode,
    pub started: Instant,
    work: Work,
}

enum Work {
    Model(BackgroundRun),
    Thread {
        receiver: Receiver<RunResult>,
        cancel: Arc<AtomicBool>,
    },
}

impl ActiveRun {
    fn cancel(&self) {
        match &self.work {
            Work::Model(run) => run.cancel(),
            Work::Thread { cancel, .. } => cancel.store(true, Ordering::SeqCst),
        }
    }

    fn try_result(&mut self) -> Option<RunResult> {
        match &mut self.work {
            Work::Model(run) => run.try_result(),
            Work::Thread { receiver, .. } => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => None,
            },
        }
    }
}

/// The Studio's scenarios and runs.
#[derive(Default)]
pub struct RunsState {
    pub selected: Option<ElementId>,
    /// The mode the Run panel runs in.
    pub mode: Option<Mode>,
    pub active: Option<ActiveRun>,
    /// The result shown in the Run panel.
    pub result: Option<RunResult>,
    pub freshness: Option<Freshness>,
    /// The trace event the Surface shows.
    pub cursor: Option<usize>,
    pub playing: bool,
    last_step: Option<Instant>,
    /// The camera goes to what the cursor shows.
    pub follow: bool,
    pub filter: TraceFilter,
    rows: Option<(u64, usize, Vec<ScenarioRow>)>,
    /// Bumped whenever a result is saved, so lists refresh.
    pub saved_generation: usize,
    /// A live evaluation waits for the Operator to confirm it.
    pub confirm_live: bool,
    /// Which kind of step the Run panel's "Add" offers.
    pub adding: usize,
    /// The Assistant waits for the run in progress (`run_scenario`).
    pub(crate) assistant: Option<std::sync::mpsc::Sender<agq_assistant::ToolResult>>,
}

impl RunsState {
    pub fn running(&self) -> bool {
        self.active.is_some()
    }

    pub fn mode(&self) -> Mode {
        self.mode.unwrap_or(Mode::Model)
    }

    /// The events of the shown result that the filter lets through, with
    /// their index in the whole trace.
    pub fn visible_events(&self) -> Vec<usize> {
        let Some(result) = &self.result else {
            return Vec::new();
        };
        result
            .trace
            .iter()
            .enumerate()
            .filter(|(_, e)| self.filter.shows(e))
            .map(|(i, _)| i)
            .collect()
    }
}

/// How long playback stays on each event.
pub const PLAYBACK_STEP: Duration = Duration::from_millis(420);

impl Studio {
    /// Where this project's results are kept (app data, never committed).
    pub fn run_store(&self) -> Option<RunStore> {
        let project = self.project.as_ref()?;
        Some(RunStore::new(
            crate::conversation::project_data(&self.session_path, project.folder()).join("runs"),
        ))
    }

    /// The project's kept recordings of agent answers (`recordings/`).
    pub fn recordings_folder(&self) -> Option<PathBuf> {
        Some(self.project.as_ref()?.folder().join("recordings"))
    }

    /// Every scenario of the project, with its newest results; kept until
    /// the model changes or a result is saved.
    pub fn scenario_rows(&mut self) -> Vec<ScenarioRow> {
        let Some(project) = &self.project else {
            return Vec::new();
        };
        let revision = project.state().revision();
        if let Some((r, g, rows)) = &self.runs.rows
            && *r == revision
            && *g == self.runs.saved_generation
        {
            return rows.clone();
        }
        let rows = self.scenario_rows_now();
        self.runs.rows = Some((revision, self.runs.saved_generation, rows.clone()));
        rows
    }

    /// Every scenario of the project, with its newest results, read now
    /// (or from the kept rows when they are still right).
    pub fn scenario_rows_now(&self) -> Vec<ScenarioRow> {
        let Some(project) = &self.project else {
            return Vec::new();
        };
        let revision = project.state().revision();
        if let Some((r, g, rows)) = &self.runs.rows
            && *r == revision
            && *g == self.runs.saved_generation
        {
            return rows.clone();
        }
        let tree = project.state().tree();
        let summaries = self.run_store().map(|s| s.list()).unwrap_or_default();
        let mut rows = Vec::new();
        for id in tree.walk() {
            let element = &tree[id];
            if element.kind != ElementKind::VerificationDef {
                continue;
            }
            let digest = model_digest(tree, id);
            let mut latest: Vec<(Mode, RunSummary, bool)> = Vec::new();
            for summary in summaries.iter().filter(|s| s.scenario == id.raw()) {
                if latest.iter().any(|(m, ..)| *m == summary.mode) {
                    continue;
                }
                let current = summary.model_digest == digest;
                latest.push((summary.mode, summary.clone(), current));
            }
            latest.sort_by_key(|(mode, ..)| Mode::ALL.iter().position(|m| m == mode));
            rows.push(ScenarioRow {
                id,
                name: tree.effective_name(id).unwrap_or("?").to_string(),
                qualified_name: tree.qualified_name(id),
                subject: subject_type(tree, id).unwrap_or_else(|| "no subject".into()),
                doc: doc_of(tree, id),
                latest,
            });
        }
        rows
    }

    /// Shows a scenario in the Run panel with its newest result in the chosen mode.
    pub fn select_scenario(&mut self, id: ElementId) {
        self.runs.selected = Some(id);
        self.runs.playing = false;
        self.runs.cursor = None;
        self.load_latest_result();
        self.panel = Panel::Run;
        if self.inspector_hidden || self.panels_hidden {
            self.inspector_hidden = false;
            self.panels_hidden = false;
        }
        self.mark(Dirty::LAYOUT | Dirty::MODEL | Dirty::SELECTION);
    }

    /// Loads the newest result of the selected scenario in the chosen mode.
    pub fn load_latest_result(&mut self) {
        let (Some(id), Some(store)) = (self.runs.selected, self.run_store()) else {
            self.runs.result = None;
            self.runs.freshness = None;
            return;
        };
        let result = store.latest(id.raw(), self.runs.mode());
        self.show_result(result);
    }

    /// Shows a result and works out whether it is current.
    pub fn show_result(&mut self, result: Option<RunResult>) {
        self.runs.freshness = match (&result, &self.project) {
            (Some(result), Some(project)) => {
                let mut fresh = freshness(result, project.state().tree());
                if fresh.is_current()
                    && result.mode == Mode::Implementation
                    && let Some(repository) = self.implementation_repository()
                    && let Err(why) =
                        agq_implementation::harness::code_is_current(result, &repository)
                {
                    fresh = Freshness::Outdated(why);
                }
                Some(fresh)
            }
            _ => None,
        };
        self.runs.cursor = result.as_ref().and_then(|r| {
            // Where it stopped, or the last event.
            r.trace
                .iter()
                .rposition(|e| e.kind == EventKind::Stopped)
                .or_else(|| r.trace.len().checked_sub(1))
        });
        self.runs.result = result;
    }

    /// Works out again whether the shown result is current, after the model
    /// changed. Cheap: a digest of the scenario's model slice.
    pub fn refresh_run_freshness(&mut self) {
        let (Some(result), Some(project)) = (&self.runs.result, &self.project) else {
            return;
        };
        let mut fresh = freshness(result, project.state().tree());
        if fresh.is_current()
            && result.mode == Mode::Implementation
            && let Some(Freshness::Outdated(why)) = &self.runs.freshness
            && why.contains("code")
        {
            // The code has not changed with the model.
            fresh = Freshness::Outdated(why.clone());
        }
        self.runs.freshness = Some(fresh);
    }

    /// Whether the shown result may be drawn on the Surface: it describes
    /// the model as it is now. An outdated result is never overlaid on
    /// today's geometry.
    pub fn result_is_current(&self) -> bool {
        self.runs
            .freshness
            .as_ref()
            .is_some_and(Freshness::is_current)
    }

    /// Starts a run of the selected scenario in `mode`. A live evaluation
    /// only after the Operator confirmed it (`confirm_live`).
    pub fn start_run(&mut self, mode: Mode) {
        let (Some(scenario), Some(project)) = (self.runs.selected, self.project.as_ref()) else {
            self.status = "Choose a scenario to run.".into();
            return;
        };
        if self.runs.active.is_some() {
            self.status = "A run is in progress; stop it first.".into();
            return;
        }
        self.runs.mode = Some(mode);
        let tree = project.state().tree();
        let digest = model_digest(tree, scenario);
        let mut request = Request::new(mode);
        request.model_revision = project.state().revision();
        let program = match compile(tree, scenario) {
            Ok(program) => program,
            Err(blockers) => {
                let mut result = blocked_result(tree, scenario, mode, digest, &request);
                result.blockers = blockers
                    .into_iter()
                    .map(|b| (b.element.raw(), b.message))
                    .collect();
                self.finish_run(result);
                return;
            }
        };
        let work = match mode {
            Mode::Model | Mode::Walkthrough => Work::Model(BackgroundRun::start(
                program,
                digest,
                request,
                Answers::StandIns,
            )),
            Mode::Replay => {
                let folder = self.recordings_folder().unwrap_or_default();
                let recordings = Arc::new(Recordings::read(&folder));
                Work::Model(BackgroundRun::start(
                    program,
                    digest,
                    request,
                    Answers::Recordings(recordings),
                ))
            }
            Mode::Live => {
                let Some(model) = crate::live::live_model(self) else {
                    self.status = "A live evaluation needs a provider key for the agent's model: add one in Settings › Providers.".into();
                    return;
                };
                request.samples = 5;
                Work::Model(BackgroundRun::start(
                    program,
                    digest,
                    request,
                    Answers::Live(model),
                ))
            }
            Mode::Implementation => match self.implementation_run(program, digest, request) {
                Ok(work) => work,
                Err(why) => {
                    self.status = why;
                    return;
                }
            },
        };
        self.runs.active = Some(ActiveRun {
            scenario,
            mode,
            started: Instant::now(),
            work,
        });
        self.runs.playing = false;
        self.status = format!("Running in {}…", mode.label().to_lowercase());
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
    }

    /// An implementation run on its own thread, through the linked harness.
    fn implementation_run(
        &self,
        program: agq_simulation::Program,
        digest: String,
        request: Request,
    ) -> Result<Work, String> {
        let links = self.implementation_links().map_err(|e| e.to_string())?;
        if links.harness.is_empty() {
            return Err("No harness is linked: model/links.json names the command that runs the code for scenarios (`harness`).".into());
        }
        let trusted = self.implementation.choice.is_some_and(|c| c.trusted)
            || self.execution_file_choice().trusted;
        if !trusted {
            return Err("Running the real code needs trusted-local execution, which is off for this project; the Run panel turns it on.".into());
        }
        let repository = self
            .implementation_repository()
            .ok_or("The project has no implementation repository linked.")?;
        let executor = self.implementation_executor(&repository, false)?;
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::Builder::new()
            .name("agentique-implementation-run".into())
            .spawn(move || {
                let result = agq_implementation::run_implementation(
                    &program,
                    digest,
                    &request,
                    &links,
                    &repository,
                    &executor,
                    flag,
                );
                let _ = sender.send(result);
            })
            .map_err(|e| e.to_string())?;
        Ok(Work::Thread { receiver, cancel })
    }

    /// Stops the run in progress; it ends as cancelled.
    pub fn stop_run(&mut self) {
        if let Some(active) = &self.runs.active {
            active.cancel();
            self.status = "Stopping the run…".into();
            self.mark(Dirty::STATUS);
        }
    }

    /// Takes a finished run's result. Returns whether anything changed.
    pub fn poll_runs(&mut self) -> bool {
        let Some(active) = self.runs.active.as_mut() else {
            return false;
        };
        let Some(result) = active.try_result() else {
            // A run that takes long says it is still going.
            return false;
        };
        self.runs.active = None;
        self.finish_run(result);
        true
    }

    fn finish_run(&mut self, result: RunResult) {
        if let Some(store) = self.run_store()
            && let Err(error) = store.save(&result)
        {
            self.status = format!("The result could not be saved: {error}");
        }
        if let Some(reply) = self.runs.assistant.take() {
            let _ = reply.send(agq_assistant::ToolResult::answer(result.describe(None, 12)));
        }
        self.runs.saved_generation += 1;
        self.status = describe_result(&result);
        let scenario = ElementId::from_raw(result.scenario);
        if self.runs.selected == Some(scenario) {
            self.show_result(Some(result));
        }
        self.mark(Dirty::LAYOUT | Dirty::MODEL | Dirty::STATUS);
    }

    /// Moves the playback cursor; the camera follows when asked to.
    pub fn set_cursor(&mut self, cursor: Option<usize>) {
        let len = self.runs.result.as_ref().map_or(0, |r| r.trace.len());
        self.runs.cursor = cursor.filter(|c| *c < len);
        if self.runs.follow
            && let Some(element) = self.cursor_element()
        {
            self.follow_element(element);
        }
        self.mark(Dirty::MODEL | Dirty::LAYOUT);
    }

    /// Steps the cursor through the events the filter shows.
    pub fn step_cursor(&mut self, forward: bool) {
        let visible = self.runs.visible_events();
        if visible.is_empty() {
            return;
        }
        let next = match self.runs.cursor {
            None => {
                if forward {
                    visible.first().copied()
                } else {
                    visible.last().copied()
                }
            }
            Some(c) if forward => visible.iter().copied().find(|i| *i > c).or(Some(c)),
            Some(c) => visible.iter().rev().copied().find(|i| *i < c).or(Some(c)),
        };
        self.set_cursor(next);
    }

    /// Plays the trace from the cursor, one event per step; again pauses.
    pub fn toggle_playback(&mut self) {
        let Some(result) = &self.runs.result else {
            return;
        };
        if self.runs.playing {
            self.runs.playing = false;
        } else {
            if self.runs.cursor.is_none_or(|c| c + 1 >= result.trace.len()) {
                self.runs.cursor = self.runs.visible_events().first().copied();
            }
            self.runs.playing = true;
            self.runs.last_step = Some(Instant::now());
        }
        self.mark(Dirty::LAYOUT | Dirty::MODEL);
    }

    /// Advances playback on the tick. Returns whether it moved.
    pub fn playback_tick(&mut self) -> bool {
        if !self.runs.playing {
            return false;
        }
        let due = self
            .runs
            .last_step
            .is_none_or(|last| last.elapsed() >= PLAYBACK_STEP);
        if !due {
            return false;
        }
        self.runs.last_step = Some(Instant::now());
        let before = self.runs.cursor;
        self.step_cursor(true);
        if self.runs.cursor == before {
            self.runs.playing = false;
        }
        true
    }

    /// The main model element of the event at the cursor.
    pub fn cursor_element(&self) -> Option<ElementId> {
        let result = self.runs.result.as_ref()?;
        let event = result.trace.get(self.runs.cursor?)?;
        event.elements.first().map(|e| ElementId::from_raw(*e))
    }

    fn follow_element(&mut self, element: ElementId) {
        let tree = match self.project.as_ref() {
            Some(project) => project.state().tree(),
            None => return,
        };
        let mut current = Some(element);
        while let Some(id) = current {
            let target = agq_studio_scene::SceneTarget::Node(id);
            if self.scene.target_bounds(&target).is_some() {
                self.frame_target(&target);
                return;
            }
            current = tree.get(id).and_then(Element::owner);
        }
    }

    /// What the Surface marks: drift from current implementation checks,
    /// and, while the Run panel shows a current result, where its trace is.
    /// An outdated result draws nothing.
    pub fn surface_marks(&self) -> BTreeMap<ElementId, Mark> {
        let mut marks: BTreeMap<ElementId, Mark> = self
            .drift()
            .into_keys()
            .map(|element| (element, Mark::Drift))
            .collect();
        if !self.result_is_current() || self.panel != Panel::Run {
            return marks;
        }
        let Some(result) = &self.runs.result else {
            return marks;
        };
        let upto = self.runs.cursor.unwrap_or(0);
        for event in result.trace.iter().take(upto) {
            for element in &event.elements {
                marks.insert(ElementId::from_raw(*element), Mark::Visited);
            }
        }
        if let Some(event) = self.runs.cursor.and_then(|c| result.trace.get(c)) {
            for element in &event.elements {
                marks.insert(ElementId::from_raw(*element), Mark::Current);
            }
        }
        if let Some(stop) = &result.stop
            && let Some(element) = stop.element
            && stop.reason != StopReason::Cancelled
        {
            marks.insert(ElementId::from_raw(element), Mark::Failed);
        }
        marks
    }
}

/// A result for a scenario that could not start.
fn blocked_result(
    tree: &Tree,
    scenario: ElementId,
    mode: Mode,
    digest: String,
    request: &Request,
) -> RunResult {
    RunResult {
        format: agq_simulation::result::FORMAT,
        id: agq_simulation::new_run_id(),
        scenario: scenario.raw(),
        scenario_name: tree.effective_name(scenario).unwrap_or("?").to_string(),
        scenario_qualified_name: tree.qualified_name(scenario),
        mode,
        started: agq_simulation::result::utc_now(),
        wall_ms: 0,
        logical_ms: 0,
        events_processed: 0,
        status: RunStatus::Blocked,
        stop: None,
        blockers: Vec::new(),
        checks: Vec::new(),
        verifies: Vec::new(),
        trace: Vec::new(),
        provenance: agq_simulation::result::Provenance {
            runner: format!("{} {}", agq_simulation::RUNNER, mode.key()),
            model_digest: digest,
            model_revision: request.model_revision,
            seed: 0,
            implementation: None,
            live: None,
            recordings: None,
        },
        live: None,
    }
}

/// One line for the status bar.
pub fn describe_result(result: &RunResult) -> String {
    let checks = result
        .tally()
        .iter()
        .map(|(verdict, n)| format!("{n} {}", verdict.label()))
        .collect::<Vec<_>>()
        .join(", ");
    match result.status {
        RunStatus::Blocked => format!(
            "{} could not start: {}",
            result.scenario_name,
            result
                .blockers
                .first()
                .map(|(_, m)| m.as_str())
                .unwrap_or("see the Run panel")
        ),
        RunStatus::Stopped => format!(
            "{} stopped ({}); checks: {checks}",
            result.scenario_name,
            result.stop.as_ref().map_or("?", |s| s.reason.code())
        ),
        _ => format!(
            "{} {} in {}; checks: {checks}",
            result.scenario_name,
            result.status.label(),
            result.mode.label().to_lowercase()
        ),
    }
}

/// The name of the type of a scenario's subject.
pub fn subject_type(tree: &Tree, scenario: ElementId) -> Option<String> {
    let subject = tree[scenario]
        .children()
        .iter()
        .copied()
        .find(|c| tree[*c].kind == ElementKind::Subject)?;
    let reference = tree[subject].typed_by.first()?;
    Some(match reference.target() {
        Some(target) if tree.contains(target) => tree.effective_name(target)?.to_string(),
        _ => reference.last_name().to_string(),
    })
}

/// An element's documentation.
pub fn doc_of(tree: &Tree, id: ElementId) -> Option<String> {
    tree[id]
        .children()
        .iter()
        .find(|c| tree[**c].kind == ElementKind::Doc)
        .and_then(|d| tree[*d].text.clone())
}

// ---- the Assistant's requests (W8.3: the same services) ----

impl Studio {
    /// Carries out a factory tool the Assistant called; the reply goes out
    /// when the run or the checks end.
    pub fn carry_out_request(
        &mut self,
        request: agq_assistant::tools::StudioRequest,
        reply: std::sync::mpsc::Sender<agq_assistant::ToolResult>,
    ) {
        use agq_assistant::{ToolResult, tools::StudioRequest};
        match request {
            StudioRequest::Run { scenario, mode } => {
                if self.runs.active.is_some() {
                    let _ = reply.send(ToolResult::error(
                        "Not run: a run is in progress. Wait for it, or stop it with stop_run.",
                    ));
                    return;
                }
                if !self.editable() {
                    let _ = reply.send(ToolResult::error(
                        "Not run: the Operator is looking at an earlier checkpoint.",
                    ));
                    return;
                }
                // The Operator sees the run in the Run panel.
                self.select_scenario(scenario);
                self.runs.assistant = Some(reply);
                let before = self.runs.saved_generation;
                self.start_run(mode);
                // It could not start (no trust for code, say): say why.
                if self.runs.active.is_none()
                    && self.runs.saved_generation == before
                    && let Some(reply) = self.runs.assistant.take()
                {
                    let _ = reply.send(ToolResult::error(format!("Not run: {}", self.status)));
                }
            }
            StudioRequest::StopRun => {
                let text = if self.runs.running() {
                    self.stop_run();
                    "Stopping the run; its result comes back as cancelled."
                } else {
                    "Nothing is running."
                };
                let _ = reply.send(ToolResult::answer(text));
            }
            StudioRequest::ReadRun { scenario, mode } => {
                let result = self
                    .run_store()
                    .and_then(|s| s.latest(scenario.raw(), mode));
                let answer = match (result, &self.project) {
                    (Some(result), Some(project)) => {
                        let why = match freshness(&result, project.state().tree()) {
                            Freshness::Current => None,
                            Freshness::Outdated(why) => Some(why),
                        };
                        ToolResult::answer(result.describe(why.as_deref(), 12))
                    }
                    _ => ToolResult::answer(format!(
                        "Not run in {} yet.",
                        mode.label().to_lowercase()
                    )),
                };
                let _ = reply.send(answer);
            }
            StudioRequest::Explain { element } => {
                let _ = reply.send(match self.explain(element) {
                    Some(text) => ToolResult::answer(text),
                    None => ToolResult::error("Only a part def or a part can be explained."),
                });
            }
            StudioRequest::ReadCodeLinks { element } => {
                let _ = reply.send(match self.describe_links(element) {
                    Ok(text) => ToolResult::answer(text),
                    Err(error) => ToolResult::error(error),
                });
            }
            StudioRequest::Implement {
                element,
                instructions,
            } => {
                if let Some(why) = self.task_blocker() {
                    let _ = reply.send(ToolResult::error(format!("Not started: {why}")));
                    return;
                }
                if self.dialog.is_some() {
                    let _ = reply.send(ToolResult::error(
                        "Not started: the Operator is busy with a dialog; propose it again later.",
                    ));
                    return;
                }
                self.implementation.proposal = Some(reply);
                self.dialog = Some(crate::edit::Dialog::Implement {
                    element,
                    instructions,
                });
                self.mark(Dirty::OVERLAY);
            }
            StudioRequest::CheckImplementation => {
                if self.implementation.checking() {
                    let _ = reply.send(ToolResult::error("The checks are already running."));
                    return;
                }
                self.start_checks();
                if self.implementation.checking() {
                    self.implementation.assistant = Some(reply);
                } else {
                    let _ = reply.send(ToolResult::error(format!("Not run: {}", self.status)));
                }
            }
        }
        self.mark(Dirty::LAYOUT | Dirty::STATUS | Dirty::MODEL);
    }
}

// ---- writing scenarios (C-50: through controls, never SysML text) ----

/// A port of a scenario's subject and what may pass through it.
#[derive(Clone, Debug, PartialEq)]
pub struct PortChoice {
    /// The subject, then the port.
    pub path: Vec<(ElementId, String)>,
    pub label: String,
    /// Items the scenario may send in: (type, name).
    pub sends: Vec<(ElementId, String)>,
    /// Items the scenario may wait for: (type, name).
    pub accepts: Vec<(ElementId, String)>,
}

/// What the Run panel's "Add" menu adds to a scenario.
#[derive(Clone, Debug, PartialEq)]
pub enum NewStep {
    /// `send new T() via subject.port`.
    Send {
        port: Vec<(ElementId, String)>,
        ty: (ElementId, String),
    },
    /// `accept name : T via subject.port`.
    Accept {
        port: Vec<(ElementId, String)>,
        ty: (ElementId, String),
    },
    /// `accept after <ms>`.
    Wait { ms: u64 },
    /// `assert constraint name { expression }`.
    Check { expression: String },
    /// A stand-in for an agent or part inside the subject.
    StandIn {
        target: Vec<(ElementId, String)>,
        outcome: String,
    },
}

/// A reference along a path of features, each step linked.
fn chain(path: &[(ElementId, String)]) -> Reference {
    Reference {
        steps: path
            .iter()
            .map(|(id, name)| Step {
                name: QualifiedName::new([name.as_str()]),
                target: Some(*id),
            })
            .collect(),
    }
}

/// Where new members of `definition`'s owner go.
fn owner_of(tree: &Tree, definition: ElementId) -> Parent {
    tree[definition].owner().map_or(
        Parent::Document(tree.document_of(definition).unwrap_or(0)),
        Parent::Element,
    )
}

/// The subject usage of a scenario.
pub fn subject_of(tree: &Tree, scenario: ElementId) -> Option<ElementId> {
    tree[scenario]
        .children()
        .iter()
        .copied()
        .find(|c| tree[*c].kind == ElementKind::Subject)
}

impl Studio {
    /// "New scenario…": for the selected part's definition.
    pub fn start_new_scenario(&mut self) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let subject = self
            .inspected_element()
            .and_then(|e| crate::library::definition_of(tree, e))
            .filter(|d| tree[*d].kind == ElementKind::PartDef);
        let Some(subject) = subject else {
            self.status = "Select the part the scenario is about, then choose New scenario.".into();
            self.mark(Dirty::STATUS);
            return;
        };
        let base = format!("{}Scenario", tree.effective_name(subject).unwrap_or("New"));
        let name = fresh_name(tree, owner_of(tree, subject), &base);
        self.dialog = Some(crate::edit::Dialog::NewScenario { subject, name });
        self.mark(Dirty::OVERLAY);
    }

    /// Creates an empty scenario about `subject` and opens it.
    pub fn create_scenario(&mut self, subject: ElementId, name: &str) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let subject_name = tree.effective_name(subject).unwrap_or("System").to_string();
        let scenario = tree.next_id();
        let mut usage = Element::named(ElementKind::Subject, &lower_first(&subject_name));
        usage.typed_by = vec![Reference::to(subject, &subject_name)];
        let operations = vec![
            Operation::Create {
                parent: owner_of(tree, subject),
                element: Box::new(Element::named(ElementKind::VerificationDef, name.trim())),
            },
            Operation::Create {
                parent: Parent::Element(scenario),
                element: Box::new(usage),
            },
        ];
        let change = Change::new(
            Actor::Operator,
            &format!("Create scenario {}", name.trim()),
            operations,
        );
        if let Some(event) = self.submit(change)
            && let Some(created) = event.created.first().copied()
        {
            self.select_scenario(created);
        }
    }

    /// The ports of a scenario's subject, with what may be sent and awaited.
    pub fn scenario_ports(&self, scenario: ElementId) -> Vec<PortChoice> {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return Vec::new();
        };
        let Some(subject) = subject_of(tree, scenario) else {
            return Vec::new();
        };
        let semantics = Semantics::new(tree);
        let Some((definition, _)) = semantics.types_of(subject).first().copied() else {
            return Vec::new();
        };
        let subject_name = tree
            .effective_name(subject)
            .unwrap_or("subject")
            .to_string();
        let mut out = Vec::new();
        for port in semantics.features(definition) {
            if semantics.element(port).map(|e| e.kind) != Some(ElementKind::Port) {
                continue;
            }
            let port_name = semantics.name(port).unwrap_or("?").to_string();
            let mut choice = PortChoice {
                path: vec![(subject, subject_name.clone()), (port, port_name.clone())],
                label: format!("{subject_name}.{port_name}"),
                sends: Vec::new(),
                accepts: Vec::new(),
            };
            for (_, direction, ty) in semantics.directed_features(port) {
                let Some(ty) = ty else { continue };
                let entry = (ty, semantics.name(ty).unwrap_or("?").to_string());
                if matches!(direction, Direction::In | Direction::InOut)
                    && !choice.sends.contains(&entry)
                {
                    choice.sends.push(entry.clone());
                }
                if matches!(direction, Direction::Out | Direction::InOut)
                    && !choice.accepts.contains(&entry)
                {
                    choice.accepts.push(entry);
                }
            }
            out.push(choice);
        }
        out
    }

    /// The parts and agents inside a scenario's subject a stand-in may
    /// replace: (path from the subject, label).
    pub fn stand_in_targets(&self, scenario: ElementId) -> Vec<(Vec<(ElementId, String)>, String)> {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return Vec::new();
        };
        let Some(subject) = subject_of(tree, scenario) else {
            return Vec::new();
        };
        let semantics = Semantics::new(tree);
        let subject_name = tree
            .effective_name(subject)
            .unwrap_or("subject")
            .to_string();
        let mut out = Vec::new();
        let mut stack = vec![(subject, vec![(subject, subject_name)], 0)];
        while let Some((usage, path, depth)) = stack.pop() {
            if depth > 4 {
                continue;
            }
            let Some((definition, _)) = semantics.types_of(usage).first().copied() else {
                continue;
            };
            for part in semantics.features(definition) {
                if semantics.element(part).map(|e| e.kind) != Some(ElementKind::Part) {
                    continue;
                }
                let mut next = path.clone();
                next.push((part, semantics.name(part).unwrap_or("?").to_string()));
                let label = next
                    .iter()
                    .map(|(_, n)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                out.push((next.clone(), label));
                stack.push((part, next, depth + 1));
            }
        }
        out.sort_by(|a, b| a.1.cmp(&b.1));
        out
    }

    /// Adds a step, a check or a stand-in to a scenario, as one change.
    pub fn add_to_scenario(&mut self, scenario: ElementId, step: NewStep) -> Result<(), String> {
        let tree = self
            .project
            .as_ref()
            .map(|p| p.state().tree())
            .ok_or("No project is open.")?;
        let semantics = Semantics::new(tree);
        let parent = Parent::Element(scenario);
        let taken = |base: &str| fresh_name(tree, parent, base);
        let (description, head, inside) = match step {
            NewStep::Send { port, ty } => {
                let mut send = Element::new(ElementKind::Send);
                send.expression = Some(Expression::New {
                    ty: Reference::to(ty.0, &ty.1),
                    arguments: Vec::new(),
                });
                send.via = Some(chain(&port));
                (format!("Send {} in", ty.1), send, Vec::new())
            }
            NewStep::Accept { port, ty } => {
                let mut accept = Element::named(ElementKind::Accept, &taken(&lower_first(&ty.1)));
                accept.typed_by = vec![Reference::to(ty.0, &ty.1)];
                accept.via = Some(chain(&port));
                (format!("Wait for {}", ty.1), accept, Vec::new())
            }
            NewStep::Wait { ms } => {
                let mut wait = Element::new(ElementKind::Accept);
                wait.after = true;
                wait.expression = Some(Expression::Literal(Literal::Integer(ms.to_string())));
                (format!("Wait {ms} ms"), wait, Vec::new())
            }
            NewStep::Check { expression } => {
                let parsed = agq_language::parse_expression(&expression)?;
                let mut check = Element::named(ElementKind::AssertConstraint, &taken("check"));
                check.expression = Some(parsed);
                ("Add a check".to_string(), check, Vec::new())
            }
            NewStep::StandIn { target, outcome } => {
                let stand_in = semantics
                    .resolve("Scenarios::StandIn")
                    .ok_or("The Scenarios library is missing.")?;
                let outcomes = semantics
                    .resolve("Scenarios::Outcome")
                    .ok_or("The Scenarios library is missing.")?;
                let value = semantics
                    .features(outcomes)
                    .into_iter()
                    .find(|f| semantics.name(*f) == Some(outcome.as_str()))
                    .ok_or_else(|| format!("`{outcome}` is not an outcome"))?;
                let feature = |name: &str| {
                    semantics
                        .features(stand_in)
                        .into_iter()
                        .find(|f| semantics.name(*f) == Some(name))
                };
                let last = target.last().map(|(_, n)| n.clone()).unwrap_or_default();
                let mut part = Element::named(ElementKind::Part, &taken(&format!("{last}StandIn")));
                part.typed_by = vec![Reference::to(stand_in, "Scenarios::StandIn")];
                let redefine = |name: &str, expression: Expression| {
                    let mut e = Element::new(ElementKind::Reference);
                    if let Some(f) = feature(name) {
                        e.redefines = vec![Reference::to(f, name)];
                    }
                    e.expression = Some(expression);
                    e
                };
                let outcome_ref = Reference {
                    steps: vec![Step {
                        name: QualifiedName::new(["Scenarios", "Outcome", outcome.as_str()]),
                        target: Some(value),
                    }],
                };
                let inside = vec![
                    redefine("target", Expression::Name(chain(&target))),
                    redefine("outcome", Expression::Name(outcome_ref)),
                ];
                (format!("Add a stand-in for {last}"), part, inside)
            }
        };
        let first = tree.next_id();
        let mut operations = vec![Operation::Create {
            parent,
            element: Box::new(head),
        }];
        operations.extend(inside.into_iter().map(|element| Operation::Create {
            parent: Parent::Element(first),
            element: Box::new(element),
        }));
        match self.submit(Change::new(Actor::Operator, &description, operations)) {
            Some(_) => {
                self.mark(Dirty::MODEL | Dirty::LAYOUT);
                Ok(())
            }
            None => Err(self.status.clone()),
        }
    }

    /// Keeps the shown live evaluation's answers in `recordings/`, for replay.
    pub fn keep_recordings(&mut self) {
        let (Some(folder), Some(result)) = (self.recordings_folder(), self.runs.result.as_ref())
        else {
            return;
        };
        let Some(live) = &result.live else {
            return;
        };
        self.status = match Recordings::keep(&folder, &live.answers) {
            Ok(0) => "Every answer of this run is already kept.".into(),
            Ok(n) => format!(
                "Kept {n} answer(s) in recordings/ for replay; commit them with the project."
            ),
            Err(error) => format!("The recordings could not be kept: {error}"),
        };
        self.mark(Dirty::STATUS);
    }
}

/// `base` if no member of `parent` has it, else `base2`, `base3`…
fn fresh_name(tree: &Tree, parent: Parent, base: &str) -> String {
    let members: Vec<ElementId> = match parent {
        Parent::Element(id) => tree
            .get(id)
            .map(|e| e.children().to_vec())
            .unwrap_or_default(),
        Parent::Document(index) => tree
            .documents()
            .get(index)
            .map(|d| d.members().to_vec())
            .unwrap_or_default(),
    };
    let taken = |name: &str| {
        members
            .iter()
            .any(|id| tree.effective_name(*id) == Some(name))
    };
    std::iter::once(base.to_string())
        .chain((2..).map(|n| format!("{base}{n}")))
        .find(|name| !taken(name))
        .expect("an unused name exists")
}

fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

// ---- stand-ins, evidence (the Inspector's sections) ----

/// The features a stand-in sets, in the order the Inspector shows them.
pub const STAND_IN_FEATURES: [&str; 5] = ["target", "call", "outcome", "latencyMs", "output"];

/// A stand-in's features: (name, the redefining child if set, its text).
pub fn stand_in_features(
    tree: &Tree,
    element: ElementId,
) -> Option<Vec<(&'static str, Option<ElementId>, String)>> {
    let semantics = Semantics::new(tree);
    let stand_in = semantics.resolve("Scenarios::StandIn")?;
    let e = tree.get(element)?;
    let typed = e.typed_by.iter().any(|t| {
        t.target()
            .is_some_and(|ty| ty == stand_in || semantics.specializes(ty, stand_in))
    });
    if e.kind != ElementKind::Part || !typed {
        return None;
    }
    let mut out = Vec::new();
    for name in STAND_IN_FEATURES {
        let child = e.children().iter().copied().find(|c| {
            tree[*c].redefines.iter().any(|r| r.last_name() == name)
                || tree.effective_name(*c) == Some(name)
        });
        let text = child
            .map(|c| {
                let child = &tree[c];
                match (&child.value, &child.expression) {
                    (Some(literal), _) => literal.to_string(),
                    (None, Some(expression)) => agq_language::print_expression(tree, c, expression),
                    _ => String::new(),
                }
            })
            .unwrap_or_default();
        out.push((name, child, text));
    }
    Some(out)
}

impl Studio {
    /// Sets one feature of a stand-in from what the Operator typed: a
    /// number or text is a value, anything else an expression; empty
    /// removes it.
    pub fn set_stand_in_feature(
        &mut self,
        stand_in: ElementId,
        name: &str,
        text: &str,
    ) -> Result<(), String> {
        let tree = self
            .project
            .as_ref()
            .map(|p| p.state().tree())
            .ok_or("No project is open.")?;
        let features = stand_in_features(tree, stand_in).ok_or("This is not a stand-in.")?;
        let existing = features
            .iter()
            .find(|(n, ..)| *n == name)
            .and_then(|(_, c, _)| *c);
        let text = text.trim();
        let owner = tree
            .effective_name(stand_in)
            .unwrap_or("stand-in")
            .to_string();
        let description = format!("Set {name} of {owner}");
        let (literal, expression) = if text.is_empty() {
            (None, None)
        } else if let Some(literal) = crate::panels::parse_value(text)
            .filter(|l| !matches!(l, Literal::String(_)) || text.starts_with('"'))
        {
            (Some(literal), None)
        } else {
            (None, Some(agq_language::parse_expression(text)?))
        };
        let operations = match existing {
            Some(child) if text.is_empty() => vec![Operation::Delete { element: child }],
            Some(child) => vec![
                Operation::Set {
                    element: child,
                    property: agq_system_state::Property::Value(literal),
                },
                Operation::Set {
                    element: child,
                    property: agq_system_state::Property::Expression(expression),
                },
            ],
            None if text.is_empty() => return Ok(()),
            None => {
                let semantics = Semantics::new(tree);
                let library = semantics
                    .resolve("Scenarios::StandIn")
                    .and_then(|s| {
                        semantics
                            .features(s)
                            .into_iter()
                            .find(|f| semantics.name(*f) == Some(name))
                    })
                    .ok_or("The Scenarios library is missing.")?;
                let mut feature = Element::new(ElementKind::Reference);
                feature.redefines = vec![Reference::to(library, name)];
                feature.value = literal;
                feature.expression = expression;
                vec![Operation::Create {
                    parent: Parent::Element(stand_in),
                    element: Box::new(feature),
                }]
            }
        };
        match self.submit(Change::new(Actor::Operator, &description, operations)) {
            Some(_) => Ok(()),
            None => Err(self.status.clone()),
        }
    }

    /// The scenarios that verify `requirement`, with their newest results.
    pub fn verifying_scenarios(&self, requirement: ElementId) -> Vec<ScenarioRow> {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return Vec::new();
        };
        let verifies = |scenario: ElementId| {
            tree[scenario].children().iter().any(|objective| {
                tree[*objective].kind == ElementKind::Objective
                    && tree[*objective].children().iter().any(|v| {
                        tree[*v].target.as_ref().and_then(|t| t.target()) == Some(requirement)
                    })
            })
        };
        self.scenario_rows_now()
            .into_iter()
            .filter(|row| verifies(row.id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::app_tests::studio;
    use crate::studio::Sample;
    use agq_simulation::Verdict;

    /// The screening sample, open in a Studio driven without a window.
    fn screening(name: &str) -> (Studio, crate::edit::app_tests::Folder) {
        let (mut app, folder) = studio(name);
        app.create_sample(
            &folder.0.join("Screening"),
            crate::studio::SAMPLE_NAME,
            Sample::Screening,
        );
        let state = app.project.as_ref().expect("the sample is open").state();
        assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
        (app, folder)
    }

    fn scenario(app: &Studio, name: &str) -> ElementId {
        app.project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find(&format!("UrlShortener::{name}"))
            .unwrap_or_else(|| panic!("{name} exists"))
    }

    /// Runs the selected scenario and waits for its result (a background
    /// thread; the tick polls it in the app).
    fn run(app: &mut Studio, mode: Mode) -> RunResult {
        app.start_run(mode);
        let started = Instant::now();
        while !app.poll_runs() {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the run finishes"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        app.runs.result.clone().expect("the result is shown")
    }

    #[test]
    fn scenarios_run_in_the_background_and_their_results_are_kept() {
        let (mut app, _folder) = screening("runs-kept");
        let rows = app.scenario_rows();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names.len(), 7, "{names:?}");
        assert!(
            rows.iter().all(|r| r.latest.is_empty()),
            "nothing has run yet"
        );
        let allowed = scenario(&app, "ShortenAllowed");
        app.select_scenario(allowed);
        assert_eq!(app.panel, crate::studio::Panel::Run);
        let result = run(&mut app, Mode::Model);
        assert_eq!(result.status, RunStatus::Completed, "{:?}", result.stop);
        assert!(result.all_passed(), "{:?}", result.checks);
        assert!(app.result_is_current());
        assert!(app.status.contains("ShortenAllowed"), "{}", app.status);
        // Kept in the app's data, shown in the list as current.
        let rows = app.scenario_rows();
        let row = rows.iter().find(|r| r.id == allowed).unwrap();
        assert_eq!(row.latest.len(), 1);
        assert!(row.latest[0].2, "current");
        // A walkthrough runs nothing and verifies nothing.
        let walkthrough = run(&mut app, Mode::Walkthrough);
        assert_eq!(walkthrough.status, RunStatus::Walkthrough);
        assert!(
            walkthrough
                .checks
                .iter()
                .all(|c| c.verdict == Verdict::NotRun)
        );
        // Choosing the model mode again shows the model result.
        app.runs.mode = Some(Mode::Model);
        app.load_latest_result();
        assert_eq!(app.runs.result.as_ref().unwrap().id, result.id);
    }

    #[test]
    fn a_changed_setting_outdates_results_and_they_are_not_drawn() {
        let (mut app, _folder) = screening("runs-outdated");
        app.select_scenario(scenario(&app, "ReviewRequired"));
        let result = run(&mut app, Mode::Model);
        assert!(result.all_passed(), "{:?}", result.checks);
        app.set_cursor(Some(result.trace.len() / 2));
        let marks = app.surface_marks();
        assert!(marks.values().any(|m| *m == Mark::Current), "{marks:?}");
        // The agent's minimum confidence changes: the result is outdated.
        let tree = app.project.as_ref().unwrap().state().tree();
        let screening = tree.find("UrlShortener::LinkScreening").unwrap();
        let min = tree[screening]
            .children()
            .iter()
            .copied()
            .find(|c| {
                tree[*c]
                    .redefines
                    .iter()
                    .any(|r| r.last_name() == "minConfidence")
            })
            .unwrap();
        app.set_property(
            min,
            agq_system_state::Property::Value(Some(Literal::Real("0.5".into()))),
            "value",
        );
        app.show_result(app.runs.result.clone());
        assert!(
            matches!(app.runs.freshness, Some(Freshness::Outdated(_))),
            "{:?}",
            app.runs.freshness
        );
        assert!(
            app.surface_marks().is_empty(),
            "an outdated trace is not drawn"
        );
        let rows = app.scenario_rows();
        let row = rows.iter().find(|r| r.name == "ReviewRequired").unwrap();
        assert!(!row.latest[0].2, "the list says outdated");
        // Run again: with 0.5, the unsure allow now activates the link and
        // the scenario's check fails, at the element.
        let again = run(&mut app, Mode::Model);
        assert!(!again.all_passed(), "{:?}", again.checks);
        assert!(app.result_is_current());
    }

    #[test]
    fn replay_without_recordings_stops_and_never_calls_a_model() {
        let (mut app, _folder) = screening("runs-replay");
        app.select_scenario(scenario(&app, "ShortenAllowed"));
        let result = run(&mut app, Mode::Replay);
        assert_eq!(result.status, RunStatus::Stopped);
        assert_eq!(
            result.stop.as_ref().map(|s| s.reason),
            Some(StopReason::MissingRecording)
        );
        assert!(!result.all_passed());
    }

    #[test]
    fn the_assistant_runs_scenarios_through_the_studio() {
        use agq_assistant::tools::StudioRequest;
        let (mut app, _folder) = screening("runs-assistant");
        let blocked = scenario(&app, "ShortenBlocked");
        let (reply, answers) = std::sync::mpsc::channel();
        app.carry_out_request(
            StudioRequest::Run {
                scenario: blocked,
                mode: Mode::Model,
            },
            reply,
        );
        // The Operator sees the run the Assistant started.
        assert_eq!(app.runs.selected, Some(blocked));
        assert!(app.runs.running());
        let started = Instant::now();
        while !app.poll_runs() {
            assert!(started.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(5));
        }
        let answer = answers.try_recv().expect("the Assistant has its answer");
        assert!(!answer.is_error, "{}", answer.content);
        assert!(answer.content.contains("completed"), "{}", answer.content);
        // The result is kept: reading it gives the same.
        let (reply, answers) = std::sync::mpsc::channel();
        app.carry_out_request(
            StudioRequest::ReadRun {
                scenario: blocked,
                mode: Mode::Model,
            },
            reply,
        );
        let read = answers.try_recv().unwrap();
        assert!(
            read.content
                .contains("It describes the model as it is now."),
            "{}",
            read.content
        );
        // Code does not run without the Operator's trust: the reply says why.
        let (reply, answers) = std::sync::mpsc::channel();
        app.carry_out_request(
            StudioRequest::Run {
                scenario: blocked,
                mode: Mode::Implementation,
            },
            reply,
        );
        let refused = answers.try_recv().expect("answered at once");
        assert!(refused.is_error, "{}", refused.content);
        assert!(!app.runs.running());
    }

    #[test]
    fn the_sample_with_its_code_runs_its_scenarios_against_the_code() {
        let (mut app, folder) = studio("runs-code");
        app.create_sample(
            &folder.0.join("Shortener"),
            "Shortener",
            Sample::ScreeningWithCode,
        );
        assert!(
            folder.0.join("Shortener-code/src/api.rs").exists(),
            "{}",
            app.status
        );
        let links = app.implementation_links().unwrap();
        assert_eq!(links.links.len(), 23);
        assert!(
            links
                .dangling(app.project.as_ref().unwrap().state().tree())
                .is_empty()
        );
        // Not without trust.
        let allowed = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("Shortener::ShortenAllowed")
            .unwrap();
        app.select_scenario(allowed);
        app.start_run(Mode::Implementation);
        assert!(!app.runs.running());
        assert!(app.status.contains("trusted-local"), "{}", app.status);
        let mut choice = app.execution_choice();
        choice.trusted = true;
        app.set_execution_choice(choice);
        let started = Instant::now();
        app.start_run(Mode::Implementation);
        while !app.poll_runs() {
            assert!(started.elapsed() < Duration::from_secs(600), "the run ends");
            std::thread::sleep(Duration::from_millis(20));
        }
        let result = app.runs.result.clone().unwrap();
        assert_eq!(result.mode, Mode::Implementation);
        assert!(result.all_passed(), "{:?} {:?}", result.stop, result.checks);
        assert!(app.result_is_current());
        // The implementation checks find no drift.
        app.start_checks();
        let started = Instant::now();
        while app.implementation.checking() {
            app.poll_checks();
            assert!(started.elapsed() < Duration::from_secs(600));
            std::thread::sleep(Duration::from_millis(20));
        }
        let report = app.implementation.report.clone().expect("a report");
        assert!(
            report
                .checks
                .iter()
                .all(|c| c.verdict == agq_simulation::Verdict::Passed),
            "{:#?}",
            report.checks
        );
        assert!(app.drift().is_empty());
    }

    #[test]
    fn library_blocks_carry_their_behaviour_and_scenarios() {
        let (mut app, _folder) = studio("runs-library");
        let tree = app.project.as_ref().unwrap().state().tree();
        let package = tree
            .walk()
            .into_iter()
            .find(|id| tree[*id].kind == ElementKind::Package);
        let parent = package.map_or(Parent::Document(0), Parent::Element);
        for block in [
            "Library::Resilience::RetryingWorker",
            "Library::Moderation::ModeratedInbox",
        ] {
            let used = app.use_block(agq_library::Use::new(
                agq_library::BlockRef::new(agq_library::Scope::BuiltIn, block),
                parent,
            ));
            assert!(used.is_some(), "{block}: {}", app.status);
        }
        let problems = app.project.as_ref().unwrap().state().diagnostics().to_vec();
        assert!(problems.is_empty(), "{problems:?}");
        // The blocks' scenarios came with them, and pass against the model.
        let rows = app.scenario_rows();
        let mut names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            [
                "AgentFailsToReview",
                "ConfidentPublish",
                "GiveUp",
                "RetryThenDone"
            ]
        );
        for row in rows {
            app.select_scenario(row.id);
            let result = run(&mut app, Mode::Model);
            assert!(
                result.all_passed(),
                "{}: {:?} {:?}",
                row.name,
                result.stop,
                result.checks
            );
        }
    }

    #[test]
    fn scenarios_are_written_through_controls_and_run() {
        let (mut app, _folder) = screening("runs-authoring");
        let tree = app.project.as_ref().unwrap().state().tree();
        let service = tree.find("UrlShortener::UrlShortenerService").unwrap();
        app.create_scenario(service, "ShortenWhenScreeningTimesOut");
        let new = app.runs.selected.expect("the new scenario is chosen");
        let ports = app.scenario_ports(new);
        let shorten = ports
            .iter()
            .find(|p| p.label.ends_with(".shorten"))
            .unwrap();
        assert_eq!(
            shorten
                .sends
                .iter()
                .map(|t| t.1.as_str())
                .collect::<Vec<_>>(),
            ["ShortenRequest"]
        );
        assert_eq!(
            shorten
                .accepts
                .iter()
                .map(|t| t.1.as_str())
                .collect::<Vec<_>>(),
            ["ShortLink"]
        );
        let targets = app.stand_in_targets(new);
        let (target, _) = targets
            .iter()
            .find(|(_, label)| label.ends_with(".screening"))
            .cloned()
            .expect("the agent can be stood in for");
        app.add_to_scenario(
            new,
            NewStep::StandIn {
                target,
                outcome: "timeout".into(),
            },
        )
        .unwrap();
        app.add_to_scenario(
            new,
            NewStep::Send {
                port: shorten.path.clone(),
                ty: shorten.sends[0].clone(),
            },
        )
        .unwrap();
        app.add_to_scenario(
            new,
            NewStep::Accept {
                port: shorten.path.clone(),
                ty: shorten.accepts[0].clone(),
            },
        )
        .unwrap();
        app.add_to_scenario(
            new,
            NewStep::Check {
                expression: "shortLink.status == LinkStatus::held".into(),
            },
        )
        .unwrap();
        // The request's arguments are an expression on the send step.
        let tree = app.project.as_ref().unwrap().state().tree();
        let send = tree[new]
            .children()
            .iter()
            .copied()
            .find(|c| tree[*c].kind == ElementKind::Send)
            .unwrap();
        let expression = agq_language::parse_expression(
            "new ShortenRequest(longUrl = \"https://example.org\", host = \"example.org\")",
        )
        .unwrap();
        app.set_property(
            send,
            agq_system_state::Property::Expression(Some(expression)),
            "value",
        );
        let problems = app.project.as_ref().unwrap().state().diagnostics().to_vec();
        assert!(problems.is_empty(), "{problems:?}");
        let result = run(&mut app, Mode::Model);
        assert!(result.all_passed(), "{:?} {:?}", result.stop, result.checks);
        let state = app.project.as_ref().unwrap().state();
        // The text reads back as written.
        let text: String = agq_language::print(state.tree())
            .into_iter()
            .map(|source| source.text)
            .collect();
        assert!(
            text.contains("verification def ShortenWhenScreeningTimesOut"),
            "{text}"
        );
        assert!(
            text.contains(":>> outcome = Scenarios::Outcome::timeout;"),
            "{text}"
        );
        // A stand-in's latency set from the Inspector's section.
        let stand_in = state.tree()[new]
            .children()
            .iter()
            .copied()
            .find(|c| stand_in_features(state.tree(), *c).is_some())
            .unwrap();
        app.set_stand_in_feature(stand_in, "latencyMs", "250")
            .unwrap();
        let tree = app.project.as_ref().unwrap().state().tree();
        let features = stand_in_features(tree, stand_in).unwrap();
        assert_eq!(
            features.iter().find(|f| f.0 == "latencyMs").unwrap().2,
            "250"
        );
        assert!(
            app.project
                .as_ref()
                .unwrap()
                .state()
                .diagnostics()
                .is_empty()
        );
    }
}
