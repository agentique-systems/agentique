//! One live decision from TypeSafe AI's Jev, for checking a key and the
//! client (a fast agent's shape: link screening, ROADMAP §2.6):
//!
//! ```text
//! cargo run -p agq-providers --example jev
//! ```
//!
//! Needs `TYPESAFE_API_KEY`, and a spend log and stop like the Assistant's
//! live examples (`AGENTIQUE_SPEND_LOG`, `AGENTIQUE_SPEND_STOP_USD`): the call
//! is refused when TypeSafe AI's logged spend would pass the stop, and its
//! estimated cost is appended to the log. Keep the log outside the
//! repository.

use agq_providers::jev::{DEFAULT_MODEL, DecisionRequest, Question, QuestionKind};
use agq_providers::{ModelRef, Provider, Providers, key_status};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;

fn main() {
    if key_status(Provider::TypeSafe) == agq_providers::KeyStatus::Missing {
        println!("TYPESAFE_API_KEY is not set, so there is nothing to try.");
        return;
    }
    let (Ok(log), Some(stop)) = (
        std::env::var("AGENTIQUE_SPEND_LOG"),
        std::env::var("AGENTIQUE_SPEND_STOP_USD")
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok()),
    ) else {
        println!("Set AGENTIQUE_SPEND_LOG and AGENTIQUE_SPEND_STOP_USD before a live call.");
        std::process::exit(2);
    };
    let logged: f64 = std::fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|entry| entry["provider"] == Provider::TypeSafe.id())
        .filter_map(|entry| entry["cost_usd"].as_f64())
        .sum();
    // A worst case far above one small decision: 10,000 input tokens.
    let model = ModelRef::new(Provider::TypeSafe, DEFAULT_MODEL);
    let worst = agq_providers::price(&model).map_or(0.001, |price| 10_000.0 * price.input / 1e6);
    if logged + worst > stop {
        println!(
            "Refused: TypeSafe AI spend logged ${logged:.6} plus this call's worst case would pass the stop ${stop:.2}."
        );
        std::process::exit(2);
    }
    let request = DecisionRequest {
        model: DEFAULT_MODEL.into(),
        state: json!({
            "long_url": "https://free-gift-cards.example/claim?id=8841",
            "account_age_days": 0,
            "links_created_today": 37
        }),
        questions: BTreeMap::from([
            (
                "suspicious".to_string(),
                Question {
                    instructions:
                        "Is this new short link likely to be abuse (spam, phishing or scams)?"
                            .into(),
                    kind: QuestionKind::YesNo { meanings: None },
                },
            ),
            (
                "action".to_string(),
                Question {
                    instructions: "What should the URL shortener do with this link?".into(),
                    kind: QuestionKind::Choice {
                        options: BTreeMap::from([
                            ("activate".to_string(), Some("Activate it now".to_string())),
                            (
                                "hold".to_string(),
                                Some("Hold it for a person to review".to_string()),
                            ),
                        ]),
                    },
                },
            ),
        ]),
    };
    let started = std::time::Instant::now();
    let result = Providers::new().decide(&request);
    let elapsed = started.elapsed();
    // Unknown usage is logged at the worst case, never as free; a failure
    // costs nothing only when no request was sent.
    let (cost, line) = match &result {
        Ok(reply) => {
            let cost = reply.usage.cost_usd(&model).unwrap_or(worst);
            (
                cost,
                json!({
                    "outcome": "complete",
                    "input_tokens": reply.usage.input_tokens,
                    "output_tokens": reply.usage.output_tokens,
                    "usage_complete": reply.usage.is_complete(),
                    "attempts": reply.attempts,
                }),
            )
        }
        Err(failure) => (
            if failure.attempts == 0 {
                0.0
            } else {
                failure
                    .usage
                    .and_then(|usage| usage.cost_usd(&model))
                    .unwrap_or(worst * f64::from(failure.attempts))
            },
            json!({
                "outcome": "failed",
                "kind": format!("{:?}", failure.error.kind),
                "attempts": failure.attempts,
            }),
        ),
    };
    let mut entry = json!({
        "time": format!("unix:{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()),
        "purpose": "jev-example",
        "provider": Provider::TypeSafe.id(),
        "model": DEFAULT_MODEL,
        "cost_usd": cost,
    });
    if let (Some(entry), Some(extra)) = (entry.as_object_mut(), line.as_object()) {
        entry.extend(extra.clone());
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
    {
        let _ = writeln!(file, "{entry}");
    }
    match result {
        Ok(reply) => {
            println!(
                "Model {} answered in {elapsed:?} (request {:?}):",
                reply.model, reply.request_id
            );
            for (id, answer) in &reply.answers {
                println!("- {id}: {answer:?}");
            }
            let count = |n: Option<u64>| n.map_or("unknown".to_string(), |n| n.to_string());
            println!(
                "Tokens: {} in, {} out; estimated cost ${cost:.6}{}.",
                count(reply.usage.input_tokens),
                count(reply.usage.output_tokens),
                if reply.usage.is_complete() {
                    ""
                } else {
                    " (usage incomplete: logged at the worst case)"
                }
            );
        }
        Err(failure) => {
            println!(
                "Failed after {elapsed:?} and {} request(s): {failure}",
                failure.attempts
            );
            std::process::exit(1);
        }
    }
}
