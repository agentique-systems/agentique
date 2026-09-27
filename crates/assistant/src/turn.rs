//! One turn of the Conversation: the Assistant answers the Operator's latest
//! message, calling tools until it has finished.
//!
//! [`run`] is the tool-use loop. It sends the conversation to the model and
//! adds the reply; while the reply asks for tools, it checks each call's
//! input, hands valid calls to the caller's executor, adds all results as one
//! entry and asks the model again. The executor is where tool calls take
//! effect: the Studio prepares the call with [`crate::tools::prepare`] and
//! applies a change through its project like any Operator edit, asking the
//! Operator about locked elements and questions. The loop itself never
//! changes the System State.
//!
//! [`BackgroundTurn`] runs a turn on a background thread for the Studio: tool
//! calls come back to the UI thread as events, so every change to the System
//! State happens there.

use crate::conversation::{Conversation, Entry, ToolResult};
use crate::model::{Model, ModelError, Request, StreamEvent};
use crate::{skills, tools};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// Model calls in one turn before the turn pauses and waits for the
/// Operator, so a confused model cannot run on without end.
pub const MAX_MODEL_CALLS: usize = 40;

const STOPPED: &str = "Stopped by the Operator. Changes made so far stay and can be undone.";

/// How often a background turn waiting for the UI thread checks the stop
/// flag.
const POLL: Duration = Duration::from_millis(20);

/// A tool call whose input matches the tool's schema.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// What happens during a turn, for live display.
#[derive(Clone, Debug, PartialEq)]
pub enum TurnEvent {
    /// Model output as it streams in.
    Stream(StreamEvent),
    /// A tool call has ended, run or not. Every
    /// [`StreamEvent::ToolCallStarted`] is followed by one. The result is
    /// also in the next [`Entry::ToolResults`], unless the reply the call
    /// belonged to was discarded (stopped, failed or refused).
    ToolFinished(ToolResult),
    /// An entry was added to the conversation.
    Entry(Entry),
}

/// Runs one turn: answers the conversation, whose last entry is normally the
/// Operator's message. Every entry the turn adds is also reported as
/// [`TurnEvent::Entry`]. The turn ends when the model has finished, when the
/// Operator stops it (checked before every model call and every tool call),
/// or on an error; then a [`Entry::Notice`] says what happened. The
/// conversation is always left well formed for the next request.
///
/// `execute` carries out a checked call and returns its result, made with
/// [`ToolResult::answer`], [`ToolResult::error`], [`ToolResult::applied`] or
/// [`ToolResult::rejected`]; the turn fills in its `tool_use_id`.
pub fn run(
    model: &mut dyn Model,
    conversation: &mut Conversation,
    execute: &mut dyn FnMut(&ToolCall) -> ToolResult,
    on_event: &mut dyn FnMut(TurnEvent),
    stop: &AtomicBool,
) {
    for _ in 0..MAX_MODEL_CALLS {
        if stop.load(Ordering::SeqCst) {
            add(conversation, on_event, notice(STOPPED));
            return;
        }
        let messages = conversation.api_messages();
        if messages
            .last()
            .is_none_or(|message| message["role"] != "user")
        {
            add(
                conversation,
                on_event,
                notice("Write a message for the Assistant to answer."),
            );
            return;
        }
        let request = Request {
            system: skills::system_prompt().to_string(),
            tools: tools::definitions(),
            messages,
        };
        let mut partial = String::new();
        let mut started = Vec::new();
        let sent = model.send(
            &request,
            &mut |event| {
                match &event {
                    StreamEvent::Text(text) => partial.push_str(text),
                    StreamEvent::ToolCallStarted { id, .. } => started.push(id.clone()),
                    StreamEvent::ToolCallId { stream_id, id } => {
                        for started in started.iter_mut().filter(|s| *s == stream_id) {
                            *started = id.clone();
                        }
                    }
                    _ => {}
                }
                on_event(TurnEvent::Stream(event));
            },
            stop,
        );
        let mut reply = match sent {
            Ok(reply) => reply,
            Err(error) => {
                let why = match error {
                    ModelError::Stopped => "stopped by the Operator",
                    _ => "the reply failed",
                };
                discard(&started, why, on_event);
                // Text already shown stays in the conversation.
                if !partial.trim().is_empty() {
                    let text = json!({ "type": "text", "text": partial.trim_end() });
                    add(
                        conversation,
                        on_event,
                        Entry::Assistant {
                            content: vec![text],
                        },
                    );
                }
                let text = match error {
                    ModelError::Stopped => STOPPED.to_string(),
                    other => other.to_string(),
                };
                add(conversation, on_event, notice(&text));
                return;
            }
        };
        if reply.stop_reason == "refusal" {
            // A declined reply may be cut off anywhere; none of it is kept.
            discard(&started, "the reply was declined", on_event);
            add(
                conversation,
                on_event,
                notice(
                    "Claude declined to continue with this request. Nothing more was changed; rephrase the request or continue by hand.",
                ),
            );
            return;
        }
        let unreadable = take_unreadable_inputs(&mut reply.content);
        let calls = tool_calls(&reply.content);
        // Calls the API dropped from the reply (after a switch to a fallback
        // model) end here.
        let dropped: Vec<String> = started
            .into_iter()
            .filter(|id| !calls.iter().any(|call| &call.id == id))
            .collect();
        discard(
            &dropped,
            "the reply was continued by another model",
            on_event,
        );
        // The API does not accept an empty assistant message back.
        if !reply.content.is_empty() {
            add(
                conversation,
                on_event,
                Entry::Assistant {
                    content: reply.content,
                },
            );
        }
        if reply.stop_reason != "tool_use" || calls.is_empty() {
            // Tool calls in a reply that ended otherwise may be incomplete.
            if !calls.is_empty() {
                let results = calls
                    .iter()
                    .map(|call| {
                        let result = not_run(
                            &call.id,
                            "Not run: the reply ended before this call was complete.",
                        );
                        on_event(TurnEvent::ToolFinished(result.clone()));
                        result
                    })
                    .collect();
                add(conversation, on_event, Entry::ToolResults { results });
            }
            if let Some(text) = ending(&reply.stop_reason) {
                add(conversation, on_event, notice(&text));
            }
            return;
        }
        let mut results = Vec::new();
        for call in &calls {
            let result = if stop.load(Ordering::SeqCst) {
                not_run(&call.id, "Not run: stopped by the Operator.")
            } else if let Some(raw) = unreadable.get(&call.id) {
                not_run(
                    &call.id,
                    &format!(
                        "Not run: the input is not a valid JSON object. The input received was: {raw}"
                    ),
                )
            } else if let Err(message) = tools::check_input(&call.name, &call.input) {
                not_run(&call.id, &format!("Not run: {message}."))
            } else {
                ToolResult {
                    tool_use_id: call.id.clone(),
                    ..execute(call)
                }
            };
            on_event(TurnEvent::ToolFinished(result.clone()));
            results.push(result);
        }
        add(conversation, on_event, Entry::ToolResults { results });
    }
    add(
        conversation,
        on_event,
        notice(&format!(
            "Paused after {MAX_MODEL_CALLS} steps in one turn. Send a message to let the Assistant continue."
        )),
    );
}

/// What the Operator is told when a reply ends without asking for tools;
/// nothing for a normal end.
fn ending(stop_reason: &str) -> Option<String> {
    Some(match stop_reason {
        "end_turn" | "stop_sequence" | "tool_use" => return None,
        "max_tokens" => "The reply reached the output limit and was cut off. Ask the Assistant to continue, in smaller steps.".to_string(),
        "model_context_window_exceeded" => "The conversation is too long for the model. Start a new conversation; the architecture is unaffected.".to_string(),
        "pause_turn" => "The reply paused before it was complete. Send a message to let the Assistant continue.".to_string(),
        other => format!("The reply ended unexpectedly ({other})."),
    })
}

fn add(conversation: &mut Conversation, on_event: &mut dyn FnMut(TurnEvent), entry: Entry) {
    conversation.entries.push(entry.clone());
    on_event(TurnEvent::Entry(entry));
}

fn notice(text: &str) -> Entry {
    Entry::Notice {
        text: text.to_string(),
    }
}

fn not_run(id: &str, content: &str) -> ToolResult {
    ToolResult {
        tool_use_id: id.to_string(),
        ..ToolResult::error(content)
    }
}

/// Ends the cards of tool calls whose reply is not kept.
fn discard(ids: &[String], why: &str, on_event: &mut dyn FnMut(TurnEvent)) {
    for id in ids {
        let result = not_run(id, &format!("Not run: {why}."));
        on_event(TurnEvent::ToolFinished(result));
    }
}

/// Replaces tool inputs that are not JSON objects (raw text that could not
/// be read) with an empty object, so the reply can be sent back, and returns
/// the raw text by call id.
fn take_unreadable_inputs(content: &mut [Value]) -> BTreeMap<String, String> {
    let mut unreadable = BTreeMap::new();
    for block in content.iter_mut() {
        if block["type"] == "tool_use" && !block["input"].is_object() {
            let raw = match &block["input"] {
                Value::String(raw) => raw.clone(),
                other => other.to_string(),
            };
            let id = block["id"].as_str().unwrap_or_default().to_string();
            unreadable.insert(id, raw);
            block["input"] = json!({});
        }
    }
    unreadable
}

fn tool_calls(content: &[Value]) -> Vec<ToolCall> {
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    content
        .iter()
        .filter(|block| block["type"] == "tool_use")
        .map(|block| ToolCall {
            id: text(&block["id"]),
            name: text(&block["name"]),
            input: block["input"].clone(),
        })
        .collect()
}

/// A turn running on a background thread.
///
/// The Studio starts a turn with the conversation (the Operator's message
/// last) and polls [`next_event`](Self::next_event) every frame. Entries
/// arrive as [`TurnEvent::Entry`] for the Studio to add to its own copy of
/// the conversation. A [`BackgroundEvent::ToolCall`] must be carried out on
/// the UI thread, where the project lives, and answered on its `reply`
/// channel; the turn waits for it, unless the Operator stops the turn.
/// [`stop`](Self::stop) takes effect at once.
pub struct BackgroundTurn {
    events: Receiver<BackgroundEvent>,
    stop: Arc<AtomicBool>,
}

/// What a background turn reports to the UI thread.
pub enum BackgroundEvent {
    /// Streamed output, a finished tool call, or a new conversation entry.
    Turn(TurnEvent),
    /// A tool call to carry out on the UI thread; send the result on
    /// `reply`. After a stop the turn no longer waits for it, and a question
    /// or confirmation still open for it can be closed.
    ToolCall {
        call: ToolCall,
        reply: Sender<ToolResult>,
    },
    /// The turn is over; the last entry says how it ended if not normally.
    Finished,
}

impl BackgroundTurn {
    /// Starts a turn on a new thread with its own copy of the conversation.
    pub fn start(
        mut model: Box<dyn Model + Send>,
        mut conversation: Conversation,
    ) -> BackgroundTurn {
        let (sender, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::spawn(move || {
            let calls = sender.clone();
            let mut execute = |call: &ToolCall| {
                let (reply, result) = mpsc::channel();
                let call = call.clone();
                if calls
                    .send(BackgroundEvent::ToolCall { call, reply })
                    .is_err()
                {
                    return ToolResult::error("Not run: the Studio is closed.");
                }
                loop {
                    match result.recv_timeout(POLL) {
                        Ok(result) => return result,
                        Err(RecvTimeoutError::Timeout) if flag.load(Ordering::SeqCst) => {
                            return ToolResult::error("Not run: stopped by the Operator.");
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            return ToolResult::error(
                                "Not run: the Studio did not carry out this call.",
                            );
                        }
                    }
                }
            };
            let turn_events = sender.clone();
            let mut on_event = |event| {
                let _ = turn_events.send(BackgroundEvent::Turn(event));
            };
            let finished = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(
                    model.as_mut(),
                    &mut conversation,
                    &mut execute,
                    &mut on_event,
                    &flag,
                )
            }));
            if finished.is_err() {
                let entry = notice(
                    "The Assistant failed unexpectedly. The model is as the last change left it.",
                );
                let _ = sender.send(BackgroundEvent::Turn(TurnEvent::Entry(entry)));
            }
            let _ = sender.send(BackgroundEvent::Finished);
        });
        BackgroundTurn { events, stop }
    }

    /// Stops the turn: no further model call or tool call starts, a reply
    /// that is streaming in is abandoned, and a tool call waiting for the UI
    /// thread is given up.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// The next event, if one is waiting. Never blocks.
    pub fn next_event(&self) -> Option<BackgroundEvent> {
        self.events.try_recv().ok()
    }
}

impl Drop for BackgroundTurn {
    /// A turn nobody listens to stops.
    fn drop(&mut self) {
        self.stop();
    }
}
