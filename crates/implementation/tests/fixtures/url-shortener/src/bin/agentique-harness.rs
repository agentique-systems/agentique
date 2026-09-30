//! Agentique's harness for the URL shortener: runs the real service, with
//! the screening agent's model answered by the scenario's stand-ins, and
//! speaks the harness protocol (one JSON line per command) on standard
//! input and output. It calls no model itself: a scenario whose agent has
//! no stand-in is refused, not guessed.

use std::io::{BufRead, Write};
use url_shortener::json::Json;
use url_shortener::model::{
    Decision, LinkCandidate, LinkStatus, Resolution, ResolveOutcome, ResolveRequest,
    ReviewDecision, ShortLink, ShortenRequest, Verdict,
};
use url_shortener::screening::{AgentAnswer, AgentClient, AgentPolicy};
use url_shortener::service::UrlShortenerService;

/// The agent's model, answered by the stand-ins, call by call.
struct StandInClient {
    answers: Vec<Json>,
    calls: u32,
}

impl AgentClient for StandInClient {
    fn ask(&mut self, _instructions: &str, _candidate: &LinkCandidate) -> AgentAnswer {
        self.calls += 1;
        let call = f64::from(self.calls);
        let Some(answer) = self
            .answers
            .iter()
            .find(|a| a.get("call").and_then(Json::as_f64) == Some(call))
            .or_else(|| self.answers.iter().find(|a| a.get("call").is_none_or(Json::is_null)))
        else {
            return AgentAnswer::ToolUnavailable;
        };
        let latency_ms = answer.get("latencyMs").and_then(Json::as_f64).unwrap_or(0.0) as u64;
        match answer.get("outcome").and_then(Json::as_str) {
            Some("answer") => match answer.get("output").and_then(verdict) {
                Some(verdict) => AgentAnswer::Answer { verdict, latency_ms },
                None => AgentAnswer::InvalidOutput,
            },
            Some("timeout") => AgentAnswer::Timeout,
            Some("refusal") => AgentAnswer::Refusal,
            Some("toolUnavailable") => AgentAnswer::ToolUnavailable,
            _ => AgentAnswer::InvalidOutput,
        }
    }
}

fn verdict(value: &Json) -> Option<Verdict> {
    let fields = value.get("fields")?;
    let decision = match fields.get("decision")?.as_str()? {
        "allow" => Decision::Allow,
        "review" => Decision::Review,
        "block" => Decision::Block,
        _ => return None,
    };
    Some(Verdict {
        decision,
        confidence: fields.get("confidence").and_then(Json::as_f64),
        reason: fields.get("reason").and_then(Json::as_str).map(str::to_string),
    })
}

fn text(value: &Json, field: &str) -> String {
    value
        .get("fields")
        .and_then(|f| f.get(field))
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string()
}

fn item(ty: &str, fields: Vec<(&str, Json)>) -> Json {
    Json::object(vec![
        ("type", Json::String(ty.into())),
        ("fields", Json::object(fields)),
    ])
}

fn link(link: &ShortLink) -> Json {
    item(
        "ShortLink",
        vec![
            ("code", Json::String(link.code.clone())),
            ("longUrl", Json::String(link.long_url.clone())),
            (
                "status",
                Json::String(
                    match link.status {
                        LinkStatus::Active => "active",
                        LinkStatus::Held => "held",
                        LinkStatus::Blocked => "blocked",
                    }
                    .into(),
                ),
            ),
        ],
    )
}

fn resolution(resolution: &Resolution) -> Json {
    item(
        "Resolution",
        vec![
            (
                "outcome",
                Json::String(
                    match resolution.outcome {
                        ResolveOutcome::Redirect => "redirect",
                        ResolveOutcome::Held => "held",
                        ResolveOutcome::Unknown => "unknown",
                    }
                    .into(),
                ),
            ),
            ("location", Json::String(resolution.location.clone())),
        ],
    )
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut service: Option<UrlShortenerService<StandInClient>> = None;
    let mut say = |line: Json| {
        let _ = writeln!(out, "{}", line.to_text());
        let _ = out.flush();
    };
    let error = |text: String| Json::object(vec![("error", Json::String(text))]);
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let command = match Json::parse(&line) {
            Ok(command) => command,
            Err(e) => {
                say(error(format!("unreadable command: {e}")));
                continue;
            }
        };
        if let Some(start) = command.get("start") {
            let subject = start.get("subject").and_then(Json::as_str).unwrap_or("");
            if subject != "UrlShortenerService" {
                say(error(format!(
                    "this harness builds UrlShortenerService, not {subject}; {subject} calls a model, which the harness does not: evaluate it from recordings or live"
                )));
                continue;
            }
            let answers: Vec<Json> = start
                .get("standIns")
                .and_then(Json::as_array)
                .unwrap_or(&[])
                .iter()
                .filter(|s| s.get("target").and_then(Json::as_str) == Some("screening"))
                .cloned()
                .collect();
            if answers.is_empty() {
                say(error(
                    "the screening agent has no stand-in, and the harness calls no model".into(),
                ));
                continue;
            }
            service = Some(UrlShortenerService::new(
                StandInClient { answers, calls: 0 },
                AgentPolicy::default(),
            ));
        } else if let Some(send) = command.get("send") {
            let Some(service) = service.as_mut() else {
                say(error("send before start".into()));
                continue;
            };
            let port = send.get("port").and_then(Json::as_str).unwrap_or("");
            let value = send.get("value").cloned().unwrap_or(Json::Null);
            let output = match port {
                "shorten" => link(&service.shorten(&ShortenRequest {
                    long_url: text(&value, "longUrl"),
                    host: text(&value, "host"),
                })),
                "resolve" => resolution(&service.resolve(&ResolveRequest {
                    code: text(&value, "code"),
                })),
                "review" => link(&service.review(&ReviewDecision {
                    code: text(&value, "code"),
                    approve: value
                        .get("fields")
                        .and_then(|f| f.get("approve"))
                        .and_then(Json::as_bool)
                        .unwrap_or(false),
                })),
                other => {
                    say(error(format!("no port `{other}`")));
                    continue;
                }
            };
            say(Json::object(vec![(
                "output",
                Json::object(vec![("port", Json::String(port.into())), ("value", output)]),
            )]));
        }
        // `wait` and `finish`: the service answers at once; nothing is pending.
        say(Json::object(vec![("ready", Json::Bool(true))]));
        if command.get("finish").is_some() {
            break;
        }
    }
}
