//! The Conversation with the Assistant (REALIGNMENT §2.4, §3.2): the
//! Operator's messages, the Assistant's streamed replies, and its tool calls
//! carried out on the UI thread.
//!
//! A turn runs on a background thread ([`BackgroundTurn`]). Every frame the
//! Studio takes its events: streamed text, tool calls starting and finishing,
//! entries to add to the conversation (which is then saved), and tool calls
//! to carry out here. A change goes through [`StudioApp::apply_change`], the
//! path the Operator's own edits take: the Surface updates and highlights
//! it, it is one undo step, and a change to a locked element opens the same
//! confirmation, whose answer decides. A question from the Assistant waits
//! in the conversation until the Operator answers it.
//!
//! The conversation is kept per project in the Studio's local data, next to
//! the session file (never in the project folder), and saved after every
//! entry.
use crate::{app::StudioApp, edit::Outcome};
use agq_assistant::{
    BackgroundEvent, BackgroundTurn, ClaudeModel, Conversation, Entry, Model, Prepared,
    StreamEvent, ToolCall, ToolResult, TurnEvent, Usage, conversation::tool_use_ids, tools,
};
use agq_language::ElementId;
use agq_studio_scene::SceneTarget;
use agq_system_state::{Actor, ApplyError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Makes the model for each turn.
pub type ModelSource = Box<dyn Fn() -> Box<dyn Model + Send>>;

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
    /// The running turn, or the last one.
    pub last_turn: Option<TurnRecord>,
    /// A tool call waiting for the Operator.
    pub waiting: Option<Waiting>,
    /// Tool results by call id, from the conversation and as calls finish.
    pub results: HashMap<String, ToolResult>,
    /// The reply streaming in, until it is added to the conversation.
    pub live: Vec<Live>,
    pub thinking: bool,
    /// Tokens used in this session.
    pub usage: Usage,
    /// The model and effort, shown discreetly in the panel.
    pub model_name: String,
    pub key_missing: bool,
    pub new_model: ModelSource,
    /// Why the conversation could not be read or saved.
    pub error: Option<String>,
    pub shown: bool,
    /// Move the keyboard focus to the message input on the next frame.
    pub focus_input: bool,
    pub(crate) view: crate::conversation_ui::ViewCache,
}

/// Part of the reply that is streaming in.
pub enum Live {
    Text(String),
    Tool {
        id: String,
        name: String,
        /// The input as raw JSON text so far.
        input: String,
    },
}

/// One turn: the revision it started at, for undoing its changes.
pub struct TurnRecord {
    pub start: u64,
    /// The revision when it ended.
    pub end: Option<u64>,
    /// Changes it applied.
    pub changes: usize,
    /// It ended with a notice (an error, a stop, a limit) and can be retried.
    pub failed: bool,
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
}

impl Default for ConversationPanel {
    fn default() -> Self {
        let claude = ClaudeModel::from_env();
        ConversationPanel {
            conversation: Conversation::default(),
            path: None,
            input: String::new(),
            editing: None,
            turn: None,
            last_turn: None,
            waiting: None,
            results: HashMap::new(),
            live: Vec::new(),
            thinking: false,
            usage: Usage::default(),
            model_name: format!("{} · {}", claude.model, claude.effort),
            key_missing: !claude.has_key(),
            new_model: Box::new(|| Box::new(ClaudeModel::from_env())),
            error: None,
            shown: true,
            focus_input: false,
            view: Default::default(),
        }
    }
}

impl ConversationPanel {
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

    /// The number of changes "Undo the Assistant's changes" would undo, when
    /// it can: the last turn changed the model and nothing changed since.
    pub fn undoable(&self, revision: u64) -> Option<usize> {
        let record = self.last_turn.as_ref()?;
        (self.turn.is_none() && record.changes > 0 && record.end == Some(revision))
            .then_some(record.changes)
    }

    /// Whether the last turn failed and can be sent again.
    pub fn can_retry(&self) -> bool {
        self.turn.is_none() && self.last_turn.as_ref().is_some_and(|turn| turn.failed)
    }
}

/// Where a project's conversation is kept: `conversations/` next to the
/// session file, named after the project folder with a hash of its path.
pub fn conversation_path(session: &Path, folder: &Path) -> PathBuf {
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
        .join("conversations")
        .join(format!("{name}-{hash:016x}.json"))
}

impl StudioApp {
    /// Shows the conversation of the project in `folder`, stopping any turn
    /// of the previous one.
    pub fn load_conversation(&mut self, folder: &Path) {
        self.close_conversation();
        let path = conversation_path(&self.session_path, folder);
        let panel = &mut self.conversation;
        panel.conversation = match Conversation::load(&path) {
            Ok(conversation) => conversation,
            Err(error) => {
                panel.error = Some(format!(
                    "The saved conversation could not be read ({error}); a new one was started. The file is kept at {}.",
                    path.display()
                ));
                // Keep the unreadable file; the new conversation saves beside it.
                let _ = std::fs::rename(&path, path.with_extension("unreadable.json"));
                Conversation::default()
            }
        };
        panel.path = Some(path);
        panel.index_results();
    }

    /// Stops the turn and forgets everything that belongs to the open
    /// conversation, before another project is opened.
    fn close_conversation(&mut self) {
        self.stop_assistant();
        let panel = &mut self.conversation;
        panel.turn = None;
        panel.last_turn = None;
        panel.live.clear();
        panel.thinking = false;
        panel.editing = None;
        panel.error = None;
        panel.path = None;
    }

    fn save_conversation(&mut self) {
        let panel = &mut self.conversation;
        let Some(path) = &panel.path else { return };
        let saved = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| panel.conversation.save(path));
        panel.error = saved
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
            return;
        }
        let panel = &mut self.conversation;
        if panel.running() || panel.key_missing {
            return;
        }
        if let Some((index, draft)) = panel.editing.take() {
            panel.conversation.entries.truncate(index);
            panel.index_results();
            panel.input = draft;
        } else {
            panel.input.clear();
        }
        self.add_entry(Entry::Operator { text });
        self.start_turn();
    }

    fn start_turn(&mut self) {
        let Some(project) = &self.project else { return };
        let panel = &mut self.conversation;
        panel.live.clear();
        panel.thinking = false;
        panel.last_turn = Some(TurnRecord {
            start: project.state().revision(),
            end: None,
            changes: 0,
            failed: false,
        });
        let model = (panel.new_model)();
        panel.turn = Some(BackgroundTurn::start(model, panel.conversation.clone()));
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
                    match panel.live.last_mut() {
                        Some(Live::Text(live)) => live.push_str(&text),
                        _ => panel.live.push(Live::Text(text)),
                    }
                }
                StreamEvent::Thinking => panel.thinking = true,
                StreamEvent::ToolCallStarted { id, name } => {
                    panel.thinking = false;
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
                    panel.usage.input_tokens += usage.input_tokens;
                    panel.usage.cache_creation_input_tokens += usage.cache_creation_input_tokens;
                    panel.usage.cache_read_input_tokens += usage.cache_read_input_tokens;
                    panel.usage.output_tokens += usage.output_tokens;
                }
            },
            BackgroundEvent::Turn(TurnEvent::ToolFinished(result)) => {
                panel.results.insert(result.tool_use_id.clone(), result);
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
                let failed = matches!(
                    panel.conversation.entries.last(),
                    Some(Entry::Notice { .. })
                );
                let revision = self.project.as_ref().map(|p| p.state().revision());
                if let Some(record) = &mut self.conversation.last_turn {
                    record.end = revision;
                    record.failed = failed;
                }
            }
        }
    }

    /// Carries out one tool call, as the Studio does for the Operator's own
    /// edits (REALIGNMENT §3.2).
    fn carry_out(&mut self, call: ToolCall, reply: Sender<ToolResult>) {
        let Some(project) = &self.project else {
            let _ = reply.send(ToolResult::error("Not run: no project is open."));
            return;
        };
        let answer = |result| {
            let _ = reply.send(result);
        };
        match tools::prepare(project.state(), &call.name, &call.input) {
            Prepared::Answer(text) => answer(ToolResult::answer(text)),
            Prepared::Invalid(message) => answer(ToolResult::error(message)),
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
                    record.changes += 1;
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
    /// is open.
    pub fn answer_question(&mut self, answer: &str) -> bool {
        let panel = &mut self.conversation;
        let Some(Waiting {
            kind: WaitingFor::Question { .. },
            ..
        }) = &panel.waiting
        else {
            return false;
        };
        let waiting = panel.waiting.take().expect("checked above");
        let _ = waiting.reply.send(ToolResult::answer(answer.trim()));
        true
    }

    /// Stops the Assistant at once: no further model or tool call starts, a
    /// call waiting for the Operator is answered "not run", and its question
    /// or confirmation is closed. Changes made so far stay.
    pub fn stop_assistant(&mut self) {
        let panel = &mut self.conversation;
        if let Some(turn) = &panel.turn {
            turn.stop();
        }
        if let Some(waiting) = panel.waiting.take() {
            let _ = waiting
                .reply
                .send(ToolResult::error("Not run: stopped by the Operator."));
            if matches!(waiting.kind, WaitingFor::Confirmation)
                && matches!(&self.dialog, Some(crate::edit::Dialog::Confirm { change, .. }) if change.actor == Actor::Assistant)
            {
                self.dialog = None;
            }
        }
    }

    /// Undoes every change made since the last turn started (R-12).
    pub fn undo_assistant_changes(&mut self) {
        let Some(project) = &self.project else { return };
        if self
            .conversation
            .undoable(project.state().revision())
            .is_none()
        {
            return;
        }
        let Some(record) = self.conversation.last_turn.take() else {
            return;
        };
        let Some(project) = &mut self.project else {
            return;
        };
        match project.undo_since(record.start) {
            Ok(events) => {
                self.saved = Ok(());
                for event in &events {
                    self.changed(event);
                }
                self.status = format!("Undid {} change(s) by the Assistant", events.len());
                self.add_entry(Entry::Notice {
                    text: format!(
                        "You undid the Assistant's changes from this turn ({}). They can be redone with Ctrl+Y.",
                        events.len()
                    ),
                });
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
        let entries = &mut self.conversation.conversation.entries;
        while matches!(entries.last(), Some(Entry::Notice { .. })) {
            entries.pop();
        }
        if matches!(entries.last(), Some(Entry::Assistant { content }) if tool_use_ids(content).is_empty())
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
        let Some(index) = panel
            .conversation
            .entries
            .iter()
            .rposition(|entry| matches!(entry, Entry::Operator { .. }))
        else {
            return;
        };
        let Entry::Operator { text } = &panel.conversation.entries[index] else {
            return;
        };
        let draft = std::mem::replace(&mut panel.input, text.clone());
        panel.editing = Some((index, draft));
        panel.focus_input = true;
    }

    pub fn cancel_edit(&mut self) {
        let panel = &mut self.conversation;
        if let Some((_, draft)) = panel.editing.take() {
            panel.input = draft;
        }
    }

    /// Starts a new conversation for this project. The model is unaffected.
    pub fn new_conversation(&mut self) {
        self.stop_assistant();
        let panel = &mut self.conversation;
        panel.turn = None;
        panel.last_turn = None;
        panel.live.clear();
        panel.editing = None;
        panel.conversation = Conversation::default();
        panel.results.clear();
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
        self.panel = crate::app::Panel::Inspector;
        self.frame_target(&target);
    }
}

/// A scripted stand-in for the Claude API, shared by the turns of one
/// Studio: for tests and scripted journeys, never the network.
#[cfg(any(test, feature = "automation"))]
pub fn scripted(replies: Vec<agq_assistant::Reply>) -> ModelSource {
    use std::sync::{Arc, Mutex};
    struct Shared(Arc<Mutex<agq_assistant::ScriptedModel>>);
    impl Model for Shared {
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
    Box::new(move || Box::new(Shared(shared.clone())))
}

#[cfg(test)]
mod tests;
