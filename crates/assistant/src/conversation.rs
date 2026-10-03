//! The Conversation with the Assistant, kept per project (Scenario A9), in
//! conversation format 2 (R-23; the specification is in the crate's README):
//! provider-neutral entries, each reply with the model that wrote it, so a
//! conversation can be continued after a restart or on another model, and
//! the panel renders the same entries. Reasoning goes back only to the model
//! that wrote it. A format 1 file (Claude content blocks) is imported
//! read-only: its entries are shown, never sent to a model.

use crate::tools;
use agq_language::ElementId;
use agq_providers::{AssistantPart, ModelRef, Provider, Reasoning, ReasoningPart};
use agq_system_state::{ChangeEvent, Rejection, SystemState};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// The whole conversation of one project.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conversation {
    pub entries: Vec<Entry>,
    /// How many leading entries are a read-only transcript (imported from
    /// format 1): shown, never sent to a model.
    pub transcript: usize,
}

/// The conversation file's format.
pub const FORMAT: u64 = 2;

/// Why a conversation file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Not JSON, or not a conversation.
    Unreadable(String),
    /// A format this version does not read (a later version's).
    LaterFormat(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Unreadable(message) | ParseError::LaterFormat(message) => {
                f.write_str(message)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "Value", into = "Value")]
pub enum Entry {
    /// A message the Operator wrote.
    Operator { text: String },
    /// One reply from a model: the model that wrote it (none for an imported
    /// transcript) and its parts (text, reasoning, tool calls).
    Assistant {
        model: Option<ModelRef>,
        parts: Vec<AssistantPart>,
    },
    /// The results of the tool calls in the reply before it.
    ToolResults { results: Vec<ToolResult> },
    /// Something the Operator should see that is not part of the exchange
    /// with the model: an error, a stop, a missing API key.
    Notice { text: String },
    /// A runtime that keeps its own context (the Claude Agent runtime, C-51)
    /// started, resumed or was handed a session for the turn that follows:
    /// its session id, so the next turn can resume it. Never sent to a model.
    Session {
        runtime: String,
        id: String,
        /// `started`, `resumed` or `handed over`.
        event: String,
        /// The folder the session worked in (a development session's); the
        /// SDK keeps a session's transcript per folder, so it is resumed only
        /// there. `None` for a session in the runtime's own folder.
        folder: Option<String>,
    },
    /// An entry of a kind this version does not know (a later stage's), kept
    /// as it is, shown as a notice and never sent to a model.
    Other(Value),
}

/// The entries format 2 knows, as they are written.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Known {
    Operator {
        text: String,
    },
    Assistant {
        #[serde(default)]
        model: Option<ModelRef>,
        parts: Vec<AssistantPart>,
    },
    ToolResults {
        results: Vec<ToolResult>,
    },
    Notice {
        text: String,
    },
    Session {
        runtime: String,
        id: String,
        event: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
    },
}

impl From<Value> for Entry {
    fn from(value: Value) -> Entry {
        match serde_json::from_value::<Known>(value.clone()) {
            Ok(Known::Operator { text }) => Entry::Operator { text },
            Ok(Known::Assistant { model, parts }) => Entry::Assistant { model, parts },
            Ok(Known::ToolResults { results }) => Entry::ToolResults { results },
            Ok(Known::Notice { text }) => Entry::Notice { text },
            Ok(Known::Session {
                runtime,
                id,
                event,
                folder,
            }) => Entry::Session {
                runtime,
                id,
                event,
                folder,
            },
            Err(_) => Entry::Other(value),
        }
    }
}

impl From<Entry> for Value {
    fn from(entry: Entry) -> Value {
        let known = match entry {
            Entry::Other(value) => return value,
            Entry::Operator { text } => Known::Operator { text },
            Entry::Assistant { model, parts } => Known::Assistant { model, parts },
            Entry::ToolResults { results } => Known::ToolResults { results },
            Entry::Notice { text } => Known::Notice { text },
            Entry::Session {
                runtime,
                id,
                event,
                folder,
            } => Known::Session {
                runtime,
                id,
                event,
                folder,
            },
        };
        serde_json::to_value(known).expect("entries are plain JSON")
    }
}

impl Entry {
    /// An assistant entry from a reply's content blocks.
    pub fn reply(model: Option<ModelRef>, blocks: &[Value]) -> Entry {
        Entry::Assistant {
            parts: parts_from_blocks(blocks),
            model,
        }
    }
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
    /// An answer: model text read, a question answered. Cut to
    /// [`tools::RESULT_LIMIT`] with a note on narrowing the request (R-34).
    pub fn answer(content: impl Into<String>) -> Self {
        ToolResult {
            tool_use_id: String::new(),
            content: tools::cap(content.into()),
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
    /// The `messages` of the exchange as Claude API messages, keeping every
    /// model's reasoning; for inspecting the exchange (a request uses
    /// [`messages_for`](Self::messages_for)).
    pub fn api_messages(&self) -> Vec<Value> {
        self.messages_for(None)
    }

    /// The `messages` of a request to `model`, as Claude API messages (the
    /// Assistant's exchange with its model). The transcript, notices and
    /// entries of unknown kinds are left out; reasoning is kept only where
    /// `model` wrote it (every provider ties reasoning to its own models;
    /// `None` keeps all); a reply whose tool calls were never answered (for
    /// example after a stop) gets error results, and results for calls that
    /// are not there are dropped, so the exchange stays well formed;
    /// consecutive entries of one side form one message.
    pub fn messages_for(&self, model: Option<&ModelRef>) -> Vec<Value> {
        let entries = &self.entries[self.transcript.min(self.entries.len())..];
        let mut messages = Vec::new();
        // The calls of the last reply sent, which results may answer.
        let mut calls: Vec<String> = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            match entry {
                Entry::Operator { text } => {
                    push(
                        &mut messages,
                        "user",
                        vec![json!({ "type": "text", "text": text })],
                    );
                }
                Entry::Assistant {
                    model: author,
                    parts,
                } => {
                    let blocks = blocks_from_parts(parts, author.as_ref(), model);
                    if blocks.is_empty() {
                        continue;
                    }
                    push(&mut messages, "assistant", blocks.clone());
                    let answered = matches!(
                        entries[index + 1..].iter().find(|entry| {
                            !matches!(
                                entry,
                                Entry::Notice { .. } | Entry::Session { .. } | Entry::Other(_)
                            )
                        }),
                        Some(Entry::ToolResults { .. })
                    );
                    calls = tool_use_ids(&blocks);
                    if !answered && !calls.is_empty() {
                        let results: Vec<Value> = calls
                            .iter()
                            .map(|id| json!({ "type": "tool_result", "tool_use_id": id, "content": "Stopped by the Operator before this ran.", "is_error": true }))
                            .collect();
                        push(&mut messages, "user", results);
                    }
                }
                Entry::ToolResults { results } => {
                    // A reply this version cannot read (an unknown kind) is
                    // left out; its results go with it.
                    let content: Vec<Value> = results
                        .iter()
                        .filter(|result| calls.contains(&result.tool_use_id))
                        .map(|result| json!({ "type": "tool_result", "tool_use_id": result.tool_use_id, "content": result.content, "is_error": result.is_error }))
                        .collect();
                    calls.clear();
                    if !content.is_empty() {
                        push(&mut messages, "user", content);
                    }
                }
                Entry::Notice { .. } | Entry::Session { .. } | Entry::Other(_) => {}
            }
        }
        messages
    }

    /// Reads a conversation saved with [`save`](Self::save); a missing file
    /// is an empty conversation. A format 1 file is imported as a read-only
    /// transcript; another format is refused.
    pub fn load(path: &Path) -> std::io::Result<Conversation> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Conversation::default());
            }
            Err(error) => return Err(error),
        };
        Conversation::parse(&text).map_err(|error| match error {
            ParseError::Unreadable(message) => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, message)
            }
            // A later version's file: refused, and left as it is (§5.5).
            ParseError::LaterFormat(message) => {
                std::io::Error::new(std::io::ErrorKind::Unsupported, message)
            }
        })
    }

    /// Reads a conversation file's text; see [`load`](Self::load).
    pub fn parse(text: &str) -> Result<Conversation, ParseError> {
        let unreadable = |message: String| ParseError::Unreadable(message);
        let value: Value =
            serde_json::from_str(text).map_err(|error| unreadable(error.to_string()))?;
        match value.get("format") {
            Some(format) if format.as_u64() == Some(FORMAT) => {
                let entries = value["entries"]
                    .as_array()
                    .ok_or_else(|| unreadable("a conversation without entries".into()))?
                    .iter()
                    .cloned()
                    .map(Entry::from)
                    .collect();
                Ok(Conversation {
                    entries,
                    transcript: value["transcript"].as_u64().unwrap_or(0) as usize,
                })
            }
            Some(other) => Err(ParseError::LaterFormat(format!(
                "conversation format {other}, which this version does not read"
            ))),
            None if value["entries"].is_array() => Ok(Conversation::import_format_1(&value)),
            None => Err(unreadable("not a conversation".into())),
        }
    }

    /// A format 1 conversation (Claude content blocks) as a read-only
    /// transcript (R-23).
    fn import_format_1(value: &Value) -> Conversation {
        let mut entries: Vec<Entry> = value["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| match entry["type"].as_str() {
                Some("assistant") => Entry::reply(
                    None,
                    entry["content"]
                        .as_array()
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                ),
                _ => Entry::from(entry.clone()),
            })
            .collect();
        if entries.is_empty() {
            return Conversation::default();
        }
        entries.push(Entry::Notice {
            text: "The conversation above was kept by an earlier version; it is shown as it was, and the Assistant starts afresh from here.".to_string(),
        });
        let transcript = entries.len();
        Conversation {
            entries,
            transcript,
        }
    }

    /// The file's text: `{"format": 2, "transcript": n, "entries": [...]}`.
    pub fn to_text(&self) -> String {
        let entries: Vec<Value> = self.entries.iter().cloned().map(Value::from).collect();
        let mut file = json!({ "format": FORMAT, "entries": entries });
        if self.transcript > 0 {
            file["transcript"] = json!(self.transcript);
        }
        file.to_string()
    }

    /// Writes the conversation durably and atomically: a temporary file,
    /// flushed to disk, then renamed over the old one (§5.5).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let temporary = path.with_extension("json.tmp");
        let written = std::fs::File::create(&temporary).and_then(|mut file| {
            file.write_all(self.to_text().as_bytes())?;
            file.sync_all()
        });
        if let Err(error) = written.and_then(|()| std::fs::rename(&temporary, path)) {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(())
    }
}

/// Claude-style content blocks (a reply from any model, as the turn sees it)
/// as provider-neutral parts. `thinking` and `redacted_thinking` are Claude's
/// reasoning; `reasoning` blocks carry another provider's.
pub fn parts_from_blocks(blocks: &[Value]) -> Vec<AssistantPart> {
    let text = |block: &Value, field: &str| block[field].as_str().unwrap_or_default().to_string();
    blocks
        .iter()
        .filter_map(|block| match block["type"].as_str()? {
            "text" => Some(AssistantPart::Text {
                text: text(block, "text"),
            }),
            "tool_use" => Some(AssistantPart::ToolCall {
                id: text(block, "id"),
                name: text(block, "name"),
                input: block["input"].clone(),
            }),
            "thinking" => Some(AssistantPart::Reasoning(Reasoning {
                id: None,
                parts: vec![ReasoningPart::Text {
                    text: text(block, "thinking"),
                    signature: Some(text(block, "signature")).filter(|s| !s.is_empty()),
                }],
            })),
            "redacted_thinking" => Some(AssistantPart::Reasoning(Reasoning {
                id: None,
                parts: vec![ReasoningPart::Redacted {
                    data: text(block, "data"),
                }],
            })),
            "reasoning" => serde_json::from_value::<Reasoning>(block["reasoning"].clone())
                .ok()
                .map(AssistantPart::Reasoning),
            _ => None,
        })
        .collect()
}

/// Parts as Claude-style blocks for a request to `target`. Reasoning is
/// kept only when `target` is the model that wrote it (`author`): Claude's
/// as `thinking` blocks, other providers' as `reasoning` blocks naming the
/// provider and model.
pub fn blocks_from_parts(
    parts: &[AssistantPart],
    author: Option<&ModelRef>,
    target: Option<&ModelRef>,
) -> Vec<Value> {
    let same_model = author.is_some() && (target.is_none() || author == target);
    parts
        .iter()
        .flat_map(|part| match part {
            AssistantPart::Text { text } => vec![json!({ "type": "text", "text": text })],
            AssistantPart::ToolCall { id, name, input } => {
                vec![json!({ "type": "tool_use", "id": id, "name": name, "input": input })]
            }
            AssistantPart::Reasoning(reasoning) if same_model => {
                let author = author.expect("same_model has an author");
                if author.provider == Provider::Anthropic {
                    // One block per part, as the Messages API has them; a
                    // part without a signature cannot go back.
                    reasoning
                        .parts
                        .iter()
                        .filter_map(|part| match part {
                            ReasoningPart::Redacted { data } => {
                                Some(json!({ "type": "redacted_thinking", "data": data }))
                            }
                            ReasoningPart::Text {
                                text,
                                signature: Some(signature),
                            } => Some(json!({
                                "type": "thinking",
                                "thinking": text,
                                "signature": signature,
                            })),
                            _ => None,
                        })
                        .collect()
                } else {
                    vec![json!({
                        "type": "reasoning",
                        "provider": author.provider.id(),
                        "model": author.model,
                        "text": reasoning.text(),
                        "reasoning": reasoning,
                    })]
                }
            }
            AssistantPart::Reasoning(_) => Vec::new(),
        })
        .collect()
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
