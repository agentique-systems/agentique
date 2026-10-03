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
use agq_orchestrator::run::{self, Event, Setup};
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
    let repository = dir.path().join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    git(&repository, &["init", "-q", "-b", "main"]);
    git(&repository, &["config", "user.name", "Agentique test"]);
    git(
        &repository,
        &["config", "user.email", "test@example.invalid"],
    );
    std::fs::write(repository.join("README.md"), "A repository to improve.\n").unwrap();
    git(&repository, &["add", "-A"]);
    git(&repository, &["commit", "-q", "-m", "Start"]);

    let store = Store::new(dir.path().join("objectives"));
    let data = dir.path().join("agent");
    let script = companion();
    let setup = Setup {
        store: store.clone(),
        work: dir.path().join("work"),
        builds: dir.path().join("builds"),
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
    };
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
