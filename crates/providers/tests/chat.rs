//! Model calls without the network: a local server answers like each
//! provider with canned event streams, and records what it was sent.

use agq_providers::{
    AssistantPart, ChatHandle, ChatRequest, ErrorKind, Event, Message, ModelRef, Provider,
    Providers, ReasoningPart, Reply, StopReason, Tool, Usage, UserPart,
};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// One canned answer: a status line, extra headers and a body.
struct Answer {
    status: &'static str,
    headers: &'static str,
    body: String,
    /// Keep the connection open this long after the body (to test stops).
    linger: Duration,
}

fn ok(body: String) -> Answer {
    Answer {
        status: "200 OK",
        headers: "Content-Type: text/event-stream\r\n",
        body,
        linger: Duration::ZERO,
    }
}

/// Serves the answers in order, one per connection; reports each request's
/// path and JSON body.
fn serve(answers: Vec<Answer>) -> (String, Receiver<(String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    std::thread::spawn(move || {
        for answer in answers {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let _ = sender.send((path, serde_json::from_slice(&body).unwrap_or(Value::Null)));
            let mut stream = stream;
            let head = format!(
                "HTTP/1.1 {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n",
                answer.status,
                answer.headers,
                answer.body.len()
            );
            if answer.linger.is_zero() {
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(answer.body.as_bytes());
            } else {
                // Send the head and part of the body, then go silent.
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&answer.body.as_bytes()[..answer.body.len() / 2]);
                let _ = stream.flush();
                std::thread::sleep(answer.linger);
            }
        }
    });
    (url, requests)
}

/// Chat-completions stream chunks (DeepSeek, OpenRouter).
fn chunks(chunks: &[Value]) -> String {
    let mut body: String = chunks
        .iter()
        .map(|chunk| format!("data: {chunk}\n\n"))
        .collect();
    body.push_str("data: [DONE]\n\n");
    body
}

fn delta(delta: Value) -> Value {
    json!({ "id": "c1", "object": "chat.completion.chunk", "model": "deepseek-flash",
            "choices": [{ "index": 0, "delta": delta, "finish_reason": null }] })
}

fn finish(reason: &str) -> Value {
    json!({ "id": "c1", "object": "chat.completion.chunk", "model": "deepseek-flash",
            "choices": [{ "index": 0, "delta": {}, "finish_reason": reason }],
            "usage": { "prompt_tokens": 1000, "completion_tokens": 50, "total_tokens": 1050,
                       "prompt_cache_hit_tokens": 800, "prompt_cache_miss_tokens": 200,
                       "completion_tokens_details": { "reasoning_tokens": 30 } } })
}

fn request(provider: Provider, model: &str) -> ChatRequest {
    ChatRequest {
        model: ModelRef::new(provider, model),
        effort: Some("high".into()),
        max_output_tokens: 4096,
        system: "You are the Assistant.".into(),
        tools: vec![Tool {
            name: "find_elements".into(),
            description: "Find elements by name.".into(),
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } } }),
        }],
        messages: vec![Message::User(vec![UserPart::Text {
            text: "Add a link store.".into(),
        }])],
    }
}

/// Collects every event until the call finishes.
fn finish_call(mut handle: ChatHandle) -> (Vec<Event>, Result<Reply, agq_providers::Error>) {
    let mut events = Vec::new();
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end {
        match handle.next_event(Duration::from_millis(50)) {
            Some(Event::Finished(result)) => return (events, result),
            Some(event) => events.push(event),
            None => {}
        }
    }
    panic!("the call did not finish; events so far: {events:?}");
}

fn deepseek(url: &str) -> Providers {
    Providers::new()
        .with_key(Provider::DeepSeek, "test-key")
        .with_endpoint(Provider::DeepSeek, url)
}

#[test]
fn deepseek_streams_reasoning_and_text() {
    let (url, requests) = serve(vec![ok(chunks(&[
        delta(json!({ "role": "assistant", "reasoning_content": "The store " })),
        delta(json!({ "reasoning_content": "holds links." })),
        delta(json!({ "content": "Done" })),
        delta(json!({ "content": "." })),
        finish("stop"),
    ]))]);
    let (events, reply) =
        finish_call(deepseek(&url).chat(request(Provider::DeepSeek, "deepseek-flash")));
    let reply = reply.unwrap();
    assert_eq!(reply.stop, StopReason::EndTurn);
    let thinking: String = events
        .iter()
        .filter_map(|event| match event {
            Event::Thinking(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(thinking, "The store holds links.");
    assert!(events.contains(&Event::Text("Done".into())));
    // Reasoning is kept for the next request, then the text.
    assert!(
        matches!(&reply.content[0], AssistantPart::Reasoning(reasoning)
        if reasoning.parts == [ReasoningPart::Text { text: "The store holds links.".into(), signature: None }])
    );
    assert_eq!(
        reply.content[1],
        AssistantPart::Text {
            text: "Done.".into()
        }
    );
    // Cached input is counted apart from full-price input.
    assert_eq!(
        reply.usage,
        Usage {
            input_tokens: 200,
            cache_write_tokens: 0,
            cache_read_tokens: 800,
            output_tokens: 50,
            reasoning_tokens: 30,
        }
    );
    assert!(events.contains(&Event::Usage(reply.usage)));

    let (path, body) = requests.recv().unwrap();
    assert_eq!(path, "/chat/completions");
    assert_eq!(body["model"], "deepseek-flash");
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(body["stream"], true);
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["tools"][0]["function"]["name"], "find_elements");
    assert!(body.get("tool_choice").is_none_or(Value::is_null));
}

#[test]
fn deepseek_streams_a_tool_call_and_sends_the_exchange_back() {
    let (url, requests) = serve(vec![ok(chunks(&[
        delta(json!({ "role": "assistant", "reasoning_content": "Look it up." })),
        delta(
            json!({ "tool_calls": [{ "index": 0, "id": "call_7", "type": "function",
                                       "function": { "name": "find_elements", "arguments": "" } }] }),
        ),
        delta(json!({ "tool_calls": [{ "index": 0, "function": { "arguments": "{\"name\":" } }] })),
        delta(json!({ "tool_calls": [{ "index": 0, "function": { "arguments": "\"Store\"}" } }] })),
        finish("tool_calls"),
    ]))]);
    let (events, reply) =
        finish_call(deepseek(&url).chat(request(Provider::DeepSeek, "deepseek-flash")));
    let reply = reply.unwrap();
    assert_eq!(reply.stop, StopReason::ToolUse);
    let call = reply
        .content
        .iter()
        .find_map(|part| match part {
            AssistantPart::ToolCall { id, name, input } => {
                Some((id.clone(), name.clone(), input.clone()))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        call,
        (
            "call_7".to_string(),
            "find_elements".to_string(),
            json!({ "name": "Store" })
        )
    );
    // The card starts under a stream id and learns the provider's id.
    let stream_id = events
        .iter()
        .find_map(|event| match event {
            Event::ToolCallStarted { stream_id, name } if name == "find_elements" => {
                Some(stream_id.clone())
            }
            _ => None,
        })
        .expect("a tool call started");
    let input: String = events
        .iter()
        .filter_map(|event| match event {
            Event::ToolInput {
                stream_id: id,
                json,
            } if *id == stream_id => Some(json.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        serde_json::from_str::<Value>(&input).unwrap(),
        json!({ "name": "Store" })
    );
    assert!(events.contains(&Event::ToolCallId {
        stream_id,
        id: "call_7".into()
    }));
    let _ = requests.recv().unwrap();

    // The next request carries the reasoning, the call and its result.
    let (url, requests) = serve(vec![ok(chunks(&[
        delta(json!({ "content": "Found it." })),
        finish("stop"),
    ]))]);
    let mut next = request(Provider::DeepSeek, "deepseek-flash");
    next.messages
        .push(Message::Assistant(reply.content.clone()));
    next.messages.push(Message::User(vec![UserPart::ToolResult {
        call_id: "call_7".into(),
        name: "find_elements".into(),
        content: "Shop::Store".into(),
        is_error: false,
    }]));
    let (_, reply) = finish_call(deepseek(&url).chat(next));
    assert_eq!(
        reply.unwrap().content,
        [AssistantPart::Text {
            text: "Found it.".into()
        }]
    );
    let (_, body) = requests.recv().unwrap();
    let messages = body["messages"].as_array().unwrap();
    let assistant = messages.iter().find(|m| m["role"] == "assistant").unwrap();
    assert_eq!(assistant["reasoning_content"], "Look it up.");
    assert_eq!(assistant["tool_calls"][0]["id"], "call_7");
    let tool = messages.iter().find(|m| m["role"] == "tool").unwrap();
    assert_eq!(tool["tool_call_id"], "call_7");
    assert!(tool["content"].to_string().contains("Shop::Store"));
}

#[test]
fn a_refused_key_is_explained_and_not_retried() {
    // A second answer is ready and must never be asked for.
    let (url, requests) = serve(vec![
        Answer {
            status: "401 Unauthorized",
            headers: "Content-Type: application/json\r\n",
            body: json!({ "error": { "message": "Authentication Fails", "type": "authentication_error" } }).to_string(),
            linger: Duration::ZERO,
        },
        ok(chunks(&[delta(json!({ "content": "Hello" })), finish("stop")])),
    ]);
    let (_, reply) =
        finish_call(deepseek(&url).chat(request(Provider::DeepSeek, "deepseek-flash")));
    let error = reply.unwrap_err();
    assert_eq!(error.kind, ErrorKind::KeyRefused);
    assert!(
        error.message.contains("DEEPSEEK_API_KEY"),
        "{}",
        error.message
    );
    assert!(requests.recv().is_ok());
    assert!(requests.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_rate_limit_is_retried_before_anything_streamed() {
    let (url, requests) = serve(vec![
        Answer {
            status: "429 Too Many Requests",
            headers: "Content-Type: application/json\r\nRetry-After: 1\r\n",
            body: json!({ "error": { "message": "slow down" } }).to_string(),
            linger: Duration::ZERO,
        },
        ok(chunks(&[
            delta(json!({ "content": "Hello" })),
            finish("stop"),
        ])),
    ]);
    let (_, reply) =
        finish_call(deepseek(&url).chat(request(Provider::DeepSeek, "deepseek-flash")));
    assert_eq!(
        reply.unwrap().content,
        [AssistantPart::Text {
            text: "Hello".into()
        }]
    );
    assert!(requests.recv().is_ok());
    assert!(requests.recv().is_ok());
}

#[test]
fn a_missing_key_says_which_variable_to_set() {
    let providers = Providers::new().with_endpoint(Provider::DeepSeek, "http://127.0.0.1:9");
    if providers.has_key(Provider::DeepSeek) {
        return; // A real key in the environment; nothing to check here.
    }
    let (_, reply) = finish_call(providers.chat(request(Provider::DeepSeek, "deepseek-flash")));
    let error = reply.unwrap_err();
    assert_eq!(error.kind, ErrorKind::MissingKey);
    assert!(error.message.contains("DEEPSEEK_API_KEY"));
}

#[test]
fn cancel_stops_a_silent_stream_at_once() {
    let body = chunks(&[
        delta(json!({ "content": "Hel" })),
        delta(json!({ "content": "lo" })),
        finish("stop"),
    ]);
    let (url, _requests) = serve(vec![Answer {
        linger: Duration::from_secs(10),
        ..ok(body)
    }]);
    let mut handle = deepseek(&url).chat(request(Provider::DeepSeek, "deepseek-flash"));
    // Wait until the call is under way, then cancel.
    std::thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    handle.cancel();
    loop {
        match handle.next_event(Duration::from_millis(10)) {
            Some(Event::Finished(result)) => {
                assert_eq!(result.unwrap_err().kind, ErrorKind::Cancelled);
                break;
            }
            Some(_) => {}
            None => assert!(
                started.elapsed() < Duration::from_millis(500),
                "cancel took too long"
            ),
        }
    }
    // Stop is meant to take effect within about 50 ms; the bound leaves room
    // for a slow CI machine.
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_stream_cut_off_before_its_end_is_a_lost_connection() {
    // No finish reason and no [DONE]: the reply is never used, even though
    // text and a whole tool call arrived.
    let body: String = [
        delta(json!({ "content": "Adding it." })),
        delta(json!({ "tool_calls": [{ "index": 0, "id": "call_1", "type": "function",
                                       "function": { "name": "find_elements", "arguments": "{}" } }] })),
    ]
    .iter()
    .map(|chunk| format!("data: {chunk}\n\n"))
    .collect();
    let (url, _requests) = serve(vec![ok(body)]);
    let (_, reply) =
        finish_call(deepseek(&url).chat(request(Provider::DeepSeek, "deepseek-flash")));
    let error = reply.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unreachable, "{}", error.message);
}

// The paths below cannot be tried live tonight (no keys): these canned
// streams follow each provider's documented format, as rig's own tests do.

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn anthropic_streams_summaries_text_and_a_tool_call() {
    let (url, requests) = serve(vec![ok(sse(&[
        json!({ "type": "message_start", "message": { "id": "msg_1", "type": "message", "role": "assistant", "content": [], "model": "claude-opus-5",
                "usage": { "input_tokens": 25, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 3000, "output_tokens": 1 } } }),
        json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "thinking", "thinking": "", "signature": "" } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "thinking_delta", "thinking": "Plan the store." } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "signature_delta", "signature": "sig" } }),
        json!({ "type": "content_block_stop", "index": 0 }),
        json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "text", "text": "" } }),
        json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "text_delta", "text": "Looking." } }),
        json!({ "type": "content_block_stop", "index": 1 }),
        json!({ "type": "content_block_start", "index": 2, "content_block": { "type": "tool_use", "id": "toolu_1", "name": "find_elements", "input": {} } }),
        json!({ "type": "content_block_delta", "index": 2, "delta": { "type": "input_json_delta", "partial_json": "{\"name\": \"Store\"}" } }),
        json!({ "type": "content_block_stop", "index": 2 }),
        // Anthropic reports cumulative usage on `message_delta`.
        json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use", "stop_sequence": null },
                "usage": { "input_tokens": 25, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 3000, "output_tokens": 40 } }),
        json!({ "type": "message_stop" }),
    ]))]);
    let providers = Providers::new()
        .with_key(Provider::Anthropic, "test-key")
        .with_endpoint(Provider::Anthropic, url);
    let (events, reply) =
        finish_call(providers.chat(request(Provider::Anthropic, "claude-opus-5")));
    let reply = reply.unwrap();
    assert_eq!(reply.stop, StopReason::ToolUse);
    assert!(events.contains(&Event::Thinking("Plan the store.".into())));
    assert!(
        matches!(&reply.content[0], AssistantPart::Reasoning(reasoning)
        if reasoning.parts == [ReasoningPart::Text { text: "Plan the store.".into(), signature: Some("sig".into()) }])
    );
    assert!(reply.content.contains(&AssistantPart::ToolCall {
        id: "toolu_1".into(),
        name: "find_elements".into(),
        input: json!({ "name": "Store" }),
    }));
    assert_eq!(reply.usage.cache_read_tokens, 3000);
    assert_eq!(reply.usage.cache_write_tokens, 100);
    assert_eq!(reply.usage.input_tokens, 25);
    let (path, body) = requests.recv().unwrap();
    assert!(path.ends_with("/v1/messages"), "{path}");
    assert_eq!(body["thinking"]["display"], "summarized");
    assert_eq!(body["output_config"]["effort"], "high");
    assert!(body.get("fallbacks").is_none());
    assert_eq!(body["max_tokens"], 4096);
}

#[test]
fn a_tool_call_whose_input_is_not_json_is_kept_for_an_error_result() {
    let (url, _requests) = serve(vec![ok(sse(&[
        json!({ "type": "message_start", "message": { "id": "msg_1", "type": "message", "role": "assistant", "content": [], "model": "claude-opus-5",
                "usage": { "input_tokens": 25, "output_tokens": 1 } } }),
        json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "tool_use", "id": "toolu_1", "name": "find_elements", "input": {} } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "input_json_delta", "partial_json": "{\"name\": Store" } }),
        json!({ "type": "content_block_stop", "index": 0 }),
        json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use", "stop_sequence": null },
                "usage": { "input_tokens": 25, "output_tokens": 9 } }),
        json!({ "type": "message_stop" }),
    ]))]);
    let providers = Providers::new()
        .with_key(Provider::Anthropic, "test-key")
        .with_endpoint(Provider::Anthropic, url);
    let (_, reply) = finish_call(providers.chat(request(Provider::Anthropic, "claude-opus-5")));
    let reply = reply.unwrap();
    assert_eq!(reply.stop, StopReason::ToolUse);
    let raw = reply
        .content
        .iter()
        .find_map(|part| match part {
            AssistantPart::ToolCall { name, input, .. } if name == "find_elements" => {
                Some(input.clone())
            }
            _ => None,
        })
        .expect("the call is kept");
    assert_eq!(raw, Value::String("{\"name\": Store".into()));
}

#[test]
fn an_environment_key_is_never_sent_to_another_endpoint() {
    let providers = Providers::new().with_endpoint(Provider::DeepSeek, "http://127.0.0.1:9");
    assert!(!providers.has_key(Provider::DeepSeek));
}

#[test]
fn openrouter_streams_reasoning_and_a_tool_call() {
    let chunk = |delta: Value, finish: Value| {
        json!({ "id": "gen-1", "object": "chat.completion.chunk", "model": "anthropic/claude-opus-5",
                "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }] })
    };
    let (url, requests) = serve(vec![ok(chunks(&[
        chunk(
            json!({ "role": "assistant", "reasoning": "Find it." }),
            Value::Null,
        ),
        chunk(
            json!({ "tool_calls": [{ "index": 0, "id": "call_9", "type": "function",
                                        "function": { "name": "find_elements", "arguments": "{\"name\":\"Store\"}" } }] }),
            Value::Null,
        ),
        json!({ "id": "gen-1", "object": "chat.completion.chunk", "model": "anthropic/claude-opus-5",
                "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }],
                "usage": { "prompt_tokens": 500, "completion_tokens": 30, "total_tokens": 530,
                           "prompt_tokens_details": { "cached_tokens": 100 } } }),
    ]))]);
    let providers = Providers::new()
        .with_key(Provider::OpenRouter, "test-key")
        .with_endpoint(Provider::OpenRouter, url);
    let (events, reply) =
        finish_call(providers.chat(request(Provider::OpenRouter, "anthropic/claude-opus-5")));
    let reply = reply.unwrap();
    assert_eq!(reply.stop, StopReason::ToolUse);
    assert!(events.contains(&Event::Thinking("Find it.".into())));
    assert!(reply.content.contains(&AssistantPart::ToolCall {
        id: "call_9".into(),
        name: "find_elements".into(),
        input: json!({ "name": "Store" }),
    }));
    assert_eq!(reply.usage.input_tokens, 400);
    assert_eq!(reply.usage.cache_read_tokens, 100);
    let (path, body) = requests.recv().unwrap();
    assert!(path.ends_with("/chat/completions"), "{path}");
    assert_eq!(body["reasoning"]["effort"], "high");
}

#[test]
fn openai_streams_a_tool_call_through_the_responses_api() {
    let call = json!({ "type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "find_elements",
                       "arguments": "{\"name\":\"Store\"}", "status": "completed" });
    let (url, requests) = serve(vec![ok(sse(&[
        json!({ "type": "response.output_item.added", "output_index": 0, "sequence_number": 1,
                "item": { "type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "find_elements", "arguments": "", "status": "in_progress" } }),
        json!({ "type": "response.function_call_arguments.delta", "item_id": "fc_1", "output_index": 0, "sequence_number": 2, "delta": "{\"name\":\"Store\"}" }),
        json!({ "type": "response.output_item.done", "output_index": 0, "sequence_number": 3, "item": call }),
        json!({ "type": "response.completed", "sequence_number": 4,
                "response": { "id": "resp_1", "object": "response", "created_at": 0, "status": "completed", "model": "gpt-6-astra",
                              "output": [call], "tools": [],
                              "usage": { "input_tokens": 300, "input_tokens_details": { "cached_tokens": 200 },
                                         "output_tokens": 20, "output_tokens_details": { "reasoning_tokens": 5 }, "total_tokens": 320 } } }),
    ]))]);
    let providers = Providers::new()
        .with_key(Provider::OpenAi, "test-key")
        .with_endpoint(Provider::OpenAi, url);
    let (_, reply) = finish_call(providers.chat(request(Provider::OpenAi, "gpt-6-astra")));
    let reply = reply.unwrap();
    assert_eq!(reply.stop, StopReason::ToolUse);
    assert!(reply.content.contains(&AssistantPart::ToolCall {
        id: "call_1".into(),
        name: "find_elements".into(),
        input: json!({ "name": "Store" }),
    }));
    assert_eq!(reply.usage.input_tokens, 100);
    assert_eq!(reply.usage.cache_read_tokens, 200);
    let (path, body) = requests.recv().unwrap();
    assert!(path.ends_with("/responses"), "{path}");
    assert_eq!(body["reasoning"]["effort"], "high");
}
