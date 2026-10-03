//! A cycle end to end with scripted agents (C-53, W11.5): the driver, the
//! Claude Agent runtime boundary (a stand-in companion plays the lead, the
//! implementer and the reviewer), the record and journal, the worktree and
//! commits, the required checks and command criteria on a clean checkout,
//! the key gate, a repair round, the independent review, and a merge the
//! objective's permissions do not allow, which stops the cycle with the
//! reviewed change waiting on its branch. Needs Node and git (skipped
//! without Node).

use agq_assistant::claude_agent::{ClaudeAgent, Installation, find_node};
use agq_orchestrator::record::{Budgets, Permissions, Phase, State, Store};
use agq_orchestrator::run::{self, Command, Event, Setup};
use agq_providers::Secret;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const KEY: &str = "sk-fake-0123456789abcdef";

fn companion() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/role-companion.mjs")
}

fn git(folder: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(folder)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

#[test]
fn a_cycle_goes_from_proposal_through_repair_and_review_to_a_merge_it_may_not_make() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let setup = setup_with(dir.path(), &store, node);
    let objective = store
        .create(
            "Leave a note of the improvement",
            &repository,
            "main",
            Budgets {
                usd: 1.0,
                cycles: 1,
                attempts: 3,
                hours: 1.0,
            },
            Permissions::default(),
        )
        .unwrap();
    let id = objective.id.clone();
    let handle = run::start(setup, objective);
    let deadline = Instant::now() + Duration::from_secs(240);
    let mut activity = Vec::new();
    while !handle.finished() {
        assert!(
            Instant::now() < deadline,
            "the cycle did not end: {activity:#?}"
        );
        while let Ok(event) = handle.events.try_recv() {
            if let Event::Activity { role, text } = event {
                activity.push(format!("{role}: {text}"));
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let record = store.load(&id).unwrap();
    let cycle = record.cycle().expect("a cycle ran");
    // The lead's proposal, frozen.
    let proposal = cycle.proposal.as_ref().expect("a proposal");
    assert_eq!(proposal.title, "Add an improvement note");
    // The criterion failed on the base (no such test yet).
    assert!(
        cycle.before.iter().all(|o| !o.passed()),
        "{:?}",
        cycle.before
    );
    // Attempt 1 leaked the key: the gate failed it (the checks passed).
    assert_eq!(cycle.attempts.len(), 2, "{activity:#?}");
    let first = &cycle.attempts[0];
    assert!(
        first.checks.iter().all(|c| c.passed()),
        "{:?}",
        first.checks
    );
    assert!(
        first.failures().iter().any(|f| f.contains("key")),
        "{:?}",
        first.failures()
    );
    // The repair passed every check, criterion and gate, and the reviewer
    // approved exactly that commit.
    let second = &cycle.attempts[1];
    assert!(second.failures().is_empty(), "{:?}", second.failures());
    assert!(
        second
            .criteria
            .iter()
            .any(|c| c.name.starts_with("c1") && c.passed())
    );
    let review = cycle.review.as_ref().expect("a review");
    assert_eq!(review.verdict, "approve");
    assert_eq!(Some(&review.commit), second.commit.as_ref());
    // No permission to push: the cycle stops with the change on its branch.
    assert_eq!(cycle.phase, Phase::Failed);
    let blocker = cycle.blocker.as_deref().unwrap_or_default();
    assert!(blocker.contains("does not allow pushing"), "{blocker}");
    assert_eq!(record.state, State::Failed);
    let branch = cycle.branch.clone().expect("a branch");
    let shown = std::process::Command::new("git")
        .args(["show", &format!("{branch}:IMPROVEMENT.md")])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&shown.stdout), "Improved.\n");
    // The roles' sessions are kept, the spend counted, the journal written.
    for role in ["lead", "implementer", "reviewer"] {
        assert!(
            cycle.sessions.contains_key(role),
            "{role}: {:?}",
            cycle.sessions
        );
    }
    assert!(record.spent.tokens > 0);
    let journal = store.journal(&id);
    assert!(journal.iter().any(|line| line.key == "cycle-1/worktree"));
    assert!(
        journal
            .iter()
            .any(|line| line.key == "cycle-1/attempt-2/commit")
    );
    // The main branch is untouched.
    let head = std::process::Command::new("git")
        .args(["log", "--oneline", "main"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&head.stdout).lines().count(), 1);
}

/// A repository to improve, with `files` besides its README.
fn repository_with(dir: &Path, files: &[&str]) -> PathBuf {
    let repository = dir.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    git(&repository, &["init", "-q", "-b", "main"]);
    git(&repository, &["config", "user.name", "Agentique test"]);
    git(
        &repository,
        &["config", "user.email", "test@example.invalid"],
    );
    std::fs::write(repository.join("README.md"), "A repository to improve.\n").unwrap();
    for file in files {
        std::fs::write(repository.join(file), "").unwrap();
    }
    git(&repository, &["add", "-A"]);
    git(&repository, &["commit", "-q", "-m", "Start"]);
    repository
}

fn setup_with(dir: &Path, store: &Store, node: agq_assistant::claude_agent::Node) -> Setup {
    let data = dir.join("agent");
    let script = companion();
    Setup {
        store: store.clone(),
        work: dir.join("work"),
        builds: dir.join("builds"),
        runtime: Box::new(move |_| {
            let mut agent = ClaudeAgent::new(
                node.clone(),
                Installation {
                    root: data.join("runtime"),
                },
                data.clone(),
                Some("deepseek-v4-pro".into()),
                None,
                Secret::new("sk-test-session"),
            );
            agent.script = Some(script.clone());
            Ok(agent)
        }),
        keys: vec![KEY.into()],
        checks: vec![vec!["git".into(), "status".into(), "--short".into()]],
        protected: Vec::new(),
        running_build: None,
    }
}

#[test]
fn an_interrupted_objective_continues_from_the_phase_it_reached() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["HOLD-ONCE"]);
    let store = Store::new(dir.path().join("objectives"));
    let objective = store
        .create(
            "Leave a note of the improvement",
            &repository,
            "main",
            Budgets {
                usd: 1.0,
                cycles: 1,
                attempts: 3,
                hours: 1.0,
            },
            Permissions::default(),
        )
        .unwrap();
    let id = objective.id.clone();
    // The implementer works; Agentique closes and interrupts it.
    let handle = run::start(setup_with(dir.path(), &store, node.clone()), objective);
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut working = false;
    while !working {
        assert!(Instant::now() < deadline, "the implementer never started");
        while let Ok(event) = handle.events.try_recv() {
            if let Event::Activity { role, text } = event {
                working |= role == "implementer" && text == "starts";
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(500));
    handle.send(Command::Interrupt);
    while !handle.finished() {
        assert!(
            Instant::now() < deadline,
            "the interruption did not end the driver"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let interrupted = store.load(&id).unwrap();
    assert_eq!(interrupted.state, State::Running, "still to be continued");
    assert!(interrupted.active());
    let cycle = interrupted.cycle().unwrap();
    assert_eq!(cycle.phase, Phase::Implement);
    assert!(
        cycle.attempts.is_empty(),
        "no attempt is recorded for the interrupted work"
    );
    assert!(
        interrupted
            .note
            .as_deref()
            .unwrap_or_default()
            .contains("Interrupted")
    );
    // Continued (Continue in the panel): it goes on from Implement, with
    // the worktree it had, to the end.
    let handle = run::start(setup_with(dir.path(), &store, node), interrupted);
    while !handle.finished() {
        assert!(
            Instant::now() < deadline + Duration::from_secs(120),
            "the continued cycle did not end"
        );
        while handle.events.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(50));
    }
    let record = store.load(&id).unwrap();
    let cycle = record.cycle().unwrap();
    assert_eq!(record.cycles.len(), 1, "the same cycle went on");
    assert!(
        cycle
            .review
            .as_ref()
            .is_some_and(|r| r.verdict == "approve"),
        "{:?}",
        cycle
    );
    assert!(
        cycle
            .blocker
            .as_deref()
            .unwrap_or_default()
            .contains("does not allow pushing")
    );
    let worktrees = store
        .journal(&id)
        .iter()
        .filter(|l| l.key == "cycle-1/worktree" && l.state == "completed")
        .count();
    assert_eq!(worktrees, 1, "the worktree was made once");
}
