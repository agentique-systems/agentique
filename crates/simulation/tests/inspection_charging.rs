//! The inspection drone and its charging station (C-55, W13.6): a second
//! system, of another kind than Agentique, that exercises the same
//! abstractions. Its bounded questions are answered by calculation on the
//! modelled configurations (the launch mass, the bounded wait) and by model
//! execution (energise only after the permit is accepted; a lost answer
//! times out and is asked again within the budget), each with a successful
//! case and failure cases; one charger definition gives different outcomes
//! in two contextual usages, and a plausible wrong design is caught.
use agq_language::{ElementKind, Semantics, Source, Tree, parse, validate};
use agq_simulation::digest::model_digest;
use agq_simulation::requirements::{Evaluation, Status, evaluate_all};
use agq_simulation::{
    Answers, EventKind, Mode, Request, RunResult, RunStatus, Verdict, compile, run,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const MODEL: &str = include_str!("../../../models/inspection-charging/InspectionCharging.sysml");

fn load() -> Tree {
    parse(&[Source::new("InspectionCharging.sysml", MODEL)])
}

fn load_text(text: &str) -> Tree {
    parse(&[Source::new("InspectionCharging.sysml", text)])
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

fn check(result: &RunResult, name: &str) -> Verdict {
    result
        .checks
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no check {name}: {:#?}", result.checks))
        .verdict
}

fn problems(tree: &Tree) -> Vec<String> {
    validate(tree)
        .into_iter()
        .map(|d| {
            format!(
                "{}: {} ({})",
                tree.qualified_name(d.element),
                d.message,
                d.code
            )
        })
        .collect()
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

/// When the charger's contacts were first energised, if ever.
fn energised_at(result: &RunResult) -> Option<(u64, u64)> {
    result
        .trace
        .iter()
        .find(|e| {
            e.kind == EventKind::Assigned && e.text.contains("energized") && e.text.contains("true")
        })
        .map(|e| (e.time_ms, e.seq))
}

/// When the authorisation service's answer reached the charger.
fn answered_at(result: &RunResult) -> Option<(u64, u64)> {
    result
        .trace
        .iter()
        .find(|e| e.kind == EventKind::Received && e.text.contains("PermitAnswer"))
        .map(|e| (e.time_ms, e.seq))
}

/// When the station's charge status left it (its decision).
fn decided_at(result: &RunResult) -> u64 {
    result
        .trace
        .iter()
        .find(|e| e.kind == EventKind::Output)
        .map(|e| e.time_ms)
        .expect("a status came out")
}

#[test]
fn the_model_is_valid_under_the_supported_subset() {
    assert_eq!(problems(&load()), [] as [String; 0]);
}

// ---- Composition and sharing ----

/// The flight computer's supply is the drone's own bus: one occurrence in
/// two roles, so reading the bus through either role gives the same part.
#[test]
fn the_flight_computers_supply_is_the_drones_one_bus() {
    let text = MODEL.trim_end().strip_suffix('}').unwrap().to_string()
        + "    requirement def OneBus {
        subject d : Drone;
        require constraint { d.flightComputer.supply.mass == d.bus.mass }
    }
    requirement oneBus : OneBus;
    satisfy oneBus by surveyUnit;
}
";
    let tree = load_text(&text);
    assert_eq!(problems(&tree), [] as [String; 0]);
    let same = evaluation(&tree, "oneBus", "surveyUnit");
    assert_eq!(same.status, Status::Holds, "{}", same.reason);
    assert_eq!(value(&same, "d.flightComputer.supply.mass"), "150");
}

/// The bus is counted once because the sum names it once and the
/// controller's mass excludes it. The reference does not do the accounting:
/// a sum that also added the supply would count the same bus twice.
#[test]
fn the_accounting_is_in_the_sum_and_adding_the_supply_counts_the_bus_twice() {
    let survey = evaluation(&load(), "launchMass", "surveyUnit");
    assert_eq!(value(&survey, "aircraft.mass"), "5900");
    let twice = MODEL.replace(
        "+ propulsion.mass + payloadMass;",
        "+ propulsion.mass + payloadMass + flightComputer.supply.mass;",
    );
    assert_ne!(twice, MODEL);
    let counted_twice = evaluation(&load_text(&twice), "launchMass", "surveyUnit");
    assert_eq!(value(&counted_twice, "aircraft.mass"), "6050");
}

/// A composite part bound to the bus would be the bus under a second name
/// while composed by another owner: reported (SysML 7.6.3).
#[test]
fn a_composite_part_bound_to_the_bus_is_reported() {
    let shared = "ref part :>> supply = bus;";
    assert!(MODEL.contains(shared));
    let found = problems(&load_text(&MODEL.replace(shared, "part :>> supply = bus;")));
    assert!(
        found
            .iter()
            .any(|p| p.contains("flightComputer") && p.contains("wrong-value")),
        "{found:#?}"
    );
}

/// A limit, stated: a second, composite bus inside the controller (a
/// phantom component) is not reported, and a sum that does not name it
/// leaves it out of the budget. Agentique does not check that a roll-up
/// covers every composite part.
#[test]
fn a_phantom_second_bus_is_left_out_of_the_budget_unreported() {
    let tree = load_text(&MODEL.replace(
        "ref part :>> supply = bus;",
        "ref part :>> supply = bus;\n            part spare : PowerBus {\n                attribute :>> mass = 150;\n            }",
    ));
    assert_eq!(problems(&tree), [] as [String; 0]);
    let survey = evaluation(&tree, "launchMass", "surveyUnit");
    assert_eq!(
        value(&survey, "aircraft.mass"),
        "5900",
        "the phantom bus is not counted"
    );
}

// ---- Calculation on the modelled configuration ----

/// The survey drone's launch mass is 5900 g of 7000 g.
#[test]
fn the_survey_drone_is_within_its_launch_mass() {
    let survey = evaluation(&load(), "launchMass", "surveyUnit");
    assert_eq!(survey.status, Status::Holds, "{}", survey.reason);
    assert_eq!(value(&survey, "aircraft.mass"), "5900");
    assert_eq!(value(&survey, "limit"), "7000");
}

/// The upgraded drone states only what changed from the survey drone (650 g
/// avionics, 2300 g payload): it breaks the launch limit by 150 g but meets
/// the ferry limit. One requirement definition, two usages with their own
/// limits; its satisfy declaration for the launch limit is shown wrong.
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

/// The limit applies to a drone carrying a payload; without one the
/// requirement claims nothing.
#[test]
fn without_a_payload_the_launch_limit_claims_nothing() {
    let tree = load_text(&MODEL.replacen(
        "attribute :>> payloadMass = 1200;",
        "attribute :>> payloadMass = 0;",
        1,
    ));
    let empty = evaluation(&tree, "launchMass", "surveyUnit");
    assert_eq!(empty.status, Status::AssumptionsNotMet, "{}", empty.reason);
}

/// The same charger definition, configured for home and for the field:
/// both bound the drone's wait (500 ms × 2, and 2000 ms × 3 exactly at the
/// 6000 ms limit); one more attempt in the field breaks it.
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
    let field = evaluation(
        &load_text(&misconfigured),
        "shortWait",
        "fieldStation.charger",
    );
    assert_eq!(field.status, Status::Violated, "{}", field.reason);
    assert_eq!(
        value(&field, "charger.timeoutMs * charger.maxAttempts"),
        "8000"
    );
}

/// Units are not modelled (deviation 14): a measurement expression is
/// reported, never silently read as a number.
#[test]
fn a_value_with_a_unit_is_reported_not_read() {
    let tree = load_text(&MODEL.replacen(
        "attribute :>> mass = 1800;",
        "attribute :>> mass = 1.8[kg];",
        1,
    ));
    let found = problems(&tree);
    assert!(
        found
            .iter()
            .any(|p| p.contains("unsupported") || p.contains("syntax")),
        "{found:#?}"
    );
}

// ---- Behaviour in model execution ----

/// Accepted: the contacts are off while the answer is pending, and on only
/// once the answer has arrived (read from the trace).
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
    assert_eq!(check(&accepted, "offWhileAsking"), Verdict::Passed);
    assert_eq!(
        check(&accepted, "energizedAfterAcceptance"),
        Verdict::Passed
    );
    let (on_ms, on_seq) = energised_at(&accepted).expect("energised");
    let (answer_ms, answer_seq) = answered_at(&accepted).expect("answered");
    assert!(on_ms >= 120 && on_ms >= answer_ms && on_seq > answer_seq);
    let refused = run_model(&tree, "ChargingRefused");
    assert_eq!(check(&refused, "offWhileAsking"), Verdict::Passed);
    assert_eq!(check(&refused, "notEnergized"), Verdict::Passed);
    assert_eq!(energised_at(&refused), None);
}

/// A lost answer is asked again after the charger's timeout (500 ms), and
/// two lost answers end a home station's attempt at its bounded wait
/// (1000 ms, the calculated value).
#[test]
fn a_lost_answer_times_out_and_is_asked_again_within_the_budget() {
    let tree = load();
    let once = run_model(&tree, "AnswerLostOnce");
    assert_eq!(check(&once, "recoveredAfterRetry"), Verdict::Passed);
    assert_eq!(decided_at(&once), 500);
    let twice = run_model(&tree, "AnswersLostTwiceAtHome");
    assert_eq!(check(&twice, "gaveUpUnenergized"), Verdict::Passed);
    assert_eq!(decided_at(&twice), 1000);
}

/// One charger definition, two contextual usages: the field station's
/// redefined timeout and attempts make the same losses end differently,
/// and its bounded wait (6000 ms) is where a field station gives up.
#[test]
fn the_same_charger_configured_for_the_field_recovers_where_home_gives_up() {
    let tree = load();
    let field = run_model(&tree, "AnswersLostTwiceInTheField");
    assert_eq!(check(&field, "energizedOnThirdAttempt"), Verdict::Passed);
    assert_eq!(decided_at(&field), 4000);
    let all_lost = run_model(&tree, "EveryAnswerLostInTheField");
    assert_eq!(
        check(&all_lost, "gaveUpAfterThreeAttempts"),
        Verdict::Passed
    );
    assert_eq!(decided_at(&all_lost), 6000);
    // The field station's charger is the one Charger definition, not a copy.
    let semantics = Semantics::new(&tree);
    let charger = tree.find("InspectionCharging::Charger").unwrap();
    let field_charger = tree
        .find("InspectionCharging::FieldStation::charger")
        .unwrap();
    assert!(
        semantics
            .types_of(field_charger)
            .iter()
            .any(|(t, _)| *t == charger),
        "the field station's charger is typed by Charger"
    );
    let machines: Vec<String> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::State && tree[*id].exhibit)
        .map(|id| tree.qualified_name(id))
        .collect();
    assert_eq!(
        machines,
        [
            "InspectionCharging::Charger::charging",
            "InspectionCharging::EagerCharger::charging"
        ]
    );
}

/// A plausible wrong design (energise on the permit, ask afterwards,
/// switch off on a refusal) is caught by checking the contacts while the
/// answer is pending, not by the end state alone; it fails its own
/// requirement, not the correct station's.
#[test]
fn the_scenario_catches_a_charger_that_energises_before_the_answer() {
    let tree = load();
    let quick = run_model(&tree, "RefusedAtAQuickStation");
    assert_eq!(quick.status, RunStatus::Completed, "{:#?}", quick.stop);
    assert_eq!(check(&quick, "offWhileAsking"), Verdict::Failed);
    assert_eq!(
        check(&quick, "offAfterRefusal"),
        Verdict::Passed,
        "the end state alone would not show it"
    );
    let (on_ms, on_seq) = energised_at(&quick).expect("energised");
    let (_, answer_seq) = answered_at(&quick).expect("answered");
    assert!(on_ms < 120 && on_seq < answer_seq);
    let verifies = |requirement: &str| {
        let id = tree
            .find(&format!("InspectionCharging::{requirement}"))
            .unwrap();
        tree.walk()
            .into_iter()
            .filter(|v| tree[*v].kind == ElementKind::Verify)
            .filter(|v| tree[*v].target.as_ref().and_then(|t| t.target()) == Some(id))
            .count()
    };
    assert_eq!(verifies("authorizedCharging"), 6);
    assert_eq!(verifies("quickAuthorizedCharging"), 1);
}

/// The behavioural requirement is informal: not calculated, never shown as
/// holding on its declaration; its evidence is its scenarios.
#[test]
fn the_behavioural_requirement_is_not_calculated() {
    let declared = evaluation(&load(), "authorizedCharging", "homeStation");
    assert_eq!(declared.status, Status::NotEvaluable, "{}", declared.reason);
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
        "EveryAnswerLostInTheField",
        "RefusedAtAQuickStation",
    ] {
        let a = run_model(&tree, scenario);
        let b = run_model(&tree, scenario);
        assert_eq!(a.trace, b.trace, "{scenario}");
        assert_eq!(a.logical_ms, b.logical_ms, "{scenario}");
        assert_eq!(a.checks, b.checks, "{scenario}");
    }
}
