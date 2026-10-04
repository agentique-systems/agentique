//! Exploration, findings and testing knowledge (C-54, W12.4).
//!
//! The deterministic tests drive a scripted stand-in Studio (`standin`)
//! with defects planted: each invariant fires on its defect and on nothing
//! else; planted defects reproduce and a flaky one does not; reduction
//! drops what does not matter; the rules prefer what is not covered and a
//! seed changes the path; the model's answers are checked; Jev escalates in
//! order; the run recovers and keeps to its budgets.
//!
//! The live measurement compares the ways of deciding on the fixed tasks of
//! `fixtures/exploration.json` in real test instances of a debug Studio. It
//! opens windows and spends a few cents, so it is ignored by default:
//!
//! ```text
//! cargo build -p agq-studio-native
//! cargo test -p agq-orchestrator --test exploration -- --ignored --nocapture
//! ```
//!
//! It needs the TypeSafe AI and DeepSeek keys. `AGENTIQUE_STUDIO` names the
//! Studio executable (default: this workspace's debug build),
//! `AGENTIQUE_TASKS` and `AGENTIQUE_WAYS` choose tasks and ways,
//! `AGENTIQUE_STEPS` overrides the step budget, and
//! `AGENTIQUE_EVALUATION_OUT` names a file (outside the repository) for the
//! results as JSON.

use agq_orchestrator::decide::{Answers, Decider, Decision, Failure, Question, Source, Way};
use agq_orchestrator::explore::{self, Changes, Deciding, Instance, LiveInstance, Plan, Run, Step};
use agq_orchestrator::findings::{self, Check, Failed, Finding, Replay, State};
use agq_orchestrator::knowledge::Knowledge;
use agq_providers::{ModelRef, Provider};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

mod common;
mod standin;
use common::outside_the_repository;
use standin::{Chat, Defects, StandIn, Turn};

/// A model that reads the prompt's options before it answers.
type Reads = Box<dyn Fn(&str) -> String>;

/// Jev and the models, scripted: each call takes the next answer.
#[derive(Default)]
struct Scripted {
    /// Jev's choice and confidence, or its failure.
    jev: RefCell<VecDeque<Result<(String, f64), String>>>,
    /// What the model says, or its failure.
    said: RefCell<VecDeque<Result<String, String>>>,
    /// Or what the model says to a prompt, when it must read the options.
    reads: Option<Reads>,
    questions: RefCell<Vec<Question>>,
    prompts: RefCell<Vec<String>>,
}

/// The id of the first option whose description holds `text`.
fn option(prompt: &str, text: &str) -> Option<String> {
    prompt.lines().find_map(|line| {
        let (id, about) = line.strip_prefix("- ")?.split_once(": ")?;
        about.contains(text).then(|| id.to_string())
    })
}

impl Scripted {
    fn jev(self, answers: &[Result<(&str, f64), &str>]) -> Self {
        self.jev.borrow_mut().extend(
            answers
                .iter()
                .map(|a| a.map(|(c, p)| (c.to_string(), p)).map_err(str::to_string)),
        );
        self
    }

    fn said(self, answers: &[Result<&str, &str>]) -> Self {
        self.said.borrow_mut().extend(
            answers
                .iter()
                .map(|a| a.map(str::to_string).map_err(str::to_string)),
        );
        self
    }
}

impl Answers for Scripted {
    fn jev_model(&self) -> ModelRef {
        ModelRef::new(Provider::TypeSafe, "jev-1.13.0")
    }

    fn threshold(&self) -> f64 {
        0.6
    }

    fn ask_jev(&self, question: &Question) -> Result<Decision, Failure> {
        self.questions.borrow_mut().push(question.clone());
        match self.jev.borrow_mut().pop_front() {
            Some(Ok((choice, confidence))) => Ok(Decision {
                choice,
                source: Source::Jev,
                confidence: Some(confidence),
                millis: 5,
                usd: Some(0.001),
                note: String::new(),
            }),
            other => Err(Failure {
                error: other
                    .and_then(Result::err)
                    .unwrap_or_else(|| "Jev: no answer scripted".into()),
                source: Source::Jev,
                millis: 4000,
                usd: Some(0.0),
            }),
        }
    }

    fn chat(
        &self,
        _model: &ModelRef,
        _effort: Option<&str>,
        prompt: &str,
        _stop: &mut dyn FnMut() -> bool,
    ) -> Result<(String, Option<f64>), String> {
        self.prompts.borrow_mut().push(prompt.to_string());
        if let Some(reads) = &self.reads {
            return Ok((reads(prompt), Some(0.002)));
        }
        match self.said.borrow_mut().pop_front() {
            Some(Ok(text)) => Ok((text, Some(0.002))),
            Some(Err(error)) => Err(error),
            None => Err("no answer scripted".into()),
        }
    }
}

fn deciding(answers: &dyn Answers) -> Deciding<'_> {
    Deciding {
        answers,
        explorer: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
        effort: None,
        escalation: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
        escalation_effort: None,
    }
}

fn plan(way: Way, seed: u64, steps: u32) -> Plan {
    Plan {
        goal: "Look through the history and create parts".into(),
        way,
        seed,
        steps,
        seconds: 600,
        usd: 1.0,
        changes: Changes::default(),
        start: "stand-in".into(),
        conversation: false,
        // Short, so a turn that never ends takes no time in a test.
        turn_ms: 300,
        stop_ms: 300,
    }
}

fn explore_with(
    instance: &mut dyn Instance,
    plan: &Plan,
    answers: &dyn Answers,
    knowledge: &Knowledge,
) -> Run {
    explore::explore(instance, plan, &deciding(answers), knowledge, &mut || true)
}

/// A run by the rules; one that may use the Conversation where the
/// stand-in has one.
fn by_rules(instance: &mut StandIn, seed: u64, steps: u32) -> Run {
    let mut plan = plan(Way::Rules, seed, steps);
    plan.conversation = instance.chat.is_some();
    explore_with(
        instance,
        &plan,
        &Scripted::default(),
        &Knowledge::new("stand-in"),
    )
}

fn checks(run: &Run) -> BTreeSet<Check> {
    run.findings.iter().map(|f| f.check).collect()
}

/// The stand-in and the run of the first of a few fixed seeds whose path
/// meets the planted defect (a seed decides which way the rules go among
/// equally good actions).
fn found(defects: Defects) -> (StandIn, Run) {
    for seed in 1..=8 {
        let mut studio = StandIn::new(defects);
        let run = by_rules(&mut studio, seed, 80);
        if !run.findings.is_empty() {
            return (studio, run);
        }
    }
    panic!("no seed met {defects:?}");
}

#[test]
fn a_correct_studio_gives_no_findings_and_undo_is_checked() {
    for seed in 1..=4 {
        let mut studio = StandIn::new(Defects::default());
        let run = by_rules(&mut studio, seed, 60);
        assert!(run.findings.is_empty(), "seed {seed}: {:#?}", run.findings);
        assert_eq!(run.actions, 60);
        assert_eq!(run.ended, "the step budget was used");
        assert_eq!(run.unwanted.total(), 0, "{:?}", run.unwanted);
        assert!(run.notes.is_empty(), "{:?}", run.notes);
        // The explorer created parts, so it checked undo and redo.
        let checked: Vec<&str> = run
            .steps
            .iter()
            .filter(|t| t.step.by == "check")
            .map(|t| t.step.target())
            .collect();
        assert!(
            checked.starts_with(&["undo", "redo"]),
            "seed {seed}: {checked:?}"
        );
        // What an agent may not use was never tried.
        assert!(
            !studio.log.iter().any(|l| l.contains("lock-it")
                || l.contains("Message")
                || l.contains("window-close")),
            "{:?}",
            studio.log
        );
    }
}

#[test]
fn each_invariant_fires_on_its_planted_defect_and_on_nothing_else() {
    let planted: [(Defects, Check); 7] = [
        (
            Defects {
                crash: true,
                ..Defects::default()
            },
            Check::Answers,
        ),
        (
            Defects {
                unlabelled: true,
                ..Defects::default()
            },
            Check::ReadableLabels,
        ),
        (
            Defects {
                undo: true,
                ..Defects::default()
            },
            Check::UndoRestores,
        ),
        (
            Defects {
                slow: true,
                ..Defects::default()
            },
            Check::ActionTime,
        ),
        (
            Defects {
                stuck_dialog: true,
                ..Defects::default()
            },
            Check::DialogsClose,
        ),
        (
            Defects {
                internal_error: true,
                ..Defects::default()
            },
            Check::NoInternalError,
        ),
        (
            Defects {
                refuses_offered: true,
                ..Defects::default()
            },
            Check::OfferedActs,
        ),
    ];
    for (defects, check) in planted {
        // Nothing else fires, whichever way the run goes.
        for seed in 1..=8 {
            let run = by_rules(&mut StandIn::new(defects), seed, 80);
            assert!(
                checks(&run).is_subset(&BTreeSet::from([check])),
                "{defects:?}, seed {seed}: {:#?}",
                run.findings
            );
        }
        let (_, run) = found(defects);
        assert_eq!(checks(&run), BTreeSet::from([check]), "{defects:?}");
        let finding = &run.findings[0];
        assert_eq!(
            (finding.build.as_str(), finding.commit.as_str()),
            ("b1", "abc1234")
        );
        assert_eq!(finding.state, State::Open);
        // A finding's steps start at a fresh start and end where it failed.
        if check != Check::ReadableLabels {
            assert!(!finding.steps.is_empty(), "{finding:#?}");
        }
    }
    // Without a digest, undo that does nothing still shows: the model's
    // revision does not move.
    let mut studio = StandIn::new(Defects {
        undo: true,
        ..Defects::default()
    });
    studio.digest = false;
    let run = by_rules(&mut studio, 3, 80);
    assert_eq!(checks(&run), BTreeSet::from([Check::UndoRestores]));
    assert!(
        run.findings[0].message.contains("did not change"),
        "{}",
        run.findings[0].message
    );
    // An instance that ends is started again and the run goes on.
    let mut studio = StandIn::new(Defects {
        crash: true,
        ..Defects::default()
    });
    let run = by_rules(&mut studio, 3, 80);
    assert!(run.recoveries.iter().any(|r| r.kind == "restarted"));
    assert_eq!(run.actions, 80);
    assert_eq!(
        run.unwanted.ended as usize,
        run.steps.iter().filter(|t| t.outcome == "ended").count()
    );
}

#[test]
fn undo_is_not_checked_where_it_is_the_operators() {
    let mut studio = StandIn::new(Defects {
        undo: true,
        ..Defects::default()
    });
    studio.undo_refused = true;
    let run = by_rules(&mut studio, 3, 60);
    assert!(run.findings.is_empty(), "{:#?}", run.findings);
    assert_eq!(run.notes.len(), 1, "{:?}", run.notes);
    assert!(run.notes[0].contains("Operator's"), "{:?}", run.notes);
}

#[test]
fn planted_defects_reproduce_and_a_flaky_one_does_not() {
    for defects in [
        Defects {
            crash: true,
            ..Defects::default()
        },
        Defects {
            unlabelled: true,
            ..Defects::default()
        },
        Defects {
            undo: true,
            ..Defects::default()
        },
        Defects {
            slow: true,
            ..Defects::default()
        },
        Defects {
            stuck_dialog: true,
            ..Defects::default()
        },
        Defects {
            internal_error: true,
            ..Defects::default()
        },
        Defects {
            refuses_offered: true,
            ..Defects::default()
        },
    ] {
        let (mut studio, run) = found(defects);
        let mut finding = run.findings[0].clone();
        findings::reproduce(&mut studio, &mut finding, 12, &mut || false);
        assert_eq!(
            finding.state,
            State::Reproduced,
            "{defects:?}: {finding:#?}"
        );
        assert!(finding.replays.iter().take(2).all(Replay::failed));
    }
    let mut studio = StandIn::new(Defects {
        flaky: true,
        ..Defects::default()
    });
    let run = by_rules(&mut studio, 5, 80);
    assert_eq!(checks(&run), BTreeSet::from([Check::Answers]));
    let mut finding = run.findings[0].clone();
    findings::reproduce(&mut studio, &mut finding, 12, &mut || false);
    assert_eq!(finding.state, State::NotReproduced);
    assert!(finding.note.contains("the check held"), "{}", finding.note);
    assert_eq!(finding.reduced, None);
}

fn step(action: Value) -> Step {
    Step {
        key: String::new(),
        screen: "surface".into(),
        label: String::new(),
        expect: None,
        by: "explorer".into(),
        action,
    }
}

#[test]
fn reduction_keeps_only_the_steps_that_still_fail() {
    let click = |id: &str| step(json!({ "kind": "click", "control": id }));
    let steps = vec![
        click("Graph"),
        click("Architecture"),
        click("create"),
        click("dialog-cancel"),
        click("History"),
        click("checkpoint"),
        click("export"),
    ];
    let failed = Failed {
        check: Check::Answers,
        control: "export".into(),
        message: "the instance exited".into(),
        evidence: json!({}),
    };
    let mut finding = Finding::new(failed, steps, "b1", "abc1234", "stand-in");
    let mut studio = StandIn::new(Defects {
        crash: true,
        ..Defects::default()
    });
    findings::reproduce(&mut studio, &mut finding, 16, &mut || false);
    assert_eq!(finding.state, State::Reproduced, "{finding:#?}");
    let reduced: Vec<&str> = finding
        .reduced
        .as_ref()
        .expect("reduced")
        .iter()
        .map(Step::target)
        .collect();
    assert_eq!(reduced, vec!["History", "export"]);
    // Bounded: no more replays than allowed.
    assert!(
        finding.replays.len() <= 2,
        "the reduction's replays are not kept as outcomes"
    );
    let starts = studio.starts;
    let mut again = finding.clone();
    again.reduced = None;
    findings::reduce(&mut studio, &mut again, 3, &mut || false);
    assert!(studio.starts - starts <= 3);
}

#[test]
fn a_replay_fails_on_the_build_with_the_problem_and_passes_on_one_without() {
    for defects in [
        Defects {
            crash: true,
            ..Defects::default()
        },
        Defects {
            internal_error: true,
            ..Defects::default()
        },
        Defects {
            stuck_dialog: true,
            ..Defects::default()
        },
    ] {
        let (mut broken, run) = found(defects);
        let mut finding = run.findings[0].clone();
        findings::reproduce(&mut broken, &mut finding, 10, &mut || false);
        assert!(
            findings::replay(&mut broken, &finding, &mut || false).failed(),
            "{defects:?}"
        );
        let mut fixed = StandIn::new(Defects::default());
        assert_eq!(
            findings::replay(&mut fixed, &finding, &mut || false),
            Replay::Passed,
            "{defects:?}"
        );
    }
    // A step whose control is gone is no pass and no failure.
    let mut finding = Finding::new(
        Failed {
            check: Check::Answers,
            control: "export".into(),
            message: "the instance exited".into(),
            evidence: json!({}),
        },
        vec![step(json!({ "kind": "click", "control": "export" }))],
        "b1",
        "abc1234",
        "stand-in",
    );
    let mut studio = StandIn::new(Defects {
        crash: true,
        ..Defects::default()
    });
    assert!(matches!(
        findings::replay(&mut studio, &finding, &mut || false),
        Replay::Diverged { at: 1, .. }
    ));
    finding
        .steps
        .insert(0, step(json!({ "kind": "click", "control": "History" })));
    assert!(findings::replay(&mut studio, &finding, &mut || false).failed());
}

#[test]
fn the_rules_prefer_what_is_not_covered_and_a_seed_changes_the_path() {
    let keys = |run: &Run, n: usize| -> Vec<String> {
        run.steps
            .iter()
            .filter(|t| t.chosen.is_some())
            .take(n)
            .map(|t| t.step.key.clone())
            .collect()
    };
    let first = by_rules(&mut StandIn::new(Defects::default()), 1, 12);
    let second = by_rules(&mut StandIn::new(Defects::default()), 2, 12);
    // Nothing repeats while something is not covered in this run.
    let distinct: BTreeSet<String> = keys(&first, 12).into_iter().collect();
    assert_eq!(distinct.len(), 12, "{:?}", keys(&first, 12));
    // Successive runs take different paths among equally good actions.
    assert_ne!(keys(&first, 4), keys(&second, 4));
    // The same seed takes the same path.
    let again = by_rules(&mut StandIn::new(Defects::default()), 1, 12);
    assert_eq!(keys(&first, 12), keys(&again, 12));
    // What the knowledge covered comes after what it did not.
    let mut knowledge = Knowledge::new("stand-in");
    knowledge.add_run(&first);
    let next = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Rules, 1, 6),
        &Scripted::default(),
        &knowledge,
    );
    for key in keys(&next, 3) {
        assert_eq!(knowledge.count(&key), 0, "{key} was covered before");
    }
    assert_eq!(next.new_coverage.len(), next.covered.len());
    // The goal's words come first among what is not covered.
    let mut goal = plan(Way::Rules, 9, 1);
    goal.goal = "Validate the model".into();
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &goal,
        &Scripted::default(),
        &Knowledge::new("stand-in"),
    );
    assert_eq!(run.steps[0].step.target(), "validate");
}

#[test]
fn the_models_answer_must_be_an_option_with_checkable_expectations() {
    // Unreadable, then not an option: the rules decide, and both calls'
    // time and cost are kept.
    let answers =
        Scripted::default().said(&[Ok("I would click Export"), Ok(r#"{"choice": "a99"}"#)]);
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Model, 1, 1),
        &answers,
        &Knowledge::new("stand-in"),
    );
    let decision = &run.steps[0].chosen.as_ref().unwrap().decision;
    assert_eq!(decision.source, Source::Rules);
    assert!(decision.note.contains("not an option"), "{}", decision.note);
    assert_eq!(decision.usd, Some(0.004));
    // The model saw only what is valid there.
    let prompt = answers.prompts.borrow()[0].clone();
    assert!(prompt.contains("“Create part”"), "{prompt}");
    for never in ["Lock", "Message", "Close", "Undo", "Lock or unlock"] {
        assert!(
            !prompt.contains(&format!("“{never}”")),
            "{never} offered: {prompt}"
        );
    }
    // Text for a button, or an expectation nobody can check, is no answer.
    let answers = Scripted::default().said(
        &[
            r#"{"choice": "a01", "input": "x"}"#,
            r#"{"choice": "a01", "expect": {"pixels": 3}}"#,
        ]
        .map(Ok),
    );
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Model, 1, 1),
        &answers,
        &Knowledge::new("stand-in"),
    );
    assert_eq!(
        run.steps[0].chosen.as_ref().unwrap().decision.source,
        Source::Rules
    );
    // A valid answer is used; its expectation is checked after the action,
    // and one that does not hold is a finding of its own kind.
    let answers = Scripted::default().said(&[Ok(
        r#"{"choice": "a01", "expect": {"dialog": "Nothing"}, "why": "see what opens"}"#,
    )]);
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Model, 1, 1),
        &answers,
        &Knowledge::new("stand-in"),
    );
    let chosen = run.steps[0].chosen.as_ref().unwrap();
    assert_eq!(chosen.decision.source, Source::Model);
    assert_eq!(chosen.why, "see what opens");
    assert_eq!(checks(&run), BTreeSet::from([Check::Expectation]));
    assert_eq!(
        run.findings[0].steps[0].expect,
        Some(json!({ "dialog": "Nothing" }))
    );
    // Text the model chose for a field is typed, and classed for coverage.
    let answers = Scripted {
        reads: Some(Box::new(|prompt: &str| {
            match option(prompt, "into the field “Name”") {
                Some(id) => format!(r#"{{"choice": "{id}", "input": "Größe"}}"#),
                None => format!(
                    r#"{{"choice": "{}"}}"#,
                    option(prompt, "“Create part”").unwrap()
                ),
            }
        })),
        ..Scripted::default()
    };
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Model, 1, 2),
        &answers,
        &Knowledge::new("stand-in"),
    );
    let typed = &run.steps[1].step;
    assert_eq!(typed.action["text"], "Größe");
    assert!(typed.key.ends_with("|Name|fill|non-ascii"), "{}", typed.key);
}

#[test]
fn jev_escalates_to_the_model_and_the_model_to_the_rules() {
    let answers = Scripted::default()
        .jev(&[Ok(("a02", 0.9)), Ok(("a01", 0.3)), Err("Jev: timed out")])
        .said(&[
            Ok(r#"{"choice": "a03", "why": "less covered"}"#),
            Ok("??"),
            Ok("!!"),
        ]);
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Escalating, 1, 3),
        &answers,
        &Knowledge::new("stand-in"),
    );
    let sources: Vec<Source> = run
        .steps
        .iter()
        .filter_map(|t| t.chosen.as_ref())
        .map(|c| c.decision.source)
        .collect();
    assert_eq!(sources, vec![Source::Jev, Source::Escalated, Source::Rules]);
    let notes: Vec<&str> = run
        .steps
        .iter()
        .filter_map(|t| t.chosen.as_ref())
        .map(|c| c.decision.note.as_str())
        .collect();
    assert!(notes[1].contains("confidence 0.30"), "{notes:?}");
    assert!(
        notes[2].contains("timed out") && notes[2].contains("the model failed"),
        "{notes:?}"
    );
    // Jev saw at most eight options, each one valid.
    for q in answers.questions.borrow().iter() {
        assert!((2..=8).contains(&q.options.len()), "{:?}", q.options);
    }
    // Every decision's time and cost count, a failed one's too.
    assert_eq!(run.latencies[0], 5);
    assert!((5..100).contains(&run.latencies[1]), "{:?}", run.latencies);
    assert!(
        (4000..4100).contains(&run.latencies[2]),
        "{:?}",
        run.latencies
    );
    assert!(
        (run.usd - (0.001 + 0.001 + 0.002 + 0.0 + 0.004)).abs() < 1e-9,
        "{}",
        run.usd
    );
    // Jev alone: below its threshold the rules decide, and no model is asked.
    let answers = Scripted::default().jev(&[Ok(("a01", 0.2))]);
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Jev, 1, 1),
        &answers,
        &Knowledge::new("stand-in"),
    );
    assert_eq!(
        run.steps[0].chosen.as_ref().unwrap().decision.source,
        Source::Rules
    );
    assert!(answers.prompts.borrow().is_empty());
}

#[test]
fn a_run_recovers_and_keeps_to_its_budgets() {
    // A screen that changed by itself: observed again and retried.
    let mut studio = StandIn::new(Defects::default());
    studio.drift = 1;
    let run = by_rules(&mut studio, 1, 5);
    assert_eq!(run.unwanted.stale, 1);
    assert!(run.recoveries.iter().any(|r| r.kind == "observed again"));
    assert_eq!(
        run.steps
            .iter()
            .filter(|t| t.chosen.is_some() && t.outcome == "ok")
            .count(),
        5
    );
    // A dialog at the start is cancelled by rule.
    let mut studio = StandIn::new(Defects::default());
    studio.dialog_at_start = true;
    let run = by_rules(&mut studio, 1, 3);
    assert_eq!(run.recoveries[0].kind, "cancelled");
    assert_eq!(run.steps[0].step.by, "recovery");
    // A dialog that will not close: a fresh start.
    let mut studio = StandIn::new(Defects {
        stuck_dialog: true,
        ..Defects::default()
    });
    studio.dialog_at_start = true;
    let run = by_rules(&mut studio, 1, 3);
    assert!(
        run.recoveries.iter().any(|r| r.kind == "restarted"),
        "{:?}",
        run.recoveries
    );
    // Stop, time and spend.
    let mut asked = 0;
    let run = explore::explore(
        &mut StandIn::new(Defects::default()),
        &plan(Way::Rules, 1, 50),
        &deciding(&Scripted::default()),
        &Knowledge::new("stand-in"),
        &mut || {
            asked += 1;
            asked <= 3
        },
    );
    assert_eq!((run.actions, run.ended.as_str()), (3, "stopped"));
    let mut quick = plan(Way::Rules, 1, 50);
    quick.seconds = 0;
    let run = by_plan(&quick, &Scripted::default());
    assert_eq!(
        (run.actions, run.ended.as_str()),
        (0, "the time budget was used")
    );
    let mut cheap = plan(Way::Jev, 1, 50);
    cheap.usd = 0.0025;
    let answers = Scripted::default().jev(&[
        Ok(("a01", 0.9)),
        Ok(("a01", 0.9)),
        Ok(("a01", 0.9)),
        Ok(("a01", 0.9)),
    ]);
    let run = by_plan(&cheap, &answers);
    assert_eq!(
        (run.actions, run.ended.as_str()),
        (3, "the spend budget was used")
    );
    // The cancelling rule is not a way to explore.
    let run = by_plan(&plan(Way::Cancel, 1, 5), &Scripted::default());
    assert_eq!(run.actions, 0);
    assert!(run.ended.contains("not exploration"));
}

fn by_plan(plan: &Plan, answers: &Scripted) -> Run {
    explore_with(
        &mut StandIn::new(Defects::default()),
        plan,
        answers,
        &Knowledge::new("stand-in"),
    )
}

#[test]
fn a_runs_coverage_and_findings_go_into_the_testing_knowledge() {
    let mut studio = StandIn::new(Defects {
        crash: true,
        ..Defects::default()
    });
    let run = by_rules(&mut studio, 4, 40);
    let mut knowledge = Knowledge::new("stand-in");
    knowledge.add_run(&run);
    assert_eq!(knowledge.coverage.len(), run.covered.len());
    assert_eq!(knowledge.findings.len(), 1);
    let mut finding = knowledge.findings[0].clone();
    findings::reproduce(&mut studio, &mut finding, 8, &mut || false);
    knowledge.update(&finding);
    assert_eq!(knowledge.findings[0].state, State::Reproduced);
    // Fixed by a change: the next run's build replays it first and it passes.
    knowledge.fixed(&finding.identity, "c0ffee1", Some(120));
    let mut fixed = StandIn::new(Defects::default());
    for finding in knowledge
        .to_replay("b2")
        .into_iter()
        .cloned()
        .collect::<Vec<_>>()
    {
        let replay = findings::replay(&mut fixed, &finding, &mut || false);
        knowledge.replayed(&finding.identity, "b2", &replay);
    }
    assert_eq!(knowledge.findings[0].state, State::Fixed);
    assert!(knowledge.to_replay("b2").is_empty());
    let ways = knowledge.ways();
    assert_eq!(ways[&Way::Rules].runs, 1);
    assert_eq!(ways[&Way::Rules].findings, 1);
}

// The Conversation, where a test instance offers it to agents.

fn chatting(offered: bool, key: bool, turn: Turn) -> StandIn {
    let mut studio = StandIn::new(Defects::default());
    studio.chat = Some(Chat { offered, key, turn });
    studio
}

/// Whether a step sent the composer's request (Enter in it, or Send).
fn sends(t: &explore::Taken) -> bool {
    t.step.key.contains("|conversation|")
        && (t.step.action["keys"] == "enter" || t.step.action["control"] == "send")
}

/// The first of a few seeds whose run `met` what a test needs, with its
/// stand-in.
fn seeking(offered: bool, key: bool, turn: Turn, met: fn(&Run) -> bool) -> (StandIn, Run) {
    for seed in 1..=8 {
        let mut studio = chatting(offered, key, turn);
        let run = by_rules(&mut studio, seed, 80);
        if met(&run) {
            return (studio, run);
        }
    }
    panic!("no seed met it");
}

/// A request ran: its turn was waited for.
fn waited(run: &Run) -> bool {
    run.steps.iter().any(|t| t.step.by == "wait")
}

#[test]
fn the_composer_is_used_only_where_the_conversation_is_offered() {
    // The Operator's: never touched, whichever way the run goes.
    for seed in 1..=4 {
        let mut studio = chatting(false, true, Turn::Ends);
        let run = by_rules(&mut studio, seed, 60);
        assert!(run.findings.is_empty(), "{:#?}", run.findings);
        assert!(
            !studio
                .log
                .iter()
                .any(|l| l.contains("Message") || l.contains("\"send\"")),
            "{:?}",
            studio.log
        );
    }
    // Offered: requests of the fixed classes are written and sent, each
    // turn is waited for, and coverage names the request.
    let (studio, run) = seeking(true, true, Turn::Ends, waited);
    assert!(run.findings.is_empty(), "{:#?}", run.findings);
    let written: Vec<&str> = run
        .steps
        .iter()
        .filter(|t| t.step.action["control"] == "Message")
        .map(|t| t.step.key.rsplit('|').next().unwrap())
        .collect();
    assert!(!written.is_empty());
    for class in &written {
        assert!(
            explore::Request::ALL.iter().any(|r| r.name() == *class),
            "{class}"
        );
    }
    let sent = run.steps.iter().filter(|t| sends(t)).count();
    assert!(
        run.steps
            .iter()
            .any(|t| t.step.key.contains("Message=") && sends(t)),
        "sending names what is sent"
    );
    let waits = run.steps.iter().filter(|t| t.step.by == "wait").count();
    assert!(
        waits >= 1 && waits <= sent,
        "{waits} waits for {sent} sends"
    );
    // The turn was observed, not waited for with the Studio's own `idle`
    // (which also waits for runs, tasks and builds).
    assert!(!studio.log.iter().any(|l| l.contains("\"wait\"")));
    // What the instance's Assistant spent counts toward the run's budget.
    assert!(run.assistant_usd >= 0.01, "{}", run.assistant_usd);
    assert!(run.usd >= run.assistant_usd);
}

#[test]
fn a_turn_that_never_ends_is_a_finding_and_stop_must_end_it() {
    let (mut studio, run) = seeking(true, true, Turn::Never, waited);
    assert_eq!(
        checks(&run),
        BTreeSet::from([Check::TurnEnds]),
        "{:#?}",
        run.findings
    );
    // Stop was pressed and ended it.
    assert!(
        run.steps
            .iter()
            .any(|t| t.step.action["control"] == "stop" && t.step.by == "check")
    );
    let mut finding = run.findings[0].clone();
    assert!(finding.message.contains("300 ms"), "{}", finding.message);
    findings::reproduce(&mut studio, &mut finding, 8, &mut || false);
    assert_eq!(finding.state, State::Reproduced, "{finding:#?}");
    assert!(finding.note.contains("Assistant"), "{}", finding.note);
    // A turn that Stop does not end either: both, and a fresh start.
    let (_, run) = seeking(true, true, Turn::NeverStops, waited);
    assert_eq!(
        checks(&run),
        BTreeSet::from([Check::TurnEnds, Check::TurnStops]),
        "{:#?}",
        run.findings
    );
    assert!(run.recoveries.iter().any(|r| r.kind == "restarted"));
}

#[test]
fn an_assistant_without_a_key_is_a_condition_of_the_run_not_a_finding() {
    let (_, run) = seeking(true, false, Turn::Ends, |r| !r.conditions.is_empty());
    assert!(run.findings.is_empty(), "{:#?}", run.findings);
    assert_eq!(run.conditions.len(), 1, "{:?}", run.conditions);
    assert!(
        run.conditions[0].contains("needs a key"),
        "{:?}",
        run.conditions
    );
    // Nothing ran, so nothing was waited for.
    assert!(!run.steps.iter().any(|t| t.step.by == "wait"));
}

#[test]
fn an_expectation_about_the_reply_is_recorded_never_a_finding() {
    // The model writes a request, sends it, and expects words in the reply
    // and a new part in view: the reply's part is a note; the rest is
    // checked.
    let answers = Scripted {
        reads: Some(Box::new(|prompt: &str| {
            if let Some(id) = option(prompt, "“Send”") {
                format!(
                    r#"{{"choice": "{id}", "expect": {{"replyContains": "Gauge was added", "anyLabelContains": "Gauge"}}}}"#
                )
            } else {
                let id = option(prompt, "into the Conversation").unwrap();
                format!(r#"{{"choice": "{id}", "input": "Add a part named Gauge to the model."}}"#)
            }
        })),
        ..Scripted::default()
    };
    let mut studio = chatting(true, true, Turn::Ends);
    let mut talking = plan(Way::Model, 1, 3);
    talking.conversation = true;
    let run = explore_with(&mut studio, &talking, &answers, &Knowledge::new("stand-in"));
    let typed = &run.steps[0].step;
    assert!(typed.key.ends_with("|fill|written"), "{}", typed.key);
    // Sending started the turn; its end was waited for, and what was
    // expected of it was checked then, on the wait: the part the Assistant
    // added is a label in view (an outline row).
    assert!(sends(&run.steps[1]), "{:?}", run.steps[1].step);
    assert_eq!(run.steps[1].step.expect, None);
    assert_eq!(run.steps[2].step.by, "wait");
    assert!(run.steps[2].step.expect.is_some());
    assert!(run.findings.is_empty(), "{:#?}", run.findings);
    // The reply did not say what was expected: recorded as a note.
    assert!(
        run.notes
            .iter()
            .any(|n| n.contains("Gauge was added") && n.contains("not a finding")),
        "{:?}",
        run.notes
    );
    // What can be checked deterministically and fails is a finding on the
    // wait for the turn; its replay sends the same request and re-checks it
    // there, and says the reply's wording is not re-checked.
    let answers = Scripted {
        reads: Some(Box::new(|prompt: &str| {
            if let Some(id) = option(prompt, "“Send”") {
                format!(r#"{{"choice": "{id}", "expect": {{"anyLabelContains": "Gauge2"}}}}"#)
            } else {
                let id = option(prompt, "into the Conversation").unwrap();
                format!(r#"{{"choice": "{id}", "input": "Add a part named Gauge to the model."}}"#)
            }
        })),
        ..Scripted::default()
    };
    let mut studio = chatting(true, true, Turn::Ends);
    let mut talking = plan(Way::Model, 1, 2);
    talking.conversation = true;
    let run = explore_with(&mut studio, &talking, &answers, &Knowledge::new("stand-in"));
    assert_eq!(checks(&run), BTreeSet::from([Check::Expectation]));
    let mut finding = run.findings[0].clone();
    assert_eq!(finding.steps.last().unwrap().by, "wait");
    findings::reproduce(&mut studio, &mut finding, 4, &mut || false);
    assert_eq!(finding.state, State::Reproduced, "{finding:#?}");
    assert!(
        finding.note.contains("never the reply's wording"),
        "{}",
        finding.note
    );
}

#[test]
fn a_hang_and_an_exit_are_one_finding_and_never_pass_a_replay() {
    // History's Freeze leaves the instance running and silent.
    let (mut studio, run) = found(Defects {
        hang: true,
        ..Defects::default()
    });
    assert_eq!(checks(&run), BTreeSet::from([Check::Answers]));
    let mut finding = run.findings[0].clone();
    assert_eq!(finding.message, findings::ENDED);
    assert_eq!(finding.evidence["exited"], false, "it hung");
    findings::reproduce(&mut studio, &mut finding, 8, &mut || false);
    assert_eq!(finding.state, State::Reproduced, "{finding:#?}");
    // A build where the same step exits instead fails the replay too.
    let mut exits = StandIn::new(Defects {
        crash: true,
        ..Defects::default()
    });
    let mut crashing = finding.clone();
    crashing.reduced = None;
    crashing.steps = vec![
        step(json!({ "kind": "click", "control": "History" })),
        step(json!({ "kind": "click", "control": "export" })),
    ];
    crashing.control = "export".into();
    crashing.identity = findings::identity(Check::Answers, "export", findings::ENDED);
    assert!(findings::replay(&mut exits, &crashing, &mut || false).failed());
}

#[test]
fn an_instance_that_ends_on_undo_is_found_with_the_undo() {
    let (mut studio, run) = found(Defects {
        undo_crash: true,
        ..Defects::default()
    });
    assert_eq!(checks(&run), BTreeSet::from([Check::Answers]));
    let mut finding = run.findings[0].clone();
    let last = finding.steps.last().unwrap();
    assert_eq!((last.target(), last.by.as_str()), ("undo", "check"));
    findings::reproduce(&mut studio, &mut finding, 8, &mut || false);
    assert_eq!(finding.state, State::Reproduced, "{finding:#?}");
}

#[test]
fn a_slow_long_fill_is_not_taken_for_a_hang_or_a_slow_action() {
    let mut studio = chatting(true, true, Turn::Ends);
    studio.slow_typing = true;
    // Every request, the long one (about 2,500 characters, 75 s at 30 ms a
    // character) among them, by the rules over a few seeds.
    for seed in 1..=4 {
        let run = by_rules(&mut studio, seed, 60);
        assert!(run.findings.is_empty(), "{:#?}", run.findings);
    }
    assert!(studio.log.iter().any(|l| l.contains("Describe every part")));
}

#[test]
fn the_conversation_is_left_alone_unless_the_run_may_use_it() {
    let mut studio = chatting(true, true, Turn::Ends);
    let run = explore_with(
        &mut studio,
        &plan(Way::Rules, 1, 60),
        &Scripted::default(),
        &Knowledge::new("stand-in"),
    );
    assert!(run.findings.is_empty());
    assert!(
        !studio
            .log
            .iter()
            .any(|l| l.contains("Message") || l.contains("\"send\"")),
        "{:?}",
        studio.log
    );
}

#[test]
fn findings_are_counted_once_each() {
    // An internal error stays in the status line: one finding, not one for
    // every action after it.
    let (_, run) = found(Defects {
        internal_error: true,
        ..Defects::default()
    });
    assert_eq!(run.findings.len(), 1, "{:#?}", run.findings);
    assert_eq!(run.findings[0].control, "");
    // The unlabelled button: one finding, however often it is seen.
    let (_, run) = found(Defects {
        unlabelled: true,
        ..Defects::default()
    });
    assert_eq!(run.findings.len(), 1, "{:#?}", run.findings);
}

#[test]
fn escapes_out_of_a_dead_end_count_and_end_in_a_fresh_start() {
    // A stand-in that offers only Escape and refuses it as the Operator's:
    // a dead end, left by Escape (refused too), then by fresh starts.
    struct Locked {
        starts: u32,
        acts: u32,
    }
    impl Instance for Locked {
        fn observe(&mut self) -> Result<Value, String> {
            Ok(json!({
                "screen": "surface", "screenRevision": 1, "dialog": null, "approval": null,
                "palette": null, "status": "", "controls": [], "commands": [],
                "identity": { "build": "b1", "commit": "c" }
            }))
        }
        fn act(&mut self, _: &explore::Act) -> Result<Value, String> {
            self.acts += 1;
            // Escape, refused every time, so nothing valid is left.
            Ok(json!({ "ok": false, "error": "refused: nothing here is an agent's" }))
        }
        fn restart(&mut self, _: &mut dyn FnMut() -> bool) -> Result<(), String> {
            self.starts += 1;
            Ok(())
        }
        fn alive(&mut self) -> bool {
            true
        }
        fn folder(&self) -> Option<PathBuf> {
            None
        }
    }
    let mut locked = Locked { starts: 0, acts: 0 };
    let run = explore_with(
        &mut locked,
        &plan(Way::Rules, 1, 30),
        &Scripted::default(),
        &Knowledge::new("stand-in"),
    );
    // It ends: by its step budget, or by giving up on an instance that
    // keeps leaving it nothing to do; never in a loop.
    assert!(locked.acts <= 30, "{}", locked.acts);
    assert!(
        locked.starts > 1,
        "Escapes without progress lead to a fresh start"
    );
    assert!(
        run.ended == "the step budget was used" || run.ended.contains("keeps ending"),
        "{}",
        run.ended
    );
}

#[test]
fn a_run_stops_during_a_long_turn_and_out_of_time() {
    struct Stopping(u32);
    impl explore::Supervisor for Stopping {
        fn go_on(&mut self) -> bool {
            true
        }
        fn stopped(&mut self) -> bool {
            self.0 += 1;
            self.0 > 3
        }
    }
    let mut studio = chatting(true, true, Turn::Never);
    let mut long = plan(Way::Rules, 1, 80);
    long.conversation = true;
    long.turn_ms = 600_000;
    let started = std::time::Instant::now();
    let mut supervisor = Stopping(0);
    let run = explore::explore(
        &mut studio,
        &long,
        &deciding(&Scripted::default()),
        &Knowledge::new("stand-in"),
        &mut supervisor,
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(run.ended, "stopped", "{}", run.ended);
}

#[test]
fn a_decision_of_unknown_cost_counts_at_the_most_it_could_have_cost() {
    struct Unpriced;
    impl Answers for Unpriced {
        fn jev_model(&self) -> ModelRef {
            ModelRef::new(Provider::TypeSafe, "jev-1.13.0")
        }
        fn threshold(&self) -> f64 {
            0.6
        }
        fn ask_jev(&self, _: &Question) -> Result<Decision, Failure> {
            unreachable!()
        }
        fn chat(
            &self,
            _: &ModelRef,
            _: Option<&str>,
            _: &str,
            _: &mut dyn FnMut() -> bool,
        ) -> Result<(String, Option<f64>), String> {
            // Sent, and failed: its cost is unknown.
            Err("the connection was reset".into())
        }
    }
    let mut cheap = plan(Way::Model, 1, 50);
    cheap.usd = 0.05;
    let run = explore_with(
        &mut StandIn::new(Defects::default()),
        &cheap,
        &Unpriced,
        &Knowledge::new("stand-in"),
    );
    assert!(run.unpriced >= 1);
    assert!(run.usd > 0.0, "unknown is not free");
    // deepseek-flash: a prompt and 8,000 tokens of output, twice, is about
    // two cents; the budget stops the run within a few decisions.
    assert_eq!(run.ended, "the spend budget was used");
    assert!(run.actions < 10, "{}", run.actions);
}

// The fixed exploration tasks and the live measurement.

struct Task {
    id: String,
    split: String,
    goal: String,
    start: PathBuf,
    steps: u32,
    regions: Vec<String>,
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tasks() -> Vec<Task> {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/exploration.json")).expect("JSON");
    value["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .map(|t| Task {
            id: t["id"].as_str().unwrap().to_string(),
            split: t["split"].as_str().unwrap().to_string(),
            goal: t["goal"].as_str().unwrap().to_string(),
            start: repository().join(t["start"].as_str().unwrap()),
            steps: t["steps"].as_u64().unwrap() as u32,
            regions: t["regions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap().to_string())
                .collect(),
        })
        .collect()
}

#[test]
fn the_exploration_tasks_are_fixed_and_held_out_from_tuning() {
    let tasks = tasks();
    let ids: BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids.len(), tasks.len(), "ids are unique");
    let splits: BTreeSet<&str> = tasks.iter().map(|t| t.split.as_str()).collect();
    assert_eq!(splits, BTreeSet::from(["held-out", "tuning"]));
    for task in &tasks {
        assert!(
            !task.goal.is_empty() && task.steps > 0 && !task.regions.is_empty(),
            "{}",
            task.id
        );
        let has_model = std::fs::read_dir(&task.start)
            .unwrap_or_else(|e| panic!("{}: {e}", task.start.display()))
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "sysml"));
        assert!(has_model, "{}: no model files", task.id);
    }
}

fn studio() -> PathBuf {
    if let Some(path) = std::env::var_os("AGENTIQUE_STUDIO") {
        return PathBuf::from(path);
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repository().join("target"));
    target.join("debug").join(if cfg!(windows) {
        "agq-studio-native.exe"
    } else {
        "agq-studio-native"
    })
}

fn chosen(variable: &str) -> Option<BTreeSet<String>> {
    let text = std::env::var(variable).ok()?;
    Some(
        text.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

/// What one way did on a split.
#[derive(Default)]
struct Score {
    runs: u32,
    useful: usize,
    reached: usize,
    regions: usize,
    unwanted: u32,
    found: usize,
    reproduced: usize,
    expectations: usize,
    latencies: Vec<u64>,
    usd: f64,
    unpriced: u32,
}

impl Score {
    fn percentile(&self, p: f64) -> u64 {
        explore::percentile(&self.latencies, p)
    }
}

#[test]
#[ignore = "live: opens windows, needs the TypeSafe AI and DeepSeek keys, spends a few cents"]
fn live_exploration_compared_by_way_of_deciding() {
    let exe = studio();
    assert!(exe.is_file(), "build the Studio first: {}", exe.display());
    let tasks_wanted = chosen("AGENTIQUE_TASKS");
    let ways_wanted = chosen("AGENTIQUE_WAYS");
    let steps: Option<u32> = std::env::var("AGENTIQUE_STEPS")
        .ok()
        .and_then(|s| s.parse().ok());
    let decider = Decider {
        model: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
        ..Decider::default()
    };
    let deciding = Deciding {
        answers: &decider,
        explorer: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
        effort: Some("low".into()),
        escalation: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
        escalation_effort: Some("low".into()),
    };
    let base = std::env::temp_dir().join(format!("agq-explore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut scores: BTreeMap<(Way, String), Score> = BTreeMap::new();
    let mut rows = Vec::new();
    let mut spent = 0.0;
    for task in tasks() {
        if tasks_wanted.as_ref().is_some_and(|w| !w.contains(&task.id)) {
            continue;
        }
        // The same start and seed for every way: they differ only in how
        // they decide.
        for way in [Way::Rules, Way::Jev, Way::Model, Way::Escalating] {
            if ways_wanted
                .as_ref()
                .is_some_and(|w| !w.contains(&format!("{way:?}").to_lowercase()))
            {
                continue;
            }
            assert!(spent < 0.5, "the measurement's spend passed $0.50: stopped");
            let folder = base.join(format!("{}-{way:?}", task.id).to_lowercase());
            let mut instance = LiveInstance::new(&exe, &task.start, &folder);
            let plan = Plan {
                goal: task.goal.clone(),
                way,
                seed: 17,
                steps: steps.unwrap_or(task.steps),
                seconds: 900,
                usd: 0.15,
                changes: Changes::default(),
                start: task
                    .start
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                // Nothing to the instance's Assistant: it has no key of its
                // own yet (W12.5).
                conversation: false,
                turn_ms: findings::TURN_BUDGET_MS,
                stop_ms: findings::STOP_BUDGET_MS,
            };
            let knowledge = Knowledge::new("agentique");
            let mut run =
                explore::explore(&mut instance, &plan, &deciding, &knowledge, &mut || true);
            // Each finding reproduced (a few, within a bounded number of
            // replays each).
            for finding in run.findings.iter_mut().take(4) {
                findings::reproduce(&mut instance, finding, 6, &mut || false);
            }
            drop(instance);
            let reached: Vec<&String> = task
                .regions
                .iter()
                .filter(|r| run.areas.contains(*r))
                .collect();
            let score = scores.entry((way, task.split.clone())).or_default();
            score.runs += 1;
            score.useful += run.new_coverage.len();
            score.reached += reached.len();
            score.regions += task.regions.len();
            score.unwanted += run.unwanted.total();
            score.found += run.findings.len();
            score.reproduced += run
                .findings
                .iter()
                .filter(|f| f.state == State::Reproduced)
                .count();
            score.expectations += run
                .findings
                .iter()
                .filter(|f| f.check == Check::Expectation)
                .count();
            score.latencies.extend(&run.latencies);
            score.usd += run.usd;
            score.unpriced += run.unpriced;
            spent += run.usd;
            eprintln!(
                "{} {way:?}: {} steps, {} new keys, reached {reached:?}, unwanted {:?}, findings {:?}, ${:.4}, ended: {}",
                task.id,
                run.actions,
                run.new_coverage.len(),
                run.unwanted,
                run.findings
                    .iter()
                    .map(|f| (f.identity.clone(), f.state))
                    .collect::<Vec<_>>(),
                run.usd,
                run.ended
            );
            rows.push(json!({ "task": task.id, "split": task.split, "way": way, "reached": reached, "run": run }));
            let _ = std::fs::remove_dir_all(&folder);
        }
    }
    let mut summary = Vec::new();
    eprintln!(
        "way        split     runs useful progress unwanted findings(reproduced, expectation) p50ms p95ms usd"
    );
    for ((way, split), s) in &scores {
        let line = json!({
            "way": way, "split": split, "runs": s.runs, "usefulCoverage": s.useful,
            "progress": format!("{}/{}", s.reached, s.regions), "unwanted": s.unwanted,
            "findings": s.found, "reproduced": s.reproduced, "expectationFindings": s.expectations,
            "latencyP50Ms": s.percentile(0.5), "latencyP95Ms": s.percentile(0.95),
            "usd": s.usd, "unpriced": s.unpriced,
        });
        eprintln!(
            "{:<10} {:<9} {:>4} {:>6} {:>8} {:>8} {:>3} ({}, {}) {:>6} {:>6} {:.4}",
            format!("{way:?}"),
            split,
            s.runs,
            s.useful,
            format!("{}/{}", s.reached, s.regions),
            s.unwanted,
            s.found,
            s.reproduced,
            s.expectations,
            s.percentile(0.5),
            s.percentile(0.95),
            s.usd
        );
        summary.push(line);
    }
    if let Some(path) = outside_the_repository("AGENTIQUE_EVALUATION_OUT") {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&json!({ "summary": summary, "runs": rows })).unwrap(),
        )
        .unwrap();
    }
    let _ = std::fs::remove_dir_all(&base);
    // A measurement, not a check: each way ran.
    assert!(!scores.is_empty());
}
