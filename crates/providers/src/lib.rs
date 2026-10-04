//! Providers (ROADMAP §4.7, §4.8; part `Providers` in
//! `model/Agentique.sysml`): talks to model providers through rig,
//! reads keys, knows each model's capabilities and reports usage.
//!
//! This is the only crate that depends on rig, tokio or reqwest (R-21, R-41).
//! None of their types appear here: the Assistant, the Studio and later
//! simulation see only the small types below. The API is synchronous: a
//! request returns a [`ChatHandle`] that delivers [`Event`]s over a channel
//! and can be cancelled; the async runtime stays inside this crate.
//!
//! - [`Providers::chat`] streams one model call (the Assistant's turns).
//! - [`Providers::decide_start`] asks TypeSafe AI's Jev typed questions
//!   (fast agents, C-35) by a deadline and returns a [`DecisionHandle`]
//!   that can be cancelled; [`Providers::decide`] waits for one. A thin
//!   client until the migration to rig's released one (C-34, C-52).
//! - [`capabilities`] is the capability table (§4.8): code outside this crate
//!   asks it, never a provider's name (§8.7).
//! - [`key_status`] says where a provider's key comes from: its environment
//!   variable, else the Windows Credential Manager ([`keys`]);
//!   [`Providers::check_key`] tests a key and [`Providers::list_models`]
//!   lists a provider's models, for Settings.
#![forbid(unsafe_code)]

mod capabilities;
mod chat;
mod fallback;
pub mod jev;
pub mod keys;
mod models;
mod runtime;

pub use models::{KeyCheck, ModelInfo, read_models};

pub use capabilities::{
    Capabilities, DECISION_MODELS, Price, PromptCache, ReasoningText, capabilities, price,
    resolve_model,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// The chat adapter and its revision, as a run's binding names it (C-52):
/// change it when what a chat call sends or how its reply is read changes.
pub const CHAT_ADAPTER: &str = "agq-providers chat 1";

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
    /// TypeSafe AI: Jev's typed decisions for fast agents, never the
    /// Assistant's model (C-35).
    #[serde(rename = "typesafe")]
    TypeSafe,
}

impl Provider {
    pub const ALL: [Provider; 5] = [
        Provider::Anthropic,
        Provider::OpenAi,
        Provider::OpenRouter,
        Provider::DeepSeek,
        Provider::TypeSafe,
    ];

    /// The stable id used in settings and conversations: `anthropic`,
    /// `openai`, `openrouter`, `deepseek`, `typesafe`.
    pub fn id(self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
            Provider::OpenRouter => "openrouter",
            Provider::DeepSeek => "deepseek",
            Provider::TypeSafe => "typesafe",
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
            Provider::TypeSafe => "TypeSafe AI",
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
            Provider::TypeSafe => "TYPESAFE_API_KEY",
        }
    }

    /// The model used when the Operator has not chosen one (for Anthropic,
    /// the Assistant's default since C-54).
    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-5-5",
            Provider::OpenAi => "gpt-6-astra",
            Provider::OpenRouter => "anthropic/claude-opus-5",
            Provider::DeepSeek => "deepseek-flash",
            Provider::TypeSafe => jev::DEFAULT_MODEL,
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

    /// `provider/model`, as Settings write a role's model (C-54): the
    /// provider's id, then the model's id (which may hold `/` itself).
    pub fn parse(text: &str) -> Option<ModelRef> {
        let (provider, model) = text.trim().split_once('/')?;
        let provider = Provider::from_id(provider)?;
        (!model.trim().is_empty()).then(|| ModelRef::new(provider, model.trim()))
    }
}

/// `provider/model` (read back by [`ModelRef::parse`]).
impl std::fmt::Display for ModelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.provider.id(), self.model)
    }
}

/// A secret Agentique may hold (R-25, C-54): a provider's API key, or the
/// Operator's Claude subscription token (from `claude setup-token`), which
/// Anthropic accepts only through the Claude Agent runtime, so nothing in
/// this crate sends it anywhere ([`Providers`] never reads it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Credential {
    Key(Provider),
    ClaudeSubscription,
}

impl From<Provider> for Credential {
    fn from(provider: Provider) -> Credential {
        Credential::Key(provider)
    }
}

impl Credential {
    /// Its name in the credential store (`agentique:<id>`): the provider's
    /// id for a key, as before C-54.
    pub fn id(self) -> String {
        match self {
            Credential::Key(provider) => provider.id().to_string(),
            Credential::ClaudeSubscription => "anthropic-subscription".into(),
        }
    }

    /// The environment variable that holds it; a non-empty value wins over
    /// a stored one (R-25).
    pub fn variable(self) -> &'static str {
        match self {
            Credential::Key(provider) => provider.key_variable(),
            Credential::ClaudeSubscription => "CLAUDE_CODE_OAUTH_TOKEN",
        }
    }

    /// What the Operator calls it.
    pub fn name(self) -> String {
        match self {
            Credential::Key(provider) => format!("{} key", provider.name()),
            Credential::ClaudeSubscription => "Claude subscription token".into(),
        }
    }
}

/// Where a provider's key comes from (R-25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyStatus {
    Missing,
    /// Set by the named environment variable, which wins over a stored key.
    FromEnvironment {
        variable: &'static str,
    },
    /// Stored in the Windows Credential Manager.
    Stored,
    /// The credential store could not be read; the message says why.
    Unavailable(String),
}

/// Where the key for `provider` comes from. The key itself never leaves this
/// crate except in the request to its own provider (§8.7).
pub fn key_status(provider: Provider) -> KeyStatus {
    credential_status(Credential::Key(provider))
}

/// Where a key or the Claude subscription token comes from: its
/// environment variable, else the credential store (R-25).
pub fn credential_status(credential: Credential) -> KeyStatus {
    if environment(credential).is_some() {
        KeyStatus::FromEnvironment {
            variable: credential.variable(),
        }
    } else {
        match keys::stored(credential) {
            Ok(Some(_)) => KeyStatus::Stored,
            Ok(None) => KeyStatus::Missing,
            Err(error) => KeyStatus::Unavailable(error.0),
        }
    }
}

/// A key on its way to the one place that may hold it; never printed.
pub struct Secret(String);

impl Secret {
    /// A key given directly (tests, a key the Operator just typed).
    pub fn new(key: impl Into<String>) -> Secret {
        Secret(key.into())
    }

    /// The key itself, for the environment of the process it is for.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(…)")
    }
}

/// The Anthropic key for the Claude Agent runtime's process (ROADMAP §4.7):
/// the one recorded exception to §8.7 rule 4 (§7.6, 2026-10-01). The SDK in
/// that process sends it to Anthropic and nowhere else; the Assistant puts it
/// into that process's environment and nowhere else. The environment
/// variable wins over the stored key, as for every provider (R-25).
pub fn claude_agent_key() -> Result<Option<Secret>, String> {
    runtime_key(Provider::Anthropic)
}

/// The key for the Claude Agent runtime's process when its model answers
/// through `provider`: Anthropic's own API, or the provider's
/// Anthropic-compatible endpoint (DeepSeek's, C-53). The same exception to
/// §8.7 rule 4, and the key still goes only to its own provider's endpoint.
pub fn runtime_key(provider: Provider) -> Result<Option<Secret>, String> {
    runtime_credential(Credential::Key(provider))
}

/// A key, or the Claude subscription token, for the Claude Agent runtime's
/// process (the same exception to §8.7 rule 4; C-54 adds the token, which
/// that process's Claude Code sends to Anthropic and nowhere else).
pub fn runtime_credential(credential: Credential) -> Result<Option<Secret>, String> {
    if let Some(key) = environment(credential) {
        return Ok(Some(Secret(key)));
    }
    keys::stored(credential)
        .map(|key| key.map(Secret))
        .map_err(|error| error.0)
}

fn environment_key(provider: Provider) -> Option<String> {
    environment(Credential::Key(provider))
}

fn environment(credential: Credential) -> Option<String> {
    std::env::var(credential.variable())
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
    /// The reply arrived but cannot be used: too large, unreadable, or not
    /// an answer to the request that was sent. Never turned into an answer.
    InvalidReply,
    /// The caller cancelled the call.
    Cancelled,
    /// The call did not finish before its deadline.
    TimedOut,
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

/// A running typed decision (C-52): its result arrives once, by its
/// deadline. [`cancel`](Self::cancel) and dropping the handle stop it at
/// once: no further request is sent, a wait for a retry ends, the
/// connection closes, and a reply that arrives afterwards is never
/// delivered. (Whether the provider stops working, or billing, is not
/// known.)
pub struct DecisionHandle {
    result: Receiver<Result<jev::DecisionReply, jev::DecisionFailure>>,
    abort: futures::future::AbortHandle,
    cancelled: Arc<AtomicBool>,
    attempts: Arc<AtomicU32>,
    started: Instant,
    deadline: Instant,
    finished: bool,
}

/// How long past its deadline a handle waits for its task before it ends
/// the decision itself (a busy runtime must not stall a caller).
const DEADLINE_GRACE: Duration = Duration::from_millis(250);

impl DecisionHandle {
    /// The result, waiting at most `wait`; `None` if it has not arrived.
    /// After [`cancel`](Self::cancel) it is `Cancelled`, even when a reply
    /// was already on its way; after the deadline, `TimedOut`. Once a
    /// result was given, `None`.
    pub fn next_result(
        &mut self,
        wait: Duration,
    ) -> Option<Result<jev::DecisionReply, jev::DecisionFailure>> {
        if self.finished {
            return None;
        }
        let stopped = || self.cancelled.load(Ordering::SeqCst);
        if stopped() {
            self.finished = true;
            return Some(Err(jev::cancelled(self.attempts())));
        }
        let late = self.deadline + DEADLINE_GRACE;
        let wait = wait.min(late.saturating_duration_since(Instant::now()));
        let result = match self.result.recv_timeout(wait) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < late => return None,
            Err(RecvTimeoutError::Timeout) => {
                self.abort.abort();
                Err(jev::timed_out(self.attempts(), self.started.elapsed()))
            }
            Err(RecvTimeoutError::Disconnected) => Err(jev::DecisionFailure {
                error: Error {
                    kind: ErrorKind::Other,
                    message: "The request to TypeSafe AI failed unexpectedly. Try again.".into(),
                },
                attempts: self.attempts(),
                usage: None,
                request_id: None,
            }),
        };
        self.finished = true;
        // A stop that came while the result was on its way wins.
        if stopped() {
            return Some(Err(jev::cancelled(self.attempts())));
        }
        Some(result)
    }

    /// Stops the decision at once.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.abort.abort();
    }

    /// Requests sent so far, retries included.
    pub fn attempts(&self) -> u32 {
        self.attempts.load(Ordering::SeqCst)
    }
}

impl Drop for DecisionHandle {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.abort.abort();
    }
}

/// The entry point: makes model calls with keys from the environment, or
/// with explicit keys and endpoints (for testing a key before saving it, and
/// for tests against a local server).
#[derive(Clone, Default)]
pub struct Providers {
    keys: BTreeMap<Provider, String>,
    endpoints: BTreeMap<Provider, String>,
}

/// Never shows a key.
impl std::fmt::Debug for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Providers")
            .field("keys_for", &self.keys.keys().collect::<Vec<_>>())
            .field("endpoints", &self.endpoints)
            .finish()
    }
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

    /// An explicit key, or the environment's, or the stored one, unless the
    /// provider's requests go to another endpoint: a real key is sent only to
    /// its own provider (§8.7).
    fn key(&self, provider: Provider) -> Option<String> {
        self.keys.get(&provider).cloned().or_else(|| {
            if self.endpoints.contains_key(&provider) {
                None
            } else {
                environment_key(provider).or_else(|| keys::stored(provider).ok().flatten())
            }
        })
    }

    /// Tests `key` (or, without one, the configured key) against an endpoint
    /// that needs it and runs no model, so it costs nothing (R-25).
    /// Blocking, for up to ten seconds: call it off the UI thread.
    pub fn check_key(&self, provider: Provider, key: Option<&str>) -> KeyCheck {
        let key = key
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty())
            .or_else(|| self.key(provider));
        models::check(
            provider,
            key,
            self.endpoints.get(&provider).map(String::as_str),
        )
    }

    /// The provider's models, with their capabilities and list prices.
    /// Blocking, for up to ten seconds: call it off the UI thread.
    pub fn list_models(&self, provider: Provider) -> Result<Vec<ModelInfo>, Error> {
        models::list(
            provider,
            self.key(provider),
            self.endpoints.get(&provider).map(String::as_str),
        )
    }

    /// Asks Jev typed questions about a state and waits for the answers
    /// (fast agents, C-35), checked against the request. Blocking; retries
    /// rate limits and overload twice. A failure says how many requests
    /// were sent, since a sent request may be billed.
    pub fn decide(
        &self,
        request: &jev::DecisionRequest,
    ) -> Result<jev::DecisionReply, jev::DecisionFailure> {
        let mut handle = self.decide_start(request.clone(), Instant::now() + jev::TIMEOUT);
        loop {
            if let Some(result) = handle.next_result(Duration::from_millis(100)) {
                return result;
            }
        }
    }

    /// Starts a typed decision that must finish by `deadline` (monotonic):
    /// requests, reading replies, retries and their waits all share it, and
    /// a retry is made only when its wait fits. The handle cancels it.
    pub fn decide_start(&self, request: jev::DecisionRequest, deadline: Instant) -> DecisionHandle {
        let started = Instant::now();
        let running = jev::start(
            request,
            self.key(Provider::TypeSafe),
            self.endpoints.get(&Provider::TypeSafe).cloned(),
            deadline,
        );
        DecisionHandle {
            result: running.result,
            abort: running.abort,
            cancelled: running.cancelled,
            attempts: running.attempts,
            started,
            deadline,
            finished: false,
        }
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
