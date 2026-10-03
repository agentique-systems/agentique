//! Jev's thin client without the network: a local server answers like the
//! API and records the request.

use agq_providers::jev::{Answer, DecisionRequest, Question, QuestionKind};
use agq_providers::{ErrorKind, Provider, Providers};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};

/// A complete HTTP response with a JSON body.
fn response(status: &str, headers: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// A response whose body is sent in these chunks (chunked encoding).
fn chunked(status: &str, chunks: &[&[u8]]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    for chunk in chunks {
        out.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

/// Serves `(status line, extra headers, body)` answers in order; reports
/// each request's path, authorization header and JSON body.
fn serve(
    answers: Vec<(&'static str, &'static str, String)>,
) -> (String, Receiver<(String, String, Value)>) {
    serve_raw(
        answers
            .into_iter()
            .map(|(status, headers, body)| response(status, headers, &body))
            .collect(),
    )
}

/// Serves raw responses in order, one connection each; the connection
/// closes after the bytes are written.
fn serve_raw(answers: Vec<Vec<u8>>) -> (String, Receiver<(String, String, Value)>) {
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
            let _ = stream.write_all(&answer);
            let _ = stream.flush();
        }
    });
    (url, requests)
}

fn providers(url: String) -> Providers {
    Providers::new()
        .with_key(Provider::TypeSafe, "test-key")
        .with_endpoint(Provider::TypeSafe, url)
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

/// Regression (A1): a successful reply was cut to 300 characters before it
/// was parsed, so any realistic reply longer than that failed.
#[test]
fn a_reply_longer_than_300_characters_is_read_whole() {
    let mut request = request();
    request.questions.insert(
        "risk".into(),
        Question {
            instructions: "How risky is the link?".into(),
            kind: QuestionKind::Score {
                levels: vec!["low".into(), "medium".into(), "high".into()],
            },
        },
    );
    request.questions.insert(
        "suspicious".into(),
        Question {
            instructions: "Is the link likely abuse?".into(),
            kind: QuestionKind::YesNo { meanings: None },
        },
    );
    let answer = r#"{"model":"jev-1.13.0","answers":{"action":{"type":"choice","choice":"hold","probabilities":{"hold":0.91,"activate":0.09},"confidence":0.88},"risk":{"type":"score","score":1.2,"legend":{"0":"low","1":"medium","2":"high"},"probabilities":{"0":0.1,"1":0.6,"2":0.3},"confidence":0.5},"suspicious":{"type":"noul","noul":0.93}},"usage":{"input_tokens":120,"output_tokens":8}}"#;
    assert!(answer.len() > 300, "{}", answer.len());
    let (url, _) = serve(vec![("200 OK", "", answer.to_string())]);
    let providers = Providers::new()
        .with_key(Provider::TypeSafe, "test-key")
        .with_endpoint(Provider::TypeSafe, url);
    let reply = providers.decide(&request).unwrap();
    assert_eq!(reply.answers.len(), 3);
    assert!(matches!(&reply.answers["risk"], Answer::Score { score, .. } if *score == 1.2));
    assert_eq!(reply.answers["suspicious"], Answer::YesNo { yes: 0.93 });
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

/// A reply sent in chunks, one splitting a two-byte character, is read
/// whole; the option's name keeps its non-ASCII letters.
#[test]
fn a_chunked_reply_with_non_ascii_text_is_read_whole() {
    let mut request = request();
    request.questions.insert(
        "åtgärd".into(),
        Question {
            instructions: "Släpp länken eller håll den för granskning?".into(),
            kind: QuestionKind::Choice {
                options: BTreeMap::from([
                    ("släpp".to_string(), Some("Länken går live".to_string())),
                    (
                        "håll".to_string(),
                        Some("En människa granskar den".to_string()),
                    ),
                ]),
            },
        },
    );
    let body = r#"{"model":"jev-1.13.0","answers":{"action":{"type":"choice","choice":"hold","probabilities":{"hold":0.91,"activate":0.09},"confidence":0.88},"åtgärd":{"type":"choice","choice":"håll","probabilities":{"håll":0.8,"släpp":0.2},"confidence":0.7}},"usage":{"input_tokens":64,"output_tokens":4}}"#;
    let bytes = body.as_bytes();
    let split = body.find("håll").unwrap() + 1; // inside the two-byte `å`
    let (url, _) = serve_raw(vec![chunked(
        "200 OK",
        &[&bytes[..40], &bytes[40..split], &bytes[split..]],
    )]);
    let reply = providers(url).decide(&request).unwrap();
    assert!(matches!(&reply.answers["åtgärd"], Answer::Choice { choice, .. } if choice == "håll"));
}

/// A successful reply over the local limit is refused, not cut and parsed,
/// whether its length is announced or only found while reading.
#[test]
fn a_reply_over_the_limit_is_refused() {
    let limit = agq_providers::jev::REPLY_LIMIT;
    let announced = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{{",
        limit + 1
    )
    .into_bytes();
    let padding = vec![b' '; 64 * 1024];
    let mut pieces: Vec<&[u8]> = vec![b"{\"model\":\"jev-1.13.0\","];
    for _ in 0..(limit / padding.len() + 1) {
        pieces.push(&padding);
    }
    let streamed = chunked("200 OK", &pieces);
    let (url, requests) = serve_raw(vec![announced, streamed]);
    let providers = providers(url);
    for _ in 0..2 {
        let error = providers.decide(&request()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidReply, "{error}");
        assert!(error.message.contains("larger than"), "{error}");
    }
    assert_eq!(requests.try_iter().count(), 2);
}

/// A reply cut off before its announced end is an unreachable provider,
/// not an answer.
#[test]
fn a_reply_cut_off_while_read_is_an_error() {
    let cut = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        ANSWER.len() + 100,
        ANSWER
    )
    .into_bytes();
    let (url, _) = serve_raw(vec![cut]);
    let error = providers(url).decide(&request()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unreachable, "{error}");
    assert!(error.message.contains("cut off"), "{error}");
}

/// An error reply is read only up to its own limit and shown as a short,
/// single-line excerpt that never repeats the key.
#[test]
fn an_error_reply_is_shown_short_and_without_the_key() {
    let mut body = String::from("{\"error\":\"denied for test-key\n\tline two\u{7}");
    body.push_str(&"x".repeat(20_000));
    body.push_str("\"}");
    let (url, _) = serve(vec![("403 Forbidden", "", body)]);
    let error = providers(url).decide(&request()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::NoAccess);
    let message = &error.message;
    assert!(!message.contains("test-key"), "{message}");
    assert!(!message.contains(['\n', '\t', '\u{7}']), "{message}");
    assert!(message.contains("denied for … line two"), "{message}");
    assert!(message.chars().count() < 450, "{}", message.chars().count());
    assert!(message.ends_with('…'), "{message}");
}
