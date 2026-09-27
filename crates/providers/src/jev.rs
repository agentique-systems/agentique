//! TypeSafe AI's Jev: typed decisions for fast agents (C-35), through a thin
//! client until rig releases `rig-typesafeai` (C-34; ROADMAP §4.11).
//!
//! Jev answers typed questions about a state: yes or no, a choice among up
//! to 255 options, or a score on 2–10 ordered levels, with probabilities and
//! (for choices and scores) a confidence. It holds no conversation, so it is
//! never the Assistant's model. `POST /v1/systemone` with a bearer key
//! (`TYPESAFE_API_KEY`); the request id comes back in `x-typesafe-request-id`
//! (https://docs.typesafe.ai/api, read 2026-09-27).

use crate::{Error, ErrorKind, Provider, Usage, runtime};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;

pub const API_URL: &str = "https://api.typesafe.ai";
/// A fixed version, so answers do not move under a scenario's feet; the
/// API accepts versioned ids whether or not its model list shows them.
pub const DEFAULT_MODEL: &str = "jev-1.13.0";
/// Attempts after the first for rate limits and overload (429, 529).
const RETRIES: u32 = 2;
const MAX_RETRY_WAIT: Duration = Duration::from_secs(10);
/// A decision that takes longer than this has failed (the vendor claims
/// 70–500 ms per call).
const TIMEOUT: Duration = Duration::from_secs(30);

/// Questions about one state.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionRequest {
    /// A versioned id such as `jev-1.13.0`, or `jev-latest`.
    pub model: String,
    /// What the questions are about: text or JSON.
    pub state: Value,
    /// By id; the id is not shown to the model.
    pub questions: BTreeMap<String, Question>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub instructions: String,
    pub kind: QuestionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuestionKind {
    /// Yes or no, optionally with what yes and what no mean (both or
    /// neither, as the API requires).
    YesNo { meanings: Option<(String, String)> },
    /// One of these options (2–255), each with an optional description.
    Choice {
        options: BTreeMap<String, Option<String>>,
    },
    /// A level on this ordered scale (2–10 levels, low to high).
    Score { levels: Vec<String> },
}

/// The answers, by question id.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionReply {
    /// The versioned model that answered.
    pub model: String,
    pub answers: BTreeMap<String, Answer>,
    pub usage: Usage,
    pub request_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// The probability of yes.
    YesNo { yes: f64 },
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
    /// `score` is the probability-weighted level (it can fall between
    /// levels); `probabilities` are by level index.
    Score {
        score: f64,
        probabilities: BTreeMap<usize, f64>,
        confidence: f64,
    },
}

impl DecisionRequest {
    /// The request body, after checking the limits the API documents.
    pub(crate) fn body(&self) -> Result<Value, String> {
        if self.questions.is_empty() {
            return Err("A decision needs at least one question.".into());
        }
        let mut questions = serde_json::Map::new();
        for (id, question) in &self.questions {
            if id.trim().is_empty() {
                return Err("A question id must not be empty.".into());
            }
            let value = match &question.kind {
                QuestionKind::YesNo { meanings } => {
                    let mut value =
                        json!({ "type": "noul", "instructions": question.instructions });
                    if let Some((yes, no)) = meanings {
                        value["criteria"] = json!({ "true": yes, "false": no });
                    }
                    value
                }
                QuestionKind::Choice { options } => {
                    if !(2..=255).contains(&options.len())
                        || options.keys().any(|option| option.trim().is_empty())
                    {
                        return Err(format!("`{id}`: a choice needs 2 to 255 named options."));
                    }
                    json!({ "type": "choice", "instructions": question.instructions, "criteria": options })
                }
                QuestionKind::Score { levels } => {
                    if !(2..=10).contains(&levels.len()) {
                        return Err(format!("`{id}`: a score needs 2 to 10 levels."));
                    }
                    json!({ "type": "score", "instructions": question.instructions, "criteria": levels })
                }
            };
            questions.insert(id.clone(), value);
        }
        Ok(json!({ "state": self.state, "model": self.model, "questions": questions }))
    }
}

/// The wire form of an answer.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum WireAnswer {
    Noul {
        noul: f64,
    },
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
    Score {
        score: f64,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
}

#[derive(Deserialize)]
struct WireReply {
    model: String,
    answers: BTreeMap<String, WireAnswer>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Deserialize, Serialize)]
struct WireUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

/// Reads a reply body.
pub(crate) fn read_reply(body: &str, request_id: Option<String>) -> Result<DecisionReply, Error> {
    let wire: WireReply = serde_json::from_str(body).map_err(|error| Error {
        kind: ErrorKind::Other,
        message: format!("TypeSafe AI sent a reply that could not be read ({error})."),
    })?;
    let answers = wire
        .answers
        .into_iter()
        .map(|(id, answer)| {
            let answer = match answer {
                WireAnswer::Noul { noul } => Answer::YesNo { yes: noul },
                WireAnswer::Choice {
                    choice,
                    probabilities,
                    confidence,
                } => Answer::Choice {
                    choice,
                    probabilities,
                    confidence,
                },
                WireAnswer::Score {
                    score,
                    probabilities,
                    confidence,
                } => Answer::Score {
                    score,
                    probabilities: probabilities
                        .into_iter()
                        .filter_map(|(level, p)| level.parse().ok().map(|level| (level, p)))
                        .collect(),
                    confidence,
                },
            };
            (id, answer)
        })
        .collect();
    let usage = wire.usage.unwrap_or(WireUsage {
        input_tokens: None,
        output_tokens: None,
    });
    Ok(DecisionReply {
        model: wire.model,
        answers,
        usage: Usage {
            input_tokens: usage.input_tokens.unwrap_or_default(),
            output_tokens: usage.output_tokens.unwrap_or_default(),
            ..Usage::default()
        },
        request_id,
    })
}

/// One decision, blocking; retries rate limits and overload twice.
pub(crate) fn decide(
    request: &DecisionRequest,
    key: Option<String>,
    endpoint: Option<String>,
) -> Result<DecisionReply, Error> {
    let key = key.ok_or_else(|| Error {
        kind: ErrorKind::MissingKey,
        message: format!(
            "No TypeSafe AI key is set. Set {} and restart Agentique.",
            Provider::TypeSafe.key_variable()
        ),
    })?;
    let body = request.body().map_err(|message| Error {
        kind: ErrorKind::Rejected,
        message,
    })?;
    let url = format!(
        "{}/v1/systemone",
        endpoint.as_deref().unwrap_or(API_URL).trim_end_matches('/')
    );
    runtime::run(TIMEOUT, async move {
        let client = client();
        let mut attempt = 0;
        loop {
            let sent = client
                .post(&url)
                .bearer_auth(&key)
                .json(&body)
                .send()
                .await;
            let response = match sent {
                Ok(response) => response,
                Err(error) => {
                    return Err(Error {
                        kind: ErrorKind::Unreachable,
                        message: format!(
                            "Could not reach TypeSafe AI ({}). Check the network connection and try again.",
                            error.without_url()
                        ),
                    });
                }
            };
            let status = response.status().as_u16();
            let header = |name: &str| {
                response
                    .headers()
                    .get(name)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string)
            };
            let request_id = header("x-typesafe-request-id");
            let wait = header("retry-after-ms")
                .and_then(|ms| ms.trim().parse().ok())
                .map(Duration::from_millis)
                .or_else(|| {
                    header("retry-after")
                        .and_then(|seconds| seconds.trim().parse().ok())
                        .map(Duration::from_secs)
                });
            // An error body is shown to the Operator: keep it short.
            let text: String = response
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(300)
                .collect();
            if (200..300).contains(&status) {
                return read_reply(&text, request_id);
            }
            if matches!(status, 429 | 529) && attempt < RETRIES {
                attempt += 1;
                let wait = wait.unwrap_or(Duration::from_secs(u64::from(attempt)));
                if wait <= MAX_RETRY_WAIT {
                    runtime::sleep(wait).await;
                    continue;
                }
            }
            let (kind, message) = match status {
                401 => (
                    ErrorKind::KeyRefused,
                    format!(
                        "TypeSafe AI did not accept the API key (401). Check {}.",
                        Provider::TypeSafe.key_variable()
                    ),
                ),
                403 => (
                    ErrorKind::NoAccess,
                    format!("This TypeSafe AI key may not make this request (403): {text}"),
                ),
                404 => (
                    ErrorKind::UnknownModel,
                    "TypeSafe AI does not know this model (404).".to_string(),
                ),
                422 | 400 => (
                    ErrorKind::Rejected,
                    format!("TypeSafe AI did not accept the questions ({status}): {text}"),
                ),
                429 => (
                    ErrorKind::RateLimited,
                    "The TypeSafe AI rate limit was reached (429). Try again in a minute.".into(),
                ),
                500..=599 => (
                    ErrorKind::Unavailable,
                    format!("TypeSafe AI is overloaded or unavailable ({status}). Try again in a moment."),
                ),
                _ => (
                    ErrorKind::Other,
                    format!("TypeSafe AI answered with an error ({status}): {text}"),
                ),
            };
            return Err(Error { kind, message });
        }
    })
    .unwrap_or_else(|failure| {
        Err(Error {
            kind: ErrorKind::Unreachable,
            message: match failure {
                runtime::Failure::TimedOut => format!(
                    "TypeSafe AI did not answer within {} seconds.",
                    TIMEOUT.as_secs()
                ),
                runtime::Failure::Crashed => {
                    "The request to TypeSafe AI failed unexpectedly. Try again.".to_string()
                }
            },
        })
    })
}

/// One HTTP client for every decision, so connections and TLS sessions are
/// reused (a fast agent may decide many times a second).
fn client() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triage() -> DecisionRequest {
        DecisionRequest {
            model: DEFAULT_MODEL.into(),
            state: json!("Short link https://example.test/free-prizes created by a new account."),
            questions: BTreeMap::from([
                (
                    "suspicious".to_string(),
                    Question {
                        instructions: "Is this link likely abuse?".into(),
                        kind: QuestionKind::YesNo { meanings: None },
                    },
                ),
                (
                    "action".to_string(),
                    Question {
                        instructions: "What should happen to the link?".into(),
                        kind: QuestionKind::Choice {
                            options: BTreeMap::from([
                                ("activate".to_string(), None),
                                ("hold".to_string(), Some("Hold for review".to_string())),
                            ]),
                        },
                    },
                ),
            ]),
        }
    }

    #[test]
    fn the_body_follows_the_api() {
        let body = triage().body().unwrap();
        assert_eq!(body["model"], "jev-1.13.0");
        assert_eq!(
            body["questions"]["suspicious"],
            json!({ "type": "noul", "instructions": "Is this link likely abuse?" })
        );
        assert_eq!(
            body["questions"]["action"]["criteria"]["hold"],
            "Hold for review"
        );
        assert_eq!(
            body["questions"]["action"]["criteria"]["activate"],
            Value::Null
        );
    }

    #[test]
    fn yes_and_no_are_described_together() {
        let mut request = triage();
        request.questions.insert(
            "urgent".into(),
            Question {
                instructions: "Is it urgent?".into(),
                kind: QuestionKind::YesNo {
                    meanings: Some(("It must be handled now".into(), "It can wait".into())),
                },
            },
        );
        let body = request.body().unwrap();
        assert_eq!(
            body["questions"]["urgent"]["criteria"],
            json!({ "true": "It must be handled now", "false": "It can wait" })
        );
    }

    #[test]
    fn limits_are_checked_before_sending() {
        let mut request = triage();
        request.questions.insert(
            "level".into(),
            Question {
                instructions: "How risky?".into(),
                kind: QuestionKind::Score {
                    levels: vec!["low".into()],
                },
            },
        );
        assert!(request.body().unwrap_err().contains("2 to 10 levels"));
        request.questions.clear();
        assert!(request.body().is_err());
    }

    #[test]
    fn a_reply_is_read() {
        let reply = read_reply(
            r#"{"model":"jev-1.13.0","answers":{
                "suspicious":{"type":"noul","noul":0.93},
                "action":{"type":"choice","choice":"hold","probabilities":{"hold":0.9,"activate":0.1},"confidence":0.87},
                "risk":{"type":"score","score":1.2,"legend":{"0":"low","1":"medium","2":"high"},"probabilities":{"0":0.1,"1":0.6,"2":0.3},"confidence":0.5}},
                "usage":{"input_tokens":120,"output_tokens":8}}"#,
            Some("req_1".into()),
        )
        .unwrap();
        assert_eq!(reply.answers["suspicious"], Answer::YesNo { yes: 0.93 });
        assert!(
            matches!(&reply.answers["action"], Answer::Choice { choice, confidence, .. } if choice == "hold" && *confidence == 0.87)
        );
        assert!(
            matches!(&reply.answers["risk"], Answer::Score { probabilities, .. } if probabilities[&1] == 0.6)
        );
        assert_eq!(reply.usage.input_tokens, 120);
        assert_eq!(reply.request_id.as_deref(), Some("req_1"));
    }
}
