//! The language model behind the Assistant.
//!
//! [`Model`] is what the tool-use loop talks to: the Claude API client in
//! production, a scripted stand-in in tests. The model's output is untrusted:
//! its tool calls are checked and go through [`crate::tools`], and change the
//! System State only the way the Operator's own edits do.

use serde_json::Value;
use std::sync::atomic::AtomicBool;

/// One request to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub system: String,
    /// Tool definitions ([`crate::tools::definitions`]).
    pub tools: Value,
    /// The conversation so far ([`crate::Conversation::api_messages`]).
    pub messages: Vec<Value>,
}

/// Something the model produced while a reply streams in, for live display.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamEvent {
    /// More text of the reply.
    Text(String),
    /// The model started thinking. The thinking itself is not shown.
    Thinking,
    /// The model started a tool call; its input follows.
    ToolCallStarted { id: String, name: String },
    /// More of a tool call's input, as raw JSON text.
    ToolInput { id: String, json: String },
    /// The tokens the reply used, once it is complete; for showing costs.
    Usage(Usage),
}

/// Tokens billed for one model call. Total input is the sum of the three
/// input counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Input read at the full price.
    pub input_tokens: u64,
    /// Input written to the prompt cache (a little above the full price).
    pub cache_creation_input_tokens: u64,
    /// Input read from the prompt cache (about a tenth of the price).
    pub cache_read_input_tokens: u64,
    /// Output, thinking included.
    pub output_tokens: u64,
}

/// A complete reply.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    /// The content blocks exactly as returned, to be stored in the
    /// conversation and sent back unchanged. A `tool_use` block whose input
    /// could not be read as a JSON object has the raw text as its `input`;
    /// the turn answers it with an error instead of running it.
    pub content: Vec<Value>,
    /// Why the model stopped: `end_turn`, `tool_use`, `max_tokens`, `refusal`, ...
    pub stop_reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    /// No API key is configured.
    MissingKey,
    /// The Operator pressed stop.
    Stopped,
    /// The request was refused or failed; the message is written for the
    /// Operator (invalid key, rate limit, service unavailable, network down).
    Failed(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::MissingKey => f.write_str(
                "No Claude API key is set. Set ANTHROPIC_API_KEY and restart Agentique; the Surface keeps working without it.",
            ),
            ModelError::Stopped => f.write_str("Stopped."),
            ModelError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ModelError {}

pub trait Model {
    /// Sends a request and streams the reply. Returns [`ModelError::Stopped`]
    /// promptly once `stop` is set; a stopped or failed reply is dropped.
    fn send(
        &mut self,
        request: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError>;
}

/// A deterministic stand-in that returns prepared replies in order: for tests
/// of the tool contracts and the loop without a network.
#[derive(Clone, Debug, Default)]
pub struct ScriptedModel {
    pub replies: std::collections::VecDeque<Reply>,
    /// Every request it received, for assertions.
    pub requests: Vec<Request>,
}

impl ScriptedModel {
    pub fn new(replies: impl IntoIterator<Item = Reply>) -> Self {
        ScriptedModel {
            replies: replies.into_iter().collect(),
            requests: Vec::new(),
        }
    }
}

impl Model for ScriptedModel {
    fn send(
        &mut self,
        request: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        self.requests.push(request.clone());
        if stop.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ModelError::Stopped);
        }
        let reply = self
            .replies
            .pop_front()
            .ok_or_else(|| ModelError::Failed("the script has no more replies".to_string()))?;
        for block in &reply.content {
            match block.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if let Some(text) = block.get("text").and_then(Value::as_str) {
                        on_event(StreamEvent::Text(text.to_string()));
                    }
                }
                Some("thinking") => on_event(StreamEvent::Thinking),
                Some("tool_use") => {
                    let id = block["id"].as_str().unwrap_or_default().to_string();
                    on_event(StreamEvent::ToolCallStarted {
                        id: id.clone(),
                        name: block["name"].as_str().unwrap_or_default().to_string(),
                    });
                    let json = match &block["input"] {
                        Value::String(raw) => raw.clone(),
                        input => input.to_string(),
                    };
                    on_event(StreamEvent::ToolInput { id, json });
                }
                _ => {}
            }
        }
        Ok(reply)
    }
}
