//! What a run produced (ROADMAP §4.14): its status and why it stopped, check
//! verdicts kept apart from run completion, the trace, and the provenance
//! that freshness is computed from. Results are machine-generated
//! observations: stored in the app's per-project data, never committed.

use serde::{Deserialize, Serialize};

/// What a run actually exercised. Shown on every result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// The model's explicit behaviour with the scenario's stand-ins.
    Model,
    /// The model, with agents answered from recordings.
    Replay,
    /// Real code through the project's harness.
    Implementation,
    /// The model, with agents answered by a real provider, several samples.
    Live,
    /// The steps shown in order; nothing runs and nothing is verified.
    Walkthrough,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::Model,
        Mode::Replay,
        Mode::Implementation,
        Mode::Live,
        Mode::Walkthrough,
    ];

    /// As the Operator reads it.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Model => "Model execution",
            Mode::Replay => "Recorded replay",
            Mode::Implementation => "Implementation",
            Mode::Live => "Live evaluation",
            Mode::Walkthrough => "Walkthrough",
        }
    }

    /// As tools and links write it.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Model => "model",
            Mode::Replay => "replay",
            Mode::Implementation => "implementation",
            Mode::Live => "live",
            Mode::Walkthrough => "walkthrough",
        }
    }

    pub fn from_key(key: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.key() == key)
    }

    /// Whether the same inputs give the same result.
    pub fn deterministic(self) -> bool {
        matches!(self, Mode::Model | Mode::Replay | Mode::Walkthrough)
    }
}

/// A check's verdict. Run completion is never a verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Passed,
    Failed,
    /// The run stopped before the check was reached.
    NotRun,
    /// The runner cannot evaluate this check (for example, it reads internal
    /// state the implementation does not expose).
    Unsupported,
    /// A value it needs never arrived.
    Blocked,
    /// Not enough to decide (a live evaluation's samples disagree).
    Inconclusive,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::Passed => "passed",
            Verdict::Failed => "failed",
            Verdict::NotRun => "not run",
            Verdict::Unsupported => "unsupported",
            Verdict::Blocked => "blocked",
            Verdict::Inconclusive => "inconclusive",
        }
    }
}

/// The one summary of several verdicts, shared by runs, tasks and
/// implementation checks: passed only when there is at least one and every
/// one passed; failed when any failed; otherwise the first of blocked, not
/// run, unsupported and inconclusive found. Nothing is a pass by default.
pub fn summary(verdicts: impl IntoIterator<Item = Verdict>) -> Verdict {
    let verdicts: Vec<Verdict> = verdicts.into_iter().collect();
    if verdicts.is_empty() {
        return Verdict::NotRun;
    }
    if verdicts.contains(&Verdict::Failed) {
        return Verdict::Failed;
    }
    [
        Verdict::Blocked,
        Verdict::NotRun,
        Verdict::Unsupported,
        Verdict::Inconclusive,
    ]
    .into_iter()
    .find(|v| verdicts.contains(v))
    .unwrap_or(Verdict::Passed)
}

/// Why a run stopped before completing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopReason {
    AmbiguousTransition,
    UnhandledMessage,
    MissingBehaviour,
    MissingConnection,
    MissingStandIn,
    MissingRecording,
    AgentFailedWithoutFallback,
    AwaitedOutputMissing,
    EvaluationError,
    Unsupported,
    EventLimit,
    TimeLimit,
    DepthLimit,
    WallClockLimit,
    Cancelled,
    HarnessFailed,
}

impl StopReason {
    /// The stable code, as in `missing-recording`.
    pub fn code(self) -> &'static str {
        match self {
            StopReason::AmbiguousTransition => "ambiguous-transition",
            StopReason::UnhandledMessage => "unhandled-message",
            StopReason::MissingBehaviour => "missing-behaviour",
            StopReason::MissingConnection => "missing-connection",
            StopReason::MissingStandIn => "missing-stand-in",
            StopReason::MissingRecording => "missing-recording",
            StopReason::AgentFailedWithoutFallback => "agent-failed-without-fallback",
            StopReason::AwaitedOutputMissing => "awaited-output-missing",
            StopReason::EvaluationError => "evaluation-error",
            StopReason::Unsupported => "unsupported",
            StopReason::EventLimit => "event-limit",
            StopReason::TimeLimit => "time-limit",
            StopReason::DepthLimit => "depth-limit",
            StopReason::WallClockLimit => "wall-clock-limit",
            StopReason::Cancelled => "cancelled",
            StopReason::HarnessFailed => "harness-failed",
        }
    }
}

/// A stop, at the element concerned, in plain words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stop {
    pub reason: StopReason,
    pub element: Option<u64>,
    pub message: String,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    /// Its steps ran and nothing was left pending.
    Completed,
    /// It stopped early: see [`RunResult::stop`].
    Stopped,
    /// The Operator (or the Assistant) cancelled it.
    Cancelled,
    /// It could not start: see [`RunResult::blockers`].
    Blocked,
    /// A walkthrough: nothing ran.
    Walkthrough,
}

impl RunStatus {
    pub fn label(self) -> &'static str {
        match self {
            RunStatus::Completed => "completed",
            RunStatus::Stopped => "stopped",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Blocked => "could not start",
            RunStatus::Walkthrough => "walkthrough",
        }
    }
}

/// Pass counts of one check over a live evaluation's samples.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleCounts {
    pub samples: u32,
    pub passed: u32,
    pub failed: u32,
    /// Samples that did not reach the check or could not evaluate it.
    pub other: u32,
    /// A 95% Wilson interval for the pass rate, over the samples that
    /// reached a verdict.
    pub interval: (f64, f64),
}

/// A check's result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub element: u64,
    pub name: String,
    /// The constraint as written.
    pub expression: String,
    pub verdict: Verdict,
    /// Why, in plain words, with the values it saw.
    pub message: String,
    /// Deterministic (evaluated by code) or not (a live sample count).
    pub deterministic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub samples: Option<SampleCounts>,
    /// Set for implicit checks the runner adds (no unexpected output).
    #[serde(default)]
    pub implicit: bool,
}

/// What happened, for the trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    /// A scenario step began.
    Step,
    /// A part sent an item through a port.
    Sent,
    /// An item reached a part.
    Received,
    /// An item left the subject, towards the scenario.
    Output,
    StateEntered,
    Transition,
    Assigned,
    Timer,
    /// An agent's model was asked.
    AgentCalled,
    AgentAnswered,
    AgentFailed,
    /// A call went to the agent's fallback.
    Fallback,
    /// A stand-in answered for a part.
    StandIn,
    Check,
    Stopped,
    Note,
}

/// One event of a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEvent {
    pub seq: u64,
    pub time_ms: u64,
    pub kind: EventKind,
    /// What happened, in plain words.
    pub text: String,
    /// The model elements involved, the main one first: a part usage, a
    /// port, a connection, a state, a transition, a step.
    #[serde(default)]
    pub elements: Vec<u64>,
    /// The instance, such as `service.dispatcher`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Values involved, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<(String, String)>,
    /// Where an agent's answer came from: a stand-in, a recording, a live model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Where the code of an implementation run came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationProvenance {
    pub repository: String,
    pub commit: String,
    /// Whether the working tree had changes not committed.
    pub dirty: bool,
    /// A digest of the tracked files' content when dirty.
    pub tree_digest: String,
    pub harness: String,
}

/// The configuration of a live evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveProvenance {
    pub provider: String,
    pub model: String,
    /// Digest of the instructions sent, and where they came from.
    pub instructions_digest: String,
    pub instructions_source: String,
    pub samples: u32,
}

/// What a result depended on; freshness compares it with the present.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// The runner and its version, such as `agq-simulation 0.1.0 model`.
    pub runner: String,
    /// The digest of the scenario's model slice when the run started.
    pub model_digest: String,
    /// The System State revision when the run started.
    pub model_revision: u64,
    pub seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<ImplementationProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<LiveProvenance>,
    /// Digest of the recordings file a replay used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recordings: Option<String>,
}

/// A live evaluation's summary over its samples.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSummary {
    pub samples: u32,
    pub completed: u32,
    /// Agent failures by category (`timeout`, `invalidOutput`, `refusal`,
    /// `providerError`, ...), across samples.
    pub failures: Vec<(String, u32)>,
    pub cost_usd: Option<f64>,
    pub latency_ms_median: Option<u64>,
    /// The model's answers, ready to keep as recordings for replay.
    #[serde(default)]
    pub answers: Vec<crate::agents::Recording>,
}

/// Everything a run produced.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub format: u32,
    pub id: String,
    pub scenario: u64,
    pub scenario_name: String,
    pub scenario_qualified_name: String,
    pub mode: Mode,
    /// UTC, RFC 3339.
    pub started: String,
    pub wall_ms: u64,
    pub logical_ms: u64,
    pub events_processed: u64,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    /// Why it could not start, at the elements concerned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<(u64, String)>,
    pub checks: Vec<CheckResult>,
    /// Requirements the scenario verifies (qualified names).
    #[serde(default)]
    pub verifies: Vec<(u64, String)>,
    pub trace: Vec<TraceEvent>,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<LiveSummary>,
}

pub const FORMAT: u32 = 1;

impl RunResult {
    /// Whether every check passed and the run completed: the one summary
    /// the Studio may show as green. Never true for a walkthrough.
    pub fn all_passed(&self) -> bool {
        self.status == RunStatus::Completed
            && summary(self.checks.iter().map(|c| c.verdict)) == Verdict::Passed
    }

    /// The result in plain words, for the Assistant and for reports: how it
    /// ended, each check with its verdict and why, and the events that led
    /// to where it stopped (at most `events` of them). `current` says
    /// whether it still describes the model.
    pub fn describe(&self, current: Option<&str>, events: usize) -> String {
        let mut lines = vec![format!(
            "{} in {} ({}): {}, {} logical ms, {} events.",
            self.scenario_qualified_name,
            self.mode.label().to_lowercase(),
            self.started,
            self.status.label(),
            self.logical_ms,
            self.events_processed
        )];
        match current {
            None => lines.push("It describes the model as it is now.".into()),
            Some(why) => lines.push(format!(
                "Outdated: {why}. Run it again before relying on it."
            )),
        }
        if let Some(stop) = &self.stop {
            lines.push(format!(
                "Stopped ({}): {}",
                stop.reason.code(),
                stop.message
            ));
        }
        for (_, message) in &self.blockers {
            lines.push(format!("Cannot start: {message}"));
        }
        for check in &self.checks {
            let mut line = format!(
                "- check {}: {}. {}",
                check.name,
                check.verdict.label(),
                check.message
            );
            if let Some(s) = &check.samples {
                line.push_str(&format!(
                    " ({}/{} samples passed; 95% interval {:.2}-{:.2})",
                    s.passed, s.samples, s.interval.0, s.interval.1
                ));
            }
            lines.push(line);
        }
        if let Some(live) = &self.live {
            let failures = if live.failures.is_empty() {
                "none".to_string()
            } else {
                live.failures
                    .iter()
                    .map(|(what, n)| format!("{what} {n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            lines.push(format!(
                "Live: {} samples, {} completed; agent failures: {failures}; median latency {}; cost {}.",
                live.samples,
                live.completed,
                live.latency_ms_median
                    .map_or("unknown".into(), |ms| format!("{ms} ms")),
                live.cost_usd
                    .map_or("unknown".into(), |c| format!("${c:.4}"))
            ));
        }
        let end = self
            .trace
            .iter()
            .rposition(|e| e.kind == EventKind::Stopped)
            .map_or(self.trace.len(), |i| i + 1);
        let start = end.saturating_sub(events);
        if start < end {
            lines.push(format!(
                "Trace, events {}-{} of {}:",
                start + 1,
                end,
                self.trace.len()
            ));
            for event in &self.trace[start..end] {
                lines.push(format!("  {} ms: {}", event.time_ms, event.text));
            }
        }
        lines.join("\n")
    }

    /// The counts of each verdict.
    pub fn tally(&self) -> Vec<(Verdict, usize)> {
        let mut out: Vec<(Verdict, usize)> = Vec::new();
        for check in &self.checks {
            match out.iter_mut().find(|(v, _)| *v == check.verdict) {
                Some((_, n)) => *n += 1,
                None => out.push((check.verdict, 1)),
            }
        }
        out
    }
}

/// A 95% Wilson score interval for `passed` out of `n`.
pub fn wilson(passed: u32, n: u32) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let z = 1.959_963_984_540_054_f64;
    let n = n as f64;
    let p = passed as f64 / n;
    let denominator = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denominator;
    let half = z * ((p * (1.0 - p) / n) + z * z / (4.0 * n * n)).sqrt() / denominator;
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

/// Now, in UTC, as RFC 3339 (`2026-09-30T21:15:04Z`).
pub fn utc_now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    utc(seconds)
}

/// Seconds since 1970 as RFC 3339, in UTC.
pub fn utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_intervals_widen_with_fewer_samples() {
        let (low, high) = wilson(9, 10);
        assert!(low > 0.55 && low < 0.6, "{low}");
        assert!(high > 0.98 && high <= 1.0, "{high}");
        let (low, high) = wilson(90, 100);
        assert!(low > 0.82 && high < 0.95, "{low} {high}");
        assert_eq!(wilson(0, 0), (0.0, 1.0));
    }

    #[test]
    fn dates_are_utc_rfc_3339() {
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc(1_790_000_000), "2026-09-21T14:13:20Z");
    }
}
