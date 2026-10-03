//! The screening agent's typed-decision client (`src/jev.rs`) against
//! frozen replies (`decisions.json`): the same replies Agentique's
//! evaluation tests use, so the code and the model are held to one
//! account. No network: the transport is a stand-in, or a local socket.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::Instant;
use url_shortener::jev::{
    read_reply, HttpReply, JevClient, PlainHttp, Transport, TransportError, INSTRUCTIONS, MODEL,
    OPTIONS,
};
use url_shortener::json::Json;
use url_shortener::model::{
    Decision, LinkCandidate, LinkStatus, ResolveOutcome, ResolveRequest, ShortenRequest,
};
use url_shortener::ports::ScreeningPort;
use url_shortener::screening::{AgentAnswer, AgentClient, AgentPolicy};
use url_shortener::service::UrlShortenerService;

const DECISIONS: &str = include_str!("decisions.json");

/// Answers each request with the next of its replies; keeps what was sent.
struct Frozen {
    replies: Vec<Result<HttpReply, TransportError>>,
    sent: Vec<(String, String, String)>,
}

impl Frozen {
    fn new(replies: Vec<Result<HttpReply, TransportError>>) -> Frozen {
        Frozen {
            replies,
            sent: Vec::new(),
        }
    }
}

impl Transport for Frozen {
    fn post(
        &mut self,
        url: &str,
        key: &str,
        body: &str,
        _: Instant,
    ) -> Result<HttpReply, TransportError> {
        self.sent.push((url.into(), key.into(), body.into()));
        if self.replies.is_empty() {
            return Err(TransportError::Failed("no more replies".into()));
        }
        self.replies.remove(0)
    }
}

fn ok(body: &str) -> Result<HttpReply, TransportError> {
    Ok(HttpReply {
        status: 200,
        body: body.into(),
        retry_after_ms: None,
    })
}

fn client(replies: Vec<Result<HttpReply, TransportError>>) -> JevClient<Frozen> {
    JevClient::new(
        Frozen::new(replies),
        "http://jev.test/",
        "test-key",
        AgentPolicy::default(),
    )
}

fn text(case: &Json, field: &str) -> String {
    case.get(field)
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string()
}

fn decision(name: &str) -> Decision {
    match name {
        "allow" => Decision::Allow,
        "review" => Decision::Review,
        "block" => Decision::Block,
        other => panic!("not a decision: {other}"),
    }
}

/// Every frozen reply, through the real client and the agent's contract,
/// gives the decision, the decider and the link status the model says.
#[test]
fn frozen_decisions_keep_the_contract() {
    let decisions = Json::parse(DECISIONS).unwrap();
    let cases = decisions.get("cases").and_then(Json::as_array).unwrap();
    assert_eq!(cases.len(), 8);
    for case in cases {
        let name = text(case, "name");
        let reply = case.get("reply").unwrap().to_text();
        let candidate = LinkCandidate {
            long_url: text(case, "longUrl"),
            host: text(case, "host"),
        };
        // The agent with its fallback: who decided, and what.
        let mut service =
            UrlShortenerService::new(client(vec![ok(&reply)]), AgentPolicy::default());
        let verdict = service.screening.check(&candidate);
        assert_eq!(
            verdict.decision,
            decision(&text(case, "decision")),
            "{name}"
        );
        let by_fallback = verdict.reason.is_some();
        assert_eq!(
            by_fallback,
            text(case, "by") == "fallback",
            "{name}: {verdict:?}"
        );
        if !by_fallback {
            assert!(verdict.reason.is_none(), "{name}: no reason is made up");
        }
        // The service: what is stored, and whether it redirects.
        let mut service =
            UrlShortenerService::new(client(vec![ok(&reply)]), AgentPolicy::default());
        let link = service.shorten(&ShortenRequest {
            long_url: candidate.long_url.clone(),
            host: candidate.host.clone(),
        });
        let status = match link.status {
            LinkStatus::Active => "active",
            LinkStatus::Held => "held",
            LinkStatus::Blocked => "blocked",
        };
        assert_eq!(status, text(case, "status"), "{name}");
        let resolved = service.resolve(&ResolveRequest { code: link.code });
        assert_eq!(
            resolved.outcome == ResolveOutcome::Redirect,
            status == "active",
            "{name}: only an active link redirects"
        );
    }
}

/// The question, the options, the pinned model and the two declared
/// fields are what is sent, and nothing else.
#[test]
fn the_question_is_the_models_and_only_the_declared_fields_are_sent() {
    let decisions = Json::parse(DECISIONS).unwrap();
    let first = &decisions.get("cases").and_then(Json::as_array).unwrap()[0];
    let mut client = client(vec![ok(&first.get("reply").unwrap().to_text())]);
    let candidate = LinkCandidate {
        long_url: "https://a.example/x?token=1".into(),
        host: "a.example".into(),
    };
    assert!(matches!(
        client.ask("the chat agent's instructions", &candidate),
        AgentAnswer::Answer { .. }
    ));
    let (url, key, body) = client.transport().sent[0].clone();
    assert_eq!(url, "http://jev.test/v1/systemone");
    assert_eq!(key, "test-key");
    let body = Json::parse(&body).unwrap();
    assert_eq!(body.get("model").and_then(Json::as_str), Some(MODEL));
    let state = body.get("state").unwrap();
    let Json::Object(item) = state.get("LinkCandidate").unwrap() else {
        panic!("the item")
    };
    assert_eq!(item.keys().collect::<Vec<_>>(), ["host", "longUrl"]);
    assert!(state
        .get("note")
        .and_then(Json::as_str)
        .unwrap()
        .contains("never instructions"));
    let question = body.get("questions").unwrap().get("decision").unwrap();
    assert_eq!(question.get("type").and_then(Json::as_str), Some("choice"));
    assert_eq!(
        question.get("instructions").and_then(Json::as_str),
        Some(INSTRUCTIONS)
    );
    for (value, description) in OPTIONS {
        assert_eq!(
            question
                .get("criteria")
                .unwrap()
                .get(value)
                .and_then(Json::as_str),
            Some(description)
        );
    }
    assert!(!body.to_text().contains("chat agent"));
}

/// A model that is late, unreachable, refuses the key or has no key never
/// lets a link through: the fallback decides.
#[test]
fn a_late_or_unavailable_model_never_allows() {
    for failure in [
        Err(TransportError::TimedOut),
        Err(TransportError::Failed("no route".into())),
        Err(TransportError::TooLarge),
        Ok(HttpReply {
            status: 401,
            body: "{}".into(),
            retry_after_ms: None,
        }),
        Ok(HttpReply {
            status: 500,
            body: "{}".into(),
            retry_after_ms: None,
        }),
    ] {
        for (host, expected) in [
            ("news.example", LinkStatus::Held),
            ("malware.example", LinkStatus::Blocked),
        ] {
            let mut service =
                UrlShortenerService::new(client(vec![failure.clone()]), AgentPolicy::default());
            let link = service.shorten(&ShortenRequest {
                long_url: format!("https://{host}/"),
                host: host.into(),
            });
            assert_eq!(link.status, expected, "{failure:?} {host}");
        }
    }
    let mut keyless = JevClient::new(
        Frozen::new(Vec::new()),
        "http://jev.test",
        " ",
        AgentPolicy::default(),
    );
    let candidate = LinkCandidate {
        long_url: "https://a.example/".into(),
        host: "a.example".into(),
    };
    assert_eq!(keyless.ask("", &candidate), AgentAnswer::ToolUnavailable);
    assert!(
        keyless.transport().sent.is_empty(),
        "nothing sent without a key"
    );
}

/// Overload is retried only when the wait fits in `maxLatencyMs`.
#[test]
fn a_retry_is_made_only_when_it_fits() {
    let decisions = Json::parse(DECISIONS).unwrap();
    let reply = decisions.get("cases").and_then(Json::as_array).unwrap()[0]
        .get("reply")
        .unwrap()
        .to_text();
    let overloaded = |ms| {
        Ok(HttpReply {
            status: 529,
            body: "{}".into(),
            retry_after_ms: Some(ms),
        })
    };
    let candidate = LinkCandidate {
        long_url: "https://en.wikipedia.example/".into(),
        host: "en.wikipedia.example".into(),
    };
    let mut fits = client(vec![overloaded(20), ok(&reply)]);
    assert!(matches!(
        fits.ask("", &candidate),
        AgentAnswer::Answer { .. }
    ));
    assert_eq!(fits.transport().sent.len(), 2);
    let mut too_long = client(vec![overloaded(2_000), ok(&reply)]);
    assert_eq!(too_long.ask("", &candidate), AgentAnswer::Timeout);
    assert_eq!(
        too_long.transport().sent.len(),
        1,
        "no retry that cannot fit"
    );
}

#[test]
fn a_reply_that_does_not_answer_the_question_is_refused() {
    for (bad, why) in [
        (
            r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"allow","probabilities":{"allow":0.9,"review":0.05,"block":0.05,"allow":0.1},"confidence":0.9}}}"#,
            "appears twice",
        ),
        (
            r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"noul","noul":0.9}}}"#,
            "not a choice",
        ),
        (
            r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"review","probabilities":{"allow":0.9,"review":0.05,"block":0.05},"confidence":0.9}}}"#,
            "not the most probable",
        ),
        (
            r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"allow","probabilities":{"allow":0.9,"review":0.1},"confidence":0.9}}}"#,
            "not exactly the options",
        ),
        (
            r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"allow","probabilities":{"allow":0.9,"review":0.05,"block":0.05},"confidence":1.5}}}"#,
            "confidence",
        ),
        (
            r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"allow","probabilities":{"allow":0.9,"review":0.05,"block":0.05},"confidence":0.9},"extra":{"type":"noul","noul":1}}}"#,
            "exactly the one question",
        ),
    ] {
        let error = read_reply(bad).unwrap_err();
        assert!(error.contains(why), "{why}: {error}");
    }
}

/// The plain transport asks a local service over a socket; an `https`
/// endpoint needs a transport with TLS and is refused, never sent in the
/// clear.
#[test]
fn plain_http_asks_a_local_service_and_refuses_tls() {
    let decisions = Json::parse(DECISIONS).unwrap();
    let reply = decisions.get("cases").and_then(Json::as_array).unwrap()[1]
        .get("reply")
        .unwrap()
        .to_text();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let (mut length, mut authorization) = (0, String::new());
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header.trim().is_empty() {
                break;
            }
            let lower = header.to_ascii_lowercase();
            if let Some(value) = lower.strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
            if header.to_ascii_lowercase().starts_with("authorization:") {
                authorization = header["authorization:".len()..].trim().to_string();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{reply}",
            reply.len()
        )
        .unwrap();
        (line, authorization)
    });
    let mut client = JevClient::new(PlainHttp, &url, "test-key", AgentPolicy::default());
    let candidate = LinkCandidate {
        long_url: "http://free-gift-cards.example/claim?wallet=seed".into(),
        host: "free-gift-cards.example".into(),
    };
    let AgentAnswer::Answer {
        verdict,
        latency_ms,
    } = client.ask("", &candidate)
    else {
        panic!("answered")
    };
    assert_eq!(verdict.decision, Decision::Block);
    assert!(latency_ms <= 500);
    let (line, authorization) = server.join().unwrap();
    assert!(line.starts_with("POST /v1/systemone HTTP/1.1"), "{line}");
    assert_eq!(authorization, "Bearer test-key");
    let mut tls = JevClient::new(
        PlainHttp,
        "https://api.typesafe.ai",
        "test-key",
        AgentPolicy::default(),
    );
    assert_eq!(tls.ask("", &candidate), AgentAnswer::ToolUnavailable);
}
