//! The Claude API client: sends a [`Request`] to the Messages API and streams
//! the reply back as [`StreamEvent`]s.
//!
//! There is no official Rust SDK, so this is plain HTTPS: one streamed POST to
//! `/v1/messages` per model call. The response body is read on its own thread
//! and handed over in chunks, so a stop takes effect at once even while the
//! network is silent. [`read_stream`] turns the server-sent events into a
//! [`Reply`]; it is a plain function over any [`BufRead`], tested with canned
//! streams.
//!
//! Every failure becomes a [`ModelError`] written for the Operator: a missing
//! or refused API key, an unknown model, rate limits, an overloaded service,
//! a lost connection. Rate limits, server errors and connection failures are
//! retried twice (after one, then two seconds, or the wait the API asks for
//! if that is at most ten seconds) before the error is reported.

use crate::model::{Model, ModelError, Reply, Request, StreamEvent, Usage};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

pub const API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const DEFAULT_MODEL: &str = "claude-opus-5";
pub const DEFAULT_EFFORT: &str = "high";
/// Upper limit for thinking and reply together, per model call.
pub const MAX_TOKENS: u32 = 64_000;
/// Enables `fallbacks: "default"`, which the API uses to rerun a request that
/// the model's safety classifiers declined on a recommended other model.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// Attempts after the first for rate limits, server errors and lost
/// connections, before anything was streamed.
const RETRIES: u32 = 2;
/// How often a wait checks the stop flag.
const POLL: Duration = Duration::from_millis(50);

/// The Claude API as the Assistant's [`Model`].
pub struct ClaudeModel {
    /// `ANTHROPIC_API_KEY`. Without it every request fails with
    /// [`ModelError::MissingKey`]; the rest of the Studio works as usual.
    pub key: Option<String>,
    /// `AGENTIQUE_MODEL`, default [`DEFAULT_MODEL`].
    pub model: String,
    /// `AGENTIQUE_EFFORT` (`low`, `medium`, `high`, `xhigh` or `max`),
    /// default [`DEFAULT_EFFORT`].
    pub effort: String,
    /// The Messages API endpoint, [`API_URL`]; tests point it elsewhere.
    pub url: String,
    client: Option<reqwest::blocking::Client>,
}

impl ClaudeModel {
    /// Configuration from the environment: `ANTHROPIC_API_KEY`,
    /// `AGENTIQUE_MODEL` and `AGENTIQUE_EFFORT`.
    pub fn from_env() -> ClaudeModel {
        let var = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        ClaudeModel {
            key: var("ANTHROPIC_API_KEY"),
            model: var("AGENTIQUE_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            effort: var("AGENTIQUE_EFFORT").unwrap_or_else(|| DEFAULT_EFFORT.to_string()),
            url: API_URL.to_string(),
            client: None,
        }
    }

    /// Whether an API key is configured.
    pub fn has_key(&self) -> bool {
        self.key.is_some()
    }

    /// The JSON body of one streamed Messages API request.
    ///
    /// The system prompt and tools never change within a conversation, so
    /// they are cached (the explicit breakpoint on the system prompt), and so
    /// is the growing conversation (the top-level `cache_control`). Tool input
    /// streams as it is generated (`eager_input_streaming`); the API then does
    /// not check it, so the turn checks every input before it runs.
    pub fn body(&self, request: &Request) -> Value {
        let tools: Vec<Value> = request
            .tools
            .as_array()
            .into_iter()
            .flatten()
            .map(|tool| {
                let mut tool = tool.clone();
                tool["eager_input_streaming"] = json!(true);
                tool
            })
            .collect();
        let mut body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "stream": true,
            // Readable summaries for the thinking row (R-31); on
            // claude-opus-5 and newer the default display is omitted.
            "thinking": { "type": "adaptive", "display": "summarized" },
            "output_config": { "effort": self.effort },
            "cache_control": { "type": "ephemeral" },
            "system": [{ "type": "text", "text": request.system, "cache_control": { "type": "ephemeral" } }],
            "tools": tools,
            "messages": claude_messages(&request.messages),
        });
        if self.uses_fallbacks() {
            body["fallbacks"] = json!("default");
        }
        body
    }

    /// Server-side fallbacks are requested for the default model, which the
    /// API documents them for; another model is sent without them.
    fn uses_fallbacks(&self) -> bool {
        self.model == DEFAULT_MODEL
    }

    fn client(&mut self) -> Result<reqwest::blocking::Client, ModelError> {
        if self.client.is_none() {
            let client = reqwest::blocking::Client::builder()
                // A reply may think for minutes before its next event; the
                // stop flag, not a timeout, ends a wait.
                .timeout(None)
                .connect_timeout(Duration::from_secs(10))
                .build()
                .map_err(|error| {
                    ModelError::Failed(format!(
                        "Could not prepare the connection to the Claude API ({error})."
                    ))
                })?;
            self.client = Some(client);
        }
        Ok(self.client.clone().expect("the client was just created"))
    }

    /// Starts one request on its own thread; the receiver gets either a
    /// failure or the response body in chunks. Dropping the receiver ends the
    /// thread and closes the connection at its next chunk.
    fn start(&mut self, key: &str, body: Vec<u8>) -> Result<Receiver<Incoming>, ModelError> {
        let mut request = self
            .client()?
            .post(&self.url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body);
        if self.uses_fallbacks() {
            request = request.header("anthropic-beta", FALLBACK_BETA);
        }
        let model = self.model.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut response = match request.send() {
                Ok(response) => response,
                Err(error) => {
                    let _ = sender.send(Incoming::Failed(Failure {
                        message: format!(
                            "Could not reach the Claude API ({}). Check the network connection and try again.",
                            plain_error(error)
                        ),
                        retry_after: Some(0),
                    }));
                    return;
                }
            };
            let status = response.status().as_u16();
            if !response.status().is_success() {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.trim().parse().ok());
                let text = response.text().unwrap_or_default();
                let _ = sender.send(Incoming::Failed(failure(
                    status,
                    retry_after,
                    &text,
                    &model,
                )));
                return;
            }
            loop {
                let mut chunk = vec![0; 16 * 1024];
                match response.read(&mut chunk) {
                    Ok(0) => return,
                    Ok(read) => {
                        chunk.truncate(read);
                        if sender.send(Incoming::Bytes(chunk)).is_err() {
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Incoming::Broken(error));
                        return;
                    }
                }
            }
        });
        Ok(receiver)
    }
}

impl Model for ClaudeModel {
    fn send(
        &mut self,
        request: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        let key = self.key.clone().ok_or(ModelError::MissingKey)?;
        let body = serde_json::to_vec(&self.body(request)).map_err(|error| {
            ModelError::Failed(format!("Could not write the request ({error})."))
        })?;
        let mut attempt = 0;
        loop {
            let receiver = self.start(&key, body.clone())?;
            match wait(&receiver, stop)? {
                Some(Incoming::Bytes(first)) => {
                    let body = Body {
                        receiver,
                        chunk: first,
                        position: 0,
                        stop,
                    };
                    return read_stream(body, on_event, stop);
                }
                Some(Incoming::Failed(failure)) => match failure.retry_after {
                    Some(seconds) if attempt < RETRIES && seconds <= 10 => {
                        attempt += 1;
                        pause(Duration::from_secs(seconds.max(u64::from(attempt))), stop)?;
                    }
                    _ => return Err(ModelError::Failed(failure.message)),
                },
                Some(Incoming::Broken(error)) => return Err(cut_off(&error)),
                None => return Err(cut_off(&io::Error::from(io::ErrorKind::UnexpectedEof))),
            }
        }
    }
}

/// What the request thread hands over.
enum Incoming {
    /// The request failed before a reply began.
    Failed(Failure),
    /// The next part of the reply's event stream.
    Bytes(Vec<u8>),
    /// Reading the reply failed.
    Broken(io::Error),
}

struct Failure {
    /// Written for the Operator.
    message: String,
    /// Seconds to wait before an automatic retry, if one makes sense.
    retry_after: Option<u64>,
}

/// The Operator-facing failure for an HTTP error status.
fn failure(status: u16, retry_after: Option<u64>, body: &str, model: &str) -> Failure {
    let detail: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let said = detail["error"]["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| body.trim().chars().take(300).collect());
    let (message, retry) = match status {
        401 => (
            "The Claude API did not accept the API key (401). Check ANTHROPIC_API_KEY and restart Agentique; the Surface keeps working by hand.".to_string(),
            None,
        ),
        402 => (
            format!("The Claude API account has a billing problem (402): {said}"),
            None,
        ),
        403 => (
            format!("This API key may not make this request (403): {said}"),
            None,
        ),
        404 => (
            format!(
                "The model `{model}` was not found or is not available to this API key (404). Set AGENTIQUE_MODEL to a model you can use, or leave it unset for {DEFAULT_MODEL}."
            ),
            None,
        ),
        413 => (
            "The conversation has become too large for one request (413). Start a new conversation; the architecture is unaffected.".to_string(),
            None,
        ),
        429 => (
            match retry_after {
                Some(seconds) => format!(
                    "The Claude API rate limit was reached (429). Try again in {seconds} seconds."
                ),
                None => "The Claude API rate limit was reached (429). Try again in a minute."
                    .to_string(),
            },
            Some(retry_after.unwrap_or(10)),
        ),
        500 => (
            "The Claude API had an internal error (500). Try again in a moment.".to_string(),
            Some(retry_after.unwrap_or(1)),
        ),
        502..=504 => (
            format!("The Claude API is not available right now ({status}). Try again in a moment."),
            Some(retry_after.unwrap_or(1)),
        ),
        529 => (
            "The Claude API is overloaded right now (529). Try again in a moment.".to_string(),
            Some(retry_after.unwrap_or(1)),
        ),
        400 => (
            format!("The Claude API did not accept the request (400): {said}"),
            None,
        ),
        _ => (
            format!("The Claude API answered with an error ({status}): {said}"),
            None,
        ),
    };
    Failure {
        message,
        retry_after: retry,
    }
}

/// A network error without the request URL, which says nothing new.
fn plain_error(error: reqwest::Error) -> String {
    let error = error.without_url();
    let mut text = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(inner) = source {
        text = format!("{text}: {inner}");
        source = inner.source();
    }
    text
}

fn cut_off(error: &io::Error) -> ModelError {
    ModelError::Failed(format!(
        "The connection to the Claude API was lost before the reply was complete ({error}). Try again."
    ))
}

/// Waits for the next message from the request thread, checking the stop
/// flag. `None` means the thread is done.
fn wait(receiver: &Receiver<Incoming>, stop: &AtomicBool) -> Result<Option<Incoming>, ModelError> {
    loop {
        if stop.load(Ordering::SeqCst) {
            return Err(ModelError::Stopped);
        }
        match receiver.recv_timeout(POLL) {
            Ok(incoming) => return Ok(Some(incoming)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(None),
        }
    }
}

/// Sleeps, unless the Operator stops first.
fn pause(duration: Duration, stop: &AtomicBool) -> Result<(), ModelError> {
    let end = std::time::Instant::now() + duration;
    while std::time::Instant::now() < end {
        if stop.load(Ordering::SeqCst) {
            return Err(ModelError::Stopped);
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

/// The response body as the request thread hands it over.
struct Body<'a> {
    receiver: Receiver<Incoming>,
    chunk: Vec<u8>,
    position: usize,
    stop: &'a AtomicBool,
}

impl Read for Body<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(out.len());
        out[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl BufRead for Body<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        while self.position == self.chunk.len() {
            match wait(&self.receiver, self.stop) {
                Err(_) => return Err(io::Error::other("stopped")),
                Ok(None) => return Ok(&[]),
                Ok(Some(Incoming::Bytes(chunk))) => {
                    self.chunk = chunk;
                    self.position = 0;
                }
                Ok(Some(Incoming::Broken(error))) => return Err(error),
                Ok(Some(Incoming::Failed(failure))) => {
                    return Err(io::Error::other(failure.message));
                }
            }
        }
        Ok(&self.chunk[self.position..])
    }

    fn consume(&mut self, count: usize) {
        self.position += count;
    }
}

/// Reads a Messages API event stream to its end and returns the reply.
///
/// Text and tool calls are reported through `on_event` as they arrive. The
/// stop flag is checked before every line; once set, the stream is abandoned
/// and [`ModelError::Stopped`] returned. A stream that ends before
/// `message_stop`, an `error` event and unreadable data are failures with a
/// message for the Operator. The reply's `stop_reason` is returned as sent,
/// including `refusal` and `max_tokens`; the caller decides what they mean.
pub fn read_stream(
    mut reader: impl BufRead,
    on_event: &mut dyn FnMut(StreamEvent),
    stop: &AtomicBool,
) -> Result<Reply, ModelError> {
    let mut stream = Stream::default();
    let mut line = Vec::new();
    let mut data = String::new();
    loop {
        if stop.load(Ordering::SeqCst) {
            return Err(ModelError::Stopped);
        }
        line.clear();
        let read = reader.read_until(b'\n', &mut line).map_err(|error| {
            if stop.load(Ordering::SeqCst) {
                ModelError::Stopped
            } else {
                cut_off(&error)
            }
        })?;
        let text = String::from_utf8_lossy(&line);
        let text = text.trim_end_matches(['\r', '\n']);
        // A blank line (or the end) completes an event; only its `data`
        // matters, since every data object names its own type.
        if text.is_empty() {
            if !data.is_empty() {
                stream.event(&data, on_event)?;
                data.clear();
            }
            if read == 0 {
                return stream.finish();
            }
        } else if let Some(rest) = text.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
}

/// The reply as it is being put together from stream events.
#[derive(Default)]
struct Stream {
    /// Content blocks by index.
    blocks: BTreeMap<u64, Value>,
    /// Raw tool input by block index.
    inputs: BTreeMap<u64, String>,
    stop_reason: Option<String>,
    usage: Usage,
    complete: bool,
}

impl Stream {
    fn event(
        &mut self,
        data: &str,
        on_event: &mut dyn FnMut(StreamEvent),
    ) -> Result<(), ModelError> {
        let event: Value = serde_json::from_str(data).map_err(|_| {
            ModelError::Failed(
                "The Claude API sent a reply that could not be read. Try again.".to_string(),
            )
        })?;
        let index = event["index"].as_u64().unwrap_or(0);
        match event["type"].as_str().unwrap_or_default() {
            "content_block_start" => {
                let block = event["content_block"].clone();
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = block["text"].as_str().filter(|text| !text.is_empty()) {
                            on_event(StreamEvent::Text(text.to_string()));
                        }
                    }
                    Some("thinking") | Some("redacted_thinking") => {
                        on_event(StreamEvent::Thinking(String::new()))
                    }
                    Some("tool_use") => {
                        self.inputs.insert(index, String::new());
                        on_event(StreamEvent::ToolCallStarted {
                            id: block["id"].as_str().unwrap_or_default().to_string(),
                            name: block["name"].as_str().unwrap_or_default().to_string(),
                        });
                    }
                    _ => {}
                }
                self.blocks.insert(index, block);
            }
            "content_block_delta" => {
                let delta = &event["delta"];
                let Some(block) = self.blocks.get_mut(&index) else {
                    return Ok(());
                };
                let piece = |field: &str| delta[field].as_str().unwrap_or_default().to_string();
                match delta["type"].as_str().unwrap_or_default() {
                    "text_delta" => {
                        let text = piece("text");
                        append(block, "text", &text);
                        on_event(StreamEvent::Text(text));
                    }
                    "thinking_delta" => {
                        let text = piece("thinking");
                        append(block, "thinking", &text);
                        on_event(StreamEvent::Thinking(text));
                    }
                    "signature_delta" => append(block, "signature", &piece("signature")),
                    "input_json_delta" => {
                        let json = piece("partial_json");
                        self.inputs.entry(index).or_default().push_str(&json);
                        let id = block["id"].as_str().unwrap_or_default().to_string();
                        on_event(StreamEvent::ToolInput { id, json });
                    }
                    _ => {}
                }
            }
            "message_start" => self.count(&event["message"]["usage"]),
            "message_delta" => {
                if let Some(reason) = event["delta"]["stop_reason"].as_str() {
                    self.stop_reason = Some(reason.to_string());
                }
                self.count(&event["usage"]);
            }
            "message_stop" => {
                self.complete = true;
                on_event(StreamEvent::Usage(self.usage));
            }
            "error" => {
                let message = event["error"]["message"].as_str().unwrap_or("no details");
                return Err(ModelError::Failed(
                    match event["error"]["type"].as_str() {
                        Some("overloaded_error") => {
                            "The Claude API became overloaded while replying. Try again in a moment."
                                .to_string()
                        }
                        Some("rate_limit_error") => {
                            "The Claude API rate limit was reached while replying. Try again in a minute."
                                .to_string()
                        }
                        _ => format!("The Claude API reported an error while replying: {message}"),
                    },
                ));
            }
            // `content_block_stop`, `ping` and anything new carry nothing
            // the reply needs.
            _ => {}
        }
        Ok(())
    }

    /// Takes the token counts a `message_start` or `message_delta` reports;
    /// a later count replaces an earlier one (they are running totals).
    fn count(&mut self, usage: &Value) {
        let fields = [
            ("input_tokens", &mut self.usage.input_tokens),
            (
                "cache_creation_input_tokens",
                &mut self.usage.cache_creation_input_tokens,
            ),
            (
                "cache_read_input_tokens",
                &mut self.usage.cache_read_input_tokens,
            ),
            ("output_tokens", &mut self.usage.output_tokens),
        ];
        for (name, count) in fields {
            if let Some(value) = usage[name].as_u64() {
                *count = value;
            }
        }
    }

    fn finish(mut self) -> Result<Reply, ModelError> {
        let (true, Some(stop_reason)) = (self.complete, self.stop_reason.take()) else {
            return Err(cut_off(&io::Error::from(io::ErrorKind::UnexpectedEof)));
        };
        for (index, raw) in &self.inputs {
            let Some(block) = self.blocks.get_mut(index) else {
                continue;
            };
            if raw.trim().is_empty() {
                continue;
            }
            block["input"] = match serde_json::from_str::<Value>(raw) {
                Ok(input @ Value::Object(_)) => input,
                _ => Value::String(raw.clone()),
            };
        }
        let content = after_fallback(self.blocks.into_values().collect());
        Ok(Reply {
            content,
            stop_reason,
        })
    }
}

/// The conversation's messages without blocks other providers wrote (their
/// `reasoning`), which the Messages API does not accept.
fn claude_messages(messages: &[Value]) -> Vec<Value> {
    const CLAUDE_BLOCKS: [&str; 6] = [
        "text",
        "thinking",
        "redacted_thinking",
        "tool_use",
        "tool_result",
        "image",
    ];
    messages
        .iter()
        .map(|message| {
            let mut message = message.clone();
            if let Some(blocks) = message["content"].as_array_mut() {
                blocks.retain(|block| {
                    block["type"]
                        .as_str()
                        .is_some_and(|kind| CLAUDE_BLOCKS.contains(&kind))
                });
            }
            message
        })
        .collect()
}

fn append(block: &mut Value, field: &str, text: &str) {
    let joined = format!("{}{text}", block[field].as_str().unwrap_or_default());
    block[field] = Value::String(joined);
}

/// When the API switched to a fallback model mid-reply, it marks the switch
/// with a `fallback` block. The declined model's thinking and tool calls
/// before the last switch must not be sent back; its text stays, as the
/// fallback model continued from it. The marker itself is dropped.
fn after_fallback(content: Vec<Value>) -> Vec<Value> {
    let Some(last) = content
        .iter()
        .rposition(|block| block["type"] == "fallback")
    else {
        return content;
    };
    content
        .into_iter()
        .enumerate()
        .filter(|(index, block)| *index > last || (*index < last && block["type"] == "text"))
        .map(|(_, block)| block)
        .collect()
}
