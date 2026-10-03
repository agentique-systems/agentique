//! The Conversation with the Assistant (ROADMAP §3.6, §4.2): the
//! Operator's messages, the Assistant's streamed replies, and its tool calls
//! carried out on the UI thread.
//!
//! A turn runs on a background thread ([`BackgroundTurn`]). Every frame the
//! Studio takes its events: streamed text, tool calls starting and finishing,
//! entries to add to the conversation (which is then saved), and tool calls
//! to carry out here. A change goes through [`Studio::apply_change`], the
//! path the Operator's own edits take: the Surface updates and highlights
//! it, it is one undo step, and a change to a locked element opens the same
//! confirmation, whose answer decides. A question from the Assistant waits
//! in the conversation until the Operator answers it.
//!
//! The conversation is kept per project in the Studio's local data, next to
//! the session file (never in the project folder), and saved after every
//! entry.
use crate::{edit::Outcome, studio::Studio};
use agq_assistant::{
    BackgroundEvent, BackgroundTurn, Conversation, Entry, ModelChoice, Prepared, StreamEvent,
    ToolCall, ToolResult, TurnEvent, Usage, tools,
};
use agq_language::ElementId;
use agq_studio_scene::SceneTarget;
use agq_system_state::{Actor, ApplyError, ChangeEvent, SystemState};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// Makes the runtime for each turn (ROADMAP §4.10): the loop over the
/// chosen model, or the Claude Agent runtime.
pub type RuntimeSource = Box<dyn Fn() -> Box<dyn agq_assistant::Runtime>>;

/// What the next runtime is made with, decided by the Studio just before
/// the turn starts (C-53): the development session for the open project, if
/// any. Shared with the runtime source, which reads it when it makes the
/// runtime.
#[derive(Default)]
pub struct RuntimeInputs {
    pub development: Option<agq_assistant::policy::Development>,
}

/// The Conversation panel's state.
pub struct ConversationPanel {
    pub conversation: Conversation,
    /// Where the open project's conversation is saved.
    path: Option<PathBuf>,
    /// The message being written. It is only cleared once it is in the
    /// conversation, so an error never loses it.
    pub input: String,
    /// The Operator message being edited: when sent, it and everything after
    /// it are replaced. The draft that was in the input before is kept.
    pub editing: Option<(usize, String)>,
    turn: Option<BackgroundTurn>,
    /// The Operator stopped the running turn: tool calls it still sends are
    /// answered "not run" and never carried out.
    stopped: bool,
    /// The running turn, or the last one.
    pub last_turn: Option<TurnRecord>,
    /// A tool call waiting for the Operator.
    pub waiting: Option<Waiting>,
    /// Tool results by call id, from the conversation and as calls finish.
    pub results: HashMap<String, ToolResult>,
    /// The reply streaming in, until it is added to the conversation.
    pub live: Vec<Live>,
    pub thinking: bool,
    /// What the Assistant is doing now, from its latest step (the visible
    /// phase of the turn).
    pub phase: Option<&'static str>,
    /// Tokens used in this session.
    pub usage: Usage,
    /// The running or last turn's model and tokens, for its estimated cost.
    pub turn_model: Option<agq_providers::ModelRef>,
    pub turn_usage: Usage,
    /// The model and effort, shown discreetly in the panel.
    pub model_name: String,
    /// What to tell the Operator while no key is set for the model.
    pub key_missing: Option<String>,
    pub new_runtime: RuntimeSource,
    /// Why the saved conversation could not be read; shown until another
    /// project is opened.
    pub read_error: Option<String>,
    /// Why the conversation could not be saved the last time.
    pub save_error: Option<String>,
    pub shown: bool,
    /// Move the keyboard focus to the message input on the next frame.
    pub focus_input: bool,
    /// Counts replacements of `input` by the Studio (an inserted selection,
    /// a message taken back to edit), so the composer shows them.
    pub input_set: u64,
    /// Counts replacements of earlier entries (edit and resend, retry, a new
    /// conversation, another project), after which the view forgets what it
    /// kept about them, such as selected text.
    pub epoch: u64,
    /// The next runtime's inputs, shared with `new_runtime`.
    pub inputs: std::rc::Rc<std::cell::RefCell<RuntimeInputs>>,
    /// Messages queued into the running turn and its pause gate (C-53);
    /// shared with the runtime the source makes.
    pub steering: agq_assistant::policy::Steering,
    /// Whether the runtime takes queued messages and the pause gate (the
    /// Claude Agent runtime does; the loop does not).
    pub steerable: bool,
    /// The tool the running turn is held at, while paused.
    pub held_at: Option<String>,
}

/// Part of the reply that is streaming in.
pub enum Live {
    Text(String),
    /// The model's thinking as it may be shown (a summary or the reasoning).
    Thinking(String),
    Tool {
        id: String,
        name: String,
        /// The input as raw JSON text so far.
        input: String,
    },
}

/// One turn: the revision it started at and the revisions of the changes
/// it applied, for undoing them.
pub struct TurnRecord {
    pub start: u64,
    pub applied: Vec<u64>,
    /// It ended with a notice (an error, a stop, a limit) and can be retried.
    pub failed: bool,
}

/// What undoing the last turn would undo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Undo {
    /// Only changes the Assistant made in the turn, this many.
    Assistant(usize),
    /// This many changes since the turn started, some of them not the
    /// Assistant's (the Operator's edits, or a redo).
    All(usize),
}

/// A tool call waiting for the Operator.
pub struct Waiting {
    pub call: ToolCall,
    reply: Sender<ToolResult>,
    pub kind: WaitingFor,
}

pub enum WaitingFor {
    /// `ask_operator`: answered in the conversation.
    Question {
        question: String,
        options: Vec<String>,
    },
    /// A change to locked elements: answered in the lock confirmation.
    Confirmation,
    /// A change that waits until the Operator closes the dialog they have
    /// open, so it never replaces it.
    Dialog(agq_system_state::Change),
    /// `save_to_library`: shown as a question with Save and Don't save
    /// ([`SAVE_OPTIONS`]); only Save changes My Library (C-49).
    SaveToLibrary {
        plan: Box<agq_library::SavePlan>,
        question: String,
        saved: String,
    },
}

/// The answers to the Assistant's request to save to My Library.
pub const SAVE_OPTIONS: [&str; 2] = ["Save to My Library", "Don't save"];

impl Default for ConversationPanel {
    fn default() -> Self {
        ConversationPanel::with_choice(ModelChoice::from_env())
    }
}

impl ConversationPanel {
    pub fn with_choice(choice: ModelChoice) -> Self {
        ConversationPanel {
            conversation: Conversation::default(),
            path: None,
            input: String::new(),
            editing: None,
            turn: None,
            stopped: false,
            last_turn: None,
            waiting: None,
            results: HashMap::new(),
            live: Vec::new(),
            thinking: false,
            phase: None,
            usage: Usage::default(),
            turn_model: None,
            turn_usage: Usage::default(),
            model_name: choice.label(),
            key_missing: (!choice.has_key()).then(|| choice.missing_key_message()),
            new_runtime: Box::new(move || agq_assistant::LoopRuntime::boxed(choice.start())),
            read_error: None,
            save_error: None,
            shown: true,
            focus_input: false,
            input_set: 0,
            epoch: 0,
            inputs: Default::default(),
            steering: Default::default(),
            steerable: false,
            held_at: None,
        }
    }

    /// The model the next turn uses (Settings changed it); a running turn
    /// keeps its own.
    pub fn use_choice(&mut self, choice: ModelChoice) {
        self.model_name = choice.label();
        self.key_missing = (!choice.has_key()).then(|| choice.missing_key_message());
        self.new_runtime = Box::new(move || agq_assistant::LoopRuntime::boxed(choice.start()));
        self.steerable = false;
    }

    /// Another runtime for the next turns (the Claude Agent runtime): its
    /// label, what keeps it from working if anything, and how to make it.
    pub fn use_runtime(&mut self, label: String, problem: Option<String>, source: RuntimeSource) {
        self.model_name = label;
        self.key_missing = problem;
        self.new_runtime = source;
        self.steerable = true;
    }

    pub fn running(&self) -> bool {
        self.turn.is_some()
    }

    /// Results of the tool calls in the conversation.
    fn index_results(&mut self) {
        self.results.clear();
        for entry in &self.conversation.entries {
            if let Entry::ToolResults { results } = entry {
                for result in results {
                    self.results
                        .insert(result.tool_use_id.clone(), result.clone());
                }
            }
        }
    }

    /// What undoing the last turn would undo, once it is over: only the
    /// Assistant's changes of that turn, or also changes by others made
    /// since it started.
    pub fn undoable(&self, state: &SystemState) -> Option<Undo> {
        let record = self.last_turn.as_ref()?;
        if self.turn.is_some() || record.applied.is_empty() {
            return None;
        }
        let steps = state.steps_since(record.start);
        if steps.is_empty() {
            return None;
        }
        let assistant = steps.iter().all(|(revision, actor)| {
            *actor == Actor::Assistant && record.applied.contains(revision)
        });
        Some(if assistant {
            Undo::Assistant(steps.len())
        } else {
            Undo::All(steps.len())
        })
    }

    /// The Operator's last message after the read-only transcript: the one
    /// that can be edited and sent again.
    pub fn last_operator(&self) -> Option<usize> {
        let transcript = self.conversation.transcript;
        self.conversation
            .entries
            .iter()
            .rposition(|entry| matches!(entry, Entry::Operator { .. }))
            .filter(|index| *index >= transcript)
    }

    /// Whether the last turn failed and can be sent again.
    pub fn can_retry(&self) -> bool {
        self.turn.is_none() && self.last_turn.as_ref().is_some_and(|turn| turn.failed)
    }
}

/// Where a project's conversation is kept (R-43): the project's own folder
/// under `projects/` next to the session file (`%APPDATA%\Agentique`),
/// named after the project folder with a hash of its path.
pub fn conversation_path(session: &Path, folder: &Path) -> PathBuf {
    project_data(session, folder).join("conversation.json")
}

/// Where Stages 3 and 4 kept it: `conversations/<name>-<hash>.json`. Read
/// once when the new place is empty, then left as it is.
fn earlier_conversation_path(session: &Path, folder: &Path) -> PathBuf {
    let data = project_data(session, folder);
    let name = data.file_name().unwrap_or_default().to_string_lossy();
    let directory = session.parent().unwrap_or(Path::new("."));
    directory.join("conversations").join(format!("{name}.json"))
}

/// `conversation.unreadable.json`, or with a number when that exists.
fn unused_name(path: &Path) -> PathBuf {
    (1..)
        .map(|n| match n {
            1 => path.with_extension("unreadable.json"),
            n => path.with_extension(format!("unreadable-{n}.json")),
        })
        .find(|name| !name.exists())
        .expect("some name is free")
}

/// The app's data for the project in `folder`.
pub(crate) fn project_data(session: &Path, folder: &Path) -> PathBuf {
    let folder = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    // FNV-1a: stable across runs and Rust versions.
    let hash = folder
        .to_string_lossy()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    let name: String = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let directory = session.parent().unwrap_or(Path::new("."));
    directory
        .join("projects")
        .join(format!("{name}-{hash:016x}"))
}

impl Studio {
    /// Shows the conversation of the project in `folder`, stopping any turn
    /// of the previous one.
    pub fn load_conversation(&mut self, folder: &Path) {
        self.close_conversation();
        let path = conversation_path(&self.session_path, folder);
        let earlier = earlier_conversation_path(&self.session_path, folder);
        // A conversation kept by Stages 3 and 4 is read (format 1, as a
        // read-only transcript) until the new place has one.
        let read = if !path.exists() && earlier.exists() {
            &earlier
        } else {
            &path
        };
        let read = read.clone();
        let panel = &mut self.conversation;
        panel.path = Some(path.clone());
        panel.conversation = match Conversation::load(&read) {
            Ok(conversation) => conversation,
            // A later version's file is left as it is, and this session
            // saves nothing over it (§5.5).
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
                panel.path = None;
                panel.read_error = Some(format!(
                    "The saved conversation is from a later version of Agentique ({error}); it is left as it is at {}, and this conversation is not saved.",
                    read.display()
                ));
                Conversation::default()
            }
            Err(error) => {
                // Keep the unreadable file under a name nothing overwrites;
                // a Stage 4 file stays where it is.
                let place = if read == path {
                    let kept = unused_name(&path);
                    match std::fs::rename(&path, &kept) {
                        Ok(()) => format!("It was kept as {}.", kept.display()),
                        Err(_) => format!("It is at {}.", path.display()),
                    }
                } else {
                    format!("It is at {}.", read.display())
                };
                panel.read_error = Some(format!(
                    "The saved conversation could not be read ({error}); a new one was started. {place}"
                ));
                Conversation::default()
            }
        };
        panel.index_results();
        panel.epoch += 1;
    }

    /// Stops the turn and forgets everything that belongs to the open
    /// conversation, before another project is opened.
    fn close_conversation(&mut self) {
        self.end_turn();
        let panel = &mut self.conversation;
        panel.last_turn = None;
        panel.editing = None;
        panel.read_error = None;
        panel.save_error = None;
        panel.path = None;
    }

    /// Stops the turn and takes its remaining events, so every result it
    /// produced is recorded and saved before the conversation is closed or
    /// replaced. A stopped turn ends within a fraction of a second.
    pub fn end_turn(&mut self) {
        self.stop_assistant();
        let started = Instant::now();
        while self.conversation.turn.is_some() && started.elapsed() < Duration::from_secs(2) {
            self.poll_conversation();
            std::thread::sleep(Duration::from_millis(2));
        }
        let panel = &mut self.conversation;
        panel.turn = None;
        panel.live.clear();
        panel.thinking = false;
        // Views name messages by position; the live ones are gone.
        panel.epoch += 1;
    }

    fn save_conversation(&mut self) {
        let panel = &mut self.conversation;
        let Some(path) = &panel.path else { return };
        let saved = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| panel.conversation.save(path));
        panel.save_error = saved
            .err()
            .map(|error| format!("The conversation could not be saved: {error}"));
    }

    fn add_entry(&mut self, entry: Entry) {
        self.conversation.conversation.entries.push(entry);
        self.save_conversation();
    }

    /// Sends the message in the input. While a question is open, the message
    /// answers it instead.
    pub fn send_message(&mut self) {
        let text = self.conversation.input.trim().to_string();
        if text.is_empty() || self.project.is_none() {
            return;
        }
        if self.answer_question(&text) {
            self.conversation.input.clear();
            self.conversation.input_set += 1;
            return;
        }
        let panel = &mut self.conversation;
        if panel.running() && panel.steerable && panel.editing.is_none() {
            // Steering (C-39, C-53): the message joins the running turn.
            panel.steering.queue(&text);
            panel.input.clear();
            panel.input_set += 1;
            self.add_entry(Entry::Notice {
                text: format!("You added, while the Assistant worked: {text}"),
            });
            return;
        }
        if panel.running() || panel.key_missing.is_some() {
            return;
        }
        if let Some((index, draft)) = panel.editing.take() {
            panel.conversation.entries.truncate(index);
            panel.epoch += 1;
            panel.index_results();
            panel.input = draft;
            panel.input_set += 1;
        } else {
            panel.input.clear();
            panel.input_set += 1;
        }
        self.add_entry(Entry::Operator { text });
        self.start_turn();
    }

    fn start_turn(&mut self) {
        let Some(project) = &self.project else { return };
        let panel = &mut self.conversation;
        panel.live.clear();
        panel.thinking = false;
        panel.stopped = false;
        panel.last_turn = Some(TurnRecord {
            start: project.state().revision(),
            applied: Vec::new(),
            failed: false,
        });
        panel.steering.set_gate(agq_assistant::policy::Gate::Run);
        panel.held_at = None;
        let development = self.conversation_development();
        let panel = &mut self.conversation;
        panel.inputs.borrow_mut().development = development;
        let runtime = (panel.new_runtime)();
        panel.turn_model = runtime.model();
        panel.turn_usage = Usage::default();
        panel.turn = Some(BackgroundTurn::start(runtime, panel.conversation.clone()));
    }

    /// Takes the running turn's events. Called every frame; never blocks.
    pub fn poll_conversation(&mut self) {
        // A change the Assistant made while a dialog was open applies once
        // the dialog is closed.
        if self.dialog.is_none()
            && let Some(waiting) = self.conversation.waiting.take()
        {
            match waiting.kind {
                WaitingFor::Dialog(change) => {
                    self.conversation.waiting = Some(Waiting {
                        kind: WaitingFor::Confirmation,
                        ..waiting
                    });
                    let outcome = self.apply_change(change);
                    self.assistant_change_done(outcome);
                }
                kind => self.conversation.waiting = Some(Waiting { kind, ..waiting }),
            }
        }
        let mut events = Vec::new();
        if let Some(turn) = &self.conversation.turn {
            while let Some(event) = turn.next_event() {
                events.push(event);
            }
        }
        for event in events {
            self.conversation_event(event);
        }
    }

    fn conversation_event(&mut self, event: BackgroundEvent) {
        let panel = &mut self.conversation;
        match event {
            BackgroundEvent::Turn(TurnEvent::Stream(stream)) => match stream {
                StreamEvent::Text(text) => {
                    panel.thinking = false;
                    panel.phase = Some("Writing the answer");
                    match panel.live.last_mut() {
                        Some(Live::Text(live)) => live.push_str(&text),
                        _ => panel.live.push(Live::Text(text)),
                    }
                }
                StreamEvent::Thinking(text) => {
                    panel.thinking = true;
                    match panel.live.last_mut() {
                        Some(Live::Thinking(live)) => live.push_str(&text),
                        _ => panel.live.push(Live::Thinking(text)),
                    }
                }
                StreamEvent::ToolCallId { stream_id, id } => {
                    for live in &mut panel.live {
                        if let Live::Tool { id: call, .. } = live
                            && *call == stream_id
                        {
                            *call = id.clone();
                        }
                    }
                }
                StreamEvent::ToolCallStarted { id, name } => {
                    panel.thinking = false;
                    panel.phase = Some(agq_assistant::phase(&name));
                    panel.live.push(Live::Tool {
                        id,
                        name,
                        input: String::new(),
                    });
                }
                StreamEvent::ToolInput { id, json } => {
                    for live in &mut panel.live {
                        if let Live::Tool {
                            id: call, input, ..
                        } = live
                            && *call == id
                        {
                            input.push_str(&json);
                        }
                    }
                }
                StreamEvent::Usage(usage) => {
                    panel.usage.add(usage);
                    panel.turn_usage.add(usage);
                    if let Some(cost) = panel.turn_model.as_ref().and_then(|m| usage.cost_usd(m)) {
                        self.daily_cost.add(cost);
                    }
                }
            },
            BackgroundEvent::Turn(TurnEvent::ToolFinished(result)) => {
                panel.results.insert(result.tool_use_id.clone(), result);
            }
            BackgroundEvent::Turn(TurnEvent::Activity(activity)) => {
                use agq_assistant::{Activity, TaskEvent};
                match activity {
                    Activity::Task {
                        event: TaskEvent::Started,
                        description,
                        agent,
                        ..
                    } => self.add_entry(Entry::Notice {
                        text: match agent {
                            Some(agent) => format!("Subagent {agent} started: {description}"),
                            None => format!("Started in the background: {description}"),
                        },
                    }),
                    Activity::Task {
                        event: TaskEvent::Done,
                        description,
                        agent,
                        status,
                        summary,
                        ..
                    } => self.add_entry(Entry::Notice {
                        text: format!(
                            "{} {}: {}{}",
                            agent
                                .map(|a| format!("Subagent {a}"))
                                .unwrap_or_else(|| "Background work".into()),
                            status.as_deref().unwrap_or("ended"),
                            description,
                            summary.map(|s| format!(" — {s}")).unwrap_or_default()
                        ),
                    }),
                    Activity::Task { .. } | Activity::Compacted { .. } => {}
                    Activity::Paused { tool } => {
                        self.status =
                            format!("The Assistant is paused before {tool}: Step or Resume");
                        self.conversation.held_at = Some(tool);
                    }
                }
            }
            BackgroundEvent::Turn(TurnEvent::Entry(entry)) => {
                match &entry {
                    Entry::Assistant { .. } => panel.live.clear(),
                    Entry::ToolResults { results } => {
                        for result in results {
                            panel
                                .results
                                .insert(result.tool_use_id.clone(), result.clone());
                        }
                    }
                    _ => {}
                }
                self.add_entry(entry);
            }
            BackgroundEvent::ToolCall { call, reply } => self.carry_out(call, reply),
            BackgroundEvent::Finished => {
                panel.turn = None;
                panel.live.clear();
                panel.thinking = false;
                panel.phase = None;
                let failed = matches!(
                    panel.conversation.entries.last(),
                    Some(Entry::Notice { .. })
                );
                if let Some(record) = &mut panel.last_turn {
                    record.failed = failed;
                }
                // Nothing waits for an answer any more.
                self.close_waiting(None);
            }
        }
    }

    /// Carries out one tool call, as the Studio does for the Operator's own
    /// edits (ROADMAP §4.2).
    fn carry_out(&mut self, call: ToolCall, reply: Sender<ToolResult>) {
        if self.conversation.stopped {
            let _ = reply.send(ToolResult::error("Not run: stopped by the Operator."));
            return;
        }
        let Some(project) = &self.project else {
            let _ = reply.send(ToolResult::error("Not run: no project is open."));
            return;
        };
        let answer = |result| {
            let _ = reply.send(result);
        };
        match tools::prepare(
            project.state(),
            &self.library.source,
            &call.name,
            &call.input,
        ) {
            Prepared::Answer(text) => answer(ToolResult::answer(text)),
            Prepared::Invalid(message) => answer(ToolResult::error(message)),
            Prepared::Studio(request) => self.carry_out_request(request, reply),
            Prepared::SaveToLibrary {
                plan,
                question,
                saved,
            } => {
                let panel = &mut self.conversation;
                panel.waiting = Some(Waiting {
                    call,
                    reply,
                    kind: WaitingFor::SaveToLibrary {
                        plan,
                        question,
                        saved,
                    },
                });
                panel.shown = true;
            }
            Prepared::Question { question, options } => {
                let panel = &mut self.conversation;
                panel.waiting = Some(Waiting {
                    call,
                    reply,
                    kind: WaitingFor::Question { question, options },
                });
                panel.shown = true;
                panel.focus_input = true;
            }
            Prepared::Change(_) if !self.editable() => answer(ToolResult::error(
                "Not applied: the Operator is looking at an earlier checkpoint, so the model cannot change now. Wait, or ask the Operator to return to the current model, then try again.",
            )),
            Prepared::Change(change) if self.dialog.is_some() => {
                self.conversation.waiting = Some(Waiting {
                    call,
                    reply,
                    kind: WaitingFor::Dialog(change),
                });
            }
            Prepared::Change(change) => {
                self.conversation.waiting = Some(Waiting {
                    call,
                    reply,
                    kind: WaitingFor::Confirmation,
                });
                let outcome = self.apply_change(change);
                self.assistant_change_done(outcome);
            }
        }
    }

    /// Replies to the Assistant's change with its outcome, unless the
    /// Operator is still being asked to confirm it.
    pub fn assistant_change_done(&mut self, outcome: Outcome) {
        if matches!(outcome, Outcome::Asking) {
            return;
        }
        let panel = &mut self.conversation;
        if !matches!(
            panel.waiting,
            Some(Waiting {
                kind: WaitingFor::Confirmation,
                ..
            })
        ) {
            return;
        }
        let waiting = panel.waiting.take().expect("checked above");
        let Some(project) = &self.project else {
            let _ = waiting
                .reply
                .send(ToolResult::error("Not applied: no project is open."));
            return;
        };
        let state = project.state();
        let result = match outcome {
            Outcome::Applied(event) => {
                if let Some(record) = &mut self.conversation.last_turn {
                    record.applied.push(event.revision);
                }
                ToolResult::applied(state, &event)
            }
            Outcome::NotApplied(ApplyError::Rejection(rejection)) => {
                ToolResult::rejected(state, &rejection)
            }
            Outcome::NotApplied(ApplyError::Project(error)) => ToolResult::error(format!(
                "Not applied: the change could not be saved: {error}"
            )),
            Outcome::NoProject | Outcome::Asking => {
                ToolResult::error("Not applied: no project is open.")
            }
        };
        let _ = waiting.reply.send(result);
    }

    /// Answers the Assistant's open question. Returns false when no question
    /// is open or the turn that asked it is gone.
    pub fn answer_question(&mut self, answer: &str) -> bool {
        let panel = &mut self.conversation;
        if !matches!(
            &panel.waiting,
            Some(Waiting {
                kind: WaitingFor::Question { .. } | WaitingFor::SaveToLibrary { .. },
                ..
            })
        ) {
            return false;
        }
        let waiting = panel.waiting.take().expect("checked above");
        let result = match waiting.kind {
            WaitingFor::SaveToLibrary { plan, saved, .. } => {
                let yes = matches!(
                    answer.trim().to_lowercase().as_str(),
                    "save to my library" | "save" | "yes"
                );
                if yes {
                    match self.library.source.save(*plan) {
                        Ok(()) => {
                            self.status = "Saved to My Library".into();
                            self.mark(crate::studio::Dirty::ALL);
                            ToolResult::answer(saved)
                        }
                        Err(error) => ToolResult::error(format!(
                            "Not saved: My Library could not be written: {error}"
                        )),
                    }
                } else {
                    ToolResult::answer(
                        "Not saved: the Operator chose not to save it to My Library.",
                    )
                }
            }
            _ => ToolResult::answer(answer.trim()),
        };
        waiting.reply.send(result).is_ok()
    }

    /// Holds the running turn at its next tool call (C-53).
    pub fn pause_assistant(&mut self) {
        if self.conversation.running() && self.conversation.steerable {
            self.conversation
                .steering
                .set_gate(agq_assistant::policy::Gate::Pause);
            self.status = "The Assistant pauses before its next tool call".into();
        }
    }

    /// Lets the paused turn take one tool call, then holds it again.
    pub fn step_assistant(&mut self) {
        if self.conversation.running() && self.conversation.steerable {
            self.conversation.held_at = None;
            self.conversation
                .steering
                .set_gate(agq_assistant::policy::Gate::Step);
        }
    }

    /// Lets the paused turn go on.
    pub fn resume_assistant(&mut self) {
        if self.conversation.steerable {
            self.conversation.held_at = None;
            self.conversation
                .steering
                .set_gate(agq_assistant::policy::Gate::Run);
            self.status = "The Assistant goes on".into();
        }
    }

    /// Whether the running turn is paused (or will pause at its next call).
    pub fn assistant_paused(&self) -> bool {
        self.conversation.running()
            && self.conversation.steering.gate() != agq_assistant::policy::Gate::Run
    }

    /// Stops the Assistant at once: no further model or tool call starts, a
    /// call waiting for the Operator is answered "not run", and its question
    /// or confirmation is closed. Changes made so far stay.
    pub fn stop_assistant(&mut self) {
        let panel = &mut self.conversation;
        if let Some(turn) = &panel.turn {
            turn.stop();
            panel.stopped = true;
        }
        self.close_waiting(Some("Not run: stopped by the Operator."));
        // Every other request the turn waits on is closed too (fail closed,
        // ROADMAP §5.6 item 4): a task it proposed is not started by a later
        // click, and runs or checks it asked for answer no one.
        if self.implementation.proposal.is_some() {
            if matches!(self.dialog, Some(crate::edit::Dialog::Implement { .. })) {
                self.dialog = None;
            }
            self.answer_proposal(Err("the Operator stopped the Assistant".into()));
        }
        for reply in [
            self.runs.assistant.take(),
            self.implementation.assistant.take(),
        ]
        .into_iter()
        .flatten()
        {
            let _ = reply.send(ToolResult::error(
                "Not waited for: the Operator stopped the Assistant. Whatever was started goes on, and its result is in the Studio.",
            ));
        }
    }

    /// Closes the call waiting for the Operator, answering it with `reply`
    /// if the turn still listens, and its lock confirmation if one is open.
    fn close_waiting(&mut self, reply: Option<&str>) {
        let Some(waiting) = self.conversation.waiting.take() else {
            return;
        };
        if let Some(reply) = reply {
            let _ = waiting.reply.send(ToolResult::error(reply));
        }
        if matches!(waiting.kind, WaitingFor::Confirmation)
            && matches!(&self.dialog, Some(crate::edit::Dialog::Confirm { change, .. }) if change.actor == Actor::Assistant)
        {
            self.dialog = None;
        }
    }

    /// Undoes every change made since the last turn started (R-12): the
    /// Assistant's, and those of others when [`ConversationPanel::undoable`]
    /// says so. Each can be redone.
    pub fn undo_assistant_changes(&mut self) {
        let Some(project) = &self.project else { return };
        let Some(undo) = self.conversation.undoable(project.state()) else {
            return;
        };
        let Some(record) = self.conversation.last_turn.take() else {
            return;
        };
        let Some(project) = &mut self.project else {
            return;
        };
        match project.undo_since(record.start) {
            Ok(events) => {
                self.saved = Ok(());
                if let Some(all) = merged(&events) {
                    self.changed(&all);
                }
                let count = plural(events.len(), "change", "changes");
                self.status = match undo {
                    Undo::Assistant(_) => format!("Undid {count} by the Assistant"),
                    Undo::All(_) => format!("Undid {count} since the Assistant started"),
                };
            }
            Err(error) => {
                self.saved = Err(error.to_string());
                self.status = format!("Not undone: could not save the project: {error}");
                self.conversation.last_turn = Some(record);
            }
        }
    }

    /// Sends the last message again after a failed turn: what the failed
    /// reply left (notices, an incomplete reply) is removed, and the turn
    /// continues from the last message or tool results.
    pub fn retry(&mut self) {
        if !self.conversation.can_retry() {
            return;
        }
        self.conversation.epoch += 1;
        let conversation = &mut self.conversation.conversation;
        // A read-only transcript is never removed.
        let floor = conversation.transcript;
        let entries = &mut conversation.entries;
        while entries.len() > floor && matches!(entries.last(), Some(Entry::Notice { .. })) {
            entries.pop();
        }
        if entries.len() > floor
            && matches!(entries.last(), Some(Entry::Assistant { parts, .. }) if !has_tool_calls(parts))
        {
            entries.pop();
        }
        let ready = self
            .conversation
            .conversation
            .api_messages()
            .last()
            .is_some_and(|message| message["role"] == "user");
        self.save_conversation();
        if ready {
            self.start_turn();
        }
    }

    /// Puts the last Operator message into the input to edit and send again.
    pub fn edit_last_message(&mut self) {
        let panel = &mut self.conversation;
        if panel.running() {
            return;
        }
        let Some(index) = panel.last_operator() else {
            return;
        };
        let Entry::Operator { text } = &panel.conversation.entries[index] else {
            return;
        };
        let draft = std::mem::replace(&mut panel.input, text.clone());
        panel.editing = Some((index, draft));
        panel.input_set += 1;
        panel.focus_input = true;
    }

    pub fn cancel_edit(&mut self) {
        let panel = &mut self.conversation;
        if let Some((_, draft)) = panel.editing.take() {
            panel.input = draft;
            panel.input_set += 1;
        }
    }

    /// Starts a new conversation for this project. The model is unaffected.
    pub fn new_conversation(&mut self) {
        self.end_turn();
        let panel = &mut self.conversation;
        panel.last_turn = None;
        panel.editing = None;
        panel.conversation = Conversation::default();
        panel.results.clear();
        panel.epoch += 1;
        panel.shown = true;
        panel.focus_input = true;
        self.save_conversation();
    }

    /// Puts the selected elements' qualified names into the message.
    pub fn insert_selection(&mut self) {
        let Some(project) = &self.project else { return };
        let tree = project.state().tree();
        let names: Vec<String> = self
            .working_elements()
            .into_iter()
            .filter(|id| tree.contains(*id))
            .map(|id| format!("`{}`", tree.qualified_name(id)))
            .collect();
        if names.is_empty() {
            self.status = "Select an element to insert it into the message".into();
            return;
        }
        let input = &mut self.conversation.input;
        if !input.is_empty() && !input.ends_with([' ', '\n']) {
            input.push(' ');
        }
        input.push_str(&names.join(", "));
        input.push(' ');
        self.conversation.input_set += 1;
        self.conversation.shown = true;
        self.conversation.focus_input = true;
    }

    /// Selects an element named in the conversation on the Surface and shows
    /// it in the Inspector: its card, a port on a card, or a connection;
    /// otherwise the card that owns it, with the element in the Inspector.
    pub fn reveal(&mut self, id: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        if !tree.contains(id) {
            self.status = "That element no longer exists".into();
            return;
        }
        let own = [SceneTarget::Node(id)]
            .into_iter()
            .chain(
                self.scene
                    .ports
                    .iter()
                    .filter(|port| port.id == id)
                    .map(|port| SceneTarget::Port(port.owner, id)),
            )
            .chain(
                self.scene
                    .edges
                    .iter()
                    .filter(|edge| edge.semantic.element == Some(id))
                    .map(|edge| SceneTarget::Edge(edge.semantic.id.clone())),
            )
            .find(|target| self.scene.target_bounds(target).is_some());
        let target = match own {
            Some(target) => Some((target, None)),
            None => {
                let mut owner = tree.get(id).and_then(|element| element.owner());
                let mut found = None;
                while let Some(card) = owner {
                    let target = SceneTarget::Node(card);
                    if self.scene.target_bounds(&target).is_some() {
                        found = Some((target, Some(id)));
                        break;
                    }
                    owner = tree.get(card).and_then(|element| element.owner());
                }
                found
            }
        };
        let Some((target, inspected)) = target else {
            self.status = "That element is not shown in this view".into();
            return;
        };
        self.select(target.clone(), false);
        self.inspected = inspected.map(|id| (self.selection.primary.clone(), id));
        self.panel = crate::studio::Panel::Inspector;
        self.frame_target(&target);
    }
}

/// `1 change`, `2 changes`.
pub fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// One event for a series of undone changes, so the Surface is rebuilt and
/// highlighted once.
fn merged(events: &[ChangeEvent]) -> Option<ChangeEvent> {
    let mut all = events.last()?.clone();
    for event in &events[..events.len() - 1] {
        all.created.extend(&event.created);
        all.updated.extend(&event.updated);
        all.deleted.extend(&event.deleted);
    }
    for ids in [&mut all.created, &mut all.updated, &mut all.deleted] {
        ids.sort();
        ids.dedup();
    }
    Some(all)
}

/// A scripted stand-in for the Claude API, shared by the turns of one
/// Studio: for tests and scripted journeys, never the network.
#[cfg(any(test, feature = "automation"))]
pub fn scripted(replies: Vec<agq_assistant::Reply>) -> RuntimeSource {
    use std::sync::{Arc, Mutex};
    struct Shared(Arc<Mutex<agq_assistant::ScriptedModel>>);
    impl agq_assistant::Model for Shared {
        fn send(
            &mut self,
            request: &agq_assistant::Request,
            on_event: &mut dyn FnMut(StreamEvent),
            stop: &std::sync::atomic::AtomicBool,
        ) -> Result<agq_assistant::Reply, agq_assistant::ModelError> {
            self.0
                .lock()
                .expect("the script")
                .send(request, on_event, stop)
        }
    }
    let shared = Arc::new(Mutex::new(agq_assistant::ScriptedModel::new(replies)));
    Box::new(move || agq_assistant::LoopRuntime::boxed(Box::new(Shared(shared.clone()))))
}

#[cfg(test)]
mod tests;

/// Whether a reply asked for tool calls.
fn has_tool_calls(parts: &[agq_providers::AssistantPart]) -> bool {
    parts
        .iter()
        .any(|part| matches!(part, agq_providers::AssistantPart::ToolCall { .. }))
}
