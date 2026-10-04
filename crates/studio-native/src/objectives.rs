//! Objectives in the Studio (C-53, ROADMAP §4.16): the Operator gives an
//! intent with budgets and permissions in the Objectives panel; the
//! Orchestrator (`agq-orchestrator`) takes it through its cycles on its own
//! thread, and the Studio shows its record and its thread (C-54: read from
//! the objective's records, then followed as entries are added), carries its
//! requests (an observation of this Studio for the lead, the handover to an
//! adopted build), and passes on the Operator's Pause, Step, Resume, Stop
//! and messages. An objective the adopted build is to continue starts again
//! by itself; one interrupted when Agentique closed waits for Continue.
//! Before an objective starts, the Studio resolves each role's model from
//! Settings and the credentials (C-54) and records it in the objective; the
//! Orchestrator builds every session of a role on that model, effort and
//! credential, and never moves a role to another.

use crate::studio::{Dirty, Studio};
use agq_assistant::claude_agent::{self, Installation};
use agq_orchestrator::record::{Access, Budgets, Objective, Permissions, Store};
use agq_orchestrator::run::{self, Command, Event, Handle, RuntimeFactory, Setup};
use agq_orchestrator::thread::{Author, Kind, ThreadEntry};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The latest entries of the shown objective's thread kept in memory (the
/// thread itself is in its records).
const SHOWN: usize = 1000;

#[derive(Default)]
pub struct ObjectivesState {
    pub handle: Option<Handle>,
    /// The objective shown: the running one, or the last one.
    pub current: Option<Objective>,
    /// The latest entries of its thread, in order.
    pub thread: Vec<ThreadEntry>,
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

    /// Shows `entry` of the thread in its place (entries can arrive out of
    /// order from the Orchestrator's thread and the Studio's own), each
    /// once; an entry that could not be kept (number 0) stays where it
    /// arrived, after what was shown then.
    pub fn add(&mut self, entry: ThreadEntry) {
        if entry.seq != 0 && self.thread.iter().any(|e| e.seq == entry.seq) {
            return;
        }
        let at = if entry.seq == 0 {
            self.thread.len()
        } else {
            let mut at = self
                .thread
                .iter()
                .rposition(|e| e.seq != 0 && e.seq < entry.seq)
                .map_or(0, |i| i + 1);
            while self.thread.get(at).is_some_and(|e| e.seq == 0) {
                at += 1;
            }
            at
        };
        self.thread.insert(at, entry);
        if self.thread.len() > SHOWN {
            self.thread.drain(..self.thread.len() - SHOWN);
        }
    }

    /// Reads the thread of objective `id` from its records.
    fn read(&mut self, store: &Store, id: &str) {
        self.thread.clear();
        for entry in store.thread(id, 0) {
            self.add(entry);
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

    /// What the Orchestrator works with: the Claude Agent runtime building
    /// each role's sessions on the model, effort and credential the
    /// objective recorded for it (C-54), the keys and token never to be in a
    /// change, Agentique's required checks, the protected paths.
    fn objective_setup(&self, repository: &Path) -> Result<Setup, String> {
        let node = claude_agent::find_node()?;
        let installation = Installation {
            root: Installation::default_root(),
        };
        if !installation.installed() {
            return Err(
                "the Claude Agent runtime is not installed: Settings › Assistant › Install".into(),
            );
        }
        let data = self.claude_agent_data();
        let runtime: RuntimeFactory = Box::new(move |model, _| {
            // The credential recorded for the role, and no other: a token
            // or key that is gone stops the session, never moves it to
            // another credential or account.
            let credential = match model.access {
                Access::Subscription => agq_providers::Credential::ClaudeSubscription,
                Access::Key => agq_providers::Credential::Key(model.model.provider),
            };
            let secret = agq_providers::runtime_credential(credential)?.ok_or_else(|| {
                format!(
                    "the {} the {} runs on is gone; it does not move to another credential",
                    credential.name(),
                    model.role
                )
            })?;
            crate::agent_runtime::runtime_on(
                credential,
                secret,
                node.clone(),
                installation.clone(),
                data.clone(),
                Some(model.model.model.clone()),
                model.effort.clone(),
            )
        });
        let keys = agq_providers::Provider::ALL
            .into_iter()
            .map(agq_providers::Credential::Key)
            .chain([agq_providers::Credential::ClaudeSubscription])
            .filter_map(|c| agq_providers::runtime_credential(c).ok().flatten())
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
        let repository = self.agentique_repository().ok_or_else(|| {
            format!(
                "this Agentique does not know where its repository is: {}",
                crate::develop::UNKNOWN_REPOSITORY
            )
        })?;
        let store = self.objective_store();
        if let Some(active) = store.active() {
            return Err(format!(
                "the objective “{}” is not finished: continue or stop it first",
                active.intent
            ));
        }
        let setup = self.objective_setup(&repository)?;
        // Each role's model, resolved now from the credentials read now and
        // recorded (C-54): nothing it needs is left unresolved. An objective
        // does not explore yet (W12.5), so the explorer, escalation and
        // typed decisions are recorded but not needed. Whether this
        // computer has a Claude login is said when it is known.
        self.read_credentials_now();
        let resolved = self
            .agent_models(false)
            .map_err(|problems| problems.join("; "))?;
        let mut objective = store.create(intent, &repository, "main", budgets, permissions)?;
        objective.models = resolved.models;
        objective.roles_unavailable = resolved.unavailable.into_iter().collect();
        store.save(&objective)?;
        // Its thread starts with the intent and each role's model, written
        // by the Orchestrator as it starts.
        self.objectives.thread.clear();
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
        let mut objective = store.active().ok_or("there is no objective to continue")?;
        let setup = self.objective_setup(&objective.repository)?;
        // The models it started with stay its models (C-54); a record
        // without them (an earlier build saved it) gets them now, from the
        // current Settings, and the activity says so.
        let resolved_now = objective.models.is_empty();
        if resolved_now {
            self.read_credentials_now();
            let resolved = self
                .agent_models(false)
                .map_err(|problems| problems.join("; "))?;
            objective.models = resolved.models;
            objective.roles_unavailable = resolved.unavailable.into_iter().collect();
            store.save(&objective)?;
        }
        // A record an earlier build saved has no thread yet: it starts with
        // the intent, as the Operator gave it.
        if store.thread_last(&objective.id) == 0
            && let Err(error) = store.append_thread(
                &objective.id,
                ThreadEntry::new(Kind::Human, Author::Operator, objective.intent.clone()),
            )
        {
            self.objectives.message = Some(format!("The objective's thread: {error}"));
        }
        self.objectives.read(&store, &objective.id);
        self.objectives.current = Some(objective.clone());
        if resolved_now {
            self.objective_note(
                Author::Agentique,
                "Its record had no models (an earlier build saved it): they were resolved now from the current Settings",
            );
            for model in &objective.models {
                self.objective_note(
                    Author::Agentique,
                    format!("The {} runs on {}", model.role, model.label()),
                );
            }
        }
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
            objective.settle_running(
                agq_orchestrator::record::DirectiveStatus::Stopped,
                "stopped by the Operator",
            );
            let _ = store.save(&objective);
            self.objectives.current = Some(objective);
            self.objective_note(Author::Operator, "Stopped");
            self.mark(Dirty::LAYOUT);
        }
    }

    /// Adds an event to the shown objective's thread and shows it: the
    /// Studio's own part (the Operator's stop of an objective not running,
    /// an adoption refused, the project read again).
    fn objective_note(&mut self, author: Author, text: impl Into<String>) {
        let Some(id) = self.objectives.current.as_ref().map(|o| o.id.clone()) else {
            return;
        };
        let entry = ThreadEntry::new(Kind::Event, author, text);
        let shown = self
            .objective_store()
            .append_thread(&id, entry.clone())
            .unwrap_or_else(|error| ThreadEntry {
                objective: id,
                at: agq_launcher::now(),
                text: format!("{} (not kept in the thread: {error})", entry.text),
                ..entry
            });
        self.objectives.add(shown);
    }

    /// The Operator's command to the running objective.
    pub fn objective_command(&mut self, command: Command) {
        if self.refused_to_agents("steering an objective") {
            return;
        }
        // The Orchestrator puts it in the thread as it takes it.
        if let Some(handle) = &self.objectives.handle {
            handle.send(command);
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
                self.objectives.read(&store, &active.id);
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
                Event::Thread(entry) => self.objectives.add(entry),
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
                        self.objective_note(
                            Author::Agentique,
                            "The project was read again after the merge",
                        );
                    }
                }
                Event::Adopt { build, reply } => {
                    let result = self.use_build(&build);
                    if let Err(problem) = &result {
                        self.objective_note(Author::Agentique, format!("Not adopted: {problem}"));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seq: u64, text: &str) -> ThreadEntry {
        ThreadEntry {
            seq,
            ..ThreadEntry::event(text)
        }
    }

    /// The thread as shown: in order whatever order entries arrive in (the
    /// Orchestrator's thread and the Studio's own), each once, one that
    /// could not be kept (number 0) where it arrived, and at most the latest
    /// [`SHOWN`].
    #[test]
    fn the_thread_shown_is_in_order_once_and_bounded() {
        let mut state = ObjectivesState::default();
        for seq in [2, 1, 4, 3, 3, 2] {
            state.add(entry(seq, &seq.to_string()));
        }
        state.add(entry(0, "not kept"));
        state.add(entry(5, "5"));
        let shown: Vec<(u64, String)> = state
            .thread
            .iter()
            .map(|e| (e.seq, e.text.clone()))
            .collect();
        assert_eq!(
            shown,
            vec![
                (1, "1".to_string()),
                (2, "2".into()),
                (3, "3".into()),
                (4, "4".into()),
                (0, "not kept".into()),
                (5, "5".into()),
            ]
        );
        for seq in 6..(SHOWN as u64 + 50) {
            state.add(entry(seq, "x"));
        }
        assert_eq!(state.thread.len(), SHOWN);
        assert_eq!(
            state.thread.last().map(|e| e.seq),
            Some(SHOWN as u64 + 49),
            "the latest are kept"
        );
        assert!(state.thread.iter().all(|e| e.seq > 5 || e.seq == 0));
    }
}
