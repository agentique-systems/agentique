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
    drop(agq_system_state::Project::open(repository));
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

/// AR-T-004 (reproduction, ROADMAP §4.16 "Durable, continuing work"): the
/// handover to an adopted build is asked only once the objective's
/// continuation is durable. Here the objective's own record cannot be
/// written when the cycle adopts (a folder stands where its atomic write
/// puts its temporary file), while its thread and journal still can: the
/// Studio must not be told to hand over to a build the record on disk does
/// not say to continue in. The objective pauses instead; once the record
/// can be written again, Resume hands over, with the continuation on disk.
#[test]
fn an_adoption_waits_for_its_continuation_to_be_saved() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let mut objective = store
        .create(
            "Adopt the merged build",
            &repository,
            "main",
            Budgets::default(),
            Permissions {
                push: true,
                merge: true,
                adopt: true,
                ..Permissions::default()
            },
        )
        .unwrap();
    // A cycle whose build passed its trial: it is adopted next.
    let mut cycle = agq_orchestrator::record::Cycle::new(1);
    cycle.phase = Phase::Adopt;
    cycle.build = Some("0.1.0-adopted".into());
    objective.cycles.push(cycle);
    store.save(&objective).unwrap();
    let id = objective.id.clone();
    // From here the objective's record cannot be written (as on a full disk
    // or a locked file); nothing else is in the way.
    let in_the_way = store
        .folder
        .join(&id)
        .join(format!("objective.tmp.{}", std::process::id()));
    std::fs::create_dir_all(&in_the_way).unwrap();
    let setup = Setup {
        store: store.clone(),
        work: dir.path().join("work"),
        builds: dir.path().join("builds"),
        runtime: Box::new(|_, _| Err("no agent works in this test".into())),
        keys: Vec::new(),
        checks: Vec::new(),
        protected: Vec::new(),
        running_build: None,
        speed: "instant".into(),
        credential: Box::new(|_| None),
        studios: Box::new(run::Live),
    };
    let handle = run::start(setup, objective);
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut stopped = false;
    let mut resumed = false;
    let mut thread = Vec::new();
    // Each handover asked: its build, and the continuation on disk then.
    let mut handovers = Vec::new();
    loop {
        match handle.events.recv_timeout(Duration::from_millis(50)) {
            Ok(Event::Adopt { build, reply }) => {
                let saved = store.load(&id).unwrap().continuation;
                handovers.push((build, saved));
                // The Studio saves and exits here: the Driver ends.
                let _ = reply.send(Ok(()));
            }
            Ok(Event::Thread(entry)) => {
                // Paused, as it should be: nothing was handed over. The
                // Operator clears what was in the way and resumes it.
                if entry.text.starts_with("Paused until you resume it") && !resumed {
                    assert!(handovers.is_empty(), "{handovers:?}");
                    std::fs::remove_dir_all(&in_the_way).unwrap();
                    handle.send(Command::Resume);
                    resumed = true;
                }
                thread.push(entry.text);
            }
            Ok(_) => {}
            Err(_) if handle.finished() => break,
            Err(_) => {}
        }
        if Instant::now() > deadline && !stopped {
            handle.send(Command::Stop);
            stopped = true;
        }
        assert!(
            Instant::now() < deadline + Duration::from_secs(30),
            "the objective did not end: {thread:#?}"
        );
    }
    // Resumed, it handed over once, with its continuation on disk.
    assert!(resumed && !stopped, "{thread:#?}");
    assert_eq!(handovers.len(), 1, "{handovers:?}\n{thread:#?}");
    // It waits for the Operator, saying why, rather than failing the cycle.
    assert!(
        thread
            .iter()
            .any(|t| t.starts_with("Paused until you resume it")
                && t.contains("was not handed over")),
        "{thread:#?}"
    );
    assert!(
        !thread.iter().any(|t| t.starts_with("Cycle 1 stopped")),
        "{thread:#?}"
    );
    // The failure took effect, and the objective said so.
    assert!(
        thread
            .iter()
            .any(|t| t.starts_with("The objective's record could not be saved")),
        "{thread:#?}"
    );
    // No handover without its continuation on disk.
    assert!(
        handovers
            .iter()
            .all(|(build, saved)| saved.as_ref().is_some_and(|c| &c.build == build)),
        "the Studio was told to hand over although the record on disk holds no continuation for it: {handovers:?}\n{thread:#?}"
    );
}

/// An objective that may merge and adopt, whose first cycle merged the
/// repository's head and is at `phase` with `build` (AR-T-014); its roles
/// cannot work (no agent), so nothing after the merge needs one.
fn merged_cycle_at(
    dir: &Path,
    phase: Phase,
    build: Option<&str>,
    running_build: Option<&str>,
    cycles: u32,
) -> (Store, Objective, Setup, String) {
    // An empty manifest: a release build of it fails at once.
    let repository = repository_with(dir, &["Cargo.toml"]);
    let merged = git_out(&repository, &["rev-parse", "HEAD"]);
    let store = Store::new(dir.join("objectives"));
    let mut objective = store
        .create(
            "Improve it, then again in the adopted build",
            &repository,
            "main",
            Budgets {
                cycles,
                ..Budgets::default()
            },
            Permissions {
                push: true,
                merge: true,
                adopt: true,
                ..Permissions::default()
            },
        )
        .unwrap();
    let mut cycle = agq_orchestrator::record::Cycle::new(1);
    cycle.phase = phase;
    cycle.merged = Some(merged.clone());
    cycle.build = build.map(str::to_string);
    objective.cycles.push(cycle);
    store.save(&objective).unwrap();
    let setup = adopting_setup(dir, &store, running_build);
    (store, objective, setup, merged)
}

/// A Setup whose roles cannot work, for objectives past their merge.
fn adopting_setup(dir: &Path, store: &Store, running_build: Option<&str>) -> Setup {
    Setup {
        store: store.clone(),
        work: dir.join("work"),
        builds: dir.join("builds"),
        runtime: Box::new(|_, _| Err("no agent works in this test".into())),
        keys: Vec::new(),
        checks: Vec::new(),
        protected: Vec::new(),
        running_build: running_build.map(str::to_string),
        speed: "instant".into(),
        credential: Box::new(|_| None),
        studios: Box::new(run::Live),
    }
}

/// What a held objective's run shows: its thread, the handovers it asked
/// (each answered by `answer`), and how often it was held before `then`
/// (Resume or Stop) was sent after the `holds`-th hold.
struct Held {
    thread: Vec<String>,
    handovers: Vec<(String, Option<agq_orchestrator::record::Continuation>)>,
    holds: usize,
}

fn run_held(
    store: &Store,
    setup: Setup,
    objective: Objective,
    answer: impl Fn(usize) -> Result<(), String>,
    then: impl Fn(usize) -> Command,
) -> Held {
    let id = objective.id.clone();
    let handle = run::start(setup, objective);
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut held = Held {
        thread: Vec::new(),
        handovers: Vec::new(),
        holds: 0,
    };
    loop {
        match handle.events.recv_timeout(Duration::from_millis(50)) {
            Ok(Event::Adopt { build, reply }) => {
                let saved = store.load(&id).unwrap().continuation;
                held.handovers.push((build, saved));
                let _ = reply.send(answer(held.handovers.len()));
            }
            Ok(Event::Thread(entry)) => {
                if entry.text.contains("is not adopted:") && entry.text.contains("Resume") {
                    held.holds += 1;
                    // The record on disk holds it: paused, at its phase.
                    let record = store.load(&id).unwrap();
                    assert_eq!(record.state, State::Paused, "{:?}", entry.text);
                    assert!(record.continuation.is_none(), "{record:?}");
                    assert_eq!(record.cycles.len(), 1, "no next cycle starts");
                    handle.send(then(held.holds));
                }
                held.thread.push(entry.text);
            }
            Ok(_) => {}
            Err(_) if handle.finished() => break,
            Err(_) => {}
        }
        assert!(
            Instant::now() < deadline,
            "the objective did not end: {:#?}",
            held.thread
        );
    }
    held
}

/// AR-T-014 (J8, K8, §4.16): a merged change whose release build fails is
/// held, not followed. The objective pauses at Build, saying it is merged
/// and not adopted, and no next cycle starts on a commit no build ran
/// (before, cycle 2 started on it). Resume builds it again; Stop ends the
/// objective, still saying so.
#[test]
fn a_merged_change_whose_build_fails_is_held_until_resumed_or_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let (store, objective, setup, merged) =
        merged_cycle_at(dir.path(), Phase::Build, None, None, 2);
    let id = objective.id.clone();
    // The fixture's manifest is empty: its release build fails.
    let held = run_held(
        &store,
        setup,
        objective,
        |_| Ok(()),
        |holds| {
            if holds == 1 {
                Command::Resume
            } else {
                Command::Stop
            }
        },
    );
    assert_eq!(
        held.holds, 2,
        "held, built again, held again: {:#?}",
        held.thread
    );
    assert!(held.handovers.is_empty());
    let short = &merged[..10];
    assert!(
        held.thread.iter().any(|t| t
            .starts_with(&format!("Cycle 1, merged as {short}, is not adopted:"))
            && t.contains("Resume builds it again")),
        "{:#?}",
        held.thread
    );
    let record = store.load(&id).unwrap();
    assert_eq!(record.state, State::Stopped);
    assert_eq!(
        record.cycles.len(),
        1,
        "no cycle started on the merged commit"
    );
    assert_eq!(record.cycles[0].phase, Phase::Build);
    assert!(!record.cycles[0].adopted);
    // The cycle itself says so, whatever later happens to the note.
    let blocker = record.cycles[0].blocker.clone().unwrap_or_default();
    assert!(
        blocker.starts_with(&format!("Merged as {short}, not adopted:")),
        "{blocker}"
    );
    let note = record.note.unwrap_or_default();
    assert!(
        note.contains(&format!("Cycle 1, merged as {short}, is not adopted")),
        "{note}"
    );
}

/// AR-T-014: a build that does not pass its trial (here, one with no
/// manifest) is held at Try in the same way.
#[test]
fn a_merged_change_whose_trial_fails_is_held() {
    let dir = tempfile::tempdir().unwrap();
    let (store, objective, setup, _) = merged_cycle_at(dir.path(), Phase::Try, Some("b1"), None, 2);
    let id = objective.id.clone();
    let held = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    assert_eq!(held.holds, 1, "{:#?}", held.thread);
    assert!(
        held.thread
            .iter()
            .any(|t| t.contains("Resume tries its build again")),
        "{:#?}",
        held.thread
    );
    let record = store.load(&id).unwrap();
    assert_eq!(record.cycles.len(), 1);
    assert_eq!(record.cycles[0].phase, Phase::Try);
}

/// AR-T-014: a handover the Studio refuses is held at Adopt, with no
/// continuation (before, the record kept Resume and the next cycle
/// started); Resume hands over again, with its continuation on disk.
#[test]
fn a_refused_handover_is_held_and_resume_hands_over_again() {
    let dir = tempfile::tempdir().unwrap();
    let (store, objective, setup, _) =
        merged_cycle_at(dir.path(), Phase::Adopt, Some("b1"), None, 2);
    let id = objective.id.clone();
    let held = run_held(
        &store,
        setup,
        objective,
        |n| {
            if n == 1 {
                Err("it reads and writes other data formats".into())
            } else {
                Ok(())
            }
        },
        |_| Command::Resume,
    );
    assert_eq!(held.holds, 1, "{:#?}", held.thread);
    assert_eq!(held.handovers.len(), 2, "{:?}", held.handovers);
    assert!(
        held.handovers.iter().all(|(build, saved)| build == "b1"
            && saved
                .as_ref()
                .is_some_and(|c| c.build == "b1" && c.cycle == 1)),
        "{:?}",
        held.handovers
    );
    assert!(
        held.thread
            .iter()
            .any(|t| t.contains("Resume hands it over again")),
        "{:#?}",
        held.thread
    );
    // Handed over: the adopted build goes on from the record, the hold's
    // reason and note gone with it.
    let record = store.load(&id).unwrap();
    assert_eq!(record.cycles.len(), 1);
    assert_eq!(record.cycles[0].phase, Phase::Resume);
    assert!(record.continuation.is_some());
    assert_eq!(record.cycles[0].blocker, None);
    assert!(
        !record.note.unwrap_or_default().starts_with("Merged as"),
        "the note no longer says it is held"
    );
}

/// AR-T-014: a handover that took longer than its wait, resumed in the
/// build it handed over to, is not asked again: that build runs, so the
/// cycle is adopted, as `resume` records it.
#[test]
fn a_handover_resumed_in_its_build_is_adopted() {
    let dir = tempfile::tempdir().unwrap();
    let (store, objective, setup, _) =
        merged_cycle_at(dir.path(), Phase::Adopt, Some("b1"), Some("b1"), 1);
    let id = objective.id.clone();
    let held = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    assert!(held.handovers.is_empty(), "{:?}", held.handovers);
    assert_eq!(held.holds, 0, "{:#?}", held.thread);
    let record = store.load(&id).unwrap();
    assert!(record.cycles[0].adopted, "{:#?}", held.thread);
    assert_eq!(record.state, State::Done, "{:#?}", held.thread);
}

/// AR-T-014's controls: a failure before the merge, and a merge the
/// objective may not adopt, are not held: the next cycle starts as before.
#[test]
fn a_failure_before_the_merge_and_a_merge_only_objective_go_on() {
    let dir = tempfile::tempdir().unwrap();
    let (store, mut objective, setup, _) =
        merged_cycle_at(dir.path(), Phase::Failed, None, None, 2);
    // Failed before its merge: nothing merged.
    objective.cycles[0].merged = None;
    objective.cycles[0].blocker = Some("the implementer changed nothing".into());
    store.save(&objective).unwrap();
    let id = objective.id.clone();
    let held = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    assert_eq!(held.holds, 0, "{:#?}", held.thread);
    let record = store.load(&id).unwrap();
    assert_eq!(
        record.cycles.len(),
        2,
        "the next cycle started: {:#?}",
        held.thread
    );
    // Merged, and not allowed to adopt: done, and the next cycle starts.
    let other = tempfile::tempdir().unwrap();
    let (store, mut objective, setup, _) =
        merged_cycle_at(other.path(), Phase::Done, None, None, 2);
    objective.permissions.adopt = false;
    store.save(&objective).unwrap();
    let id = objective.id.clone();
    let held = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    assert_eq!(held.holds, 0, "{:#?}", held.thread);
    assert_eq!(
        store.load(&id).unwrap().cycles.len(),
        2,
        "{:#?}",
        held.thread
    );
}

/// AR-DEC-002: an objective continued in a build or repository that would
/// check it with less than it was started with does not go on: before any
/// phase, it stays not finished, its note naming what is missing.
#[test]
fn continuing_with_a_check_fewer_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (store, mut objective, mut setup, _) =
        merged_cycle_at(dir.path(), Phase::Build, None, None, 2);
    let words = |w: &str| w.split(' ').map(str::to_string).collect::<Vec<_>>();
    objective.frozen = Some(agq_orchestrator::record::FrozenChecks {
        checks: vec![
            words("cargo fmt --all -- --check"),
            words("cargo test --workspace"),
        ],
        protected: Vec::new(),
        safeguards: Vec::new(),
        build: Some("b1".into()),
    });
    store.save(&objective).unwrap();
    let id = objective.id.clone();
    setup.checks = vec![words("cargo fmt --all -- --check")];
    let held = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    assert_eq!(held.holds, 0, "it built nothing: {:#?}", held.thread);
    let record = store.load(&id).unwrap();
    assert_eq!(record.state, State::Running, "not finished");
    assert_eq!(record.cycles.len(), 1);
    assert_eq!(record.cycles[0].phase, Phase::Build, "no phase ran");
    let note = record.note.unwrap_or_default();
    assert!(
        note.contains("Missing now: the required check `cargo test --workspace`")
            && note.contains("started with them in b1"),
        "{note}"
    );
    assert!(
        held.thread.iter().any(|t| t.contains("Missing now")),
        "{:#?}",
        held.thread
    );
}

/// AR-DEC-002: a build that requires more than the objective was started
/// with goes on, holding it to the more, and says so; a record without
/// frozen checks is frozen at its first run, its thread listing them.
#[test]
fn continuing_with_a_check_more_goes_on_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let (store, mut objective, mut setup, _) =
        merged_cycle_at(dir.path(), Phase::Build, None, None, 2);
    let words = |w: &str| w.split(' ').map(str::to_string).collect::<Vec<_>>();
    setup.checks = vec![words("cargo fmt --all -- --check")];
    let id = objective.id.clone();
    // Its record has none yet: frozen now, said in the thread.
    let first = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    assert!(
        first
            .thread
            .iter()
            .any(|t| t.starts_with("Its record had no frozen checks")),
        "{:#?}",
        first.thread
    );
    objective = store.load(&id).unwrap();
    let frozen = objective.frozen.clone().expect("frozen at its first run");
    assert_eq!(frozen.checks, vec![words("cargo fmt --all -- --check")]);
    assert!(frozen.protected.iter().any(|p| p == ".git"), "{frozen:?}");
    assert!(
        frozen.safeguards.iter().any(|p| p == "crates/orchestrator"),
        "{frozen:?}"
    );
    // Continued with a check more: it goes on (and is held at its build).
    let mut more = adopting_setup(dir.path(), &store, None);
    more.checks = vec![
        words("cargo fmt --all -- --check"),
        words("cargo test --workspace"),
    ];
    objective.state = State::Running;
    objective.note = None;
    store.save(&objective).unwrap();
    let second = run_held(&store, more, objective, |_| Ok(()), |_| Command::Stop);
    assert!(
        second.thread.iter().any(|t| t
            .starts_with("This build also requires the required check `cargo test --workspace`")),
        "{:#?}",
        second.thread
    );
    assert_eq!(
        second.holds, 1,
        "it went on to its build: {:#?}",
        second.thread
    );
    // What it was started with stays what it is held to at least.
    assert_eq!(store.load(&id).unwrap().frozen, Some(frozen));
}

/// AR-DEC-002, after an adoption: a build that took over records the
/// adoption (Resume runs nothing else), and is then refused before the next
/// phase if it would check with less than the objective was started with.
#[test]
fn an_adopted_build_with_a_check_fewer_records_the_adoption_and_goes_no_further() {
    let dir = tempfile::tempdir().unwrap();
    let (store, mut objective, mut setup, _) =
        merged_cycle_at(dir.path(), Phase::Resume, Some("b1"), Some("b1"), 2);
    let words = |w: &str| w.split(' ').map(str::to_string).collect::<Vec<_>>();
    objective.continuation = Some(agq_orchestrator::record::Continuation {
        cycle: 1,
        build: "b1".into(),
        at: "t".into(),
    });
    objective.frozen = Some(agq_orchestrator::record::FrozenChecks {
        checks: vec![
            words("cargo fmt --all -- --check"),
            words("cargo test --workspace"),
        ],
        protected: Vec::new(),
        safeguards: Vec::new(),
        build: Some("b0".into()),
    });
    store.save(&objective).unwrap();
    let id = objective.id.clone();
    setup.checks = vec![words("cargo fmt --all -- --check")];
    let held = run_held(&store, setup, objective, |_| Ok(()), |_| Command::Stop);
    let record = store.load(&id).unwrap();
    assert!(record.cycles[0].adopted, "{:#?}", held.thread);
    assert_eq!(record.continuation, None);
    assert_eq!(record.cycles.len(), 1, "no next cycle: {:#?}", held.thread);
    assert_eq!(record.state, State::Running, "not finished");
    let note = record.note.unwrap_or_default();
    assert!(note.contains("Missing now"), "{note}");
}
