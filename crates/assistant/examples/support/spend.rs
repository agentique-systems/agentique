//! A spend guard for live runs of the examples (developer tooling, not the
//! product: C-37 keeps the product without spending limits).
//!
//! Every model call appends one JSON line to the log named by
//! `AGENTIQUE_SPEND_LOG` (time, purpose, provider, model, tokens, estimated
//! cost). A run refuses to start when the provider's logged total plus the
//! run's worst case would pass `AGENTIQUE_SPEND_STOP_USD`, and refuses every
//! call after its own maximum or once the logged total reaches the stop.
//! Keep the log outside the repository: it is evidence, never committed.

use agq_assistant::{Model, ModelError, Reply, Request, StreamEvent};
use agq_providers::{ModelRef, price};
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Tokens assumed per call for the worst case: full-price input.
const WORST_INPUT_TOKENS: f64 = 60_000.0;

pub struct SpendGuard {
    log: PathBuf,
    stop_usd: f64,
    purpose: String,
    model: ModelRef,
    max_calls: u32,
    calls: u32,
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
            .map_err(|_| "Set AGENTIQUE_SPEND_LOG (a JSON-lines file outside the repository) before a live run.".to_string())?;
        let stop_usd: f64 = std::env::var("AGENTIQUE_SPEND_STOP_USD")
            .ok()
            .and_then(|value| value.trim().parse().ok())
            .ok_or(
                "Set AGENTIQUE_SPEND_STOP_USD (the hard stop in US dollars) before a live run.",
            )?;
        // Unknown prices use a conservative stand-in: $1 in, $4 out per million.
        let (input, output) = price(model).map_or((1.0, 4.0), |price| (price.input, price.output));
        let worst_call = (WORST_INPUT_TOKENS * input + max_output_tokens as f64 * output) / 1e6;
        let worst = worst_call * f64::from(max_calls);
        let logged = logged_usd(&log, model.provider.id());
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
        Ok(Arc::new(Mutex::new(SpendGuard {
            log,
            stop_usd,
            purpose: purpose.to_string(),
            model: model.clone(),
            max_calls,
            calls: 0,
            run_usd: 0.0,
        })))
    }

    fn allow(&mut self) -> Result<(), String> {
        if self.calls >= self.max_calls {
            return Err(format!(
                "the run's maximum of {} model calls was reached",
                self.max_calls
            ));
        }
        let logged = logged_usd(&self.log, self.model.provider.id());
        if logged >= self.stop_usd {
            return Err(format!("the logged spend (${logged:.4}) reached the stop"));
        }
        self.calls += 1;
        Ok(())
    }

    fn record(&mut self, usage: &agq_assistant::Usage) {
        let neutral = agq_providers::Usage {
            input_tokens: usage.input_tokens,
            cache_write_tokens: usage.cache_creation_input_tokens,
            cache_read_tokens: usage.cache_read_input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_tokens: 0,
        };
        let cost = neutral.cost_usd(&self.model).unwrap_or_else(|| {
            ((usage.input_tokens
                + usage.cache_creation_input_tokens
                + usage.cache_read_input_tokens) as f64
                + usage.output_tokens as f64 * 4.0)
                / 1e6
        });
        self.run_usd += cost;
        let line = json!({
            "time": timestamp(),
            "purpose": self.purpose,
            "provider": self.model.provider.id(),
            "model": self.model.model,
            "input_tokens": usage.input_tokens,
            "cached_input_tokens": usage.cache_read_input_tokens,
            "cache_write_tokens": usage.cache_creation_input_tokens,
            "output_tokens": usage.output_tokens,
            "cost_usd": cost,
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)
        {
            let _ = writeln!(file, "{line}");
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
}

impl Model for GuardedModel {
    fn send(
        &mut self,
        request: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        self.guard
            .lock()
            .unwrap()
            .allow()
            .map_err(|why| ModelError::Failed(format!("Spend guard: {why}.")))?;
        let guard = self.guard.clone();
        self.inner.send(
            request,
            &mut |event| {
                if let StreamEvent::Usage(usage) = &event {
                    guard.lock().unwrap().record(usage);
                }
                on_event(event);
            },
            stop,
        )
    }
}
