use agq_application::{Actor, Application, Command, MemoryStore, Operation};
use agq_model::WorkControl;
use agq_workspace::Edit;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[test]
fn cancel_during_parsing_preserves_accepted_revision_and_receipts() {
    let mut app = Application::open(
        Box::<MemoryStore>::default(),
        BTreeMap::from([("base.sysml".into(), "package Base;".into())]),
    )
    .unwrap();
    let original = app.head.id.clone();
    let work = WorkControl::default();
    let worker_control = work.clone();
    let command = Command {
        command_id: "cancel-during-parse".into(),
        project_id: app.project_id.clone(),
        base_revision_id: original.clone(),
        payload: Operation::ProposeChange {
            edits: vec![Edit::AddSource {
                file: "large.sysml".into(),
                source: format!(
                    "package Large {{ {} }}",
                    (0..30_000)
                        .map(|i| format!("part def P{i};"))
                        .collect::<String>()
                ),
            }],
        },
    };
    let worker = std::thread::spawn(move || {
        let result = app.command_controlled(Actor::Operator, command, &worker_control);
        (app, result)
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while !["lexing", "parsing"].contains(&work.progress().stage.as_str()) {
        assert!(Instant::now() < deadline, "Parser did not publish progress");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(work.cancel());
    let (app, result) = worker.join().unwrap();
    assert_eq!(result.unwrap_err().code, "cancelled");
    assert_eq!(app.head.id, original);
    assert_eq!(app.store.latest_sequence().unwrap(), 1);
    assert!(app.store.list("proposal").unwrap().is_empty());
    assert!(
        app.store
            .receipt("local-operator:cancel-during-parse")
            .unwrap()
            .is_none()
    );
}

#[test]
fn cancellation_commit_boundary_is_unambiguous() {
    let work = WorkControl::default();
    assert!(work.cancel());
    assert_eq!(work.begin_commit().unwrap_err().code, "cancelled");
    let work = WorkControl::default();
    work.begin_commit().unwrap();
    assert!(!work.cancel());
    assert!(work.check().is_ok());
}

#[test]
fn parser_memory_reservations_reject_oversized_work() {
    let m = agq_semantics::compile(
        BTreeMap::from([("a.sysml".into(), "; ".repeat(200_001))]),
        &BTreeMap::new(),
    );
    assert!(!m.accepted());
    assert!(m.diagnostics.iter().any(|d| d.code == "resource_limit"));
}
