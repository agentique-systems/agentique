//! The inspection drone and its charging station (C-55, W13.6): a second
//! system, of another kind than Agentique, that exercises the same
//! abstractions. Its bounded behavioural question, "is the contact
//! energised only after the permit was accepted, and does a lost answer
//! time out and recover within its budget?", is answered by model
//! execution with a successful case and failure cases; one charger
//! definition gives different outcomes in two contextual usages; and a
//! plausible wrong design is caught by the same scenario.
use agq_language::{Source, Tree, parse, validate};
use agq_simulation::digest::model_digest;
use agq_simulation::requirements::{Evaluation, Status, evaluate_all};
use agq_simulation::{Answers, Mode, Request, RunResult, RunStatus, Verdict, compile, run};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const MODEL: &str = include_str!("../../../models/inspection-charging/InspectionCharging.sysml");

fn load() -> Tree {
    parse(&[Source::new("InspectionCharging.sysml", MODEL)])
}

fn run_model(tree: &Tree, scenario: &str) -> RunResult {
    let id = tree
        .find(&format!("InspectionCharging::{scenario}"))
        .unwrap_or_else(|| panic!("no {scenario}"));
    let program = compile(tree, id).unwrap_or_else(|b| panic!("{scenario} blocked: {b:?}"));
    run(
        &program,
        model_digest(tree, id),
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    )
}

fn verdicts(result: &RunResult) -> Vec<(String, Verdict)> {
    result
        .checks
        .iter()
        .map(|c| (c.name.clone(), c.verdict))
        .collect()
}

fn check(result: &RunResult, name: &str) -> Verdict {
    result
        .checks
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no check {name}: {:?}", verdicts(result)))
        .verdict
}

#[test]
fn the_model_is_valid_under_the_supported_subset() {
    let tree = load();
    let problems: Vec<String> = validate(&tree)
        .into_iter()
        .map(|d| {
            format!(
                "{}: {} ({})",
                tree.qualified_name(d.element),
                d.message,
                d.code
            )
        })
        .collect();
    assert_eq!(problems, [] as [String; 0]);
}

#[test]
fn the_contacts_are_energised_only_after_the_permit_is_accepted() {
    let tree = load();
    let accepted = run_model(&tree, "ChargingAuthorized");
    assert_eq!(
        accepted.status,
        RunStatus::Completed,
        "{:#?}",
        accepted.stop
    );
    assert_eq!(
        check(&accepted, "energizedAfterAcceptance"),
        Verdict::Passed
    );
    let refused = run_model(&tree, "ChargingRefused");
    assert_eq!(refused.status, RunStatus::Completed, "{:#?}", refused.stop);
    assert_eq!(check(&refused, "notEnergized"), Verdict::Passed);
}

#[test]
fn a_lost_answer_times_out_and_is_asked_again_within_the_budget() {
    let tree = load();
    let once = run_model(&tree, "AnswerLostOnce");
    assert_eq!(check(&once, "recoveredAfterRetry"), Verdict::Passed);
    // The retry waited the charger's timeout in logical time: the answer
    // came after 500 ms, not at once.
    assert!(
        once.trace.iter().any(|e| e.time_ms >= 500),
        "the run never reached the timeout"
    );
    let twice = run_model(&tree, "AnswersLostTwiceAtHome");
    assert_eq!(check(&twice, "gaveUpUnenergized"), Verdict::Passed);
}

/// One charger definition, two contextual usages: the field station's
/// redefined timeout and attempts make the same losses end differently,
/// without a copy of the charger or a branch for stations in its behaviour.
#[test]
fn the_same_charger_configured_for_the_field_recovers_where_home_gives_up() {
    let tree = load();
    let home = run_model(&tree, "AnswersLostTwiceAtHome");
    let field = run_model(&tree, "AnswersLostTwiceInTheField");
    assert_eq!(check(&home, "gaveUpUnenergized"), Verdict::Passed);
    assert_eq!(check(&field, "energizedOnThirdAttempt"), Verdict::Passed);
    let charger = tree.find("InspectionCharging::Charger").unwrap();
    let definitions = tree
        .walk()
        .into_iter()
        .filter(|id| {
            tree[*id].kind == agq_language::ElementKind::PartDef
                && tree.effective_name(*id) == Some("Charger")
        })
        .count();
    assert_eq!(definitions, 1, "one charger definition");
    assert!(tree.contains(charger));
}

/// A plausible wrong design (energise on the permit, ask afterwards) is
/// caught: the refused-permit scenario's check fails on it.
#[test]
fn the_scenario_catches_a_charger_that_energises_before_the_answer() {
    let tree = load();
    let quick = run_model(&tree, "RefusedAtAQuickStation");
    assert_eq!(quick.status, RunStatus::Completed, "{:#?}", quick.stop);
    assert_eq!(check(&quick, "notEnergized"), Verdict::Failed);
}

#[test]
fn every_run_is_the_same_run() {
    let tree = load();
    for scenario in [
        "ChargingAuthorized",
        "ChargingRefused",
        "AnswerLostOnce",
        "AnswersLostTwiceAtHome",
        "AnswersLostTwiceInTheField",
        "RefusedAtAQuickStation",
    ] {
        let a = run_model(&tree, scenario);
        let b = run_model(&tree, scenario);
        assert_eq!(verdicts(&a), verdicts(&b), "{scenario}");
        assert_eq!(a.trace.len(), b.trace.len(), "{scenario}");
    }
}

/// Composition versus reference: the flight computer's supply refers to
/// the drone's one bus. Written as a composite part bound to the bus, it
/// would be a second, phantom bus counted twice, which is reported.
#[test]
fn the_shared_bus_is_referred_to_and_a_copy_of_it_is_reported() {
    let shared = "ref part :>> supply = bus;";
    assert!(MODEL.contains(shared), "the model shares the bus");
    let copied = MODEL.replace(shared, "part :>> supply = bus;");
    let tree = parse(&[Source::new("InspectionCharging.sysml", &copied)]);
    let problems: Vec<String> = validate(&tree)
        .into_iter()
        .map(|d| {
            format!(
                "{} {} {}",
                tree.qualified_name(d.element),
                d.code,
                d.message
            )
        })
        .collect();
    assert!(
        problems
            .iter()
            .any(|p| p.contains("flightComputer") && p.contains("wrong-value")),
        "{problems:#?}"
    );
}

fn evaluation(tree: &Tree, requirement: &str, subject: &str) -> Evaluation {
    evaluate_all(tree)
        .into_iter()
        .find(|e| {
            tree.qualified_name(e.requirement) == format!("InspectionCharging::{requirement}")
                && e.subject == subject
        })
        .unwrap_or_else(|| panic!("no evaluation of {requirement} by {subject}"))
}

fn value<'a>(e: &'a Evaluation, name: &str) -> &'a str {
    e.required
        .iter()
        .chain(&e.assumptions)
        .flat_map(|c| &c.values)
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("no value {name} in {e:#?}"))
}

/// The bounded engineering question, by calculation on the modelled
/// configuration: the survey drone's launch mass, its assemblies and payload
/// counted once each, is 5900 g of 7000 g. The flight computer's supply is
/// the drone's own bus, referred to, so the bus's 150 g is counted once (a
/// copy would make it 6050 g).
#[test]
fn the_survey_drone_is_within_its_launch_mass_with_the_bus_counted_once() {
    let tree = load();
    let survey = evaluation(&tree, "launchMass", "surveyUnit");
    assert_eq!(survey.status, Status::Holds, "{}", survey.reason);
    assert_eq!(value(&survey, "aircraft.mass"), "5900");
    assert_eq!(value(&survey, "limit"), "7000");
}

/// One drone definition in two configurations, and one requirement
/// definition in two usages with their own limits: the upgraded drone
/// (650 g avionics, 2300 g payload) breaks the launch limit by 150 g but
/// meets the ferry limit. No definition is copied.
#[test]
fn the_upgraded_drone_violates_the_launch_limit_but_meets_the_ferry_limit() {
    let tree = load();
    let launch = evaluation(&tree, "launchMass", "heavyUnit");
    assert_eq!(launch.status, Status::Violated, "{}", launch.reason);
    assert_eq!(value(&launch, "aircraft.mass"), "7150");
    assert!(
        launch.reason.contains("aircraft.mass <= limit"),
        "{}",
        launch.reason
    );
    let ferry = evaluation(&tree, "ferryMass", "heavyUnit");
    assert_eq!(ferry.status, Status::Holds, "{}", ferry.reason);
    assert_eq!(value(&ferry, "limit"), "8000");
}

/// The same charger definition, configured for home and for the field:
/// both bound the drone's wait (500 ms × 2 and 2000 ms × 3, the field one
/// exactly at the 6000 ms limit); one more attempt in the field breaks it.
#[test]
fn the_wait_for_a_decision_is_bounded_and_a_misconfiguration_is_caught() {
    let tree = load();
    let home = evaluation(&tree, "shortWait", "homeStation.charger");
    assert_eq!(home.status, Status::Holds, "{}", home.reason);
    assert_eq!(
        value(&home, "charger.timeoutMs * charger.maxAttempts"),
        "1000"
    );
    let field = evaluation(&tree, "shortWait", "fieldStation.charger");
    assert_eq!(field.status, Status::Holds, "{}", field.reason);
    assert_eq!(
        value(&field, "charger.timeoutMs * charger.maxAttempts"),
        "6000"
    );
    let misconfigured = MODEL.replace(
        "attribute :>> maxAttempts = 3;",
        "attribute :>> maxAttempts = 4;",
    );
    assert_ne!(misconfigured, MODEL);
    let tree = parse(&[Source::new("InspectionCharging.sysml", &misconfigured)]);
    let field = evaluation(&tree, "shortWait", "fieldStation.charger");
    assert_eq!(field.status, Status::Violated, "{}", field.reason);
    assert_eq!(
        value(&field, "charger.timeoutMs * charger.maxAttempts"),
        "8000"
    );
}

/// The behavioural requirement (energise only after authorisation) is
/// informal text: it is not calculated, and never shown as holding on its
/// declaration; its evidence is the scenarios that verify it.
#[test]
fn the_behavioural_requirement_is_not_calculated_but_verified_by_scenarios() {
    let tree = load();
    let declared = evaluation(&tree, "authorizedCharging", "homeStation");
    assert_eq!(declared.status, Status::NotEvaluable, "{}", declared.reason);
    let requirement = tree.find("InspectionCharging::authorizedCharging").unwrap();
    let verifying = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == agq_language::ElementKind::Verify)
        .filter(|id| tree[*id].target.as_ref().and_then(|t| t.target()) == Some(requirement))
        .count();
    assert_eq!(verifying, 6, "six scenarios verify it");
}
