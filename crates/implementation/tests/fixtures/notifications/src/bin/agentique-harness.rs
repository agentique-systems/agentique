//! Agentique's harness for the notification service: runs the real
//! dispatcher, with the gateway replaced by the scenario's stand-ins, and
//! speaks the harness protocol (one JSON line per command) on standard input
//! and output.

use notifications::dispatcher::{Clock, Dispatcher};
use notifications::gateway::{Gateway, GatewayError};
use notifications::json::Json;
use notifications::model::{DeliveryStatus, Notification, Receipt, SendRequest, SendResult};
use std::io::{BufRead, Write};

/// Logical time: the harness's clock only counts.
struct Logical(u64);

impl Clock for Logical {
    fn sleep_ms(&mut self, ms: u64) {
        self.0 += ms;
    }
}

/// The gateway answered by the scenario's stand-ins, call by call.
struct StandInGateway {
    answers: Vec<Json>,
    calls: u32,
}

impl Gateway for StandInGateway {
    fn send(&mut self, request: &SendRequest) -> Result<SendResult, GatewayError> {
        self.calls += 1;
        let call = f64::from(self.calls);
        let answer = self
            .answers
            .iter()
            .find(|a| a.get("call").and_then(Json::as_f64) == Some(call))
            .or_else(|| self.answers.iter().find(|a| a.get("call").is_none_or(Json::is_null)))
            .ok_or_else(|| GatewayError::Unavailable(format!("no stand-in answers call {}", self.calls)))?;
        match answer.get("outcome").and_then(Json::as_str) {
            Some("answer") => {
                let fields = answer
                    .get("output")
                    .and_then(|o| o.get("fields"))
                    .ok_or_else(|| GatewayError::Unavailable("a stand-in answer without an output".into()))?;
                Ok(SendResult {
                    id: fields.get("id").and_then(Json::as_str).unwrap_or(&request.id).to_string(),
                    ok: fields.get("ok").and_then(Json::as_bool).unwrap_or(false),
                })
            }
            _ => Err(GatewayError::Timeout),
        }
    }
}

fn receipt(receipt: &Receipt) -> Json {
    Json::object(vec![
        ("type", Json::String("Receipt".into())),
        (
            "fields",
            Json::object(vec![
                ("id", Json::String(receipt.id.clone())),
                (
                    "status",
                    Json::String(
                        match receipt.status {
                            DeliveryStatus::Delivered => "delivered",
                            DeliveryStatus::Failed => "failed",
                        }
                        .into(),
                    ),
                ),
                ("attempts", Json::Number(f64::from(receipt.attempts))),
            ]),
        ),
    ])
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut dispatcher: Option<Dispatcher<StandInGateway, Logical>> = None;
    let mut say = |line: Json| {
        let _ = writeln!(out, "{}", line.to_text());
        let _ = out.flush();
    };
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let command = match Json::parse(&line) {
            Ok(command) => command,
            Err(e) => {
                say(Json::object(vec![("error", Json::String(format!("unreadable command: {e}")))]));
                continue;
            }
        };
        if let Some(start) = command.get("start") {
            let answers: Vec<Json> = start
                .get("standIns")
                .and_then(Json::as_array)
                .unwrap_or(&[])
                .iter()
                .filter(|s| s.get("target").and_then(Json::as_str) == Some("gateway"))
                .cloned()
                .collect();
            dispatcher = Some(Dispatcher::new(StandInGateway { answers, calls: 0 }, Logical(0)));
        } else if let Some(send) = command.get("send") {
            let Some(dispatcher) = dispatcher.as_mut() else {
                say(Json::object(vec![("error", Json::String("send before start".into()))]));
                continue;
            };
            let port = send.get("port").and_then(Json::as_str).unwrap_or("");
            if port != "notifications" {
                say(Json::object(vec![("error", Json::String(format!("no port `{port}`")))]));
                continue;
            }
            let fields = send.get("value").and_then(|v| v.get("fields"));
            let text = |key: &str| {
                fields
                    .and_then(|f| f.get(key))
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            let notification = Notification {
                id: text("id"),
                text: text("text"),
            };
            let result = dispatcher.dispatch(&notification);
            say(Json::object(vec![(
                "output",
                Json::object(vec![
                    ("port", Json::String("notifications".into())),
                    ("value", receipt(&result)),
                ]),
            )]));
        }
        // `wait` and `finish` have nothing pending here: the dispatcher is synchronous.
        say(Json::object(vec![("ready", Json::Bool(true))]));
        if command.get("finish").is_some() {
            break;
        }
    }
}
