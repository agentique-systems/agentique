//! Anthropic's server-side refusal fallbacks through rig (C-27, Q-18): a thin
//! adapter until rig reads the `fallback` content block itself.
//!
//! With `fallbacks: "default"` and the [`BETA`] header, a request the model's
//! safety classifiers decline is continued on another model in the same
//! stream; a content block of type `fallback` marks the switch. rig 0.43.0
//! does not know that block type and ends the stream with an error, so rig's
//! Anthropic model is given [`FallbackTransport`]: rig's own transport,
//! except that the streamed frames pass through a [`Filter`] that removes
//! the `fallback` block's frames before rig decodes them and records the
//! switch. rig splits the event stream into frames itself.
//!
//! After a switch, the declined model's reasoning and tool calls before it
//! are not part of the reply; its text stays, as the fallback model continued
//! from it (Anthropic's rule for sending a fallback turn back, and what
//! `after_fallback` does in the Assistant's hand-written client).
//!
//! The same transport keeps a tool call whose input is not JSON from ending
//! the reply: rig 0.43 ends a stream at such a call, which would lose what
//! follows (after a switch, the whole fallback reply). A tool call's frames
//! are held until the call closes (rig hands a call over whole anyway); if
//! its input does not read as JSON, rig is given it as one JSON string of
//! the raw text, which the turn answers with an error (R-22), as before.

use crate::AssistantPart;
use futures::StreamExt;
use rig_core::driver::{Exchange, Opening, Transport};
use rig_core::http_client::DynHttpClient;
use rig_core::providers::anthropic;
use rig_core::wire::{Encoded, WireFrame};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
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

/// rig's transport for Anthropic's Messages wire, with the `fallback` block's
/// frames removed from streamed replies. Clones share what the latest
/// streamed reply said about a switch, so each call builds its own (as
/// `chat::call` does); one shared between calls would mix them up.
#[derive(Clone, Debug)]
pub(crate) struct FallbackTransport {
    inner: DynHttpClient,
    switch: Arc<Mutex<Option<Switch>>>,
}

impl FallbackTransport {
    pub(crate) fn new(inner: DynHttpClient) -> FallbackTransport {
        FallbackTransport {
            inner,
            switch: Arc::default(),
        }
    }

    /// The last switch in the latest streamed reply, if one happened.
    pub(crate) fn switch(&self) -> Option<Switch> {
        *self.switch.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Transport<anthropic::Messages> for FallbackTransport {
    fn send(&self, payload: Encoded, exchange: Exchange) -> Opening<WireFrame> {
        let opening =
            <DynHttpClient as Transport<anthropic::Messages>>::send(&self.inner, payload, exchange);
        let switch = self.switch.clone();
        Opening::new(async move {
            let opened = opening.await?;
            *switch.lock().unwrap_or_else(PoisonError::into_inner) = None;
            Ok(opened.map_frames(move |frames| {
                let mut filter = Filter::default();
                frames
                    .map(move |frame| {
                        let out = match frame {
                            Ok(frame) => filter.pass(frame).into_iter().map(Ok).collect(),
                            // An error is rig's to report.
                            Err(error) => vec![Err(error)],
                        };
                        if filter.switch.is_some() {
                            *switch.lock().unwrap_or_else(PoisonError::into_inner) = filter.switch;
                        }
                        futures::stream::iter(out)
                    })
                    .flatten()
            }))
        })
    }
}

/// Removes the `fallback` block's frames from an Anthropic event stream,
/// counts what came before the last switch, and holds each tool call's
/// frames until it closes.
#[derive(Debug, Default)]
struct Filter {
    /// Tool calls not closed yet, by index: their frames and input so far.
    calls: BTreeMap<u64, (Vec<WireFrame>, String)>,
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
    /// The frames to pass on to rig for one frame (one event's JSON): none
    /// while a tool call is held or for the `fallback` block, the held
    /// call's frames when it closes, the frame itself otherwise.
    fn pass(&mut self, frame: WireFrame) -> Vec<WireFrame> {
        let Ok(event) = serde_json::from_str::<Value>(&frame.as_str()) else {
            return vec![frame];
        };
        let index = event["index"].as_u64();
        let kind = event["type"].as_str();
        if let Some(index) = index {
            if kind == Some("content_block_start")
                && event["content_block"]["type"].as_str() == Some("tool_use")
            {
                self.keep_event(&event);
                self.calls.insert(index, (vec![frame], String::new()));
                return Vec::new();
            }
            if let Some((frames, input)) = self.calls.get_mut(&index) {
                if kind == Some("content_block_delta") {
                    if let Some(json) = event["delta"]["partial_json"].as_str() {
                        input.push_str(json);
                    }
                    frames.push(frame);
                    return Vec::new();
                }
                if kind == Some("content_block_stop") {
                    let (mut frames, input) = self.calls.remove(&index).expect("held");
                    if !input.trim().is_empty() && serde_json::from_str::<Value>(&input).is_err() {
                        // The raw text, as one JSON string.
                        frames.truncate(1);
                        let repaired = json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": {
                                "type": "input_json_delta",
                                "partial_json": serde_json::to_string(&input).expect("a string is JSON"),
                            }
                        });
                        frames.push(WireFrame::Text(repaired.to_string()));
                    }
                    frames.push(frame);
                    return frames;
                }
            }
        }
        if self.keep_event(&event) {
            vec![frame]
        } else {
            Vec::new()
        }
    }

    fn keep_event(&mut self, event: &Value) -> bool {
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
            Some("content_block_stop") => {
                // A block's index is not reused after its stop.
                if self.fallbacks.remove(&index) {
                    return false;
                }
                self.empty_thinking.remove(&index);
                true
            }
            Some("content_block_delta") => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    /// the fallback model: the events to keep, and the `fallback` block's.
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
    fn the_fallback_block_is_removed_and_the_switch_counted() {
        let (all, kept) = declined_then_continued();
        let mut filter = Filter::default();
        let output: Vec<Value> = all
            .into_iter()
            .filter(|event| filter.keep_event(event))
            .collect();
        assert_eq!(output, kept);
        assert_eq!(
            filter.switch,
            Some(Switch {
                reasoning: 1,
                tool_calls: 1
            })
        );
    }

    #[test]
    fn a_stream_without_a_switch_passes_unchanged() {
        let (_, kept) = declined_then_continued();
        let mut filter = Filter::default();
        assert!(kept.iter().all(|event| filter.keep_event(event)));
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
        let kept: Vec<bool> = events.iter().map(|e| filter.keep_event(e)).collect();
        assert_eq!(kept, [false, false, true]);
        assert_eq!(filter.switch, Some(Switch::default()));
    }

    #[test]
    fn events_without_an_index_pass_unchanged() {
        let mut filter = Filter::default();
        for event in [
            json!({ "type": "ping" }),
            json!({ "type": "message_stop" }),
            json!("not an object"),
        ] {
            assert!(filter.keep_event(&event));
        }
    }

    fn frames(events: &[Value]) -> Vec<WireFrame> {
        events
            .iter()
            .map(|event| WireFrame::Text(event.to_string()))
            .collect()
    }

    fn events(frames: Vec<WireFrame>) -> Vec<Value> {
        frames
            .iter()
            .map(|frame| serde_json::from_str(&frame.as_str()).unwrap())
            .collect()
    }

    /// A tool call is held until it closes; readable input passes as sent,
    /// unreadable input becomes one JSON string of its raw text.
    #[test]
    fn tool_input_is_held_and_unreadable_input_kept_as_text() {
        let readable = [
            start(
                0,
                json!({ "type": "tool_use", "id": "t1", "name": "f", "input": {} }),
            ),
            delta(
                0,
                json!({ "type": "input_json_delta", "partial_json": "{\"a\"" }),
            ),
            delta(
                0,
                json!({ "type": "input_json_delta", "partial_json": ": 1}" }),
            ),
            stop(0),
        ];
        let mut filter = Filter::default();
        let mut out = Vec::new();
        for (i, frame) in frames(&readable).into_iter().enumerate() {
            let passed = filter.pass(frame);
            if i < 3 {
                assert!(passed.is_empty(), "held until the call closes");
            }
            out.extend(passed);
        }
        assert_eq!(events(out), readable);
        let unreadable = [
            start(
                1,
                json!({ "type": "tool_use", "id": "t2", "name": "f", "input": {} }),
            ),
            delta(
                1,
                json!({ "type": "input_json_delta", "partial_json": "{\"na" }),
            ),
            stop(1),
        ];
        let out: Vec<WireFrame> = frames(&unreadable)
            .into_iter()
            .flat_map(|frame| filter.pass(frame))
            .collect();
        let out = events(out);
        assert_eq!(out.len(), 3);
        assert_eq!(out[1]["delta"]["partial_json"], "\"{\\\"na\"");
        let input: Value =
            serde_json::from_str(out[1]["delta"]["partial_json"].as_str().unwrap()).unwrap();
        assert_eq!(input, Value::String("{\"na".into()));
        assert_eq!(filter.seen.tool_calls, 2);
    }

    #[test]
    fn declined_reasoning_and_calls_are_dropped_and_text_stays() {
        let switch = Switch {
            reasoning: 1,
            tool_calls: 1,
        };
        let mut content = vec![
            AssistantPart::Reasoning(crate::Reasoning::default()),
            AssistantPart::Text {
                text: "kept".into(),
            },
            AssistantPart::ToolCall {
                id: "toolu_1".into(),
                name: "read_model".into(),
                input: json!({}),
            },
            AssistantPart::Reasoning(crate::Reasoning::default()),
            AssistantPart::ToolCall {
                id: "toolu_2".into(),
                name: "read_model".into(),
                input: json!({}),
            },
        ];
        switch.drop_declined(&mut content, &["toolu_1", "toolu_2"]);
        assert_eq!(content.len(), 3);
        assert!(matches!(&content[0], AssistantPart::Text { .. }));
        assert!(matches!(&content[2], AssistantPart::ToolCall { id, .. } if id == "toolu_2"));
    }
}
