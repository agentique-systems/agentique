//! The Orchestrator's driver (C-53, ROADMAP §4.16): deterministic code that
//! takes an objective through its cycles. It decides the next phase from
//! recorded results, runs each role's agent session, checks what the agents
//! hand over, records every outcome and side effect, and stops on a budget
//! used up, on no progress, or when the Operator stops it.

use crate::control::{Client, TestInstance};
use crate::record::{
    Attempt, Check, Continuation, Criterion, Cycle, Objective, Outcome, Phase, Review, State, Store,
};
use crate::roles::{self, Role};
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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
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
    /// A message for the agent at work (queued into its session).
    Message(String),
}

/// What the Orchestrator tells the Studio.
pub enum Event {
    /// The record, after each change.
    Changed(Box<Objective>),
    /// What an agent or the Orchestrator does now.
    Activity { role: String, text: String },
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

/// Makes a Claude Agent runtime for a role's session.
pub type RuntimeFactory = Box<dyn Fn(&Development) -> Result<ClaudeAgent, String> + Send>;

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
}

/// A running objective, as the Studio holds it.
pub struct Handle {
    commands: Sender<Command>,
    pub events: Receiver<Event>,
    thread: Option<std::thread::JoinHandle<()>>,
    paused: Arc<AtomicBool>,
}

impl Handle {
    pub fn send(&self, command: Command) {
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

/// Starts (or resumes) `objective` on its own thread.
pub fn start(setup: Setup, objective: Objective) -> Handle {
    let (commands, received) = std::sync::mpsc::channel();
    let (events, events_received) = std::sync::mpsc::channel();
    let controls = Controls::default();
    if objective.state == State::Paused {
        controls.apply(Command::Pause);
    }
    let paused = controls.paused.clone();
    // The Operator's commands take effect as they arrive, also while an
    // agent works (its session reads the same steering and stop).
    let applied = controls.clone();
    std::thread::spawn(move || {
        let received: Receiver<Command> = received;
        for command in received {
            applied.apply(command);
        }
    });
    if let Err(error) = cargo_settings(&setup.work) {
        let _ = events.send(Event::Activity {
            role: "orchestrator".into(),
            text: format!("The build settings could not be written: {error}"),
        });
    }
    let thread = std::thread::Builder::new()
        .name("agentique-objective".into())
        .spawn(move || {
            let mut driver = Driver {
                setup,
                objective,
                events,
                controls,
                last: Instant::now(),
            };
            driver.run();
        })
        .expect("a thread can start");
    Handle {
        commands,
        events: events_received,
        thread: Some(thread),
        paused,
    }
}

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
    let number_after = |line: &str, marker: &str| -> Option<usize> {
        let rest = &line[line.find(marker)? + marker.len()..];
        rest.trim_start()
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse()
            .ok()
    };
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
}

impl Controls {
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
                self.steering.set_gate(Gate::Run);
            }
            Command::Interrupt => {
                self.interrupted.store(true, Ordering::SeqCst);
                self.stop.store(true, Ordering::SeqCst);
                self.steering.set_gate(Gate::Run);
            }
            Command::Message(text) => self.steering.queue(&text),
        }
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

struct Driver {
    setup: Setup,
    objective: Objective,
    events: Sender<Event>,
    controls: Controls,
    /// Since when time worked is not yet counted.
    last: Instant,
}

/// What a role's session handed over.
struct Session {
    submitted: Option<Value>,
    /// What the session said last, for a report.
    said: String,
}

type Next = Result<Phase, String>;

impl Driver {
    fn note(&self, role: &str, text: impl Into<String>) {
        let _ = self.events.send(Event::Activity {
            role: role.to_string(),
            text: text.into(),
        });
    }

    fn save(&mut self) {
        self.count_time();
        if let Err(error) = self.setup.store.save(&self.objective) {
            self.note(
                "orchestrator",
                format!("The objective's record could not be saved: {error}"),
            );
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
        if spent.usd >= budgets.usd {
            return Some((
                State::Failed,
                format!(
                    "The spend budget is used up (${:.2} of ${:.2}).",
                    spent.usd, budgets.usd
                ),
            ));
        }
        let hours = spent.seconds / 3600.0;
        if hours >= budgets.hours {
            return Some((
                State::Failed,
                format!(
                    "The time budget is used up ({hours:.1} of {} hours).",
                    budgets.hours
                ),
            ));
        }
        if self.controls.stopped() {
            return Some((State::Stopped, "Stopped by the Operator.".into()));
        }
        None
    }

    fn run(&mut self) {
        self.note(
            "orchestrator",
            format!("Objective: {}", self.objective.intent),
        );
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
                    let done = self.objective.cycles.len() as u32;
                    if done >= self.objective.budgets.cycles {
                        let adopted = self.objective.cycles.iter().filter(|c| c.adopted).count();
                        self.end(
                            State::Done,
                            format!("{done} cycle(s) done, {adopted} adopted."),
                        );
                        return;
                    }
                    self.objective.cycles.push(Cycle::new(done + 1));
                    self.save();
                    Phase::Propose
                }
                Some(Phase::Failed) => {
                    let blocker = self.cycle().blocker.clone().unwrap_or_default();
                    let done = self.objective.cycles.len() as u32;
                    if done >= self.objective.budgets.cycles {
                        self.end(State::Failed, blocker);
                        return;
                    }
                    self.objective.cycles.push(Cycle::new(done + 1));
                    self.save();
                    Phase::Propose
                }
                Some(phase) => phase,
            };
            self.note(
                "orchestrator",
                format!("Cycle {}: {}", self.cycle().n, phase.label()),
            );
            let next = match phase {
                Phase::Propose => self.propose(),
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
                    self.objective.state = State::Paused;
                    self.objective.note =
                        Some(blocker.trim_start_matches("paused:").trim().to_string());
                    self.controls.apply(Command::Pause);
                    self.save();
                }
                Err(blocker) => {
                    self.note(
                        "orchestrator",
                        format!("Cycle {} stopped: {blocker}", self.cycle().n),
                    );
                    let cycle = self.cycle_mut();
                    cycle.blocker = Some(blocker);
                    cycle.phase = Phase::Failed;
                    self.save();
                }
            }
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
        self.save();
        true
    }

    fn end(&mut self, state: State, note: String) {
        self.note("orchestrator", note.clone());
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
        mut test: Option<&mut Client>,
    ) -> Result<Session, String> {
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
        let mut agent = (self.setup.runtime)(&development)?;
        agent.development = Some(development);
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
        let toolset = Toolset {
            system: roles::instructions(role),
            definitions: roles::tools(role),
        };
        let submitted: RefCell<Option<Value>> = RefCell::new(None);
        let model = RefCell::new(Some(model));
        let events = self.events.clone();
        let role_name = role.name().to_string();
        let mut executor = |call: &ToolCall| -> ToolResult {
            let _ = events.send(Event::Activity {
                role: role_name.clone(),
                text: format!("{} {}", agq_assistant::phase(&call.name), call.name),
            });
            match call.name.as_str() {
                roles::SUBMIT_PROPOSAL
                | roles::SUBMIT_IMPLEMENTATION
                | roles::SUBMIT_REVIEW
                | roles::SUBMIT_EVALUATION => {
                    if call.name == roles::SUBMIT_PROPOSAL
                        && let Err(problem) = roles::read_proposal(&call.input)
                    {
                        return ToolResult::error(format!(
                            "Not accepted: {problem}. Fix it and submit again."
                        ));
                    }
                    *submitted.borrow_mut() = Some(call.input.clone());
                    ToolResult::answer("Received. End your turn now with one short sentence.")
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
        let budget = self.objective.budgets.usd;
        let cost = RefCell::new((0.0f64, 0u64, false));
        let session_id: RefCell<Option<String>> = RefCell::new(None);
        let stop = self.controls.stop.clone();
        let mut on_event = |event: TurnEvent| match event {
            TurnEvent::Stream(StreamEvent::ModelUsage { model, usage }) => {
                let mut c = cost.borrow_mut();
                match usage.cost_usd(&model) {
                    Some(usd) => c.0 += usd,
                    None => {
                        // No price known: counted at a high one, so the
                        // spend budget still stops it.
                        c.0 += unpriced(&usage);
                        c.2 = true;
                    }
                }
                c.1 += usage.input_tokens
                    + usage.output_tokens
                    + usage.cache_read_input_tokens
                    + usage.cache_creation_input_tokens;
                if spent_before + c.0 >= budget {
                    stop.store(true, Ordering::SeqCst);
                }
            }
            TurnEvent::Stream(StreamEvent::Usage(usage)) => {
                let mut c = cost.borrow_mut();
                c.1 += usage.input_tokens + usage.output_tokens;
                c.0 += unpriced(&usage);
                c.2 = true;
            }
            TurnEvent::Stream(StreamEvent::ToolCallStarted { name, .. }) => {
                let _ = self.events.send(Event::Activity {
                    role: role.name().into(),
                    text: format!("{}: {name}", agq_assistant::phase(&name)),
                });
            }
            TurnEvent::Activity(Activity::Task {
                description,
                agent,
                event,
                ..
            }) => {
                let _ = self.events.send(Event::Activity {
                    role: role.name().into(),
                    text: format!(
                        "subagent {} {:?}: {description}",
                        agent.unwrap_or_default(),
                        event
                    )
                    .to_lowercase(),
                });
            }
            TurnEvent::Entry(Entry::Session { id, .. }) => *session_id.borrow_mut() = Some(id),
            TurnEvent::Entry(Entry::Notice { text }) => {
                let _ = self.events.send(Event::Activity {
                    role: role.name().into(),
                    text,
                });
            }
            _ => {}
        };
        self.note(role.name(), "starts");
        agent.run(
            &mut conversation,
            &toolset,
            role.max_calls(),
            &mut execute,
            &mut on_event,
            &controls.stop,
        );
        drop(execute);
        controls.step.store(false, Ordering::SeqCst);
        if let Some(model) = model.borrow_mut().as_mut() {
            model.close();
        }
        let (usd, tokens, unknown) = *cost.borrow();
        self.objective.spent.usd += usd;
        self.objective.spent.tokens += tokens;
        self.objective.spent.unknown |= unknown;
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
            self.note(
                "orchestrator",
                format!(
                    "removed build settings an agent left: {}",
                    removed.join(", ")
                ),
            );
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
            self.note(
                "orchestrator",
                format!("a {dialog} dialog was in the way: {}", decision.choice),
            );
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
    fn observe_criterion(
        &mut self,
        client: &mut Client,
        criterion: &Criterion,
        setup: &[Value],
        expect: &Value,
    ) -> Outcome {
        let outcome = |verdict: &str, detail: String| Outcome {
            name: criterion.id.clone(),
            verdict: verdict.into(),
            detail,
        };
        let cleared = match self.clear_dialogs(client, &criterion.statement) {
            Ok(cleared) => cleared,
            Err(problem) => {
                return outcome(
                    "not run",
                    format!("the way to it could not be cleared: {problem}"),
                );
            }
        };
        let mut result = Ok(());
        for action in setup {
            result = match client.act("orchestrator", &criterion.id, action.clone()) {
                Ok(answer) if answer["ok"] == false => {
                    Err(format!("setup action failed: {answer}"))
                }
                Ok(_) => Ok(()),
                Err(error) => Err(error),
            };
            if result.is_err() {
                break;
            }
        }
        let result = result.and_then(|()| {
            client
                .observe(true)
                .and_then(|o| crate::control::holds(&o, expect))
        });
        let (verdict, mut detail) = match result {
            Ok(()) => ("passed", "observed".to_string()),
            Err(problem) => ("failed", problem),
        };
        if !cleared.is_empty() {
            detail = format!("{detail} (first cancelled: {})", cleared.join(", "));
        }
        outcome(verdict, detail)
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
        let head = git::head(&repository).map_err(|e| e.to_string())?;
        if head.branch.as_deref() != Some(self.objective.base_branch.as_str()) {
            return Err(format!(
                "paused: the repository is on {}, not {}",
                head.branch.as_deref().unwrap_or("a detached commit"),
                self.objective.base_branch
            ));
        }
        let base = self.cycle().base.clone().unwrap_or(head.commit);
        self.cycle_mut().base = Some(base.clone());
        let lead = self.checkout("lead", &base)?;
        let policy = self.policy(&lead, false, false);
        let mut brief = roles::brief(Role::Lead, &self.objective, "");
        for attempt in 0..2 {
            let session = self.session(
                Role::Lead,
                &lead,
                policy.clone(),
                brief.clone(),
                attempt > 0,
                None,
            )?;
            match session.submitted.as_ref().map(roles::read_proposal) {
                Some(Ok(proposal)) => {
                    self.note("lead", format!("proposes: {}", proposal.title));
                    self.cycle_mut().proposal = Some(proposal);
                    let _ = git::remove_worktree(
                        &repository,
                        &format!("{}-{}-lead", self.id(), self.cycle().n),
                    );
                    return Ok(Phase::Implement);
                }
                Some(Err(problem)) => {
                    brief = format!(
                        "Your proposal was not accepted: {problem}. Submit a corrected one."
                    )
                }
                None => {
                    brief =
                        "You ended without submit_proposal. Choose one improvement and submit it."
                            .into()
                }
            }
            if self.over().is_some() {
                break;
            }
        }
        Err("the lead did not hand over an acceptable proposal".into())
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
        let brief = roles::brief(Role::Implementer, &self.objective, &context);
        let session = self.session(Role::Implementer, &work, policy, brief, repairing, None)?;
        let summary = session
            .submitted
            .as_ref()
            .and_then(|v| v["summary"].as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("(no summary; it said: {})", session.said));
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
        self.note(
            "implementer",
            format!("committed {}", builds::short(&commit)),
        );
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
                }
            }
            Ok(finished) if finished.success => Outcome {
                name,
                verdict: "passed".into(),
                detail: String::new(),
            },
            Ok(finished) => Outcome {
                name,
                verdict: "failed".into(),
                detail: agq_execution::process::last_lines(
                    &format!("{}\n{}", finished.stdout, finished.stderr),
                    40,
                ),
            },
            Err(error) => Outcome {
                name,
                verdict: "not run".into(),
                detail: error.to_string(),
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
        // Once per cycle: the command criteria on the base must fail (or
        // run no test), so that passing after shows the change.
        if self.cycle().before.is_empty() {
            let before = self.checkout("base", &base)?;
            let mut outcomes = Vec::new();
            for criterion in &proposal.criteria {
                if let Check::Command { program } = &criterion.check {
                    let mut outcome = self.criterion(&before, program, Duration::from_secs(1800));
                    outcome.name = criterion.id.clone();
                    outcomes.push(outcome);
                }
            }
            if self.controls.stopped() {
                return Err("stopped".into());
            }
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
            self.note("orchestrator", format!("check: {}", words.join(" ")));
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
        // A repository without a model has nothing locked, unless the
        // change removed the model.
        let removes_model = patch
            .files
            .iter()
            .any(|f| f.path.starts_with("model/") && f.status == "deleted");
        let unlocked = if verify.join("model").is_dir() {
            agq_assistant::model_tools::locked_changes(
                &verify,
                &base,
                &self.objective.permissions.locked,
            )
        } else if removes_model {
            Ok(vec!["the model (the change removes it)".to_string()])
        } else {
            Ok(Vec::new())
        };
        gates.push(match unlocked {
            Ok(found) if found.is_empty() => Outcome {
                name: "locked elements unchanged".into(),
                verdict: "passed".into(),
                detail: String::new(),
            },
            Ok(found) => Outcome {
                name: "locked elements unchanged".into(),
                verdict: "failed".into(),
                detail: format!(
                    "the change touches locked elements the objective does not name: {}",
                    found.join(", ")
                ),
            },
            Err(error) => Outcome {
                name: "locked elements unchanged".into(),
                verdict: "not run".into(),
                detail: error,
            },
        });
        // The code of locked parts, by the links (the change's and the
        // base's), unless the objective names the part.
        let files: Vec<String> = patch
            .files
            .iter()
            .map(|f| f.path.replace('\\', "/"))
            .collect();
        let mut links = Vec::new();
        if let Ok(text) = std::fs::read_to_string(verify.join("model").join("links.json")) {
            links.push(text);
        }
        if let Ok(shown) = forge::run(
            &repository,
            &["git", "show", &format!("{base}:model/links.json")],
            Duration::from_secs(60),
        ) {
            links.push(shown.stdout);
        }
        gates.push(if verify.join("model").is_dir() {
            match agq_assistant::model_tools::locked_code(
                &verify,
                &base,
                &files,
                &links,
                &self.objective.permissions.locked,
            ) {
                Ok(found) if found.is_empty() => Outcome {
                    name: "code of locked parts unchanged".into(),
                    verdict: "passed".into(),
                    detail: String::new(),
                },
                Ok(found) => Outcome {
                    name: "code of locked parts unchanged".into(),
                    verdict: "failed".into(),
                    detail: format!(
                        "the change touches the code of locked parts the objective does not name: {}",
                        found.join(", ")
                    ),
                },
                Err(error) => Outcome {
                    name: "code of locked parts unchanged".into(),
                    verdict: "not run".into(),
                    detail: error,
                },
            }
        } else {
            Outcome {
                name: "code of locked parts unchanged".into(),
                verdict: "passed".into(),
                detail: "no model, so nothing is locked".into(),
            }
        });
        // The criteria failed before the change.
        let already: Vec<String> = self
            .cycle()
            .before
            .iter()
            .filter(|o| o.passed())
            .map(|o| o.name.clone())
            .collect();
        gates.push(Outcome {
            name: "the criteria fail before the change".into(),
            verdict: if already.is_empty() {
                "passed"
            } else {
                "failed"
            }
            .into(),
            detail: if already.is_empty() {
                String::new()
            } else {
                format!(
                    "{} already pass on the base, so they do not show the change",
                    already.join(", ")
                )
            },
        });
        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
        attempt.checks = checks;
        attempt.criteria = criteria;
        attempt.gates = gates;
        let failed = attempt.failures();
        if failed.is_empty() {
            self.note(
                "orchestrator",
                "every required check, command criterion and gate passed",
            );
            Ok(Phase::Evaluate)
        } else {
            self.note(
                "orchestrator",
                format!("{} failed: {}", failed.len(), failed.join("; ")),
            );
            Ok(Phase::Repair)
        }
    }

    fn evaluate(&mut self) -> Next {
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let behavioural: Vec<_> = proposal
            .criteria
            .iter()
            .filter(|c| !matches!(c.check, Check::Command { .. }))
            .cloned()
            .collect();
        if behavioural.is_empty() {
            return Ok(Phase::Review);
        }
        let commit = self
            .cycle()
            .attempt()
            .and_then(|a| a.commit.clone())
            .ok_or("nothing to evaluate")?;
        let verify = self.checkout("verify", &commit)?;
        self.note("orchestrator", "building the change for a test instance");
        let exe = builds::debug_studio(&verify, &self.target(), self.controls.stop.clone())?;
        let mut instance = TestInstance::start(&exe, &self.folder("instance"), &verify, &verify)?;
        let mut client = instance.connect(Duration::from_secs(180))?;
        let mut outcomes = Vec::new();
        if let Some(dialog) = Self::dialog_at_start(&mut client) {
            outcomes.push(Outcome {
                name: "no dialog when it starts".into(),
                verdict: "failed".into(),
                detail: format!("the {dialog} dialog is open when the test instance starts"),
            });
        }
        for criterion in &behavioural {
            if let Check::Observation { setup, expect } = &criterion.check {
                let outcome = self.observe_criterion(&mut client, criterion, setup, expect);
                outcomes.push(outcome);
            }
        }
        let judged: Vec<_> = behavioural
            .iter()
            .filter(|c| matches!(c.check, Check::Judgment))
            .collect();
        if !judged.is_empty() {
            let context = format!(
                "Criteria to judge in the test instance: {}\nThe test instance shows the cycle's commit {} with the project open.",
                judged
                    .iter()
                    .map(|c| format!("{} ({})", c.id, c.statement))
                    .collect::<Vec<_>>()
                    .join("; "),
                builds::short(&commit)
            );
            let brief = roles::brief(Role::Evaluator, &self.objective, &context);
            let policy = self.policy(&verify, false, false);
            let session = self.session(
                Role::Evaluator,
                &verify,
                policy,
                brief,
                false,
                Some(&mut client),
            )?;
            let reported = session.submitted.unwrap_or_default();
            for criterion in judged {
                let found = reported["criteria"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|c| c["id"] == criterion.id.as_str());
                outcomes.push(match found {
                    Some(c) => Outcome {
                        name: criterion.id.clone(),
                        verdict: c["outcome"].as_str().unwrap_or("not run").to_string(),
                        detail: c["observations"].as_str().unwrap_or_default().to_string(),
                    },
                    None => Outcome {
                        name: criterion.id.clone(),
                        verdict: "not run".into(),
                        detail: "the evaluator did not report it".into(),
                    },
                });
            }
        }
        drop(client);
        drop(instance);
        if self.controls.stopped() {
            return Err("stopped".into());
        }
        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
        attempt.criteria.extend(outcomes);
        if attempt.failures().is_empty() {
            Ok(Phase::Review)
        } else {
            Ok(Phase::Repair)
        }
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
        let outcomes = attempt
            .checks
            .iter()
            .chain(&attempt.criteria)
            .chain(&attempt.gates)
            .map(|o| format!("- {}: {}", o.name, o.verdict))
            .collect::<Vec<_>>()
            .join("\n");
        let changes = gates::listed_test_changes(&patch);
        let context = format!(
            "The implementer's summary: {}\n\nOutcomes:\n{outcomes}\n\nChanges to tests, checks or budgets the baseline guard lists: {}\n\nThe diff against {}:\n{diff}",
            attempt.summary,
            if changes.is_empty() {
                "none".to_string()
            } else {
                changes.join("; ")
            },
            builds::short(&base),
        );
        let brief = roles::brief(Role::Reviewer, &self.objective, &context);
        let policy = self.policy(&verify, false, false);
        let session = self.session(Role::Reviewer, &verify, policy, brief, false, None)?;
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
        };
        self.note(
            "reviewer",
            format!("{}: {}", review.verdict, review.findings.join("; ")),
        );
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let baseline = gates::baseline(&patch, &proposal, Some(&review));
        let approved = review.verdict == "approve";
        self.cycle_mut().review = Some(review);
        let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
        attempt.gates.retain(|g| g.name != baseline.name);
        let kept = baseline.passed();
        attempt.gates.push(baseline);
        if approved && kept {
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
        let parent = self.cycle().pushed.clone().unwrap_or(base);
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
        self.note(
            "orchestrator",
            format!(
                "pull request {} opened; waiting for the repository's checks",
                pr.url
            ),
        );
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
                let attempt = self.cycle_mut().attempts.last_mut().expect("an attempt");
                attempt.gates.push(Outcome {
                    name: "the repository's checks".into(),
                    verdict: "failed".into(),
                    detail: what,
                });
                return Ok(Phase::Repair);
            }
            forge::Checks::Pending => {
                return Err("the repository's checks did not finish in time".into());
            }
        }
        let subject = format!("{} (#{})", proposal.title, pr.number);
        let merged = self
            .setup
            .store
            .once(&id, &format!("cycle-{n}/merge-{short}"), || {
                forge::merge(&repository, pr.number, &pushed, &subject)
            })?;
        self.cycle_mut().merged = Some(merged.clone());
        self.save();
        self.note(
            "orchestrator",
            format!("merged as {}", builds::short(&merged)),
        );
        if let Err(problem) = forge::follow(&repository, &base_branch, &merged) {
            return Err(format!("paused: {problem}"));
        }
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
            "Made by Agentique's Orchestrator for the objective “{}” (C-53, ROADMAP §4.16), cycle {}.\n\n**Why**: {}\n\n**Acceptance criteria** (frozen at the proposal):\n{}\n\n**Implementer's summary**: {}\n\n**Checks on a clean checkout of the reviewed commit**:\n{}\n\n**Independent review**: {} — {}\n",
            self.objective.intent,
            cycle.n,
            proposal.map(|p| p.why.as_str()).unwrap_or(""),
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
        let merged = self
            .cycle()
            .merged
            .clone()
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
        self.note(
            "orchestrator",
            format!("release build of {}", builds::short(&merged)),
        );
        let repository = self.objective.repository.clone();
        let root = self.setup.builds.clone();
        let stop = self.controls.stop.clone();
        let task = format!("{id} cycle {n}");
        let build = self.setup.store.once(
            &id,
            &format!("cycle-{n}/build-{}", builds::short(&merged)),
            || builds::build(&repository, &merged, Some(task), checks, &root, stop).map(|m| m.id),
        )?;
        self.cycle_mut().build = Some(build);
        Ok(Phase::Try)
    }

    fn trial(&mut self) -> Next {
        let build = self.cycle().build.clone().ok_or("no build to try")?;
        let merged = self.cycle().merged.clone().ok_or("nothing merged")?;
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
            }),
            Ok(mut client) => {
                outcomes.push(Outcome {
                    name: "it starts".into(),
                    verdict: "passed".into(),
                    detail: String::new(),
                });
                let observed = client.observe(false)?;
                let opened = observed["project"]["folder"].as_str().map(PathBuf::from);
                outcomes.push(if opened.is_some_and(|p| p.ends_with("trial")) {
                    Outcome {
                        name: "it opens the project".into(),
                        verdict: "passed".into(),
                        detail: String::new(),
                    }
                } else {
                    Outcome {
                        name: "it opens the project".into(),
                        verdict: "failed".into(),
                        detail: format!("it shows {}", observed["screen"]),
                    }
                });
                if let Some(dialog) = Self::dialog_at_start(&mut client) {
                    outcomes.push(Outcome {
                        name: "no dialog when it starts".into(),
                        verdict: "failed".into(),
                        detail: format!("the {dialog} dialog is open when the build starts"),
                    });
                }
                let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
                for criterion in &proposal.criteria {
                    if let Check::Observation { setup, expect } = &criterion.check {
                        let outcome = self.observe_criterion(&mut client, criterion, setup, expect);
                        outcomes.push(outcome);
                    }
                }
            }
        }
        drop(instance);
        let passed = outcomes.iter().all(Outcome::passed);
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
        self.note("orchestrator", format!("handing over to {build}"));
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
            self.note(
                "orchestrator",
                format!("running the adopted build {}; going on", continuation.build),
            );
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
