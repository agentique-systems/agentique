//! Anthropic's server-side refusal fallbacks through rig (C-27, Q-18): a thin
//! adapter until rig reads the `fallback` content block itself.
//!
//! With `fallbacks: "default"` and the [`BETA`] header, a request the model's
//! safety classifiers decline is continued on another model in the same
//! stream; a content block of type `fallback` marks the switch. rig 0.42.0
//! does not know that block type and ends the stream with an error, so rig's
//! Anthropic client is given [`HttpClient`]: reqwest, except that a streamed
//! response passes through a [`Filter`] that removes the `fallback` block's
//! frames before rig parses them and records the switch.
//!
//! After a switch, the declined model's reasoning and tool calls before it
//! are not part of the reply; its text stays, as the fallback model continued
//! from it (Anthropic's rule for sending a fallback turn back, and what
//! `after_fallback` does in the Assistant's hand-written client).

use crate::AssistantPart;
use bytes::Bytes;
use futures::StreamExt;
use rig_core::http_client::sse::BoxedStream;
use rig_core::http_client::{
    self, HttpClientExt, LazyBody, MultipartForm, Request, Response, StreamingResponse,
};
use rig_core::wasm_compat::WasmCompatSend;
use serde_json::Value;
use std::collections::BTreeSet;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};

/// Enables `fallbacks: "default"`: the API picks the fallback model by the
/// category of the refusal.
pub(crate) const BETA: &str = "server-side-fallback-2026-07-01";

/// What the declined model (or models) wrote before the last switch, counted
/// as rig makes reply parts of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Switch {
    /// Reasoning blocks: thinking with text or a signature, and redacted
    /// thinking.
    reasoning: usize,
    /// Tool calls, in the order they started.
    tool_calls: usize,
}

impl Switch {
    /// Removes the declined reasoning and tool calls from a reply's
    /// `content`; text stays. Reasoning parts are in the order they started,
    /// so the declined ones come first; `calls_started` holds the reply's
    /// tool call ids in the order the calls started.
    pub(crate) fn drop_declined(&self, content: &mut Vec<AssistantPart>, calls_started: &[&str]) {
        let declined = &calls_started[..self.tool_calls.min(calls_started.len())];
        let mut reasoning = self.reasoning;
        content.retain(|part| match part {
            AssistantPart::Reasoning(_) if reasoning > 0 => {
                reasoning -= 1;
                false
            }
            AssistantPart::ToolCall { id, .. } => !declined.contains(&id.as_str()),
            _ => true,
        });
    }
}

/// rig's HTTP client for Anthropic: reqwest, with the `fallback` block's
/// frames removed from streamed responses. Clones share what the latest
/// streamed response said about a switch.
#[derive(Clone, Debug, Default)]
pub(crate) struct HttpClient {
    inner: reqwest::Client,
    switch: Arc<Mutex<Option<Switch>>>,
}

impl HttpClient {
    /// The last switch in the latest streamed response, if one happened.
    pub(crate) fn switch(&self) -> Option<Switch> {
        *self.switch.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HttpClientExt for HttpClient {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes>,
        T: WasmCompatSend,
        U: From<Bytes>,
        U: WasmCompatSend + 'static,
    {
        self.inner.send(req)
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        U: From<Bytes>,
        U: WasmCompatSend + 'static,
    {
        self.inner.send_multipart(req)
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        let response = self.inner.send_streaming(req);
        let switch = self.switch.clone();
        async move {
            let response = response.await?;
            *switch.lock().unwrap_or_else(PoisonError::into_inner) = None;
            Ok(response.map(|body| filtered(body, switch)))
        }
    }
}

/// `body` without the `fallback` block's frames; a switch is recorded in
/// `switch` as soon as its frame passes.
fn filtered(body: BoxedStream, switch: Arc<Mutex<Option<Switch>>>) -> BoxedStream {
    let state = (body, Filter::default(), switch, false);
    Box::pin(futures::stream::unfold(
        state,
        |(mut body, mut filter, switch, ended)| async move {
            if ended {
                return None;
            }
            loop {
                let (kept, ended) = match body.next().await {
                    Some(Ok(chunk)) => (filter.push(&chunk), false),
                    Some(Err(error)) => return Some((Err(error), (body, filter, switch, true))),
                    None => (filter.finish(), true),
                };
                if filter.switch.is_some() {
                    *switch.lock().unwrap_or_else(PoisonError::into_inner) = filter.switch;
                }
                if !kept.is_empty() {
                    return Some((Ok(Bytes::from(kept)), (body, filter, switch, ended)));
                }
                if ended {
                    return None;
                }
            }
        },
    ))
}

/// Removes the `fallback` block's frames from an Anthropic event stream and
/// counts what came before the last switch. Bytes arrive in any chunks; a
/// frame is passed on, unchanged, once its blank line has arrived. Lines end
/// with LF or CRLF, as Anthropic's do.
#[derive(Debug, Default)]
struct Filter {
    /// The start of a frame whose blank line has not arrived yet.
    pending: Vec<u8>,
    /// Indices of `fallback` blocks.
    fallbacks: BTreeSet<u64>,
    /// What rig makes parts of, so far.
    seen: Switch,
    /// Thinking blocks with neither text nor a signature yet: rig makes no
    /// part of them until they get one.
    empty_thinking: BTreeSet<u64>,
    /// The last switch, if any.
    switch: Option<Switch>,
}

impl Filter {
    /// Takes the next chunk; returns the complete frames to pass on.
    fn push(&mut self, chunk: &[u8]) -> Vec<u8> {
        self.pending.extend_from_slice(chunk);
        let mut kept = Vec::new();
        while let Some(end) = frame_end(&self.pending) {
            let frame: Vec<u8> = self.pending.drain(..end).collect();
            if self.keep(&frame) {
                kept.extend_from_slice(&frame);
            }
        }
        kept
    }

    /// At the end of the stream: an incomplete last frame is passed on as it
    /// is, for rig to judge.
    fn finish(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }

    fn keep(&mut self, frame: &[u8]) -> bool {
        let Some(event) = event(frame) else {
            return true;
        };
        let Some(index) = event["index"].as_u64() else {
            return true;
        };
        let text = |value: &Value| value.as_str().is_some_and(|text| !text.is_empty());
        match event["type"].as_str() {
            Some("content_block_start") => {
                let block = &event["content_block"];
                match block["type"].as_str() {
                    Some("fallback") => {
                        self.fallbacks.insert(index);
                        self.switch = Some(self.seen);
                        return false;
                    }
                    Some("thinking") if text(&block["thinking"]) || text(&block["signature"]) => {
                        self.seen.reasoning += 1;
                    }
                    Some("thinking") => {
                        self.empty_thinking.insert(index);
                    }
                    Some("redacted_thinking") => self.seen.reasoning += 1,
                    Some("tool_use") => self.seen.tool_calls += 1,
                    _ => {}
                }
                true
            }
            Some("content_block_delta" | "content_block_stop") => {
                if self.fallbacks.contains(&index) {
                    return false;
                }
                let delta = &event["delta"];
                let content = match delta["type"].as_str() {
                    // rig opens a reasoning part on any thinking delta.
                    Some("thinking_delta") => true,
                    Some("signature_delta") => text(&delta["signature"]),
                    _ => false,
                };
                if content && self.empty_thinking.remove(&index) {
                    self.seen.reasoning += 1;
                }
                true
            }
            _ => true,
        }
    }
}

/// The end of the first complete frame (after its blank line), if any.
fn frame_end(bytes: &[u8]) -> Option<usize> {
    let mut line_start = 0;
    for (at, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            if matches!(&bytes[line_start..at], b"" | b"\r") {
                return Some(at + 1);
            }
            line_start = at + 1;
        }
    }
    None
}

/// A frame's JSON data, if it has any.
fn event(frame: &[u8]) -> Option<Value> {
    let frame = std::str::from_utf8(frame).ok()?;
    let data: Vec<&str> = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|data| data.strip_prefix(' ').unwrap_or(data))
        .collect();
    if data.is_empty() {
        return None;
    }
    serde_json::from_str(&data.join("\n")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sse(events: &[Value], line_end: &str) -> String {
        events
            .iter()
            .map(|event| {
                format!(
                    "event: {}{line_end}data: {event}{line_end}{line_end}",
                    event["type"].as_str().unwrap()
                )
            })
            .collect()
    }

    fn start(index: u64, block: Value) -> Value {
        json!({ "type": "content_block_start", "index": index, "content_block": block })
    }

    fn delta(index: u64, delta: Value) -> Value {
        json!({ "type": "content_block_delta", "index": index, "delta": delta })
    }

    fn stop(index: u64) -> Value {
        json!({ "type": "content_block_stop", "index": index })
    }

    /// A reply declined after thinking, text and a tool call, continued by
    /// the fallback model: the frames to keep, and the `fallback` block's.
    fn declined_then_continued() -> (Vec<Value>, Vec<Value>) {
        let marker = vec![
            start(
                3,
                json!({ "type": "fallback", "from": { "model": "claude-opus-5" }, "to": { "model": "claude-opus-4-8" } }),
            ),
            stop(3),
        ];
        let before = vec![
            json!({ "type": "message_start", "message": { "id": "msg_1", "type": "message", "role": "assistant", "content": [], "model": "claude-opus-5", "usage": { "input_tokens": 5, "output_tokens": 1 } } }),
            start(
                0,
                json!({ "type": "thinking", "thinking": "", "signature": "" }),
            ),
            delta(
                0,
                json!({ "type": "thinking_delta", "thinking": "Declined thought." }),
            ),
            delta(
                0,
                json!({ "type": "signature_delta", "signature": "sig-0" }),
            ),
            stop(0),
            // Thinking with nothing in it: rig makes no part of it.
            start(
                1,
                json!({ "type": "thinking", "thinking": "", "signature": "" }),
            ),
            stop(1),
            start(
                2,
                json!({ "type": "tool_use", "id": "toolu_1", "name": "read_model", "input": {} }),
            ),
            delta(
                2,
                json!({ "type": "input_json_delta", "partial_json": "{\"na" }),
            ),
            stop(2),
        ];
        let after = vec![
            start(4, json!({ "type": "text", "text": "" })),
            delta(4, json!({ "type": "text_delta", "text": "Done." })),
            stop(4),
            json!({ "type": "message_delta", "delta": { "stop_reason": "end_turn", "stop_sequence": null }, "usage": { "output_tokens": 9 } }),
            json!({ "type": "message_stop" }),
        ];
        let all = [before.clone(), marker, after.clone()].concat();
        (all, [before, after].concat())
    }

    #[test]
    fn the_fallback_block_is_removed_in_any_chunking() {
        let (all, kept) = declined_then_continued();
        for line_end in ["\n", "\r\n"] {
            let input = sse(&all, line_end);
            let expected = sse(&kept, line_end);
            for size in 1..=input.len().min(97) {
                let mut filter = Filter::default();
                let mut output = Vec::new();
                for chunk in input.as_bytes().chunks(size) {
                    output.extend(filter.push(chunk));
                }
                output.extend(filter.finish());
                assert_eq!(
                    String::from_utf8(output).unwrap(),
                    expected,
                    "chunks of {size}"
                );
                assert_eq!(
                    filter.switch,
                    Some(Switch {
                        reasoning: 1,
                        tool_calls: 1
                    }),
                    "chunks of {size}"
                );
            }
        }
    }

    #[test]
    fn a_stream_without_a_switch_passes_unchanged() {
        let (_, kept) = declined_then_continued();
        let input = sse(&kept, "\n");
        let mut filter = Filter::default();
        let mut output = filter.push(input.as_bytes());
        output.extend(filter.finish());
        assert_eq!(output, input.as_bytes());
        assert_eq!(filter.switch, None);
    }

    #[test]
    fn a_switch_before_any_output_declines_nothing() {
        let events = [
            start(
                0,
                json!({ "type": "fallback", "from": { "model": "claude-opus-5" }, "to": { "model": "claude-opus-4-8" } }),
            ),
            stop(0),
            start(1, json!({ "type": "text", "text": "" })),
        ];
        let mut filter = Filter::default();
        let output = filter.push(sse(&events, "\n").as_bytes());
        assert_eq!(String::from_utf8(output).unwrap(), sse(&events[2..], "\n"));
        assert_eq!(filter.switch, Some(Switch::default()));
    }

    #[test]
    fn frames_without_json_and_an_unfinished_last_frame_pass_unchanged() {
        let mut filter = Filter::default();
        let input = ": keep-alive\n\nevent: ping\ndata: {\"type\": \"ping\"}\n\ndata: {\"type\"";
        let mut output = filter.push(input.as_bytes());
        output.extend(filter.finish());
        assert_eq!(output, input.as_bytes());
    }
}
