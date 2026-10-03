//! The URL shortener with AI link screening (Scenario I): the agent's
//! contract, its fallback, stand-ins with injected failures, replay from
//! recordings without a network fallback, and a live evaluation through a
//! model client (a scripted one here: CI never calls a provider).
use agq_language::{Source, Tree, parse, validate};
use agq_simulation::agents::{CallLimits, LiveAnswer, LiveModel, Recording, Recordings};
use agq_simulation::digest::model_digest;
use agq_simulation::{
    AgentRequest, Answers, EventKind, Mode, Outcome, Request, RunResult, RunStatus, StopReason,
    Verdict, compile, run,
};
use serde_json::{Value as Json, json};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const SHORTENER: &str = include_str!("../../../models/link-screening/UrlShortener.sysml");

fn load(text: &str) -> Tree {
    parse(&[Source::new("UrlShortener.sysml", text)])
}

fn run_in(tree: &Tree, scenario: &str, mode: Mode, answers: Answers) -> RunResult {
    let id = tree
        .find(&format!("UrlShortener::{scenario}"))
        .unwrap_or_else(|| panic!("no {scenario}"));
    let program = compile(tree, id).unwrap_or_else(|b| panic!("blocked: {b:#?}"));
    let mut request = Request::new(mode);
    request.samples = 5;
    run(
        &program,
        model_digest(tree, id),
        &request,
        answers,
        Arc::new(AtomicBool::new(false)),
    )
}

fn model(tree: &Tree, scenario: &str) -> RunResult {
    run_in(tree, scenario, Mode::Model, Answers::StandIns)
}

fn verdicts(result: &RunResult) -> Vec<(&str, Verdict)> {
    result
        .checks
        .iter()
        .map(|c| (c.name.as_str(), c.verdict))
        .collect()
}

fn events(result: &RunResult, kind: EventKind) -> Vec<String> {
    result
        .trace
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| e.text.clone())
        .collect()
}

#[test]
fn the_model_is_valid() {
    let tree = load(SHORTENER);
    let problems: Vec<String> = validate(&tree)
        .into_iter()
        .map(|d| {
            format!(
                "{} {}: {}",
                tree.qualified_name(d.element),
                d.code,
                d.message
            )
        })
        .collect();
    assert_eq!(problems, [] as [String; 0]);
    assert_eq!(
        agq_language::print(&tree)[0].text,
        SHORTENER,
        "canonical text"
    );
}

#[test]
fn every_screening_outcome_runs_with_stand_ins_and_keeps_the_rule() {
    let tree = load(SHORTENER);
    for scenario in [
        "ShortenAllowed",
        "ShortenBlocked",
        "ScreeningTimesOut",
        "InvalidScreeningOutput",
        "ReviewRequired",
        "BlocklistedWhenScreeningRefuses",
    ] {
        let result = model(&tree, scenario);
        assert_eq!(
            result.status,
            RunStatus::Completed,
            "{scenario}: {:#?}",
            result.stop
        );
        assert!(result.all_passed(), "{scenario}: {:#?}", verdicts(&result));
    }
}

#[test]
fn a_timeout_goes_to_the_fallback_after_max_latency_and_the_trace_says_why() {
    let tree = load(SHORTENER);
    let result = model(&tree, "ScreeningTimesOut");
    let failed = events(&result, EventKind::AgentFailed);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(
        failed[0].contains("no answer within 500 ms"),
        "{}",
        failed[0]
    );
    let fallback = events(&result, EventKind::Fallback);
    assert!(
        fallback[0].contains("service.screening.fallback"),
        "{fallback:?}"
    );
    let answered = result
        .trace
        .iter()
        .find(|e| e.kind == EventKind::AgentFailed)
        .unwrap();
    assert_eq!(
        answered.time_ms, 500,
        "the timeout is noticed at maxLatencyMs"
    );
    let called = result
        .trace
        .iter()
        .find(|e| e.kind == EventKind::AgentCalled)
        .unwrap();
    assert_eq!(called.source.as_deref(), Some("stand-in slow"));
}

#[test]
fn invalid_output_and_low_confidence_are_contract_failures_not_answers() {
    let tree = load(SHORTENER);
    let invalid = model(&tree, "InvalidScreeningOutput");
    let failed = events(&invalid, EventKind::AgentFailed);
    assert!(
        failed[0].contains("confidence 1.7 is not between 0 and 1"),
        "{failed:?}"
    );
    let unsure = model(&tree, "ReviewRequired");
    let failed = events(&unsure, EventKind::AgentFailed);
    assert!(failed[0].contains("below minConfidence 0.8"), "{failed:?}");
}

#[test]
fn a_usage_can_override_the_agents_settings() {
    // With minConfidence 0.5 on the usage, the unsure allow goes through.
    let text = SHORTENER.replace(
        "        part screening : LinkScreening;\n",
        "        part screening : LinkScreening {\n            :>> minConfidence = 0.5;\n        }\n",
    );
    let tree = load(&text);
    let result = model(&tree, "ReviewRequired");
    assert_eq!(result.checks[0].name, "isHeld");
    assert_eq!(result.checks[0].verdict, Verdict::Failed);
    assert!(
        result.checks[0].message.contains("link.status = active"),
        "{}",
        result.checks[0].message
    );
}

#[test]
fn an_agent_without_a_fallback_stops_where_it_fails() {
    let text = SHORTENER.replace("        part : BlocklistScreening :>> fallback;\n", "");
    let tree = load(&text);
    let result = model(&tree, "ScreeningTimesOut");
    let stop = result.stop.unwrap();
    assert_eq!(stop.reason, StopReason::AgentFailedWithoutFallback);
    assert!(
        stop.message.contains("no outcome is guessed"),
        "{}",
        stop.message
    );
    assert_eq!(
        stop.element,
        Some(
            tree.find("UrlShortener::UrlShortenerService::screening")
                .unwrap()
                .raw()
        )
    );
}

#[test]
fn a_broken_rule_in_the_model_is_caught() {
    // Break the modelled rule: a review verdict activates the link.
    let broken = SHORTENER.replacen(
        "status = LinkStatus::held) via storage;",
        "status = LinkStatus::active) via storage;",
        1,
    );
    let tree = load(&broken);
    let result = model(&tree, "ReviewRequired");
    let verdicts = verdicts(&result);
    assert_eq!(
        verdicts[1],
        ("heldDoesNotRedirect", Verdict::Failed),
        "{verdicts:?}"
    );
}

#[test]
fn evaluation_cases_do_not_run_without_answers_and_say_so() {
    let tree = load(SHORTENER);
    let result = model(&tree, "ScreeningCases");
    let stop = result.stop.unwrap();
    assert_eq!(stop.reason, StopReason::MissingStandIn);
    assert!(
        stop.message.contains("never calls a real model"),
        "{}",
        stop.message
    );
}

/// A scripted live model: answers by host, counting calls.
struct Scripted {
    calls: AtomicU32,
    seen: Mutex<Vec<AgentRequest>>,
}

impl LiveModel for Scripted {
    fn label(&self) -> String {
        "scripted/screening-1".into()
    }

    fn answer(&self, request: &AgentRequest, _: CallLimits, _: &AtomicBool) -> LiveAnswer {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(request.clone());
        let host = request.input["fields"]["host"].as_str().unwrap_or("");
        let (decision, confidence) = match host {
            "free-gift-cards.example" | "paypa1-login.example" => ("block", 0.92),
            // Every fifth call it hesitates on documentation.
            "docs.rust-lang.example" if n % 5 == 3 => ("review", 0.6),
            _ => ("allow", 0.9),
        };
        LiveAnswer {
            outcome: Outcome::Answer,
            output: Some(
                json!({"type": "Verdict", "fields": {"decision": decision, "confidence": confidence, "reason": "scripted"}}),
            ),
            latency_ms: 200,
            cost_usd: Some(0.0001),
            error: None,
            evidence: None,
        }
    }
}

#[test]
fn a_live_evaluation_reports_samples_intervals_and_provenance() {
    let tree = load(SHORTENER);
    let live = Arc::new(Scripted {
        calls: AtomicU32::new(0),
        seen: Mutex::new(Vec::new()),
    });
    let result = run_in(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live.clone()),
    );
    assert_eq!(result.mode, Mode::Live);
    let summary = result.live.clone().unwrap();
    assert_eq!(summary.samples, 5);
    assert_eq!(live.calls.load(Ordering::SeqCst), 20, "4 cases x 5 samples");
    let provenance = result.provenance.live.clone().unwrap();
    assert_eq!(provenance.provider, "scripted");
    assert_eq!(provenance.model, "screening-1");
    assert!(!provenance.instructions_digest.is_empty());
    let documentation = result
        .checks
        .iter()
        .find(|c| c.name == "documentationIsAllowed")
        .unwrap();
    // It hesitates in some samples: not a pass, not a fail.
    assert_eq!(
        documentation.verdict,
        Verdict::Inconclusive,
        "{}",
        documentation.message
    );
    let counts = documentation.samples.clone().unwrap();
    assert!(counts.failed > 0 && counts.passed > 0);
    assert!(!documentation.deterministic);
    assert!(counts.interval.0 < counts.interval.1);
    let scam = result
        .checks
        .iter()
        .find(|c| c.name == "giftCardScamIsNotAllowed")
        .unwrap();
    assert_eq!(scam.verdict, Verdict::Passed);
    assert!(summary.cost_usd.unwrap() > 0.0);
    // The request carries the instructions and the output's shape.
    let seen = live.seen.lock().unwrap();
    assert!(
        seen[0]
            .instructions
            .contains("Decides whether a new short link may go live")
    );
    assert_eq!(seen[0].model.as_deref(), Some("deepseek-flash"));
    assert_eq!(seen[0].output["type"], "Verdict");
    assert!(
        seen[0].output["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "decision" && f["values"] == json!(["allow", "review", "block"]))
    );
}

#[test]
fn replay_uses_recordings_by_request_digest_and_never_falls_through() {
    let tree = load(SHORTENER);
    let folder = tempfile::tempdir().unwrap();
    // Nothing recorded yet: the replay stops at the first call.
    let empty = Arc::new(Recordings::read(folder.path()));
    let result = run_in(
        &tree,
        "ScreeningCases",
        Mode::Replay,
        Answers::Recordings(empty),
    );
    let stop = result.stop.clone().unwrap();
    assert_eq!(stop.reason, StopReason::MissingRecording);
    assert!(
        stop.message.contains("never falls through to a live call"),
        "{}",
        stop.message
    );
    // Record a live run's answers, as "keep this run as recordings" does.
    let live = Arc::new(Scripted {
        calls: AtomicU32::new(0),
        seen: Mutex::new(Vec::new()),
    });
    let id = tree.find("UrlShortener::ScreeningCases").unwrap();
    let program = compile(&tree, id).unwrap();
    let mut recordings = Vec::new();
    for request in first_sample_requests(&program, live.clone()) {
        let limits = CallLimits {
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(1),
        };
        let answer = live.answer(&request, limits, &AtomicBool::new(false));
        recordings.push(Recording {
            digest: request.digest(),
            request,
            outcome: answer.outcome,
            output: answer.output,
            latency_ms: answer.latency_ms,
            answered_by: live.label(),
            recorded_at: "2026-09-30T00:00:00Z".into(),
            run: None,
            evidence: None,
        });
    }
    assert_eq!(Recordings::keep(folder.path(), &recordings).unwrap(), 4);
    assert_eq!(
        Recordings::keep(folder.path(), &recordings).unwrap(),
        0,
        "kept once"
    );
    let kept = Arc::new(Recordings::read(folder.path()));
    assert_eq!(kept.len(), 4);
    let first = run_in(
        &tree,
        "ScreeningCases",
        Mode::Replay,
        Answers::Recordings(kept.clone()),
    );
    let second = run_in(
        &tree,
        "ScreeningCases",
        Mode::Replay,
        Answers::Recordings(kept.clone()),
    );
    assert_eq!(first.status, RunStatus::Completed, "{:#?}", first.stop);
    assert!(
        first.all_passed(),
        "{:#?}",
        events(&first, EventKind::AgentFailed)
    );
    assert_eq!(first.trace, second.trace, "replay is deterministic");
    assert!(
        first
            .trace
            .iter()
            .filter_map(|e| e.source.as_ref())
            .all(|s| s.starts_with("recording ")),
    );
    assert_eq!(
        first.provenance.recordings.as_deref(),
        Some(kept.digest.as_str())
    );
    // Changing the agent's instructions changes the request: no recording
    // matches, and the replay says so rather than guessing.
    let changed = load(&SHORTENER.replace(
        "Decides whether a new short link may go live.",
        "Decides whether a new short link may be published.",
    ));
    let result = run_in(
        &changed,
        "ScreeningCases",
        Mode::Replay,
        Answers::Recordings(kept),
    );
    assert_eq!(result.stop.unwrap().reason, StopReason::MissingRecording);
}

/// The requests a run makes, answered live once, to record them.
fn first_sample_requests(
    program: &agq_simulation::Program,
    live: Arc<Scripted>,
) -> Vec<AgentRequest> {
    let before = live.seen.lock().unwrap().len();
    let mut request = Request::new(Mode::Live);
    request.samples = 1;
    let _ = run(
        program,
        String::new(),
        &request,
        Answers::Live(live.clone()),
        Arc::new(AtomicBool::new(false)),
    );
    let seen = live.seen.lock().unwrap();
    seen[before..].to_vec()
}

#[test]
fn agent_requests_are_canonical_json_with_a_stable_digest() {
    let request = AgentRequest {
        agent: "P::A".into(),
        mode: Some("fast".into()),
        model: None,
        instructions: "x".into(),
        input: json!({"b": 1, "a": 2}),
        output: Json::Null,
        binding: None,
    };
    assert_eq!(
        request.canonical(),
        r#"{"agent":"P::A","input":{"a":2,"b":1},"instructions":"x","mode":"fast","model":null,"output":null}"#
    );
    assert_eq!(request.digest(), request.clone().digest());
    assert_eq!(request.digest().len(), 64);
}
