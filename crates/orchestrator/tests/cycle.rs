//! A cycle end to end with scripted agents (C-53, W11.5): the driver, the
//! Claude Agent runtime boundary (a stand-in companion plays the lead, the
//! implementer and the reviewer), the record and journal, the worktree and
//! commits, the required checks and command criteria on a clean checkout,
//! the key gate, a repair round, the independent review, and a merge the
//! objective's permissions do not allow, which stops the cycle with the
//! reviewed change waiting on its branch; each role on its own model and
//! effort, and the spend by role and model (C-54). Needs Node and git
//! (skipped without Node).

use agq_assistant::claude_agent::{ClaudeAgent, Installation, find_node};
use agq_orchestrator::record::{
    Access, Budgets, Objective, Permissions, Phase, RoleModel, State, Store,
};
use agq_orchestrator::record::{DirectiveStatus, Recipient};
use agq_orchestrator::run::{self, Command, Event, Setup};
use agq_orchestrator::thread::{Author, Kind};
use agq_providers::{ModelRef, Provider, Secret};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
    let mut setup = setup_with(dir.path(), &store, node.clone());
    // The factory is given each role's own model (C-54).
    let built = Arc::new(Mutex::new(Vec::<(String, String, Option<String>)>::new()));
    let made = built.clone();
    let factory = setup.runtime;
    setup.runtime = Box::new(move |model, development| {
        made.lock().unwrap().push((
            model.role.clone(),
            model.model.model.clone(),
            model.effort.clone(),
        ));
        factory(model, development)
    });
    let objective = with_models(
        store
            .create(
                "Leave a note of the improvement",
                &repository,
                "main",
                Budgets {
                    usd: Some(1.0),
                    cycles: 1,
                    attempts: 3,
                    hours: Some(1.0),
                    ..Budgets::default()
                },
                Permissions::default(),
            )
            .unwrap(),
        &store,
    );
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
            if let Event::Thread(entry) = event {
                activity.push(format!("{}: {}", entry.author.label(), entry.text));
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
    // Each role ran on its own model and effort, and spent under its role,
    // by model: its own, and its subagent's.
    let built = built.lock().unwrap().clone();
    for (role, model, effort) in [
        ("lead", "deepseek-v4-pro", Some("max")),
        ("implementer", "deepseek-flash", Some("high")),
        ("reviewer", "deepseek-v4-pro", Some("low")),
    ] {
        assert!(
            built
                .iter()
                .any(|(r, m, e)| r == role && m == model && e.as_deref() == effort),
            "{role}: {built:?}"
        );
        let spent = &record.spent.roles[role];
        assert!(
            spent[&format!("deepseek/{model}")].tokens >= 1100,
            "{role}: {spent:?}"
        );
    }
    // Every session of a role is built on its model: the implementer's
    // first attempt and its repair alike.
    let implementer: Vec<_> = built.iter().filter(|(r, ..)| r == "implementer").collect();
    assert!(implementer.len() >= 2, "{built:?}");
    assert!(
        implementer
            .iter()
            .all(|(_, m, e)| m == "deepseek-flash" && e.as_deref() == Some("high")),
        "{built:?}"
    );
    assert!(record.spent.roles["lead"]["deepseek/deepseek-flash"].tokens > 0);
    assert!(
        !record.spent.roles["implementer"].contains_key("deepseek/deepseek-v4-pro"),
        "the implementer's spend is its own"
    );
    let by_role: u64 = record
        .spent
        .roles
        .keys()
        .map(|role| record.spent.role(role).tokens)
        .sum();
    assert_eq!(by_role, record.spent.tokens);
    let journal = store.journal(&id);
    assert!(journal.iter().any(|line| line.key == "cycle-1/worktree"));
    assert!(
        journal
            .iter()
            .any(|line| line.key == "cycle-1/attempt-2/commit")
    );
    // The thread (C-54): the intent first, as the Operator's; each role's
    // model; the lead's proposal as a directive to the implementer, which
    // refers to it; the implementer's submissions and the reviewer's
    // verdict as results; the checks as events; each role's tool calls as
    // its activity, under the directive it worked on; in order.
    let thread = store.thread(&id, 0);
    assert_eq!(
        thread.iter().map(|e| e.seq).collect::<Vec<_>>(),
        (1..=thread.len() as u64).collect::<Vec<_>>()
    );
    assert_eq!(thread[0].kind, Kind::Human);
    assert_eq!(thread[0].author, Author::Operator);
    assert_eq!(thread[0].text, "Leave a note of the improvement");
    assert!(
        thread.iter().any(
            |e| e.kind == Kind::Event && e.text.starts_with("The lead runs on deepseek-v4-pro")
        ),
        "{activity:#?}"
    );
    let directive = &record.directives[0];
    assert_eq!(directive.recipient, Recipient::Role("implementer".into()));
    assert_eq!(directive.refers_to.as_deref(), Some("cycle-1/proposal"));
    assert_eq!(directive.author.role, "lead");
    // The review approved its implementation: the directive is done, though
    // the cycle stopped at the merge it may not make.
    assert_eq!(directive.status, DirectiveStatus::Done);
    let proposed = thread
        .iter()
        .find(|e| e.kind == Kind::Directive)
        .expect("the proposal in the thread");
    assert_eq!(proposed.directive.as_deref(), Some(directive.id.as_str()));
    assert!(
        proposed
            .details
            .as_deref()
            .unwrap_or_default()
            .contains("Acceptance criteria")
    );
    let results: Vec<_> = thread.iter().filter(|e| e.kind == Kind::Result).collect();
    assert_eq!(
        results.len(),
        3,
        "two submissions and a verdict: {activity:#?}"
    );
    assert!(results[2].text.starts_with("Approves"));
    assert!(
        thread.iter().any(|e| e.kind == Kind::Activity
            && e.text == "submit_implementation"
            && e.directive.as_deref() == Some(directive.id.as_str())),
        "{activity:#?}"
    );
    assert!(
        thread
            .iter()
            .any(|e| e.kind == Kind::Event && e.text.contains("failed: "))
    );
    // Each role's activity folds under the start of its session.
    for activity in thread.iter().filter(|e| e.kind == Kind::Activity) {
        let step = thread
            .iter()
            .find(|e| Some(e.seq) == activity.under)
            .expect("its step");
        assert_eq!(step.author, activity.author);
        assert!(step.text.contains("s its session on "), "{}", step.text);
    }
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

/// The objective with each role's model, as the Studio records them before
/// it starts (C-54): different models and efforts, to see each role get its
/// own.
fn with_models(mut objective: Objective, store: &Store) -> Objective {
    let model = |role: &str, id: &str, effort: &str| RoleModel {
        role: role.into(),
        model: ModelRef::new(Provider::DeepSeek, id),
        effort: Some(effort.into()),
        access: Access::Key,
        configured: ModelRef::new(Provider::Anthropic, "claude-opus-5-5"),
        fallback: Some("no Anthropic API key or Claude subscription token".into()),
        credential: "DEEPSEEK_API_KEY".into(),
        billed: "per token, to the DeepSeek account of this key".into(),
    };
    objective.models = vec![
        model("lead", "deepseek-v4-pro", "max"),
        model("implementer", "deepseek-flash", "high"),
        model("reviewer", "deepseek-v4-pro", "low"),
        model("evaluator", "deepseek-flash", "high"),
    ];
    store.save(&objective).unwrap();
    objective
}

fn setup_with(dir: &Path, store: &Store, node: agq_assistant::claude_agent::Node) -> Setup {
    let data = dir.join("agent");
    let script = companion();
    Setup {
        store: store.clone(),
        work: dir.join("work"),
        builds: dir.join("builds"),
        runtime: Box::new(move |model, _| {
            let mut agent = ClaudeAgent::new(
                node.clone(),
                Installation {
                    root: data.join("runtime"),
                },
                data.clone(),
                Some(model.model.model.clone()),
                model.effort.clone(),
                Secret::new("sk-test-session"),
            );
            agent.script = Some(script.clone());
            Ok(agent)
        }),
        keys: vec![KEY.into()],
        checks: vec![vec!["git".into(), "status".into(), "--short".into()]],
        protected: Vec::new(),
        running_build: None,
        speed: "instant".into(),
        credential: Box::new(|_| None),
        studios: Box::new(run::Live),
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
    let objective = with_models(
        store
            .create(
                "Leave a note of the improvement",
                &repository,
                "main",
                Budgets {
                    usd: Some(1.0),
                    cycles: 1,
                    attempts: 3,
                    hours: Some(1.0),
                    ..Budgets::default()
                },
                Permissions::default(),
            )
            .unwrap(),
        &store,
    );
    let id = objective.id.clone();
    // The implementer works; Agentique closes and interrupts it.
    let handle = run::start(setup_with(dir.path(), &store, node.clone()), objective);
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut working = false;
    while !working {
        assert!(Instant::now() < deadline, "the implementer never started");
        while let Ok(event) = handle.events.try_recv() {
            if let Event::Thread(entry) = event {
                working |= matches!(&entry.author, Author::Agent { role, .. } if role == "implementer")
                    && entry.text.starts_with("Starts its session");
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
    // Interrupted when Agentique closed: it waits for Continue (C-54).
    assert!(interrupted.interrupted);
    assert!(matches!(
        interrupted.on_start(true),
        agq_orchestrator::record::Resuming::Wait(_)
    ));
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
    assert!(!record.interrupted, "continued");
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

fn git_out(folder: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(folder)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Commits everything, the model's new elements given their identities
/// first (opening the project does).
fn commit_model(repository: &Path, message: &str) -> String {
    let _ = agq_assistant::model_tools::model_at(repository, "HEAD");
    git(repository, &["add", "-A"]);
    git(repository, &["commit", "-q", "-m", message]);
    git_out(repository, &["rev-parse", "HEAD"])
}

/// A repository with a model (C-55): `Shop` with its purpose requirement
/// (`Purpose`, `purpose`: governed, as Agentique is), the parts `Store` and
/// `Cart`, each linked to its code, and the requirement `Fast`, and a
/// `ROADMAP.md`; the tag `approved-baseline` pushed to its `origin` one
/// commit before its head, which adds the part `Basket`. Returns it and the
/// approved commit.
fn modelled_repository(dir: &Path, markers: &[&str]) -> (PathBuf, String) {
    let repository = repository_with(dir, markers);
    let origin = dir.join("origin.git");
    git(dir, &["init", "-q", "--bare", "origin.git"]);
    git(
        &repository,
        &["remote", "add", "origin", &origin.display().to_string()],
    );
    let shop = "package Shop {\n    requirement def Purpose {\n        doc /* Shops sell. */\n    }\n    requirement purpose : Purpose;\n    part def Store;\n    part def Cart;\n    requirement def Fast;\n}\n";
    std::fs::write(repository.join("ROADMAP.md"), "The shop's direction.\n").unwrap();
    std::fs::create_dir_all(repository.join("model")).unwrap();
    std::fs::create_dir_all(repository.join("src")).unwrap();
    std::fs::write(repository.join("model/Shop.sysml"), shop).unwrap();
    std::fs::write(repository.join("src/store.rs"), "fn store() {}\n").unwrap();
    std::fs::write(repository.join("src/cart.rs"), "fn cart() {}\n").unwrap();
    let approved = commit_model(&repository, "Model");
    git(&repository, &["tag", "approved-baseline"]);
    git(&repository, &["push", "-q", "origin", "approved-baseline"]);
    let model = agq_assistant::model_tools::model_at(&repository, &approved).unwrap();
    let id = |name: &str| model.values().find(|e| e.name == name).unwrap().id;
    std::fs::write(
        repository.join("model/links.json"),
        format!(
            "{{\"format\": 1, \"repository\": \".\", \"links\": [{{\"element\": {}, \"name\": \"Shop::Store\", \"kind\": \"crate\", \"path\": \"src/store.rs\"}}, {{\"element\": {}, \"name\": \"Shop::Cart\", \"kind\": \"crate\", \"path\": \"src/cart.rs\"}}]}}",
            id("Shop::Store"),
            id("Shop::Cart")
        ),
    )
    .unwrap();
    std::fs::write(
        repository.join("model/Shop.sysml"),
        shop.replace("part def Cart;", "part def Cart;\n    part def Basket;"),
    )
    .unwrap();
    commit_model(&repository, "Links and a basket");
    (repository, approved)
}

/// Runs an objective of one cycle on `repository` to its end, and returns
/// its record.
fn run_objective(
    dir: &Path,
    repository: &Path,
    node: agq_assistant::claude_agent::Node,
) -> Objective {
    run_objective_with(dir, repository, node, None)
}

/// The same, with the project's required checks `checks` when given.
fn run_objective_with(
    dir: &Path,
    repository: &Path,
    node: agq_assistant::claude_agent::Node,
    checks: Option<Vec<Vec<String>>>,
) -> Objective {
    let store = Store::new(dir.join("objectives"));
    let objective = with_models(
        store
            .create(
                "Leave a note of the improvement",
                repository,
                "main",
                Budgets {
                    usd: Some(1.0),
                    cycles: 1,
                    attempts: 3,
                    hours: Some(1.0),
                    ..Budgets::default()
                },
                Permissions::default(),
            )
            .unwrap(),
        &store,
    );
    let id = objective.id.clone();
    let mut setup = setup_with(dir, &store, node);
    if let Some(checks) = checks {
        setup.checks = checks;
    }
    let handle = run::start(setup, objective);
    let deadline = Instant::now() + Duration::from_secs(240);
    while !handle.finished() {
        assert!(Instant::now() < deadline, "the objective did not end");
        while handle.events.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(50));
    }
    store.load(&id).unwrap()
}

/// C-55, end to end: the proposal's `serves` and `parts` are resolved in the
/// base commit's model by identity; the review is given, and the cycle
/// records, what the commit changed but the proposal did not name (the
/// code of `Cart`) and what it named but did not change (`Store`), and the
/// cumulative change since the approved baseline on `origin` (the part
/// `Basket`, added before the cycle's base); the thread says both, the
/// reviewer's judgments are recorded, and the purpose gate passed.
#[test]
fn a_cycle_resolves_its_proposal_and_traces_its_change_for_the_review() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let (repository, approved) = modelled_repository(dir.path(), &[]);
    let record = run_objective(dir.path(), &repository, node);
    let cycle = record.cycle().expect("a cycle ran");
    let proposal = cycle.proposal.as_ref().expect("a proposal");
    let resolved = proposal.resolved.as_ref().expect("resolved");
    assert_eq!(resolved.serves[0].name, "Shop::Fast");
    assert_eq!(resolved.serves[0].kind, "requirement def");
    assert_eq!(resolved.parts[0].name, "Shop::Store");
    let traced = cycle.traceability.as_ref().expect("traced");
    assert_eq!(
        traced.not_named,
        vec!["Shop::Cart (part def, its linked code changed)".to_string()],
        "{traced:?}"
    );
    assert_eq!(
        traced.not_changed,
        vec!["Shop::Store (part def)".to_string()]
    );
    assert_eq!(
        Some(&traced.commit),
        cycle.review.as_ref().map(|r| &r.commit)
    );
    let cumulative = cycle.cumulative.as_ref().expect("the cumulative change");
    assert!(cumulative.approved);
    assert_eq!(cumulative.since, approved);
    let parts = cumulative
        .groups
        .iter()
        .find(|g| g.what == "top-level part defs")
        .unwrap();
    assert_eq!(parts.added, vec!["Shop::Basket".to_string()]);
    let review = cycle.review.as_ref().unwrap();
    assert_eq!(review.traceability, "none listed");
    assert!(review.purpose.contains("purpose"));
    let gate = cycle
        .attempt()
        .unwrap()
        .gates
        .iter()
        .find(|g| g.name == "purpose and governance unchanged")
        .expect("the purpose gate");
    assert!(gate.passed(), "{gate:?}");
    let store = Store::new(dir.path().join("objectives"));
    let thread = store.thread(&record.id, 0);
    let traced_entry = thread
        .iter()
        .find(|e| e.text.starts_with("Traceability of "))
        .expect("traceability in the thread");
    assert!(
        traced_entry
            .text
            .contains("1 changed but not named, 1 named but not changed"),
        "{}",
        traced_entry.text
    );
    assert!(
        traced_entry
            .details
            .as_deref()
            .unwrap_or_default()
            .contains("Shop::Cart")
    );
    assert!(thread.iter().any(|e| {
        e.text
            .starts_with("Cumulative change since the approved baseline at")
    }));
    // The approved baseline is where the Operator put it, here and on
    // origin: the cycle moved no tag.
    assert_eq!(
        record.origin.as_deref(),
        Some(dir.path().join("origin.git").display().to_string().as_str())
    );
    assert_eq!(
        git_out(&repository, &["rev-parse", "approved-baseline^{commit}"]),
        approved
    );
    let remote = git_out(
        &repository,
        &[
            "ls-remote",
            "--tags",
            "origin",
            "refs/tags/approved-baseline",
        ],
    );
    assert!(remote.starts_with(&approved), "{remote}");
}

/// The gates of every attempt of the cycle, and whether the cycle merged.
fn purpose_gates(record: &Objective) -> Vec<agq_orchestrator::record::Outcome> {
    let cycle = record.cycle().expect("a cycle ran");
    assert!(
        cycle.merged.is_none() && cycle.review.is_none(),
        "{cycle:?}"
    );
    cycle
        .attempts
        .iter()
        .flat_map(|a| a.gates.iter())
        .filter(|g| g.name == "purpose and governance unchanged")
        .cloned()
        .collect()
}

/// C-55, end to end: a cycle that edits ROADMAP.md in a governed project
/// fails the purpose gate on every attempt, goes to repair like any gate's
/// failure, and is never reviewed nor merged.
#[test]
fn a_cycle_that_edits_the_roadmap_is_not_merged() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let (repository, _) = modelled_repository(dir.path(), &["EDIT-ROADMAP"]);
    let record = run_objective(dir.path(), &repository, node);
    let gates = purpose_gates(&record);
    assert!(gates.len() >= 2, "a repair followed: {gates:?}");
    assert!(
        gates
            .iter()
            .all(|g| !g.passed() && g.detail.contains("ROADMAP.md states the purpose")),
        "{gates:?}"
    );
    assert!(record.cycle().unwrap().blocker.is_some());
}

/// C-55, end to end, and the gate reads the commit, not the checkout the
/// checks ran in: a cycle rewrites the purpose requirement while a required
/// check deletes `model/` in that checkout; the gate still finds the change
/// by identity, and the cycle is not merged.
#[test]
fn a_cycle_that_rewrites_the_purpose_is_not_merged_even_when_a_check_hides_the_model() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let (repository, _) = modelled_repository(dir.path(), &["EDIT-PURPOSE"]);
    let words = |w: &[&str]| w.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let checks = vec![
        words(&["git", "rm", "-r", "-q", "--", "model"]),
        words(&["git", "status", "--short"]),
    ];
    let record = run_objective_with(dir.path(), &repository, node, Some(checks));
    let gates = purpose_gates(&record);
    assert!(!gates.is_empty());
    assert!(
        gates.iter().all(|g| !g.passed()
            && g.detail
                .contains("purpose requirement changes (Shop::Purpose::(doc) (doc, updated))")),
        "{gates:?}"
    );
    // The check did run, and did delete the model there.
    let first = &record.cycle().unwrap().attempts[0];
    assert!(first.checks[0].passed(), "{:?}", first.checks);
}

/// C-55, end to end: a proposal whose `serves` names a part, not a
/// requirement of the model, is refused with the reason, which the thread
/// shows; refused again, the cycle stops without a proposal.
#[test]
fn a_proposal_serving_a_part_is_refused() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let (repository, _) = modelled_repository(dir.path(), &["SERVES-PART"]);
    let record = run_objective(dir.path(), &repository, node);
    let cycle = record.cycle().expect("a cycle ran");
    assert!(cycle.proposal.is_none());
    assert!(
        cycle
            .blocker
            .as_deref()
            .unwrap_or_default()
            .contains("acceptable proposal"),
        "{:?}",
        cycle.blocker
    );
    let store = Store::new(dir.path().join("objectives"));
    let refused: Vec<_> = store
        .thread(&record.id, 0)
        .into_iter()
        .filter(|e| e.text.contains("not accepted"))
        .collect();
    assert_eq!(refused.len(), 2, "{refused:?}");
    assert!(
        refused[0]
            .text
            .contains("`Shop::Store` is a part def, not a requirement"),
        "{}",
        refused[0].text
    );
    assert!(refused[0].text.contains("its requirements are: "));
    assert!(refused[0].text.contains("Shop::Fast"));
}
