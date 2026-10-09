//! Exploring cycles end to end with scripted agents and a stand-in Studio
//! (C-54, W12.5): Explore and Reproduce before Propose, the finding's replay
//! frozen as a criterion that fails on the base and passes on the change,
//! evidence on the base, the testing knowledge, the bounds on worktrees and
//! test instances; delegation of a child objective within the parent's
//! budget, its result back to the lead, a refused delegation, a child
//! stopped alone; the Operator's messages routed to the implementer or the
//! lead; and two explorations without a new problem ending the objective.
//! The stand-in companion plays the roles (`fixtures/explore-companion.mjs`)
//! and the stand-in Studio (`standin`) has an unlabelled button until a
//! build's checkout holds the fix (`FIXED`). Needs Node and git (skipped
//! without Node).

#[allow(dead_code)]
mod standin;

use agq_assistant::claude_agent::{ClaudeAgent, Installation, find_node};
use agq_orchestrator::control::Options;
use agq_orchestrator::explore::Instance;
use agq_orchestrator::findings::{Check as Found, DispositionKind, State as FoundState};
use agq_orchestrator::knowledge::Knowledge;
use agq_orchestrator::record::{
    Access, Budgets, DirectiveStatus, Objective, Permissions, Recipient, RoleModel, State, Store,
};
use agq_orchestrator::run::{self, Command, Event, Setup, Studios};
use agq_orchestrator::thread::{Author, Kind, ThreadEntry};
use agq_providers::{ModelRef, Provider, Secret};
use standin::{Defects, StandIn};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn companion() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/explore-companion.mjs")
}

fn git(folder: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(folder)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A repository to improve, with `markers` (files that steer the scripted
/// agents) besides its README.
fn repository_with(dir: &Path, markers: &[&str]) -> PathBuf {
    let repository = dir.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    git(&repository, &["init", "-q", "-b", "main"]);
    git(&repository, &["config", "user.name", "Agentique test"]);
    git(
        &repository,
        &["config", "user.email", "test@example.invalid"],
    );
    std::fs::write(repository.join("README.md"), "A repository to improve.\n").unwrap();
    for marker in markers {
        std::fs::write(repository.join(marker), "").unwrap();
    }
    git(&repository, &["add", "-A"]);
    git(&repository, &["commit", "-q", "-m", "Start"]);
    repository
}

/// The stand-in Studio: a build is its checkout, and its test instance has
/// History's Archive button unlabelled unless the checkout holds `FIXED`
/// (or the Studio has no defect at all). Every build and instance asked
/// for is logged, with whether it got a key or the stand-in Assistant.
struct StandInStudios {
    defects: bool,
    log: Arc<Mutex<Vec<String>>>,
}

impl Studios for StandInStudios {
    fn build(
        &self,
        checkout: &Path,
        _target: &Path,
        _builds: &Path,
        _cancel: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        self.log
            .lock()
            .unwrap()
            .push(format!("build {}", checkout.display()));
        Ok(checkout.to_path_buf())
    }

    fn instance(
        &self,
        exe: &Path,
        _start: &Path,
        _folder: &Path,
        options: Options,
    ) -> Box<dyn Instance> {
        let fixed = exe.join("FIXED").exists();
        self.log.lock().unwrap().push(format!(
            "instance fixed={fixed} key={} stand-in={}",
            options.key.is_some(),
            options.stand_in
        ));
        Box::new(StandIn::new(Defects {
            unlabelled: self.defects && !fixed,
            ..Defects::default()
        }))
    }
}

/// An exploring objective with the cycle's four sessions' models (no
/// explorer, escalation or decisions model: the rules explore, asking no
/// model).
fn exploring(store: &Store, repository: &Path, intent: &str, budgets: Budgets) -> Objective {
    let mut objective = store
        .create(intent, repository, "main", budgets, Permissions::default())
        .unwrap();
    let model = |role: &str| RoleModel {
        role: role.into(),
        model: ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro"),
        effort: None,
        access: Access::Key,
        configured: ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro"),
        fallback: None,
        credential: "DEEPSEEK_API_KEY".into(),
        billed: "per token".into(),
    };
    objective.models = ["lead", "implementer", "reviewer", "evaluator"]
        .into_iter()
        .map(model)
        .collect();
    objective.explore = true;
    store.save(&objective).unwrap();
    objective
}

fn budgets() -> Budgets {
    Budgets {
        usd: 2.0,
        cycles: 1,
        attempts: 3,
        hours: 1.0,
        steps: 30,
        ..Budgets::default()
    }
}

fn setup_with(
    dir: &Path,
    store: &Store,
    node: agq_assistant::claude_agent::Node,
    defects: bool,
) -> (Setup, Arc<Mutex<Vec<String>>>) {
    let data = dir.join("agent");
    let script = companion();
    let log = Arc::new(Mutex::new(Vec::new()));
    let setup = Setup {
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
        keys: vec!["sk-fake-0123456789abcdef".into()],
        checks: vec![vec!["git".into(), "status".into(), "--short".into()]],
        protected: Vec::new(),
        running_build: None,
        speed: "instant".into(),
        credential: Box::new(|_| None),
        studios: Box::new(StandInStudios {
            defects,
            log: log.clone(),
        }),
    };
    (setup, log)
}

/// Runs the objective to its end, calling `on` with each thread entry as it
/// comes (it may send a command), and returns every entry seen.
fn run_to_end(
    setup: Setup,
    objective: Objective,
    mut on: impl FnMut(&ThreadEntry, &run::Handle),
) -> Vec<ThreadEntry> {
    let handle = run::start(setup, objective);
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut seen = Vec::new();
    loop {
        // Read before draining: what was sent before the end is taken.
        let finished = handle.finished();
        while let Ok(event) = handle.events.try_recv() {
            if let Event::Thread(entry) = event {
                on(&entry, &handle);
                seen.push(entry);
            }
        }
        if finished {
            return seen;
        }
        assert!(
            Instant::now() < deadline,
            "the objective did not end: {}",
            texts(&seen)
        );
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn texts(thread: &[ThreadEntry]) -> String {
    thread
        .iter()
        .map(|e| format!("#{} {:?} {}: {}", e.seq, e.kind, e.author.label(), e.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `FindingsReproduce` and `DefectShownBefore`: an exploring cycle finds
/// the unlabelled button by exploring the base build, reproduces and
/// reduces it, the lead proposes to fix it, the replay is frozen as a
/// criterion that fails on the base (its reproduction reused) and passes on
/// the change, a test that fails on the base with the change's test file
/// brought over is evidence too; then the merge it may not make stops the
/// cycle. The testing knowledge keeps the run and the finding; the cycle's
/// worktrees and test instances are gone but its work.
#[test]
fn an_exploring_cycle_reproduces_a_finding_shows_it_on_the_base_and_fixes_it() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    // REJUDGE: once its proposal is accepted, the lead tries to judge the
    // finding it fixes a wrong expectation, which is refused (C-55).
    let repository = repository_with(dir.path(), &["REJUDGE"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, log) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(&store, &repository, "Find and fix problems", budgets());
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    let cycle = record.cycle().expect("a cycle ran");
    // Explore: the lead's plan as its directive to the explorer.
    let plan = record
        .directives
        .iter()
        .find(|d| d.recipient == Recipient::Role("explorer".into()))
        .expect("the lead's plan");
    assert!(plan.scope.instruction.contains("History"), "{plan:?}");
    assert_eq!(cycle.explorations.len(), 1, "{}", texts(&seen));
    let exploration = &cycle.explorations[0];
    assert_eq!(exploration.reproduced, 1);
    assert!(exploration.build.starts_with("debug-"), "{exploration:?}");
    // Reproduce: the unlabelled button, reproduced twice and reduced.
    let finding = &cycle.findings[0];
    assert_eq!(finding.check, Found::ReadableLabels);
    assert_eq!(finding.state, FoundState::Reproduced);
    assert!(finding.replays.iter().filter(|r| r.failed()).count() >= 2);
    // Reduced to the one step that shows it (opening History), unless it
    // was found in that one step already.
    assert_eq!(finding.replay_steps().len(), 1, "{finding:#?}");
    assert_eq!(finding.replay_steps()[0].target(), "History");
    // Propose: the finding chosen; its replay frozen with the proposal.
    let proposal = cycle.proposal.as_ref().expect("a proposal");
    assert_eq!(proposal.finding.as_deref(), Some(finding.identity.as_str()));
    assert_eq!(
        cycle.replay.as_ref().map(|f| &f.identity),
        Some(&finding.identity)
    );
    // Evidence on the base: the replay (its reproduction reused) and the
    // test, brought over, both fail there; the gate passed.
    let before = |name: &str| cycle.before.iter().find(|o| o.name == name).unwrap();
    assert_eq!(before("replay").verdict, "failed");
    assert!(before("replay").detail.contains("when it was reproduced"));
    assert_eq!(before("c1").verdict, "failed", "{:?}", before("c1"));
    assert!(before("c1").detail.contains("fixed.test.mjs"));
    let attempt = cycle.attempt().expect("an attempt");
    let gate = attempt
        .gates
        .iter()
        .find(|g| g.name == "the criteria show the defect on the base")
        .unwrap();
    assert!(gate.passed(), "{gate:?}");
    // Evaluate: the replay passes on the change's build (no key, the
    // stand-in Assistant); every criterion passed.
    let replay = attempt
        .criteria
        .iter()
        .find(|o| o.name == "replay")
        .unwrap();
    assert!(replay.passed(), "{replay:?}");
    assert!(attempt.failures().is_empty(), "{:?}", attempt.failures());
    let log = log.lock().unwrap().clone();
    assert!(
        log.iter()
            .any(|l| l == "instance fixed=true key=false stand-in=true"),
        "{log:#?}"
    );
    assert!(
        log.iter()
            .all(|l| !l.starts_with("instance") || l.contains("key=false")),
        "no key without a credential"
    );
    // The merge it may not make stops the cycle.
    assert!(
        cycle
            .blocker
            .as_deref()
            .unwrap_or_default()
            .contains("does not allow pushing")
    );
    assert_eq!(record.state, State::Failed);
    // The thread: the plan, the exploration's result, the reproduction,
    // the criteria on the base.
    let all = store.thread(&id, 0);
    for wanted in [
        "Explore: Look at the History panel",
        "Explores debug-",
        "Explored ",
        "Reproduced: readable-labels",
        "Judges finding f1 a defect, to be fixed",
        "The criteria on the base, before the change",
        "Proposes: Label the History panel's Archive button",
    ] {
        assert!(
            all.iter().any(|e| e.text.starts_with(wanted)),
            "{wanted}: {}",
            texts(&all)
        );
    }
    // The testing knowledge keeps the run and the finding.
    let knowledge = Knowledge::load(
        &Knowledge::file(&store, &repository),
        &Knowledge::key(&repository),
    )
    .unwrap();
    assert_eq!(knowledge.runs.len(), 1);
    assert!(!knowledge.coverage.is_empty());
    assert_eq!(knowledge.runs[0].commit, cycle.base.clone().unwrap());
    assert!(
        knowledge
            .findings
            .iter()
            .any(|f| f.identity == finding.identity && f.state == FoundState::Reproduced)
    );
    let adjudications = all
        .iter()
        .filter(|e| e.kind == Kind::Activity && e.text == "adjudicate_finding")
        .count();
    assert_eq!(adjudications, 2, "the lead tried to judge it again");
    // Judged a defect before it was fixed (C-55), on the cycle and in the
    // knowledge; judging it otherwise after its proposal was accepted was
    // refused.
    assert_eq!(
        finding.disposition.as_ref().map(|d| d.kind),
        Some(DispositionKind::Defect)
    );
    assert_eq!(
        knowledge.disposition(&finding.identity).map(|d| d.kind),
        Some(DispositionKind::Defect)
    );
    // Bounds: the failed cycle keeps its work and nothing else.
    let worktrees = git(&repository, &["worktree", "list"]);
    assert_eq!(worktrees.lines().count(), 2, "{worktrees}");
    assert!(worktrees.contains(&format!("{id}-1")) || worktrees.contains("cycle-1"));
    let folders: Vec<String> = std::fs::read_dir(dir.path().join("work").join(&id).join("cycle-1"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(folders, vec!["work".to_string()]);
}

/// Two explorations in a row that reproduce nothing new end the objective
/// as an outcome; the cycle's worktrees all go.
#[test]
fn two_explorations_without_a_new_problem_end_the_objective() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, false);
    let mut budgets = budgets();
    budgets.cycles = 3;
    budgets.steps = 8;
    let objective = exploring(&store, &repository, "Find problems", budgets);
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    assert_eq!(record.state, State::Done, "{}", texts(&seen));
    assert!(
        record
            .note
            .as_deref()
            .unwrap_or_default()
            .starts_with("Nothing new reproduced"),
        "{:?}",
        record.note
    );
    assert_eq!(record.cycles.len(), 1);
    let cycle = record.cycle().unwrap();
    assert_eq!(cycle.explorations.len(), 2);
    assert!(cycle.proposal.is_none());
    assert_ne!(
        cycle.explorations[0].start, cycle.explorations[1].start,
        "another start"
    );
    let worktrees = git(&repository, &["worktree", "list"]);
    assert_eq!(worktrees.lines().count(), 1, "{worktrees}");
}

/// C-55, end to end: the lead judges the reproduced finding a wrong
/// expectation before anything is fixed: nothing is proposed or
/// implemented, the cycle ends with nothing to fix, the disposition is kept
/// in the testing knowledge and shown in the thread, and the next cycle
/// neither reproduces nor offers that finding again.
#[test]
fn a_finding_judged_a_wrong_expectation_is_not_fixed_nor_offered_again() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["WRONG-EXPECTATION"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let mut budgets = budgets();
    budgets.cycles = 2;
    let objective = exploring(&store, &repository, "Find and fix problems", budgets);
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    assert_eq!(record.state, State::Done, "{}", texts(&seen));
    let first = &record.cycles[0];
    let finding = first
        .findings
        .iter()
        .find(|f| f.state == FoundState::Reproduced)
        .expect("a reproduced finding");
    let judged = finding.disposition.as_ref().expect("its disposition");
    assert_eq!(judged.kind, DispositionKind::WrongExpectation);
    assert_eq!((judged.role.as_str(), judged.cycle), ("lead", 1));
    assert_eq!(judged.objective, id);
    // Nothing proposed, nothing implemented.
    assert!(record.cycles.iter().all(|c| c.proposal.is_none()));
    assert!(
        !record
            .directives
            .iter()
            .any(|d| d.recipient == Recipient::Role("implementer".into()))
    );
    // Kept in the testing knowledge, across objectives.
    let knowledge = Knowledge::load(
        &Knowledge::file(&store, &repository),
        &Knowledge::key(&repository),
    )
    .unwrap();
    assert_eq!(
        knowledge.disposition(&finding.identity).map(|d| d.kind),
        Some(DispositionKind::WrongExpectation)
    );
    // Not offered again: the next cycle has no such finding.
    assert_eq!(record.cycles.len(), 2, "{}", texts(&seen));
    assert!(
        record.cycles[1]
            .findings
            .iter()
            .all(|f| f.identity != finding.identity),
        "{:?}",
        record.cycles[1].findings
    );
    let all = store.thread(&id, 0);
    for wanted in [
        "Judges finding f1 a wrong expectation, not a defect",
        "Nothing to fix in this cycle",
    ] {
        assert!(
            all.iter().any(|e| e.text.starts_with(wanted)),
            "{wanted}: {}",
            texts(&all)
        );
    }
}

/// C-55, end to end: a finding judged an ambiguous requirement is a
/// question for the Operator in the thread, is not proposed, and is not
/// reproduced or offered again in later cycles until the Operator answers:
/// two explorations without anything new then end the objective, rather
/// than the same question being asked cycle after cycle.
#[test]
fn an_ambiguous_requirement_is_asked_once_and_the_objective_ends() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["AMBIGUOUS"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let mut budgets = budgets();
    budgets.cycles = 3;
    let objective = exploring(&store, &repository, "Find and fix problems", budgets);
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    assert_eq!(record.state, State::Done, "{}", texts(&seen));
    assert!(
        record
            .note
            .as_deref()
            .unwrap_or_default()
            .starts_with("Nothing new reproduced"),
        "{:?}",
        record.note
    );
    assert_eq!(record.cycles.len(), 2, "{}", texts(&seen));
    let finding = record.cycles[0]
        .findings
        .iter()
        .find(|f| f.state == FoundState::Reproduced)
        .expect("a reproduced finding");
    assert_eq!(
        finding.disposition.as_ref().map(|d| d.kind),
        Some(DispositionKind::AmbiguousRequirement)
    );
    assert!(record.cycles.iter().all(|c| c.proposal.is_none()));
    assert!(
        record.cycles[1]
            .findings
            .iter()
            .all(|f| f.identity != finding.identity),
        "not offered again"
    );
    let all = store.thread(&id, 0);
    let asked: Vec<_> = all
        .iter()
        .filter(|e| {
            e.text
                .starts_with("A question for you: is finding f1 a defect?")
        })
        .collect();
    assert_eq!(asked.len(), 1, "{}", texts(&all));
    // The cycle's directives end with the real reason.
    assert!(
        record
            .directives
            .iter()
            .filter(|d| d.refers_to.as_deref() == Some("cycle-1/exploration-1"))
            .all(|d| d
                .result
                .as_deref()
                .unwrap_or_default()
                .starts_with("nothing to fix")),
        "{:?}",
        record.directives
    );
}

/// `ChildWorkBounded`: the lead's delegation over the budget left is
/// refused with the reason and recorded; one within it becomes a child
/// objective one deeper, explore-only, with no push, merge or adopt, whose
/// result comes back to the lead as its next message, whose spend counts in
/// the parent's, and whose reproduced finding the lead may choose.
#[test]
fn the_lead_delegates_a_child_within_its_budget_and_gets_its_result() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["DELEGATE", "DELEGATE-TOO-MUCH"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(
        &store,
        &repository,
        "DELEGATE-ME: find and fix problems",
        budgets(),
    );
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    // The refused one: recorded with why.
    let refused = record
        .directives
        .iter()
        .find(|d| matches!(d.status, DirectiveStatus::Refused { .. }))
        .expect("a refused delegation");
    assert!(
        matches!(&refused.status, DirectiveStatus::Refused { reason } if reason.contains("at most what is left")),
        "{refused:?}"
    );
    // The child: recorded, run, its result on the directive.
    let directive = record
        .directives
        .iter()
        .find(|d| matches!(&d.recipient, Recipient::Child(c) if !c.is_empty()))
        .expect("a child's directive");
    let Recipient::Child(child_id) = &directive.recipient else {
        unreachable!()
    };
    assert_eq!(directive.status, DirectiveStatus::Done, "{}", texts(&seen));
    assert!(
        directive
            .result
            .as_deref()
            .unwrap_or_default()
            .contains("exploration(s)")
    );
    let scope = directive.scope.budgets.as_ref().unwrap();
    assert!((scope.usd - 0.3).abs() < 1e-9 && scope.steps == 30);
    let child = store.load(child_id).unwrap();
    assert_eq!(child.parent.as_deref(), Some(id.as_str()));
    assert_eq!(child.depth, 1);
    assert_eq!(child.requested_by.as_ref().unwrap().role, "lead");
    assert!(child.explore && child.state == State::Done);
    assert!(!child.permissions.push && !child.permissions.merge && !child.permissions.adopt);
    assert!(child.cycles[0].proposal.is_none(), "a child only explores");
    // Its thread starts with the lead's instruction; its result is in both.
    let theirs = store.thread(child_id, 0);
    assert_eq!(theirs[0].kind, Kind::Human);
    assert!(matches!(&theirs[0].author, Author::Agent { role, .. } if role == "lead"));
    assert!(
        theirs
            .iter()
            .any(|e| e.kind == Kind::Result && e.text.contains("ended"))
    );
    let ours = store.thread(&id, 0);
    let result = ours
        .iter()
        .find(|e| e.kind == Kind::Result && e.text.starts_with("The child objective"))
        .expect("its result in the parent's thread");
    assert_eq!(result.to.as_deref(), Some("lead"));
    assert!(record.delivered >= result.seq, "given to the lead");
    assert!(
        ours.iter()
            .any(|e| e.text.starts_with("Gave the lead") && e.seq > result.seq),
        "{}",
        texts(&ours)
    );
    // Its spend counts in the parent's.
    let child_lead = child.spent.role("lead").tokens;
    assert!(child_lead > 0);
    assert!(record.spent.role("lead").tokens > child_lead);
    // Its reproduced finding is among the cycle's.
    let cycle = record.cycle().unwrap();
    assert!(
        cycle
            .findings
            .iter()
            .any(|f| f.state == FoundState::Reproduced),
        "{}",
        texts(&ours)
    );
    // It explored the parent's base build, which it inherited, and counted
    // its spend once.
    assert_eq!(child.cycles[0].base, cycle.base);
    assert_eq!(child.cycles[0].base_build, cycle.base_build);
    assert_eq!(
        child.cycles[0].explorations[0].build,
        cycle.explorations[0].build
    );
    assert_eq!(directive.counted, child.spent);
}

/// Blocking review finding 1: the evidence on the base is made with each
/// attempt's own test files. Attempt 1's test fails on the base (evidence);
/// its repair weakens the test until it passes anywhere; the test runs on
/// the base are made again with the repair's test files, the test passes
/// there, and the gate fails.
#[test]
fn a_test_weakened_in_the_repair_has_its_evidence_made_again_and_fails() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["WEAKEN"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(&store, &repository, "Find and fix problems", budgets());
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    let cycle = record.cycle().unwrap();
    assert!(cycle.attempts.len() >= 2, "{}", texts(&seen));
    let gate = |attempt: &agq_orchestrator::record::Attempt| {
        attempt
            .gates
            .iter()
            .find(|g| g.name == "the criteria show the defect on the base")
            .cloned()
            .unwrap()
    };
    // Attempt 1: its test failed on the base, by an assertion.
    assert!(
        gate(&cycle.attempts[0]).passed(),
        "{:?}",
        gate(&cycle.attempts[0])
    );
    // The repair's weakened test passes on the base: no evidence any more.
    let repaired = &cycle.attempts[1];
    let failed = gate(repaired);
    assert!(!failed.passed());
    assert!(
        failed.detail.contains("c1 already pass on the base"),
        "{}",
        failed.detail
    );
    let evidence = cycle.evidence.as_ref().unwrap();
    assert_eq!(Some(&evidence.commit), repaired.commit.as_ref());
    let c1 = cycle.before.iter().find(|o| o.name == "c1").unwrap();
    assert_eq!(c1.verdict, "passed");
    assert!(
        seen.iter()
            .any(|e| e.text.starts_with("This attempt's test files differ")),
        "{}",
        texts(&seen)
    );
    // Never reviewed, never merged.
    assert!(cycle.review.is_none());
    assert_eq!(record.state, State::Failed);
}

/// `ChildWorkBounded`: a lead that delegates in every turn gets three
/// children in a turn and the fourth refused, with the reason.
#[test]
fn a_fourth_delegation_in_a_turn_is_refused() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["DELEGATE-MANY"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(
        &store,
        &repository,
        "DELEGATE-ME: find and fix problems",
        budgets(),
    );
    let id = objective.id.clone();
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    let children: Vec<_> = record
        .directives
        .iter()
        .filter(|d| matches!(&d.recipient, Recipient::Child(c) if !c.is_empty()))
        .collect();
    assert_eq!(children.len(), 3, "{}", texts(&seen));
    assert!(children.iter().all(|d| d.status == DirectiveStatus::Done));
    let refused: Vec<_> = record
        .directives
        .iter()
        .filter_map(|d| match &d.status {
            DirectiveStatus::Refused { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].contains("at most 3 children in a turn"),
        "{refused:?}"
    );
    // It planned after, and the cycle went on.
    assert!(record.cycle().unwrap().explorations.len() == 1);
}

/// `ChildWorkBounded`: what a child spent before Agentique stopped (a crash
/// with the child running) is counted in its parent's once, when the child
/// goes on with its parent and ends.
#[test]
fn a_childs_spend_before_a_restart_is_counted_once() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    // The parent, as a crash left it: exploring, with a child running.
    let mut parent = exploring(&store, &repository, "Find and fix problems", budgets());
    let mut cycle = agq_orchestrator::record::Cycle::new(1);
    cycle.exploring = Some(agq_orchestrator::record::Exploring::Explore);
    parent.cycles.push(cycle.clone());
    let child_id = format!("{}-c1", parent.id);
    let directive = parent.direct(
        "lead",
        Recipient::Child(child_id.clone()),
        agq_orchestrator::record::Scope {
            instruction: "Look closely at the History panel".into(),
            focus: None,
            budgets: Some(Budgets {
                usd: 0.5,
                ..budgets()
            }),
            permissions: Some(Permissions::default()),
        },
        Some("cycle-1/delegation".into()),
    );
    store.save(&parent).unwrap();
    let mut child = parent.clone();
    child.id = child_id.clone();
    child.intent = "Look closely at the History panel".into();
    child.parent = Some(parent.id.clone());
    child.depth = 1;
    child.directives = Vec::new();
    child.budgets = Budgets {
        usd: 0.5,
        ..budgets()
    };
    child.cycles = vec![cycle];
    // What it had spent when Agentique stopped.
    child.spent.add(
        "lead",
        &ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro"),
        agq_orchestrator::record::Cost {
            usd: 0.05,
            tokens: 5000,
            unknown: false,
        },
    );
    store.save(&child).unwrap();
    let id = parent.id.clone();
    let seen = run_to_end(setup, parent, |_, _| {});
    let record = store.load(&id).unwrap();
    let child = store.load(&child_id).unwrap();
    assert_eq!(child.state, State::Done, "{}", texts(&seen));
    let settled = record.directive(&directive).unwrap();
    assert_eq!(settled.status, DirectiveStatus::Done);
    assert_eq!(settled.counted, child.spent, "what was counted is kept");
    // The parent's lead: the child's (the 5000 before the restart and its
    // one planning session) and its own two sessions (planning, proposal),
    // each session the same.
    let child_lead = child.spent.role("lead").tokens;
    let per_session = child_lead - 5000;
    assert!(per_session > 0);
    assert_eq!(
        record.spent.role("lead").tokens,
        child_lead + 2 * per_session,
        "{}",
        texts(&seen)
    );
    assert!(record.spent.usd >= 0.05);
}

/// A reproduced finding a failed cycle left unfixed stays eligible: a
/// second objective whose exploration finds nothing new is offered it
/// again, and its replay on this base is run again (it was reproduced on
/// another build), not reused.
#[test]
fn a_known_finding_left_unfixed_is_offered_again() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node.clone(), true);
    let first = exploring(&store, &repository, "Find and fix problems", budgets());
    run_to_end(setup, first, |_, _| {});
    // The first stopped at the merge it may not make: the finding is open.
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let second = exploring(&store, &repository, "Find and fix problems", budgets());
    let id = second.id.clone();
    let seen = run_to_end(setup, second, |_, _| {});
    let record = store.load(&id).unwrap();
    let cycle = record.cycle().unwrap();
    assert!(
        seen.iter().any(|e| e
            .text
            .contains("known finding(s) not yet fixed still fail here and are offered again")),
        "{}",
        texts(&seen)
    );
    assert!(cycle.proposal.as_ref().unwrap().finding.is_some());
    // Reproduced again on this base, so its reproduction is reused there.
    let replay = cycle.before.iter().find(|o| o.name == "replay").unwrap();
    assert_eq!(replay.verdict, "failed");
    assert!(
        replay.detail.contains("when it was reproduced"),
        "{}",
        replay.detail
    );
}

/// A known finding that no longer reproduces on the new base (fixed some
/// other way) is not offered: it costs no cycle, and the testing knowledge
/// learns it does not reproduce.
#[test]
fn a_known_finding_that_no_longer_fails_is_not_offered() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node.clone(), true);
    let first = exploring(&store, &repository, "Find and fix problems", budgets());
    run_to_end(setup, first, |_, _| {});
    // The Studio no longer has the defect.
    let (setup, _) = setup_with(dir.path(), &store, node, false);
    let second = exploring(&store, &repository, "Find and fix problems", budgets());
    let id = second.id.clone();
    let seen = run_to_end(setup, second, |_, _| {});
    let record = store.load(&id).unwrap();
    assert!(
        seen.iter()
            .any(|e| e.text.starts_with("A known finding no longer reproduces")),
        "{}",
        texts(&seen)
    );
    assert!(
        record.cycle().unwrap().proposal.is_none(),
        "nothing offered"
    );
    assert!(
        record
            .note
            .as_deref()
            .unwrap_or_default()
            .starts_with("Nothing new reproduced"),
        "{:?}",
        record.note
    );
    let knowledge = Knowledge::load(
        &Knowledge::file(&store, &repository),
        &Knowledge::key(&repository),
    )
    .unwrap();
    assert!(
        knowledge
            .findings
            .iter()
            .all(|f| f.state == FoundState::NotReproduced),
        "{:?}",
        knowledge
            .findings
            .iter()
            .map(|f| f.state)
            .collect::<Vec<_>>()
    );
}

/// The Operator stops a child alone: its directive ends as stopped, the
/// parent's lead goes on with that, and the parent goes on.
#[test]
fn a_child_stopped_alone_lets_its_parent_go_on() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["DELEGATE"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(
        &store,
        &repository,
        "DELEGATE-ME: find and fix problems",
        budgets(),
    );
    let id = objective.id.clone();
    let mut stopped = false;
    run_to_end(setup, objective, |entry, handle| {
        if !stopped && entry.kind == Kind::Directive && entry.text.starts_with("Delegates a child")
        {
            let child = entry
                .text
                .split(['(', ')'])
                .nth(1)
                .unwrap_or_default()
                .to_string();
            handle.send(Command::StopChild(child));
            stopped = true;
        }
    });
    assert!(stopped);
    let record = store.load(&id).unwrap();
    let directive = record
        .directives
        .iter()
        .find(|d| matches!(&d.recipient, Recipient::Child(c) if !c.is_empty()))
        .unwrap();
    assert_eq!(directive.status, DirectiveStatus::Stopped);
    assert!(
        directive
            .result
            .as_deref()
            .unwrap_or_default()
            .starts_with("stopped by the Operator")
    );
    // The parent went on: it explored and proposed after.
    let cycle = record.cycle().unwrap();
    assert_eq!(cycle.explorations.len(), 1);
    assert!(cycle.proposal.is_some());
    let ours = store.thread(&id, 0);
    assert!(
        ours.iter()
            .any(|e| e.author == Author::Operator && e.text.starts_with("Stops the child"))
    );
}

/// The Operator's messages: one sent while the lead plans waits for the
/// lead's next turn (the proposal), and is given to it there; one sent
/// while the implementer works goes to it at its next tool call. The
/// thread says where each went.
#[test]
fn the_operators_messages_go_to_the_implementer_at_work_and_otherwise_to_the_lead() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &["WAIT-MESSAGE"]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(&store, &repository, "Find and fix problems", budgets());
    let id = objective.id.clone();
    let (mut to_lead, mut to_implementer) = (false, false);
    run_to_end(setup, objective, |entry, handle| {
        let starts = |role: &str| {
            matches!(&entry.author, Author::Agent { role: r, .. } if r == role)
                && entry.text.starts_with("Starts its session")
        };
        if !to_lead && starts("lead") {
            handle.send(Command::Message("Prefer the History panel".into()));
            to_lead = true;
        }
        if !to_implementer && starts("implementer") {
            handle.send(Command::Message("Call it Archive".into()));
            to_implementer = true;
        }
    });
    let record = store.load(&id).unwrap();
    let thread = store.thread(&id, 0);
    let message = |text: &str| {
        thread
            .iter()
            .find(|e| e.kind == Kind::Human && e.text == text)
            .unwrap_or_else(|| panic!("{text}: {}", texts(&thread)))
            .clone()
    };
    let lead = message("Prefer the History panel");
    assert_eq!(lead.to.as_deref(), Some("lead"));
    assert!(record.delivered >= lead.seq);
    let proposal = record.cycle().unwrap().proposal.clone().unwrap();
    assert!(
        proposal.why.contains("Prefer the History panel"),
        "{}",
        proposal.why
    );
    let implementer = message("Call it Archive");
    assert_eq!(implementer.to.as_deref(), Some("implementer"));
    let summary = &record.cycle().unwrap().attempts[0].summary;
    assert!(summary.contains("Call it Archive"), "{summary}");
}

/// A folder for an instance that is refused before it starts.
fn dir_for_refusal() -> PathBuf {
    std::env::temp_dir().join(format!("agq-refused-{}", std::process::id()))
}

/// The Studio built from this checkout (`cargo build -p agq-studio-native`).
fn built_studio() -> Option<PathBuf> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let exe = target.join("debug").join(agq_launcher::STUDIO);
    exe.is_file().then_some(exe)
}

/// Live, with the Studio built (opens windows; no key, no model): a test
/// instance starts as one (`--test-instance`, `--control-speed instant`),
/// in the stated condition `recovered` it says which build did not start,
/// in `with an objective` it has the recorded objective in its app data,
/// and an exploration by the rules runs in a live instance whose folder is
/// removed after use.
///
/// ```text
/// cargo build -p agq-studio-native
/// cargo test -p agq-orchestrator --test exploring -- --ignored --nocapture
/// ```
#[test]
#[ignore = "opens windows: run with the Studio built (no keys needed)"]
fn live_test_instances_start_in_their_conditions_and_explore_by_the_rules() {
    use agq_orchestrator::control::{CONDITIONS, FAILED_BUILD, Flags, TestInstance};
    use agq_orchestrator::explore::{self, Changes, LiveInstance, Plan};
    let Some(exe) = built_studio() else {
        panic!("build the Studio first: cargo build -p agq-studio-native");
    };
    let flags = Flags::of(&exe);
    assert!(flags.test_instance && flags.control_speed, "{flags:?}");
    // A build without the stand-in Assistant is never started unreviewed
    // (it would read the Operator's credentials as a plain Studio).
    if !flags.stand_in {
        let refused = TestInstance::start_with(
            &exe,
            &dir_for_refusal(),
            Path::new(env!("CARGO_MANIFEST_DIR")),
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &Options {
                unreviewed: true,
                ..Options::default()
            },
        );
        assert!(
            refused.is_err_and(|e| e.contains("--assistant-stand-in")),
            "an unreviewed build without the stand-in is not started"
        );
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let sample = repository.join("models").join("url-shortener");
    let dir = tempfile::tempdir().unwrap();
    for condition in CONDITIONS {
        let folder = dir.path().join(condition.replace(' ', "-"));
        let options = Options {
            speed: Some("instant".into()),
            condition: Some(condition.to_string()),
            ..Options::default()
        };
        let mut instance =
            TestInstance::start_with(&exe, &folder, &repository, &repository, &options).unwrap();
        let mut client = instance.connect(Duration::from_secs(180)).unwrap();
        let observed = client.observe(true).unwrap();
        eprintln!(
            "{condition}: status “{}”, speed {}",
            observed["status"], observed["agents"]["speed"]
        );
        assert_eq!(observed["agents"]["speed"], "instant");
        if condition == "recovered" {
            let status = observed["status"].as_str().unwrap_or_default();
            assert!(
                status.contains(FAILED_BUILD) && status.contains("did not start"),
                "{status}"
            );
        } else {
            let store = Store::new(folder.join("session").join("objectives"));
            assert_eq!(store.list().len(), 1);
        }
        drop(client);
        drop(instance);
        assert!(!folder.exists(), "the test instance's folder is removed");
    }
    // An exploration by the rules in a live instance, as a cycle runs it.
    let folder = dir.path().join("explore");
    let mut live = LiveInstance::with(
        &exe,
        &sample,
        &folder,
        Options {
            speed: Some("instant".into()),
            ..Options::default()
        },
    );
    let plan = Plan {
        goal: "Look at the history".into(),
        way: agq_orchestrator::decide::Way::Rules,
        seed: 3,
        steps: 8,
        seconds: 300,
        // The rules spend nothing (a run stops when spend reaches budget).
        usd: 0.01,
        changes: Changes::default(),
        start: "models/url-shortener".into(),
        conversation: false,
        turn_ms: 10_000,
        stop_ms: 10_000,
    };
    let answers = agq_orchestrator::decide::Decider::default();
    let none = ModelRef::new(Provider::DeepSeek, "none");
    let deciding = explore::Deciding {
        answers: &answers,
        explorer: none.clone(),
        effort: None,
        escalation: none,
        escalation_effort: None,
    };
    let run = explore::explore(
        &mut live,
        &plan,
        &deciding,
        &Knowledge::new("live"),
        &mut || true,
    );
    eprintln!(
        "explored {} steps in {:.0} s: {} keys covered, {} finding(s), {} recoveries; {}",
        run.actions,
        run.seconds,
        run.covered.len(),
        run.findings.len(),
        run.recoveries.len(),
        run.ended
    );
    assert_eq!(run.actions, 8, "{}", run.ended);
    drop(live);
    assert!(!folder.exists(), "the live instance's folder is removed");
}

/// With W12.6's Studio: the Operator's message to an objective that is not
/// running is appended to its thread `to` the lead (as the Studio's
/// `message_objective` does); when the objective continues, the lead's next
/// session is given it, the record says it was delivered (the Studio's
/// chip then reads "given to the lead"), and the thread says so.
#[test]
fn a_message_left_while_not_running_reaches_the_lead_after_continue() {
    let Ok(node) = find_node() else {
        eprintln!("Node is not available: skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let repository = repository_with(dir.path(), &[]);
    let store = Store::new(dir.path().join("objectives"));
    let (setup, _) = setup_with(dir.path(), &store, node, true);
    let objective = exploring(&store, &repository, "Find and fix problems", budgets());
    let id = objective.id.clone();
    // Its thread as an earlier run left it, then the Operator's message.
    store
        .append_thread(
            &id,
            ThreadEntry::new(Kind::Human, Author::Operator, objective.intent.clone()),
        )
        .unwrap();
    let left = store
        .append_thread(
            &id,
            ThreadEntry::message("Look at the Archive button", "lead"),
        )
        .unwrap();
    assert_eq!(left.to.as_deref(), Some("lead"));
    let seen = run_to_end(setup, objective, |_, _| {});
    let record = store.load(&id).unwrap();
    assert!(record.delivered >= left.seq, "{}", texts(&seen));
    let plan = record
        .directives
        .iter()
        .find(|d| d.recipient == Recipient::Role("explorer".into()))
        .unwrap();
    assert!(
        plan.scope
            .instruction
            .contains("Look at the Archive button"),
        "the lead's plan took it: {}",
        plan.scope.instruction
    );
    let thread = store.thread(&id, 0);
    assert!(
        thread
            .iter()
            .any(|e| e.seq > left.seq && e.text.starts_with("Gave the lead")),
        "{}",
        texts(&thread)
    );
}
