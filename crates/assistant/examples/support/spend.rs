//! A spend guard for live runs of the examples (developer tooling, not the
//! product: C-37 keeps the product without spending limits).
//!
//! Every model call appends one JSON line to the log named by
//! `AGENTIQUE_SPEND_LOG` (time, purpose, provider, model, tokens, estimated
//! cost, outcome); a call that fails or is stopped before reporting its
//! usage is logged at its worst case. A run refuses to start when the
//! provider's logged total plus the run's worst case would pass
//! `AGENTIQUE_SPEND_STOP_USD`, and each call is refused once its own worst
//! case would pass the stop or the run's maximum of calls is reached. Models
//! without a known price are refused. Keep the log outside the repository:
//! it is evidence, never committed.

use agq_assistant::{Model, ModelError, Reply, Request, StreamEvent};
use agq_providers::{ModelRef, price};
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Tokens assumed for a call's input when estimating a run's worst case
/// before it starts; each call is checked against its real request size.
const ASSUMED_INPUT_TOKENS: u64 = 60_000;

pub struct SpendGuard {
    log: PathBuf,
    stop_usd: f64,
    purpose: String,
    model: ModelRef,
    max_calls: u32,
    max_output_tokens: u64,
    calls: u32,
    /// Worst cases of calls admitted and not yet recorded (parallel runs).
    reserved_usd: f64,
    /// A log line could not be written: no further call is admitted.
    broken: bool,
    /// This run's estimated cost so far.
    pub run_usd: f64,
}

impl SpendGuard {
    /// Checks the log and the run's worst case before anything is sent.
    pub fn start(
        purpose: &str,
        model: &ModelRef,
        max_calls: u32,
        max_output_tokens: u64,
    ) -> Result<Arc<Mutex<SpendGuard>>, String> {
        let log = std::env::var("AGENTIQUE_SPEND_LOG")
            .map(PathBuf::from)
            .map_err(|_| {
                "Set AGENTIQUE_SPEND_LOG (a JSON-lines file outside the repository) before a live run."
                    .to_string()
            })?;
        let stop_usd: f64 = std::env::var("AGENTIQUE_SPEND_STOP_USD")
            .ok()
            .and_then(|value| value.trim().parse().ok())
            .ok_or(
                "Set AGENTIQUE_SPEND_STOP_USD (the hard stop in US dollars) before a live run.",
            )?;
        if price(model).is_none() {
            return Err(format!(
                "Refused: `{}` has no known price, so its cost cannot be bounded.",
                model.model
            ));
        }
        let guard = SpendGuard {
            log,
            stop_usd,
            purpose: purpose.to_string(),
            model: model.clone(),
            max_calls,
            max_output_tokens,
            calls: 0,
            reserved_usd: 0.0,
            broken: false,
            run_usd: 0.0,
        };
        let worst = guard.worst_call(ASSUMED_INPUT_TOKENS) * f64::from(max_calls);
        let logged = guard.logged();
        if logged + worst > stop_usd {
            return Err(format!(
                "Refused: {} spend logged so far is ${logged:.4}; this run's worst case is ${worst:.4} ({max_calls} calls); together they pass the stop of ${stop_usd:.2}.",
                model.provider.name()
            ));
        }
        println!(
            "[spend] {} logged ${logged:.4}; this run at most {max_calls} calls, worst case ${worst:.4}; stop ${stop_usd:.2}.",
            model.provider.name()
        );
        Ok(Arc::new(Mutex::new(guard)))
    }

    /// The most one call can cost: all input at the dearer of the input and
    /// cache-write prices, and the whole output limit.
    fn worst_call(&self, input_tokens: u64) -> f64 {
        let price = price(&self.model).expect("checked at start");
        (input_tokens as f64 * price.input.max(price.cache_write)
            + self.max_output_tokens as f64 * price.output)
            / 1e6
    }

    fn logged(&self) -> f64 {
        logged_usd(&self.log, self.model.provider.id())
    }

    /// Admits one call of about `input_tokens`, or says why not.
    fn allow(&mut self, input_tokens: u64) -> Result<f64, String> {
        if self.calls >= self.max_calls {
            return Err(format!(
                "the run's maximum of {} model calls was reached",
                self.max_calls
            ));
        }
        if self.broken {
            return Err("the spend log could not be written".to_string());
        }
        let worst = self.worst_call(input_tokens);
        let logged = self.logged();
        if logged + self.reserved_usd + worst > self.stop_usd {
            return Err(format!(
                "the logged spend (${logged:.4}), calls under way (${:.4}) and this call's worst case (${worst:.4}) would pass the stop",
                self.reserved_usd
            ));
        }
        self.calls += 1;
        self.reserved_usd += worst;
        Ok(worst)
    }

    fn record(&mut self, usage: Option<&agq_assistant::Usage>, outcome: &str, worst: f64) {
        let (cost, tokens) = match usage {
            Some(usage) => {
                let neutral = agq_providers::Usage {
                    input_tokens: usage.input_tokens,
                    cache_write_tokens: usage.cache_creation_input_tokens,
                    cache_read_tokens: usage.cache_read_input_tokens,
                    output_tokens: usage.output_tokens,
                    reasoning_tokens: 0,
                };
                let cost = neutral.cost_usd(&self.model).unwrap_or(worst);
                (cost, Some(*usage))
            }
            // Nothing reported: count the worst case, never nothing.
            None => (worst, None),
        };
        self.run_usd += cost;
        self.reserved_usd = (self.reserved_usd - worst).max(0.0);
        let line = json!({
            "time": timestamp(),
            "purpose": self.purpose,
            "provider": self.model.provider.id(),
            "model": self.model.model,
            "outcome": outcome,
            "input_tokens": tokens.map(|t| t.input_tokens),
            "cached_input_tokens": tokens.map(|t| t.cache_read_input_tokens),
            "cache_write_tokens": tokens.map(|t| t.cache_creation_input_tokens),
            "output_tokens": tokens.map(|t| t.output_tokens),
            "cost_usd": cost,
        });
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)
            .and_then(|mut file| writeln!(file, "{line}"));
        if written.is_err() {
            self.broken = true;
        }
    }
}

/// The logged total for one provider.
pub fn logged_usd(log: &PathBuf, provider: &str) -> f64 {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|entry| entry["provider"] == provider)
        .filter_map(|entry| entry["cost_usd"].as_f64())
        .sum()
}

fn timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

/// A model whose every call passes the guard and is logged.
pub struct GuardedModel {
    pub inner: Box<dyn Model + Send>,
    pub guard: Arc<Mutex<SpendGuard>>,
    /// Calls that failed or were refused, for runs that must not count a
    /// trial that never ran.
    pub failures: usize,
}

impl Model for GuardedModel {
    fn send(
        &mut self,
        request: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        // About four characters per token, for the request as sent.
        let size = request.system.len()
            + request.tools.to_string().len()
            + request
                .messages
                .iter()
                .map(|message| message.to_string().len())
                .sum::<usize>();
        let worst = self
            .guard
            .lock()
            .unwrap()
            .allow((size / 4) as u64 + 1)
            .map_err(|why| {
                self.failures += 1;
                ModelError::Failed(format!("Spend guard: {why}."))
            })?;
        let guard = self.guard.clone();
        let mut reported = false;
        let result = self.inner.send(
            request,
            &mut |event| {
                if let StreamEvent::Usage(usage) = &event {
                    reported = true;
                    guard.lock().unwrap().record(Some(usage), "complete", worst);
                }
                on_event(event);
            },
            stop,
        );
        if !reported {
            let outcome = match &result {
                Ok(_) => "complete without usage",
                Err(ModelError::Stopped) => "stopped",
                Err(_) => "failed",
            };
            self.guard.lock().unwrap().record(None, outcome, worst);
        }
        if result.is_err() {
            self.failures += 1;
        }
        result
    }
}
