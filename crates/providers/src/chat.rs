//! One streamed model call through rig, for every provider (C-34).
//!
//! Every provider goes through the same code: the request is converted into
//! rig's request once, the provider's client and its provider-specific
//! parameters are chosen from data, and one generic function streams the
//! reply into [`Event`]s. rig's own agent loop is not used: the Assistant's
//! turn loop holds Agentique's policy (R-21).

use crate::{
    AssistantPart, ChatRequest, Error, ErrorKind, Event, Message, Provider, Reasoning,
    ReasoningPart, Reply, StopReason, Usage, UserPart, capabilities, runtime,
};
use futures::StreamExt;
use rig_core::client::CompletionClient;
use rig_core::completion::{
    CompletionError, CompletionModel, CompletionRequest, FinishReason, ToolDefinition,
};
use rig_core::message::{
    self as rig, AssistantContent, ReasoningContent, ToolCallId, ToolFunction, ToolResultContent,
    UserContent,
};
use rig_core::providers::{anthropic, deepseek, openai, openrouter};
use rig_core::streaming::{StreamedAssistantContent, ToolCallDeltaContent};
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
    let Some(key) = key else {
        return Err(Error {
            kind: ErrorKind::MissingKey,
            message: format!(
                "No {} key is set. Set {} and restart Agentique; the Surface keeps working without it.",
                provider.name(),
                provider.key_variable()
            ),
        });
    };
    let rig_request = rig_request(request)?;
    let model = request.model.model.clone();
    let setup = |error: rig_core::http_client::Error| Error {
        kind: ErrorKind::Other,
        message: format!(
            "Could not prepare the connection to {} ({error}).",
            provider.name()
        ),
    };
    macro_rules! stream_with {
        ($client:ty) => {{
            let mut builder = <$client>::builder().api_key(key);
            if let Some(url) = endpoint {
                builder = builder.base_url(url);
            }
            let client = builder.build().map_err(setup)?;
            stream(client.completion_model(model), rig_request, request, sender).await
        }};
    }
    match provider {
        Provider::DeepSeek => stream_with!(deepseek::Client),
        Provider::Anthropic => stream_with!(anthropic::Client),
        Provider::OpenAi => stream_with!(openai::Client),
        Provider::OpenRouter => stream_with!(openrouter::Client),
    }
}

/// Converts the request into rig's, with the provider's own parameters for
/// reasoning and effort passed through `additional_params` (R-22).
fn rig_request(request: &ChatRequest) -> Result<CompletionRequest, Error> {
    let mut history = vec![rig::Message::System {
        content: request.system.clone(),
    }];
    for message in &request.messages {
        history.push(match message {
            Message::User(parts) => rig::Message::User {
                content: parts.iter().map(user_content).collect(),
            },
            Message::Assistant(parts) => rig::Message::Assistant {
                id: None,
                content: parts.iter().map(assistant_content).collect(),
            },
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
        preamble: None,
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
        // Adaptive thinking with readable summaries (R-31) and effort.
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
    }
}

fn user_content(part: &UserPart) -> UserContent {
    match part {
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
                call: ToolCallId::new_or_mint(call_id.clone()),
                provider: rig::ProviderCallId::new(call_id.clone()),
                name: name.clone(),
                content: vec![ToolResultContent::Text(rig::Text::new(text))],
            })
        }
    }
}

fn assistant_content(part: &AssistantPart) -> AssistantContent {
    match part {
        AssistantPart::Text { text } => AssistantContent::Text(rig::Text::new(text.clone())),
        AssistantPart::Reasoning(reasoning) => AssistantContent::Reasoning(rig::Reasoning {
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
                    ReasoningPart::Encrypted { data } => ReasoningContent::Encrypted(data.clone()),
                    ReasoningPart::Redacted { data } => {
                        ReasoningContent::Redacted { data: data.clone() }
                    }
                })
                .collect(),
        }),
        AssistantPart::ToolCall { id, name, input } => AssistantContent::ToolCall(
            rig::ToolCall::from_wire(id.clone(), ToolFunction::new(name.clone(), input.clone())),
        ),
    }
}

/// Streams one call, retrying a failure (rate limit, server error, lost
/// connection) twice as long as nothing has been streamed yet.
async fn stream<M: CompletionModel>(
    model: M,
    rig_request: CompletionRequest,
    request: &ChatRequest,
    sender: &Sender<Event>,
) -> Result<Reply, Error> {
    let mut attempt = 0;
    loop {
        let mut streamed = false;
        match stream_once(&model, rig_request.clone(), request, sender, &mut streamed).await {
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

async fn stream_once<M: CompletionModel>(
    model: &M,
    rig_request: CompletionRequest,
    request: &ChatRequest,
    sender: &Sender<Event>,
    streamed: &mut bool,
) -> Result<Reply, Failure> {
    let provider = request.model.provider;
    let failed = |error: CompletionError| failure(provider, &request.model.model, &error);
    let mut response = model.stream(rig_request).await.map_err(failed)?;
    let mut started = BTreeSet::new();
    let mut thinking_shown = BTreeSet::new();
    let mut usage = None;
    let mut finish = None;
    let mut send = |event| {
        *streamed = true;
        let _ = sender.send(event);
    };
    while let Some(item) = response.next().await {
        let item = item.map_err(failed)?;
        match item {
            StreamedAssistantContent::Text(text) => {
                if !text.text.is_empty() {
                    send(Event::Text(text.text));
                }
            }
            StreamedAssistantContent::ReasoningDelta { id, reasoning, .. } => {
                thinking_shown.insert(id);
                send(Event::Thinking(reasoning));
            }
            StreamedAssistantContent::Reasoning { id, reasoning } => {
                // A complete block replaces its deltas; show it only if no
                // delta did.
                if thinking_shown.insert(id) {
                    send(Event::Thinking(reasoning.display_text()));
                }
            }
            StreamedAssistantContent::ToolCallDelta {
                internal_call_id,
                content,
            } => {
                let (name, json) = match content {
                    ToolCallDeltaContent::Name(name) => (name, None),
                    ToolCallDeltaContent::Delta(json) => (String::new(), Some(json)),
                };
                if started.insert(internal_call_id.clone()) {
                    send(Event::ToolCallStarted {
                        stream_id: internal_call_id.clone(),
                        name,
                    });
                }
                if let Some(json) = json {
                    send(Event::ToolInput {
                        stream_id: internal_call_id,
                        json,
                    });
                }
            }
            StreamedAssistantContent::ToolCall {
                tool_call,
                internal_call_id,
            } => {
                if started.insert(internal_call_id.clone()) {
                    // The whole call arrived at once.
                    send(Event::ToolCallStarted {
                        stream_id: internal_call_id.clone(),
                        name: tool_call.function.name.clone(),
                    });
                    send(Event::ToolInput {
                        stream_id: internal_call_id.clone(),
                        json: tool_call.function.arguments.to_string(),
                    });
                }
                let id = tool_call.wire_call_id().to_string();
                if id != internal_call_id {
                    send(Event::ToolCallId {
                        stream_id: internal_call_id,
                        id,
                    });
                }
            }
            StreamedAssistantContent::Final(last) => {
                usage = Some(last.usage);
                finish = last.finish_reason.clone();
            }
            _ => {}
        }
    }
    let usage = self::usage(provider, usage.unwrap_or_else(|| response.usage()));
    send(Event::Usage(usage));
    let content: Vec<AssistantPart> = response.choice.iter().filter_map(part).collect();
    let has_calls = content
        .iter()
        .any(|part| matches!(part, AssistantPart::ToolCall { .. }));
    let stop = match finish {
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
            _ => StopReason::Other(other),
        },
        // No finish reason: a stream that ended early is a lost connection.
        None if content.is_empty() => {
            return Err(Failure {
                error: Error {
                    kind: ErrorKind::Unreachable,
                    message: format!(
                        "The connection to {} was lost before the reply was complete. Try again.",
                        provider.name()
                    ),
                },
                retry_after: Some(0),
            });
        }
        None if has_calls => StopReason::ToolUse,
        None => StopReason::EndTurn,
    };
    Ok(Reply {
        content,
        stop,
        usage,
    })
}

fn part(content: &AssistantContent) -> Option<AssistantPart> {
    Some(match content {
        AssistantContent::Text(text) if text.text.is_empty() => return None,
        AssistantContent::Text(text) => AssistantPart::Text {
            text: text.text.clone(),
        },
        AssistantContent::ToolCall(call) => AssistantPart::ToolCall {
            id: call.wire_call_id().to_string(),
            name: call.function.name.clone(),
            input: call.function.arguments.clone(),
        },
        AssistantContent::Reasoning(reasoning) => AssistantPart::Reasoning(Reasoning {
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
        }),
        AssistantContent::Image(_) => return None,
    })
}

/// rig's usage in Agentique's terms. OpenAI-style providers count cached
/// input inside the input total; Anthropic counts it apart.
fn usage(provider: Provider, usage: rig_core::completion::Usage) -> Usage {
    let cached = usage.cached_input_tokens;
    let input = match provider {
        Provider::Anthropic => usage.input_tokens,
        Provider::OpenAi | Provider::OpenRouter | Provider::DeepSeek => {
            usage.input_tokens.saturating_sub(cached)
        }
    };
    Usage {
        input_tokens: input,
        cache_write_tokens: usage.cache_creation_input_tokens,
        cache_read_tokens: cached,
        output_tokens: usage.output_tokens,
        reasoning_tokens: usage.reasoning_tokens,
    }
}

struct Failure {
    error: Error,
    /// Seconds to wait before an automatic retry, if one makes sense.
    retry_after: Option<u64>,
}

/// The Operator-facing failure for a rig error.
fn failure(provider: Provider, model: &str, error: &CompletionError) -> Failure {
    let name = provider.name();
    let (status, body, retry_after) = http_details(error);
    let said = body
        .as_deref()
        .map(provider_message)
        .unwrap_or_else(|| error.to_string());
    let fail = |kind, message: String, retry_after| Failure {
        error: Error { kind, message },
        retry_after,
    };
    let Some(status) = status else {
        return match error {
            CompletionError::HttpError(_) | CompletionError::RequestError(_) => fail(
                ErrorKind::Unreachable,
                format!(
                    "Could not reach {name} ({said}). Check the network connection and try again."
                ),
                Some(0),
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
                "{name} did not accept the API key (401). Check {variable} and restart Agentique; the Surface keeps working by hand."
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

/// The HTTP status, body and `Retry-After` seconds of a failed request, if
/// the error came from an HTTP response.
fn http_details(error: &CompletionError) -> (Option<u16>, Option<String>, Option<u64>) {
    use rig_core::http_client::Error as Http;
    let retry = |headers: &reqwest::header::HeaderMap| {
        headers
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
    };
    match error {
        CompletionError::HttpError(Http::InvalidStatusCodeWithDetails {
            status,
            body,
            headers,
        }) => (Some(status.as_u16()), Some(body.clone()), retry(headers)),
        CompletionError::HttpError(Http::InvalidStatusCodeWithMessage(status, body)) => {
            (Some(status.as_u16()), Some(body.clone()), None)
        }
        CompletionError::HttpError(Http::InvalidStatusCode(status)) => {
            (Some(status.as_u16()), None, None)
        }
        CompletionError::ProviderResponse(response) => (
            response.status.map(|status| status.as_u16()),
            Some(response.body.clone()),
            response.headers.as_deref().and_then(retry),
        ),
        _ => (None, None, None),
    }
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
    fn anthropic_asks_for_summaries_and_fallbacks_on_the_default_model() {
        let params =
            additional_params(&request(Provider::Anthropic, "claude-opus-5", None)).unwrap();
        assert_eq!(params["thinking"]["display"], "summarized");
        assert_eq!(params["output_config"]["effort"], "high");
        assert_eq!(params["fallbacks"], "default");
        let params =
            additional_params(&request(Provider::Anthropic, "claude-opus-5-5", None)).unwrap();
        assert!(params.get("fallbacks").is_none());
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
