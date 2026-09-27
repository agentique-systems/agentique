//! The Claude API client without the network: event streams read from canned
//! text, and requests sent to a local server that answers like the API.

use agq_assistant::claude::{ClaudeModel, read_stream};
use agq_assistant::{Model, ModelError, Request, StreamEvent, Usage, tools};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// A server-sent event stream of the given data objects.
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

fn start() -> Value {
    json!({ "type": "message_start", "message": { "id": "msg_1", "type": "message", "role": "assistant", "content": [], "model": "claude-opus-5",
        "usage": { "input_tokens": 25, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 3000, "output_tokens": 1 } } })
}

fn block(index: u64, block: Value) -> Value {
    json!({ "type": "content_block_start", "index": index, "content_block": block })
}

fn delta(index: u64, delta: Value) -> Value {
    json!({ "type": "content_block_delta", "index": index, "delta": delta })
}

fn block_stop(index: u64) -> Value {
    json!({ "type": "content_block_stop", "index": index })
}

fn end(reason: &str) -> [Value; 2] {
    [
        json!({ "type": "message_delta", "delta": { "stop_reason": reason, "stop_sequence": null }, "usage": { "output_tokens": 12 } }),
        json!({ "type": "message_stop" }),
    ]
}

fn read(text: &str) -> (Result<agq_assistant::Reply, ModelError>, Vec<StreamEvent>) {
    let mut events = Vec::new();
    let stop = AtomicBool::new(false);
    let reply = read_stream(text.as_bytes(), &mut |event| events.push(event), &stop);
    (reply, events)
}

fn text_stream() -> String {
    let mut events = vec![
        start(),
        block(0, json!({ "type": "text", "text": "" })),
        json!({ "type": "ping" }),
        delta(0, json!({ "type": "text_delta", "text": "Hello" })),
        delta(0, json!({ "type": "text_delta", "text": " world" })),
        block_stop(0),
    ];
    events.extend(end("end_turn"));
    sse(&events)
}

#[test]
fn a_text_reply_streams_its_text() {
    let (reply, events) = read(&text_stream());
    let reply = reply.unwrap();
    assert_eq!(reply.stop_reason, "end_turn");
    assert_eq!(
        reply.content,
        [json!({ "type": "text", "text": "Hello world" })]
    );
    // The tokens used come last, from `message_start` and `message_delta`.
    assert_eq!(
        events,
        [
            StreamEvent::Text("Hello".into()),
            StreamEvent::Text(" world".into()),
            StreamEvent::Usage(Usage {
                input_tokens: 25,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 3000,
                output_tokens: 12,
            }),
        ]
    );
}

#[test]
fn windows_line_endings_are_read_too() {
    let (reply, _) = read(&text_stream().replace('\n', "\r\n"));
    assert_eq!(reply.unwrap().content[0]["text"], "Hello world");
}

#[test]
fn thinking_keeps_its_signature_for_sending_back() {
    let mut events = vec![
        start(),
        block(
            0,
            json!({ "type": "thinking", "thinking": "", "signature": "" }),
        ),
        delta(
            0,
            json!({ "type": "thinking_delta", "thinking": "Plan the parts." }),
        ),
        delta(
            0,
            json!({ "type": "signature_delta", "signature": "c2lnbmF0dXJl" }),
        ),
        block_stop(0),
        block(1, json!({ "type": "text", "text": "" })),
        delta(1, json!({ "type": "text_delta", "text": "Done." })),
        block_stop(1),
    ];
    events.extend(end("end_turn"));
    let (reply, stream) = read(&sse(&events));
    let reply = reply.unwrap();
    assert_eq!(
        reply.content[0],
        json!({ "type": "thinking", "thinking": "Plan the parts.", "signature": "c2lnbmF0dXJl" })
    );
    assert_eq!(reply.content[1]["text"], "Done.");
    // The thinking row learns that thinking started, then its summary.
    assert_eq!(stream[0], StreamEvent::Thinking(String::new()));
    assert_eq!(stream[1], StreamEvent::Thinking("Plan the parts.".into()));
}

#[test]
fn a_tool_call_input_is_put_together_from_its_pieces() {
    let mut events = vec![
        start(),
        block(
            0,
            json!({ "type": "tool_use", "id": "toolu_1", "name": "read_model", "input": {} }),
        ),
        delta(
            0,
            json!({ "type": "input_json_delta", "partial_json": "{\"elem" }),
        ),
        delta(
            0,
            json!({ "type": "input_json_delta", "partial_json": "ent\": \"UrlShortener\"}" }),
        ),
        block_stop(0),
    ];
    events.extend(end("tool_use"));
    let (reply, stream) = read(&sse(&events));
    let reply = reply.unwrap();
    assert_eq!(reply.stop_reason, "tool_use");
    assert_eq!(
        reply.content,
        [
            json!({ "type": "tool_use", "id": "toolu_1", "name": "read_model", "input": { "element": "UrlShortener" } })
        ]
    );
    assert_eq!(
        stream[..3],
        [
            StreamEvent::ToolCallStarted {
                id: "toolu_1".into(),
                name: "read_model".into()
            },
            StreamEvent::ToolInput {
                id: "toolu_1".into(),
                json: "{\"elem".into()
            },
            StreamEvent::ToolInput {
                id: "toolu_1".into(),
                json: "ent\": \"UrlShortener\"}".into()
            },
        ]
    );
    assert!(matches!(stream.last(), Some(StreamEvent::Usage(_))));
}

#[test]
fn two_tool_calls_after_text_keep_their_order() {
    let mut events = vec![
        start(),
        block(0, json!({ "type": "text", "text": "" })),
        delta(0, json!({ "type": "text_delta", "text": "Reading first." })),
        block_stop(0),
        block(
            1,
            json!({ "type": "tool_use", "id": "toolu_1", "name": "read_model", "input": {} }),
        ),
        block_stop(1),
        block(
            2,
            json!({ "type": "tool_use", "id": "toolu_2", "name": "find_elements", "input": {} }),
        ),
        delta(
            2,
            json!({ "type": "input_json_delta", "partial_json": "{\"name\": \"store\"}" }),
        ),
        block_stop(2),
    ];
    events.extend(end("tool_use"));
    let (reply, _) = read(&sse(&events));
    let content = reply.unwrap().content;
    assert_eq!(content.len(), 3);
    assert_eq!(content[1]["input"], json!({}));
    assert_eq!(content[2]["id"], "toolu_2");
    assert_eq!(content[2]["input"], json!({ "name": "store" }));
}

#[test]
fn an_unreadable_tool_input_is_kept_as_text() {
    let mut events = vec![
        start(),
        block(
            0,
            json!({ "type": "tool_use", "id": "toolu_1", "name": "apply_changes", "input": {} }),
        ),
        delta(
            0,
            json!({ "type": "input_json_delta", "partial_json": "{\"description\": \"Add \"fast\" links\"}" }),
        ),
        block_stop(0),
    ];
    events.extend(end("tool_use"));
    let (reply, _) = read(&sse(&events));
    assert_eq!(
        reply.unwrap().content[0]["input"],
        "{\"description\": \"Add \"fast\" links\"}"
    );
}

#[test]
fn an_error_event_is_explained() {
    let events = [
        start(),
        block(0, json!({ "type": "text", "text": "" })),
        delta(0, json!({ "type": "text_delta", "text": "Partly" })),
        json!({ "type": "error", "error": { "type": "overloaded_error", "message": "Overloaded" } }),
    ];
    let (reply, _) = read(&sse(&events));
    let Err(ModelError::Failed(message)) = reply else {
        panic!("expected a failure, got {reply:?}");
    };
    assert!(message.contains("overloaded"), "{message}");
}

#[test]
fn a_refusal_is_returned_as_its_stop_reason() {
    let events = [
        start(),
        json!({ "type": "message_delta", "delta": { "stop_reason": "refusal", "stop_details": { "type": "refusal", "category": "cyber" } }, "usage": { "output_tokens": 0 } }),
        json!({ "type": "message_stop" }),
    ];
    let (reply, _) = read(&sse(&events));
    let reply = reply.unwrap();
    assert_eq!(reply.stop_reason, "refusal");
    assert!(reply.content.is_empty());
}

#[test]
fn a_stream_cut_off_before_its_end_is_a_failure() {
    let events = [
        start(),
        block(0, json!({ "type": "text", "text": "" })),
        delta(0, json!({ "type": "text_delta", "text": "Half a" })),
    ];
    let (reply, _) = read(&sse(&events));
    let Err(ModelError::Failed(message)) = reply else {
        panic!("expected a failure, got {reply:?}");
    };
    assert!(
        message.contains("lost before the reply was complete"),
        "{message}"
    );
}

#[test]
fn a_stop_abandons_the_stream() {
    let stop = AtomicBool::new(true);
    let reply = read_stream(text_stream().as_bytes(), &mut |_| {}, &stop);
    assert_eq!(reply, Err(ModelError::Stopped));
}

#[test]
fn after_a_fallback_only_the_text_before_it_is_kept() {
    let mut events = vec![
        start(),
        block(
            0,
            json!({ "type": "thinking", "thinking": "", "signature": "" }),
        ),
        delta(0, json!({ "type": "signature_delta", "signature": "abc" })),
        block_stop(0),
        block(1, json!({ "type": "text", "text": "" })),
        delta(1, json!({ "type": "text_delta", "text": "I will " })),
        block_stop(1),
        block(
            2,
            json!({ "type": "tool_use", "id": "toolu_1", "name": "read_model", "input": {} }),
        ),
        block_stop(2),
        block(
            3,
            json!({ "type": "fallback", "from": { "model": "claude-opus-5" }, "to": { "model": "claude-opus-4-8" } }),
        ),
        block_stop(3),
        block(4, json!({ "type": "text", "text": "" })),
        delta(
            4,
            json!({ "type": "text_delta", "text": "read the model." }),
        ),
        block_stop(4),
    ];
    events.extend(end("end_turn"));
    let (reply, _) = read(&sse(&events));
    let content = reply.unwrap().content;
    assert_eq!(
        content,
        [
            json!({ "type": "text", "text": "I will " }),
            json!({ "type": "text", "text": "read the model." })
        ]
    );
}

// Requests over HTTP, to a local server.

fn request() -> Request {
    Request {
        system: "You are a test.".into(),
        tools: tools::definitions(),
        messages: vec![json!({ "role": "user", "content": [{ "type": "text", "text": "Hello" }] })],
    }
}

fn model(url: &str) -> ClaudeModel {
    let mut model = ClaudeModel::from_env();
    model.key = Some("test-key".into());
    model.model = "claude-opus-5".into();
    model.effort = "high".into();
    model.url = url.to_string();
    model
}

/// One canned response per connection, in order. Each received request (head
/// and body) is sent on the returned channel. `linger` keeps the last
/// connection open, silent, for that long after its response.
fn serve(responses: Vec<String>, linger: Duration) -> (String, Receiver<(String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/messages", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    std::thread::spawn(move || {
        let count = responses.len();
        for (index, response) in responses.into_iter().enumerate() {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut head = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let _ = sender.send((head, serde_json::from_slice(&body).unwrap_or(Value::Null)));
            let mut stream = reader.into_inner();
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            if index + 1 == count {
                std::thread::sleep(linger);
            }
        }
    });
    (url, requests)
}

fn ok(body: &str) -> String {
    format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n{body}")
}

fn status(code: u16, extra_headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {code} Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n{extra_headers}\r\n{body}",
        body.len()
    )
}

fn api_error(kind: &str, message: &str) -> String {
    json!({ "type": "error", "error": { "type": kind, "message": message } }).to_string()
}

fn send(model: &mut ClaudeModel) -> Result<agq_assistant::Reply, ModelError> {
    model.send(&request(), &mut |_| {}, &AtomicBool::new(false))
}

#[test]
fn a_missing_key_is_reported_without_a_request() {
    let mut model = model("http://127.0.0.1:9/unused");
    model.key = None;
    assert!(!model.has_key());
    assert_eq!(send(&mut model), Err(ModelError::MissingKey));
    assert!(
        ModelError::MissingKey
            .to_string()
            .contains("ANTHROPIC_API_KEY")
    );
}

#[test]
fn a_request_is_streamed_with_the_documented_settings() {
    let (url, requests) = serve(vec![ok(&text_stream())], Duration::ZERO);
    let mut model = model(&url);
    let reply = send(&mut model).unwrap();
    assert_eq!(reply.content[0]["text"], "Hello world");

    let (head, body) = requests.recv().unwrap();
    let head = head.to_lowercase();
    assert!(head.starts_with("post /v1/messages"), "{head}");
    assert!(head.contains("x-api-key: test-key"), "{head}");
    assert!(head.contains("anthropic-version: 2023-06-01"), "{head}");
    assert!(
        head.contains("anthropic-beta: server-side-fallback-2026-07-01"),
        "{head}"
    );
    assert_eq!(body["model"], "claude-opus-5");
    assert_eq!(body["stream"], true);
    assert_eq!(body["max_tokens"], 64000);
    assert_eq!(
        body["thinking"],
        json!({ "type": "adaptive", "display": "summarized" })
    );
    assert_eq!(body["output_config"], json!({ "effort": "high" }));
    assert_eq!(body["fallbacks"], "default");
    assert_eq!(body["cache_control"], json!({ "type": "ephemeral" }));
    assert_eq!(body["system"][0]["text"], "You are a test.");
    assert_eq!(
        body["system"][0]["cache_control"],
        json!({ "type": "ephemeral" })
    );
    assert_eq!(body["messages"], json!(request().messages));
    for tool in body["tools"].as_array().unwrap() {
        assert_eq!(tool["eager_input_streaming"], true, "{tool}");
        assert!(tool["input_schema"].is_object());
    }
}

#[test]
fn another_model_is_sent_without_fallbacks() {
    let (url, requests) = serve(vec![ok(&text_stream())], Duration::ZERO);
    let mut model = model(&url);
    model.model = "claude-sonnet-5".into();
    send(&mut model).unwrap();
    let (head, body) = requests.recv().unwrap();
    assert!(!head.to_lowercase().contains("anthropic-beta"), "{head}");
    assert!(body.get("fallbacks").is_none());
    assert_eq!(body["model"], "claude-sonnet-5");
}

#[test]
fn errors_are_explained_in_plain_words() {
    let cases = [
        (
            401,
            "",
            api_error("authentication_error", "invalid x-api-key"),
            "ANTHROPIC_API_KEY",
        ),
        (
            403,
            "",
            api_error("permission_error", "Not allowed in this region."),
            "Not allowed in this region.",
        ),
        (
            404,
            "",
            api_error("not_found_error", "model: claude-opus-5"),
            "`claude-opus-5` was not found",
        ),
        (
            413,
            "",
            api_error("request_too_large", "too large"),
            "too large for one request",
        ),
        (
            400,
            "",
            api_error("invalid_request_error", "messages: roles must alternate"),
            "roles must alternate",
        ),
        (
            429,
            "retry-after: 30\r\n",
            api_error("rate_limit_error", "slow down"),
            "Try again in 30 seconds",
        ),
    ];
    for (code, headers, body, expected) in cases {
        let (url, _) = serve(vec![status(code, headers, &body)], Duration::ZERO);
        let error = send(&mut model(&url)).unwrap_err();
        let ModelError::Failed(message) = error else {
            panic!("{code}: expected a failure, got {error:?}");
        };
        assert!(message.contains(expected), "{code}: {message}");
        assert!(message.contains(&code.to_string()), "{code}: {message}");
    }
}

#[test]
fn an_overloaded_service_is_tried_again() {
    let overloaded = status(529, "", &api_error("overloaded_error", "Overloaded"));
    let (url, requests) = serve(vec![overloaded, ok(&text_stream())], Duration::ZERO);
    let reply = send(&mut model(&url)).unwrap();
    assert_eq!(reply.content[0]["text"], "Hello world");
    assert_eq!(requests.try_iter().count(), 2);
}

#[test]
fn a_service_that_is_briefly_unavailable_is_tried_again() {
    for code in [500, 502, 503, 504] {
        let unavailable = status(code, "", &api_error("api_error", "unavailable"));
        let (url, requests) = serve(vec![unavailable, ok(&text_stream())], Duration::ZERO);
        let reply = send(&mut model(&url)).unwrap();
        assert_eq!(reply.content[0]["text"], "Hello world", "{code}");
        assert_eq!(requests.try_iter().count(), 2, "{code}");
    }
}

#[test]
fn a_long_rate_limit_wait_is_reported_not_waited_out() {
    let limited = status(
        429,
        "retry-after: 30
",
        &api_error("rate_limit_error", "slow down"),
    );
    let (url, requests) = serve(vec![limited, ok(&text_stream())], Duration::ZERO);
    let begun = Instant::now();
    let error = send(&mut model(&url)).unwrap_err();
    assert!(
        error.to_string().contains("Try again in 30 seconds"),
        "{error}"
    );
    assert!(begun.elapsed() < Duration::from_secs(2));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(requests.try_iter().count(), 1);
}

#[test]
fn an_unreachable_service_is_explained() {
    // Nothing listens on this port once the listener is gone.
    let url = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}/v1/messages", listener.local_addr().unwrap())
    };
    let error = send(&mut model(&url)).unwrap_err();
    let ModelError::Failed(message) = error else {
        panic!("expected a failure, got {error:?}");
    };
    assert!(
        message.contains("Could not reach the Claude API"),
        "{message}"
    );
}

#[test]
fn a_stop_ends_a_silent_reply_at_once() {
    // The server starts the reply, then says nothing for ten seconds.
    let started = sse(&[
        start(),
        block(
            0,
            json!({ "type": "thinking", "thinking": "", "signature": "" }),
        ),
    ]);
    let (url, _) = serve(vec![ok(&started)], Duration::from_secs(10));
    let mut model = model(&url);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let mut events = Vec::new();
    let begun = Instant::now();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        flag.store(true, Ordering::SeqCst);
    });
    let reply = model.send(&request(), &mut |event| events.push(event), &stop);
    assert_eq!(reply, Err(ModelError::Stopped));
    assert!(
        begun.elapsed() < Duration::from_secs(2),
        "{:?}",
        begun.elapsed()
    );
    assert_eq!(events, [StreamEvent::Thinking(String::new())]);
}
