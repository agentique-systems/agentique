//! The Orchestrator's driver (C-53, ROADMAP §4.16): deterministic code that
//! takes an objective through its cycles. It decides the next phase from
//! recorded results, runs each role's agent session, checks what the agents
//! hand over, records every outcome and side effect, and stops on a budget
//! used up, on no progress, or when the Operator stops it.
//!
//! Since C-54: a cycle of an objective that explores starts with Explore and
//! Reproduce (`explore`); its criteria are checked on the base for evidence
//! and a user-facing change is evaluated by behaviour (`evidence`); the
//! lead may delegate a child objective, which runs inside this run
//! (`children`); the Operator's messages go to the implementer while it
//! works and otherwise wait for the lead's next turn; and a cycle's
//! worktrees are removed when it ends.

mod children;
mod evidence;
mod explore;
mod trace;

use crate::blockers;
use crate::control::{Client, Options, TestInstance};
use crate::explore::Instance;
use crate::findings::Disposition;
use crate::knowledge::Knowledge;
use crate::record;
use crate::record::{
    Attempt, Check, Continuation, Cost, Criterion, Cycle, DirectiveStatus, Objective, Outcome,
    Phase, Recipient, Review, RoleModel, State, Store,
};
use crate::roles::{self, Role};
use crate::thread::{self, Author, Kind, ThreadEntry};
use crate::traceability::Elements;
use crate::{builds, forge, gates};
use agq_assistant::conversation::{Conversation, Entry, ToolResult};
use agq_assistant::model_tools::WorkingModel;
use agq_assistant::policy::{Development, Gate, Permissions, Place, Policy, Steering, Undecided};
use agq_assistant::runtime::{Runtime, checked};
use agq_assistant::turn::{Activity, ToolCall, Toolset, TurnEvent};
use agq_assistant::{StreamEvent, claude_agent::ClaudeAgent};
use agq_execution::git;
use agq_execution::process::Program;
use agq_execution::{Executor, Scope};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What the Operator tells a running objective.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Pause,
    Step,
    Resume,
    Stop,
    /// Ends the sessions and leaves the objective as it is, to continue
    /// later (the Studio is closing).
    Interrupt,
    /// The Operator's message: to the implementer at its next tool call
    /// while it works, otherwise waiting for the lead's next turn (C-54);
    /// never to the reviewer or the explorer.
    Message(String),
    /// Stops one child objective (its id) running inside this one: its
    /// directive ends as stopped, and the parent's lead goes on with that.
    StopChild(String),
}

/// What the Orchestrator tells the Studio.
pub enum Event {
    /// The record, after each change.
    Changed(Box<Objective>),
    /// An entry added to the objective's thread (C-54): what an agent, the
    /// Operator or the Orchestrator did. Its number is 0 when it could not
    /// be kept.
    Thread(ThreadEntry),
    /// A request for the running Studio's control interface (the lead
    /// observes the application).
    Control { body: Value, reply: Sender<Value> },
    /// The default branch here moved to a merged change: a project open
    /// from this repository must be read again.
    Merged { repository: PathBuf },
    /// Hand over to the adopted build: the Studio saves and exits.
    Adopt {
        build: String,
        reply: Sender<Result<(), String>>,
    },
}

/// Makes a Claude Agent runtime for a role's session, on the role's model,
/// effort and credential as the objective recorded them (C-54).
pub type RuntimeFactory =
    Box<dyn Fn(&RoleModel, &Development) -> Result<ClaudeAgent, String> + Send>;

/// What the Orchestrator works with, given once.
pub struct Setup {
    pub store: Store,
    /// Its own folder: worktrees, checkouts, test instances, a build target.
    pub work: PathBuf,
    /// The builds folder (the launcher's).
    pub builds: PathBuf,
    pub runtime: RuntimeFactory,
    /// Configured keys, never to appear in a change.
    pub keys: Vec<String>,
    /// The project's required checks, as words.
    pub checks: Vec<Vec<String>>,
    /// The links' protected paths.
    pub protected: Vec<String>,
    /// The build this Studio runs, if it runs from the builds folder.
    pub running_build: Option<String>,
    /// How fast agents act in a test instance the Operator may watch
    /// (Settings' `control.speed`: `observe`, `fast` or `instant`; C-54).
    pub speed: String,
    /// A provider's key, for a test instance of a merged build that
    /// explores (C-54): the Studio's credentials; none in tests.
    pub credential: CredentialSource,
    /// What builds the Studio and starts its test instances.
    pub studios: Box<dyn Studios + Send>,
}

/// Where a key for a test instance comes from.
pub type CredentialSource =
    Box<dyn Fn(agq_providers::Credential) -> Option<agq_providers::Secret> + Send>;

/// What builds the Studio for test instances and starts them (C-54):
/// [`Live`] does it for real; tests put a stand-in in its place, as
/// exploration's [`Instance`] boundary allows.
pub trait Studios {
    /// A debug build of the Studio in `checkout` into `target`, holding the
    /// lock of the builds folder `builds`: its executable.
    fn build(
        &self,
        checkout: &Path,
        target: &Path,
        builds: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<PathBuf, String>;

    /// A test instance of `exe`, started fresh from a copy of `start` (a
    /// project) for each run or replay, in `folder`, with `options`.
    fn instance(
        &self,
        exe: &Path,
        start: &Path,
        folder: &Path,
        options: Options,
    ) -> Box<dyn Instance>;
}

/// The real Studio: debug builds with Cargo, test instances as processes.
pub struct Live;

impl Studios for Live {
    fn build(
        &self,
        checkout: &Path,
        target: &Path,
        builds: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        builds::debug_studio(checkout, target, builds, cancel)
    }

    fn instance(
        &self,
        exe: &Path,
        start: &Path,
        folder: &Path,
        options: Options,
    ) -> Box<dyn Instance> {
        Box::new(crate::explore::LiveInstance::with(
            exe, start, folder, options,
        ))
    }
}

/// A running objective, as the Studio holds it.
pub struct Handle {
    commands: Sender<Command>,
    pub events: Receiver<Event>,
    thread: Option<std::thread::JoinHandle<()>>,
    paused: Arc<AtomicBool>,
    controls: Controls,
}

impl Handle {
    pub fn send(&self, command: Command) {
        // Agentique is closing: it takes effect now, not when the commands'
        // thread gets to it, so every save after says so.
        if command == Command::Interrupt {
            self.controls.apply(Command::Interrupt);
        }
        let _ = self.commands.send(command);
    }

    /// Whether the Operator paused it (also while an agent's session holds
    /// at its next tool call, before the record says so).
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Whether the driver has ended.
    pub fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }
}

/// Adds entries to an objective's thread and tells the Studio of each.
#[derive(Clone)]
struct Poster {
    store: Store,
    objective: String,
    events: Sender<Event>,
    /// Configured keys: replaced by a hint in what is written (§4.9).
    keys: Vec<String>,
}

impl Poster {
    /// Adds `entry` and returns it as added (number 0 when it could not be
    /// kept: shown all the same, saying so).
    fn post(&self, mut entry: ThreadEntry) -> ThreadEntry {
        entry.text = thread::redacted(&entry.text, &self.keys);
        entry.details = entry.details.map(|d| thread::redacted(&d, &self.keys));
        let shown = match self.store.append_thread(&self.objective, entry.clone()) {
            Ok(added) => added,
            Err(error) => ThreadEntry {
                objective: self.objective.clone(),
                at: agq_launcher::now(),
                text: format!("{} (not kept in the thread: {error})", entry.text),
                ..entry
            },
        };
        let _ = self.events.send(Event::Thread(shown.clone()));
        shown
    }
}

/// An attempt's outcomes, a line each, as the thread folds them.
fn outcome_lines(attempt: &Attempt) -> String {
    attempt
        .checks
        .iter()
        .chain(&attempt.criteria)
        .chain(&attempt.gates)
        .map(|o| format!("- {}: {}", o.name, o.verdict))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a user-facing change was evaluated in a test instance: one of
/// its behavioural outcomes (an observation or judgment criterion, the
/// replay, the changed areas' exploration) ran there, passing or failing.
fn evaluated(outcomes: &[Outcome], proposal: &record::Proposal, user_facing: &[String]) -> Outcome {
    let name = "the user-facing change was evaluated in a test instance";
    let behavioural: Vec<&str> = proposal
        .criteria
        .iter()
        .filter(|c| !matches!(c.check, Check::Command { .. }))
        .map(|c| c.id.as_str())
        .chain([record::REPLAY, evidence::CHANGED_AREAS])
        .collect();
    let ran: Vec<&str> = outcomes
        .iter()
        .filter(|o| behavioural.contains(&o.name.as_str()))
        .filter(|o| o.verdict == "passed" || o.verdict == "failed")
        .map(|o| o.name.as_str())
        .collect();
    if ran.is_empty() {
        Outcome::new(
            name,
            "failed",
            format!(
                "nothing behavioural ran in a test instance of the change to {}",
                user_facing.join(", ")
            ),
        )
    } else {
        Outcome::new(
            name,
            "passed",
            format!("{} ran for {}", ran.join(", "), user_facing.join(", ")),
        )
    }
}

/// The agent of `role` in `objective`, with the model recorded for it.
fn agent_of(objective: &Objective, role: &str) -> Author {
    Author::agent(
        role,
        objective
            .models
            .iter()
            .find(|m| m.role == role)
            .map(|m| m.model.clone()),
    )
}

/// The Operator's command as its thread shows it.
fn command_entry(command: &Command) -> Option<ThreadEntry> {
    let text = match command {
        // Its entry says where it went: the commands' thread routes it.
        Command::Message(_) => return None,
        Command::StopChild(child) => {
            return Some(ThreadEntry::new(
                Kind::Event,
                Author::Operator,
                format!("Stops the child objective {child}"),
            ));
        }
        Command::Pause => "Paused",
        Command::Step => "One step",
        Command::Resume => "Resumed",
        Command::Stop => "Stopped",
        // Agentique closes: the objective says so as it saves.
        Command::Interrupt => return None,
    };
    Some(ThreadEntry::new(Kind::Event, Author::Operator, text))
}

/// Starts (or resumes) `objective` on its own thread.
pub fn start(setup: Setup, objective: Objective) -> Handle {
    let (commands, received) = std::sync::mpsc::channel();
    let (events, events_received) = std::sync::mpsc::channel();
    let controls = Controls::default();
    if objective.state == State::Paused {
        controls.apply(Command::Pause);
    }
    let paused = controls.paused.clone();
    let held = controls.clone();
    let poster = Poster {
        store: setup.store.clone(),
        objective: objective.id.clone(),
        events: events.clone(),
        keys: setup.keys.clone(),
    };
    // The Operator's commands take effect as they arrive, also while an
    // agent works (its session reads the same steering and stop), and go
    // into the thread.
    let applied = controls.clone();
    let commanded = poster.clone();
    std::thread::spawn(move || {
        let received: Receiver<Command> = received;
        for command in received {
            match command {
                Command::Message(text) => {
                    // To the implementer while it works; otherwise it waits
                    // in the thread for the lead's next turn.
                    let to = if applied.working().as_deref() == Some("implementer") {
                        "implementer"
                    } else {
                        "lead"
                    };
                    let _ = commanded.post(ThreadEntry::message(text.clone(), to));
                    if to == "implementer" {
                        applied.apply(Command::Message(text));
                    }
                }
                command => {
                    if let Some(entry) = command_entry(&command) {
                        let _ = commanded.post(entry);
                    }
                    applied.apply(command);
                }
            }
        }
    });
    if let Err(error) = cargo_settings(&setup.work) {
        let _ = poster.post(ThreadEntry::event(format!(
            "The build settings could not be written: {error}"
        )));
    }
    let thread = std::thread::Builder::new()
        .name("agentique-objective".into())
        .spawn(move || {
            let mut driver = Driver {
                setup: Rc::new(setup),
                objective,
                events,
                poster,
                controls,
                last: Instant::now(),
                delegated: 0,
            };
            driver.run();
        })
        .expect("a thread can start");
    Handle {
        commands,
        events: events_received,
        thread: Some(thread),
        paused,
        controls: held,
    }
}

/// The `work` worktrees of failed or interrupted cycles kept (C-54).
const KEPT_WORK: usize = 3;

/// Makes every file of a checkout newer than anything built before (Cargo
/// judges freshness by file times, and the build folder is shared).
fn freshen(folder: &Path) {
    let now = std::time::SystemTime::now();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|n| n == ".git" || n == "target" || n == "node_modules")
            {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(file) = std::fs::File::options().write(true).open(&path) {
                let _ = file.set_modified(now);
            }
        }
    }
}

/// How many tests a test run says it ran: cargo's `test result:` lines,
/// Node's `# pass` / `ℹ pass`, Python's `Ran N tests`; `None` when it says
/// nothing countable.
fn tests_ran(output: &str) -> Option<usize> {
    let mut total = None;
    let mut add = |n: usize| total = Some(total.unwrap_or(0) + n);
    for line in output.lines() {
        if line.contains("test result:") {
            let passed = number_after(line, ". ").unwrap_or(0);
            let failed = number_after(line, "passed; ").unwrap_or(0);
            add(passed + failed);
        } else if let Some(n) =
            number_after(line, "# pass").or_else(|| number_after(line, "ℹ pass"))
        {
            add(n);
        } else if line.starts_with("Ran ")
            && line.contains(" test")
            && let Some(n) = number_after(line, "Ran ")
        {
            add(n);
        }
    }
    total
}

/// The number written after `marker` in `line`, if any.
fn number_after(line: &str, marker: &str) -> Option<usize> {
    let rest = &line[line.find(marker)? + marker.len()..];
    rest.trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// Cargo's settings for every checkout in the work folder (Cargo reads a
/// folder's `.cargo/config.toml` for the folders below it): no debug
/// information and no incremental state, so the shared build folder stays a
/// few GB, and four jobs, so a build leaves the machine usable.
fn cargo_settings(work: &Path) -> Result<(), String> {
    let folder = work.join(".cargo");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let text = [
        "# Written by Agentique's Orchestrator for the checkouts below (C-53).",
        "[build]",
        "jobs = 4",
        "incremental = false",
        "",
        "[profile.dev]",
        "debug = 0",
        "",
        "[profile.test]",
        "debug = 0",
        "",
    ]
    .join("\n");
    agq_launcher::write_atomically(&folder.join("config.toml"), text.as_bytes())
}

/// Usage without a known price, at a high one ($15 a million tokens in, $75
/// out), so an unpriced model never makes the spend budget blind.
fn unpriced(usage: &agq_assistant::Usage) -> f64 {
    (usage.input_tokens + usage.cache_read_input_tokens + usage.cache_creation_input_tokens) as f64
        * 15.0
        / 1e6
        + usage.output_tokens as f64 * 75.0 / 1e6
}

/// The Operator's commands, as they stand.
#[derive(Clone, Default)]
struct Controls {
    stop: Arc<AtomicBool>,
    /// The stop is an interruption: the record stays as it is.
    interrupted: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    /// One step asked while paused (between phases).
    step: Arc<AtomicBool>,
    /// The session at work's steering (messages and its gate).
    steering: Steering,
    /// The role whose session runs now: the Operator's messages go to the
    /// implementer's.
    working: Arc<Mutex<Option<String>>>,
    /// The stop of each child objective running inside this run, by id.
    children: Arc<Mutex<BTreeMap<String, Arc<AtomicBool>>>>,
}

impl Controls {
    fn working(&self) -> Option<String> {
        self.working.lock().ok().and_then(|w| w.clone())
    }

    fn set_working(&self, role: Option<&str>) {
        if let Ok(mut working) = self.working.lock() {
            *working = role.map(str::to_string);
        }
    }

    /// The controls of child objective `id` running inside this run: the
    /// same pause, steering and interruption, and a stop of its own, which
    /// this run's stop also sets.
    fn child(&self, id: &str) -> Controls {
        let stop = match self.children.lock() {
            // Stopped already (the Operator's stop came first), or new.
            Ok(mut children) => children
                .entry(id.to_string())
                .or_insert_with(|| Arc::new(AtomicBool::new(self.stopped())))
                .clone(),
            Err(_) => Arc::new(AtomicBool::new(self.stopped())),
        };
        Controls {
            stop,
            ..self.clone()
        }
    }

    fn forget(&self, id: &str) {
        if let Ok(mut children) = self.children.lock() {
            children.remove(id);
        }
    }

    fn stop_children(&self) {
        if let Ok(children) = self.children.lock() {
            for stop in children.values() {
                stop.store(true, Ordering::SeqCst);
            }
        }
    }

    fn apply(&self, command: Command) {
        match command {
            Command::Pause => {
                self.paused.store(true, Ordering::SeqCst);
                self.steering.set_gate(Gate::Pause);
            }
            Command::Step => {
                self.step.store(true, Ordering::SeqCst);
                self.steering.set_gate(Gate::Step);
            }
            Command::Resume => {
                self.paused.store(false, Ordering::SeqCst);
                self.steering.set_gate(Gate::Run);
            }
            Command::Stop => {
                self.stop.store(true, Ordering::SeqCst);
                self.stop_children();
                self.steering.set_gate(Gate::Run);
            }
            Command::Interrupt => {
                self.interrupted.store(true, Ordering::SeqCst);
                self.stop.store(true, Ordering::SeqCst);
                self.stop_children();
                self.steering.set_gate(Gate::Run);
            }
            Command::StopChild(id) => {
                // Kept for a child about to start, too.
                if let Ok(mut children) = self.children.lock() {
                    children
                        .entry(id)
                        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
                        .store(true, Ordering::SeqCst);
                }
            }
            Command::Message(text) => self.steering.queue(&text),
        }
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

struct Driver {
    /// Shared with the child objectives that run inside this run.
    setup: Rc<Setup>,
    objective: Objective,
    events: Sender<Event>,
    poster: Poster,
    controls: Controls,
    /// Since when time worked is not yet counted.
    last: Instant,
    /// Children the lead delegated in its current turn.
    delegated: usize,
}

/// What a role's session handed over.
struct Session {
    submitted: Option<Value>,
    /// The lead's proposal, as the Orchestrator accepted it.
    proposal: Option<record::Proposal>,
    /// Why the lead's last proposal was not accepted, if it was not.
    refusal: Option<String>,
    /// What the lead judged findings to be (C-55), by identity.
    adjudicated: Vec<(String, Disposition)>,
    /// What the session said last, for a report.
    said: String,
    /// The child objective the lead delegated, checked (C-54).
    delegated: Option<children::Asked>,
    /// Delegations the Orchestrator refused, with why.
    refused: Vec<(Value, String)>,
}

/// What a session works with besides its role's own: the test instance its
/// control tools operate, the lead's tools for an exploring cycle, the
/// reproduced findings it may choose among, the base commit's model a
/// proposal's names are resolved in (C-55), and the lead's planner, which
/// checks a plan of an exploration and a child's project (the W13.7
/// repair).
#[derive(Default)]
struct With<'a> {
    test: Option<&'a mut Client>,
    kit: Option<Toolset>,
    offered: Vec<(String, String)>,
    model: Option<&'a Result<Elements, String>>,
    planner: Option<&'a explore::Planner>,
}

/// What an exploration by the rules decides with: they ask no model, so
/// the models named are never used.
fn by_rules(answers: &crate::decide::Decider) -> crate::explore::Deciding<'_> {
    let none = agq_providers::ModelRef::new(agq_providers::Provider::DeepSeek, "none");
    crate::explore::Deciding {
        answers,
        explorer: none.clone(),
        effort: None,
        escalation: none,
        escalation_effort: None,
    }
}

/// What supervises an exploration or a replay in the driver: the
/// Operator's Pause holds it between steps (Step lets one through), Stop
/// ends it, and its progress goes to the thread when it has a report.
struct Watch {
    controls: Controls,
    report: Option<Report>,
}

/// Where a run's progress goes (the W13.7 repair): the objective's thread,
/// as the explorer's activity folded under the entry that started it.
struct Report {
    poster: Poster,
    author: Author,
    under: Option<u64>,
    directive: Option<String>,
}

impl crate::explore::Supervisor for Watch {
    fn go_on(&mut self) -> bool {
        while self.controls.paused.load(Ordering::SeqCst) && !self.controls.stopped() {
            if self.controls.step.swap(false, Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        !self.controls.stopped()
    }

    fn stopped(&mut self) -> bool {
        self.controls.stopped()
    }

    fn progress(&mut self, progress: &crate::explore::Progress) {
        let Some(report) = &self.report else {
            return;
        };
        let (text, details) = progress.entry();
        let mut entry = ThreadEntry::new(Kind::Activity, report.author.clone(), text)
            .with_details(details)
            .for_directive(report.directive.as_deref());
        entry.under = report.under;
        report.poster.post(entry);
    }
}

type Next = Result<Phase, String>;

/// Why a blocked change was not carried onto its repair (W13.7).
enum Carry {
    /// It is not carried: the thread says why, and it stays on its pull
    /// request.
    NotCarried(String),
    /// The Operator stopped it, or Agentique closed: taken again later.
    Stopped,
    /// The host or the working copy is in the way: the Operator resumes it.
    Paused(String),
}

impl Driver {
    /// Agentique's event in the thread.
    fn event(&self, text: impl Into<String>) {
        self.poster.post(ThreadEntry::event(text));
    }

    fn post(&self, entry: ThreadEntry) -> ThreadEntry {
        self.poster.post(entry)
    }

    /// The agent of `role`, with the model the objective recorded for it.
    fn agent(&self, role: &str) -> Author {
        agent_of(&self.objective, role)
    }

    /// Records a directive of `author` for `recipient` and shows it in the
    /// thread as `text`, with `details` (what it hands over) folded under
    /// it; returns its id.
    fn direct(
        &mut self,
        author: &str,
        recipient: Recipient,
        scope: record::Scope,
        refers_to: Option<String>,
        text: String,
        details: String,
    ) -> String {
        let id = self.objective.direct(author, recipient, scope, refers_to);
        self.save();
        self.post(
            ThreadEntry::new(Kind::Directive, self.agent(author), text)
                .with_details(details)
                .for_directive(Some(&id)),
        );
        id
    }

    fn save(&mut self) {
        self.count_time();
        // Interrupted because the Operator closed Agentique: every save from
        // now says so (the Studio marks the record too as it closes).
        if self.controls.interrupted.load(Ordering::SeqCst) {
            self.objective.interrupted = true;
        }
        if let Err(error) = self.setup.store.save(&self.objective) {
            self.event(format!(
                "The objective's record could not be saved: {error}"
            ));
        }
        let _ = self
            .events
            .send(Event::Changed(Box::new(self.objective.clone())));
    }

    fn id(&self) -> String {
        self.objective.id.clone()
    }

    fn cycle(&self) -> &Cycle {
        self.objective.cycle().expect("a cycle runs")
    }

    fn cycle_mut(&mut self) -> &mut Cycle {
        self.objective.cycle_mut().expect("a cycle runs")
    }

    fn folder(&self, name: &str) -> PathBuf {
        self.setup
            .work
            .join(&self.objective.id)
            .join(format!("cycle-{}", self.cycle().n))
            .join(name)
    }

    /// One build folder for every objective's agents, checks and test
    /// instances, so each builds only what changed.
    fn target(&self) -> PathBuf {
        self.setup.work.join("target")
    }

    fn count_time(&mut self) {
        self.objective.spent.seconds += self.last.elapsed().as_secs_f64();
        self.last = Instant::now();
    }

    /// Between phases: while paused, waits for Resume, Step or Stop (the
    /// time paused does not count).
    fn hold(&mut self) {
        let controls = self.controls.clone();
        while controls.paused.load(Ordering::SeqCst) && !controls.stopped() {
            if controls.step.swap(false, Ordering::SeqCst) {
                break;
            }
            if self.objective.state == State::Running {
                self.objective.state = State::Paused;
                self.save();
            }
            std::thread::sleep(Duration::from_millis(200));
            self.last = Instant::now();
        }
        if !controls.paused.load(Ordering::SeqCst) && self.objective.state == State::Paused {
            self.objective.state = State::Running;
            self.objective.note = None;
            self.save();
        }
    }

    /// Why the objective cannot go on: a budget used up, or the Operator's
    /// stop; and the state it ends in.
    fn over(&mut self) -> Option<(State, String)> {
        self.count_time();
        let budgets = &self.objective.budgets;
        let spent = &self.objective.spent;
        if let Some(usd) = budgets.usd
            && spent.usd >= usd
        {
            return Some((
                State::Failed,
                format!(
                    "The spend budget is used up (${:.2} of ${usd:.2}).",
                    spent.usd
                ),
            ));
        }
        let hours = spent.seconds / 3600.0;
        if let Some(limit) = budgets.hours
            && hours >= limit
        {
            return Some((
                State::Failed,
                format!("The time budget is used up ({hours:.1} of {limit} hours)."),
            ));
        }
        if self.controls.stopped() {
            return Some((State::Stopped, "Stopped by the Operator.".into()));
        }
        None
    }

    /// The thread's first entries: the intent, as its author gave it, and
    /// each role's model with why a fallback was taken; or, for an
    /// objective going on, where it goes on from.
    fn opening(&mut self) {
        let id = self.id();
        if self.setup.store.thread_last(&id) > 0 {
            let at = self
                .objective
                .cycle()
                .map(|c| format!("cycle {}, {}", c.n, c.phase.label().to_lowercase()))
                .unwrap_or_else(|| "its start".into());
            self.event(format!("Going on from {at}"));
            return;
        }
        let author = match &self.objective.requested_by {
            Some(by) => agent_of(&self.objective, &by.role),
            None => Author::Operator,
        };
        self.post(ThreadEntry::new(
            Kind::Human,
            author,
            self.objective.intent.clone(),
        ));
        // What it does, as inferred from the intent and as started
        // (the Operator's amendment of C-54).
        let objective = &self.objective;
        let started = crate::decide::Shape {
            explore: objective.explore,
            cycles: objective.budgets.cycles,
            merge: objective.permissions.merge,
            adopt: objective.permissions.adopt,
        };
        if let Some(inferred) = objective.inferred.clone() {
            let changed = if inferred.shape == started {
                String::new()
            } else {
                format!(". You changed it: it {}", started.describe())
            };
            self.event(format!(
                "{} It {}{changed}",
                inferred.by(),
                inferred.shape.describe()
            ));
        }
        let budgets = &self.objective.budgets;
        if self.objective.parent.is_none() && budgets.usd.is_none() && budgets.hours.is_none() {
            self.event(
                "No spend or time limit is set: it ends after its improvements (each made or not), when two explorations in a row reproduce nothing new, or when you stop it",
            );
        }
        for model in self.objective.models.clone() {
            let why = model
                .fallback
                .as_ref()
                .map(|why| format!(" (instead of {}: {why})", model.configured))
                .unwrap_or_default();
            self.event(format!(
                "The {} runs on {}{why}; billed {}",
                model.role,
                model.label(),
                model.billed
            ));
        }
        for (role, why) in self.objective.roles_unavailable.clone() {
            self.event(format!(
                "The {role} has no model, which this objective does not need: {why}"
            ));
        }
    }

    fn run(&mut self) {
        self.opening();
        if self.objective.interrupted {
            // Continued: it was interrupted when Agentique closed.
            self.objective.interrupted = false;
            self.save();
        }
        if self.objective.continuation.is_some()
            && let Some(cycle) = self.objective.cycle_mut()
        {
            cycle.phase = Phase::Resume;
        }
        loop {
            self.hold();
            if self.interrupted() {
                return;
            }
            if let Some((state, reason)) = self.over() {
                self.end(state, reason);
                return;
            }
            let phase = match self.objective.cycle().map(|c| c.phase) {
                None | Some(Phase::Done) => {
                    let done = self.improvements();
                    if self.objective.parent.is_some() && done > 0 {
                        // A child explores once; its result goes back.
                        let result = children::child_result(&self.objective);
                        self.end(State::Done, result);
                        return;
                    }
                    if self.objective.explore
                        && self.empty_explorations() >= 2
                        && self.untried().is_empty()
                    {
                        self.end(
                            State::Done,
                            "Nothing new reproduced: two explorations in a row found no new problem that reproduced.".into(),
                        );
                        return;
                    }
                    if done >= self.objective.budgets.cycles {
                        let adopted = self.objective.cycles.iter().filter(|c| c.adopted).count();
                        self.end(
                            State::Done,
                            format!("{done} cycle(s) done, {adopted} adopted."),
                        );
                        return;
                    }
                    self.new_cycle(self.objective.cycles.len() as u32 + 1);
                    Phase::Propose
                }
                Some(Phase::Failed) => {
                    let blocker = self.cycle().blocker.clone().unwrap_or_default();
                    // A reviewed change the checks blocked elsewhere: a cycle
                    // repairs the cause first, when the objective may merge.
                    if let Some(blocked) = self.repair_due() {
                        self.new_repair_cycle(blocked);
                        Phase::Propose
                    } else {
                        let done = self.improvements();
                        if done >= self.objective.budgets.cycles || self.objective.parent.is_some()
                        {
                            self.end(State::Failed, blocker);
                            return;
                        }
                        self.new_cycle(self.objective.cycles.len() as u32 + 1);
                        Phase::Propose
                    }
                }
                Some(phase) => phase,
            };
            self.event(format!(
                "Cycle {}: {}",
                self.cycle().n,
                self.cycle().label()
            ));
            let next = match phase {
                Phase::Propose => match self.cycle().exploring {
                    Some(record::Exploring::Explore) => self.explore(),
                    Some(record::Exploring::Reproduce) => self.reproduce(),
                    None => self.propose(),
                },
                Phase::Implement => self.implement(String::new()),
                Phase::Check => self.check(),
                Phase::Evaluate => self.evaluate(),
                Phase::Review => self.review(),
                Phase::Repair => self.repair(),
                Phase::Merge => self.merge(),
                Phase::Build => self.build(),
                Phase::Try => self.trial(),
                Phase::Adopt => match self.adopt() {
                    // Handed over: this Studio ends; the adopted build goes on.
                    Ok(None) => return,
                    Ok(Some(next)) => Ok(next),
                    Err(error) => Err(error),
                },
                Phase::Resume => match self.resume() {
                    Ok(next) => Ok(next),
                    // The adopted build did not take over: this build goes
                    // no further on top of it.
                    Err(problem) => {
                        self.cycle_mut().blocker = Some(problem.clone());
                        self.cycle_mut().phase = Phase::Failed;
                        self.end(State::Failed, problem);
                        return;
                    }
                },
                Phase::Done | Phase::Failed => continue,
            };
            match next {
                Ok(next) => {
                    self.cycle_mut().phase = next;
                    // It got further: a resume after this is a fresh one.
                    self.objective.resumes = 0;
                    if next == Phase::Done {
                        self.objective
                            .settle_running(DirectiveStatus::Done, "the cycle is done");
                        self.clean_cycle(false);
                    }
                    self.save();
                }
                Err(_) if self.interrupted() => return,
                Err(_) if self.controls.stopped() => {
                    let (state, reason) =
                        self.over().unwrap_or((State::Stopped, "Stopped.".into()));
                    self.end(state, reason);
                    return;
                }
                Err(blocker) if blocker.starts_with("paused:") => {
                    // Waits for the Operator (their working copy, say), then
                    // tries the same phase again.
                    let why = blocker.trim_start_matches("paused:").trim().to_string();
                    self.event(format!("Paused until you resume it: {why}"));
                    self.objective.state = State::Paused;
                    self.objective.note = Some(why);
                    self.controls.apply(Command::Pause);
                    self.save();
                }
                Err(blocker) => {
                    self.event(format!("Cycle {} stopped: {blocker}", self.cycle().n));
                    self.objective
                        .settle_running(DirectiveStatus::Failed, &blocker);
                    let cycle = self.cycle_mut();
                    cycle.blocker = Some(blocker);
                    cycle.phase = Phase::Failed;
                    self.clean_cycle(true);
                    self.save();
                }
            }
        }
    }

    /// Starts cycle `n`: exploring first when the objective explores.
    fn new_cycle(&mut self, n: u32) {
        let mut cycle = Cycle::new(n);
        if self.objective.explore {
            cycle.exploring = Some(record::Exploring::Explore);
        }
        self.objective.cycles.push(cycle);
        self.save();
    }

    fn improvements(&self) -> u32 {
        self.objective.improvements()
    }

    fn repair_due(&self) -> Option<u32> {
        self.objective.repair_due()
    }

    fn new_repair_cycle(&mut self, blocked: u32) {
        let n = self.objective.start_repair(blocked);
        self.save();
        self.event(format!(
            "Cycle {n} repairs what blocks cycle {blocked}'s reviewed change; once it is merged, that change is carried onto it"
        ));
    }

    /// The cycle's worktrees and folders, removed as it ends (C-54): all of
    /// them, but its `work` worktree when `keep_work` (a failed or
    /// interrupted cycle's, of which the three most recent are kept), each
    /// by its name.
    fn clean_cycle(&mut self, keep_work: bool) {
        let Some(n) = self.objective.cycle().map(|c| c.n) else {
            return;
        };
        let repository = self.objective.repository.clone();
        let id = self.id();
        for name in ["lead", "base", "base-tests", "verify", "trial", "model"] {
            let _ = git::remove_worktree(&repository, &format!("{id}-{n}-{name}"));
        }
        let cycle_folder = self.folder("work");
        let cycle_folder = cycle_folder.parent().unwrap_or(&cycle_folder).to_path_buf();
        for entry in std::fs::read_dir(&cycle_folder)
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry.file_name() != "work" {
                crate::control::remove_folder(&entry.path());
            }
        }
        if !keep_work {
            self.remove_work(false);
        }
        // Each by its name: the Operator's own worktrees are never touched.
        self.prune_kept_work();
    }

    /// The cycle's `work` worktree, removed (its branch stays).
    fn remove_work(&mut self, _merged: bool) {
        let repository = self.objective.repository.clone();
        let n = self.cycle().n;
        let name = format!("{}-{n}", self.id());
        let _ = git::remove_worktree(&repository, &name);
        crate::control::remove_folder(&self.folder("work"));
    }

    /// The merged branch, deleted here if the host's merge did not, and
    /// what became of it here and on the host, said in the thread.
    fn delete_branch(&mut self) {
        let Some(branch) = self.cycle().branch.clone() else {
            return;
        };
        let repository = self.objective.repository.clone();
        let here = forge::run(
            &repository,
            &[
                "git",
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ],
            Duration::from_secs(60),
        )
        .is_ok();
        let local = if !here {
            "deleted here".to_string()
        } else {
            match forge::run(
                &repository,
                &["git", "branch", "-D", &branch],
                Duration::from_secs(60),
            ) {
                Ok(_) => "deleted here".to_string(),
                Err(error) => format!("kept here ({error})"),
            }
        };
        let remote = match forge::run(
            &repository,
            &["git", "ls-remote", "--heads", "origin", &branch],
            Duration::from_secs(120),
        ) {
            Ok(listed) if listed.stdout.trim().is_empty() => "deleted on the host".to_string(),
            Ok(_) => "kept on the host".to_string(),
            Err(error) => format!("not known on the host ({error})"),
        };
        self.event(format!("The merged branch {branch}: {local}, {remote}"));
    }

    /// Whose the failed checks are (W13.7): each failed job's log read into
    /// its failing tests, and those placed against the crates the change
    /// from `base` to `commit` touches and affects (its paths without rename
    /// detection, so a moved file counts where it was and where it is). A
    /// log or workspace that cannot be read leaves the failure the change's,
    /// but a cancelled check whose log cannot be read is the machinery's.
    fn attribute(
        &self,
        what: &str,
        base: &str,
        commit: &str,
    ) -> (blockers::CiFailure, blockers::Attribution) {
        let repository = self.objective.repository.clone();
        let folder = self
            .cycle()
            .worktree
            .clone()
            .filter(|w| w.is_dir())
            .unwrap_or_else(|| repository.clone());
        let paths = forge::run(
            &repository,
            &["git", "diff", "--name-only", "--no-renames", base, commit],
            Duration::from_secs(120),
        )
        .map(|listed| {
            listed
                .stdout
                .lines()
                .map(|l| l.trim().replace('\\', "/"))
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        });
        let workspace = blockers::Workspace::read(&folder);
        let mut failures = Vec::new();
        let mut attributions = Vec::new();
        for line in what.lines().filter(|l| !l.trim().is_empty()) {
            let check = line.split(' ').next().unwrap_or("a check").to_string();
            let cancelled = line.contains(" cancel ");
            let failure = match forge::failed_log(&repository, line) {
                Ok(log) => blockers::CiFailure {
                    cancelled,
                    ..blockers::parse_failed_log(&check, &log)
                },
                Err(error) => {
                    attributions.push(if cancelled {
                        blockers::Attribution::Infrastructure(format!(
                            "{check} was cancelled before it reached a verdict"
                        ))
                    } else {
                        blockers::Attribution::Change(format!(
                            "the log of {check} could not be read: {error}"
                        ))
                    });
                    failures.push(blockers::CiFailure {
                        check,
                        cancelled,
                        ..blockers::CiFailure::default()
                    });
                    continue;
                }
            };
            attributions.push(match (&paths, &workspace) {
                (Ok(paths), Ok(workspace)) => {
                    blockers::attribute(&failure, paths, workspace, &folder)
                }
                (Err(error), _) => blockers::Attribution::Change(format!(
                    "the paths the change touches could not be read: {error}"
                )),
                (_, Err(error)) => blockers::Attribution::Change(format!(
                    "the workspace's crates could not be read: {error}"
                )),
            });
            failures.push(failure);
        }
        if attributions.is_empty() {
            attributions.push(blockers::Attribution::Change(
                "the host named no failed check".into(),
            ));
        }
        (
            blockers::CiFailure::joined(failures),
            blockers::combined(attributions),
        )
    }

    /// After this repair cycle merged as `repaired`: the change it unblocks,
    /// carried onto `repaired` (see [`Driver::carry`]). Done once: a cycle
    /// that carried it already goes on. Not carried (the repair does not
    /// touch what failed, the patch changed, or the checks fail on it): the
    /// thread says why and the change stays on its pull request. Stopped or
    /// interrupted, or the host or the working copy in the way: an error, so
    /// the merge phase is taken again (each side effect is done once).
    fn carry_blocked(&mut self, repaired: &str) -> Result<(), String> {
        let Some(n) = self.cycle().repairs else {
            return Ok(());
        };
        if let Some(carried) = self.cycle().carried.clone() {
            // Merged already: the local branch follows it (again, after a
            // pause), so the next cycle starts from it.
            let repository = self.objective.repository.clone();
            let base_branch = self.objective.base_branch.clone();
            return forge::follow(&repository, &base_branch, &carried).map_err(|problem| {
                format!("paused: the local branch could not follow: {problem}")
            });
        }
        match self.carry(n, repaired) {
            Ok(_) => Ok(()),
            Err(Carry::NotCarried(why)) => {
                if let Some(b) = self
                    .objective
                    .cycles
                    .iter_mut()
                    .find(|c| c.n == n)
                    .and_then(|c| c.blocked.as_mut())
                {
                    b.note = why.clone();
                }
                self.save();
                self.event(format!(
                    "Cycle {n}'s change is not carried onto the repair: {why}; it stays on its pull request for you"
                ));
                Ok(())
            }
            Err(Carry::Stopped) => {
                let pushed = self
                    .objective
                    .cycles
                    .iter()
                    .find(|c| c.n == n)
                    .and_then(|c| c.blocked.as_ref())
                    .and_then(|b| b.carried_as.clone());
                if let Some(pushed) = pushed {
                    self.event(format!(
                        "Cycle {n}'s change, carried onto the repair as {}, stays pushed on its pull request, not merged",
                        builds::short(&pushed)
                    ));
                }
                Err("stopped".into())
            }
            Err(Carry::Paused(why)) => Err(format!("paused: {why}")),
        }
    }

    /// Carries cycle `n`'s reviewed change onto `repaired`: only when the
    /// repair touches what failed (otherwise it would only run the failing
    /// checks again), and its patch is unchanged on the new base; pushed onto
    /// its pull request without forcing, merged once the repository's checks
    /// pass on exactly that commit (their failure is attributed again).
    fn carry(&mut self, n: u32, repaired: &str) -> Result<String, Carry> {
        let missing = |what: &str| Carry::NotCarried(format!("cycle {n} has no {what}"));
        let blocked = self
            .objective
            .cycles
            .iter()
            .find(|c| c.n == n)
            .cloned()
            .ok_or_else(|| missing("record"))?;
        let b = blocked.blocked.clone().ok_or_else(|| missing("blocker"))?;
        let base = blocked.base.clone().ok_or_else(|| missing("base"))?;
        let reviewed = blocked
            .review
            .as_ref()
            .map(|r| r.commit.clone())
            .ok_or_else(|| missing("review"))?;
        let pr = blocked
            .pull_request
            .clone()
            .ok_or_else(|| missing("pull request"))?;
        let branch = blocked.branch.clone().ok_or_else(|| missing("branch"))?;
        let pushed = blocked
            .pushed
            .clone()
            .ok_or_else(|| missing("pushed commit"))?;
        let proposal = blocked
            .proposal
            .clone()
            .ok_or_else(|| missing("proposal"))?;
        let repository = self.objective.repository.clone();
        let base_branch = self.objective.base_branch.clone();
        let id = self.id();
        let paused = |what: &str, error: String| Carry::Paused(format!("{what}: {error}"));
        // The repair must touch what failed: its own merge's change (a
        // squash merge, one parent).
        let repair_paths = forge::run(
            &repository,
            &[
                "git",
                "diff",
                "--name-only",
                "--no-renames",
                &format!("{repaired}~1"),
                repaired,
            ],
            Duration::from_secs(120),
        )
        .map_err(|e| paused("the repair's paths could not be read", e))?;
        let paths: Vec<String> = repair_paths
            .stdout
            .lines()
            .map(|l| l.trim().replace('\\', "/"))
            .filter(|l| !l.is_empty())
            .collect();
        let workspace = blockers::Workspace::read(&repository)
            .map_err(|e| paused("the workspace's crates could not be read", e))?;
        blockers::repairs_what_failed(&b.failure.tests, &paths, &workspace).map_err(|why| {
            Carry::NotCarried(format!(
                "{why}, so carrying the change onto it would only run the failing checks again"
            ))
        })?;
        let tree = blockers::carry_over(&repository, &base, &reviewed, repaired)
            .map_err(Carry::NotCarried)?;
        let message = format!(
            "{}\n\nObjective {id}, cycle {n}: the reviewed commit {}, carried onto {} after cycle {} repaired what blocked it.",
            proposal.title,
            builds::short(&reviewed),
            builds::short(repaired),
            self.cycle().n
        );
        let gate = gates::keys_in("the commit message", &message, &self.setup.keys);
        if !gate.passed() {
            return Err(Carry::NotCarried(gate.detail));
        }
        let carried = self
            .setup
            .store
            .once(
                &id,
                &format!("cycle-{n}/carry-{}", builds::short(repaired)),
                || forge::commit_on(&repository, &tree, &[&pushed, repaired], &message),
            )
            .map_err(|e| paused("the carried commit could not be made", e))?;
        self.setup
            .store
            .once(
                &id,
                &format!("cycle-{n}/push-{}", builds::short(&carried)),
                || {
                    forge::push_commit(&repository, &carried, &branch, &base_branch)
                        .map(|_| String::new())
                },
            )
            .map_err(|e| paused("the carried commit could not be pushed", e))?;
        if let Some(cycle) = self.objective.cycles.iter_mut().find(|c| c.n == n) {
            cycle.pushed = Some(carried.clone());
            if let Some(b) = cycle.blocked.as_mut() {
                b.carried_as = Some(carried.clone());
            }
        }
        self.save();
        self.event(format!(
            "Cycle {n}'s reviewed change, its patch unchanged, carried onto {} as {} on pull request #{}; waiting for the repository's checks",
            builds::short(repaired),
            builds::short(&carried),
            pr.number
        ));
        let since = Instant::now();
        while forge::head(&repository, pr.number).ok().as_deref() != Some(carried.as_str()) {
            if self.controls.stopped() {
                return Err(Carry::Stopped);
            }
            if since.elapsed() > Duration::from_secs(180) {
                return Err(Carry::Paused(
                    "the pull request does not show the carried commit".into(),
                ));
            }
            std::thread::sleep(Duration::from_secs(5));
        }
        let stop = self.controls.stop.clone();
        let checks = forge::wait_for_checks(&repository, pr.number, forge::CHECKS_WITHIN, &|| {
            stop.load(Ordering::SeqCst)
        });
        match checks {
            Ok(forge::Checks::Passed) => {}
            Ok(forge::Checks::Failed(what)) => {
                // Attributed again: on the repaired base the change itself
                // may fail now.
                let (_, attribution) = self.attribute(&what, repaired, &carried);
                return Err(match attribution {
                    blockers::Attribution::Change(why) => Carry::NotCarried(format!(
                        "the repository's checks fail on the carried change, and the change may have caused it ({why})"
                    )),
                    blockers::Attribution::Elsewhere(why) => Carry::NotCarried(format!(
                        "the repository's checks fail on the carried change again, not because of it ({why})"
                    )),
                    // No verdict says nothing about the change: it waits.
                    blockers::Attribution::Infrastructure(why) => Carry::Paused(format!(
                        "the repository's checks reached no verdict on the carried change ({why})"
                    )),
                });
            }
            Ok(forge::Checks::Pending) => {
                return Err(Carry::Paused(
                    "the repository's checks did not finish in time".into(),
                ));
            }
            Err(_) if self.controls.stopped() => return Err(Carry::Stopped),
            Err(error) => return Err(paused("the repository's checks could not be read", error)),
        }
        let subject = format!("{} (#{})", proposal.title, pr.number);
        let _ = git::remove_worktree(&repository, &format!("{id}-{n}"));
        let merged = self
            .setup
            .store
            .once(
                &id,
                &format!("cycle-{n}/merge-{}", builds::short(&carried)),
                || forge::merge(&repository, pr.number, &carried, &subject),
            )
            .map_err(|e| paused("the carried change could not be merged", e))?;
        if let Some(cycle) = self.objective.cycles.iter_mut().find(|c| c.n == n) {
            cycle.merged = Some(merged.clone());
            cycle.phase = Phase::Done;
            cycle.blocker = None;
        }
        // What this cycle builds and adopts from now on.
        self.cycle_mut().carried = Some(merged.clone());
        // The finding it fixed is fixed.
        if let Some(finding) = blocked.replay.clone() {
            let file = crate::knowledge::Knowledge::file(&self.setup.store, &repository);
            let project = crate::knowledge::Knowledge::key(&repository);
            if let Err(error) = crate::knowledge::Knowledge::change(&file, &project, |k| {
                k.fixed(&finding.identity, &merged, Some(pr.number))
            }) {
                self.event(format!(
                    "The testing knowledge could not record the fix: {error}"
                ));
            }
        }
        self.save();
        self.event(format!(
            "merged cycle {n}'s change (#{}) as {}; this cycle builds and adopts it",
            pr.number,
            builds::short(&merged)
        ));
        forge::follow(&repository, &base_branch, &merged).map_err(|problem| {
            paused(
                &format!(
                    "merged as {}, but the local branch could not follow",
                    builds::short(&merged)
                ),
                problem,
            )
        })?;
        Ok(merged)
    }

    /// The `work` worktrees kept from failed or interrupted cycles, of all
    /// objectives: the three most recent stay, the others go.
    fn prune_kept_work(&self) {
        let mut kept: Vec<(std::time::SystemTime, String, PathBuf)> = Vec::new();
        for objective in std::fs::read_dir(&self.setup.work)
            .into_iter()
            .flatten()
            .flatten()
        {
            for cycle in std::fs::read_dir(objective.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let work = cycle.path().join("work");
                let Some(n) = cycle
                    .file_name()
                    .to_str()
                    .and_then(|c| c.strip_prefix("cycle-"))
                    .map(str::to_string)
                else {
                    continue;
                };
                if work.is_dir() {
                    let at = std::fs::metadata(&work)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    let name = format!("{}-{n}", objective.file_name().to_string_lossy());
                    kept.push((at, name, work));
                }
            }
        }
        kept.sort_by_key(|k| std::cmp::Reverse(k.0));
        for (_, name, work) in kept.into_iter().skip(KEPT_WORK) {
            let _ = git::remove_worktree(&self.objective.repository, &name);
            crate::control::remove_folder(&work);
        }
    }

    /// Interrupted: the phase reached is saved, to go on when the
    /// objective is continued.
    fn interrupted(&mut self) -> bool {
        if !self.controls.interrupted.load(Ordering::SeqCst) {
            return false;
        }
        self.objective.note = Some(
            "Interrupted when Agentique closed; continue it from the Objectives panel.".into(),
        );
        self.objective.interrupted = true;
        self.event("Interrupted when Agentique closed; it goes on when you continue it");
        self.save();
        true
    }

    fn end(&mut self, state: State, note: String) {
        // A cycle still under way when it stops keeps its work (the three
        // most recent are kept); the rest of its worktrees go.
        if self
            .objective
            .cycle()
            .is_some_and(|c| !matches!(c.phase, Phase::Done | Phase::Failed))
        {
            self.clean_cycle(true);
        }
        self.event(note.clone());
        let settled = match state {
            State::Stopped => DirectiveStatus::Stopped,
            State::Done => DirectiveStatus::Done,
            _ => DirectiveStatus::Failed,
        };
        self.objective.settle_running(settled, &note);
        self.objective.state = state;
        self.objective.note = Some(note);
        self.save();
    }

    /// Runs one role's session in `cwd` under `policy`, with `brief` as its
    /// message (resuming its earlier session there when `resume`). Its model
    /// tools work on the model in `cwd`; `test` is the test instance its
    /// control tools operate.
    fn session(
        &mut self,
        role: Role,
        cwd: &Path,
        policy: Policy,
        brief: String,
        resume: bool,
        with: With,
    ) -> Result<Session, String> {
        let With {
            mut test,
            kit,
            offered,
            model: base_model,
            planner,
        } = with;
        // What the lead may delegate now, as its `delegate` tool checks it.
        let bounds = self.bounds();
        // What the lead's submissions are checked against (C-55): the base
        // commit's model, and what findings were judged to be, its own
        // judgments in this session included.
        let unread: Result<Elements, String> =
            Err("the base commit's model was not read for this session".into());
        let base_model = base_model.unwrap_or(&unread);
        let dispositions = RefCell::new(if role == Role::Lead {
            self.dispositions()
        } else {
            BTreeMap::new()
        });
        let cycle_findings: Vec<crate::findings::Finding> = self
            .objective
            .cycle()
            .map(|c| c.findings.clone())
            .unwrap_or_default();
        let (objective_id, cycle_n) = (self.id(), self.objective.cycle().map_or(0, |c| c.n));
        let knowledge_file = Knowledge::file(&self.setup.store, &self.objective.repository);
        let knowledge_key = Knowledge::key(&self.objective.repository);
        let accepted: RefCell<Option<record::Proposal>> = RefCell::new(None);
        let refusal: RefCell<Option<String>> = RefCell::new(None);
        let adjudicated: RefCell<Vec<(String, Disposition)>> = RefCell::new(Vec::new());
        let mut model = WorkingModel::new(cwd.to_path_buf());
        if role == Role::Implementer {
            model = model.confirming(self.objective.permissions.locked.clone());
        }
        let development = Development {
            cwd: cwd.to_path_buf(),
            policy,
            setting_sources: vec!["project".into()],
            agents: json!({}),
            preset: true,
            // One build folder for the objective's agents and checks, not
            // one per worktree.
            env: vec![(
                "CARGO_TARGET_DIR".into(),
                self.target().display().to_string(),
            )],
        };
        // The role's own model, effort and credential, as recorded when the
        // objective started (C-54); never another role's.
        let assigned = crate::models::for_role(&self.objective.models, role.name())?.clone();
        let mut agent = (self.setup.runtime)(&assigned, &development)?;
        agent.development = Some(development);
        // What is left of the spend budget, as a ceiling the SDK enforces
        // within the session (on Anthropic's API, where its prices are the
        // model's own; the totals below stop the objective in any case).
        agent.spend_ceiling = self
            .objective
            .budgets
            .usd
            .map(|usd| usd - self.objective.spent.usd);
        let controls = self.controls.clone();
        controls.step.store(false, Ordering::SeqCst);
        controls
            .steering
            .set_gate(if controls.paused.load(Ordering::SeqCst) {
                Gate::Pause
            } else {
                Gate::Run
            });
        agent.steering = controls.steering.clone();
        let folder = cwd.display().to_string();
        let mut conversation = Conversation::default();
        if resume && let Some(id) = self.cycle().sessions.get(role.name()).cloned() {
            conversation.entries.push(Entry::Operator {
                text: "(the earlier part of this session)".into(),
            });
            conversation.entries.push(Entry::Session {
                runtime: agq_assistant::claude_agent::RUNTIME.into(),
                id,
                event: "started".into(),
                folder: Some(folder.clone()),
            });
        }
        conversation.entries.push(Entry::Operator { text: brief });
        let toolset = kit.unwrap_or_else(|| Toolset {
            system: roles::instructions(role),
            definitions: roles::tools(role),
        });
        // This session's cost by model (`provider/model`): its own model and
        // those of its SDK subagents, each at its own price.
        let cost: RefCell<Vec<(agq_providers::ModelRef, Cost)>> = RefCell::new(Vec::new());
        let submitted: RefCell<Option<Value>> = RefCell::new(None);
        let delegated: RefCell<Option<children::Asked>> = RefCell::new(None);
        let refused: RefCell<Vec<(Value, String)>> = RefCell::new(Vec::new());
        let model = RefCell::new(Some(model));
        let events = self.events.clone();
        let role_name = role.name().to_string();
        let lead = self.agent("lead");
        let refusals = self.poster.clone();
        let mut executor = |call: &ToolCall| -> ToolResult {
            match call.name.as_str() {
                roles::SUBMIT_PROPOSAL
                | roles::SUBMIT_EXPLORATION
                | roles::SUBMIT_IMPLEMENTATION
                | roles::SUBMIT_REVIEW
                | roles::SUBMIT_EVALUATION => {
                    if call.name == roles::SUBMIT_REVIEW
                        && let Err(problem) = roles::read_review(&call.input)
                    {
                        return ToolResult::error(format!(
                            "Not accepted: {problem}. Submit your review again."
                        ));
                    }
                    if call.name == roles::SUBMIT_EXPLORATION
                        && let Some(planner) = planner
                    {
                        // Accepted, it is what the objective explores from
                        // now, also for a child delegated in this turn.
                        if let Err(problem) = planner.plan(&call.input) {
                            refusals.post(ThreadEntry::new(
                                Kind::Result,
                                lead.clone(),
                                format!(
                                    "Plans the exploration of {} (not accepted: {problem})",
                                    call.input["project"].as_str().unwrap_or("no project")
                                ),
                            ));
                            *refusal.borrow_mut() = Some(problem.clone());
                            return ToolResult::error(format!(
                                "Not accepted: {problem}. Fix it and submit again."
                            ));
                        }
                    }
                    if call.name == roles::SUBMIT_PROPOSAL {
                        let read = roles::read_proposal(
                            &call.input,
                            &roles::Given {
                                findings: &offered,
                                dispositions: &dispositions.borrow(),
                                model: base_model,
                            },
                        )
                        .and_then(|proposal| {
                            // A hypothesis's finding: the change serves the
                            // requirement it contradicts (E3).
                            let requirement = proposal
                                .finding
                                .as_ref()
                                .and_then(|id| cycle_findings.iter().find(|f| &f.identity == id))
                                .and_then(|f| f.requirement.as_deref());
                            roles::serves_the_finding(&proposal, requirement).map(|()| proposal)
                        });
                        match read {
                            Ok(proposal) => *accepted.borrow_mut() = Some(proposal),
                            Err(problem) => {
                                refusals.post(ThreadEntry::new(
                                    Kind::Result,
                                    lead.clone(),
                                    format!(
                                        "Proposes “{}” (not accepted: {problem})",
                                        call.input["title"].as_str().unwrap_or_default()
                                    ),
                                ));
                                *refusal.borrow_mut() = Some(problem.clone());
                                return ToolResult::error(format!(
                                    "Not accepted: {problem}. Fix it and submit again."
                                ));
                            }
                        }
                    }
                    *submitted.borrow_mut() = Some(call.input.clone());
                    ToolResult::answer("Received. End your turn now with one short sentence.")
                }
                roles::ADJUDICATE_FINDING if role == Role::Lead => {
                    let read = roles::read_disposition(
                        &call.input,
                        &roles::Given {
                            findings: &offered,
                            dispositions: &dispositions.borrow(),
                            model: base_model,
                        },
                        &objective_id,
                        cycle_n,
                    );
                    let (identity, mut disposition) = match read {
                        Ok(read) => read,
                        Err(problem) => {
                            return ToolResult::error(format!("Not recorded: {problem}."));
                        }
                    };
                    if disposition.kind != crate::findings::DispositionKind::Defect
                        && accepted.borrow().as_ref().and_then(|p| p.finding.as_ref())
                            == Some(&identity)
                    {
                        return ToolResult::error(
                            "Not recorded: your accepted proposal fixes this finding as a defect; it stays judged a defect for this cycle.",
                        );
                    }
                    let Some(finding) = cycle_findings.iter().find(|f| f.identity == identity)
                    else {
                        return ToolResult::error(
                            "Not recorded: it is not one of this cycle's findings.",
                        );
                    };
                    // Judged on the build this cycle reproduced it on.
                    disposition.build = Some(finding.build.clone());
                    // In the testing knowledge at once: kept across
                    // objectives, whatever this session does next.
                    if let Err(error) = Knowledge::change(&knowledge_file, &knowledge_key, |k| {
                        k.adjudicate(finding, disposition.clone())
                    }) {
                        return ToolResult::error(format!(
                            "Not recorded: the testing knowledge could not be written ({error})."
                        ));
                    }
                    let id = call.input["finding"].as_str().unwrap_or_default().trim();
                    refusals.post(explore::adjudication_entry(
                        lead.clone(),
                        id,
                        finding,
                        &disposition,
                    ));
                    dispositions
                        .borrow_mut()
                        .insert(identity.clone(), disposition.clone());
                    let kind = disposition.kind;
                    adjudicated.borrow_mut().push((identity, disposition));
                    ToolResult::answer(match kind {
                        crate::findings::DispositionKind::Defect => {
                            "Recorded: a defect, which you may choose to fix with submit_proposal's `finding`."
                        }
                        crate::findings::DispositionKind::AmbiguousRequirement => {
                            "Recorded: it goes to the Operator as a question, and is not proposed."
                        }
                        _ => "Recorded: it is set aside and not offered again.",
                    })
                }
                roles::DELEGATE if role == Role::Lead => {
                    // Checked now; recorded and started when the turn ends.
                    let checked = match planner {
                        _ if delegated.borrow().is_some() => {
                            Err("one child at a time: you delegated one in this turn".into())
                        }
                        Some(planner) => bounds.check(
                            &call.input,
                            cost.borrow().iter().map(|(_, c)| c.usd).sum(),
                            &planner.planning.borrow(),
                        ),
                        None => Err("this session has no projects to delegate".into()),
                    };
                    match checked {
                        Ok(asked) => {
                            if let Some(planner) = planner {
                                planner.planning.borrow_mut().gave(&asked.target.project);
                            }
                            *delegated.borrow_mut() = Some(asked);
                            ToolResult::answer(
                                "Delegated: the Orchestrator starts the child objective when your turn ends, and you will receive its result as your next message. End your turn now with one short sentence.",
                            )
                        }
                        Err(reason) => {
                            refusals.post(
                                ThreadEntry::new(
                                    Kind::Directive,
                                    lead.clone(),
                                    format!(
                                        "Asks to delegate: {} (refused: {reason})",
                                        call.input["instruction"].as_str().unwrap_or_default()
                                    ),
                                )
                                .with_details(call.input.to_string()),
                            );
                            refused
                                .borrow_mut()
                                .push((call.input.clone(), reason.clone()));
                            ToolResult::error(format!("Refused: {reason}."))
                        }
                    }
                }
                "observe_app" | "act_in_app" => {
                    let mut body = if call.name == "observe_app" {
                        json!({ "op": "observe", "detail": call.input["detail"].as_str().unwrap_or("summary") })
                    } else {
                        let mut body = call.input.clone();
                        body["op"] = json!("act");
                        body
                    };
                    body["agent"] = json!(role_name);
                    let answer = match test.as_deref_mut() {
                        Some(client) => client.call(body),
                        None if call.name == "observe_app" => {
                            let (reply, answer) = std::sync::mpsc::channel();
                            let _ = events.send(Event::Control { body, reply });
                            answer
                                .recv_timeout(Duration::from_secs(60))
                                .map_err(|_| "the Studio did not answer".to_string())
                        }
                        None => Err("acting in the application is the evaluator's".into()),
                    };
                    match answer {
                        Ok(value) if value["ok"] != false => {
                            ToolResult::answer(agq_assistant::tools::cap(
                                serde_json::to_string_pretty(&value).unwrap_or_default(),
                            ))
                        }
                        Ok(value) => ToolResult::error(value.to_string()),
                        Err(error) => ToolResult::error(error),
                    }
                }
                _ => match model.borrow_mut().as_mut() {
                    Some(model) => model.execute(call),
                    None => ToolResult::error(format!(
                        "`{}` is not available to the {role_name}",
                        call.name
                    )),
                },
            }
        };
        let mut execute = checked(&toolset.definitions, &mut executor);
        let spent_before = self.objective.spent.usd;
        let budget = self.objective.budgets.usd.unwrap_or(f64::INFINITY);
        let count = |model: &agq_providers::ModelRef, add: Cost| {
            let mut costs = cost.borrow_mut();
            match costs.iter_mut().find(|(m, _)| m == model) {
                Some((_, total)) => total.add(add),
                None => costs.push((model.clone(), add)),
            }
            costs.iter().map(|(_, c)| c.usd).sum::<f64>()
        };
        let session_id: RefCell<Option<String>> = RefCell::new(None);
        let stop = self.controls.stop.clone();
        // Its tool calls go into the thread as its activity, under the
        // directive it works on (C-54): each once its input is whole.
        let author = self.agent(role.name());
        let directive = self
            .objective
            .running_for(role.name())
            .map(|d| d.id.clone());
        // The role at work from now: the Operator's messages to the
        // implementer go to its session.
        self.controls.set_working(Some(role.name()));
        // The session's start is the step its activity folds under.
        let step = self.post(
            ThreadEntry::new(
                Kind::Event,
                author.clone(),
                format!(
                    "{} its session on {}",
                    if resume { "Resumes" } else { "Starts" },
                    assigned.label()
                ),
            )
            .for_directive(directive.as_deref()),
        );
        let under = (step.seq > 0).then_some(step.seq);
        let poster = self.poster.clone();
        let activity = |text: String, details: Option<String>| {
            let mut entry = ThreadEntry::new(Kind::Activity, author.clone(), text)
                .for_directive(directive.as_deref());
            entry.details = details;
            entry.under = under;
            poster.post(entry);
        };
        // Tool calls whose input is still coming: (id, tool, input so far).
        let calls: RefCell<Vec<(String, String, String)>> = RefCell::new(Vec::new());
        let mut on_event = |event: TurnEvent| match event {
            TurnEvent::Stream(StreamEvent::ModelUsage { model, usage }) => {
                let tokens = usage.input_tokens
                    + usage.output_tokens
                    + usage.cache_read_input_tokens
                    + usage.cache_creation_input_tokens;
                let add = match usage.cost_usd(&model) {
                    Some(usd) => Cost {
                        usd,
                        tokens,
                        unknown: false,
                    },
                    // No price known: counted at a high one, so the spend
                    // budget still stops it.
                    None => Cost {
                        usd: unpriced(&usage),
                        tokens,
                        unknown: true,
                    },
                };
                if spent_before + count(&model, add) >= budget {
                    stop.store(true, Ordering::SeqCst);
                }
            }
            TurnEvent::Stream(StreamEvent::Usage(usage)) => {
                // Usage without its model: the role's own, unpriced.
                let add = Cost {
                    usd: unpriced(&usage),
                    tokens: usage.input_tokens + usage.output_tokens,
                    unknown: true,
                };
                if spent_before + count(&assigned.model, add) >= budget {
                    stop.store(true, Ordering::SeqCst);
                }
            }
            TurnEvent::Stream(StreamEvent::ToolCallStarted { id, name }) => {
                calls.borrow_mut().push((id, name, String::new()));
            }
            TurnEvent::Stream(StreamEvent::ToolInput { id, json }) => {
                let mut calls = calls.borrow_mut();
                if let Some(at) = calls.iter().position(|(i, ..)| *i == id) {
                    calls[at].2.push_str(&json);
                    // Read once it can be whole (an object ends with `}`).
                    if calls[at].2.trim_end().ends_with('}')
                        && let Ok(input) = serde_json::from_str::<Value>(&calls[at].2)
                    {
                        let (_, name, _) = calls.remove(at);
                        let (text, details) = thread::activity(&name, &input);
                        activity(text, details);
                    }
                }
            }
            TurnEvent::Stream(StreamEvent::ToolCallId { stream_id, id }) => {
                // The provider's id for a call that started under the
                // stream's: its result comes under this one.
                if let Some(call) = calls.borrow_mut().iter_mut().find(|c| c.0 == stream_id) {
                    call.0 = id;
                }
            }
            TurnEvent::ToolFinished(result) => {
                // A call whose input never came whole: shown by its name.
                let mut calls = calls.borrow_mut();
                if let Some(at) = calls.iter().position(|(i, ..)| *i == result.tool_use_id) {
                    let (_, name, _) = calls.remove(at);
                    activity(thread::activity(&name, &Value::Null).0, None);
                }
            }
            TurnEvent::Activity(Activity::Task {
                description,
                agent,
                event,
                ..
            }) => activity(
                format!(
                    "subagent {} {:?}: {description}",
                    agent.unwrap_or_default(),
                    event
                )
                .to_lowercase(),
                None,
            ),
            TurnEvent::Entry(Entry::Session { id, .. }) => *session_id.borrow_mut() = Some(id),
            TurnEvent::Entry(Entry::Notice { text }) => activity(text, None),
            _ => {}
        };
        agent.run(
            &mut conversation,
            &toolset,
            self.objective.budgets.calls_of(role.name()) as usize,
            &mut execute,
            &mut on_event,
            &controls.stop,
        );
        controls.set_working(None);
        drop(execute);
        controls.step.store(false, Ordering::SeqCst);
        // Messages the implementer's session ended before taking wait for
        // the lead's next turn, and the thread says so.
        for text in controls.steering.clear_messages() {
            self.event("The implementer's session ended before it took your message: it waits for the lead's next turn");
            self.post(ThreadEntry::message(text, "lead"));
        }
        if let Some(model) = model.borrow_mut().as_mut() {
            model.close();
        }
        for (model, add) in cost.take() {
            self.objective.spent.add(role.name(), &model, add);
        }
        if let Some(id) = session_id.borrow().clone() {
            self.cycle_mut().sessions.insert(role.name().into(), id);
        }
        let said = conversation
            .entries
            .iter()
            .rev()
            .find_map(|e| match e {
                Entry::Assistant { parts, .. } => parts.iter().find_map(|p| match p {
                    agq_providers::AssistantPart::Text { text } if !text.trim().is_empty() => {
                        Some(text.clone())
                    }
                    _ => None,
                }),
                Entry::Notice { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        self.save();
        if controls.stopped() {
            return Err("stopped".into());
        }
        Ok(Session {
            submitted: submitted.into_inner(),
            proposal: accepted.into_inner(),
            refusal: refusal.into_inner(),
            adjudicated: adjudicated.into_inner(),
            delegated: delegated.into_inner(),
            refused: refused.into_inner(),
            said,
        })
    }

    /// Build settings an agent's command left below the work folder
    /// (Cargo reads every folder's `.cargo` above a checkout, the nearest
    /// first): removed before the checks, so only the Orchestrator's own
    /// apply.
    fn remove_build_settings(&self) -> Result<(), String> {
        let ours = self.setup.work.join(".cargo");
        let mut pending = vec![self.setup.work.clone()];
        let mut removed = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if !path.is_dir()
                    || path
                        .file_name()
                        .is_some_and(|n| n == "target" || n == ".git" || n == "node_modules")
                {
                    continue;
                }
                if path.file_name().is_some_and(|n| n == ".cargo") && path != ours {
                    std::fs::remove_dir_all(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    removed.push(path.display().to_string());
                } else if path.components().count() < self.setup.work.components().count() + 6 {
                    pending.push(path);
                }
            }
        }
        if !removed.is_empty() {
            self.event(format!(
                "removed build settings an agent left: {}",
                removed.join(", ")
            ));
        }
        Ok(())
    }

    /// Clears dialogs a criterion's setup left in a test instance's way,
    /// by rule (§4.16: the answer is known, so no model is asked): an
    /// approval waits, any other dialog is cancelled. Returns the dialogs
    /// cleared, for the criterion's record.
    fn clear_dialogs(&mut self, client: &mut Client, goal: &str) -> Result<Vec<String>, String> {
        let by_rule = |s: &crate::decide::Situation| Ok(crate::decide::cancel(s));
        let (made, cleared) = crate::decide::clear_dialogs(client, &by_rule, goal, 4, true);
        let mut dialogs = Vec::new();
        for (dialog, decision) in made {
            self.objective.spent.decisions += 1;
            self.event(format!(
                "a {dialog} dialog was in the way: {}",
                decision.choice
            ));
            dialogs.push(dialog);
        }
        match cleared {
            Ok(()) => Ok(dialogs),
            Err(problem) if dialogs.is_empty() => Err(problem),
            Err(problem) => Err(format!(
                "{problem} (after cancelling {})",
                dialogs.join(", ")
            )),
        }
    }

    /// The dialog a test instance shows as it starts, if any: a finding
    /// about the build, never something to clear silently.
    fn dialog_at_start(client: &mut Client) -> Option<String> {
        client
            .observe(false)
            .ok()
            .and_then(|o| o["dialog"].as_str().map(str::to_string))
    }

    /// An observation criterion in a test instance: dialogs a previous
    /// criterion left are cleared (and named in the detail), its setup acts,
    /// and its expectation is observed. A setup action that fails fails the
    /// criterion; when the way cannot be cleared (an approval left open, or
    /// the instance gone), the criterion is not run, which is no pass.
    /// Its outcome, and whether the criterion's own expectation decided it
    /// (a setup action that failed, a broken connection or a way that could
    /// not be cleared did not: on the base, that is no evidence).
    fn observe_criterion(
        &mut self,
        client: &mut Client,
        criterion: &Criterion,
        setup: &[Value],
        expect: &Value,
    ) -> (Outcome, bool) {
        let outcome =
            |verdict: &str, detail: String| Outcome::new(criterion.id.clone(), verdict, detail);
        let cleared = match self.clear_dialogs(client, &criterion.statement) {
            Ok(cleared) => cleared,
            Err(problem) => {
                return (
                    outcome(
                        "not run",
                        format!("the way to it could not be cleared: {problem}"),
                    ),
                    false,
                );
            }
        };
        let mut setup_failed = None;
        for action in setup {
            match client.act("orchestrator", &criterion.id, action.clone()) {
                Ok(answer) if answer["ok"] == false => {
                    setup_failed = Some(format!("setup action failed: {answer}"));
                }
                Ok(_) => {}
                Err(error) => setup_failed = Some(format!("setup action not carried out: {error}")),
            }
            if setup_failed.is_some() {
                break;
            }
        }
        let (verdict, mut detail, own) = match setup_failed {
            Some(problem) => ("failed", problem, false),
            None => match client.observe(true) {
                Err(error) => (
                    "failed",
                    format!("it could not be observed: {error}"),
                    false,
                ),
                Ok(observation) => match crate::control::holds(&observation, expect) {
                    Ok(()) => ("passed", "observed".to_string(), true),
                    Err(problem) => ("failed", problem, true),
                },
            },
        };
        if !cleared.is_empty() {
            detail = format!("{detail} (first cancelled: {})", cleared.join(", "));
        }
        (outcome(verdict, detail), own)
    }

    fn policy(&self, folder: &Path, write: bool, commands: bool) -> Policy {
        let policy = Policy::development(
            folder,
            &[],
            &self.setup.protected,
            Place::Worktree,
            Permissions {
                commands,
                network: false,
                push: false,
                mcp_servers: Vec::new(),
                undecided: Undecided::Refuse,
            },
        )
        .allowing(&self.objective.permissions.configuration);
        if write { policy } else { policy.read_only() }
    }

    /// A clean checkout of `commit` at `folder` of the cycle: the one there
    /// if it is exactly that commit with nothing changed, else a new one.
    fn checkout(&mut self, name: &str, commit: &str) -> Result<PathBuf, String> {
        let folder = self.folder(name);
        let unchanged = git::head(&folder).is_ok_and(|h| h.commit == commit)
            && git::changed_files(&folder).is_ok_and(|c| c.is_empty());
        if unchanged {
            return Ok(folder);
        }
        let repository = self.objective.repository.clone();
        let worktree = format!("{}-{}-{name}", self.id(), self.cycle().n);
        let _ = git::remove_worktree(&repository, &worktree);
        let _ = std::fs::remove_dir_all(&folder);
        git::checkout_worktree(&repository, &worktree, &folder, commit)
            .map_err(|e| e.to_string())?;
        Ok(folder)
    }

    fn propose(&mut self) -> Next {
        let repository = self.objective.repository.clone();
        let base = self.cycle_base()?;
        // Proposed and handed over before a restart: not asked again.
        let handed = format!("cycle-{}/proposal", self.cycle().n);
        if self.cycle().proposal.is_some()
            && self
                .objective
                .directives
                .iter()
                .any(|d| d.refers_to.as_deref() == Some(handed.as_str()))
        {
            return Ok(Phase::Implement);
        }
        let lead = self.checkout("lead", &base)?;
        // The base commit's model, read before the lead's session opens it
        // (C-55): the proposal's `serves` and `parts` are resolved in it. A
        // model that cannot be read stops the cycle; none resolves nothing.
        let model = match crate::traceability::base_model(&repository, &lead, &base)? {
            Some(model) => Ok(model),
            None => Err(crate::traceability::NO_MODEL.to_string()),
        };
        let reading = match &model {
            Err(_) => Some(
                "This project has no model: `serves` and `parts` are recorded as you state them, not resolved.",
            ),
            Ok(model) if !crate::traceability::has_requirements(model) => Some(
                "This project's model declares no requirement yet: `serves` is recorded as you state it (say what the change is for); `parts` are resolved in its model.",
            ),
            Ok(_) => None,
        };
        let policy = self.policy(&lead, false, false);
        let delegates =
            self.objective.explore && self.may_delegate() && self.cycle().repairs.is_none();
        let kit = Toolset {
            system: roles::instructions(Role::Lead),
            definitions: roles::lead_tools(false, delegates),
        };
        let mut context = match self.repair_brief() {
            Some(brief) => brief,
            None => self.findings_brief(),
        };
        // A child delegated while proposing explores what the objective
        // explores (the W13.7 repair): the planner, where it may delegate.
        let planner = if delegates {
            Some(self.planner(&lead, &base)?)
        } else {
            None
        };
        if let Some(reading) = reading {
            context = format!("{reading}\n\n{context}");
        }
        let mut brief = roles::brief(Role::Lead, &self.objective, &context);
        for attempt in 0..2 {
            let session = self.lead(
                &lead,
                policy.clone(),
                brief.clone(),
                kit.clone(),
                attempt > 0,
                children::Against {
                    model: Some(&model),
                    planner: planner.as_ref(),
                },
            )?;
            // Accepted when it was submitted, against the findings it was
            // offered and its own judgments of them.
            match session.proposal {
                Some(proposal) => {
                    // The proposal, frozen, handed to the implementer; the
                    // finding it fixes is frozen with it (its replay).
                    let title = proposal.title.clone();
                    let text = roles::proposal_text(&proposal);
                    let finding = proposal.finding.as_ref().and_then(|identity| {
                        self.cycle()
                            .findings
                            .iter()
                            .find(|f| &f.identity == identity)
                            .cloned()
                    });
                    let cycle = self.cycle_mut();
                    cycle.proposal = Some(proposal);
                    cycle.replay = finding.clone();
                    self.direct(
                        "lead",
                        Recipient::Role("implementer".into()),
                        record::Scope {
                            instruction: format!("Implement the frozen proposal “{title}”"),
                            focus: None,
                            budgets: None,
                            permissions: None,
                        },
                        Some(handed.clone()),
                        format!("Proposes: {title}"),
                        match &finding {
                            Some(f) => format!(
                                "{text}\nFixes: {} (its replay is the frozen criterion `{}`)",
                                explore::finding_line(f),
                                record::REPLAY
                            ),
                            None => text,
                        },
                    );
                    let _ = git::remove_worktree(
                        &repository,
                        &format!("{}-{}-lead", self.id(), self.cycle().n),
                    );
                    return Ok(Phase::Implement);
                }
                None if self.nothing_to_fix() => {
                    // Every reproduced finding was judged other than a
                    // defect (C-55): nothing to fix, an outcome, and the
                    // cycle's directives end saying so.
                    let why = "nothing to fix: the lead judged every reproduced finding other than a defect";
                    self.objective.settle_running(DirectiveStatus::Done, why);
                    self.event(format!("Nothing to fix in this cycle: {why}"));
                    let _ = git::remove_worktree(
                        &repository,
                        &format!("{}-{}-lead", self.id(), self.cycle().n),
                    );
                    return Ok(Phase::Done);
                }
                None => {
                    brief = match &session.refusal {
                        Some(problem) => format!(
                            "Your proposal was not accepted: {problem}. Submit a corrected one."
                        ),
                        None => "You ended without submit_proposal. Choose one improvement and submit it."
                            .into(),
                    };
                    // The repair, or the findings by the same ids (C-55),
                    // again.
                    let again = self.repair_brief().unwrap_or_else(|| self.findings_brief());
                    if !again.is_empty() {
                        brief = format!("{brief}\n\n{again}");
                    }
                }
            }
            if self.over().is_some() {
                break;
            }
        }
        Err("the lead did not hand over an acceptable proposal".into())
    }

    /// What a repair cycle repairs, for the lead: the failure that blocks
    /// another cycle's reviewed change, and its scope.
    fn repair_brief(&self) -> Option<String> {
        let blocked_n = self.cycle().repairs?;
        let blocked = self.objective.cycles.iter().find(|c| c.n == blocked_n)?;
        let b = blocked.blocked.as_ref()?;
        let tests: Vec<String> = b
            .failure
            .tests
            .iter()
            .map(|t| format!("- {}", t.line()))
            .collect();
        Some(format!(
            "This cycle repairs only what blocks cycle {blocked_n}'s reviewed change{}: the repository's checks ({}) fail where that change cannot have caused it — {}.\n\nThe failing tests:\n{}\n\nWhat the log shows:\n{}\n\nFind the cause in the code and repair it, so the checks pass for every change; propose nothing else. Never delete, ignore, loosen or rerun a test or check to make it pass (a deliberate change to one is named, with its reason, for the reviewer). The repository's checks may run on another system than this one (Linux in CI): reason from the code and the log when the failure does not happen here. Serve the requirement of the project's model that the required checks decide, where it has one.",
            blocked
                .pull_request
                .as_ref()
                .map(|pr| format!(" (pull request #{})", pr.number))
                .unwrap_or_default(),
            b.failure.check,
            b.why,
            tests.join("\n"),
            b.failure.excerpt,
        ))
    }

    /// A repair cycle's scope, for the implementer and the reviewer: what
    /// failed, that only its repair belongs in this change, and that no test
    /// or check may be weakened or rerun to pass.
    fn repair_scope(&self) -> String {
        let Some(n) = self.cycle().repairs else {
            return String::new();
        };
        let Some(b) = self
            .objective
            .cycles
            .iter()
            .find(|c| c.n == n)
            .and_then(|c| c.blocked.as_ref())
        else {
            return String::new();
        };
        let tests: Vec<String> = b.failure.tests.iter().map(|t| t.line()).collect();
        format!(
            "This change only repairs what blocks cycle {n}'s reviewed change: {} fail{} on the repository's checks ({}). Nothing else belongs in it, and no test or check may be deleted, ignored, loosened or rerun to make it pass. Its change is carried over only if it touches what failed.\n\n",
            tests.join(", "),
            if tests.len() == 1 { "s" } else { "" },
            b.why
        )
    }

    /// What the lead is told of an exploring cycle: the reproduced findings
    /// to choose among, and what the testing knowledge says.
    fn findings_brief(&self) -> String {
        if !self.objective.explore {
            return String::new();
        }
        let (offered, text) = self.offered_findings();
        // How this cycle's explorations answered their hypotheses (E3).
        let answers: Vec<String> = self
            .cycle()
            .explorations
            .iter()
            .flat_map(|e| &e.answers)
            .map(|a| {
                format!(
                    "- {}",
                    a.line(|identity| {
                        offered
                            .iter()
                            .find(|(_, offered)| offered == identity)
                            .map(|(id, _)| id.clone())
                    })
                )
            })
            .collect();
        format!(
            "{}Reproduced findings of this cycle{}:\n{}\n\n{}",
            if answers.is_empty() {
                String::new()
            } else {
                format!(
                    "The hypotheses this cycle's exploration tested (a contradiction reproduced is still only a finding: judge it against the requirement before anything is changed):\n{}\n\n",
                    answers.join("\n")
                )
            },
            if offered.is_empty() {
                ""
            } else {
                " (judge each you consider with adjudicate_finding; choose the defect you fix with `finding`)"
            },
            if text.is_empty() { "none" } else { &text },
            self.testing_summary()
        )
    }

    fn implement(&mut self, context: String) -> Next {
        let repository = self.objective.repository.clone();
        let n = self.cycle().n;
        let id = self.id();
        let name = format!("{id}-{n}");
        let work = self.folder("work");
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        if self.cycle().worktree.is_none() {
            let created = self
                .setup
                .store
                .once(&id, &format!("cycle-{n}/worktree"), || {
                    // Made again if an earlier start was cut short.
                    let _ = git::remove_worktree(&repository, &name);
                    let _ = std::fs::remove_dir_all(&work);
                    let branch = git::create_worktree(&repository, &name, &work)
                        .map(|w| w.branch)
                        .map_err(|e| e.to_string())?;
                    // At the cycle's base, whatever the repository's HEAD is
                    // now.
                    forge::start_at(&work, &base)?;
                    Ok(branch)
                })?;
            let cycle = self.cycle_mut();
            cycle.branch = Some(created);
            cycle.worktree = Some(work.clone());
            self.save();
        }
        let policy = self.policy(&work, true, true);
        let repairing = !context.is_empty();
        let brief = roles::brief(
            Role::Implementer,
            &self.objective,
            &format!("{}{context}", self.repair_scope()),
        );
        let directive = self
            .objective
            .running_for("implementer")
            .map(|d| d.id.clone());
        let session = self.session(
            Role::Implementer,
            &work,
            policy,
            brief,
            repairing,
            With::default(),
        )?;
        let submitted = session
            .submitted
            .as_ref()
            .and_then(|v| v["summary"].as_str())
            .map(str::to_string);
        let summary = submitted
            .clone()
            .unwrap_or_else(|| format!("(no summary; it said: {})", session.said));
        self.post(
            ThreadEntry::new(
                Kind::Result,
                self.agent("implementer"),
                match &submitted {
                    Some(_) => "Submits the implementation",
                    None => "Ends without submitting the implementation",
                },
            )
            .with_details(summary.clone())
            .for_directive(directive.as_deref()),
        );
        // A repair it was directed to make is done when it hands it over,
        // failed when it ends without; the proposal's directive goes on
        // until the review approves or the cycle ends.
        if let Some(id) = &directive
            && self.objective.directive(id).is_some_and(|d| {
                d.refers_to
                    .as_deref()
                    .is_some_and(|r| r.contains("/review-"))
            })
        {
            let status = if submitted.is_some() {
                DirectiveStatus::Done
            } else {
                DirectiveStatus::Failed
            };
            self.objective.settle(id, status, Some(summary.clone()));
        }
        let a = self.cycle().attempts.len() as u32 + 1;
        let message = format!(
            "{} (objective {id}, cycle {n}, attempt {a})",
            self.cycle()
                .proposal
                .as_ref()
                .map(|p| p.title.as_str())
                .unwrap_or("Agentique objective")
        );
        let work_for_commit = work.clone();
        let commit =
            self.setup
                .store
                .once(&id, &format!("cycle-{n}/attempt-{a}/commit"), || {
                    match git::commit_worktree(&work_for_commit, &message) {
                        Ok(commit) => Ok(commit),
                        Err(error) => {
                            // The implementer may have committed itself.
                            let head = git::head(&work_for_commit).map_err(|e| e.to_string())?;
                            if head.commit != base {
                                Ok(head.commit)
                            } else {
                                Err(format!("the implementer changed nothing ({error})"))
                            }
                        }
                    }
                })?;
        let previous = self.cycle().attempt().and_then(|a| a.commit.clone());
        if commit == base || Some(&commit) == previous.as_ref() {
            return Err("the implementer changed nothing".into());
        }
        self.event(format!(
            "Committed the implementer's work as {} (attempt {a})",
            builds::short(&commit)
        ));
        self.cycle_mut().attempts.push(Attempt {
            n: a,
            commit: Some(commit),
            summary,
            ..Attempt::default()
        });
        Ok(Phase::Check)
    }

    /// A criterion's test run in `folder`: it must pass and run at least
    /// one test; a program that is no test run is refused.
    fn criterion(&self, folder: &Path, words: &[String], timeout: Duration) -> Outcome {
        if let Err(problem) = roles::test_command(words) {
            return Outcome {
                name: words.join(" "),
                verdict: "not run".into(),
                detail: problem,
                judged: false,
            };
        }
        self.run_command(folder, words, timeout, true)
    }

    /// Runs `words` in `folder` as an exact allowed command.
    fn command(&self, folder: &Path, words: &[String], timeout: Duration) -> Outcome {
        self.run_command(folder, words, timeout, false)
    }

    fn run_command(
        &self,
        folder: &Path,
        words: &[String],
        timeout: Duration,
        tests: bool,
    ) -> Outcome {
        let name = words.join(" ");
        let Some(program) = Program::from_list(words) else {
            return Outcome {
                name,
                verdict: "not run".into(),
                detail: "an empty command".into(),
                judged: false,
            };
        };
        let executor = match Scope::read_only(folder) {
            Ok(scope) => Executor::new(scope)
                .trusted(true)
                .target_dir(self.target())
                .allow(vec![program.clone()])
                .cancel_flag(self.controls.stop.clone()),
            Err(error) => {
                return Outcome {
                    name,
                    verdict: "not run".into(),
                    detail: error.to_string(),
                    judged: false,
                };
            }
        };
        match executor.run(&program, "", timeout) {
            Ok(finished)
                if finished.success
                    && tests
                    && tests_ran(&format!("{}\n{}", finished.stdout, finished.stderr))
                        == Some(0) =>
            {
                Outcome {
                    name,
                    verdict: "failed".into(),
                    detail: "it ran no test".into(),
                    judged: false,
                }
            }
            Ok(finished)
                if finished.success
                    && format!("{}{}", finished.stdout, finished.stderr)
                        .contains("test result: FAILED") =>
            {
                Outcome {
                    name,
                    verdict: "failed".into(),
                    detail: "the run exited well but says tests failed".into(),
                    judged: false,
                }
            }
            Ok(finished) if finished.success => Outcome {
                name,
                verdict: "passed".into(),
                detail: String::new(),
                judged: false,
            },
            Ok(finished) => Outcome {
                name,
                verdict: "failed".into(),
                detail: agq_execution::process::last_lines(
                    &format!("{}\n{}", finished.stdout, finished.stderr),
                    40,
                ),
                judged: false,
            },
            Err(error) => Outcome {
                name,
                verdict: "not run".into(),
                detail: error.to_string(),
                judged: false,
            },
        }
    }

    fn check(&mut self) -> Next {
        let commit = self
            .cycle()
            .attempt()
            .and_then(|a| a.commit.clone())
            .ok_or("nothing to check")?;
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let repository = self.objective.repository.clone();
        if !forge::is_ancestor(&repository, &base, &commit)? {
            return Err(format!(
                "the commit {} does not descend from the cycle's base {}",
                builds::short(&commit),
                builds::short(&base)
            ));
        }
        // The build settings, as written: an agent's command may have
        // changed them.
        cargo_settings(&self.setup.work)?;
        self.remove_build_settings()?;
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        // Each criterion on the base, before the change, where none may
        // pass and one must fail with evidence (C-54): once per cycle, and
        // the test runs again whenever this commit's test files differ from
        // those the evidence was made with.
        let tests = self.test_files(&base, &commit)?;
        let made = self.cycle().evidence.clone();
        if !self.cycle().before.is_empty() && made.as_ref().is_none_or(|e| e.tests != tests) {
            let again = self.commands_on_base(&commit, &tests)?;
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            self.post(
                ThreadEntry::event(format!(
                    "This attempt's test files differ: its test runs on the base were made again with those of {}",
                    builds::short(&commit)
                ))
                .with_details(
                    again
                        .iter()
                        .map(|o| format!("- {}: {} ({})", o.name, o.verdict, o.detail))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
            );
            let cycle = self.cycle_mut();
            for outcome in again {
                match cycle.before.iter_mut().find(|o| o.name == outcome.name) {
                    Some(earlier) => *earlier = outcome,
                    None => cycle.before.push(outcome),
                }
            }
            cycle.evidence = Some(record::Evidence {
                commit: commit.clone(),
                tests: tests.clone(),
            });
            self.save();
        }
        if self.cycle().before.is_empty() {
            let outcomes = self.on_base(&commit, &tests)?;
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            self.cycle_mut().evidence = Some(record::Evidence {
                commit: commit.clone(),
                tests: tests.clone(),
            });
            self.post(
                ThreadEntry::event("The criteria on the base, before the change").with_details(
                    outcomes
                        .iter()
                        .map(|o| format!("- {}: {} ({})", o.name, o.verdict, o.detail))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
            );
            self.cycle_mut().before = outcomes;
            self.save();
        }
        // The change's checkout after the base's build, its files newer than
        // what the shared build folder holds, so nothing of the base's build
        // stands in for the change's.
        let verify = self.checkout("verify", &commit)?;
        freshen(&verify);
        let mut checks = Vec::new();
        for words in self.setup.checks.clone() {
            self.hold();
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            self.event(format!("check: {}", words.join(" ")));
            checks.push(self.command(&verify, &words, Duration::from_secs(3600)));
        }
        let mut criteria = Vec::new();
        for criterion in &proposal.criteria {
            if let Check::Command { program } = &criterion.check {
                let mut outcome = self.criterion(&verify, program, Duration::from_secs(1800));
                outcome.name = format!("{}: {}", criterion.id, outcome.name);
                criteria.push(outcome);
            }
        }
        // Stopped meanwhile: nothing is recorded as a failure of the change.
        if self.controls.stopped() {
            return Err("stopped".into());
        }
        let patch = git::patch_of(&repository, &base, &commit).map_err(|e| e.to_string())?;
        let mut gates = vec![
            gates::paths(
                &patch,
                &self.setup.protected,
                &self.objective.permissions.configuration,
            ),
            gates::keys(&patch, &self.setup.keys),
        ];
        // The model is read by the commits' trees, through a clean checkout
        // made now in which nothing ran: never `verify`, where the checks
        // just ran the change's code (C-55). A repository without a model
        // has nothing locked, unless the change removed the model.
        let commit_model = crate::traceability::has_model(&repository, &commit)?;
        let base_model = crate::traceability::has_model(&repository, &base)?;
        let reader = self.reader(&[&commit, &base])?;
        let locked = self.objective.permissions.locked.clone();
        let unlocked = match (&reader, commit_model, base_model) {
            (Some(folder), true, _) => {
                agq_assistant::model_tools::locked_changes(folder, &base, &locked)
            }
            (_, false, true) => Ok(vec!["the model (the change removes it)".to_string()]),
            _ => Ok(Vec::new()),
        };
        gates.push(match unlocked {
            Ok(found) if found.is_empty() => {
                Outcome::new("locked elements unchanged", "passed", "")
            }
            Ok(found) => Outcome::new(
                "locked elements unchanged",
                "failed",
                format!(
                    "the change touches locked elements the objective does not name: {}",
                    found.join(", ")
                ),
            ),
            Err(error) => Outcome::new("locked elements unchanged", "not run", error),
        });
        // The code of locked parts, by the links (the change's and the
        // base's, by their trees), unless the objective names the part.
        let files: Vec<String> = patch
            .files
            .iter()
            .map(|f| f.path.replace('\\', "/"))
            .collect();
        let links = crate::traceability::links_of(&repository, &base, &commit);
        let name = "code of locked parts unchanged";
        gates.push(match &reader {
            Some(folder) => match agq_assistant::model_tools::locked_code(
                folder, &base, &files, &links, &locked,
            ) {
                Ok(found) if found.is_empty() => Outcome::new(name, "passed", ""),
                Ok(found) => Outcome::new(
                    name,
                    "failed",
                    format!(
                        "the change touches the code of locked parts the objective does not name: {}",
                        found.join(", ")
                    ),
                ),
                Err(error) => Outcome::new(name, "not run", error),
            },
            None => Outcome::new(name, "passed", "no model, so nothing is locked"),
        });
        // Purpose and governance stay the Operator's (C-55), whatever the
        // objective names, as the base and the commits the objective
        // protects (its start, the approved baseline) declare them.
        let earlier = self.protected_commits();
        let purpose = crate::traceability::purpose_in(reader.as_deref(), &base, &commit, &earlier);
        gates.push(gates::purpose(&patch, &purpose));

        // The criteria show the defect on the base (C-54), by their own
        // outcomes there, made with this commit's test files.
        let mut ids: Vec<String> = proposal.criteria.iter().map(|c| c.id.clone()).collect();
        if self.cycle().replay.is_some() {
            ids.push(record::REPLAY.to_string());
        }
        let mut shown = gates::defect_shown(&self.cycle().before, &ids);
        match &self.cycle().evidence {
            Some(made) if made.tests == tests => {
                let note = format!(
                    "evidence made on the base with the test files of {}{}",
                    builds::short(&made.commit),
                    if made.commit == commit {
                        String::new()
                    } else {
                        format!(", the same as {}'s", builds::short(&commit))
                    }
                );
                shown.detail = if shown.detail.is_empty() {
                    note
                } else {
                    format!("{}\n{note}", shown.detail)
                };
            }
            _ => {
                shown.verdict = "failed".into();
                shown.detail = format!(
                    "the evidence on the base was not made with {}'s test files",
                    builds::short(&commit)
                );
            }
        }
        gates.push(shown);
        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
        attempt.checks = checks;
        attempt.criteria = criteria;
        attempt.gates = gates;
        let failed = attempt.failures();
        let outcomes = outcome_lines(attempt);
        if failed.is_empty() {
            self.post(
                ThreadEntry::event("Every required check, command criterion and gate passed")
                    .with_details(outcomes),
            );
            Ok(Phase::Evaluate)
        } else {
            self.post(
                ThreadEntry::event(format!("{} failed: {}", failed.len(), failed.join("; ")))
                    .with_details(outcomes),
            );
            Ok(Phase::Repair)
        }
    }

    fn evaluate(&mut self) -> Next {
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let commit = self
            .cycle()
            .attempt()
            .and_then(|a| a.commit.clone())
            .ok_or("nothing to evaluate")?;
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let verify = self.checkout("verify", &commit)?;
        // User-facing code: the part Studio's, by its links on the base and
        // in the change (C-54), so dropping a link does not hide it.
        let patch =
            git::patch_of(&self.objective.repository, &base, &commit).map_err(|e| e.to_string())?;
        let files: Vec<String> = patch
            .files
            .iter()
            .map(|f| f.path.replace('\\', "/"))
            .collect();
        let links = crate::traceability::links_of(&self.objective.repository, &base, &commit);
        let user_facing = gates::user_facing(&files, &links);
        let observed: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Observation { .. }))
            .collect();
        let judged: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Judgment))
            .collect();
        let replay = self.cycle().replay.is_some();
        if observed.is_empty() && judged.is_empty() && !replay && user_facing.is_empty() {
            return Ok(Phase::Review);
        }
        if !user_facing.is_empty() && observed.is_empty() && judged.is_empty() && !replay {
            // Its criteria are frozen: no later attempt can add one.
            let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
            attempt.gates.push(Outcome::new(
                "a user-facing change has a behavioural criterion",
                "failed",
                format!("it changes {}", user_facing.join(", ")),
            ));
            return Err(format!(
                "the change touches user-facing code ({}), but its frozen criteria have no behavioural one, which a later attempt cannot add",
                user_facing.join(", ")
            ));
        }
        self.event("Building the change for its test instances");
        let exe = self.setup.studios.build(
            &verify,
            &self.target(),
            &self.setup.builds,
            self.controls.stop.clone(),
        )?;
        // An unreviewed build: started only as a test instance with no
        // credential and its Assistant on the scripted stand-in.
        let unreviewed = Options {
            speed: Some("instant".into()),
            stand_in: true,
            unreviewed: true,
            ..Options::default()
        };
        let mut outcomes: Vec<Outcome> = self
            .observe_criteria(&exe, &verify, &observed, "change", &unreviewed)
            .into_iter()
            .map(|seen| seen.outcome)
            .collect();
        // The replay and the changed areas start from the base's start
        // projects, which the change cannot alter.
        let start = self.checkout("base", &base)?;
        outcomes.extend(self.evaluate_by_behaviour(&exe, &start, &user_facing)?);
        if !judged.is_empty() {
            let options = Options {
                speed: Some(self.setup.speed.clone()),
                ..unreviewed.clone()
            };
            outcomes.extend(self.judge(&exe, &verify, &commit, &judged, &options)?);
        }
        if !user_facing.is_empty() {
            outcomes.push(evaluated(&outcomes, &proposal, &user_facing));
        }
        if self.controls.stopped() {
            return Err("stopped".into());
        }
        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
        attempt.criteria.extend(outcomes);
        let failed = attempt.failures();
        let lines = outcome_lines(attempt);
        if failed.is_empty() {
            self.post(
                ThreadEntry::event("The test instance showed every behavioural criterion")
                    .with_details(lines),
            );
            Ok(Phase::Review)
        } else {
            self.post(
                ThreadEntry::event(format!(
                    "In the test instance, {} failed: {}",
                    failed.len(),
                    failed.join("; ")
                ))
                .with_details(lines),
            );
            Ok(Phase::Repair)
        }
    }

    /// The judgment criteria, by the evaluator in a test instance of the
    /// change; when the instance does not start, each is not run (no pass).
    fn judge(
        &mut self,
        exe: &Path,
        verify: &Path,
        commit: &str,
        judged: &[&Criterion],
        options: &Options,
    ) -> Result<Vec<Outcome>, String> {
        let not_run = |why: &str| -> Vec<Outcome> {
            judged
                .iter()
                .map(|c| Outcome {
                    judged: true,
                    ..Outcome::new(c.id.clone(), "not run", why)
                })
                .collect()
        };
        let started =
            TestInstance::start_with(exe, &self.folder("instance"), verify, verify, options)
                .and_then(|mut instance| {
                    let client = instance.connect(Duration::from_secs(180))?;
                    Ok((instance, client))
                });
        let (instance, mut client) = match started {
            Ok(started) => started,
            Err(problem) => {
                return Ok(not_run(&format!(
                    "its test instance did not start: {problem}"
                )));
            }
        };
        let context = format!(
            "Criteria to judge in the test instance: {}\nThe test instance shows the cycle's commit {} with the project open.",
            judged
                .iter()
                .map(|c| format!("{} ({})", c.id, c.statement))
                .collect::<Vec<_>>()
                .join("; "),
            builds::short(commit)
        );
        let brief = roles::brief(Role::Evaluator, &self.objective, &context);
        let policy = self.policy(verify, false, false);
        let session = self.session(
            Role::Evaluator,
            verify,
            policy,
            brief,
            false,
            With {
                test: Some(&mut client),
                ..With::default()
            },
        )?;
        let reported = session.submitted.unwrap_or_default();
        self.post(
            ThreadEntry::new(
                Kind::Result,
                self.agent("evaluator"),
                "Reports the criteria it judged",
            )
            .with_details(
                reported["criteria"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|c| {
                        format!(
                            "- {}: {} ({})",
                            c["id"].as_str().unwrap_or_default(),
                            c["outcome"].as_str().unwrap_or("not run"),
                            c["observations"].as_str().unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );
        let mut outcomes = Vec::new();
        for criterion in judged {
            let found = reported["criteria"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|c| c["id"] == criterion.id.as_str());
            outcomes.push(match found {
                Some(c) => Outcome {
                    judged: true,
                    ..Outcome::new(
                        criterion.id.clone(),
                        c["outcome"].as_str().unwrap_or("not run"),
                        c["observations"].as_str().unwrap_or_default(),
                    )
                },
                None => Outcome {
                    judged: true,
                    ..Outcome::new(
                        criterion.id.clone(),
                        "not run",
                        "the evaluator did not report it",
                    )
                },
            });
        }
        drop(client);
        drop(instance);
        Ok(outcomes)
    }

    fn review(&mut self) -> Next {
        let commit = self
            .cycle()
            .attempt()
            .and_then(|a| a.commit.clone())
            .ok_or("nothing to review")?;
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let verify = self.checkout("verify", &commit)?;
        let repository = self.objective.repository.clone();
        let patch = git::patch_of(&repository, &base, &commit).map_err(|e| e.to_string())?;
        let mut diff = patch.text();
        if diff.chars().count() > 60_000 {
            diff = diff.chars().take(60_000).collect::<String>()
                + "\n… (the diff goes on; read the files)";
        }
        let attempt = self.cycle().attempt().cloned().unwrap_or_default();
        let outcomes = outcome_lines(&attempt);
        let changes = gates::listed_test_changes(&patch);
        // What the commit changed against what the proposal named, and the
        // cumulative change since the approved baseline (C-55).
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let mut traced = self.trace_for_review(&base, &commit, &patch, &proposal)?;
        // How the lead judged the finding it fixes, for the reviewer to
        // check against the requirement (C-55).
        if let Some(finding) = &self.cycle().replay {
            traced = format!(
                "The finding it fixes: {}; {}.\n\n{traced}",
                explore::finding_line(finding),
                finding
                    .disposition
                    .as_ref()
                    .map(explore::disposition_line)
                    .unwrap_or_else(|| "not adjudicated".into())
            );
        }
        let context = format!(
            "The implementer's summary: {}\n\nOutcomes:\n{outcomes}\n\nChanges to tests, checks or budgets the baseline guard lists: {}\n\n{traced}\n\nThe diff against {}:\n{diff}",
            attempt.summary,
            if changes.is_empty() {
                "none".to_string()
            } else {
                changes.join("; ")
            },
            builds::short(&base),
        );
        let brief = roles::brief(
            Role::Reviewer,
            &self.objective,
            &format!("{}{context}", self.repair_scope()),
        );
        let policy = self.policy(&verify, false, false);
        let session = self.session(
            Role::Reviewer,
            &verify,
            policy,
            brief,
            false,
            With::default(),
        )?;
        let verdict = session.submitted.ok_or("the reviewer gave no verdict")?;
        let review = Review {
            verdict: verdict["verdict"]
                .as_str()
                .unwrap_or("request_changes")
                .to_string(),
            findings: verdict["findings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f.as_str().map(str::to_string))
                .collect(),
            test_changes_accepted: verdict["test_changes_accepted"] == true,
            commit: commit.clone(),
            traceability: verdict["traceability"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            purpose: verdict["purpose"].as_str().unwrap_or_default().to_string(),
        };
        let findings = review
            .findings
            .iter()
            .map(|f| format!("- {f}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.post(
            ThreadEntry::new(
                Kind::Result,
                self.agent("reviewer"),
                if review.verdict == "approve" {
                    format!("Approves {}", builds::short(&commit))
                } else {
                    format!("Asks for changes to {}", builds::short(&commit))
                },
            )
            .with_details(format!(
                "{findings}\nTraceability: {}\nPurpose: {}",
                review.traceability, review.purpose
            )),
        );
        if review.verdict != "approve" {
            // Its findings, handed to the implementer for repair (a
            // repeated review of the same attempt replaces the earlier).
            let refers = format!("cycle-{}/review-{}", self.cycle().n, attempt.n);
            let earlier: Vec<String> = self
                .objective
                .directives
                .iter()
                .filter(|d| {
                    d.refers_to.as_deref() == Some(refers.as_str())
                        && d.status == DirectiveStatus::Running
                })
                .map(|d| d.id.clone())
                .collect();
            for id in earlier {
                self.objective.settle(
                    &id,
                    DirectiveStatus::Stopped,
                    Some("a later review of the same attempt replaced it".into()),
                );
            }
            self.direct(
                "reviewer",
                Recipient::Role("implementer".into()),
                record::Scope {
                    instruction: format!(
                        "Repair attempt {} as the review asks:\n{findings}",
                        attempt.n
                    ),
                    focus: None,
                    budgets: None,
                    permissions: None,
                },
                Some(refers),
                "Asks the implementer to repair what the review found".into(),
                findings,
            );
        }
        let baseline = gates::baseline(&patch, &proposal, Some(&review));
        let approved = review.verdict == "approve";
        let asked = review.findings.join("\n");
        self.cycle_mut().review = Some(review);
        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
        attempt
            .gates
            .retain(|g| g.name != baseline.name && g.name != record::REVIEW);
        if !approved {
            // Identified as `review`, whatever its wording (C-54).
            attempt
                .gates
                .push(Outcome::new(record::REVIEW, "failed", asked));
        }
        let kept = baseline.passed();
        attempt.gates.push(baseline);
        if approved && kept {
            // The proposal's directive is done: its implementation passed
            // every check and the review.
            let handed = format!("cycle-{}/proposal", self.cycle().n);
            let done: Vec<String> = self
                .objective
                .directives
                .iter()
                .filter(|d| {
                    d.refers_to.as_deref() == Some(handed.as_str())
                        && d.status == DirectiveStatus::Running
                })
                .map(|d| d.id.clone())
                .collect();
            for id in done {
                self.objective.settle(
                    &id,
                    DirectiveStatus::Done,
                    Some(format!("approved as {}", builds::short(&commit))),
                );
            }
            Ok(Phase::Merge)
        } else {
            Ok(Phase::Repair)
        }
    }

    fn repair(&mut self) -> Next {
        let cycle = self.cycle();
        let attempts = cycle.attempts.len() as u32;
        if attempts >= self.objective.budgets.attempts {
            return Err(format!(
                "the attempt budget is used up ({attempts} attempts)"
            ));
        }
        let last = cycle
            .attempts
            .last()
            .map(Attempt::failures)
            .unwrap_or_default();
        if cycle.attempts.len() >= 2 {
            let before = cycle.attempts[cycle.attempts.len() - 2].failures();
            if !last.is_empty() && last == before {
                return Err(format!(
                    "the same failures twice in a row: {}",
                    last.join("; ")
                ));
            }
        }
        if cycle.attempts.len() >= 3 {
            let counts: Vec<usize> = cycle
                .attempts
                .iter()
                .rev()
                .take(3)
                .map(|a| a.failures().len())
                .collect();
            if counts[0] >= counts[1] && counts[1] >= counts[2] && counts[0] > 0 {
                return Err("no fewer failures in two rounds".into());
            }
        }
        let findings = match &cycle.review {
            Some(review)
                if review.verdict != "approve"
                    && Some(&review.commit) == cycle.attempt().and_then(|a| a.commit.as_ref()) =>
            {
                review.findings.clone()
            }
            _ => Vec::new(),
        };
        let context = roles::repair_context(cycle.attempt(), &findings);
        self.implement(context)
    }

    fn merge(&mut self) -> Next {
        let permissions = self.objective.permissions.clone();
        let branch = self
            .cycle()
            .branch
            .clone()
            .ok_or("the cycle has no branch")?;
        if !(permissions.push && permissions.merge) {
            return Err(format!(
                "the objective does not allow pushing and merging: the reviewed change waits on {branch}"
            ));
        }
        let commit = self
            .cycle()
            .attempt()
            .and_then(|a| a.commit.clone())
            .ok_or("nothing to merge")?;
        let reviewed = self.cycle().review.as_ref().map(|r| r.commit.clone());
        if reviewed.as_deref() != Some(commit.as_str()) {
            return Err("the commit to merge is not the reviewed one".into());
        }
        let repository = self.objective.repository.clone();
        let base_branch = self.objective.base_branch.clone();
        let id = self.id();
        let n = self.cycle().n;
        let short = builds::short(&commit).to_string();
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let body = self.pull_request_body();
        // What leaves this computer: one commit with the reviewed tree, on
        // the last one pushed (or the base), and the pull request's text;
        // no configured key in any of it.
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let parent = self.cycle().pushed.clone().unwrap_or_else(|| base.clone());
        let message = format!(
            "{}\n\nObjective {id}, cycle {n}: the reviewed commit {short}.",
            proposal.title
        );
        for (what, text) in [
            ("the commit message", &message),
            ("the pull request's title", &proposal.title),
            ("the pull request's body", &body),
        ] {
            let gate = gates::keys_in(what, text, &self.setup.keys);
            if !gate.passed() {
                return Err(gate.detail);
            }
        }
        let tree = git::tree_of(&repository, &commit).map_err(|e| e.to_string())?;
        let pushed = self
            .setup
            .store
            .once(&id, &format!("cycle-{n}/squash-{short}"), || {
                forge::commit_tree(&repository, &tree, &parent, &message)
            })?;
        if git::tree_of(&repository, &pushed).map_err(|e| e.to_string())? != tree {
            return Err("the commit to push does not hold the reviewed tree".into());
        }
        self.setup.store.once(
            &id,
            &format!("cycle-{n}/push-{}", builds::short(&pushed)),
            || {
                forge::push_commit(&repository, &pushed, &branch, &base_branch)
                    .map(|_| String::new())
            },
        )?;
        self.cycle_mut().pushed = Some(pushed.clone());
        self.save();
        let pr = match self.cycle().pull_request.clone() {
            Some(pr) => pr,
            None => {
                let (number, url) = forge::pull_request(
                    &repository,
                    &branch,
                    &base_branch,
                    &proposal.title,
                    &body,
                )?;
                let pr = crate::record::PullRequest { number, url };
                self.cycle_mut().pull_request = Some(pr.clone());
                self.save();
                pr
            }
        };
        self.event(format!(
            "pull request {} opened; waiting for the repository's checks",
            pr.url
        ));
        // The host has the pushed commit as the pull request's head before
        // its checks are read (a re-push's checks, not the last ones).
        let since = Instant::now();
        while forge::head(&repository, pr.number).ok().as_deref() != Some(pushed.as_str()) {
            if since.elapsed() > Duration::from_secs(180) || self.controls.stopped() {
                return Err("the pull request does not show the pushed commit".into());
            }
            std::thread::sleep(Duration::from_secs(5));
        }
        let stop = self.controls.stop.clone();
        match forge::wait_for_checks(&repository, pr.number, forge::CHECKS_WITHIN, &|| {
            stop.load(Ordering::SeqCst)
        })? {
            forge::Checks::Passed => {}
            forge::Checks::Failed(what) => {
                // Whose failure it is: the change's goes back to the
                // implementer; one the change cannot have caused blocks it,
                // its reviewed work kept on the pull request.
                let (failure, attribution) = self.attribute(&what, &base, &commit);
                let detail = format!("{what}\n{}", failure.excerpt);
                let (cause, why, said) = match attribution {
                    blockers::Attribution::Change(why) => {
                        self.post(
                            ThreadEntry::event(format!(
                                "The repository's checks failed on pull request #{}; back to the implementer ({why})",
                                pr.number
                            ))
                            .with_details(detail.clone()),
                        );
                        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
                        attempt.gates.push(Outcome {
                            name: "the repository's checks".into(),
                            verdict: "failed".into(),
                            detail,
                            judged: false,
                        });
                        return Ok(Phase::Repair);
                    }
                    blockers::Attribution::Elsewhere(why) => (
                        record::Cause::Elsewhere,
                        why.clone(),
                        format!(
                            "The repository's checks fail on pull request #{}, but not because of this change: {why}. A defect already on the base: the reviewed change stays on #{} until that is repaired",
                            pr.number, pr.number
                        ),
                    ),
                    blockers::Attribution::Infrastructure(why) => (
                        record::Cause::Infrastructure,
                        why.clone(),
                        format!(
                            "The repository's checks did not reach a verdict on pull request #{}: {why}. The checks' own machinery, not this change: the reviewed change stays on #{}, and nothing reruns them here",
                            pr.number, pr.number
                        ),
                    ),
                };
                self.post(ThreadEntry::event(said).with_details(detail.clone()));
                let cycle = self.cycle_mut();
                let attempt = cycle.attempts.last_mut().expect("an attempt");
                attempt.gates.push(Outcome {
                    name: "the repository's checks".into(),
                    verdict: match cause {
                        record::Cause::Elsewhere => "failed elsewhere",
                        record::Cause::Infrastructure => "no verdict",
                    }
                    .into(),
                    detail,
                    judged: false,
                });
                cycle.blocked = Some(record::Blocked {
                    failure,
                    why: why.clone(),
                    cause,
                    repaired_in: None,
                    carried_as: None,
                    note: String::new(),
                });
                self.save();
                return Err(format!(
                    "blocked ({why}); the reviewed change waits on pull request #{}",
                    pr.number
                ));
            }
            forge::Checks::Pending => {
                return Err("the repository's checks did not finish in time".into());
            }
        }
        let subject = format!("{} (#{})", proposal.title, pr.number);
        // The branch is merged next: its worktree goes first, so the host's
        // merge can delete the branch here and on the host.
        self.remove_work(false);
        let merged = self
            .setup
            .store
            .once(&id, &format!("cycle-{n}/merge-{short}"), || {
                forge::merge(&repository, pr.number, &pushed, &subject)
            })?;
        self.cycle_mut().merged = Some(merged.clone());
        self.save();
        // The finding it fixed is fixed (the next exploration replays it
        // first), and the merged branch and its worktree go (C-54).
        if let Some(finding) = self.cycle().replay.clone() {
            let file = crate::knowledge::Knowledge::file(&self.setup.store, &repository);
            let project = crate::knowledge::Knowledge::key(&repository);
            let number = pr.number;
            if let Err(error) = crate::knowledge::Knowledge::change(&file, &project, |k| {
                k.fixed(&finding.identity, &merged, Some(number))
            }) {
                self.event(format!(
                    "The testing knowledge could not record the fix: {error}"
                ));
            }
        }
        self.event(format!("merged as {}", builds::short(&merged)));
        self.delete_branch();
        if let Err(problem) = forge::follow(&repository, &base_branch, &merged) {
            return Err(format!("paused: {problem}"));
        }
        // A repair merged: the change it unblocks goes onto it.
        self.carry_blocked(&merged)?;
        let _ = self.events.send(Event::Merged {
            repository: repository.clone(),
        });
        if self.objective.permissions.adopt {
            Ok(Phase::Build)
        } else {
            Ok(Phase::Done)
        }
    }

    fn pull_request_body(&self) -> String {
        let cycle = self.cycle();
        let proposal = cycle.proposal.as_ref();
        let attempt = cycle.attempt().cloned().unwrap_or_default();
        let outcomes = attempt
            .checks
            .iter()
            .chain(&attempt.criteria)
            .chain(&attempt.gates)
            .map(|o| format!("- {}: {}", o.name, o.verdict))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "Made by Agentique's Orchestrator for the objective “{}” (C-53, ROADMAP §4.16), cycle {}.\n\n**Why**: {}\n\n**Serves** (C-55): {}. **Benefit**: {}\n\n**Acceptance criteria** (frozen at the proposal):\n{}\n\n**Implementer's summary**: {}\n\n**Checks on a clean checkout of the reviewed commit**:\n{}\n\n**Independent review**: {} — {}\n",
            self.objective.intent,
            cycle.n,
            proposal.map(|p| p.why.as_str()).unwrap_or(""),
            proposal.map(|p| p.serves.join(", ")).unwrap_or_default(),
            proposal.map(|p| p.benefit.as_str()).unwrap_or(""),
            proposal
                .map(|p| p
                    .criteria
                    .iter()
                    .map(|c| format!("- {}: {}", c.id, c.statement))
                    .collect::<Vec<_>>()
                    .join("\n"))
                .unwrap_or_default(),
            attempt.summary,
            outcomes,
            cycle
                .review
                .as_ref()
                .map(|r| r.verdict.as_str())
                .unwrap_or("none"),
            cycle
                .review
                .as_ref()
                .map(|r| r.findings.join("; "))
                .unwrap_or_default(),
        )
    }

    fn build(&mut self) -> Next {
        // A repair cycle that carried a blocked change builds that merge,
        // which holds both.
        let merged = self
            .cycle()
            .to_build()
            .cloned()
            .ok_or("nothing merged to build")?;
        let id = self.id();
        let n = self.cycle().n;
        if self.cycle().build.is_some() {
            return Ok(Phase::Try);
        }
        let checks: Vec<(String, String)> = self
            .cycle()
            .attempt()
            .map(|a| {
                a.checks
                    .iter()
                    .map(|c| (c.name.clone(), c.verdict.clone()))
                    .collect()
            })
            .unwrap_or_default();
        self.event(format!("release build of {}", builds::short(&merged)));
        let repository = self.objective.repository.clone();
        let root = self.setup.builds.clone();
        let stop = self.controls.stop.clone();
        let task = format!("{id} cycle {n}");
        let build = self.setup.store.once(
            &id,
            &format!("cycle-{n}/build-{}", builds::short(&merged)),
            || builds::build(&repository, &merged, Some(task), checks, &root, stop).map(|m| m.id),
        )?;
        self.event(format!("Built {build}"));
        self.cycle_mut().build = Some(build);
        Ok(Phase::Try)
    }

    fn trial(&mut self) -> Next {
        let build = self.cycle().build.clone().ok_or("no build to try")?;
        // The commit built: a carried change's merge, or this cycle's own.
        let merged = self.cycle().to_build().cloned().ok_or("nothing merged")?;
        let exe = self.setup.builds.join(&build).join(agq_launcher::STUDIO);
        let manifest = agq_launcher::Manifest::load(&self.setup.builds.join(&build))?;
        manifest.matches(&self.setup.builds.join(&build))?;
        if manifest.commit != merged {
            return Err(format!(
                "the build {build} is of {}, not the merged {merged}",
                manifest.commit
            ));
        }
        let source = self.checkout("trial", &merged)?;
        let mut instance =
            TestInstance::start(&exe, &self.folder("trial-instance"), &source, &source)?;
        let mut outcomes = Vec::new();
        match instance.connect(Duration::from_secs(180)) {
            Err(problem) => outcomes.push(Outcome {
                name: "it starts".into(),
                verdict: "failed".into(),
                detail: problem,
                judged: false,
            }),
            Ok(mut client) => {
                outcomes.push(Outcome {
                    name: "it starts".into(),
                    verdict: "passed".into(),
                    detail: String::new(),
                    judged: false,
                });
                let observed = client.observe(false)?;
                let opened = observed["project"]["folder"].as_str().map(PathBuf::from);
                outcomes.push(if opened.is_some_and(|p| p.ends_with("trial")) {
                    Outcome {
                        name: "it opens the project".into(),
                        verdict: "passed".into(),
                        detail: String::new(),
                        judged: false,
                    }
                } else {
                    Outcome {
                        name: "it opens the project".into(),
                        verdict: "failed".into(),
                        detail: format!("it shows {}", observed["screen"]),
                        judged: false,
                    }
                });
                if let Some(dialog) = Self::dialog_at_start(&mut client) {
                    outcomes.push(Outcome {
                        name: "no dialog when it starts".into(),
                        verdict: "failed".into(),
                        detail: format!("the {dialog} dialog is open when the build starts"),
                        judged: false,
                    });
                }
            }
        }
        drop(instance);
        // The criteria observable in the application, each in its stated
        // condition (the build is merged, but its Assistant is the stand-in:
        // a trial spends nothing).
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let observed: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Observation { .. }))
            .collect();
        if !observed.is_empty() {
            // A merged build: one without the newer flags still starts.
            let merged_build = Options {
                speed: Some("instant".into()),
                stand_in: true,
                ..Options::default()
            };
            outcomes.extend(
                self.observe_criteria(&exe, &source, &observed, "trial", &merged_build)
                    .into_iter()
                    .map(|seen| seen.outcome),
            );
        }
        let passed = outcomes.iter().all(Outcome::passed);
        let lines = outcomes
            .iter()
            .map(|o| format!("- {}: {}", o.name, o.verdict))
            .collect::<Vec<_>>()
            .join("\n");
        self.post(
            ThreadEntry::event(if passed {
                format!("The build {build} passed its trial")
            } else {
                format!("The build {build} did not pass its trial")
            })
            .with_details(lines),
        );
        self.cycle_mut().trial = outcomes;
        if passed {
            Ok(Phase::Adopt)
        } else {
            Err(format!(
                "the build {build} did not pass its trial; it is not adopted"
            ))
        }
    }

    /// Hands over to the adopted build: `None` when the handover started
    /// (this Studio ends), else what to do instead.
    fn adopt(&mut self) -> Result<Option<Phase>, String> {
        let build = self.cycle().build.clone().ok_or("no build to adopt")?;
        if !self.objective.permissions.adopt {
            return Ok(Some(Phase::Done));
        }
        self.objective.continuation = Some(Continuation {
            cycle: self.cycle().n,
            build: build.clone(),
            at: agq_launcher::now(),
        });
        self.cycle_mut().phase = Phase::Resume;
        self.save();
        self.event(format!("handing over to {build}"));
        let (reply, answer) = std::sync::mpsc::channel();
        let _ = self.events.send(Event::Adopt {
            build: build.clone(),
            reply,
        });
        match answer.recv_timeout(Duration::from_secs(120)) {
            Ok(Ok(())) => Ok(None),
            Ok(Err(problem)) => {
                self.objective.continuation = None;
                Err(format!("the build {build} could not be adopted: {problem}"))
            }
            Err(_) => {
                self.objective.continuation = None;
                Err("the Studio did not hand over".into())
            }
        }
    }

    /// In the adopted build (or the last known good one, if it did not
    /// start): records how the adoption went and goes on.
    fn resume(&mut self) -> Next {
        let continuation = self
            .objective
            .continuation
            .take()
            .ok_or("nothing to resume")?;
        let running = self.setup.running_build.clone();
        if running.as_deref() == Some(continuation.build.as_str()) {
            self.cycle_mut().adopted = true;
            // A change carried onto this repair is adopted with it.
            if self.cycle().carried.is_some()
                && let Some(n) = self.cycle().repairs
                && let Some(carried) = self.objective.cycles.iter_mut().find(|c| c.n == n)
            {
                carried.adopted = true;
            }
            self.event(format!(
                "running the adopted build {}; going on",
                continuation.build
            ));
            Ok(Phase::Done)
        } else {
            Err(format!(
                "the adopted build {} did not take over (this is {}); the last known good build goes on",
                continuation.build,
                running.as_deref().unwrap_or("a development build")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A user-facing change counts as evaluated only when something
    /// behavioural ran in a test instance (passing or failing).
    #[test]
    fn a_user_facing_change_is_evaluated_only_when_something_behavioural_ran() {
        let proposal = record::Proposal {
            title: "t".into(),
            kind: "usability".into(),
            why: "w".into(),
            parts: Vec::new(),
            plan: Vec::new(),
            criteria: vec![
                Criterion {
                    id: "c1".into(),
                    statement: "s".into(),
                    check: Check::Command {
                        program: vec!["cargo".into(), "test".into()],
                    },
                },
                Criterion {
                    id: "c2".into(),
                    statement: "s".into(),
                    check: Check::Judgment,
                },
            ],
            intended_test_changes: Vec::new(),
            ..record::Proposal::default()
        };
        let files = vec!["crates/studio-native/src/a.rs".to_string()];
        let not_run = [
            Outcome::new("c1: cargo test", "passed", ""),
            Outcome::new("c2", "not run", "its test instance did not start"),
            Outcome::new(evidence::CHANGED_AREAS, "not run", "it did not explore"),
        ];
        assert!(!evaluated(&not_run, &proposal, &files).passed());
        let ran = [
            Outcome::new("c2", "not run", "x"),
            Outcome::new(record::REPLAY, "failed", "it still fails"),
        ];
        let outcome = evaluated(&ran, &proposal, &files);
        assert!(outcome.passed() && outcome.detail.starts_with("replay"));
    }

    #[test]
    fn a_test_run_says_how_many_tests_ran() {
        let cargo = "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out\n\ntest result: ok. 3 passed; 1 failed; 0 ignored";
        assert_eq!(tests_ran(cargo), Some(4));
        assert_eq!(
            tests_ran("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out"),
            Some(0)
        );
        assert_eq!(tests_ran("# tests 2\n# pass 2\n# fail 0"), Some(2));
        assert_eq!(tests_ran("Ran 5 tests in 0.010s\n\nOK"), Some(5));
        assert_eq!(tests_ran("Compiling things"), None);
    }
}
