//! The factory tools (C-50, W8.3): the Assistant writes scenarios and
//! state machines through `apply_changes`, reads behaviour, and asks the
//! Studio to run scenarios; here the requests are carried out headless.

use agq_assistant::Prepared;
use agq_assistant::tools::{
    self, APPLY_CHANGES, INSPECT_BEHAVIOUR, LIST_SCENARIOS, READ_RUN, RUN_SCENARIO, StudioRequest,
};
use agq_language::{Source, parse, print};
use agq_library::Library;
use agq_simulation::Mode;
use agq_system_state::SystemState;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SCREENING: &str = include_str!("../../../models/link-screening/UrlShortener.sysml");

fn screening() -> SystemState {
    let state = SystemState::new(
        parse(&[Source::new("UrlShortener.sysml", SCREENING)]),
        BTreeSet::new(),
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    state
}

fn prepare(state: &SystemState, tool: &str, input: Value) -> Prepared {
    tools::check_input(tool, &input).unwrap_or_else(|e| panic!("{tool}: {e}"));
    tools::prepare(state, &Library::built_in_only(), tool, &input)
}

fn apply(state: &mut SystemState, input: Value) {
    match prepare(state, APPLY_CHANGES, input) {
        Prepared::Change(change) => {
            state.apply(change).expect("applied");
        }
        other => panic!("expected a change, got {other:?}"),
    }
}

fn text(prepared: Prepared) -> String {
    match prepared {
        Prepared::Answer(text) => text,
        other => panic!("expected an answer, got {other:?}"),
    }
}

fn written(state: &SystemState) -> String {
    print(state.tree()).into_iter().map(|s| s.text).collect()
}

fn run(state: &SystemState, scenario: &str, mode: &str) -> Result<String, String> {
    match prepare(
        state,
        RUN_SCENARIO,
        json!({ "scenario": scenario, "mode": mode }),
    ) {
        Prepared::Studio(request) => tools::carry_out_headless(state.tree(), &request),
        other => panic!("expected a Studio request, got {other:?}"),
    }
}

#[test]
fn the_assistant_writes_a_scenario_and_runs_it() {
    let mut state = screening();
    let at = "UrlShortener::ShortenWhenScreeningFails";
    apply(
        &mut state,
        json!({ "description": "A scenario for a screening timeout", "operations": [
            { "op": "create", "parent": "UrlShortener", "kind": "verification def", "name": "ShortenWhenScreeningFails", "subject": "UrlShortenerService", "subject_name": "service", "verifies": ["screenedLinks"], "doc": "Screening times out: the fallback holds the link." },
            { "op": "create", "parent": at, "kind": "part", "name": "slow", "type": "Scenarios::StandIn", "features": { "target": "service.screening", "outcome": "Scenarios::Outcome::timeout", "latencyMs": 50 } },
            { "op": "create", "parent": at, "kind": "send", "expression": "new ShortenRequest(longUrl = \"https://a.example/x\", host = \"a.example\")", "via": "service.shorten" },
            { "op": "create", "parent": at, "kind": "accept", "name": "link", "type": "ShortLink", "via": "service.shorten" },
            { "op": "create", "parent": at, "kind": "assert constraint", "name": "isHeld", "expression": "link.status == LinkStatus::held" }
        ]}),
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text_now = written(&state);
    for line in [
        "verification def ShortenWhenScreeningFails {",
        "subject service : UrlShortenerService;",
        "verify screenedLinks;",
        ":>> outcome = Scenarios::Outcome::timeout;",
        "then accept link : ShortLink via service.shorten;",
        "link.status == LinkStatus::held",
    ] {
        assert!(text_now.contains(line), "{line} in\n{text_now}");
    }
    let result = run(&state, at, "model").expect("it runs");
    assert!(result.contains(": completed,"), "{result}");
    assert!(result.contains("check isHeld: passed"), "{result}");
    // A walkthrough verifies nothing.
    let walk = run(&state, at, "walkthrough").unwrap();
    assert!(walk.contains("check isHeld: not run"), "{walk}");
    // The scenario is listed, as written.
    assert!(text(prepare(&state, LIST_SCENARIOS, json!({}))).contains("ShortenWhenScreeningFails"));
}

#[test]
fn a_failing_check_says_why_and_a_live_run_is_the_operators() {
    let mut state = screening();
    // The unsure allow activates the link with a low minimum confidence.
    apply(
        &mut state,
        json!({ "description": "Lower the agent's minimum confidence", "operations": [
            { "op": "set", "element": "UrlShortener::LinkScreening", "features": { "minConfidence": 0.5 } }
        ]}),
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let result = run(&state, "UrlShortener::ReviewRequired", "model").unwrap();
    assert!(result.contains(": failed."), "{result}");
    // Live evaluation costs money: the Assistant cannot start one, by the
    // schema and again when the call is prepared.
    let input = json!({ "scenario": "UrlShortener::ReviewRequired", "mode": "live" });
    assert!(tools::check_input(RUN_SCENARIO, &input).is_err());
    let live = tools::prepare(&state, &Library::built_in_only(), RUN_SCENARIO, &input);
    assert!(
        matches!(live, Prepared::Invalid(ref m) if m.contains("only the Operator")),
        "{live:?}"
    );
    // Only scenarios run.
    let wrong = prepare(
        &state,
        RUN_SCENARIO,
        json!({ "scenario": "UrlShortener::LinkStore" }),
    );
    assert!(
        matches!(wrong, Prepared::Invalid(ref m) if m.contains("not a scenario")),
        "{wrong:?}"
    );
    // Reading a result is the Studio's: it keeps them.
    assert_eq!(
        prepare(
            &state,
            READ_RUN,
            json!({ "scenario": "UrlShortener::ReviewRequired" })
        ),
        Prepared::Studio(StudioRequest::ReadRun {
            scenario: state.tree().find("UrlShortener::ReviewRequired").unwrap(),
            mode: Mode::Model
        })
    );
}

#[test]
fn replay_without_recordings_stops_rather_than_calling_a_model() {
    let state = screening();
    let result = run(&state, "UrlShortener::ShortenAllowed", "replay").unwrap();
    assert!(result.contains("stopped"), "{result}");
    assert!(result.contains("missing-recording"), "{result}");
}

#[test]
fn inspect_behaviour_reads_ports_machine_agent_and_parts() {
    let state = screening();
    let service = text(prepare(
        &state,
        INSPECT_BEHAVIOUR,
        json!({ "element": "UrlShortener::UrlShortenerService" }),
    ));
    assert!(
        service.contains("- shorten: takes ShortenRequest; gives ShortLink"),
        "{service}"
    );
    assert!(service.contains("- screening : LinkScreening"), "{service}");
    assert!(service.contains("No state machine"), "{service}");
    let api = text(prepare(
        &state,
        INSPECT_BEHAVIOUR,
        json!({ "element": "UrlShortener::LinkApi" }),
    ));
    assert!(api.contains("state idle;"), "{api}");
    let agent = text(prepare(
        &state,
        INSPECT_BEHAVIOUR,
        json!({ "element": "UrlShortener::LinkScreening" }),
    ));
    assert!(agent.contains("- minConfidence: 0.8"), "{agent}");
    assert!(agent.contains("- model: \"deepseek-flash\""), "{agent}");
    assert!(
        agent.contains("Instructions (its doc): Decides whether"),
        "{agent}"
    );
}

#[test]
fn the_assistant_writes_a_state_machine() {
    let mut state = SystemState::new(
        parse(&[Source::new(
            "Jobs.sysml",
            "package Jobs {
    item def Job { attribute size : ScalarValues::Integer; }
    item def Ack { attribute size : ScalarValues::Integer; }
    port def JobPort { in item job : Job; out item ack : Ack; }
    part def Worker { port jobs : JobPort; }
}",
        )]),
        BTreeSet::new(),
    );
    apply(
        &mut state,
        json!({ "description": "The worker acknowledges jobs", "operations": [
            { "op": "create", "parent": "Jobs::Worker", "kind": "state", "name": "working", "exhibit": true, "initial": "idle" },
            { "op": "create", "parent": "Jobs::Worker::working", "kind": "state", "name": "idle" },
            { "op": "create", "parent": "Jobs::Worker::working", "kind": "transition", "from": "idle", "to": "idle", "trigger": { "name": "job", "type": "Job", "via": "jobs" }, "guard": "job.size > 0", "effect": { "send": "new Ack(size = job.size)", "via": "jobs" } },
            { "op": "create", "parent": "Jobs", "kind": "verification def", "name": "Acknowledges", "subject": "Worker" },
            { "op": "create", "parent": "Jobs::Acknowledges", "kind": "send", "expression": "new Job(size = 3)", "via": "worker.jobs" },
            { "op": "create", "parent": "Jobs::Acknowledges", "kind": "accept", "name": "ack", "type": "Ack", "via": "worker.jobs" },
            { "op": "create", "parent": "Jobs::Acknowledges", "kind": "assert constraint", "name": "sameSize", "expression": "ack.size == 3" }
        ]}),
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text_now = written(&state);
    assert!(
        text_now.contains("transition first idle accept job : Job via jobs if job.size > 0 do send new Ack(size = job.size) via jobs then idle;"),
        "{text_now}"
    );
    let result = run(&state, "Jobs::Acknowledges", "model").unwrap();
    assert!(result.contains("check sameSize: passed"), "{result}");
}

#[test]
fn factory_inputs_are_checked_against_their_schemas() {
    for (tool, input) in [
        (RUN_SCENARIO, json!({})),
        (RUN_SCENARIO, json!({ "scenario": "X", "mode": "fast" })),
        (INSPECT_BEHAVIOUR, json!({ "element": 3 })),
        (
            APPLY_CHANGES,
            json!({ "description": "x", "operations": [{ "op": "create", "features": ["target"] }] }),
        ),
        (
            APPLY_CHANGES,
            json!({ "description": "x", "operations": [{ "op": "create", "trigger": { "port": "p" } }] }),
        ),
    ] {
        assert!(tools::check_input(tool, &input).is_err(), "{tool} {input}");
    }
    // An expression that cannot be read changes nothing and says why.
    let state = screening();
    let bad = tools::prepare(
        &state,
        &Library::built_in_only(),
        APPLY_CHANGES,
        &json!({ "description": "x", "operations": [
            { "op": "create", "parent": "UrlShortener::ShortenAllowed", "kind": "assert constraint", "expression": "link.status ==" }
        ]}),
    );
    assert!(
        matches!(bad, Prepared::Invalid(ref m) if m.contains("cannot be read")),
        "{bad:?}"
    );
}
