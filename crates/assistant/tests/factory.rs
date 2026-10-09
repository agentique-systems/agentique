//! The factory tools (C-50, W8.3): the Assistant writes scenarios and
//! state machines through `apply_changes`, reads behaviour, and asks the
//! Studio to run scenarios; here the requests are carried out headless.

use agq_assistant::Prepared;
use agq_assistant::tools::{
    self, APPLY_CHANGES, INSPECT_BEHAVIOUR, LIST_SCENARIOS, ObjectiveProposal, PROPOSE_OBJECTIVE,
    READ_RUN, RUN_SCENARIO, StudioRequest,
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

/// C-55: an effect of several steps (assign, then send) is one composite
/// action whose steps run in the order given, so agents can write the
/// behaviour model execution runs without editing text.
#[test]
fn the_assistant_writes_an_effect_of_several_steps() {
    let mut state = SystemState::new(
        parse(&[Source::new(
            "Jobs.sysml",
            "package Jobs {
    item def Job { attribute size : ScalarValues::Integer; }
    item def Ack { attribute size : ScalarValues::Integer; attribute count : ScalarValues::Integer; }
    port def JobPort { in item job : Job; out item ack : Ack; }
    part def Worker {
        port jobs : JobPort;
        attribute count : ScalarValues::Integer = 0;
    }
}",
        )]),
        BTreeSet::new(),
    );
    apply(
        &mut state,
        json!({ "description": "The worker counts jobs and acknowledges each", "operations": [
            { "op": "create", "parent": "Jobs::Worker", "kind": "state", "name": "working", "exhibit": true, "initial": "idle" },
            { "op": "create", "parent": "Jobs::Worker::working", "kind": "state", "name": "idle" },
            { "op": "create", "parent": "Jobs::Worker::working", "kind": "transition", "from": "idle", "to": "idle", "trigger": { "name": "job", "type": "Job", "via": "jobs" }, "effect": [
                { "assign": "count", "value": "count + 1" },
                { "send": "new Ack(size = job.size, count = count)", "via": "jobs" }
            ] },
            { "op": "create", "parent": "Jobs", "kind": "verification def", "name": "CountsJobs", "subject": "Worker" },
            { "op": "create", "parent": "Jobs::CountsJobs", "kind": "send", "expression": "new Job(size = 3)", "via": "worker.jobs" },
            { "op": "create", "parent": "Jobs::CountsJobs", "kind": "accept", "name": "one", "type": "Ack", "via": "worker.jobs" },
            { "op": "create", "parent": "Jobs::CountsJobs", "kind": "send", "expression": "new Job(size = 5)", "via": "worker.jobs" },
            { "op": "create", "parent": "Jobs::CountsJobs", "kind": "accept", "name": "two", "type": "Ack", "via": "worker.jobs" },
            { "op": "create", "parent": "Jobs::CountsJobs", "kind": "assert constraint", "name": "countedInOrder", "expression": "one.count == 1 and two.count == 2 and two.size == 5" }
        ]}),
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text_now = written(&state);
    assert!(
        text_now.contains("do action {")
            && text_now.contains("assign count := count + 1;")
            && text_now.contains("then send new Ack(size = job.size, count = count) via jobs;"),
        "{text_now}"
    );
    let result = run(&state, "Jobs::CountsJobs", "model").unwrap();
    assert!(result.contains("check countedInOrder: passed"), "{result}");
    // An empty list is refused, and so is a step that is neither.
    for effect in [json!([]), json!([{ "send": "x", "assign": "y" }])] {
        let refused = tools::prepare(
            &state,
            &Library::built_in_only(),
            APPLY_CHANGES,
            &json!({ "description": "x", "operations": [
                { "op": "create", "parent": "Jobs::Worker::working", "kind": "transition", "from": "idle", "to": "idle", "trigger": { "type": "Job", "via": "jobs" }, "effect": effect }
            ]}),
        );
        assert!(matches!(refused, Prepared::Invalid(_)), "{refused:?}");
    }
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

/// C-54: the Assistant proposes an objective; only the Studio can show it
/// to the Operator, who starts it or not. Headless, nothing is shown and
/// nothing starts.
#[test]
fn the_assistant_proposes_an_objective_and_starts_nothing() {
    let state = screening();
    let proposed = prepare(
        &state,
        PROPOSE_OBJECTIVE,
        json!({ "intent": "  Find and fix problems in the Library panel ", "explore": true,
                "budgets": { "usd": 2.5, "cycles": 3, "steps": 40 } }),
    );
    let expected = ObjectiveProposal {
        intent: "Find and fix problems in the Library panel".into(),
        explore: true,
        usd: Some(2.5),
        cycles: Some(3),
        attempts: None,
        hours: None,
        steps: Some(40),
    };
    assert_eq!(
        proposed,
        Prepared::Studio(StudioRequest::ProposeObjective(expected.clone()))
    );
    // Only the intent is needed; the start form has the rest.
    assert_eq!(
        prepare(
            &state,
            PROPOSE_OBJECTIVE,
            json!({ "intent": "Tidy the Inspector" })
        ),
        Prepared::Studio(StudioRequest::ProposeObjective(ObjectiveProposal {
            intent: "Tidy the Inspector".into(),
            ..ObjectiveProposal::default()
        }))
    );
    // An empty intent, or an improvement and a half, is refused (the
    // turn's input check refuses the second first).
    let half = json!({ "intent": "x", "budgets": { "cycles": 1.5 } });
    assert!(tools::check_input(PROPOSE_OBJECTIVE, &half).is_err());
    for input in [json!({ "intent": " " }), half] {
        let refused = tools::prepare(&state, &Library::built_in_only(), PROPOSE_OBJECTIVE, &input);
        assert!(matches!(refused, Prepared::Invalid(_)), "{refused:?}");
    }
    assert!(
        tools::check_input(PROPOSE_OBJECTIVE, &json!({ "intent": "x", "start": true })).is_err(),
        "there is no way to start it"
    );
    let headless =
        tools::carry_out_headless(state.tree(), &StudioRequest::ProposeObjective(expected));
    assert!(headless.unwrap_err().contains("nothing started"));
}

/// C-55: an element without a name (here a connection) is changed or
/// deleted by the name the tools show it under.
#[test]
fn an_unnamed_connection_is_deleted_by_its_shown_name() {
    let mut state = screening();
    let shown = "UrlShortener::UrlShortenerService::(connect api.storage to store.links)";
    assert!(
        written(&state).contains("connect api.storage to store.links;"),
        "the fixture has the connection"
    );
    apply(
        &mut state,
        json!({ "description": "Remove the storage connection", "operations": [
            { "op": "delete", "element": shown }
        ]}),
    );
    assert!(!written(&state).contains("connect api.storage to store.links;"));
    let gone = prepare(
        &state,
        APPLY_CHANGES,
        json!({ "description": "x", "operations": [{ "op": "delete", "element": shown }] }),
    );
    assert!(
        matches!(gone, Prepared::Invalid(ref m) if m.contains("there is no element")),
        "{gone:?}"
    );
}

/// C-55: an existing transition gets a new effect of several steps (its old
/// effect deleted, a composite action created in its place), so a
/// transition keeps its identity when its behaviour is generalised.
#[test]
fn a_transition_keeps_its_identity_when_its_effect_is_replaced_by_several_steps() {
    let mut state = SystemState::new(
        parse(&[Source::new(
            "Jobs.sysml",
            "package Jobs {
    item def Job { attribute size : ScalarValues::Integer; }
    item def Ack { attribute size : ScalarValues::Integer; }
    port def JobPort { in item job : Job; out item ack : Ack; }
    part def Worker {
        port jobs : JobPort;
        attribute last : ScalarValues::Integer = 0;
        exhibit state working {
            entry;
            then idle;
            state idle;
            transition acknowledge first idle accept job : Job via jobs do send new Ack(size = job.size) via jobs then idle;
        }
    }
}",
        )]),
        BTreeSet::new(),
    );
    let transition = state
        .tree()
        .find("Jobs::Worker::working::acknowledge")
        .unwrap();
    apply(
        &mut state,
        json!({ "description": "Remember the last size before acknowledging", "operations": [
            { "op": "delete", "element": "Jobs::Worker::working::acknowledge::(send)" },
            { "op": "create", "parent": "Jobs::Worker::working::acknowledge", "kind": "action", "steps": [
                { "assign": "last", "value": "job.size" },
                { "send": "new Ack(size = last)", "via": "jobs" }
            ] }
        ]}),
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    assert_eq!(
        state.tree().find("Jobs::Worker::working::acknowledge"),
        Some(transition)
    );
    let text_now = written(&state);
    assert!(
        text_now.contains("assign last := job.size;")
            && text_now.contains("then send new Ack(size = last) via jobs;"),
        "{text_now}"
    );
}
