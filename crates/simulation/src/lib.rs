//! Simulation (ROADMAP §4.14; part `Simulation` in
//! `model/Agentique.sysml`): scenarios run over a fixed model
//! snapshot, with results that say what actually ran.
//!
//! - [`compile`]: a scenario (a `verification def`) and the model it runs
//!   over, copied into disposable runtime structures. What keeps it from
//!   running is reported at the elements concerned.
//! - [`script`]: the scenario's steps and checks, run against a [`Target`]:
//!   the model engine here, or a real implementation's harness (the
//!   Implementation part). The same interpreter evaluates the same checks.
//! - [`engine`]: model execution in logical time, with stand-ins, recorded
//!   replay or a live model client for agents.
//! - [`agents`]: agent requests, recordings, and the live model client the
//!   Studio implements; Simulation never reaches a provider by itself.
//! - [`result`]: statuses, stop reasons, check verdicts, traces and
//!   provenance; [`digest`] and [`freshness`] say whether a result is still
//!   current; [`store`] keeps results in the app's per-project data.
//! - [`runner`]: a run on a background thread that can be cancelled.
//! - [`requirements`]: requirement evaluation (C-55): a requirement's
//!   assumed and required constraints calculated on the modelled
//!   configuration of what satisfies it; nothing runs.
//!
//! Nothing here changes the System State, and nothing here derives
//! behaviour from a name, an icon or documentation.
#![forbid(unsafe_code)]

pub mod agents;
pub mod compile;
pub mod digest;
pub mod engine;
pub mod eval;
pub mod requirements;
pub mod result;
pub mod runner;
pub mod script;
pub mod store;
pub mod value;

pub use agents::{
    AgentRequest, Binding, CallLimits, Evidence, LiveAnswer, LiveModel, Recording, Recordings,
    RunBinding,
};
pub use compile::{Blocker, Outcome, Program, compile};
pub use engine::{AgentCall, AgentInfo, AnswerField, Answers, Limits, describe_agents};
pub use result::{
    CheckResult, EventKind, Mode, RunResult, RunStatus, Stop, StopReason, TraceEvent, Verdict,
    summary,
};
pub use runner::BackgroundRun;
pub use script::{Output, Target, Trace, run_steps};
pub use store::RunStore;
pub use value::Value;

use agq_language::{ElementId, Tree};
use result::{LiveProvenance, LiveSummary, Provenance, SampleCounts, wilson};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// This runner's name and version, recorded in every result.
pub const RUNNER: &str = concat!("agq-simulation ", env!("CARGO_PKG_VERSION"));

/// What to run and how.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub mode: Mode,
    pub limits: Limits,
    /// Recorded for later stochastic workloads; this subset has no randomness.
    pub seed: u64,
    /// The System State revision the snapshot was taken at.
    pub model_revision: u64,
    /// Live evaluation: samples of the whole scenario.
    pub samples: u32,
    /// Replay and live: the binding the Studio prepared for the scenario's
    /// agent (C-52). Never stored; its digest is in the provenance.
    pub binding: Option<RunBinding>,
}

impl Request {
    pub fn new(mode: Mode) -> Request {
        let mut limits = Limits::default();
        // A live sample waits for a real model's answers: minutes, not the
        // seconds a model run is allowed.
        if mode == Mode::Live {
            limits.max_wall = std::time::Duration::from_secs(300);
        }
        Request {
            mode,
            limits,
            seed: 0,
            model_revision: 0,
            samples: 1,
            binding: None,
        }
    }
}

/// A result's freshness, computed against the present model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Freshness {
    Current,
    /// Why it no longer shows the present.
    Outdated(String),
}

impl Freshness {
    pub fn is_current(&self) -> bool {
        *self == Freshness::Current
    }
}

/// What a result is compared with: the model now and, for replay and live
/// results, the binding the Studio would prepare now and the digest of the
/// recordings now (C-52).
#[derive(Clone, Copy, Debug)]
pub struct Present<'a> {
    pub tree: &'a Tree,
    /// The digest of the binding a replay or live run of the scenario would
    /// get now: `Ok(None)` when it asks no agent, `Err(why)` when none can
    /// be prepared.
    pub binding: Result<Option<&'a str>, &'a str>,
    /// The digest of the project's recordings now.
    pub recordings: Option<&'a str>,
}

impl<'a> Present<'a> {
    /// The model alone: enough for model runs and walkthroughs; replay and
    /// live results compared with it are outdated.
    pub fn model(tree: &'a Tree) -> Present<'a> {
        Present {
            tree,
            binding: Err("only the model was compared"),
            recordings: None,
        }
    }
}

/// Whether a result still describes the present: its scenario exists, its
/// model slice is unchanged, this runner made it, and, for a replay or a
/// live evaluation, the binding and (for a replay) the recordings are the
/// same. A replay or live result without a binding was made before
/// bindings were recorded and is never current. (Implementation results
/// also depend on code; the Implementation part compares that.)
pub fn freshness(result: &RunResult, present: &Present) -> Freshness {
    let scenario = ElementId::from_raw(result.scenario);
    if !present.tree.contains(scenario) {
        return Freshness::Outdated("its scenario no longer exists".into());
    }
    if digest::model_digest(present.tree, scenario) != result.provenance.model_digest {
        return Freshness::Outdated(
            "the model it ran over has changed since (the scenario, its subject or something they use)".into(),
        );
    }
    let runner = format!("{RUNNER} {}", result.mode.key());
    if result.provenance.runner != runner {
        return Freshness::Outdated(format!(
            "another version of the runner made it ({})",
            result.provenance.runner
        ));
    }
    if matches!(result.mode, Mode::Replay | Mode::Live) {
        let outdated = match (&result.provenance.binding, present.binding) {
            (_, Err(why)) => Some(format!(
                "how the agent's model is called cannot be compared now ({why})"
            )),
            (None, Ok(None)) => None,
            (Some(_), Ok(None)) => Some("the scenario no longer asks the agent it asked".into()),
            (None, Ok(Some(_))) => Some(
                "it was made before Agentique recorded which provider, model and mapping answered"
                    .into(),
            ),
            (Some(then), Ok(Some(now))) => (then != now).then(|| {
                "how the agent's model is called has changed since (the provider, the model, or how its answer is asked for and mapped)".into()
            }),
        };
        if let Some(why) = outdated {
            return Freshness::Outdated(why);
        }
    }
    if result.mode == Mode::Replay && result.provenance.recordings.as_deref() != present.recordings
    {
        return Freshness::Outdated("the recordings have changed since".into());
    }
    Freshness::Current
}

/// A fresh result id: the start time and a counter.
pub fn new_run_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!(
        "{seconds:x}-{:x}-{:x}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    )
}

/// Runs a compiled scenario in `request.mode` (model, replay, live or
/// walkthrough; implementation runs belong to the Implementation part and
/// use [`run_steps`] with their own target). `model_digest` is the digest
/// of the scenario's model slice when it was compiled.
pub fn run(
    program: &Program,
    model_digest: String,
    request: &Request,
    answers: Answers,
    cancel: Arc<AtomicBool>,
) -> RunResult {
    let started = Instant::now();
    let mut result = begin(program, model_digest, request, &answers);
    let scenario = &program.scenario;
    match request.mode {
        Mode::Walkthrough => walkthrough(program, &mut result),
        Mode::Implementation => {
            result.status = RunStatus::Blocked;
            result.blockers.push((
                scenario.element.raw(),
                "an implementation run needs the project's harness; start it from the Implementation part".into(),
            ));
        }
        Mode::Live => {
            let label = match &answers {
                Answers::Live(model) => model.label(),
                _ => {
                    result.status = RunStatus::Blocked;
                    result.blockers.push((
                        scenario.element.raw(),
                        "a live evaluation needs a live model client, started explicitly by the Operator".into(),
                    ));
                    return result;
                }
            };
            live(program, request, answers, &cancel, &mut result, label);
        }
        Mode::Model | Mode::Replay => {
            let mut trace = Trace::default();
            let (outcome, _calls, logical, events) =
                once(program, request, answers, &cancel, &mut trace);
            finish(&mut result, outcome, &mut trace);
            result.logical_ms = logical;
            result.events_processed = events;
        }
    }
    result.wall_ms = started.elapsed().as_millis() as u64;
    result
}

/// A result for `program`, not yet run: its identity, mode and provenance.
/// Runners in other parts (the implementation runner) start from it.
pub fn begin(
    program: &Program,
    model_digest: String,
    request: &Request,
    answers: &Answers,
) -> RunResult {
    let scenario = &program.scenario;
    RunResult {
        format: result::FORMAT,
        id: new_run_id(),
        scenario: scenario.element.raw(),
        scenario_name: scenario.name.clone(),
        scenario_qualified_name: scenario.qualified_name.clone(),
        mode: request.mode,
        started: result::utc_now(),
        wall_ms: 0,
        logical_ms: 0,
        events_processed: 0,
        status: RunStatus::Completed,
        stop: None,
        blockers: Vec::new(),
        checks: Vec::new(),
        verifies: scenario
            .verifies
            .iter()
            .map(|(id, name)| (id.raw(), name.clone()))
            .collect(),
        trace: Vec::new(),
        provenance: Provenance {
            runner: format!("{RUNNER} {}", request.mode.key()),
            model_digest,
            model_revision: request.model_revision,
            seed: request.seed,
            implementation: None,
            live: None,
            recordings: match answers {
                Answers::Recordings(recordings) => Some(recordings.digest.clone()),
                _ => None,
            },
            binding: match (request.mode, &request.binding) {
                (Mode::Replay | Mode::Live, Some(prepared)) => Some(prepared.binding.digest()),
                _ => None,
            },
        },
        live: None,
    }
}

/// Every check of the scenario as not run, for a run that stopped first.
pub fn not_run_checks(program: &Program) -> Vec<CheckResult> {
    program
        .scenario
        .steps
        .iter()
        .filter_map(|step| match step {
            compile::Step::Check {
                element,
                name,
                text,
                ..
            } => Some(CheckResult {
                element: element.raw(),
                name: name.clone(),
                expression: text.clone(),
                verdict: Verdict::NotRun,
                message: "the run stopped before this check".into(),
                deterministic: true,
                samples: None,
                implicit: false,
            }),
            _ => None,
        })
        .collect()
}

/// Puts the steps' outcome and the trace into the result.
pub fn finish(result: &mut RunResult, outcome: script::Outcome, trace: &mut Trace) {
    result.checks = outcome.checks;
    if let Some(stop) = outcome.stop {
        result.status = if stop.reason == StopReason::Cancelled {
            RunStatus::Cancelled
        } else {
            RunStatus::Stopped
        };
        result.stop = Some(stop);
    }
    result.trace = std::mem::take(&mut trace.events);
}

/// One pass of the steps against the model engine.
fn once(
    program: &Program,
    request: &Request,
    answers: Answers,
    cancel: &Arc<AtomicBool>,
    trace: &mut Trace,
) -> (script::Outcome, Vec<AgentCall>, u64, u64) {
    match engine::Engine::start(
        program,
        answers,
        request.limits,
        request.binding.clone(),
        cancel.clone(),
        trace,
    ) {
        Ok(mut engine) => {
            let outcome = run_steps(program, &mut engine, trace, cancel);
            let calls = std::mem::take(&mut engine.agent_calls);
            let logical = engine.now_ms();
            let events = engine.events();
            (outcome, calls, logical, events)
        }
        Err(stop) => {
            // Nothing ran: every check is not run.
            let checks = not_run_checks(program);
            trace.push(
                0,
                EventKind::Stopped,
                format!("Stopped ({}): {}", stop.reason.code(), stop.message),
                stop.element.map(ElementId::from_raw).into_iter().collect(),
            );
            (
                script::Outcome {
                    checks,
                    stop: Some(stop),
                    unexpected: Vec::new(),
                },
                Vec::new(),
                0,
                0,
            )
        }
    }
}

fn walkthrough(program: &Program, result: &mut RunResult) {
    let mut trace = Trace::default();
    trace.push(
        0,
        EventKind::Note,
        "A walkthrough shows the intended steps in order; nothing runs and nothing is verified.",
        vec![program.scenario.element],
    );
    for step in &program.scenario.steps {
        let text = match step {
            compile::Step::Send { port, .. } => format!("Send into `{}`", port.text),
            compile::Step::Accept {
                port, type_name, ..
            } => {
                format!("Expect a `{type_name}` from `{}`", port.text)
            }
            compile::Step::Wait { .. } => "Wait".into(),
            compile::Step::Check { name, text, .. } => format!("Check `{name}`: {text}"),
        };
        trace.push(0, EventKind::Step, text, vec![step.element()]);
        if let compile::Step::Check {
            element,
            name,
            text,
            ..
        } = step
        {
            result.checks.push(CheckResult {
                element: element.raw(),
                name: name.clone(),
                expression: text.clone(),
                verdict: Verdict::NotRun,
                message: "a walkthrough verifies nothing".into(),
                deterministic: true,
                samples: None,
                implicit: false,
            });
        }
    }
    result.status = RunStatus::Walkthrough;
    result.trace = trace.events;
}

/// A live evaluation: the scenario `samples` times, agents answered by the
/// live model. Per check: counts, an interval, and a verdict that is
/// `passed` or `failed` only when every sample agrees.
fn live(
    program: &Program,
    request: &Request,
    answers: Answers,
    cancel: &Arc<AtomicBool>,
    result: &mut RunResult,
    label: String,
) {
    let Answers::Live(model) = answers else {
        return;
    };
    let samples = request.samples.max(1);
    let mut per_check: Vec<(CheckResult, u32, u32, u32)> = Vec::new();
    let mut failures: Vec<(String, u32)> = Vec::new();
    let mut latencies = Vec::new();
    let mut cost = 0.0;
    let mut costed = false;
    let mut unknown_cost = 0;
    let mut made = 0;
    let mut remaining = request.limits.max_live_calls;
    let mut completed = 0;
    let mut trace = Trace::default();
    let mut instructions = String::new();
    let mut answers: Vec<Recording> = Vec::new();
    for sample in 1..=samples {
        if cancel.load(Ordering::SeqCst) {
            result.status = RunStatus::Cancelled;
            break;
        }
        trace.push(
            0,
            EventKind::Note,
            format!("Sample {sample} of {samples}"),
            vec![program.scenario.element],
        );
        // The allowance is shared by every sample.
        let mut this = request.clone();
        this.limits.max_live_calls = remaining;
        let (outcome, calls, logical, events) = once(
            program,
            &this,
            Answers::Live(model.clone()),
            cancel,
            &mut trace,
        );
        let calls_made = calls.len() as u32;
        made += calls_made;
        remaining = remaining.map(|n| n.saturating_sub(calls_made));
        result.logical_ms = result.logical_ms.max(logical);
        result.events_processed += events;
        for call in &calls {
            if let Some(agents::AnswerOutput::Json(output)) = &call.answer.output
                && !answers
                    .iter()
                    .any(|r: &Recording| r.digest == call.request.digest())
            {
                answers.push(Recording {
                    digest: call.request.digest(),
                    request: call.request.clone(),
                    outcome: call.answer.outcome,
                    output: Some(output.clone()),
                    latency_ms: call.answer.latency_ms,
                    answered_by: call
                        .answer
                        .evidence
                        .as_ref()
                        .and_then(|e| e.model.as_ref())
                        .map(|answered| {
                            let provider = label.split_once('/').map_or("", |(p, _)| p);
                            format!("{provider}/{answered}")
                        })
                        .unwrap_or_else(|| label.clone()),
                    recorded_at: result::utc_now(),
                    run: Some(result.id.clone()),
                    evidence: call.answer.evidence.clone(),
                });
            }
            if instructions.is_empty() {
                instructions = call.request.instructions.clone();
            }
            latencies.push(call.answer.latency_ms);
            match call.answer.cost_usd {
                Some(c) => {
                    cost += c;
                    costed = true;
                }
                // Never counted as free.
                None => unknown_cost += 1,
            }
            if let Some(failure) = &call.failure {
                let category = if call.provider_error.is_some() {
                    "providerError".to_string()
                } else if failure == "cancelled" {
                    "cancelled".to_string()
                } else if call.answer.outcome != compile::Outcome::Answer {
                    call.answer.outcome.name().to_string()
                } else if failure.contains("below minConfidence") {
                    "lowConfidence".into()
                } else if failure.contains("over maxLatencyMs") {
                    "timeout".into()
                } else {
                    "invalidOutput".into()
                };
                match failures.iter_mut().find(|(c, _)| *c == category) {
                    Some((_, n)) => *n += 1,
                    None => failures.push((category, 1)),
                }
            }
        }
        // A stop caused by the last call (a provider failure, a stop during
        // the call) is already counted with that call.
        let by_last_call = calls.last().is_some_and(|c| {
            c.provider_error.is_some() || c.failure.as_deref() == Some("cancelled")
        });
        let mut ends_evaluation = None;
        if outcome.stop.is_none() {
            completed += 1;
        } else if let Some(stop) = &outcome.stop {
            let caused = by_last_call
                && matches!(
                    stop.reason,
                    StopReason::HarnessFailed | StopReason::Cancelled
                );
            if !caused {
                let category = stop.reason.code().to_string();
                match failures.iter_mut().find(|(c, _)| *c == category) {
                    Some((_, n)) => *n += 1,
                    None => failures.push((category, 1)),
                }
            }
            // A provider that cannot be asked, or an allowance used up, ends
            // the evaluation: no further sample sends anything (C-52).
            if matches!(
                stop.reason,
                StopReason::HarnessFailed | StopReason::BudgetExhausted | StopReason::Cancelled
            ) {
                ends_evaluation = Some(stop.clone());
            }
        }
        for check in outcome.checks {
            let entry = match per_check
                .iter_mut()
                .find(|(c, ..)| c.element == check.element && c.name == check.name)
            {
                Some(entry) => entry,
                None => {
                    per_check.push((check.clone(), 0, 0, 0));
                    per_check.last_mut().expect("pushed")
                }
            };
            match check.verdict {
                Verdict::Passed => entry.1 += 1,
                Verdict::Failed => entry.2 += 1,
                _ => entry.3 += 1,
            }
        }
        if let Some(stop) = ends_evaluation {
            result.status = if stop.reason == StopReason::Cancelled {
                RunStatus::Cancelled
            } else {
                RunStatus::Stopped
            };
            trace.push(
                0,
                EventKind::Note,
                format!(
                    "The evaluation ended after sample {sample} of {samples}: {}",
                    stop.message
                ),
                vec![program.scenario.element],
            );
            result.stop = Some(stop);
            break;
        }
    }
    latencies.sort_unstable();
    result.checks = per_check
        .into_iter()
        .map(|(mut check, passed, failed, other)| {
            let decided = passed + failed;
            check.verdict = if decided == 0 {
                Verdict::NotRun
            } else if failed == 0 && other == 0 {
                Verdict::Passed
            } else if passed == 0 && other == 0 {
                Verdict::Failed
            } else {
                Verdict::Inconclusive
            };
            check.message = format!(
                "passed in {passed} of {samples} samples, failed in {failed}{}",
                if other > 0 {
                    format!(", not decided in {other}")
                } else {
                    String::new()
                }
            );
            check.deterministic = false;
            check.samples = Some(SampleCounts {
                samples,
                passed,
                failed,
                other,
                interval: wilson(passed, decided),
            });
            check
        })
        .collect();
    result.live = Some(LiveSummary {
        samples,
        completed,
        failures,
        cost_usd: (costed && unknown_cost == 0).then_some(cost),
        latency_ms_median: latencies.get(latencies.len() / 2).copied(),
        answers,
        calls: made,
        unknown_cost,
        known_cost_usd: (costed && unknown_cost > 0).then_some(cost),
    });
    let (provider, model_name) = label
        .split_once('/')
        .map(|(p, m)| (p.to_string(), m.to_string()))
        .unwrap_or((label.clone(), String::new()));
    result.provenance.live = Some(LiveProvenance {
        provider,
        model: model_name,
        instructions_digest: digest::text_digest(&instructions),
        instructions_source: "the agent's documentation".into(),
        samples,
    });
    result.trace = trace.events;
}
