//! Providers (ROADMAP §4.7, §4.8; part `Providers` in
//! `models/agentique/Agentique.sysml`): talks to model providers through rig,
//! reads keys, knows each model's capabilities and reports usage.
//!
//! This is the only crate that depends on rig, tokio or reqwest (R-21, R-41).
//! None of their types appear here: the Assistant, the Studio and later
//! simulation see only the small types below. The API is synchronous: a
//! request returns a [`ChatHandle`] that delivers [`Event`]s over a channel
//! and can be cancelled; the async runtime stays inside this crate.
//!
//! - [`Providers::chat`] streams one model call (the Assistant's turns).
//! - [`capabilities`] is the capability table (§4.8): code outside this crate
//!   asks it, never a provider's name (§8.7).
//! - [`key_status`] says where a provider's key comes from.
#![forbid(unsafe_code)]

mod capabilities;
mod chat;
mod runtime;

pub use capabilities::{Capabilities, Price, PromptCache, ReasoningText, capabilities, price};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

/// A model provider (C-35).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "openrouter")]
    OpenRouter,
    #[serde(rename = "deepseek")]
    DeepSeek,
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::Anthropic,
        Provider::OpenAi,
        Provider::OpenRouter,
        Provider::DeepSeek,
    ];

    /// The stable id used in settings and conversations: `anthropic`,
    /// `openai`, `openrouter`, `deepseek`.
    pub fn id(self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
            Provider::OpenRouter => "openrouter",
            Provider::DeepSeek => "deepseek",
        }
    }

    pub fn from_id(id: &str) -> Option<Provider> {
        Provider::ALL
            .into_iter()
            .find(|provider| provider.id() == id.trim().to_ascii_lowercase())
    }

    /// The name shown to the Operator.
    pub fn name(self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::OpenAi => "OpenAI",
            Provider::OpenRouter => "OpenRouter",
            Provider::DeepSeek => "DeepSeek",
        }
    }

    /// The environment variable that holds the key; a non-empty value wins
    /// over a stored key (R-25).
    pub fn key_variable(self) -> &'static str {
        match self {
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::OpenAi => "OPENAI_API_KEY",
            Provider::OpenRouter => "OPENROUTER_API_KEY",
            Provider::DeepSeek => "DEEPSEEK_API_KEY",
        }
    }

    /// The model used when the Operator has not chosen one.
    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-opus-5",
            Provider::OpenAi => "gpt-6-astra",
            Provider::OpenRouter => "anthropic/claude-opus-5",
            Provider::DeepSeek => "deepseek-flash",
        }
    }
}

/// A model at a provider.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider: Provider,
    /// The provider's model id, for example `deepseek-flash`.
    pub model: String,
}

impl ModelRef {
    pub fn new(provider: Provider, model: impl Into<String>) -> ModelRef {
        ModelRef {
            provider,
            model: model.into(),
        }
    }
}

/// Where a provider's key comes from. Stage 5 adds keys stored in the
/// Windows Credential Manager (R-25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyStatus {
    Missing,
    /// Set by the named environment variable.
    FromEnvironment {
        variable: &'static str,
    },
}

/// Where the key for `provider` comes from. The key itself never leaves this
/// crate except in the request to its own provider (§8.7).
pub fn key_status(provider: Provider) -> KeyStatus {
    match environment_key(provider) {
        Some(_) => KeyStatus::FromEnvironment {
            variable: provider.key_variable(),
        },
        None => KeyStatus::Missing,
    }
}

fn environment_key(provider: Provider) -> Option<String> {
    std::env::var(provider.key_variable())
        .ok()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

/// A tool the model may call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// JSON Schema of the input object.
    pub input_schema: Value,
}

/// One model call.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatRequest {
    pub model: ModelRef,
    /// Reasoning effort, for models whose capabilities list efforts; ignored
    /// otherwise. `None` uses the model's default.
    pub effort: Option<String>,
    /// Upper limit for reasoning and reply together. Always explicit: rig's
    /// defaults are never relied on (§4.7).
    pub max_output_tokens: u64,
    pub system: String,
    pub tools: Vec<Tool>,
    /// The conversation so far; the last message is the Operator's or the
    /// tool results.
    pub messages: Vec<Message>,
}

/// A message of the conversation, provider-neutral (R-23).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", content = "content", rename_all = "snake_case")]
pub enum Message {
    User(Vec<UserPart>),
    Assistant(Vec<AssistantPart>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserPart {
    Text {
        text: String,
    },
    ToolResult {
        /// The provider's id of the call this answers.
        call_id: String,
        /// The tool's name.
        name: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantPart {
    Text {
        text: String,
    },
    /// The model's reasoning, kept to be sent back unchanged: providers tie it
    /// to the model and the conversation.
    Reasoning(Reasoning),
    ToolCall {
        /// The provider's id; tool results refer to it.
        id: String,
        name: String,
        /// The input object, or the raw text when it could not be read as a
        /// JSON object (the caller answers such a call with an error).
        input: Value,
    },
}

/// Reasoning as the provider returned it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reasoning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub parts: Vec<ReasoningPart>,
}

impl Reasoning {
    /// The readable text: full reasoning or summaries, without encrypted or
    /// redacted parts.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                ReasoningPart::Text { text, .. } | ReasoningPart::Summary { text } => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningPart {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    Summary {
        text: String,
    },
    Encrypted {
        data: String,
    },
    Redacted {
        data: String,
    },
}

/// Something the model produced while a reply streams in (the Assistant's
/// event protocol, interface 5 of §6.2).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// More text of the reply.
    Text(String),
    /// More reasoning text (full reasoning or a summary, per the model's
    /// capabilities); empty when the model thinks without showing it.
    Thinking(String),
    /// A tool call started. `stream_id` identifies it while it streams; the
    /// provider's own id follows in [`Event::ToolCallId`] when it differs.
    ToolCallStarted { stream_id: String, name: String },
    /// More of a tool call's input, as raw JSON text.
    ToolInput { stream_id: String, json: String },
    /// The provider's id for a call that started under `stream_id`; the
    /// reply's [`AssistantPart::ToolCall`] carries it.
    ToolCallId { stream_id: String, id: String },
    /// The tokens the call used, once it is complete.
    Usage(Usage),
    /// The call is over. Always the last event.
    Finished(Result<Reply, Error>),
}

/// A complete reply.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub content: Vec<AssistantPart>,
    pub stop: StopReason,
    pub usage: Usage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// The model finished its reply.
    EndTurn,
    /// The model asks for tools.
    ToolUse,
    /// The reply reached the output limit and was cut off.
    MaxTokens,
    /// The model or the provider declined the request.
    Refusal,
    Other(String),
}

/// Tokens billed for one model call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input read at the full price.
    pub input_tokens: u64,
    /// Input written to the prompt cache.
    pub cache_write_tokens: u64,
    /// Input read from the prompt cache.
    pub cache_read_tokens: u64,
    /// Output, reasoning included.
    pub output_tokens: u64,
    /// The part of the output that was reasoning, when the provider says.
    pub reasoning_tokens: u64,
}

impl Usage {
    pub fn add(&mut self, other: Usage) {
        self.input_tokens += other.input_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.output_tokens += other.output_tokens;
        self.reasoning_tokens += other.reasoning_tokens;
    }

    /// The estimated cost in US dollars from the dated price table ([`price`]),
    /// or `None` when the model has no known price.
    pub fn cost_usd(&self, model: &ModelRef) -> Option<f64> {
        let price = price(model)?;
        let million = 1_000_000.0;
        Some(
            (self.input_tokens as f64 * price.input
                + self.cache_write_tokens as f64 * price.cache_write
                + self.cache_read_tokens as f64 * price.cache_read
                + self.output_tokens as f64 * price.output)
                / million,
        )
    }
}

/// Why a call failed, in plain words for the Operator, with the kind of
/// failure for code that reacts to it (for example a link to Settings).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// No key is configured for the provider.
    MissingKey,
    /// 401: the key was refused.
    KeyRefused,
    /// 403, or 402 (billing): the key may not make this request.
    NoAccess,
    /// 404: the model does not exist or is not available to this key.
    UnknownModel,
    /// 429: try again later.
    RateLimited,
    /// 5xx and overload.
    Unavailable,
    /// The network or the provider could not be reached, or the reply was cut.
    Unreachable,
    /// The request was not accepted (400, 413, 422).
    Rejected,
    /// The caller cancelled the call.
    Cancelled,
    Other,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

/// A running model call. Events arrive in order and end with
/// [`Event::Finished`]. Dropping the handle cancels the call.
pub struct ChatHandle {
    events: Receiver<Event>,
    cancel: futures::future::AbortHandle,
    finished: bool,
}

impl ChatHandle {
    /// The next event, waiting at most `timeout`. `None` if nothing arrived.
    /// After [`cancel`](Self::cancel) the call ends with
    /// `Finished(Err(Cancelled))`.
    pub fn next_event(&mut self, timeout: Duration) -> Option<Event> {
        if self.finished {
            return None;
        }
        let event = match self.events.recv_timeout(timeout) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => return None,
            // The task ended without a result: it was cancelled.
            Err(RecvTimeoutError::Disconnected) => Event::Finished(Err(Error {
                kind: ErrorKind::Cancelled,
                message: "Stopped.".to_string(),
            })),
        };
        if matches!(event, Event::Finished(_)) {
            self.finished = true;
        }
        Some(event)
    }

    /// Stops the call at once: the request or stream is dropped and the
    /// connection closed.
    pub fn cancel(&self) {
        self.cancel.abort();
    }
}

impl Drop for ChatHandle {
    fn drop(&mut self) {
        self.cancel.abort();
    }
}

/// The entry point: makes model calls with keys from the environment, or
/// with explicit keys and endpoints (for testing a key before saving it, and
/// for tests against a local server).
#[derive(Clone, Debug, Default)]
pub struct Providers {
    keys: BTreeMap<Provider, String>,
    endpoints: BTreeMap<Provider, String>,
}

impl Providers {
    pub fn new() -> Providers {
        Providers::default()
    }

    /// Uses this key for `provider` instead of the environment's.
    pub fn with_key(mut self, provider: Provider, key: impl Into<String>) -> Providers {
        self.keys.insert(provider, key.into());
        self
    }

    /// Sends `provider`'s requests to another base URL, such as a local
    /// stand-in in tests.
    pub fn with_endpoint(mut self, provider: Provider, url: impl Into<String>) -> Providers {
        self.endpoints.insert(provider, url.into());
        self
    }

    /// Whether a key is available for `provider`.
    pub fn has_key(&self, provider: Provider) -> bool {
        self.key(provider).is_some()
    }

    fn key(&self, provider: Provider) -> Option<String> {
        self.keys
            .get(&provider)
            .cloned()
            .or_else(|| environment_key(provider))
    }

    /// Starts one streamed model call on the background runtime.
    pub fn chat(&self, request: ChatRequest) -> ChatHandle {
        let (sender, events) = std::sync::mpsc::channel();
        let key = self.key(request.model.provider);
        let endpoint = self.endpoints.get(&request.model.provider).cloned();
        let cancel = runtime::spawn(chat::run(request, key, endpoint, sender));
        ChatHandle {
            events,
            cancel,
            finished: false,
        }
    }
}
