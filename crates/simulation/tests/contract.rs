use agq_model::*;
use agq_simulation::*;
use std::collections::BTreeMap;
fn model() -> Model {
    agq_semantics::compile(
        BTreeMap::from([
            (
                "models/AgentiqueBehaviour.sysml".into(),
                include_str!("../../../models/AgentiqueBehaviour.sysml").into(),
            ),
            (
                "models/AgentiqueArchitecture.sysml".into(),
                include_str!("../../../models/AgentiqueArchitecture.sysml").into(),
            ),
        ]),
        &BTreeMap::new(),
    )
}
fn accepted() -> Scenario {
    serde_json::from_str(include_str!("../../../scenarios/accepted.json")).unwrap()
}
fn make(m: &Model, s: &Scenario) -> Run {
    Run::new(prepare(m, "model-rev", "scenario-rev", s, "test-build").unwrap())
}
#[test]
fn shorthand_succession_after_state_is_not_an_initial_transition() {
    let mut sources = model().sources;
    let text = sources.get_mut("models/AgentiqueBehaviour.sysml").unwrap();
    *text = text.replace("entry;", "entry; state misleading;");
    let m = agq_semantics::compile(sources, &BTreeMap::new());
    let err = prepare(&m, "r", "s", &accepted(), "build").unwrap_err();
    assert_eq!(err.code, "unsupported_feature");
    assert!(err.message.contains("immediately"));
}
#[test]
fn at_sim01_resolved_manifest() {
    let m = model();
    let r = make(&m, &accepted());
    assert_eq!(r.plan.selected.len(), 5);
    assert!(
        r.plan
            .inputs
            .iter()
            .all(|i| m.elements.contains_key(&i.receiver)
                && m.elements.contains_key(&i.payload_type))
    );
    assert!(!r.plan.library_digest.is_empty());
    assert!(!r.plan.plan_digest.is_empty());
}
#[test]
fn at_sim02_isolation() {
    let m = model();
    let before = m.source_digest();
    let mut a = make(&m, &accepted());
    let b = make(&m, &accepted());
    a.initialise().unwrap();
    a.step().unwrap();
    assert_eq!(b.active, None);
    assert_eq!(b.next_input, 0);
    assert_ne!(a.occurrence_id, b.occurrence_id);
    assert_eq!(m.source_digest(), before);
}
#[test]
fn at_sim03_sim04_manual_continuous_repeat() {
    let m = model();
    let mut a = make(&m, &accepted());
    a.initialise().unwrap();
    while !a.status.terminal() {
        a.step().unwrap()
    }
    let mut b = make(&m, &accepted());
    b.execute().unwrap();
    let mut c = make(&m, &accepted());
    c.execute().unwrap();
    assert_eq!(a.trace_digest(), b.trace_digest());
    assert_eq!(b.trace_digest(), c.trace_digest());
    assert_eq!(a.stop_reason.as_deref(), Some("condition_met"));
    assert_eq!(a.plan.states[a.active.as_ref().unwrap()], "accepted");
}
#[test]
fn at_sim05_completion_is_not_verdict() {
    let m = model();
    let mut s: Scenario =
        serde_json::from_str(include_str!("../../../scenarios/rejected.json")).unwrap();
    let mut r = make(&m, &s);
    r.execute().unwrap();
    assert_eq!(r.plan.states[r.active.as_ref().unwrap()], "rejected");
    assert_eq!(r.status, Status::Completed);
    assert_eq!(r.checks[0].status, "not_run");
    s.inputs.clear();
    let mut exhausted = make(&m, &s);
    exhausted.execute().unwrap();
    assert_eq!(exhausted.stop_reason.as_deref(), Some("input_exhausted"));
    assert_eq!(exhausted.checks[1].status, "fail");
}
#[test]
fn at_sim06_source_trace() {
    let m = model();
    let mut r = make(&m, &accepted());
    r.execute().unwrap();
    assert_eq!(r.trace.len(), 5);
    for t in r.trace.iter() {
        assert!(m.elements.contains_key(&t.source_element_id));
        assert_eq!(t.model_revision_id, "model-rev");
        assert_eq!(t.run_id, r.id);
    }
}
#[test]
fn ambiguous_step_does_not_consume() {
    let mut sources = model().sources;
    sources
        .get_mut("models/AgentiqueBehaviour.sysml")
        .unwrap()
        .insert_str(0, "// ambiguity fixture\n");
    let text = sources.get_mut("models/AgentiqueBehaviour.sysml").unwrap();
    *text=text.replace("transition beginCheck","transition duplicate first idle accept CheckRequested via input then rejected;\ntransition beginCheck");
    let m = agq_semantics::compile(sources, &BTreeMap::new());
    let mut r = make(&m, &accepted());
    r.initialise().unwrap();
    let before = r.active.clone();
    r.step().unwrap();
    assert_eq!(r.status, Status::Blocked);
    assert_eq!(r.stop_reason.as_deref(), Some("ambiguous_transition"));
    assert_eq!(r.active, before);
    assert_eq!(r.next_input, 0);
    assert_eq!(r.trace.len(), 1);
}
#[test]
fn unhandled_input_is_atomic() {
    let m = model();
    let mut s = accepted();
    s.inputs[0].payload_type = "AgentiqueBehaviour::ValidationFailed".into();
    let mut r = make(&m, &s);
    r.initialise().unwrap();
    r.step().unwrap();
    assert_eq!(r.stop_reason.as_deref(), Some("unhandled_input"));
    assert_eq!(r.next_input, 0);
    assert_eq!(r.trace.len(), 1);
}
#[test]
fn resource_limits_explicit() {
    let m = model();
    for (which, expected) in [(0, "step_limit"), (1, "trace_limit"), (2, "memory_limit")] {
        let mut s = accepted();
        if which == 0 {
            s.limits.max_semantic_steps = 1
        } else if which == 1 {
            s.limits.max_trace_records = 1
        } else {
            let plan = prepare(&m, "model-rev", "scenario-rev", &s, "test-build").unwrap();
            s.limits.max_memory_bytes = plan.memory_base_bytes + 4096;
        }
        let mut r = make(&m, &s);
        r.initialise().unwrap();
        r.step().unwrap();
        assert_eq!(r.stop_reason.as_deref(), Some(expected));
        assert_eq!(r.next_input, 0);
    }
    let mut s = accepted();
    s.limits.max_input_deliveries = 1;
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "resource_limit"
    );
    let mut s = accepted();
    s.limits.max_memory_bytes = 1;
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "resource_limit"
    );
}
#[test]
fn input_order_and_receiver_identity() {
    let m = model();
    let mut s = accepted();
    s.inputs[1].ordinal = 1;
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "invalid_binding"
    );
}
#[test]
fn at_std02_parallel_preserved() {
    let source = include_str!("../../../tests/fixtures/Parallel.sysml");
    let m = agq_semantics::compile(
        BTreeMap::from([("parallel.sysml".into(), source.into())]),
        &BTreeMap::new(),
    );
    assert!(m.accepted());
    assert_eq!(m.sources["parallel.sysml"], source);
    let mut s = accepted();
    s.selected_behaviour_path = "ParallelFixture::Health".into();
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "unsupported_feature"
    );
}
#[test]
fn at_sec01_live_action_rejected() {
    let mut sources = model().sources;
    let source = sources.get_mut("models/AgentiqueBehaviour.sysml").unwrap();
    *source = source.replace(
        "state checking;",
        "state checking { do action externalCall; }",
    );
    let m = agq_semantics::compile(sources, &BTreeMap::new());
    assert_eq!(
        prepare(&m, "r", "s", &accepted(), "b").unwrap_err().code,
        "unsupported_feature"
    );
}
fn controller() -> (Model, Scenario) {
    let m = agq_semantics::compile(
        BTreeMap::from([(
            "controller.sysml".into(),
            include_str!("../../../tests/fixtures/Controller.sysml").into(),
        )]),
        &BTreeMap::new(),
    );
    assert!(m.accepted(), "{:?}", m.diagnostics);
    let s = Scenario {
        execution_contract: CONTRACT.into(),
        selected_behaviour_path: "Controller::firstDevice.mode".into(),
        inputs: vec![
            Input {
                ordinal: 1,
                receiver_path: "Controller::firstDevice.mode.commands".into(),
                payload_type: "Controller::Switch".into(),
                values: BTreeMap::new(),
            },
            Input {
                ordinal: 3,
                receiver_path: "Controller::firstDevice.mode.commands".into(),
                payload_type: "Controller::Report".into(),
                values: BTreeMap::from([(
                    "count".into(),
                    Value::Integer("9999999999999999999999999999999999999999".into()),
                )]),
            },
        ],
        stop_when_active_state: Some("Controller::firstDevice.mode.retired".into()),
        bindings: BTreeMap::from([
            ("enabled".into(), Value::Boolean(true)),
            (
                "threshold".into(),
                Value::Integer("9999999999999999999999999999999999999998".into()),
            ),
        ]),
        limits: Limits::default(),
        expected: None,
    };
    (m, s)
}
#[test]
fn independent_model_exact_payload_guard() {
    let (m, s) = controller();
    let mut r = make(&m, &s);
    r.execute().unwrap();
    assert_eq!(r.status, Status::Completed);
    assert_eq!(r.plan.states[r.active.as_ref().unwrap()], "retired");
    assert_eq!(r.trace[4].input_ordinal, Some(3));
}
#[test]
fn missing_and_mismatched_bindings() {
    let (m, mut s) = controller();
    s.bindings.remove("enabled");
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "missing_binding"
    );
    let (m, mut s) = controller();
    s.inputs[1].values.clear();
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "missing_binding"
    );
    let (m, mut s) = controller();
    s.bindings
        .insert("enabled".into(), Value::Integer("1".into()));
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "invalid_binding"
    );
}
#[test]
fn occurrence_receiver_cannot_alias() {
    let (m, mut s) = controller();
    s.inputs[0].receiver_path = "Controller::secondDevice.mode.commands".into();
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "invalid_binding"
    );
}
#[test]
fn unsupported_guard_refused() {
    let (m, s) = controller();
    let mut sources = m.sources;
    sources.get_mut("controller.sysml").unwrap().clone_from(
        &include_str!("../../../tests/fixtures/Controller.sysml")
            .replace("if enabled", "if threshold + 1 > 0"),
    );
    let m = agq_semantics::compile(sources, &BTreeMap::new());
    assert_eq!(
        prepare(&m, "r", "s", &s, "b").unwrap_err().code,
        "unsupported_feature"
    );
}
#[test]
fn control_state_machine() {
    let m = model();
    let mut r = make(&m, &accepted());
    assert!(r.step().is_err());
    r.control("initialise").unwrap();
    r.control("run").unwrap();
    r.control("pause").unwrap();
    r.control("step").unwrap();
    r.control("stop").unwrap();
    let before = json_digest(&r);
    assert!(r.control("initialise").is_err());
    assert_eq!(json_digest(&r), before);
}
