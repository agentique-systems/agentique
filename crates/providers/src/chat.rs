//! One streamed model call through rig, for every provider (C-34).
//!
//! Every provider goes through the same code: the request is converted into
//! rig's request once, the provider's client and its provider-specific
//! parameters are chosen from data, and one generic function streams the
//! reply into [`Event`]s. rig's own agent loop is not used: the Assistant's
//! turn loop holds Agentique's policy (R-21).

use crate::{
    AssistantPart, ChatRequest, Error, ErrorKind, Event, Message, Provider, Reasoning,
    ReasoningPart, Reply, StopReason, Usage, UserPart, capabilities, fallback, runtime,
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
use std::collections::{BTreeMap, BTreeSet};
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
            let model = client.completion_model(model);
            stream(model, rig_request, request, sender, None).await
        }};
    }
    match provider {
        Provider::DeepSeek => stream_with!(deepseek::Client),
        // Through the fallback adapter (Q-18): the beta header, like
        // `fallbacks` in the request, only for models that use them (C-27).
        Provider::Anthropic => {
            let http = fallback::HttpClient::default();
            let mut builder = anthropic::Client::builder()
                .api_key(anthropic::client::AnthropicKey::from(key))
                .http_client(http.clone());
            if capabilities(&request.model).refusal_fallbacks {
                builder = builder.anthropic_beta(fallback::BETA);
            }
            if let Some(url) = endpoint {
                builder = builder.base_url(url);
            }
            let client = builder.build().map_err(setup)?;
            let model = client.completion_model(model);
            stream(model, rig_request, request, sender, Some(&http)).await
        }
        Provider::OpenAi => stream_with!(openai::Client),
        Provider::OpenRouter => stream_with!(openrouter::Client),
        // Refused above (no tools); kept as an error, never a panic.
        Provider::TypeSafe => Err(Error {
            kind: ErrorKind::Rejected,
            message: "TypeSafe AI is not a model for the Assistant.".to_string(),
        }),
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
            Message::Assistant(parts) => {
                let mut content: Vec<AssistantContent> =
                    parts.iter().map(assistant_content).collect();
                if needs_reasoning(request, parts) {
                    content.insert(
                        0,
                        AssistantContent::Reasoning(rig::Reasoning::new(NO_REASONING)),
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
/// connection) twice as long as nothing has been streamed yet. `fallbacks`
/// is the HTTP client of an Anthropic call, which reports a switch to a
/// fallback model.
async fn stream<M: CompletionModel>(
    model: M,
    rig_request: CompletionRequest,
    request: &ChatRequest,
    sender: &Sender<Event>,
    fallbacks: Option<&fallback::HttpClient>,
) -> Result<Reply, Error> {
    let mut attempt = 0;
    loop {
        let mut streamed = false;
        let once = stream_once(
            &model,
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

async fn stream_once<M: CompletionModel>(
    model: &M,
    rig_request: CompletionRequest,
    request: &ChatRequest,
    sender: &Sender<Event>,
    fallbacks: Option<&fallback::HttpClient>,
    streamed: &mut bool,
) -> Result<Reply, Failure> {
    let provider = request.model.provider;
    let failed = |error: CompletionError| failure(provider, &request.model.model, &error);
    let mut response = model.stream(rig_request).await.map_err(failed)?;
    // Tool calls by stream id: name and raw input so far.
    let mut calls: BTreeMap<String, (String, String)> = BTreeMap::new();
    // Stream ids in the order the calls started, and the provider's id of
    // each call that arrived complete.
    let mut started: Vec<String> = Vec::new();
    let mut provider_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut unreadable = Vec::new();
    let mut thinking_shown = BTreeSet::new();
    let mut last = None;
    let mut send = |event| {
        *streamed = true;
        let _ = sender.send(event);
    };
    while let Some(item) = response.next().await {
        let item = match item {
            Ok(item) => item,
            // rig reports a tool call whose input is not JSON in-band and
            // goes on; it becomes a call the turn answers with an error
            // (R-22), never a failed reply.
            Err(CompletionError::ResponseError(message))
                if message.contains("arrived with malformed JSON input") =>
            {
                unreadable.push(message);
                continue;
            }
            Err(error) => return Err(failed(error)),
        };
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
                let call = calls
                    .entry(internal_call_id.clone())
                    .or_insert_with(|| (String::new(), String::new()));
                match content {
                    // Every adapter names the call before its input.
                    ToolCallDeltaContent::Name(name) => {
                        if call.0.is_empty() {
                            call.0 = name.clone();
                            started.push(internal_call_id.clone());
                            send(Event::ToolCallStarted {
                                stream_id: internal_call_id,
                                name,
                            });
                        }
                    }
                    ToolCallDeltaContent::Delta(json) => {
                        call.1.push_str(&json);
                        send(Event::ToolInput {
                            stream_id: internal_call_id,
                            json,
                        });
                    }
                }
            }
            StreamedAssistantContent::ToolCall {
                tool_call,
                internal_call_id,
            } => {
                if !calls.contains_key(&internal_call_id) {
                    // The whole call arrived at once.
                    started.push(internal_call_id.clone());
                    let json = tool_call.function.arguments.to_string();
                    send(Event::ToolCallStarted {
                        stream_id: internal_call_id.clone(),
                        name: tool_call.function.name.clone(),
                    });
                    send(Event::ToolInput {
                        stream_id: internal_call_id.clone(),
                        json: json.clone(),
                    });
                }
                calls.remove(&internal_call_id);
                let id = tool_call.wire_call_id().to_string();
                provider_ids.insert(internal_call_id.clone(), id.clone());
                send(Event::ToolCallId {
                    stream_id: internal_call_id,
                    id,
                });
            }
            StreamedAssistantContent::Final(record) => last = Some(record),
            _ => {}
        }
    }
    // Without the provider's terminal record the stream was cut off: nothing
    // of it is used, whatever arrived (rig emits none on an early end).
    let Some(last) = last else {
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
    };
    let mut content: Vec<AssistantPart> = response.choice.iter().filter_map(part).collect();
    // Calls whose input could not be read stay in the reply with their raw
    // text, under their stream id, so the turn answers them with an error.
    if !unreadable.is_empty() {
        for (stream_id, (name, raw)) in calls {
            content.push(AssistantPart::ToolCall {
                id: stream_id,
                name,
                input: serde_json::Value::String(raw),
            });
        }
    }
    // After a switch to a fallback model, what the declined model wrote
    // before it is not part of the reply, except its text (Q-18).
    if let Some(switch) = fallbacks.and_then(fallback::HttpClient::switch) {
        let calls_started: Vec<&str> = started
            .iter()
            .map(|stream_id| provider_ids.get(stream_id).unwrap_or(stream_id).as_str())
            .collect();
        switch.drop_declined(&mut content, &calls_started);
    }
    let has_calls = content
        .iter()
        .any(|part| matches!(part, AssistantPart::ToolCall { .. }));
    let stop = match last.finish_reason {
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
        None if has_calls => StopReason::ToolUse,
        None => StopReason::EndTurn,
    };
    let usage = self::usage(provider, last.usage);
    send(Event::Usage(usage));
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
        Provider::Anthropic | Provider::TypeSafe => usage.input_tokens,
        Provider::OpenAi | Provider::DeepSeek => usage.input_tokens.saturating_sub(cached),
        // OpenRouter's prompt count includes cache reads and writes.
        Provider::OpenRouter => usage
            .input_tokens
            .saturating_sub(cached + usage.cache_creation_input_tokens),
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
            // Connection failures and resets on a streamed request.
            (CompletionError::HttpError(_) | CompletionError::ProviderError(_), _) => fail(
                ErrorKind::Unreachable,
                format!(
                    "Could not reach {name} ({said}). Check the network connection and try again."
                ),
                Some(0),
            ),
            // The request could not be built: retrying cannot help.
            (CompletionError::RequestError(_), _) => fail(
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
            matches!(&content[0], AssistantContent::Reasoning(r) if r.display_text() == NO_REASONING)
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
