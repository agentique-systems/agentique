//! The Conversation with the Assistant, kept per project (Scenario A9).
//!
//! One record serves both the Conversation panel and the model: the
//! Assistant's content blocks are stored exactly as the Claude API returned
//! them (text, thinking, tool use), so a conversation can be continued after
//! a restart, and the panel renders the same entries.

use crate::tools;
use agq_language::ElementId;
use agq_system_state::{ChangeEvent, Rejection, SystemState};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// The whole conversation of one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    /// A message the Operator wrote.
    Operator { text: String },
    /// One reply from the model: its content blocks exactly as returned
    /// (`text`, `thinking`, `tool_use`, ...).
    Assistant { content: Vec<Value> },
    /// The results of the tool calls in the reply before it.
    ToolResults { results: Vec<ToolResult> },
    /// Something the Operator should see that is not part of the exchange
    /// with the model: an error, a stop, a missing API key.
    Notice { text: String },
}

/// The result of one tool call, as the model reads it. The executor makes
/// it with one of the constructors below; the turn fills in `tool_use_id`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
    /// What a change did, for the Operator; the model reads `content`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeSummary>,
}

/// What a change did, or which locked elements the Operator kept, by raw
/// element ids: for the Conversation's cards and links to the Surface.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSummary {
    #[serde(default)]
    pub created: Vec<u64>,
    #[serde(default)]
    pub changed: Vec<u64>,
    #[serde(default)]
    pub deleted: usize,
    /// Problems at the created and changed elements after the change.
    #[serde(default)]
    pub problems: usize,
    /// Locked elements the Operator did not allow to change.
    #[serde(default)]
    pub refused: Vec<u64>,
}

impl ToolResult {
    /// An answer: model text read, a question answered.
    pub fn answer(content: impl Into<String>) -> Self {
        ToolResult {
            tool_use_id: String::new(),
            content: content.into(),
            is_error: false,
            change: None,
        }
    }

    /// The call could not be carried out; `content` says why.
    pub fn error(content: impl Into<String>) -> Self {
        ToolResult {
            is_error: true,
            ..ToolResult::answer(content)
        }
    }

    /// A change was applied: what changed, and problems at those elements.
    pub fn applied(state: &SystemState, event: &ChangeEvent) -> Self {
        let raw = |ids: &[ElementId]| ids.iter().map(|id| id.raw()).collect();
        let problems = state
            .diagnostics()
            .iter()
            .filter(|d| event.created.contains(&d.element) || event.updated.contains(&d.element))
            .count();
        ToolResult {
            change: Some(ChangeSummary {
                created: raw(&event.created),
                changed: raw(&event.updated),
                deleted: event.deleted.len(),
                problems,
                refused: Vec::new(),
            }),
            ..ToolResult::answer(tools::describe_event(state, event))
        }
    }

    /// A change was not applied, for example because the Operator did not
    /// allow a change to a locked element.
    pub fn rejected(state: &SystemState, rejection: &Rejection) -> Self {
        let refused = match rejection {
            Rejection::Locked { elements } => elements.iter().map(|id| id.raw()).collect(),
            _ => Vec::new(),
        };
        ToolResult {
            change: Some(ChangeSummary {
                refused,
                ..ChangeSummary::default()
            }),
            ..ToolResult::error(tools::describe_rejection(state, rejection))
        }
    }
}

impl Conversation {
    /// The `messages` for a Claude API request. Notices are left out, and a
    /// reply whose tool calls were never answered (for example after a stop)
    /// gets error results so the exchange stays well formed. Consecutive
    /// entries of one side (an Operator message after a failed request, say)
    /// form one message, so user and assistant messages alternate.
    pub fn api_messages(&self) -> Vec<Value> {
        let mut messages = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            match entry {
                Entry::Operator { text } => {
                    push(
                        &mut messages,
                        "user",
                        vec![json!({ "type": "text", "text": text })],
                    );
                }
                Entry::Assistant { content } => {
                    push(&mut messages, "assistant", content.clone());
                    let answered = matches!(
                        self.entries[index + 1..]
                            .iter()
                            .find(|entry| !matches!(entry, Entry::Notice { .. })),
                        Some(Entry::ToolResults { .. })
                    );
                    let calls = tool_use_ids(content);
                    if !answered && !calls.is_empty() {
                        let results: Vec<Value> = calls
                            .iter()
                            .map(|id| json!({ "type": "tool_result", "tool_use_id": id, "content": "Stopped by the Operator before this ran.", "is_error": true }))
                            .collect();
                        push(&mut messages, "user", results);
                    }
                }
                Entry::ToolResults { results } => {
                    let content: Vec<Value> = results
                        .iter()
                        .map(|result| json!({ "type": "tool_result", "tool_use_id": result.tool_use_id, "content": result.content, "is_error": result.is_error }))
                        .collect();
                    push(&mut messages, "user", content);
                }
                Entry::Notice { .. } => {}
            }
        }
        messages
    }

    /// Reads a conversation saved with [`save`](Self::save); a missing file
    /// is an empty conversation.
    pub fn load(path: &Path) -> std::io::Result<Conversation> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Conversation::default())
            }
            Err(error) => Err(error),
        }
    }

    /// Writes the conversation atomically (temporary file, then rename).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(temporary, path)
    }
}

/// Adds a message, or adds the content to the last message if it has the
/// same role.
fn push(messages: &mut Vec<Value>, role: &str, content: Vec<Value>) {
    if let Some(last) = messages.last_mut()
        && last["role"] == role
        && let Some(blocks) = last["content"].as_array_mut()
    {
        blocks.extend(content);
        return;
    }
    messages.push(json!({ "role": role, "content": content }));
}

/// The ids of the `tool_use` blocks in a reply.
pub fn tool_use_ids(content: &[Value]) -> Vec<String> {
    content
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
        .filter_map(|block| block.get("id").and_then(Value::as_str).map(str::to_string))
        .collect()
}
