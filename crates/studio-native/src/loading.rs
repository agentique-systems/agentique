//! Local opening feedback. Worker phases are observations, never acceptance authority.
use agq_studio_platform::BootstrapPhase;
use std::time::{Duration, Instant};

pub struct OpeningProgress {
    request: u64,
    started: Instant,
    phase_started: Instant,
    ended: Option<Instant>,
    pub failed: bool,
    pub title: &'static str,
    pub detail: &'static str,
}

impl OpeningProgress {
    pub fn new(request: u64) -> Self {
        let now = Instant::now();
        Self {
            request,
            started: now,
            phase_started: now,
            ended: None,
            failed: false,
            title: "Opening your workspace",
            detail: "Preparing the local runtime and project repository.",
        }
    }

    pub fn observe(&mut self, request: u64, phase: &BootstrapPhase) {
        if request != self.request || !self.running() {
            return;
        }
        let (title, detail) = phase_text(phase);
        if title != self.title {
            self.phase_started = Instant::now();
        }
        self.title = title;
        self.detail = detail;
    }

    pub fn finish(&mut self, request: u64, failed: bool) {
        if request == self.request && self.running() {
            self.ended = Some(Instant::now());
            self.failed = failed;
        }
    }

    pub fn running(&self) -> bool {
        self.ended.is_none()
    }

    pub fn elapsed_text(&self) -> String {
        let now = self.ended.unwrap_or_else(Instant::now);
        format!(
            "{} elapsed · {} in this step",
            duration(now.duration_since(self.started)),
            duration(now.duration_since(self.phase_started)),
        )
    }
}

fn duration(value: Duration) -> String {
    let seconds = value.as_secs();
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    }
}

pub fn phase_text(phase: &BootstrapPhase) -> (&'static str, &'static str) {
    match phase {
        // The platform exposes its dependency's serialized phase contract. Keep
        // the native host independent of runtime implementation crate imports.
        BootstrapPhase::Runtime(phase) => match serde_json::to_value(phase)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .as_deref()
        {
            Some("locating_package") => (
                "Locating the semantic runtime",
                "Finding the accepted language libraries on this computer.",
            ),
            Some("copying") => (
                "Preparing runtime files",
                "Preparing a local copy of the selected runtime bundle.",
            ),
            Some("authenticating_kerml") => (
                "Checking KerML language libraries",
                "Authenticating the local KerML runtime against its accepted publication.",
            ),
            Some("authenticating_sysml") => (
                "Checking SysML language libraries",
                "Authenticating the local SysML runtime against its accepted publication.",
            ),
            Some("installing") => (
                "Installing the checked runtime",
                "Saving the authenticated language libraries in the local runtime store.",
            ),
            Some("ready") => (
                "Language libraries authenticated",
                "The accepted runtime is ready for project restoration.",
            ),
            _ => (
                "Preparing the semantic runtime",
                "The runtime worker is preparing the language libraries.",
            ),
        },
        BootstrapPhase::OpeningRepository => (
            "Opening your project repository",
            "Reading durable project data from this computer.",
        ),
        BootstrapPhase::OpeningProject => (
            "Opening the Agentique project",
            "Finding the project or preparing its first revision.",
        ),
        BootstrapPhase::ValidatingArchitecture => (
            "Validating the system architecture",
            "Checking architecture, references and model consistency before accepting a revision.",
        ),
        BootstrapPhase::ValidatingAgentFabric => (
            "Validating the agent architecture",
            "Checking the modeled agents and their relationships with the system.",
        ),
        BootstrapPhase::RestoringRevision => (
            "Restoring saved project revisions",
            "Reopening saved models with the authenticated runtime.",
        ),
        BootstrapPhase::Ready => ("Workspace prepared", "Loading the project list."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_phase_does_not_reset_opening_time_or_accept_stale_progress() {
        let mut progress = OpeningProgress::new(7);
        progress.started -= Duration::from_secs(125);
        let start = progress.started;
        progress.observe(6, &BootstrapPhase::Ready);
        assert_eq!(progress.title, "Opening your workspace");
        progress.observe(7, &BootstrapPhase::ValidatingArchitecture);
        assert_eq!(progress.started, start);
        assert_eq!(progress.title, "Validating the system architecture");
        progress.finish(6, true);
        assert!(progress.running());
        progress.finish(7, true);
        let ended = progress.ended;
        assert!(progress.failed);
        assert!(!progress.running());
        progress.observe(7, &BootstrapPhase::Ready);
        progress.finish(7, false);
        assert_eq!(progress.ended, ended);
        assert!(progress.failed);
        assert_eq!(progress.title, "Validating the system architecture");
        assert!(progress.elapsed_text().starts_with("2m 05s elapsed"));
        let retry = OpeningProgress::new(8);
        assert!(retry.running());
        assert!(!retry.failed);
        assert!(retry.started > start);
    }

    #[test]
    fn runtime_phases_have_human_labels_without_debug_payloads() {
        for (serialized, title) in [
            ("locating_package", "Locating the semantic runtime"),
            ("copying", "Preparing runtime files"),
            ("authenticating_kerml", "Checking KerML language libraries"),
            ("authenticating_sysml", "Checking SysML language libraries"),
            ("installing", "Installing the checked runtime"),
            ("ready", "Language libraries authenticated"),
        ] {
            let phase = BootstrapPhase::Runtime(
                serde_json::from_value(serde_json::json!(serialized)).unwrap(),
            );
            assert_eq!(phase_text(&phase).0, title);
        }
    }
}
