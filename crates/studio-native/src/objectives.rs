//! Objectives in the Studio (C-53, C-54, ROADMAP §4.16): the Operator
//! starts an objective from the Conversation or the Objectives panel with
//! one start form (its intent, whether it explores, budgets, permissions
//! and each role's model, shown before Start), or the Assistant proposes
//! one and the Operator starts it; the Orchestrator (`agq-orchestrator`)
//! takes it through its cycles on its own thread. The Studio shows its
//! record, its child objectives and their threads (read from the
//! objectives' records once, then followed as entries arrive), carries its
//! requests (an observation of this Studio for the lead, the handover to an
//! adopted build), and passes on the Operator's Pause, Step, Resume, Stop
//! and messages: the same application commands from the Conversation, the
//! panel and the palette. An objective the adopted build is to continue
//! starts again by itself; one interrupted when Agentique closed waits for
//! Continue. Before an objective starts, the Studio resolves each role's
//! model from Settings and the credentials and records it in the
//! objective; the Orchestrator builds every session of a role on that
//! model, effort and credential, and never moves a role to another.

use crate::studio::{Dirty, Studio};
use agq_assistant::claude_agent::{self, Installation};
use agq_orchestrator::decide::{Inferred, Shape};
use agq_orchestrator::record::{
    Access, Budgets, Cost, DirectiveStatus, Objective, Permissions, Resuming, State, Store,
};
use agq_orchestrator::run::{self, Command, Event, Handle, RuntimeFactory, Setup};
use agq_orchestrator::thread::{self, Author, Kind, ThreadEntry};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// The latest entries of each shown objective's thread kept in memory (the
/// thread itself is in its records).
const SHOWN: usize = 2000;

/// The longest a directive's text takes to stream in, whatever its length.
const STREAM_AT_MOST: Duration = Duration::from_secs(6);

/// What the Operator starts: the start form's fields (C-54, as the
/// Operator amended it): the intent, and what it does as read from the
/// intent and possibly changed.
#[derive(Clone, Debug, PartialEq)]
pub struct StartRequest {
    pub intent: String,
    /// Its cycles explore the running application first.
    pub explore: bool,
    pub budgets: Budgets,
    pub permissions: Permissions,
    /// What the intent was read as before Start, and by whom.
    pub inferred: Option<Inferred>,
}

impl StartRequest {
    /// An objective that does what `shape` says: its improvements, no
    /// spend or time limit, and the permissions to merge and adopt.
    pub fn new(intent: &str, shape: Shape, inferred: Option<Inferred>) -> StartRequest {
        StartRequest {
            intent: intent.trim().to_string(),
            explore: shape.explore,
            budgets: Budgets {
                cycles: shape.cycles,
                ..Budgets::default()
            },
            permissions: Permissions {
                push: shape.merge,
                merge: shape.merge,
                adopt: shape.merge && shape.adopt,
                ..Permissions::default()
            },
            inferred,
        }
    }
}

/// What the start form shows when it opens: the composer's message, or
/// what the Assistant proposed (`propose_objective`).
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub intent: String,
    /// The Assistant proposed it.
    pub by_assistant: bool,
}

impl Proposal {
    pub fn new(intent: &str) -> Proposal {
        Proposal {
            intent: intent.trim().to_string(),
            by_assistant: false,
        }
    }

    /// What the Assistant proposed.
    pub fn from_assistant(proposed: &agq_assistant::tools::ObjectiveProposal) -> Proposal {
        Proposal {
            by_assistant: true,
            ..Proposal::new(&proposed.intent)
        }
    }
}

/// An intent being read, or read: what the objective does (the Operator's
/// amendment of C-54). Jev answers off the window's thread.
pub struct Reading {
    pub intent: String,
    /// `None` while it is read.
    pub inferred: Option<Inferred>,
    receiver: Option<Receiver<Inferred>>,
    /// Set when another reading replaces this one: its model call stops.
    replaced: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for Reading {
    fn drop(&mut self) {
        self.replaced
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// What an objective does when its intent could not be read, and why.
fn defaults(why: String) -> Inferred {
    Inferred {
        shape: Shape::DEFAULT,
        source: agq_orchestrator::decide::Source::Rules,
        confidence: None,
        millis: 0,
        usd: Some(0.0),
        note: why,
    }
}

/// The one start form (C-54): open in the Conversation (from the composer
/// or the Assistant's proposal) or else in the Objectives panel, and what
/// it was last given to show.
#[derive(Default)]
pub struct StartForm {
    pub in_conversation: bool,
    pub proposal: Option<Proposal>,
    /// Counts the proposals given, so the form takes each once.
    pub given: u64,
    /// The intent last read, and what it was read as.
    pub reading: Option<Reading>,
}

#[derive(Default)]
pub struct ObjectivesState {
    pub handle: Option<Handle>,
    /// The objective shown: the running one, or the newest one the
    /// Operator started (never a child).
    pub current: Option<Objective>,
    /// Its child objectives and theirs, as their records say.
    pub children: Vec<Objective>,
    /// The latest entries of the shown objectives' threads, by objective,
    /// each in order.
    pub threads: BTreeMap<String, Vec<ThreadEntry>>,
    /// Directives that arrived while shown, streaming in: when each
    /// arrived and how many of its characters show.
    pub streaming: HashMap<(String, u64), (Instant, usize)>,
    /// Counts changes to what is shown, so views rebuild only then.
    pub version: u64,
    /// Counts steps of the directives streaming in: a view updates only
    /// their rows.
    pub stream_version: u64,
    /// The objectives run in this Studio since it started (started or
    /// continued here): the Conversation shows their threads once they end
    /// too.
    pub ran: HashSet<String>,
    /// The configured keys, for redacting the Studio's own entries: read on
    /// a thread of their own, never per entry on the window's thread.
    keys: Option<Vec<String>>,
    reading_keys: Option<Receiver<Vec<String>>>,
    /// Why the last start or command did not happen.
    pub message: Option<String>,
    pub form: StartForm,
    /// Looked at start for an objective to show or continue.
    looked: bool,
}

impl ObjectivesState {
    pub fn running(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| !h.finished())
    }

    /// Whether the Operator paused the running objective (as soon as they
    /// did, also while an agent's session holds at its next tool call).
    pub fn paused(&self) -> bool {
        self.running() && self.handle.as_ref().is_some_and(|h| h.paused())
    }

    /// Not finished and not running: interrupted, waiting for Continue.
    pub fn unfinished(&self) -> bool {
        !self.running() && self.current.as_ref().is_some_and(Objective::active)
    }

    /// Whether a message to the objective shown can reach an agent: it runs,
    /// or waits to continue (a finished one has no lead left to read it).
    pub fn takes_messages(&self) -> bool {
        self.running() || self.unfinished()
    }

    /// An entry of a shown thread, by its objective and number.
    pub fn entry(&self, objective: &str, seq: u64) -> Option<&ThreadEntry> {
        self.threads
            .get(objective)?
            .iter()
            .rev()
            .find(|e| e.seq == seq)
    }

    /// The configured keys as last read: read again on a thread of their
    /// own when they may have changed ([`ObjectivesState::read_keys`]);
    /// read here, once, only before the first reading arrives.
    fn keys(&mut self) -> Vec<String> {
        if let Some(read) = self.reading_keys.as_ref().and_then(|r| r.try_recv().ok()) {
            self.keys = Some(read);
            self.reading_keys = None;
        }
        self.keys.get_or_insert_with(configured_keys).clone()
    }

    /// Reads the configured keys again, on a thread of its own (a key or
    /// the token was saved or removed, or an objective is shown).
    pub fn read_keys(&mut self) {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(configured_keys());
        });
        self.reading_keys = Some(receiver);
    }

    /// Whether the workspace's tick should poll.
    pub fn wants_poll(&self) -> bool {
        self.handle.is_some()
            || !self.looked
            || !self.streaming.is_empty()
            || self
                .form
                .reading
                .as_ref()
                .is_some_and(|r| r.receiver.is_some())
    }

    /// The shown objective or one of its children, by id.
    pub fn objective(&self, id: &str) -> Option<&Objective> {
        self.current
            .iter()
            .chain(&self.children)
            .find(|o| o.id == id)
    }

    /// Shows `entry` in its objective's thread, in its place (entries can
    /// arrive out of order from the Orchestrator's thread and the Studio's
    /// own), each once; an entry that could not be kept (number 0) stays
    /// where it arrived, after what was shown then. A directive that
    /// arrives `live` streams in.
    pub fn add(&mut self, entry: ThreadEntry, live: bool) {
        let thread = self.threads.entry(entry.objective.clone()).or_default();
        if entry.seq != 0 && thread.iter().any(|e| e.seq == entry.seq) {
            return;
        }
        if live && entry.kind == Kind::Directive && entry.seq != 0 {
            self.streaming
                .insert((entry.objective.clone(), entry.seq), (Instant::now(), 0));
        }
        let at = if entry.seq == 0 {
            thread.len()
        } else {
            let mut at = thread
                .iter()
                .rposition(|e| e.seq != 0 && e.seq < entry.seq)
                .map_or(0, |i| i + 1);
            while thread.get(at).is_some_and(|e| e.seq == 0) {
                at += 1;
            }
            at
        };
        thread.insert(at, entry);
        if thread.len() > SHOWN {
            thread.drain(..thread.len() - SHOWN);
        }
        self.version += 1;
    }

    /// The number of the last entry shown of objective `id`'s thread.
    pub fn last_shown(&self, id: &str) -> u64 {
        self.threads
            .get(id)
            .and_then(|t| t.iter().map(|e| e.seq).max())
            .unwrap_or(0)
    }

    /// How many characters of `entry`'s text show now: all (`None`), unless
    /// it is streaming in.
    pub fn shown_chars(&self, entry: &ThreadEntry) -> Option<usize> {
        self.streaming
            .get(&(entry.objective.clone(), entry.seq))
            .map(|(_, shown)| *shown)
    }

    /// Moves the directives streaming in on to `now`, at `per_second`
    /// characters a second (none: at once), each within
    /// [`STREAM_AT_MOST`]. Returns whether anything shows more.
    pub fn stream(&mut self, now: Instant, per_second: Option<f64>) -> bool {
        if self.streaming.is_empty() {
            return false;
        }
        let threads = &self.threads;
        let length = |(objective, seq): &(String, u64)| {
            threads
                .get(objective)
                .and_then(|t| t.iter().find(|e| e.seq == *seq))
                .map_or(0, |e| e.text.chars().count())
        };
        let mut changed = false;
        self.streaming.retain(|key, (arrived, shown)| {
            let length = length(key);
            let now_shown = streamed(length, now.duration_since(*arrived), per_second);
            if now_shown != *shown {
                *shown = now_shown;
                changed = true;
            }
            now_shown < length
        });
        // Only the rows streaming change: views update just them.
        if changed {
            self.stream_version += 1;
        }
        changed
    }

    /// Shows `objective` (one the Operator started) and reads its thread
    /// and its children's from their records (`all`, the records read).
    fn show(&mut self, store: &Store, objective: Objective, all: &[Objective]) {
        self.threads.clear();
        self.streaming.clear();
        self.children = descendants(all, &objective.id);
        let ids: Vec<String> = std::iter::once(objective.id.clone())
            .chain(self.children.iter().map(|c| c.id.clone()))
            .collect();
        for id in ids {
            self.read_tail(store, &id);
        }
        self.current = Some(objective);
        self.version += 1;
    }

    /// Reads the latest [`SHOWN`] entries of objective `id`'s thread from
    /// its records, from the end; later ones arrive as events.
    fn read_tail(&mut self, store: &Store, id: &str) {
        let since = store.thread_last(id).saturating_sub(SHOWN as u64);
        for entry in store.thread(id, since) {
            self.add(entry, false);
        }
    }

    /// Takes a record the Orchestrator changed: the objective shown, or a
    /// child of it (a new child's thread so far is read from its records).
    fn changed(&mut self, store: &Store, objective: Objective) {
        if self.current.as_ref().is_some_and(|c| c.id == objective.id) {
            self.current = Some(objective);
        } else if let Some(known) = self.children.iter_mut().find(|c| c.id == objective.id) {
            *known = objective;
        } else if objective
            .parent
            .as_deref()
            .is_some_and(|p| self.objective(p).is_some())
        {
            let id = objective.id.clone();
            self.children.push(objective);
            self.read_tail(store, &id);
        }
        self.version += 1;
    }
}

/// Where an objective stands, in a word or two (`running` when its run is
/// going on in this Studio).
pub fn state_word(objective: &Objective, running: bool) -> &'static str {
    match objective.state {
        State::Running if running => "running",
        State::Running => "interrupted",
        State::Paused if running => "paused",
        State::Paused => "paused, not running",
        State::Stopped => "stopped",
        State::Done => "done",
        State::Failed => "ended without finishing",
    }
}

/// How many of `length` characters show `elapsed` after a directive
/// arrived, at `per_second` (none: all at once), all within
/// [`STREAM_AT_MOST`].
pub fn streamed(length: usize, elapsed: Duration, per_second: Option<f64>) -> usize {
    let Some(rate) = per_second else {
        return length;
    };
    let rate = rate.max(length as f64 / STREAM_AT_MOST.as_secs_f64());
    ((elapsed.as_secs_f64() * rate) as usize).min(length)
}

/// The characters a second a directive streams in at the Operator's
/// observer speed (C-54): at `observe` about as fast as one reads, at
/// `fast` quickly, at `instant` at once.
fn stream_rate(speed: crate::control::Speed) -> Option<f64> {
    use crate::control::Speed;
    match speed {
        Speed::Instant => None,
        Speed::Fast => Some(240.0),
        Speed::Observe => Some(30.0),
    }
}

/// The child objectives of `root` in `all`, and theirs, in the order they
/// were created.
fn descendants(all: &[Objective], root: &str) -> Vec<Objective> {
    let mut found: Vec<Objective> = Vec::new();
    let mut parents = vec![root.to_string()];
    while let Some(parent) = parents.pop() {
        for child in all
            .iter()
            .filter(|o| o.parent.as_deref() == Some(parent.as_str()))
        {
            if child.id != root && !found.iter().any(|f| f.id == child.id) {
                parents.push(child.id.clone());
                found.push(child.clone());
            }
        }
    }
    found.sort_by(|a, b| a.created.cmp(&b.created));
    found
}

/// The configured keys and the Claude subscription token: never in a
/// change, and never in a thread ([`thread::redacted`]).
fn configured_keys() -> Vec<String> {
    agq_providers::Provider::ALL
        .into_iter()
        .map(agq_providers::Credential::Key)
        .chain([agq_providers::Credential::ClaudeSubscription])
        .filter_map(|c| agq_providers::runtime_credential(c).ok().flatten())
        .map(|k| k.expose().to_string())
        .filter(|k| k.chars().count() >= thread::KEY_CHARS)
        .collect()
}

/// The Studio's own `entry` (an Operator's message, a note quoting an
/// error) with every configured key in it replaced by a hint: keys never
/// reach a thread.
fn redacted(mut entry: ThreadEntry, keys: &[String]) -> ThreadEntry {
    entry.text = thread::redacted(&entry.text, keys);
    entry.details = entry.details.map(|d| thread::redacted(&d, keys));
    entry
}

/// The objective the Operator started that is not finished, if any (its
/// children are not finished then either, and are not it).
fn active_root(store: &Store) -> Option<Objective> {
    store
        .list()
        .into_iter()
        .find(|o| o.active() && o.parent.is_none())
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
        // Read now, at the Operator's Start or Continue.
        let keys = configured_keys();
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
            // Exploration's test instances run at the Operator's speed, so
            // they can watch (C-54).
            speed: format!("{:?}", self.control_speed()).to_lowercase(),
            // A merged build that explores may get the explorer's key (C-54).
            credential: Box::new(|credential| {
                agq_providers::runtime_credential(credential).ok().flatten()
            }),
            studios: Box::new(run::Live),
        })
    }

    /// Starts an objective on Agentique's own repository as the start form
    /// asks (C-54): its budgets are checked and each role's model resolved
    /// (an objective that explores also needs the explorer, escalation and
    /// typed decisions) and recorded before anything starts.
    pub fn start_objective(&mut self, request: StartRequest) -> Result<(), String> {
        if self.refused_to_agents("starting an objective") {
            return Err("starting an objective is the Operator's own".into());
        }
        self.runs_objectives()?;
        if self.objectives.running() {
            return Err("an objective is running: stop it first".into());
        }
        if request.intent.trim().is_empty() {
            return Err("say what to improve".into());
        }
        request
            .budgets
            .check()
            .map_err(|problems| problems.join("; "))?;
        let repository = self.agentique_repository().ok_or_else(|| {
            format!(
                "this Agentique does not know where its repository is: {}",
                crate::develop::UNKNOWN_REPOSITORY
            )
        })?;
        let store = self.objective_store();
        if let Some(active) = active_root(&store) {
            return Err(format!(
                "the objective “{}” is not finished: continue or stop it first",
                active.intent
            ));
        }
        let setup = self.objective_setup(&repository)?;
        // Each role's model, resolved now from the credentials read now and
        // recorded (C-54): nothing it needs is left unresolved. Whether this
        // computer has a Claude login is said when it is known.
        self.read_credentials_now();
        let resolved = self
            .agent_models(request.explore)
            .map_err(|problems| problems.join("; "))?;
        let mut objective = store.create(
            &request.intent,
            &repository,
            "main",
            request.budgets,
            request.permissions,
        )?;
        objective.explore = request.explore;
        // Reading the intent was a typed decision: the cost of the reading
        // used is the objective's, under the decisions role, by the model
        // that answered (§4.16); a reading that asked nothing costs nothing.
        if let Some(inferred) = &request.inferred
            && inferred.usd != Some(0.0)
        {
            let model = match inferred.source {
                agq_orchestrator::decide::Source::Jev => "decisions",
                _ => "escalation",
            };
            if let Some(role) = resolved.models.iter().find(|m| m.role == model) {
                objective.spent.add(
                    "decisions",
                    &role.model,
                    Cost {
                        usd: inferred.usd.unwrap_or(0.0),
                        tokens: 0,
                        unknown: inferred.usd.is_none(),
                    },
                );
            }
        }
        objective.inferred = request.inferred;
        objective.models = resolved.models;
        objective.roles_unavailable = resolved.unavailable.into_iter().collect();
        store.save(&objective)?;
        // The open conversation shows its thread (C-54).
        if self.project.is_some() {
            self.conversation.started.push(objective.id.clone());
            self.save_conversation();
        }
        // Its thread starts with the intent and each role's model, written
        // by the Orchestrator as it starts.
        let state = &mut self.objectives;
        state.ran.insert(objective.id.clone());
        state.threads.clear();
        state.streaming.clear();
        state.children.clear();
        state.current = Some(objective.clone());
        state.version += 1;
        state.handle = Some(run::start(setup, objective));
        state.message = None;
        state.form.in_conversation = false;
        self.mark(Dirty::LAYOUT | Dirty::STATUS | Dirty::CONVERSATION);
        Ok(())
    }

    /// Opens the start form in the Conversation with `proposal` (C-54):
    /// nothing starts until the Operator presses Start.
    pub fn open_start_form(&mut self, proposal: Proposal) -> Result<(), String> {
        // The reason goes to the Assistant's model too: it names no
        // objective.
        if self.objectives.running() || self.objectives.unfinished() {
            return Err(
                "an objective is not finished, and one runs at a time; the Operator sees it in the Objectives panel"
                    .into(),
            );
        }
        let form = &mut self.objectives.form;
        form.proposal = Some(proposal);
        form.given += 1;
        form.in_conversation = true;
        self.objectives.message = None;
        self.conversation.shown = true;
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
        Ok(())
    }

    /// "Start as objective" (C-54): the message being written becomes the
    /// intent of the start form shown in the Conversation; the message
    /// stays in the composer until the objective starts.
    pub fn start_form_from_message(&mut self) {
        if self.refused_to_agents("starting an objective") {
            return;
        }
        let intent = self.conversation.input.clone();
        if let Err(problem) = self.open_start_form(Proposal::new(&intent)) {
            self.status = format!("Not shown: {problem}");
        }
    }

    /// Reads `intent` for what the objective does (the Operator's
    /// amendment of C-54): Jev on the `decisions` role's model, escalating
    /// to the `escalation` role's, off the window's thread; the form shows
    /// it when it arrives (`poll_objective`). Without those roles' models,
    /// the defaults, with why.
    pub fn read_intent(&mut self, intent: &str) {
        let intent = intent.trim().to_string();
        // Read once per intent: the same text, read or being read, is not
        // asked again (typing a character and taking it back).
        if self
            .objectives
            .form
            .reading
            .as_ref()
            .is_some_and(|r| r.intent == intent)
        {
            return;
        }
        // Jev on the decisions role's model and the escalation role's,
        // as Settings and the credentials resolve them; without either,
        // the defaults, with why.
        let mut models = Vec::new();
        let mut missing = Vec::new();
        for (role, model) in self.agent_models_each() {
            match model {
                Ok(model) => models.push(model),
                Err(why) if matches!(role, "decisions" | "escalation") => {
                    missing.push(format!("the {role} role: {why}"))
                }
                Err(_) => {}
            }
        }
        let decider = if missing.is_empty() {
            agq_orchestrator::models::decider(&models)
        } else {
            Err(missing.join("; "))
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let replaced = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = replaced.clone();
        let read = intent.clone();
        std::thread::spawn(move || {
            let inferred = match decider {
                Ok(decider) => decider.infer(&read, &mut || {
                    stop.load(std::sync::atomic::Ordering::SeqCst)
                }),
                Err(why) => defaults(why),
            };
            let _ = sender.send(inferred);
        });
        // The reading it replaces, if any, is dropped and so stopped.
        self.objectives.form.reading = Some(Reading {
            intent,
            inferred: None,
            receiver: Some(receiver),
            replaced,
        });
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
    }

    /// Puts the start form back in the Objectives panel.
    pub fn close_start_form(&mut self) {
        self.objectives.form.in_conversation = false;
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
    }

    /// Continues the objective that is not finished (interrupted, or handed
    /// over to this build).
    pub fn continue_objective(&mut self) -> Result<(), String> {
        if self.refused_to_agents("continuing an objective") {
            return Err("continuing an objective is the Operator's own".into());
        }
        self.runs_objectives()?;
        if self.objectives.running() {
            return Ok(());
        }
        let store = self.objective_store();
        let mut objective = active_root(&store).ok_or("there is no objective to continue")?;
        // A record whose budgets no start form would accept (a role that
        // may make no model call, say) does not run.
        objective
            .budgets
            .check()
            .map_err(|problems| format!("its budgets cannot run: {}", problems.join("; ")))?;
        let setup = self.objective_setup(&objective.repository)?;
        // The models it started with stay its models (C-54); a record
        // without them (an earlier build saved it) gets them now, from the
        // current Settings, and the activity says so.
        let resolved_now = objective.models.is_empty();
        if resolved_now {
            self.read_credentials_now();
            let resolved = self
                .agent_models(objective.explore)
                .map_err(|problems| problems.join("; "))?;
            objective.models = resolved.models;
            objective.roles_unavailable = resolved.unavailable.into_iter().collect();
            store.save(&objective)?;
        }
        // A record an earlier build saved has no thread yet: it starts with
        // the intent, as the Operator gave it.
        let keys = self.objectives.keys();
        if store.thread_last(&objective.id) == 0
            && let Err(error) = store.append_thread(
                &objective.id,
                redacted(
                    ThreadEntry::new(Kind::Human, Author::Operator, objective.intent.clone()),
                    &keys,
                ),
            )
        {
            self.objectives.message = Some(format!("The objective's thread: {error}"));
        }
        // Shown already when it was found at start: its thread is not read
        // again, only its record taken.
        if self
            .objectives
            .current
            .as_ref()
            .is_some_and(|c| c.id == objective.id)
            && store.thread_last(&objective.id) <= self.objectives.last_shown(&objective.id)
        {
            self.objectives.current = Some(objective.clone());
        } else {
            let all = store.list();
            self.objectives.show(&store, objective.clone(), &all);
        }
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
        self.objectives.ran.insert(objective.id.clone());
        self.objectives.handle = Some(run::start(setup, objective));
        self.objectives.message = None;
        self.mark(Dirty::LAYOUT | Dirty::STATUS | Dirty::CONVERSATION);
        Ok(())
    }

    /// Whether this Studio may run objectives: a test instance never does
    /// (C-54). Its objectives are recorded ones, there to be observed and
    /// operated; running one would start an Orchestrator with its record's
    /// permissions inside an instance that is itself under test.
    fn runs_objectives(&self) -> Result<(), String> {
        if self.args.test_instance {
            return Err(
                "a test instance runs no objective: it shows recorded ones to observe and operate"
                    .into(),
            );
        }
        Ok(())
    }

    /// Stops an objective that is not running (one interrupted earlier),
    /// and its children.
    pub fn stop_idle_objective(&mut self) {
        if self.refused_to_agents("stopping an objective") {
            return;
        }
        let store = self.objective_store();
        let Some(mut objective) = active_root(&store) else {
            return;
        };
        let all = store.list();
        for mut child in descendants(&all, &objective.id)
            .into_iter()
            .filter(Objective::active)
        {
            child.state = State::Stopped;
            child.note = Some("Stopped with its parent by the Operator.".into());
            child.settle_running(DirectiveStatus::Stopped, "stopped by the Operator");
            let _ = store.save(&child);
        }
        objective.state = State::Stopped;
        objective.note = Some("Stopped by the Operator.".into());
        objective.settle_running(DirectiveStatus::Stopped, "stopped by the Operator");
        let _ = store.save(&objective);
        let all = store.list();
        self.objectives.show(&store, objective, &all);
        self.objective_note(Author::Operator, "Stopped");
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
    }

    /// Adds an event to the shown objective's thread and shows it: the
    /// Studio's own part (the Operator's stop of an objective not running,
    /// an adoption refused, the project read again).
    fn objective_note(&mut self, author: Author, text: impl Into<String>) {
        let Some(id) = self.objectives.current.as_ref().map(|o| o.id.clone()) else {
            return;
        };
        self.objective_entry(&id, ThreadEntry::new(Kind::Event, author, text));
    }

    /// Adds `entry` to objective `id`'s thread and shows it, saying so when
    /// it could not be kept.
    fn objective_entry(&mut self, id: &str, entry: ThreadEntry) {
        let keys = self.objectives.keys();
        let entry = redacted(entry, &keys);
        let shown = self
            .objective_store()
            .append_thread(id, entry.clone())
            .unwrap_or_else(|error| ThreadEntry {
                objective: id.to_string(),
                at: agq_launcher::now(),
                text: format!("{} (not kept in the thread: {error})", entry.text),
                ..entry
            });
        self.objectives.add(shown, false);
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
    }

    /// The Operator's command to the running objective.
    pub fn objective_command(&mut self, command: Command) {
        if self.refused_to_agents("steering an objective") {
            return;
        }
        // The Orchestrator puts it in the thread as it takes it.
        if let Some(handle) = &self.objectives.handle {
            handle.send(command);
            self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
        }
    }

    /// The Operator's message to objective `id` (C-54): a reply in its
    /// thread in the Conversation or the panel's message field, one path.
    /// A running objective takes it and records where it went (the
    /// implementer at its next tool call while it works, otherwise the
    /// lead's next turn); one that is not running keeps it in its thread
    /// for the lead's next turn when it continues. Messages go to the
    /// objective the Operator started; its children are steered through it.
    pub fn message_objective(&mut self, id: &str, text: &str) -> Result<(), String> {
        if self.refused_in_operators_window("messaging an objective") {
            return Err(
                "messaging an objective in the Operator's window is the Operator's own".into(),
            );
        }
        let text = text.trim();
        if text.is_empty() {
            return Err("write a message first".into());
        }
        if self.objectives.current.as_ref().is_none_or(|o| o.id != id) {
            return Err(
                "a message goes to the objective you started; its children are steered through it"
                    .into(),
            );
        }
        // Kept for no one: a finished objective's lead takes no more turns.
        if !self.objectives.takes_messages() {
            return Err(
                "the objective has ended, so no agent would read it; start a new objective".into(),
            );
        }
        if self.objectives.running() {
            if let Some(handle) = &self.objectives.handle {
                handle.send(Command::Message(text.to_string()));
            }
        } else {
            self.objective_entry(id, ThreadEntry::message(text, "lead"));
        }
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
        Ok(())
    }

    /// Stops child objective `id` alone (C-54): a running objective's run
    /// stops it (`Command::StopChild`), its directive ends as stopped and
    /// the lead goes on with that; one that is not running has the child's
    /// record stopped here, and its directive settled, for the lead's next
    /// turn.
    pub fn stop_child(&mut self, id: &str) {
        if self.refused_to_agents("stopping an objective") {
            return;
        }
        let Some(child) = self
            .objectives
            .children
            .iter()
            .find(|c| c.id == id)
            .cloned()
        else {
            return;
        };
        if !child.active() {
            return;
        }
        if self.objectives.running() {
            self.objective_command(Command::StopChild(id.to_string()));
            return;
        }
        let store = self.objective_store();
        let all = store.list();
        let mut stopping = vec![child.clone()];
        stopping.extend(descendants(&all, id));
        for mut objective in stopping.into_iter().filter(Objective::active) {
            objective.state = State::Stopped;
            objective.note = Some("Stopped by the Operator.".into());
            objective.settle_running(DirectiveStatus::Stopped, "stopped by the Operator");
            let _ = store.save(&objective);
            self.objective_entry(
                &objective.id,
                ThreadEntry::new(Kind::Event, Author::Operator, "Stopped"),
            );
        }
        // Its parent's directive to it, and the lead's handoff.
        if let Some(parent_id) = child.parent.clone()
            && let Ok(mut parent) = store.load(&parent_id)
        {
            let directive = parent
                .directives
                .iter()
                .find(|d| d.recipient == agq_orchestrator::record::Recipient::Child(id.to_string()))
                .map(|d| d.id.clone());
            if let Some(directive) = &directive {
                parent.settle(
                    directive,
                    DirectiveStatus::Stopped,
                    Some("stopped by the Operator".into()),
                );
                let _ = store.save(&parent);
            }
            let mut handoff = ThreadEntry::new(
                Kind::Result,
                Author::Agentique,
                format!("The child objective {id} was stopped by the Operator"),
            )
            .with_details("stopped by the Operator")
            .for_directive(directive.as_deref());
            handoff.to = Some("lead".into());
            self.objective_entry(&parent_id, handoff);
        }
        let all = store.list();
        if let Some(root) = self
            .objectives
            .current
            .clone()
            .and_then(|r| store.load(&r.id).ok())
        {
            self.objectives.show(&store, root, &all);
        }
        self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
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
        // Marked interrupted now, before Agentique exits, whatever the run
        // saved by then (C-54): the next start waits for the Operator's
        // Continue. An adoption's continuation still goes on by itself.
        if let Some(id) = self.objectives.current.as_ref().map(|o| o.id.clone())
            && let Err(error) = self.objective_store().mark_interrupted(&id)
        {
            eprintln!("The objective could not be marked interrupted: {error}");
        }
    }

    /// Takes the objective's events and streams its directives in; at the
    /// first call, shows the newest objective the Operator started (finished
    /// or not) and continues one handed over to this build. Returns whether
    /// anything changed.
    pub fn poll_objective(&mut self) -> bool {
        let mut changed = false;
        if let Some(reading) = &mut self.objectives.form.reading
            && let Some(receiver) = &reading.receiver
        {
            // A reading that ended without an answer (its thread failed)
            // gives the defaults, never a form that waits forever.
            let arrived = match receiver.try_recv() {
                Ok(inferred) => Some(inferred),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(defaults("the reading ended without an answer".into()))
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
            };
            if let Some(inferred) = arrived {
                reading.inferred = Some(inferred);
                reading.receiver = None;
                self.mark(Dirty::LAYOUT | Dirty::CONVERSATION);
                changed = true;
            }
        }
        if !self.objectives.looked {
            self.objectives.looked = true;
            let store = self.objective_store();
            let all = store.list();
            let roots: Vec<&Objective> = all.iter().filter(|o| o.parent.is_none()).collect();
            let shown = roots
                .iter()
                .find(|o| o.active())
                .or(roots.first())
                .map(|o| (*o).clone());
            if let Some(mut objective) = shown {
                let active = objective.active();
                self.objectives.show(&store, objective.clone(), &all);
                self.objectives.read_keys();
                changed = true;
                if active && self.args.test_instance {
                    self.objectives.message = Some(
                        "A recorded objective: this test instance shows it and does not run it."
                            .into(),
                    );
                } else if active {
                    // It goes on by itself after an adoption, or when the
                    // launcher started the last known good build after one
                    // that did not start (`--recovered-from`; a plain start
                    // under the launcher, `--supervised`, is no recovery);
                    // it waits after the Operator closed Agentique, and
                    // otherwise; it stops after resuming twice without
                    // getting further (C-54).
                    let recovered = self.args.recovered_from.is_some();
                    match objective.on_start(recovered) {
                        Resuming::Continue(_) if self.safe_mode => {
                            self.objectives.message = Some(
                                "An objective is not finished: Continue goes on from where it was (not by itself in safe mode).".into(),
                            );
                        }
                        Resuming::Continue(why) => {
                            objective.resumes += 1;
                            let _ = store.save(&objective);
                            self.objectives.current = Some(objective);
                            self.objective_note(Author::Agentique, why);
                            if let Err(problem) = self.continue_objective() {
                                self.objectives.message =
                                    Some(format!("The objective could not continue: {problem}"));
                                self.objective_note(
                                    Author::Agentique,
                                    format!("It could not go on: {problem}"),
                                );
                            }
                        }
                        Resuming::Wait(why) => self.objectives.message = Some(why),
                        Resuming::Stop(why) => {
                            objective.state = State::Failed;
                            objective.note = Some(why.clone());
                            objective.settle_running(DirectiveStatus::Failed, &why);
                            let _ = store.save(&objective);
                            self.objectives.current = Some(objective);
                            self.objective_note(Author::Agentique, why.clone());
                            self.objectives.message = Some(why);
                        }
                    }
                }
            }
        }
        let mut events = Vec::new();
        let mut finished = false;
        if let Some(handle) = self.objectives.handle.as_ref() {
            while let Ok(event) = handle.events.try_recv() {
                events.push(event);
            }
            finished = handle.finished() && events.is_empty();
        }
        let store = self.objective_store();
        for event in events {
            changed = true;
            match event {
                Event::Changed(objective) => self.objectives.changed(&store, *objective),
                Event::Thread(entry) => self.objectives.add(entry, true),
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
            self.objectives.version += 1;
            changed = true;
        }
        if changed {
            self.mark(Dirty::LAYOUT | Dirty::STATUS | Dirty::CONVERSATION);
        }
        // Directives streaming in change the Conversation alone.
        let rate = stream_rate(self.control_speed());
        if self.objectives.stream(Instant::now(), rate) {
            self.mark(Dirty::CONVERSATION);
            changed = true;
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
            "Objective: cycle {} · {} · {}",
            cycle.map(|c| c.n).unwrap_or(0),
            cycle.map(|c| c.phase.label()).unwrap_or("starting"),
            objective.budgets.spent_text(objective.spent.usd)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(objective: &str, seq: u64, kind: Kind, text: &str) -> ThreadEntry {
        let mut entry = ThreadEntry::new(kind, Author::Agentique, text);
        entry.seq = seq;
        entry.objective = objective.into();
        entry.at = format!("2026-10-04T10:00:{seq:02}Z");
        entry
    }

    /// Entries take their place by number, whatever order they arrive in,
    /// each objective's apart; one arriving twice shows once.
    #[test]
    fn entries_take_their_place_in_their_objectives_thread() {
        let mut state = ObjectivesState::default();
        for (objective, seq) in [("o", 2), ("o", 1), ("c", 1), ("o", 3), ("o", 2)] {
            state.add(entry(objective, seq, Kind::Event, "x"), false);
        }
        let seqs = |state: &ObjectivesState, id: &str| {
            state.threads[id].iter().map(|e| e.seq).collect::<Vec<_>>()
        };
        assert_eq!(seqs(&state, "o"), vec![1, 2, 3]);
        assert_eq!(seqs(&state, "c"), vec![1]);
        // One that could not be kept stays where it arrived.
        state.add(entry("o", 0, Kind::Event, "not kept"), false);
        state.add(entry("o", 4, Kind::Event, "x"), false);
        assert_eq!(seqs(&state, "o"), vec![1, 2, 3, 0, 4]);
        // At most the latest SHOWN.
        for seq in 5..(SHOWN as u64 + 50) {
            state.add(entry("o", seq, Kind::Event, "x"), false);
        }
        assert_eq!(state.threads["o"].len(), SHOWN);
        assert_eq!(state.last_shown("o"), SHOWN as u64 + 49);
        // The oldest went: 1, 2, 3, the one not kept, and 4 to 49.
        assert_eq!(state.threads["o"].first().map(|e| e.seq), Some(50));
        assert!(state.threads["o"].iter().all(|e| e.seq >= 50));
    }

    /// A directive that arrives while shown streams in at the observer
    /// speed, within six seconds; one read from the records shows whole.
    #[test]
    fn a_directive_streams_in_at_the_observer_speed() {
        let mut state = ObjectivesState::default();
        let directive = entry("o", 1, Kind::Directive, "Implement the label fix");
        state.add(directive.clone(), true);
        state.add(entry("o", 2, Kind::Directive, "Read earlier"), false);
        assert_eq!(state.shown_chars(&directive), Some(0));
        assert_eq!(state.shown_chars(&state.threads["o"][1].clone()), None);
        let (arrived, _) = state.streaming[&("o".to_string(), 1)];
        let version = state.version;
        assert!(state.stream(arrived + Duration::from_millis(300), Some(30.0)));
        // Only the streaming rows change: the rest is not worked out again.
        assert_eq!(state.version, version);
        assert_eq!(state.stream_version, 1);
        assert_eq!(state.shown_chars(&directive), Some(9));
        // Done: it shows whole and no longer streams.
        assert!(state.stream(arrived + Duration::from_secs(5), Some(30.0)));
        assert_eq!(state.shown_chars(&directive), None);
        assert!(!state.stream(arrived + Duration::from_secs(9), Some(30.0)));
        // At `instant` at once; a long one within six seconds.
        assert_eq!(streamed(23, Duration::ZERO, None), 23);
        assert_eq!(streamed(1200, Duration::from_secs(3), Some(30.0)), 600);
        assert_eq!(streamed(1200, Duration::from_secs(7), Some(30.0)), 1200);
    }

    /// The tree under an objective: its children and theirs, not others.
    #[test]
    fn the_tree_holds_children_and_grandchildren() {
        let dir = std::env::temp_dir().join(format!("agq-tree-{}", std::process::id()));
        let store = Store::new(&dir);
        let make = |id: &str, parent: Option<&str>, created: &str| {
            let mut o = store
                .create(
                    "x",
                    Path::new("C:/agentique"),
                    "main",
                    Budgets::default(),
                    Permissions::default(),
                )
                .unwrap();
            o.id = id.into();
            o.parent = parent.map(str::to_string);
            o.created = created.into();
            o
        };
        let all = vec![
            make("root", None, "1"),
            make("child-b", Some("root"), "3"),
            make("child-a", Some("root"), "2"),
            make("grandchild", Some("child-a"), "4"),
            make("other", None, "5"),
            make("other-child", Some("other"), "6"),
        ];
        let _ = std::fs::remove_dir_all(&dir);
        let ids: Vec<String> = descendants(&all, "root")
            .into_iter()
            .map(|o| o.id)
            .collect();
        assert_eq!(ids, vec!["child-a", "child-b", "grandchild"]);
    }

    /// An intent is read once (the same text is not asked again); a
    /// reading replaced by another is told to stop; without the roles that
    /// read it, the defaults, with why (the Operator's amendment of C-54).
    #[test]
    fn an_intent_is_read_once_and_without_its_models_gives_the_defaults() {
        use std::sync::atomic::Ordering;
        let (mut app, _folder) = crate::edit::app_tests::studio("read-intent");
        app.runtime.credentials.clear();
        app.read_intent(" Explore the History panel ");
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let inferred = loop {
            app.poll_objective();
            if let Some(inferred) = app
                .objectives
                .form
                .reading
                .as_ref()
                .unwrap()
                .inferred
                .clone()
            {
                break inferred;
            }
            assert!(std::time::Instant::now() < deadline, "never read");
            std::thread::sleep(Duration::from_millis(10));
        };
        let reading = app.objectives.form.reading.as_ref().unwrap();
        assert_eq!(reading.intent, "Explore the History panel");
        assert_eq!(inferred.shape, Shape::DEFAULT);
        assert_eq!(inferred.source, agq_orchestrator::decide::Source::Rules);
        assert!(
            inferred.note.contains("the decisions role"),
            "{}",
            inferred.note
        );
        assert!(inferred.by().ends_with("so the defaults apply."));
        // The same intent is not read again.
        app.read_intent("Explore the History panel");
        assert!(
            app.objectives
                .form
                .reading
                .as_ref()
                .unwrap()
                .inferred
                .is_some()
        );
        // Another replaces it, and the replaced reading is told to stop.
        let replaced = app
            .objectives
            .form
            .reading
            .as_ref()
            .unwrap()
            .replaced
            .clone();
        app.read_intent("Rename the Add button");
        assert!(replaced.load(Ordering::SeqCst));
        let now = app.objectives.form.reading.as_ref().unwrap();
        assert!(now.intent == "Rename the Add button" && now.inferred.is_none());
        assert!(app.objectives.wants_poll());
    }

    /// What the intent is read as becomes the request: its improvements,
    /// no spend or time limit, merge and adopt as read (the Operator's
    /// amendment of C-54); the Assistant's proposal is an intent.
    #[test]
    fn a_request_does_what_the_intent_was_read_as() {
        let shape = Shape {
            explore: true,
            cycles: 2,
            merge: true,
            adopt: false,
        };
        let request = StartRequest::new(" Find and fix ", shape, None);
        assert_eq!(request.intent, "Find and fix");
        assert!(request.explore && request.budgets.cycles == 2);
        assert_eq!((request.budgets.usd, request.budgets.hours), (None, None));
        assert!(request.budgets.check().is_ok());
        assert!(request.permissions.merge && !request.permissions.adopt);
        let review = StartRequest::new(
            "x",
            Shape {
                merge: false,
                adopt: true,
                ..shape
            },
            None,
        );
        assert!(!review.permissions.push && !review.permissions.adopt);
        let proposed = agq_assistant::tools::ObjectiveProposal {
            intent: " Find and fix problems ".into(),
        };
        let proposal = Proposal::from_assistant(&proposed);
        assert!(proposal.by_assistant && proposal.intent == "Find and fix problems");
    }
}
