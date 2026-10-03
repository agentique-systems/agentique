//! One streamed model call through rig, for every provider (C-34).
//!
//! Every provider goes through the same code: the request is converted into
//! rig's request once, the provider's model is built from data with the key
//! given explicitly (rig reads no environment), and one function streams the
//! reply into [`Event`]s. rig's own agent loop is not used: the Assistant's
//! turn loop holds Agentique's policy (R-21).
//!
//! rig 0.43 hands a tool call over whole when it closes, so a call's input
//! arrives as one [`Event::ToolInput`] (the capability table says tool input
//! is not streamed). Reasoning is sealed to the provider that wrote it; the
//! Assistant sends a model only its own reasoning, so every part is sealed
//! to the provider asked. A tool call whose input is not JSON ends rig's
//! stream; it is kept with its raw text, so the turn answers it with an
//! error (R-22), as before.

use crate::{
    AssistantPart, ChatRequest, Error, ErrorKind, Event, Message, Provider, Reasoning,
    ReasoningPart, Reply, StopReason, Usage, UserPart, capabilities, fallback, runtime,
};
use futures::StreamExt;
use rig_core::DynModel;
use rig_core::completion::{CompletionRequest, FinishReason, ToolDefinition};
use rig_core::driver::Model;
use rig_core::error::ProviderError;
use rig_core::message::{
    self as rig, AssistantContent, CallId, Issuer, ReasoningContent, ToolFunction, ToolName,
    ToolResultContent, UserContent,
};
use rig_core::operation::Completion;
use rig_core::providers::{anthropic, openai};
use rig_core::streaming::{Item, StreamEvent};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Attempts after the first for rate limits, server errors and connection
/// failures, before anything streamed (Stage 3's rule, R-22).
const RETRIES: u32 = 2;
/// The longest wait a provider may ask for before an automatic retry.
const MAX_RETRY_WAIT: u64 = 10;

pub(crate) async fn run(
    request: ChatRequest,
    key: Option<String>,
    endpoint: Option<String>,
    sender: Sender<Event>,
) {
    let result = call(&request, key, endpoint, &sender).await;
    let _ = sender.send(Event::Finished(result));
}

async fn call(
    request: &ChatRequest,
    key: Option<String>,
    endpoint: Option<String>,
    sender: &Sender<Event>,
) -> Result<Reply, Error> {
    let provider = request.model.provider;
    if !capabilities(&request.model).tools {
        return Err(Error {
            kind: ErrorKind::Rejected,
            message: format!(
                "{} answers typed questions and cannot hold a conversation, so it is not a model for the Assistant.",
                provider.name()
            ),
        });
    }
    let Some(key) = key else {
        return Err(Error {
            kind: ErrorKind::MissingKey,
            message: format!(
                "No {} key is set. Add one in Settings (Ctrl+,) or set {}; the Surface keeps working without it.",
                provider.name(),
                provider.key_variable()
            ),
        });
    };
    let rig_request = rig_request(request)?;
    let (model, fallbacks) = model(request, key, endpoint)?;
    stream(&model, rig_request, request, sender, fallbacks.as_ref()).await
}

/// The provider's model, with the key given explicitly and the endpoint, if
/// one is set; for Anthropic, through the fallback adapter (Q-18), whose
/// beta header, like `fallbacks` in the request, goes only to models that
/// use them (C-27).
fn model(
    request: &ChatRequest,
    key: String,
    endpoint: Option<String>,
) -> Result<(DynModel<Completion>, Option<fallback::FallbackTransport>), Error> {
    let name = request.model.model.clone();
    Ok(match request.model.provider {
        Provider::DeepSeek | Provider::OpenRouter => {
            let dialect = if request.model.provider == Provider::DeepSeek {
                &openai::wire::DEEPSEEK
            } else {
                &openai::wire::OPENROUTER
            };
            let mut config = openai::OpenAIConfig::with_key(dialect, key);
            if let Some(url) = endpoint {
                config = config.with_base_url(url);
            }
            (config.client().chat(name).erase(), None)
        }
        Provider::OpenAi => {
            let mut config = openai::OpenAIConfig::new(key);
            if let Some(url) = endpoint {
                config = config.with_base_url(url);
            }
            (config.client().responses(name).erase(), None)
        }
        Provider::Anthropic => {
            let mut config = anthropic::AnthropicConfig::new(key);
            if capabilities(&request.model).refusal_fallbacks {
                config = config.with_beta(fallback::BETA);
            }
            if let Some(url) = endpoint {
                config = config.with_base_url(url);
            }
            let plain: Model<anthropic::Messages> = config.client().completion(name);
            let transport = fallback::FallbackTransport::new(plain.transport);
            (
                Model::new(plain.wire, transport.clone()).erase(),
                Some(transport),
            )
        }
        // Refused before (no tools); kept as an error, never a panic.
        Provider::TypeSafe => {
            return Err(Error {
                kind: ErrorKind::Rejected,
                message: "TypeSafe AI is not a model for the Assistant.".to_string(),
            });
        }
    })
}

/// The issuer rig seals a provider's reasoning to, and sends back to it
/// only: the wire's own name, or on OpenRouter the upstream model's.
fn issuer(request: &ChatRequest) -> Issuer {
    match request.model.provider {
        Provider::Anthropic => Issuer::from("anthropic"),
        Provider::OpenAi => Issuer::from("openai"),
        Provider::DeepSeek => Issuer::from("deepseek"),
        Provider::OpenRouter => Issuer::from(openai::wire::upstream_reasoning_issuer(
            "openrouter",
            &request.model.model,
        )),
        Provider::TypeSafe => Issuer::from("typesafe"),
    }
}

fn unnamed_tool() -> Error {
    Error {
        kind: ErrorKind::Other,
        message: "A tool call or result without a tool name cannot be sent.".to_string(),
    }
}

/// Converts the request into rig's, with the provider's own parameters for
/// reasoning and effort passed through `additional_params` (R-22).
fn rig_request(request: &ChatRequest) -> Result<CompletionRequest, Error> {
    let issuer = issuer(request);
    let mut history = vec![rig::Message::System {
        content: request.system.clone(),
    }];
    for message in &request.messages {
        history.push(match message {
            Message::User(parts) => rig::Message::User {
                content: parts.iter().map(user_content).collect::<Result<_, _>>()?,
            },
            Message::Assistant(parts) => {
                let mut content: Vec<AssistantContent> = parts
                    .iter()
                    .map(|part| assistant_content(part, &issuer))
                    .collect::<Result<_, _>>()?;
                if needs_reasoning(request, parts) {
                    content.insert(
                        0,
                        AssistantContent::Reasoning(
                            rig::Reasoning::new(NO_REASONING).sealed(issuer.clone()),
                        ),
                    );
                }
                rig::Message::Assistant { id: None, content }
            }
        });
    }
    let tools = request
        .tools
        .iter()
        .map(|tool| ToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        })
        .collect();
    Ok(CompletionRequest {
        model: None,
        chat_history: history,
        documents: Vec::new(),
        tools,
        temperature: None,
        max_tokens: Some(request.max_output_tokens),
        tool_choice: None,
        additional_params: additional_params(request),
        output_schema: None,
        record_telemetry_content: false,
    })
}

/// Stands in for reasoning an earlier assistant turn does not have (a stopped
/// reply, a turn another model wrote).
const NO_REASONING: &str = "(no reasoning was recorded for this turn)";

/// DeepSeek refuses a request with tools when an earlier assistant turn has
/// no `reasoning_content` (§4.8), so every such turn gets a stand-in.
fn needs_reasoning(request: &ChatRequest, parts: &[AssistantPart]) -> bool {
    request.model.provider == Provider::DeepSeek
        && !request.tools.is_empty()
        && !parts.iter().any(|part| {
            matches!(part, AssistantPart::Reasoning(reasoning) if !reasoning.text().trim().is_empty())
        })
}

/// Reasoning and effort settings, per provider (§4.7, §4.8).
fn additional_params(request: &ChatRequest) -> Option<Value> {
    let capabilities = capabilities(&request.model);
    let effort = request
        .effort
        .as_deref()
        .filter(|effort| capabilities.efforts.contains(effort))
        .or(capabilities.default_effort);
    match request.model.provider {
        // Thinking on, effort top-level; forced tool choice is never sent.
        Provider::DeepSeek => Some(match effort {
            Some(effort) => {
                json!({ "thinking": { "type": "enabled" }, "reasoning_effort": effort })
            }
            None => json!({ "thinking": { "type": "enabled" } }),
        }),
        // Adaptive thinking with readable summaries (R-31) and effort; a
        // declined request continues on the model the API recommends (C-27).
        Provider::Anthropic => {
            let mut params = json!({ "thinking": { "type": "adaptive", "display": "summarized" } });
            if let Some(effort) = effort {
                params["output_config"] = json!({ "effort": effort });
            }
            if capabilities.refusal_fallbacks {
                params["fallbacks"] = json!("default");
            }
            Some(params)
        }
        Provider::OpenAi => Some(match effort {
            Some(effort) => json!({ "reasoning": { "effort": effort, "summary": "auto" } }),
            None => json!({ "reasoning": { "summary": "auto" } }),
        }),
        Provider::OpenRouter => effort.map(|effort| json!({ "reasoning": { "effort": effort } })),
        Provider::TypeSafe => None,
    }
}

fn user_content(part: &UserPart) -> Result<UserContent, Error> {
    Ok(match part {
        UserPart::Text { text } => UserContent::Text(rig::Text::new(text.clone())),
        UserPart::ToolResult {
            call_id,
            name,
            content,
            is_error,
        } => {
            // rig's tool result has no error flag; the text says it instead.
            let text = if *is_error {
                format!("Error: {content}")
            } else {
                content.clone()
            };
            UserContent::ToolResult(rig::ToolResult {
                call: CallId::from_wire(call_id.clone()),
                name: ToolName::new(name.clone()).map_err(|_| unnamed_tool())?,
                content: vec![ToolResultContent::Text(rig::Text::new(text))],
            })
        }
    })
}

fn assistant_content(part: &AssistantPart, issuer: &Issuer) -> Result<AssistantContent, Error> {
    Ok(match part {
        AssistantPart::Text { text } => AssistantContent::Text(rig::Text::new(text.clone())),
        AssistantPart::Reasoning(reasoning) => AssistantContent::Reasoning(
            rig::Reasoning {
                id: reasoning.id.clone(),
                content: reasoning
                    .parts
                    .iter()
                    .map(|part| match part {
                        ReasoningPart::Text { text, signature } => ReasoningContent::Text {
                            text: text.clone(),
                            signature: signature.clone(),
                        },
                        ReasoningPart::Summary { text } => ReasoningContent::Summary(text.clone()),
                        ReasoningPart::Encrypted { data } => {
                            ReasoningContent::Encrypted(data.clone())
                        }
                        ReasoningPart::Redacted { data } => {
                            ReasoningContent::Redacted { data: data.clone() }
                        }
                    })
                    .collect(),
            }
            .sealed(issuer.clone()),
        ),
        AssistantPart::ToolCall { id, name, input } => {
            AssistantContent::ToolCall(rig::ToolCall::from_wire(
                id.clone(),
                ToolFunction::new(
                    ToolName::new(name.clone()).map_err(|_| unnamed_tool())?,
                    input.clone(),
                ),
            ))
        }
    })
}

/// Streams one call, retrying a failure (rate limit, server error, lost
/// connection) twice as long as nothing has been streamed yet. `fallbacks`
/// is the transport of an Anthropic call, which reports a switch to a
/// fallback model.
async fn stream(
    model: &DynModel<Completion>,
    rig_request: CompletionRequest,
    request: &ChatRequest,
    sender: &Sender<Event>,
    fallbacks: Option<&fallback::FallbackTransport>,
) -> Result<Reply, Error> {
    let mut attempt = 0;
    loop {
        let mut streamed = false;
        let once = stream_once(
            model,
            rig_request.clone(),
            request,
            sender,
            fallbacks,
            &mut streamed,
        );
        match once.await {
            Ok(reply) => return Ok(reply),
            Err(failure) => match failure.retry_after {
                Some(seconds) if !streamed && attempt < RETRIES && seconds <= MAX_RETRY_WAIT => {
                    attempt += 1;
                    let wait = seconds.max(u64::from(attempt));
                    runtime::sleep(Duration::from_secs(wait)).await;
                }
                _ => return Err(failure.error),
            },
        }
    }
}

async fn stream_once(
    model: &DynModel<Completion>,
    rig_request: CompletionRequest,
    request: &ChatRequest,
    sender: &Sender<Event>,
    fallbacks: Option<&fallback::FallbackTransport>,
    streamed: &mut bool,
) -> Result<Reply, Failure> {
    let provider = request.model.provider;
    let failed = |error: &ProviderError| failure(provider, &request.model.model, error);
    let mut response = model.stream(rig_request).map_err(|error| failed(&error))?;
    // The provider's ids of the tool calls, in the order they closed.
    let mut calls_started: Vec<String> = Vec::new();
    let mut thinking_shown = BTreeSet::new();
    let mut malformed = None;
    let mut send = |event| {
        *streamed = true;
        let _ = sender.send(event);
    };
    let mut part_count = 0;
    while let Some(item) = response.next().await {
        let event = match item {
            Ok(Item::Event(event)) => event,
            // A payload the decoder does not model.
            Ok(Item::Unknown(_)) => continue,
            // A tool call whose input is not JSON ends rig's stream; it is
            // kept, so the turn answers it with an error (R-22).
            Err(ProviderError::MalformedToolInput(input)) => {
                malformed = Some(input);
                break;
            }
            Err(error) => return Err(failed(&error)),
        };
        match event {
            StreamEvent::Text { text, .. } => {
                if !text.is_empty() {
                    send(Event::Text(text));
                }
            }
            StreamEvent::Reasoning { part, text } => {
                thinking_shown.insert(part.index());
                send(Event::Thinking(text));
            }
            StreamEvent::End { part, content } => {
                part_count = part_count.max(part.index() + 1);
                match content {
                    // A complete block shows only if no delta did.
                    AssistantContent::Reasoning(sealed) => {
                        if !thinking_shown.contains(&part.index()) {
                            let issuer = sealed.issuer().clone();
                            if let Some(reasoning) = sealed.open(&issuer) {
                                send(Event::Thinking(reasoning.display_text()));
                            }
                        }
                    }
                    AssistantContent::ToolCall(call) => {
                        let id = call.id.wire().into_owned();
                        calls_started.push(id.clone());
                        announce(
                            &mut send,
                            part.index(),
                            id,
                            call.function.name.to_string(),
                            call.function.arguments.to_string(),
                        );
                    }
                    AssistantContent::Text(_) | AssistantContent::Image(_) => {}
                }
            }
            StreamEvent::Start { .. } | StreamEvent::Arguments { .. } => {}
        }
    }
    let (mut content, stop, usage) = match malformed {
        Some(input) => {
            // What arrived before the unreadable call, and the call with its
            // raw text; rig reports no usage or stop for such a reply.
            let partial = response.partial();
            let mut content: Vec<AssistantPart> = partial.choice.iter().filter_map(part).collect();
            let id = input.id.wire().into_owned();
            calls_started.push(id.clone());
            announce(
                &mut send,
                part_count,
                id.clone(),
                input.name.clone(),
                input.raw.clone(),
            );
            content.push(AssistantPart::ToolCall {
                id,
                name: input.name,
                input: Value::String(input.raw),
            });
            (content, StopReason::ToolUse, self::usage(partial.usage))
        }
        None => {
            let response = response.finish().await.map_err(|error| failed(&error))?;
            let content: Vec<AssistantPart> = response.choice.iter().filter_map(part).collect();
            let has_calls = content
                .iter()
                .any(|part| matches!(part, AssistantPart::ToolCall { .. }));
            let stop = match response.finish_reason() {
                Some(FinishReason::ToolCalls) => StopReason::ToolUse,
                Some(FinishReason::Length) => StopReason::MaxTokens,
                Some(FinishReason::ContentFilter) => StopReason::Refusal,
                Some(FinishReason::Stop) if has_calls => StopReason::ToolUse,
                Some(FinishReason::Stop) => StopReason::EndTurn,
                Some(FinishReason::Other(other)) => match other.as_str() {
                    "refusal" => StopReason::Refusal,
                    "tool_use" | "tool_calls" => StopReason::ToolUse,
                    "end_turn" | "stop" | "stop_sequence" => StopReason::EndTurn,
                    "max_tokens" | "length" => StopReason::MaxTokens,
                    _ => StopReason::Other(other.clone()),
                },
                None if has_calls => StopReason::ToolUse,
                None => StopReason::EndTurn,
            };
            (content, stop, self::usage(response.usage))
        }
    };
    // After a switch to a fallback model, what the declined model wrote
    // before it is not part of the reply, except its text (Q-18).
    if let Some(switch) = fallbacks.and_then(fallback::FallbackTransport::switch) {
        let started: Vec<&str> = calls_started.iter().map(String::as_str).collect();
        switch.drop_declined(&mut content, &started);
    }
    send(Event::Usage(usage));
    Ok(Reply {
        content,
        stop,
        usage,
    })
}

/// A tool call, as the Conversation's events show it: started, its whole
/// input, the provider's id.
fn announce(send: &mut impl FnMut(Event), part: usize, id: String, name: String, json: String) {
    let stream_id = format!("call-{part}");
    send(Event::ToolCallStarted {
        stream_id: stream_id.clone(),
        name,
    });
    send(Event::ToolInput {
        stream_id: stream_id.clone(),
        json,
    });
    send(Event::ToolCallId { stream_id, id });
}

fn part(content: &AssistantContent) -> Option<AssistantPart> {
    Some(match content {
        AssistantContent::Text(text) if text.text.is_empty() => return None,
        AssistantContent::Text(text) => AssistantPart::Text {
            text: text.text.clone(),
        },
        AssistantContent::ToolCall(call) => AssistantPart::ToolCall {
            id: call.id.wire().into_owned(),
            name: call.function.name.to_string(),
            input: call.function.arguments.clone(),
        },
        AssistantContent::Reasoning(sealed) => {
            let issuer = sealed.issuer().clone();
            let reasoning = sealed.open(&issuer)?;
            AssistantPart::Reasoning(Reasoning {
                id: reasoning.id.clone(),
                parts: reasoning
                    .content
                    .iter()
                    .map(|part| match part {
                        ReasoningContent::Text { text, signature } => ReasoningPart::Text {
                            text: text.clone(),
                            signature: signature.clone(),
                        },
                        ReasoningContent::Summary(text) => {
                            ReasoningPart::Summary { text: text.clone() }
                        }
                        ReasoningContent::Encrypted(data) => {
                            ReasoningPart::Encrypted { data: data.clone() }
                        }
                        ReasoningContent::Redacted { data } => {
                            ReasoningPart::Redacted { data: data.clone() }
                        }
                    })
                    .collect(),
            })
        }
        AssistantContent::Image(_) => return None,
    })
}

/// rig's usage in Agentique's terms: rig 0.43 counts cache reads and writes
/// inside the input on every provider; Agentique keeps them apart. A count
/// rig does not have is 0 here (chat usage has no "unknown").
fn usage(usage: rig_core::completion::Usage) -> Usage {
    let cached = usage.cached_input_tokens.unwrap_or(0);
    let written = usage.cache_creation_input_tokens.unwrap_or(0);
    Usage {
        input_tokens: usage
            .input_tokens
            .unwrap_or(0)
            .saturating_sub(cached + written),
        cache_write_tokens: written,
        cache_read_tokens: cached,
        output_tokens: usage.output_tokens.unwrap_or(0),
        reasoning_tokens: usage.reasoning_tokens.unwrap_or(0),
    }
}

struct Failure {
    error: Error,
    /// Seconds to wait before an automatic retry, if one makes sense.
    retry_after: Option<u64>,
}

/// The Operator-facing failure for a rig error.
fn failure(provider: Provider, model: &str, error: &ProviderError) -> Failure {
    let name = provider.name();
    let status = error
        .provider_response_status()
        .map(|status| status.as_u16());
    let body = error.provider_response_body().map(str::to_string);
    let retry_after = error
        .provider_response_headers()
        .and_then(|headers| headers.get("retry-after"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok());
    let said = body
        .as_deref()
        .map(provider_message)
        .unwrap_or_else(|| error.to_string());
    let fail = |kind, message: String, retry_after| Failure {
        error: Error { kind, message },
        retry_after,
    };
    let Some(status) = status else {
        let kind = body.as_deref().and_then(error_type);
        return match (error, kind.as_deref()) {
            (_, Some("overloaded_error")) => fail(
                ErrorKind::Unavailable,
                format!("{name} became overloaded while replying. Try again in a moment."),
                Some(1),
            ),
            (_, Some("rate_limit_error")) => fail(
                ErrorKind::RateLimited,
                format!("The {name} rate limit was reached while replying. Try again in a minute."),
                Some(10),
            ),
            // The reply ended before the provider ended it: nothing of it
            // is used, whatever arrived.
            (ProviderError::Truncated, _) => fail(
                ErrorKind::Unreachable,
                format!(
                    "The connection to {name} was lost before the reply was complete. Try again."
                ),
                Some(0),
            ),
            // Connection failures and resets.
            (ProviderError::Http(_) | ProviderError::Provider(_), _) => fail(
                ErrorKind::Unreachable,
                format!(
                    "Could not reach {name} ({said}). Check the network connection and try again."
                ),
                Some(0),
            ),
            // The request could not be built: retrying cannot help.
            (ProviderError::Request(_) | ProviderError::Url(_), _) => fail(
                ErrorKind::Other,
                format!("Could not prepare the request to {name} ({said})."),
                None,
            ),
            _ => fail(
                ErrorKind::Other,
                format!("{name} sent a reply that could not be read ({said}). Try again."),
                None,
            ),
        };
    };
    let variable = provider.key_variable();
    match status {
        401 => fail(
            ErrorKind::KeyRefused,
            format!(
                "{name} did not accept the API key (401). Check the key in Settings (Ctrl+,) or {variable}; the Surface keeps working by hand."
            ),
            None,
        ),
        402 => fail(
            ErrorKind::NoAccess,
            format!("The {name} account cannot pay for this request (402): {said}"),
            None,
        ),
        403 => fail(
            ErrorKind::NoAccess,
            format!("This {name} key may not make this request (403): {said}"),
            None,
        ),
        404 => fail(
            ErrorKind::UnknownModel,
            format!(
                "The model `{model}` was not found or is not available to this {name} key (404)."
            ),
            None,
        ),
        429 => fail(
            ErrorKind::RateLimited,
            match retry_after {
                Some(1) => {
                    format!("The {name} rate limit was reached (429). Try again in a second.")
                }
                Some(seconds) => format!(
                    "The {name} rate limit was reached (429). Try again in {seconds} seconds."
                ),
                None => format!("The {name} rate limit was reached (429). Try again in a minute."),
            },
            Some(retry_after.unwrap_or(10)),
        ),
        400 | 413 | 422 => fail(
            ErrorKind::Rejected,
            format!("{name} did not accept the request ({status}): {said}"),
            None,
        ),
        500..=599 => fail(
            ErrorKind::Unavailable,
            format!("{name} is not available right now ({status}). Try again in a moment."),
            Some(retry_after.unwrap_or(1)),
        ),
        _ => fail(
            ErrorKind::Other,
            format!("{name} answered with an error ({status}): {said}"),
            None,
        ),
    }
}

/// The provider's error type from an error body (`{"error": {"type": ...}}`).
fn error_type(body: &str) -> Option<String> {
    let detail: Value = serde_json::from_str(body).ok()?;
    detail["error"]["type"].as_str().map(str::to_string)
}

/// The provider's own message from an error body, or the body's start.
fn provider_message(body: &str) -> String {
    let detail: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    detail["error"]["message"]
        .as_str()
        .or_else(|| detail["message"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body.trim().chars().take(300).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModelRef, Tool};

    fn request(provider: Provider, model: &str, effort: Option<&str>) -> ChatRequest {
        ChatRequest {
            model: ModelRef::new(provider, model),
            effort: effort.map(str::to_string),
            max_output_tokens: 1000,
            system: "system".into(),
            tools: vec![Tool {
                name: "read_model".into(),
                description: "Read".into(),
                input_schema: json!({ "type": "object" }),
            }],
            messages: vec![Message::User(vec![UserPart::Text { text: "hi".into() }])],
        }
    }

    #[test]
    fn deepseek_gets_thinking_and_a_supported_effort() {
        let params =
            additional_params(&request(Provider::DeepSeek, "deepseek-flash", Some("high")));
        assert_eq!(
            params,
            Some(json!({ "thinking": { "type": "enabled" }, "reasoning_effort": "high" }))
        );
        // `medium` is not a DeepSeek level: the default applies instead.
        let params = additional_params(&request(
            Provider::DeepSeek,
            "deepseek-flash",
            Some("medium"),
        ));
        assert_eq!(params.unwrap()["reasoning_effort"], "high");
    }

    #[test]
    fn anthropic_asks_for_summaries_and_effort() {
        let params =
            additional_params(&request(Provider::Anthropic, "claude-opus-5", None)).unwrap();
        assert_eq!(params["thinking"]["display"], "summarized");
        assert_eq!(params["output_config"]["effort"], "high");
        // Server-side fallbacks on the default model only (C-27).
        assert_eq!(params["fallbacks"], "default");
        let params =
            additional_params(&request(Provider::Anthropic, "claude-opus-5-5", None)).unwrap();
        assert!(params.get("fallbacks").is_none());
    }

    #[test]
    fn deepseek_turns_without_reasoning_get_a_stand_in() {
        let mut request = request(Provider::DeepSeek, "deepseek-flash", None);
        request.messages = vec![
            Message::User(vec![UserPart::Text { text: "hi".into() }]),
            // A stopped reply: text only.
            Message::Assistant(vec![AssistantPart::Text {
                text: "Partly".into(),
            }]),
            Message::User(vec![UserPart::Text {
                text: "go on".into(),
            }]),
        ];
        let converted = rig_request(&request).unwrap();
        let rig::Message::Assistant { content, .. } = &converted.chat_history[2] else {
            panic!("an assistant message");
        };
        assert!(
            matches!(&content[0], AssistantContent::Reasoning(r) if r.issuer().to_string() == "deepseek"
                && r.open(r.issuer()).is_some_and(|r| r.display_text() == NO_REASONING))
        );
        // Other providers are sent the turn as it was.
        let mut other = request.clone();
        other.model = ModelRef::new(Provider::OpenRouter, "deepseek/deepseek-flash");
        let converted = rig_request(&other).unwrap();
        let rig::Message::Assistant { content, .. } = &converted.chat_history[2] else {
            panic!("an assistant message");
        };
        assert_eq!(content.len(), 1);
    }

    #[test]
    fn provider_messages_are_read_from_error_bodies() {
        assert_eq!(
            provider_message(r#"{"error":{"message":"Authentication Fails"}}"#),
            "Authentication Fails"
        );
        assert_eq!(provider_message("plain text"), "plain text");
    }
}
