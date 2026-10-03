//! Objectives in the Studio (C-53, ROADMAP §4.16): the Operator gives an
//! intent with budgets and permissions in the Objectives panel; the
//! Orchestrator (`agq-orchestrator`) takes it through its cycles on its own
//! thread, and the Studio shows its record and activity, carries its
//! requests (an observation of this Studio for the lead, the handover to an
//! adopted build), and passes on the Operator's Pause, Step, Resume, Stop
//! and messages. An objective the adopted build is to continue starts again
//! by itself; one interrupted when Agentique closed waits for Continue.

use crate::studio::{Dirty, Studio};
use agq_assistant::claude_agent::{self, ClaudeAgent, Installation};
use agq_orchestrator::record::{Budgets, Objective, Permissions, Store};
use agq_orchestrator::run::{self, Command, Event, Handle, RuntimeFactory, Setup};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Activity lines kept.
const ACTIVITY: usize = 300;

/// One line of an objective's activity.
#[derive(Clone, Debug)]
pub struct Line {
    pub at: String,
    pub role: String,
    pub text: String,
}

#[derive(Default)]
pub struct ObjectivesState {
    pub handle: Option<Handle>,
    /// The objective shown: the running one, or the last one.
    pub current: Option<Objective>,
    pub activity: VecDeque<Line>,
    /// Why the last start or command did not happen.
    pub message: Option<String>,
    /// Looked at start for an objective to continue.
    looked: bool,
}

impl ObjectivesState {
    pub fn running(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| !h.finished())
    }

    /// Whether the Operator paused the running objective (as soon as they
    /// did, also while an agent's session holds at its next tool call).
    pub fn paused(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| h.paused())
    }

    /// Whether the workspace's tick should poll.
    pub fn wants_poll(&self) -> bool {
        self.handle.is_some() || !self.looked
    }

    fn note(&mut self, role: &str, text: impl Into<String>) {
        let now = agq_launcher::now();
        self.activity.push_back(Line {
            at: now.get(11..19).unwrap_or(&now).to_string(),
            role: role.to_string(),
            text: text.into(),
        });
        while self.activity.len() > ACTIVITY {
            self.activity.pop_front();
        }
    }
}

impl Studio {
    /// The objectives' records, beside the session file.
    pub fn objective_store(&self) -> Store {
        Store::new(
            self.session_path
                .parent()
                .unwrap_or(Path::new("."))
                .join("objectives"),
        )
    }

    /// Where the objectives' worktrees, checkouts, test instances and build
    /// folder go: beside the builds, in the local app data.
    fn objective_work(&self) -> PathBuf {
        let builds = self.builds_root();
        builds
            .parent()
            .map(|p| p.join("work"))
            .unwrap_or_else(|| builds.join("work"))
    }

    /// What the Orchestrator works with: the Claude Agent runtime as the
    /// Settings configure it, the keys never to be in a change, Agentique's
    /// required checks, the protected paths.
    fn objective_setup(&self, repository: &Path) -> Result<Setup, String> {
        let endpoint =
            crate::agent_runtime::model_access(self.settings.text("assistant.provider").as_str());
        let provider = endpoint
            .as_ref()
            .map(|e| e.provider)
            .unwrap_or(agq_providers::Provider::Anthropic);
        let node = claude_agent::find_node()?;
        let installation = Installation {
            root: Installation::default_root(),
        };
        if !installation.installed() {
            return Err(
                "the Claude Agent runtime is not installed: Settings › Assistant › Install".into(),
            );
        }
        if agq_providers::runtime_key(provider)?.is_none() {
            return Err(format!(
                "the agents need a {} key: Settings › Providers",
                provider.name()
            ));
        }
        let model = endpoint
            .as_ref()
            .map(|_| crate::agent_runtime::DEEPSEEK_MODEL.to_string());
        let data = self.claude_agent_data();
        let runtime: RuntimeFactory = Box::new(move |_| {
            let key = agq_providers::runtime_key(provider)?
                .ok_or_else(|| format!("the {} key is gone", provider.name()))?;
            let mut agent = ClaudeAgent::new(
                node.clone(),
                installation.clone(),
                data.clone(),
                model.clone(),
                None,
                key,
            );
            agent.endpoint = endpoint.clone();
            Ok(agent)
        });
        let keys = agq_providers::Provider::ALL
            .into_iter()
            .filter_map(|p| agq_providers::runtime_key(p).ok().flatten())
            .map(|k| k.expose().to_string())
            .filter(|k| k.len() >= 12)
            .collect();
        let checks = agq_implementation::task::ProjectChecks::agentique()
            .commands
            .into_iter()
            .map(|c| c.program)
            .collect();
        // The links' protected paths are part of the gates: an objective
        // does not start without them.
        let links = std::fs::read_to_string(repository.join("model").join("links.json"))
            .map_err(|e| format!("the repository's model/links.json cannot be read: {e}"))?;
        let protected = agq_implementation::links::Links::parse(&links)
            .map_err(|e| format!("the repository's model/links.json cannot be read: {e}"))?
            .protected;
        Ok(Setup {
            store: self.objective_store(),
            work: self.objective_work(),
            builds: self.builds_root(),
            runtime,
            keys,
            checks,
            protected,
            running_build: self.running_build(),
        })
    }

    /// Starts an objective for `intent` on Agentique's own repository.
    pub fn start_objective(
        &mut self,
        intent: &str,
        budgets: Budgets,
        permissions: Permissions,
    ) -> Result<(), String> {
        if self.refused_to_agents("starting an objective") {
            return Err("starting an objective is the Operator's own".into());
        }
        if self.objectives.running() {
            return Err("an objective is running: stop it first".into());
        }
        if intent.trim().is_empty() {
            return Err("say what to improve".into());
        }
        let repository = self
            .agentique_repository()
            .ok_or("this Agentique does not know where its repository is")?;
        let store = self.objective_store();
        if let Some(active) = store.active() {
            return Err(format!(
                "the objective “{}” is not finished: continue or stop it first",
                active.intent
            ));
        }
        let setup = self.objective_setup(&repository)?;
        let objective = store.create(intent, &repository, "main", budgets, permissions)?;
        self.objectives.activity.clear();
        self.objectives
            .note("orchestrator", format!("Started: {intent}"));
        self.objectives.current = Some(objective.clone());
        self.objectives.handle = Some(run::start(setup, objective));
        self.objectives.message = None;
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
        Ok(())
    }

    /// Continues the objective that is not finished (interrupted, or handed
    /// over to this build).
    pub fn continue_objective(&mut self) -> Result<(), String> {
        if self.refused_to_agents("continuing an objective") {
            return Err("continuing an objective is the Operator's own".into());
        }
        if self.objectives.running() {
            return Ok(());
        }
        let store = self.objective_store();
        let objective = store.active().ok_or("there is no objective to continue")?;
        let setup = self.objective_setup(&objective.repository)?;
        self.objectives
            .note("orchestrator", format!("Continuing: {}", objective.intent));
        self.objectives.current = Some(objective.clone());
        self.objectives.handle = Some(run::start(setup, objective));
        self.objectives.message = None;
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
        Ok(())
    }

    /// Stops an objective that is not running (one interrupted earlier).
    pub fn stop_idle_objective(&mut self) {
        if self.refused_to_agents("stopping an objective") {
            return;
        }
        let store = self.objective_store();
        if let Some(mut objective) = store.active() {
            objective.state = agq_orchestrator::record::State::Stopped;
            objective.note = Some("Stopped by the Operator.".into());
            let _ = store.save(&objective);
            self.objectives.current = Some(objective);
            self.mark(Dirty::LAYOUT);
        }
    }

    /// The Operator's command to the running objective.
    pub fn objective_command(&mut self, command: Command) {
        if self.refused_to_agents("steering an objective") {
            return;
        }
        if let Some(handle) = &self.objectives.handle {
            let what = match &command {
                Command::Message(text) => format!("you: {text}"),
                other => format!("you: {other:?}").to_lowercase(),
            };
            handle.send(command);
            self.objectives.note("operator", what);
            self.mark(Dirty::LAYOUT);
        }
    }

    /// Ends the running objective's sessions and waits up to `within` for
    /// it to save where it was (Agentique is closing or handing over).
    pub fn interrupt_objective(&mut self, within: Duration) {
        let Some(handle) = &self.objectives.handle else {
            return;
        };
        if handle.finished() {
            return;
        }
        handle.send(Command::Interrupt);
        let started = std::time::Instant::now();
        while !handle.finished() && started.elapsed() < within {
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Takes the objective's events; at the first call, continues an
    /// objective handed over to this build. Returns whether anything
    /// changed.
    pub fn poll_objective(&mut self) -> bool {
        let mut changed = false;
        if !self.objectives.looked {
            self.objectives.looked = true;
            let store = self.objective_store();
            if let Some(active) = store.active() {
                self.objectives.current = Some(active.clone());
                changed = true;
                if active.continuation.is_some() && !self.safe_mode {
                    if let Err(problem) = self.continue_objective() {
                        self.objectives.message =
                            Some(format!("The objective could not continue: {problem}"));
                    }
                } else {
                    self.objectives.message = Some(
                        "An objective is not finished: Continue goes on from where it was.".into(),
                    );
                }
            }
        }
        let Some(handle) = self.objectives.handle.as_ref() else {
            return changed;
        };
        let mut events = Vec::new();
        while let Ok(event) = handle.events.try_recv() {
            events.push(event);
        }
        let finished = handle.finished() && events.is_empty();
        for event in events {
            changed = true;
            match event {
                Event::Changed(objective) => self.objectives.current = Some(*objective),
                Event::Activity { role, text } => self.objectives.note(&role, text),
                Event::Control { body, reply } => {
                    self.control.submit(crate::control::Request::new(
                        body,
                        crate::control::Reply::Channel(reply),
                        Duration::from_secs(55),
                    ));
                }
                Event::Merged { repository } => {
                    // The merged change moved the files under an open
                    // project of this repository: read it again before
                    // anything is saved through it.
                    let open = self.project.as_ref().map(|p| p.folder().to_path_buf());
                    let same = |a: &Path, b: &Path| {
                        a.canonicalize()
                            .ok()
                            .zip(b.canonicalize().ok())
                            .is_some_and(|(a, b)| a == b)
                    };
                    if let Some(folder) = open.filter(|f| same(f, &repository)) {
                        self.open_project(&folder);
                        self.objectives
                            .note("orchestrator", "the project was read again after the merge");
                    }
                }
                Event::Adopt { build, reply } => {
                    let result = self.use_build(&build);
                    if let Err(problem) = &result {
                        self.objectives
                            .note("orchestrator", format!("Not adopted: {problem}"));
                    }
                    let _ = reply.send(result);
                }
            }
        }
        if finished {
            self.objectives.handle = None;
            changed = true;
        }
        if changed {
            self.mark(Dirty::LAYOUT | Dirty::STATUS);
        }
        changed
    }

    /// One line for the status bar while an objective runs.
    pub fn objective_status(&self) -> Option<String> {
        let objective = self.objectives.current.as_ref()?;
        if !self.objectives.running() {
            return None;
        }
        let cycle = objective.cycle();
        Some(format!(
            "Objective: cycle {} · {} · ${:.2} of ${:.2}",
            cycle.map(|c| c.n).unwrap_or(0),
            cycle.map(|c| c.phase.label()).unwrap_or("starting"),
            objective.spent.usd,
            objective.budgets.usd
        ))
    }
}
