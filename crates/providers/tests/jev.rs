//! Jev's thin client without the network: a local server answers like the
//! API and records the request.

use agq_providers::jev::{Answer, DecisionRequest, Question, QuestionKind};
use agq_providers::{ErrorKind, Provider, Providers};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};

/// Serves `(status line, extra headers, body)` answers in order; reports
/// each request's path, authorization header and JSON body.
fn serve(
    answers: Vec<(&'static str, &'static str, String)>,
) -> (String, Receiver<(String, String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, headers, body) in answers {
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
            let (mut length, mut authorization) = (0, String::new());
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap();
                    }
                    if name.eq_ignore_ascii_case("authorization") {
                        authorization = value.trim().to_string();
                    }
                }
            }
            let mut request = vec![0; length];
            reader.read_exact(&mut request).unwrap();
            let _ = sender.send((
                path,
                authorization,
                serde_json::from_slice(&request).unwrap_or(Value::Null),
            ));
            let mut stream = stream;
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (url, requests)
}

fn request() -> DecisionRequest {
    DecisionRequest {
        model: "jev-1.13.0".into(),
        state: json!("Short link to https://example.test/prize created by a new account."),
        questions: BTreeMap::from([(
            "action".to_string(),
            Question {
                instructions: "Activate the link or hold it for review?".into(),
                kind: QuestionKind::Choice {
                    options: BTreeMap::from([
                        ("activate".to_string(), None),
                        ("hold".to_string(), None),
                    ]),
                },
            },
        )]),
    }
}

const ANSWER: &str = r#"{"model":"jev-1.13.0","answers":{"action":{"type":"choice","choice":"hold","probabilities":{"hold":0.91,"activate":0.09},"confidence":0.88}},"usage":{"input_tokens":64,"output_tokens":4}}"#;

#[test]
fn a_decision_is_asked_and_read() {
    let (url, requests) = serve(vec![(
        "200 OK",
        "x-typesafe-request-id: req_7\r\n",
        ANSWER.to_string(),
    )]);
    let providers = Providers::new()
        .with_key(Provider::TypeSafe, "test-key")
        .with_endpoint(Provider::TypeSafe, url);
    let reply = providers.decide(&request()).unwrap();
    assert!(matches!(&reply.answers["action"], Answer::Choice { choice, .. } if choice == "hold"));
    assert_eq!(reply.request_id.as_deref(), Some("req_7"));
    assert_eq!(reply.usage.input_tokens, 64);
    let (path, authorization, body) = requests.recv().unwrap();
    assert_eq!(path, "/v1/systemone");
    assert_eq!(authorization, "Bearer test-key");
    assert_eq!(body["questions"]["action"]["type"], "choice");
}

#[test]
fn overload_is_retried_and_a_refused_key_is_not() {
    let (url, requests) = serve(vec![
        ("529 Overloaded", "retry-after-ms: 50\r\n", "{}".to_string()),
        ("200 OK", "", ANSWER.to_string()),
    ]);
    let providers = Providers::new()
        .with_key(Provider::TypeSafe, "test-key")
        .with_endpoint(Provider::TypeSafe, url);
    assert!(providers.decide(&request()).is_ok());
    assert_eq!(requests.try_iter().count(), 2);

    let (url, _) = serve(vec![("401 Unauthorized", "", "{}".to_string())]);
    let providers = Providers::new()
        .with_key(Provider::TypeSafe, "bad")
        .with_endpoint(Provider::TypeSafe, url);
    let error = providers.decide(&request()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::KeyRefused);
    assert!(error.message.contains("TYPESAFE_API_KEY"));
}

#[test]
fn jev_is_never_the_assistants_model() {
    let providers = Providers::new()
        .with_key(Provider::TypeSafe, "test-key")
        .with_endpoint(Provider::TypeSafe, "http://127.0.0.1:9");
    let mut call = providers.chat(agq_providers::ChatRequest {
        model: agq_providers::ModelRef::new(Provider::TypeSafe, "jev-1.13.0"),
        effort: None,
        max_output_tokens: 100,
        system: String::new(),
        tools: Vec::new(),
        messages: Vec::new(),
    });
    loop {
        if let Some(agq_providers::Event::Finished(result)) =
            call.next_event(std::time::Duration::from_secs(5))
        {
            assert_eq!(result.unwrap_err().kind, ErrorKind::Rejected);
            break;
        }
    }
}
