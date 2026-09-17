use agq_application::*;
use agq_model::*;
use agq_simulation::{Scenario, Status};
use agq_storage::SqliteStore;
use agq_workspace::Edit;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "models/AgentiqueBehaviour.sysml".into(),
            include_str!("../../../models/AgentiqueBehaviour.sysml").into(),
        ),
        (
            "models/AgentiqueArchitecture.sysml".into(),
            include_str!("../../../models/AgentiqueArchitecture.sysml").into(),
        ),
    ])
}
fn app(path: &std::path::Path) -> Application {
    Application::open(Box::new(SqliteStore::open(path).unwrap()), sources()).unwrap()
}
fn command(a: &Application, payload: Operation) -> Command {
    Command {
        command_id: new_id(),
        project_id: a.project_id.clone(),
        base_revision_id: a.head.id.clone(),
        payload,
    }
}
fn act(a: &mut Application, action: Action) -> Value {
    let c = command(
        a,
        Operation::Act {
            action,
            approval_id: None,
        },
    );
    a.command(Actor::Operator, c).unwrap()
}
fn scenario() -> Scenario {
    serde_json::from_str(include_str!("../../../scenarios/accepted.json")).unwrap()
}
fn prepare(a: &mut Application) -> String {
    let s = act(
        a,
        Action::SaveScenario {
            scenario: scenario(),
        },
    );
    let r = act(
        a,
        Action::PrepareRun {
            scenario_revision_id: s["id"].as_str().unwrap().into(),
        },
    );
    r["prepared_run_id"].as_str().unwrap().into()
}
fn control(a: &mut Application, run: &str, op: &str) -> Value {
    act(
        a,
        Action::ControlRun {
            run_id: run.into(),
            expected_control_version: a.run(run).unwrap().control_version,
            operation: op.into(),
        },
    )
}
#[test]
fn at_nfr01_durable_restart_interrupt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.db");
    let mut a = app(&path);
    let revision = a.head.id.clone();
    let run = prepare(&mut a);
    control(&mut a, &run, "initialise");
    control(&mut a, &run, "step");
    let before = a.run(&run).unwrap();
    drop(a);
    let b = app(&path);
    let after = b.run(&run).unwrap();
    assert_eq!(b.head.id, revision);
    assert_eq!(before.active, after.active);
    assert_eq!(before.next_input, after.next_input);
    assert_eq!(before.trace, after.trace);
    assert_eq!(after.status, Status::Interrupted);
    assert_eq!(after.stop_reason.as_deref(), Some("interrupted"));
}
#[test]
fn completed_runs_survive_restart_and_rename() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("w.db");
    let mut a = app(&path);
    let run = prepare(&mut a);
    control(&mut a, &run, "initialise");
    control(&mut a, &run, "run");
    while a.tick(&run).unwrap() {}
    let before = a.run(&run).unwrap();
    let id = a
        .head
        .model
        .by_path("AgentiqueBehaviour::RequestLifecycle::accepted")
        .unwrap()
        .id
        .clone();
    let c = command(
        &a,
        Operation::ProposeChange {
            edits: vec![Edit::Rename {
                element_id: id.clone(),
                name: "validated".into(),
            }],
        },
    );
    let p = a.command(Actor::Operator, c).unwrap();
    act(
        &mut a,
        Action::CommitChange {
            proposal_id: p["proposal_id"].as_str().unwrap().into(),
        },
    );
    let head = a.head.id.clone();
    drop(a);
    let b = app(&path);
    assert_eq!(b.head.id, head);
    assert_eq!(
        b.head
            .model
            .by_path("AgentiqueBehaviour::RequestLifecycle::validated")
            .unwrap()
            .id,
        id
    );
    let after = b.run(&run).unwrap();
    assert_eq!(after.status, Status::Completed);
    assert_eq!(before.trace, after.trace);
    assert_ne!(after.plan.model_revision_id, head);
}
#[test]
fn at_ai01_approval_actor_payload_expiry_stale() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = app(&dir.path().join("w.db"));
    let id = a
        .head
        .model
        .by_path("AgentiqueArchitecture::Agentique")
        .unwrap()
        .id
        .clone();
    let c = command(
        &a,
        Operation::ProposeChange {
            edits: vec![Edit::Rename {
                element_id: id,
                name: "Platform".into(),
            }],
        },
    );
    let p = a.command(Actor::Assistant, c).unwrap();
    let action = Action::CommitChange {
        proposal_id: p["proposal_id"].as_str().unwrap().into(),
    };
    let c = command(
        &a,
        Operation::Act {
            action: action.clone(),
            approval_id: None,
        },
    );
    assert_eq!(
        a.command(Actor::Assistant, c).unwrap_err().code,
        "approval_required"
    );
    let c = command(
        &a,
        Operation::RequestApproval {
            action: action.clone(),
        },
    );
    let review = a.command(Actor::Assistant, c).unwrap();
    let c = command(
        &a,
        Operation::Approve {
            request_id: review["id"].as_str().unwrap().into(),
            payload_digest: "wrong".into(),
            expires_in_seconds: 300,
        },
    );
    assert_eq!(
        a.command(Actor::Operator, c).unwrap_err().code,
        "approval_mismatch"
    );
    let c = command(
        &a,
        Operation::Approve {
            request_id: review["id"].as_str().unwrap().into(),
            payload_digest: review["payload_digest"].as_str().unwrap().into(),
            expires_in_seconds: 300,
        },
    );
    assert_eq!(
        a.command(Actor::Assistant, c.clone()).unwrap_err().code,
        "permission_denied"
    );
    let approval = a.command(Actor::Operator, c).unwrap();
    let c = command(
        &a,
        Operation::Act {
            action: Action::SaveScenario {
                scenario: scenario(),
            },
            approval_id: Some(approval["id"].as_str().unwrap().into()),
        },
    );
    assert_eq!(
        a.command(Actor::Assistant, c).unwrap_err().code,
        "approval_mismatch"
    );
    let c = command(
        &a,
        Operation::Act {
            action,
            approval_id: Some(approval["id"].as_str().unwrap().into()),
        },
    );
    let stale = c.clone();
    a.command(Actor::Assistant, c).unwrap();
    let mut stale = stale;
    stale.command_id = new_id();
    assert_eq!(
        a.command(Actor::Assistant, stale).unwrap_err().code,
        "revision_conflict"
    );
}
#[test]
fn idempotency_and_command_payload_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = app(&dir.path().join("w.db"));
    let c = command(
        &a,
        Operation::Act {
            action: Action::SaveScenario {
                scenario: scenario(),
            },
            approval_id: None,
        },
    );
    let first = a.command(Actor::Operator, c.clone()).unwrap();
    assert_eq!(a.command(Actor::Operator, c.clone()).unwrap(), first);
    let mut altered = c;
    altered.base_revision_id = "altered".into();
    assert_eq!(
        a.command(Actor::Operator, altered).unwrap_err().code,
        "idempotency_conflict"
    );
    assert_eq!(a.store.list("scenario").unwrap().len(), 1);
}
#[test]
fn conflicting_control_version_and_reset() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = app(&dir.path().join("w.db"));
    let run = prepare(&mut a);
    control(&mut a, &run, "initialise");
    let c = command(
        &a,
        Operation::Act {
            action: Action::ControlRun {
                run_id: run.clone(),
                expected_control_version: 0,
                operation: "step".into(),
            },
            approval_id: None,
        },
    );
    assert_eq!(
        a.command(Actor::Operator, c).unwrap_err().code,
        "control_conflict"
    );
    let reset = control(&mut a, &run, "reset_as_new_run");
    assert_ne!(reset["run_id"], run);
    assert_eq!(a.run(&run).unwrap().status, Status::Paused);
}
#[test]
fn invalid_draft_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("w.db");
    let mut a = app(&path);
    let rev = a.head.id.clone();
    let c = command(
        &a,
        Operation::SaveDraft {
            file: "models/AgentiqueBehaviour.sysml".into(),
            source: "package Broken {".into(),
        },
    );
    let draft = a.command(Actor::Operator, c).unwrap();
    assert!(!draft["diagnostics"].as_array().unwrap().is_empty());
    drop(a);
    let b = app(&path);
    assert_eq!(b.head.id, rev);
    assert_eq!(
        b.store.list("draft").unwrap()[0]["source"],
        "package Broken {"
    );
}
#[test]
fn corruption_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("w.db");
    drop(app(&path));
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE objects SET data='{}' WHERE kind='revision'", [])
        .unwrap();
    drop(c);
    assert!(
        matches!(Application::open(Box::new(SqliteStore::open(&path).unwrap()),sources()),Err(e) if e.code=="corrupt_data")
    );
}
#[test]
fn single_workspace_writer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("w.db");
    let a = app(&path);
    assert!(SqliteStore::open(&path).is_err());
    drop(a);
    assert!(SqliteStore::open(&path).is_ok());
}
#[test]
fn approval_expiry_and_single_use() {
    let mut a = Application::open(Box::<MemoryStore>::default(), sources()).unwrap();
    let action = Action::SaveScenario {
        scenario: scenario(),
    };
    let c = command(
        &a,
        Operation::RequestApproval {
            action: action.clone(),
        },
    );
    let review = a.command(Actor::Assistant, c).unwrap();
    let c = command(
        &a,
        Operation::Approve {
            request_id: review["id"].as_str().unwrap().into(),
            payload_digest: review["payload_digest"].as_str().unwrap().into(),
            expires_in_seconds: 300,
        },
    );
    let approval = a.command(Actor::Operator, c).unwrap();
    let id = approval["id"].as_str().unwrap().to_string();
    let mut expired: Approval = serde_json::from_value(approval.clone()).unwrap();
    expired.expires_at = 0;
    let event = Event {
        sequence: 0,
        actor_id: "test-clock".into(),
        command_id: new_id(),
        project_id: a.project_id.clone(),
        payload_digest: "test-only".into(),
        kind: "advance_expiry_fixture".into(),
        result: json!({}),
        time: a.head.created.clone(),
    };
    a.store
        .transact(Transaction {
            writes: vec![Write::new("approval", &id, &expired)],
            event: event.clone(),
            receipt: None,
        })
        .unwrap();
    let c = command(
        &a,
        Operation::Act {
            action: action.clone(),
            approval_id: Some(id.clone()),
        },
    );
    assert_eq!(
        a.command(Actor::Assistant, c).unwrap_err().code,
        "approval_mismatch"
    );
    a.store
        .transact(Transaction {
            writes: vec![Write::new("approval", &id, &approval)],
            event,
            receipt: None,
        })
        .unwrap();
    let c = command(
        &a,
        Operation::Act {
            action: action.clone(),
            approval_id: Some(id.clone()),
        },
    );
    a.command(Actor::Assistant, c).unwrap();
    let c = command(
        &a,
        Operation::Act {
            action,
            approval_id: Some(id),
        },
    );
    assert_eq!(
        a.command(Actor::Assistant, c).unwrap_err().code,
        "approval_mismatch"
    );
    assert_eq!(a.store.list("scenario").unwrap().len(), 1);
}
#[test]
fn newer_storage_schema_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch("PRAGMA user_version=999").unwrap();
    drop(c);
    assert!(matches!(SqliteStore::open(&path),Err(e) if e.code=="unsupported_storage_version"));
}
struct FailingStore {
    inner: MemoryStore,
    fail: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Store for FailingStore {
    fn get(&self, k: &str, id: &str) -> Result<Option<Value>> {
        self.inner.get(k, id)
    }
    fn list(&self, k: &str) -> Result<Vec<Value>> {
        self.inner.list(k)
    }
    fn events(&self, a: u64, l: usize) -> Result<Vec<Event>> {
        self.inner.events(a, l)
    }
    fn receipt(&self, k: &str) -> Result<Option<(String, Value)>> {
        self.inner.receipt(k)
    }
    fn transact(&mut self, t: Transaction) -> Result<u64> {
        if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
            Err(Error::new(
                "storage_error",
                "injected durable-commit failure",
            ))
        } else {
            self.inner.transact(t)
        }
    }
}
#[test]
fn failed_durable_commit_not_acknowledged() {
    let fail = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut a = Application::open(
        Box::new(FailingStore {
            inner: MemoryStore::default(),
            fail: fail.clone(),
        }),
        sources(),
    )
    .unwrap();
    let run = prepare(&mut a);
    control(&mut a, &run, "initialise");
    let before = json!(a.run(&run).unwrap());
    fail.store(true, std::sync::atomic::Ordering::SeqCst);
    let c = command(
        &a,
        Operation::Act {
            action: Action::ControlRun {
                run_id: run.clone(),
                expected_control_version: 1,
                operation: "step".into(),
            },
            approval_id: None,
        },
    );
    assert_eq!(
        a.command(Actor::Operator, c).unwrap_err().code,
        "storage_error"
    );
    assert_eq!(json!(a.run(&run).unwrap()), before);
}
