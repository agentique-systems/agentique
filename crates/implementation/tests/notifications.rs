//! The implementation runner and the supported checks against real code:
//! the notification dispatcher (a fixture repository with its harness), not
//! the URL shortener, so nothing here is tied to one example. The fixture is
//! copied into a temporary git repository, built with Cargo offline and
//! driven through the harness protocol.
use agq_execution::{Executor, Scope, git};
use agq_implementation::checks::{contract_shapes, linked_tests, module_boundaries};
use agq_implementation::{LinkKind, Links, drift, run_implementation};
use agq_language::{Source, Tree, parse};
use agq_simulation::digest::model_digest;
use agq_simulation::{Mode, Request, RunStatus, StopReason, Verdict, compile};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const NOTIFICATIONS: &str = include_str!("../../../models/notifications/Notifications.sysml");

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/notifications")
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" || name == "Cargo.lock" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(name));
        } else {
            std::fs::copy(&path, to.join(name)).unwrap();
        }
    }
}

struct Repo {
    _dir: tempfile::TempDir,
    path: PathBuf,
    tree: Tree,
    links: Links,
}

impl Repo {
    fn new() -> Repo {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notifications");
        copy(&fixture(), &path);
        git::init_and_commit(&path, "The notification service").unwrap();
        let tree = parse(&[Source::new("Notifications.sysml", NOTIFICATIONS)]);
        let mut links = Links {
            harness: [
                "cargo",
                "run",
                "--quiet",
                "--offline",
                "--bin",
                "agentique-harness",
            ]
            .map(String::from)
            .to_vec(),
            protected: vec!["tests/contract.rs".into()],
            ..Links::default()
        };
        let id = |name: &str| tree.find(&format!("Notifications::{name}")).unwrap();
        links.add(
            &tree,
            id("Dispatcher"),
            LinkKind::Module,
            "src/dispatcher.rs",
            None,
        );
        links.add(
            &tree,
            id("SmsGateway"),
            LinkKind::Module,
            "src/gateway.rs",
            None,
        );
        for (item, symbol) in [
            ("Notification", "Notification"),
            ("SendRequest", "SendRequest"),
            ("SendResult", "SendResult"),
            ("Receipt", "Receipt"),
            ("DeliveryStatus", "DeliveryStatus"),
        ] {
            links.add(
                &tree,
                id(item),
                LinkKind::Type,
                "src/model.rs",
                Some(symbol),
            );
        }
        links.add(
            &tree,
            id("Dispatcher"),
            LinkKind::Test,
            "tests/contract.rs",
            Some("receipts_count_attempts"),
        );
        links.add(
            &tree,
            id("Dispatcher"),
            LinkKind::Test,
            "tests/contract.rs",
            Some("a_gateway_that_keeps_failing_ends_as_failed"),
        );
        Repo {
            _dir: dir,
            path,
            tree,
            links,
        }
    }

    /// Each working copy builds in its own target folder: two copies of one
    /// crate sharing a target folder can reuse each other's stale builds.
    fn executor(&self) -> Executor {
        Executor::new(Scope::read_only(&self.path).unwrap())
            .trusted(true)
            .target_dir(self.path.parent().unwrap().join("target"))
    }

    fn read(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |path| std::fs::read_to_string(self.path.join(path)).ok()
    }

    fn edit(&self, file: &str, from: &str, to: &str) {
        let path = self.path.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "{from:?} not in {file}");
        std::fs::write(&path, text.replace(from, to)).unwrap();
    }

    fn run(&self, scenario: &str) -> agq_simulation::RunResult {
        let id = self
            .tree
            .find(&format!("Notifications::{scenario}"))
            .unwrap();
        let program = compile(&self.tree, id).unwrap();
        run_implementation(
            &program,
            model_digest(&self.tree, id),
            &Request::new(Mode::Implementation),
            &self.links,
            &self.path,
            &self.executor(),
            Arc::new(AtomicBool::new(false)),
        )
    }
}

fn verdicts(result: &agq_simulation::RunResult) -> Vec<(&str, Verdict)> {
    result
        .checks
        .iter()
        .map(|c| (c.name.as_str(), c.verdict))
        .collect()
}

#[test]
fn the_same_scenarios_run_against_the_real_dispatcher() {
    let repo = Repo::new();
    let result = repo.run("RetryThenDeliver");
    assert_eq!(result.mode, Mode::Implementation);
    assert_eq!(
        result.status,
        RunStatus::Completed,
        "{:#?}\n{:#?}",
        result.stop,
        result.trace
    );
    assert_eq!(
        verdicts(&result),
        [
            ("deliveredOnThirdAttempt", Verdict::Passed),
            ("no unexpected output", Verdict::Passed)
        ]
    );
    let provenance = result.provenance.implementation.clone().unwrap();
    assert_eq!(provenance.commit, git::head(&repo.path).unwrap().commit);
    assert!(!provenance.dirty);
    assert!(provenance.harness.contains("agentique-harness"));
    // A check that reads internal state is not evaluated here, and says so.
    let result = repo.run("GiveUpAfterThreeAttempts");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(
        verdicts(&result),
        [
            ("endsAsFailed", Verdict::Passed),
            ("dispatcherIsIdle", Verdict::Unsupported),
            ("no unexpected output", Verdict::Passed)
        ]
    );
    assert!(!result.all_passed(), "an unsupported check is not a pass");
}

#[test]
fn a_broken_implementation_is_caught_by_its_scenario_and_its_tests() {
    let repo = Repo::new();
    // The deliberate implementation break: one retry too many.
    repo.edit(
        "src/dispatcher.rs",
        "if attempts >= self.max_attempts",
        "if attempts > self.max_attempts",
    );
    let result = repo.run("GiveUpAfterThreeAttempts");
    assert_eq!(result.checks[0].name, "endsAsFailed");
    assert_eq!(result.checks[0].verdict, Verdict::Failed);
    assert!(
        result.checks[0].message.contains("receipt.attempts = 4"),
        "{}",
        result.checks[0].message
    );
    assert!(result.provenance.implementation.unwrap().dirty);
    let tests = linked_tests(&repo.links, &repo.executor(), "", Duration::from_secs(600));
    let failed: Vec<&str> = tests
        .iter()
        .filter(|t| t.verdict == Verdict::Failed)
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(
        failed,
        ["Test a_gateway_that_keeps_failing_ends_as_failed"],
        "{tests:#?}"
    );
    let drift = drift(&tests);
    let dispatcher = repo.tree.find("Notifications::Dispatcher").unwrap();
    assert_eq!(drift[&dispatcher].len(), 1);
}

#[test]
fn contract_shapes_and_module_boundaries_hold_and_break() {
    let repo = Repo::new();
    let shapes = contract_shapes(&repo.tree, &repo.links, &repo.read());
    assert_eq!(shapes.len(), 5);
    assert!(
        shapes.iter().all(|c| c.verdict == Verdict::Passed),
        "{shapes:#?}"
    );
    let boundaries = module_boundaries(&repo.tree, &repo.links, &repo.read());
    assert_eq!(boundaries.verdict, Verdict::Passed, "{boundaries:#?}");
    // Break both: a renamed field, a new value, and the gateway reaching back.
    repo.edit("src/model.rs", "pub attempts: u32,", "pub tries: u32,");
    repo.edit(
        "src/model.rs",
        "    Failed,\n}",
        "    Failed,\n    Queued,\n}",
    );
    repo.edit(
        "src/gateway.rs",
        "use crate::model::",
        "use crate::dispatcher::Clock;\nuse crate::model::",
    );
    let shapes = contract_shapes(&repo.tree, &repo.links, &repo.read());
    let failed: Vec<(&str, &Vec<String>)> = shapes
        .iter()
        .filter(|c| c.verdict == Verdict::Failed)
        .map(|c| (c.name.as_str(), &c.details))
        .collect();
    assert_eq!(failed.len(), 2, "{failed:#?}");
    assert!(
        failed[0]
            .1
            .iter()
            .chain(failed[1].1)
            .any(|d| d.contains("has no field `attempts`"))
    );
    assert!(
        failed[0]
            .1
            .iter()
            .chain(failed[1].1)
            .any(|d| d.contains("`Queued` is not a value"))
    );
    let boundaries = module_boundaries(&repo.tree, &repo.links, &repo.read());
    assert_eq!(boundaries.verdict, Verdict::Failed);
    assert!(
        boundaries.details[0]
            .contains("`src/gateway.rs` (Notifications::SmsGateway) uses `crate::dispatcher`"),
        "{:#?}",
        boundaries.details
    );
    let gateway = repo.tree.find("Notifications::SmsGateway").unwrap();
    assert_eq!(boundaries.elements, [gateway.raw()]);
}

#[test]
fn without_a_harness_or_trust_nothing_runs_and_the_result_says_why() {
    let repo = Repo::new();
    let id = repo.tree.find("Notifications::RetryThenDeliver").unwrap();
    let program = compile(&repo.tree, id).unwrap();
    let mut links = repo.links.clone();
    links.harness.clear();
    let result = run_implementation(
        &program,
        model_digest(&repo.tree, id),
        &Request::new(Mode::Implementation),
        &links,
        &repo.path,
        &repo.executor(),
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(result.status, RunStatus::Blocked);
    assert!(result.blockers[0].1.contains("no harness is linked"));
    let untrusted = Executor::new(Scope::read_only(&repo.path).unwrap());
    let result = run_implementation(
        &program,
        model_digest(&repo.tree, id),
        &Request::new(Mode::Implementation),
        &repo.links,
        &repo.path,
        &untrusted,
        Arc::new(AtomicBool::new(false)),
    );
    let stop = result.stop.unwrap();
    assert_eq!(stop.reason, StopReason::HarnessFailed);
    assert!(stop.message.contains("trusted-local"), "{}", stop.message);
}
