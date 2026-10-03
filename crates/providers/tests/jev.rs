//! Jev's thin client without the network: a local server answers like the
//! API and records the request.

use agq_providers::jev::{Answer, DecisionRequest, Question, QuestionKind};
use agq_providers::{DecisionHandle, ErrorKind, Provider, Providers};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

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
    assert_eq!(reply.usage.input_tokens, Some(64));
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
    let failure = providers.decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::KeyRefused);
    assert!(failure.error.message.contains("TYPESAFE_API_KEY"));
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
            call.next_event(Duration::from_secs(5))
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
        let failure = providers.decide(&request()).unwrap_err();
        assert_eq!(failure.error.kind, ErrorKind::InvalidReply, "{failure}");
        assert!(failure.error.message.contains("larger than"), "{failure}");
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
    let failure = providers(url).decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::Unreachable, "{failure}");
    assert!(failure.error.message.contains("cut off"), "{failure}");
}

/// An error reply is read only up to its own limit and shown as a short,
/// single-line excerpt that never repeats the key.
#[test]
fn an_error_reply_is_shown_short_and_without_the_key() {
    let mut body = String::from("{\"error\":\"denied for test-key\n\tline two\u{7}");
    body.push_str(&"x".repeat(20_000));
    body.push_str("\"}");
    let (url, _) = serve(vec![("403 Forbidden", "", body)]);
    let failure = providers(url).decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::NoAccess);
    let message = &failure.error.message;
    assert!(!message.contains("test-key"), "{message}");
    assert!(!message.contains(['\n', '\t', '\u{7}']), "{message}");
    assert!(message.contains("denied for … line two"), "{message}");
    assert!(message.chars().count() < 450, "{}", message.chars().count());
    assert!(message.ends_with('…'), "{message}");
}

/// A reply that arrives but does not answer the request is an error with
/// its request counted and the usage it reported kept, never an answer.
#[test]
fn a_reply_that_does_not_fit_the_request_fails_with_its_usage() {
    let wrong_model = ANSWER.replace("jev-1.13.0", "jev-1.14.0");
    let unknown_option = ANSWER.replace("\"activate\"", "\"publish\"");
    let (url, requests) = serve(vec![
        ("200 OK", "x-typesafe-request-id: req_9\r\n", wrong_model),
        ("200 OK", "", unknown_option),
        ("200 OK", "", "not json".to_string()),
    ]);
    let providers = providers(url);
    let failure = providers.decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::InvalidReply, "{failure}");
    assert!(
        failure.error.message.contains("`jev-1.14.0` answered"),
        "{failure}"
    );
    assert_eq!(failure.attempts, 1);
    assert_eq!(failure.request_id.as_deref(), Some("req_9"));
    let usage = failure.usage.expect("the reply's usage is kept");
    assert_eq!(
        (usage.input_tokens, usage.output_tokens),
        (Some(64), Some(4))
    );
    let failure = providers.decide(&request()).unwrap_err();
    assert!(
        failure
            .error
            .message
            .contains("the option `activate` has no probability"),
        "{failure}"
    );
    // A malformed success is not retried.
    let failure = providers.decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::InvalidReply);
    assert_eq!(failure.attempts, 1);
    assert_eq!(failure.usage, None);
    assert_eq!(requests.try_iter().count(), 3);
}

/// Refused, unknown and invalid requests are errors, never a semantic
/// answer, and are not retried.
#[test]
fn error_statuses_are_errors_and_not_retried() {
    for (status, kind) in [
        ("400 Bad Request", ErrorKind::Rejected),
        ("401 Unauthorized", ErrorKind::KeyRefused),
        ("402 Payment Required", ErrorKind::NoAccess),
        ("403 Forbidden", ErrorKind::NoAccess),
        ("404 Not Found", ErrorKind::UnknownModel),
        ("413 Payload Too Large", ErrorKind::Rejected),
        ("422 Unprocessable Entity", ErrorKind::Rejected),
        ("500 Internal Server Error", ErrorKind::Unavailable),
        ("418 I'm a teapot", ErrorKind::Other),
    ] {
        let (url, requests) = serve(vec![
            (status, "", "{\"error\":\"no\"}".to_string()),
            ("200 OK", "", ANSWER.to_string()),
        ]);
        let failure = providers(url).decide(&request()).unwrap_err();
        assert_eq!(failure.error.kind, kind, "{status}: {failure}");
        assert_eq!(failure.attempts, 1, "{status}");
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(requests.try_iter().count(), 1, "{status} was retried");
    }
}

/// Rate limits and overload are retried twice at most, and a requested
/// wait over ten seconds is not waited for.
#[test]
fn retries_are_bounded() {
    let (url, requests) = serve(vec![
        ("529 Overloaded", "retry-after-ms: 10\r\n", "{}".to_string()),
        ("529 Overloaded", "retry-after-ms: 10\r\n", "{}".to_string()),
        ("529 Overloaded", "retry-after-ms: 10\r\n", "{}".to_string()),
        ("200 OK", "", ANSWER.to_string()),
    ]);
    let failure = providers(url).decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::Unavailable, "{failure}");
    assert_eq!(failure.attempts, 3);
    assert_eq!(requests.try_iter().count(), 3);

    let (url, requests) = serve(vec![
        (
            "429 Too Many Requests",
            "retry-after: 11\r\n",
            "{}".to_string(),
        ),
        ("200 OK", "", ANSWER.to_string()),
    ]);
    let started = Instant::now();
    let failure = providers(url).decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::RateLimited, "{failure}");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(failure.attempts, 1);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(requests.try_iter().count(), 1);

    let (url, _) = serve(vec![
        (
            "429 Too Many Requests",
            "retry-after-ms: 20\r\n",
            "{}".to_string(),
        ),
        ("200 OK", "", ANSWER.to_string()),
    ]);
    let reply = providers(url).decide(&request()).unwrap();
    assert_eq!(reply.attempts, 2);
}

/// Nothing is sent without a key of TypeSafe AI's own: not a blank one,
/// not another provider's.
#[test]
fn no_request_is_sent_without_its_own_key() {
    let (url, requests) = serve(vec![("200 OK", "", ANSWER.to_string())]);
    for providers in [
        Providers::new().with_endpoint(Provider::TypeSafe, url.clone()),
        Providers::new()
            .with_key(Provider::TypeSafe, "   ")
            .with_endpoint(Provider::TypeSafe, url.clone()),
        Providers::new()
            .with_key(Provider::DeepSeek, "deepseek-key")
            .with_endpoint(Provider::TypeSafe, url.clone()),
    ] {
        let failure = providers.decide(&request()).unwrap_err();
        assert_eq!(failure.error.kind, ErrorKind::MissingKey, "{failure}");
        assert_eq!(failure.attempts, 0);
        assert!(!format!("{providers:?}").contains("deepseek-key"));
    }
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(requests.try_iter().count(), 0);
}

/// An endpoint override never receives the ambient key: run in a child
/// process whose environment holds a TypeSafe AI key.
#[test]
fn an_ambient_key_never_reaches_an_endpoint_override() {
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ambient_key_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("TYPESAFE_API_KEY", "ambient-secret")
        .env("AGQ_AMBIENT_KEY_CHILD", "1")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&child.stdout);
    assert!(child.status.success(), "{out}");
    assert!(out.contains("1 passed"), "{out}");
}

/// The child of [`an_ambient_key_never_reaches_an_endpoint_override`].
#[test]
#[ignore = "run by an_ambient_key_never_reaches_an_endpoint_override in its own environment"]
fn ambient_key_child() {
    if std::env::var("AGQ_AMBIENT_KEY_CHILD").as_deref() != Ok("1") {
        return;
    }
    let (url, requests) = serve(vec![("200 OK", "", ANSWER.to_string())]);
    let overridden = Providers::new().with_endpoint(Provider::TypeSafe, url.clone());
    assert!(!overridden.has_key(Provider::TypeSafe));
    let failure = overridden.decide(&request()).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::MissingKey);
    assert!(!failure.to_string().contains("ambient-secret"));
    // An explicit key wins and is the one sent.
    let explicit = overridden.with_key(Provider::TypeSafe, "explicit");
    explicit.decide(&request()).unwrap();
    let (_, authorization, _) = requests.recv().unwrap();
    assert_eq!(authorization, "Bearer explicit");
}

// ---- Deadlines and cancellation (C-52 step 2) ----

/// What the local server does with one request.
enum Act {
    /// Writes these bytes.
    Send(Vec<u8>),
    /// Writes these bytes (perhaps none), then stays silent until the
    /// client goes away, and reports when it did.
    Hang(Vec<u8>),
    /// Waits, then writes these bytes.
    Later(Duration, Vec<u8>),
}

struct Server {
    url: String,
    /// One per request read.
    requests: Receiver<Instant>,
    /// When a hanging connection was closed by the client.
    gone: Receiver<Instant>,
    connections: Arc<AtomicUsize>,
}

/// A complete response that keeps the connection open.
fn kept(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// Reads one request; `false` when the connection ended first.
fn read_request(reader: &mut BufReader<std::net::TcpStream>) -> bool {
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return false;
    }
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap_or(0) == 0 {
            return false;
        }
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
    reader.read_exact(&mut body).is_ok()
}

/// Serves `acts` in order, several on one connection when the client
/// keeps it open.
fn serve_acts(acts: Vec<Act>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    let (left, gone) = mpsc::channel();
    let connections = Arc::new(AtomicUsize::new(0));
    let count = connections.clone();
    std::thread::spawn(move || {
        let mut acts = acts.into_iter();
        'connections: while let Ok((stream, _)) = listener.accept() {
            count.fetch_add(1, AtomicOrdering::SeqCst);
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut stream = stream;
            while read_request(&mut reader) {
                let _ = sender.send(Instant::now());
                let Some(act) = acts.next() else {
                    break 'connections;
                };
                match act {
                    Act::Send(bytes) => {
                        let _ = stream.write_all(&bytes);
                    }
                    Act::Later(wait, bytes) => {
                        std::thread::sleep(wait);
                        let _ = stream.write_all(&bytes);
                    }
                    Act::Hang(bytes) => {
                        let _ = stream.write_all(&bytes);
                        let _ = stream.flush();
                        let mut buffer = [0; 64];
                        while matches!(stream.read(&mut buffer), Ok(n) if n > 0) {}
                        let _ = left.send(Instant::now());
                        continue 'connections;
                    }
                }
            }
        }
    });
    Server {
        url,
        requests,
        gone,
        connections,
    }
}

/// Waits for a handle's result for at most `limit`.
fn wait_for(
    handle: &mut DecisionHandle,
    limit: Duration,
) -> Result<agq_providers::jev::DecisionReply, agq_providers::jev::DecisionFailure> {
    let until = Instant::now() + limit;
    loop {
        if let Some(result) = handle.next_result(Duration::from_millis(10)) {
            return result;
        }
        assert!(Instant::now() < until, "no result within {limit:?}");
    }
}

/// Scheduling allowance past a deadline on a busy machine.
const ALLOWANCE: Duration = Duration::from_millis(250);

/// A server that never answers: the decision ends at its deadline as
/// timed out, after one request, and the connection is closed.
#[test]
fn a_silent_server_ends_at_the_deadline() {
    let server = serve_acts(vec![Act::Hang(Vec::new())]);
    let providers = providers(server.url.clone());
    let started = Instant::now();
    let mut handle = providers.decide_start(request(), started + Duration::from_millis(300));
    let failure = wait_for(&mut handle, Duration::from_secs(5)).unwrap_err();
    let took = started.elapsed();
    assert_eq!(failure.error.kind, ErrorKind::TimedOut, "{failure}");
    assert!(took >= Duration::from_millis(290), "{took:?}");
    assert!(took < Duration::from_millis(300) + ALLOWANCE, "{took:?}");
    assert_eq!(failure.attempts, 1);
    server
        .gone
        .recv_timeout(Duration::from_secs(2))
        .expect("the connection was closed");
    assert_eq!(server.requests.try_iter().count(), 1);
}

/// Headers and half a body, then silence: the read shares the deadline.
#[test]
fn a_stalled_body_ends_at_the_deadline() {
    let mut half = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        ANSWER.len()
    )
    .into_bytes();
    half.extend_from_slice(&ANSWER.as_bytes()[..ANSWER.len() / 2]);
    let server = serve_acts(vec![Act::Hang(half)]);
    let providers = providers(server.url.clone());
    let started = Instant::now();
    let mut handle = providers.decide_start(request(), started + Duration::from_millis(300));
    let failure = wait_for(&mut handle, Duration::from_secs(5)).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::TimedOut, "{failure}");
    assert!(started.elapsed() < Duration::from_millis(300) + ALLOWANCE);
    server
        .gone
        .recv_timeout(Duration::from_secs(2))
        .expect("the connection was closed");
}

/// A reply that comes after the deadline is a timeout, not an answer.
#[test]
fn a_reply_after_the_deadline_is_a_timeout() {
    let server = serve_acts(vec![Act::Later(
        Duration::from_millis(400),
        response("200 OK", "", ANSWER),
    )]);
    let providers = providers(server.url.clone());
    let mut handle = providers.decide_start(request(), Instant::now() + Duration::from_millis(200));
    let failure = wait_for(&mut handle, Duration::from_secs(5)).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::TimedOut, "{failure}");
}

/// A retry whose wait would not fit before the deadline is not made, and
/// the failure says so at once.
#[test]
fn a_retry_that_would_not_fit_is_not_waited_for() {
    let server = serve_acts(vec![
        Act::Send(response("529 Overloaded", "retry-after-ms: 2000\r\n", "{}")),
        Act::Send(response("200 OK", "", ANSWER)),
    ]);
    let providers = providers(server.url.clone());
    let started = Instant::now();
    let mut handle = providers.decide_start(request(), started + Duration::from_millis(500));
    let failure = wait_for(&mut handle, Duration::from_secs(5)).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::Unavailable, "{failure}");
    assert!(
        failure.error.message.contains("No time was left"),
        "{failure}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(failure.attempts, 1);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(server.requests.try_iter().count(), 1);
}

/// A retry that fits is made within the same deadline.
#[test]
fn a_retry_that_fits_is_made() {
    let server = serve_acts(vec![
        Act::Send(response(
            "429 Too Many Requests",
            "retry-after-ms: 50\r\n",
            "{}",
        )),
        Act::Send(response("200 OK", "", ANSWER)),
    ]);
    let providers = providers(server.url.clone());
    let mut handle = providers.decide_start(request(), Instant::now() + Duration::from_secs(5));
    let reply = wait_for(&mut handle, Duration::from_secs(5)).unwrap();
    assert_eq!(reply.attempts, 2);
    assert_eq!(handle.attempts(), 2);
}

/// Stop during the wait before a retry: the result is at once a stop, and
/// no further request is ever sent.
#[test]
fn a_stop_during_a_retry_wait_sends_nothing_more() {
    let server = serve_acts(vec![
        Act::Send(response("529 Overloaded", "retry-after-ms: 1500\r\n", "{}")),
        Act::Send(response("200 OK", "", ANSWER)),
    ]);
    let providers = providers(server.url.clone());
    let mut handle = providers.decide_start(request(), Instant::now() + Duration::from_secs(10));
    server
        .requests
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let stopped = Instant::now();
    handle.cancel();
    let failure = wait_for(&mut handle, Duration::from_secs(5)).unwrap_err();
    assert!(
        stopped.elapsed() < Duration::from_millis(100),
        "{:?}",
        stopped.elapsed()
    );
    assert_eq!(failure.error.kind, ErrorKind::Cancelled);
    assert_eq!(failure.attempts, 1);
    std::thread::sleep(Duration::from_millis(1800));
    assert_eq!(
        server.requests.try_iter().count(),
        0,
        "a request after the stop"
    );
}

/// Dropping the handle stops the request: the connection closes.
#[test]
fn dropping_the_handle_closes_the_connection() {
    let server = serve_acts(vec![Act::Hang(Vec::new())]);
    let providers = providers(server.url.clone());
    let handle = providers.decide_start(request(), Instant::now() + Duration::from_secs(10));
    server
        .requests
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let dropped = Instant::now();
    drop(handle);
    let gone = server
        .gone
        .recv_timeout(Duration::from_secs(5))
        .expect("closed");
    assert!(gone.duration_since(dropped) < Duration::from_secs(1));
}

/// A reply already on its way when the Operator stops is never delivered;
/// neither is a provider error: the stop wins.
#[test]
fn a_stop_wins_over_a_reply_or_an_error_already_on_its_way() {
    for answer in [
        response("200 OK", "", ANSWER),
        response("500 Oops", "", "{}"),
    ] {
        let server = serve_acts(vec![Act::Send(answer)]);
        let providers = providers(server.url.clone());
        let mut handle =
            providers.decide_start(request(), Instant::now() + Duration::from_secs(10));
        server
            .requests
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        // The result has arrived; nobody has taken it yet.
        std::thread::sleep(Duration::from_millis(300));
        handle.cancel();
        let failure = wait_for(&mut handle, Duration::from_secs(1)).unwrap_err();
        assert_eq!(failure.error.kind, ErrorKind::Cancelled, "{failure}");
        assert!(handle.next_result(Duration::from_millis(10)).is_none());
    }
}

/// A deadline that has already passed sends nothing.
#[test]
fn a_deadline_already_passed_sends_nothing() {
    let server = serve_acts(vec![Act::Send(response("200 OK", "", ANSWER))]);
    let providers = providers(server.url.clone());
    let mut handle = providers.decide_start(request(), Instant::now());
    let failure = wait_for(&mut handle, Duration::from_secs(5)).unwrap_err();
    assert_eq!(failure.error.kind, ErrorKind::TimedOut, "{failure}");
    assert_eq!(failure.attempts, 0);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(server.requests.try_iter().count(), 0);
}

/// Decisions share one client: the second goes over the first's
/// connection, with the handle API and the blocking one alike.
#[test]
fn connections_are_reused() {
    let server = serve_acts(vec![
        Act::Send(kept(ANSWER)),
        Act::Send(kept(ANSWER)),
        Act::Send(kept(ANSWER)),
    ]);
    let providers = providers(server.url.clone());
    providers.decide(&request()).unwrap();
    let mut handle = providers.decide_start(request(), Instant::now() + Duration::from_secs(5));
    wait_for(&mut handle, Duration::from_secs(5)).unwrap();
    providers.decide(&request()).unwrap();
    assert_eq!(server.requests.try_iter().count(), 3);
    assert_eq!(server.connections.load(AtomicOrdering::SeqCst), 1);
}
