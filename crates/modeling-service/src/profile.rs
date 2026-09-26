//! Opt-in wall-clock diagnostics. Never a semantic identity or acceptance input.
use std::{io::Write, time::Instant};

pub(crate) struct ReadProfile {
    operation: &'static str,
    start: Option<Instant>,
    previous: Option<Instant>,
}

impl ReadProfile {
    pub(crate) fn new(operation: &'static str) -> Self {
        let start = std::env::var_os("AGENTIQUE_STARTUP_PROFILE")
            .is_some_and(|value| value == "1")
            .then(Instant::now);
        Self {
            operation,
            start,
            previous: start,
        }
    }

    pub(crate) fn phase(&mut self, phase: &'static str) {
        if let Some(previous) = self.previous {
            let now = Instant::now();
            self.emit(phase, now.duration_since(previous).as_secs_f64() * 1000.0);
            self.previous = Some(now);
        }
    }

    fn emit(&self, phase: &'static str, elapsed_ms: f64) {
        let record = serde_json::json!({
            "format": "agentique-revision-read-profile/1",
            "operation": self.operation, "phase": phase, "elapsed_ms": elapsed_ms,
            "contract": "Wall time within one service call; nested operation totals overlap their containing phase. Excludes native layout, upload, and display."
        });
        let _ = writeln!(std::io::stderr().lock(), "{record}");
    }
}

impl Drop for ReadProfile {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            self.emit("total", start.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
