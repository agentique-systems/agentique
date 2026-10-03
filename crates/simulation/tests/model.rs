//! Model execution on the retrying notification dispatcher (Scenario I's
//! second example): real runs, traces, failing checks, explicit stops,
//! determinism, isolation from edits, cancellation and limits.
use agq_language::{Source, Tree, parse};
use agq_simulation::digest::model_digest;
use agq_simulation::{
    Answers, EventKind, Limits, Mode, Present, Request, RunResult, RunStatus, StopReason, Verdict,
    compile, freshness, run,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const NOTIFICATIONS: &str = include_str!("../../../models/notifications/Notifications.sysml");

fn load(text: &str) -> Tree {
    parse(&[Source::new("Notifications.sysml", text)])
}

fn run_model(tree: &Tree, scenario: &str) -> RunResult {
    run_with(tree, scenario, Request::new(Mode::Model))
}

fn run_with(tree: &Tree, scenario: &str, request: Request) -> RunResult {
    let id = tree
        .find(scenario)
        .unwrap_or_else(|| panic!("no {scenario}"));
    let program = compile(tree, id).unwrap_or_else(|b| panic!("blocked: {b:?}"));
    run(
        &program,
        model_digest(tree, id),
        &request,
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    )
}

fn verdicts(result: &RunResult) -> Vec<(&str, Verdict)> {
    result
        .checks
        .iter()
        .map(|c| (c.name.as_str(), c.verdict))
        .collect()
}

fn texts(result: &RunResult, kind: EventKind) -> Vec<String> {
    result
        .trace
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| e.text.clone())
        .collect()
}

#[test]
fn a_failing_gateway_is_retried_and_the_notification_delivered_once() {
    let tree = load(NOTIFICATIONS);
    let result = run_model(&tree, "Notifications::RetryThenDeliver");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(
        verdicts(&result),
        [
            ("deliveredOnThirdAttempt", Verdict::Passed),
            ("no unexpected output", Verdict::Passed)
        ]
    );
    // Backoff: 200 ms after the first failure, 400 ms after the second.
    assert_eq!(result.logical_ms, 600);
    let requests = texts(&result, EventKind::Sent)
        .into_iter()
        .filter(|t| t.contains("SendRequest("))
        .count();
    assert_eq!(requests, 3, "three attempts");
    let stand_ins = texts(&result, EventKind::StandIn);
    assert_eq!(stand_ins.len(), 3, "{stand_ins:?}");
    assert!(stand_ins[0].contains("failFirst"));
    assert!(stand_ins[2].contains("succeedThird"));
    assert!(
        texts(&result, EventKind::Transition)
            .iter()
            .any(|t| t.contains("sending → waiting"))
    );
    assert!(result.all_passed());
    assert_eq!(result.mode, Mode::Model);
    assert!(result.provenance.runner.contains("model"));
}

#[test]
fn giving_up_reads_internal_state_and_ends_as_failed() {
    let tree = load(NOTIFICATIONS);
    let result = run_model(&tree, "Notifications::GiveUpAfterThreeAttempts");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(
        verdicts(&result),
        [
            ("endsAsFailed", Verdict::Passed),
            ("dispatcherIsIdle", Verdict::Passed),
            ("no unexpected output", Verdict::Passed)
        ]
    );
    // Three calls of 50 ms and waits of 200 and 400 ms, then the 5 s wait.
    assert_eq!(result.logical_ms, 150 + 600 + 5000);
}

#[test]
fn a_broken_retry_guard_is_caught_by_the_model_run() {
    // The deliberate model break: the dispatcher retries once too often.
    let broken = NOTIFICATIONS.replace(
        "if not r.ok and attempts < maxAttempts then waiting",
        "if not r.ok and attempts <= maxAttempts then waiting",
    );
    let tree = load(&broken);
    let result = run_model(&tree, "Notifications::GiveUpAfterThreeAttempts");
    // With `<=` both the retry and the give-up transition are enabled on the
    // third failure: the run stops instead of guessing.
    assert_eq!(result.status, RunStatus::Stopped);
    let stop = result.stop.clone().unwrap();
    assert_eq!(stop.reason, StopReason::AmbiguousTransition, "{stop:?}");
    assert!(
        stop.message.contains("sending → waiting or sending → idle"),
        "{}",
        stop.message
    );
    assert_eq!(
        verdicts(&result),
        [
            ("endsAsFailed", Verdict::NotRun),
            ("dispatcherIsIdle", Verdict::NotRun),
            ("no unexpected output", Verdict::NotRun)
        ]
    );
    // When the give-up guard is broken to match, the dispatcher makes a
    // fourth attempt: the check sees it in the receipt.
    let broken = broken.replace(
        "if not r.ok and attempts >= maxAttempts do",
        "if not r.ok and attempts > maxAttempts do",
    );
    let tree = load(&broken);
    let result = run_model(&tree, "Notifications::GiveUpAfterThreeAttempts");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(result.checks[0].verdict, Verdict::Failed);
    assert!(
        result.checks[0].message.contains("receipt.attempts = 4"),
        "{}",
        result.checks[0].message
    );
}

#[test]
fn a_part_without_behaviour_stops_the_run_where_the_message_arrives() {
    // Without stand-ins, the gateway's (empty) definition has to answer.
    let tree = load(NOTIFICATIONS);
    let id = tree.find("Notifications::RetryThenDeliver").unwrap();
    let mut tree = tree;
    for name in ["failFirst", "failSecond", "succeedThird"] {
        let stand_in = tree
            .find(&format!("Notifications::RetryThenDeliver::{name}"))
            .unwrap();
        tree.remove(stand_in);
    }
    let program = compile(&tree, id).unwrap();
    let result = run(
        &program,
        model_digest(&tree, id),
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    );
    let stop = result.stop.unwrap();
    assert_eq!(stop.reason, StopReason::MissingBehaviour);
    assert_eq!(
        stop.element,
        Some(
            tree.find("Notifications::NotificationService::gateway")
                .unwrap()
                .raw()
        )
    );
    assert!(
        stop.message.contains("`service.gateway` received"),
        "{}",
        stop.message
    );
}

#[test]
fn a_message_no_transition_accepts_stops_the_run() {
    // The dispatcher no longer accepts results while sending.
    let broken = NOTIFICATIONS.replace(
        "transition first sending accept r : SendResult via gateway if r.ok do",
        "transition first waiting accept r : SendResult via gateway if r.ok do",
    );
    let tree = load(&broken);
    let result = run_model(&tree, "Notifications::RetryThenDeliver");
    let stop = result.stop.unwrap();
    // The first result (not ok) is still accepted; the third (ok) is not.
    assert_eq!(stop.reason, StopReason::UnhandledMessage, "{stop:?}");
    assert!(stop.message.contains("while sending"), "{}", stop.message);
}

#[test]
fn runs_are_deterministic_and_isolated_from_later_edits() {
    let mut tree = load(NOTIFICATIONS);
    let id = tree.find("Notifications::RetryThenDeliver").unwrap();
    let program = compile(&tree, id).unwrap();
    let digest = model_digest(&tree, id);
    let first = run(
        &program,
        digest.clone(),
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    );
    // Edit the model after compiling: the compiled run is its own snapshot.
    let backoff = tree.find("Notifications::Dispatcher::backoffMs").unwrap();
    tree.get_mut(backoff).unwrap().value = Some(agq_language::Literal::Integer("1000".into()));
    let second = run(
        &program,
        digest,
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(first.trace, second.trace);
    assert_eq!(second.logical_ms, 600);
    // The edit outdates the earlier result.
    assert!(!freshness(&first, &Present::model(&tree)).is_current());
    // Compiled again, the new value is used.
    let third = run_model(&tree, "Notifications::RetryThenDeliver");
    assert_eq!(third.logical_ms, 1000 + 2000);
    assert!(freshness(&third, &Present::model(&tree)).is_current());
}

#[test]
fn an_unrelated_edit_keeps_a_result_current() {
    let mut tree = load(&NOTIFICATIONS.replace(
        "    requirement def DeliveredOnce {",
        "    part def Unrelated;\n\n    requirement def DeliveredOnce {",
    ));
    let result = run_model(&tree, "Notifications::RetryThenDeliver");
    let unrelated = tree.find("Notifications::Unrelated").unwrap();
    tree.get_mut(unrelated).unwrap().name = Some("StillUnrelated".into());
    assert!(freshness(&result, &Present::model(&tree)).is_current());
}

#[test]
fn cancellation_and_limits_end_a_run_explicitly() {
    let tree = load(NOTIFICATIONS);
    let id = tree.find("Notifications::RetryThenDeliver").unwrap();
    let program = compile(&tree, id).unwrap();
    let cancelled = run(
        &program,
        model_digest(&tree, id),
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(true)),
    );
    assert_eq!(cancelled.status, RunStatus::Cancelled);
    let mut request = Request::new(Mode::Model);
    request.limits = Limits {
        max_events: 5,
        ..Limits::default()
    };
    let limited = run(
        &program,
        model_digest(&tree, id),
        &request,
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(limited.stop.unwrap().reason, StopReason::EventLimit);
    let mut request = Request::new(Mode::Model);
    request.limits = Limits {
        max_time_ms: 300,
        ..Limits::default()
    };
    let limited = run(
        &program,
        model_digest(&tree, id),
        &request,
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(limited.stop.unwrap().reason, StopReason::TimeLimit);
}

#[test]
fn a_walkthrough_runs_nothing_and_verifies_nothing() {
    let tree = load(NOTIFICATIONS);
    let result = run_with(
        &tree,
        "Notifications::RetryThenDeliver",
        Request::new(Mode::Walkthrough),
    );
    assert_eq!(result.status, RunStatus::Walkthrough);
    assert!(result.checks.iter().all(|c| c.verdict == Verdict::NotRun));
    assert!(!result.all_passed());
    assert_eq!(texts(&result, EventKind::Step).len(), 3);
}

#[test]
fn a_scenario_that_cannot_run_says_why_at_the_element() {
    let broken = NOTIFICATIONS.replace(
        "        subject service : NotificationService;\n        objective {\n            verify deliveredOnce;\n        }\n        part failFirst",
        "        objective {\n            verify deliveredOnce;\n        }\n        part failFirst",
    );
    let tree = load(&broken);
    let id = tree.find("Notifications::RetryThenDeliver").unwrap();
    let blockers = compile(&tree, id).unwrap_err();
    assert!(
        blockers.iter().any(|b| b.message.contains("no subject")),
        "{blockers:?}"
    );
}
