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
}

impl From<Value> for Entry {
    fn from(value: Value) -> Entry {
        match serde_json::from_value::<Known>(value.clone()) {
            Ok(Known::Operator { text }) => Entry::Operator { text },
            Ok(Known::Assistant { model, parts }) => Entry::Assistant { model, parts },
            Ok(Known::ToolResults { results }) => Entry::ToolResults { results },
            Ok(Known::Notice { text }) => Entry::Notice { text },
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
    /// The `messages` of a model request, as Claude API messages (the
    /// Assistant's exchange with its model). The transcript and notices are
    /// left out, reasoning is kept only where the model that wrote it is
    /// `for_model`, a reply whose tool calls were never answered (for example
    /// after a stop) gets error results so the exchange stays well formed,
    /// and consecutive entries of one side form one message.
    pub fn api_messages(&self) -> Vec<Value> {
        self.messages_for(None)
    }

    /// As [`api_messages`](Self::api_messages), keeping reasoning only for
    /// `model` (every provider ties reasoning to its own models).
    pub fn messages_for(&self, model: Option<&ModelRef>) -> Vec<Value> {
        let entries = &self.entries[self.transcript.min(self.entries.len())..];
        let mut messages = Vec::new();
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
                            !matches!(entry, Entry::Notice { .. } | Entry::Other(_))
                        }),
                        Some(Entry::ToolResults { .. })
                    );
                    let calls = tool_use_ids(&blocks);
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
                Entry::Notice { .. } | Entry::Other(_) => {}
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
        Conversation::parse(&text)
            .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))
    }

    /// Reads a conversation file's text; see [`load`](Self::load).
    pub fn parse(text: &str) -> Result<Conversation, String> {
        let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
        match value.get("format").and_then(Value::as_u64) {
            Some(FORMAT) => {
                let entries = value["entries"]
                    .as_array()
                    .ok_or("a conversation without entries")?
                    .iter()
                    .cloned()
                    .map(Entry::from)
                    .collect();
                Ok(Conversation {
                    entries,
                    transcript: value["transcript"].as_u64().unwrap_or(0) as usize,
                })
            }
            None if value["entries"].is_array() => Ok(Conversation::import_format_1(&value)),
            Some(other) => Err(format!(
                "conversation format {other}, which this version does not read"
            )),
            None => Err("not a conversation".to_string()),
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

    /// Writes the conversation atomically (temporary file, then rename).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, self.to_text())?;
        std::fs::rename(temporary, path)
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
        .filter_map(|part| match part {
            AssistantPart::Text { text } => Some(json!({ "type": "text", "text": text })),
            AssistantPart::ToolCall { id, name, input } => {
                Some(json!({ "type": "tool_use", "id": id, "name": name, "input": input }))
            }
            AssistantPart::Reasoning(reasoning) if same_model => {
                let author = author.expect("same_model has an author");
                if author.provider == Provider::Anthropic {
                    reasoning.parts.first().map(|first| match first {
                        ReasoningPart::Redacted { data } => {
                            json!({ "type": "redacted_thinking", "data": data })
                        }
                        _ => json!({
                            "type": "thinking",
                            "thinking": reasoning.text(),
                            "signature": reasoning.parts.iter().find_map(|part| match part {
                                ReasoningPart::Text { signature, .. } => signature.clone(),
                                _ => None,
                            }).unwrap_or_default(),
                        }),
                    })
                } else {
                    Some(json!({
                        "type": "reasoning",
                        "provider": author.provider.id(),
                        "model": author.model,
                        "text": reasoning.text(),
                        "reasoning": reasoning,
                    }))
                }
            }
            AssistantPart::Reasoning(_) => None,
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
