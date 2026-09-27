//! The Assistant's [`Model`] over the provider layer (`agq-providers`, on
//! rig): every provider but the hand-written Claude client, which W5.7
//! retires.
//!
//! The turn sees a reply as Claude API content blocks (conversation format 2
//! stores neutral parts; `conversation::blocks_from_parts` converts). This
//! module translates both ways: `text` and `tool_use` blocks map directly;
//! Anthropic's reasoning comes as `thinking` and `redacted_thinking` blocks,
//! another provider's as a `reasoning` block that names its provider, and
//! either is sent back only to the model that wrote it, since providers tie
//! reasoning to their own models (DeepSeek refuses a tool conversation
//! without it).

use crate::model::{Model, ModelError, Reply, Request, StreamEvent, Usage};
use agq_providers::{
    AssistantPart, ChatRequest, ErrorKind, Event, Message, ModelRef, Providers, Reasoning,
    StopReason, Tool, UserPart,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Upper limit for reasoning and reply together, per model call; always
/// explicit (§4.7).
pub const MAX_OUTPUT_TOKENS: u64 = 64_000;

/// How long a wait for the next event lasts before the stop flag is checked
/// again.
const POLL: Duration = Duration::from_millis(20);

/// A model reached through `agq-providers`.
pub struct ProviderModel {
    pub model: ModelRef,
    pub effort: Option<String>,
    /// Upper limit for reasoning and reply together, per call.
    pub max_output_tokens: u64,
    providers: Providers,
}

impl ProviderModel {
    pub fn new(model: ModelRef, effort: Option<String>) -> ProviderModel {
        ProviderModel::with_providers(model, effort, Providers::new())
    }

    /// With explicit keys or endpoints (tests, local stand-ins).
    pub fn with_providers(
        model: ModelRef,
        effort: Option<String>,
        providers: Providers,
    ) -> ProviderModel {
        ProviderModel {
            model,
            effort,
            max_output_tokens: MAX_OUTPUT_TOKENS,
            providers,
        }
    }

    /// The chat request for a turn's request.
    pub fn chat_request(&self, request: &Request) -> ChatRequest {
        ChatRequest {
            model: self.model.clone(),
            effort: self.effort.clone(),
            max_output_tokens: self.max_output_tokens,
            system: request.system.clone(),
            tools: tools(&request.tools),
            messages: messages(&request.messages, &self.model),
        }
    }
}

impl Model for ProviderModel {
    fn model(&self) -> Option<ModelRef> {
        Some(self.model.clone())
    }

    fn send(
        &mut self,
        request: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        let mut call = self.providers.chat(self.chat_request(request));
        loop {
            if stop.load(Ordering::SeqCst) {
                call.cancel();
                return Err(ModelError::Stopped);
            }
            let Some(event) = call.next_event(POLL) else {
                continue;
            };
            match event {
                Event::Text(text) => on_event(StreamEvent::Text(text)),
                Event::Thinking(text) => on_event(StreamEvent::Thinking(text)),
                Event::ToolCallStarted { stream_id, name } => {
                    on_event(StreamEvent::ToolCallStarted {
                        id: stream_id,
                        name,
                    })
                }
                Event::ToolInput { stream_id, json } => on_event(StreamEvent::ToolInput {
                    id: stream_id,
                    json,
                }),
                Event::ToolCallId { stream_id, id } => {
                    on_event(StreamEvent::ToolCallId { stream_id, id })
                }
                Event::Usage(usage) => on_event(StreamEvent::Usage(Usage {
                    input_tokens: usage.input_tokens,
                    cache_creation_input_tokens: usage.cache_write_tokens,
                    cache_read_input_tokens: usage.cache_read_tokens,
                    output_tokens: usage.output_tokens,
                })),
                Event::Finished(Ok(reply)) => return Ok(self.reply(reply)),
                Event::Finished(Err(error)) => {
                    return Err(
                        if error.kind == ErrorKind::Cancelled && stop.load(Ordering::SeqCst) {
                            ModelError::Stopped
                        } else {
                            ModelError::Failed(error.message)
                        },
                    );
                }
            }
        }
    }
}

impl ProviderModel {
    /// A provider's reply as content blocks for the conversation.
    fn reply(&self, reply: agq_providers::Reply) -> Reply {
        let content = reply
            .content
            .into_iter()
            .map(|part| match part {
                AssistantPart::Text { text } => json!({ "type": "text", "text": text }),
                AssistantPart::Reasoning(reasoning) => json!({
                    "type": "reasoning",
                    "provider": self.model.provider.id(),
                    "model": self.model.model,
                    "text": reasoning.text(),
                    "reasoning": reasoning,
                }),
                AssistantPart::ToolCall { id, name, input } => {
                    json!({ "type": "tool_use", "id": id, "name": name, "input": input })
                }
            })
            .collect();
        let stop_reason = match reply.stop {
            StopReason::EndTurn => "end_turn".to_string(),
            StopReason::ToolUse => "tool_use".to_string(),
            StopReason::MaxTokens => "max_tokens".to_string(),
            StopReason::Refusal => "refusal".to_string(),
            StopReason::Other(other) => other,
        };
        Reply {
            content,
            stop_reason,
        }
    }
}

fn tools(definitions: &Value) -> Vec<Tool> {
    definitions
        .as_array()
        .into_iter()
        .flatten()
        .map(|tool| Tool {
            name: tool["name"].as_str().unwrap_or_default().to_string(),
            description: tool["description"].as_str().unwrap_or_default().to_string(),
            input_schema: tool["input_schema"].clone(),
        })
        .collect()
}

/// Claude API messages (the stored format) as provider-neutral messages.
fn messages(messages: &[Value], model: &ModelRef) -> Vec<Message> {
    // Tool results need the tool's name; the call before them has it.
    let mut names: HashMap<String, String> = HashMap::new();
    let mut converted = Vec::new();
    for message in messages {
        let blocks = message["content"].as_array().cloned().unwrap_or_default();
        if message["role"] == "assistant" {
            let parts: Vec<AssistantPart> = blocks
                .iter()
                .filter_map(|block| assistant_part(block, model))
                .collect();
            for part in &parts {
                if let AssistantPart::ToolCall { id, name, .. } = part {
                    names.insert(id.clone(), name.clone());
                }
            }
            if !parts.is_empty() {
                converted.push(Message::Assistant(parts));
            }
        } else {
            let parts: Vec<UserPart> = blocks
                .iter()
                .filter_map(|block| match block["type"].as_str() {
                    Some("text") => Some(UserPart::Text {
                        text: block["text"].as_str().unwrap_or_default().to_string(),
                    }),
                    Some("tool_result") => {
                        let id = block["tool_use_id"].as_str().unwrap_or_default();
                        Some(UserPart::ToolResult {
                            call_id: id.to_string(),
                            name: names.get(id).cloned().unwrap_or_default(),
                            content: match &block["content"] {
                                Value::String(text) => text.clone(),
                                other => other.to_string(),
                            },
                            is_error: block["is_error"].as_bool().unwrap_or(false),
                        })
                    }
                    _ => None,
                })
                .collect();
            if !parts.is_empty() {
                converted.push(Message::User(parts));
            }
        }
    }
    converted
}

fn assistant_part(block: &Value, model: &ModelRef) -> Option<AssistantPart> {
    let text = |field: &str| block[field].as_str().unwrap_or_default().to_string();
    match block["type"].as_str()? {
        "text" => Some(AssistantPart::Text { text: text("text") }),
        "tool_use" => Some(AssistantPart::ToolCall {
            id: text("id"),
            name: text("name"),
            input: block["input"].clone(),
        }),
        // Reasoning goes back only to the provider and model that wrote it.
        "reasoning"
            if block["provider"] == model.provider.id() && block["model"] == model.model =>
        {
            serde_json::from_value::<Reasoning>(block["reasoning"].clone())
                .ok()
                .map(AssistantPart::Reasoning)
        }
        // Anthropic's reasoning, as the conversation gives it back to its
        // own model only.
        "thinking" | "redacted_thinking"
            if model.provider == agq_providers::Provider::Anthropic =>
        {
            crate::conversation::parts_from_blocks(std::slice::from_ref(block)).pop()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_providers::{Provider, ReasoningPart};

    #[test]
    fn stored_blocks_become_neutral_messages_with_reasoning_for_its_own_model() {
        let deepseek = ModelRef::new(Provider::DeepSeek, "deepseek-flash");
        let reasoning = Reasoning {
            id: None,
            parts: vec![ReasoningPart::Text {
                text: "Check the store.".into(),
                signature: None,
            }],
        };
        let stored = vec![
            json!({ "role": "user", "content": [{ "type": "text", "text": "Add a store." }] }),
            json!({ "role": "assistant", "content": [
                { "type": "reasoning", "provider": "deepseek", "model": "deepseek-flash", "text": "Check the store.", "reasoning": reasoning },
                { "type": "thinking", "thinking": "Claude's", "signature": "s" },
                { "type": "tool_use", "id": "call_1", "name": "find_elements", "input": { "name": "Store" } },
            ] }),
            json!({ "role": "user", "content": [
                { "type": "tool_result", "tool_use_id": "call_1", "content": "none", "is_error": true },
            ] }),
        ];
        let converted = messages(&stored, &deepseek);
        assert_eq!(
            converted,
            vec![
                Message::User(vec![UserPart::Text {
                    text: "Add a store.".into()
                }]),
                Message::Assistant(vec![
                    AssistantPart::Reasoning(reasoning.clone()),
                    AssistantPart::ToolCall {
                        id: "call_1".into(),
                        name: "find_elements".into(),
                        input: json!({ "name": "Store" }),
                    },
                ]),
                Message::User(vec![UserPart::ToolResult {
                    call_id: "call_1".into(),
                    name: "find_elements".into(),
                    content: "none".into(),
                    is_error: true,
                }]),
            ]
        );
        // Another model does not get DeepSeek's reasoning.
        let other = ModelRef::new(Provider::OpenRouter, "deepseek/deepseek-flash");
        assert!(
            matches!(&messages(&stored, &other)[1], Message::Assistant(parts) if parts.len() == 1)
        );
    }

    #[test]
    fn anthropic_reasoning_goes_back_to_anthropic_with_its_signatures() {
        use crate::conversation::{Conversation, Entry};
        let claude = ModelRef::new(Provider::Anthropic, "claude-sonnet-5");
        let blocks = [
            json!({ "type": "thinking", "thinking": "Read first.", "signature": "sig-1" }),
            json!({ "type": "redacted_thinking", "data": "opaque" }),
            json!({ "type": "tool_use", "id": "t1", "name": "read_model", "input": {} }),
        ];
        let conversation = Conversation {
            transcript: 0,
            entries: vec![
                Entry::Operator {
                    text: "Build it".into(),
                },
                Entry::reply(Some(claude.clone()), &blocks),
            ],
        };
        // The stored reply goes back as the blocks Anthropic returned.
        let stored = conversation.messages_for(Some(&claude));
        assert_eq!(stored[1]["content"].as_array().unwrap().as_slice(), &blocks);
        // And the provider layer keeps both reasoning parts, signature included.
        let converted = messages(&stored, &claude);
        let Message::Assistant(parts) = &converted[1] else {
            panic!("{converted:?}")
        };
        assert_eq!(parts.len(), 3, "{parts:?}");
        assert!(matches!(
            &parts[0],
            AssistantPart::Reasoning(Reasoning { parts, .. })
                if matches!(&parts[0], ReasoningPart::Text { signature: Some(s), .. } if s == "sig-1")
        ));
        assert!(matches!(
            &parts[1],
            AssistantPart::Reasoning(Reasoning { parts, .. })
                if matches!(&parts[0], ReasoningPart::Redacted { data } if data == "opaque")
        ));
        // Another model gets none of it.
        let deepseek = ModelRef::new(Provider::DeepSeek, "deepseek-flash");
        let other = conversation.messages_for(Some(&deepseek));
        assert_eq!(other[1]["content"].as_array().unwrap().len(), 1);
    }
}
