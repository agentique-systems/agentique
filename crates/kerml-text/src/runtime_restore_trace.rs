//! Opt-in elapsed phases; telemetry never changes restoration or acceptance.
use std::time::Instant;

pub(crate) struct RuntimeRestoreTrace {
    publication: &'static str,
    timing: Option<(Instant, Instant)>,
}
impl RuntimeRestoreTrace {
    pub(crate) fn new(publication: &'static str) -> Self {
        Self {
            publication,
            timing: std::env::var_os("AGENTIQUE_RUNTIME_RESTORE_TRACE").map(|_| {
                let now = Instant::now();
                (now, now)
            }),
        }
    }
    pub(crate) fn phase(&mut self, phase: &'static str) {
        if let Some((started, previous)) = &mut self.timing {
            let now = Instant::now();
            eprintln!(
                "RUNTIME_RESTORE_PHASE {}",
                serde_json::json!({
                    "publication": self.publication,
                    "phase": phase,
                    "elapsed_micros": now.duration_since(*previous).as_micros(),
                    "total_micros": now.duration_since(*started).as_micros(),
                })
            );
            *previous = now;
        }
    }
}
