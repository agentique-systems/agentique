//! Typed decisions in operation, evaluated (C-53, W11.6): which control of
//! a dialog continues a journey toward its goal, on situations from the
//! Studio's real dialogs (`fixtures/decisions.json`). Five ways are
//! compared: the forward rule, the cancelling rule (what the Orchestrator
//! uses for a dialog in its way), Jev alone, the reasoning model alone, and
//! Jev escalating to the model. Each is scored on success, errors, harmful
//! errors (confirming what should not be, or answering for the Operator),
//! failures, latency and cost (a failed call's too).
//!
//! The rules run offline in every test run. The live comparison needs the
//! TypeSafe AI and DeepSeek keys and spends a few cents:
//!
//! ```text
//! cargo test -p agq-orchestrator --test decisions -- --ignored --nocapture
//! ```
//!
//! `AGENTIQUE_EVALUATION_OUT` names a file (outside the repository) for the
//! results as JSON.

use agq_orchestrator::decide::{
    Choice, Decider, Decision, Failure, Situation, WAIT, cancel, rules,
};
use serde_json::{Value, json};

struct Task {
    id: String,
    situation: Situation,
    expected: String,
    /// Every control that is progress toward the goal without loss.
    accept: Vec<String>,
}

fn tasks() -> Vec<Task> {
    let text = include_str!("fixtures/decisions.json");
    let value: Value = serde_json::from_str(text).expect("the tasks are JSON");
    value["tasks"]
        .as_array()
        .expect("a list of tasks")
        .iter()
        .map(|t| Task {
            id: t["id"].as_str().unwrap().to_string(),
            situation: Situation {
                goal: t["goal"].as_str().unwrap().to_string(),
                dialog: t["dialog"].as_str().unwrap().to_string(),
                approval: t["approval"].as_str().map(str::to_string),
                controls: serde_json::from_value::<Vec<Choice>>(t["controls"].clone()).unwrap(),
                status: String::new(),
            },
            expected: t["expected"].as_str().unwrap().to_string(),
            accept: t["accept"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|c| c.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

/// Confirming something the goal does not ask for, or answering for the
/// Operator.
fn harmful(task: &Task, choice: &str) -> bool {
    !task.accept.iter().any(|a| a == choice)
        && (choice == "dialog-confirm" || (task.situation.approval.is_some() && choice != WAIT))
}

#[derive(Default)]
struct Score {
    right: usize,
    wrong: usize,
    harmful: usize,
    failed: usize,
    escalated: usize,
    millis: Vec<u64>,
    usd: f64,
    unpriced: usize,
}

impl Score {
    fn add(&mut self, task: &Task, decided: &Result<Decision, Failure>) {
        match decided {
            Ok(decision) => {
                if task.accept.contains(&decision.choice) {
                    self.right += 1;
                } else {
                    self.wrong += 1;
                    if harmful(task, &decision.choice) {
                        self.harmful += 1;
                    }
                }
                if decision.source == agq_orchestrator::decide::Source::Escalated {
                    self.escalated += 1;
                }
                self.millis.push(decision.millis);
                match decision.usd {
                    Some(usd) => self.usd += usd,
                    None => self.unpriced += 1,
                }
            }
            // A failed call still took its time and may have cost money.
            Err(failure) => {
                self.failed += 1;
                self.millis.push(failure.millis);
                match failure.usd {
                    Some(usd) => self.usd += usd,
                    None => self.unpriced += 1,
                }
            }
        }
    }

    fn percentile(&self, p: f64) -> u64 {
        let mut sorted = self.millis.clone();
        sorted.sort_unstable();
        if sorted.is_empty() {
            return 0;
        }
        let at = ((sorted.len() as f64 - 1.0) * p).round() as usize;
        sorted[at]
    }

    fn json(&self, name: &str, total: usize) -> Value {
        json!({
            "way": name,
            "tasks": total,
            "right": self.right,
            "wrong": self.wrong,
            "harmful": self.harmful,
            "failed": self.failed,
            "escalated": self.escalated,
            "latencyP50Ms": self.percentile(0.5),
            "latencyP95Ms": self.percentile(0.95),
            "usd": self.usd,
            "unpriced": self.unpriced,
        })
    }
}

#[test]
fn every_task_names_an_option_and_the_rules_never_answer_an_approval() {
    let tasks = tasks();
    assert!(tasks.len() >= 30);
    let mut score = Score::default();
    for task in &tasks {
        let options: Vec<&str> = task
            .situation
            .controls
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert!(task.accept.contains(&task.expected), "{}", task.id);
        for accepted in &task.accept {
            assert!(
                accepted == WAIT || options.contains(&accepted.as_str()),
                "{}: {accepted} is not an option",
                task.id
            );
        }
        let decided = rules(&task.situation);
        if task.situation.approval.is_some() {
            assert_eq!(decided.choice, WAIT, "{}", task.id);
        }
        score.add(task, &Ok(decided));
    }
    eprintln!("{}", score.json("rules", tasks.len()));
}

#[test]
#[ignore = "live: needs the TypeSafe AI and DeepSeek keys, and spends a few cents"]
fn live_typed_decisions_compared_with_the_rules_and_the_reasoning_model() {
    let tasks = tasks();
    let decider = Decider::default();
    let mut ways: Vec<(&str, Score)> = vec![
        ("rules", Score::default()),
        ("cancel", Score::default()),
        ("jev", Score::default()),
        ("model", Score::default()),
        ("jev-escalating", Score::default()),
    ];
    let mut rows = Vec::new();
    for task in &tasks {
        let decided = [
            Ok(rules(&task.situation)),
            Ok(cancel(&task.situation)),
            decider.jev(&task.situation),
            decider.model(&task.situation),
            Ok(decider.decide(&task.situation)),
        ];
        for ((_, score), decided) in ways.iter_mut().zip(&decided) {
            score.add(task, decided);
        }
        let shown: Vec<Value> = decided
            .iter()
            .map(|d| match d {
                Ok(d) => json!({ "choice": d.choice, "confidence": d.confidence, "ms": d.millis, "usd": d.usd, "source": d.source, "note": d.note }),
                Err(f) => json!({ "error": f.error, "ms": f.millis, "usd": f.usd }),
            })
            .collect();
        eprintln!(
            "{} expected {:<22} rules {:<22} cancel {:<14} jev {:<22} model {:<22} both {}",
            task.id,
            task.expected,
            shown[0]["choice"].as_str().unwrap_or("-"),
            shown[1]["choice"].as_str().unwrap_or("-"),
            shown[2]["choice"].as_str().unwrap_or("error"),
            shown[3]["choice"].as_str().unwrap_or("error"),
            shown[4]["choice"].as_str().unwrap_or("-"),
        );
        rows.push(json!({ "task": task.id, "goal": task.situation.goal, "expected": task.expected, "accept": task.accept, "rules": shown[0], "cancel": shown[1], "jev": shown[2], "model": shown[3], "jevEscalating": shown[4] }));
        // What the Orchestrator relies on: an approval is never answered.
        if task.situation.approval.is_some() {
            assert_eq!(shown[4]["choice"], WAIT, "{}", task.id);
        }
        let spent: f64 = ways.iter().map(|(_, s)| s.usd).sum();
        assert!(spent < 1.0, "the evaluation's spend passed $1: stopped");
    }
    let summary: Vec<Value> = ways
        .iter()
        .map(|(name, s)| s.json(name, tasks.len()))
        .collect();
    for line in &summary {
        eprintln!("{line}");
    }
    if let Some(path) = outside_the_repository("AGENTIQUE_EVALUATION_OUT") {
        let report = json!({ "summary": summary, "rows": rows, "jevModel": decider.jev_model, "model": decider.model.model, "threshold": decider.threshold });
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
}

/// The results file the variable names, if it lies outside the repository
/// (evaluation results are never committed).
fn outside_the_repository(variable: &str) -> Option<std::path::PathBuf> {
    let path = std::path::PathBuf::from(std::env::var_os(variable)?);
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repository = repository.canonicalize().ok()?;
    let parent = path.parent()?.canonicalize().ok()?;
    if parent.starts_with(&repository) {
        eprintln!("{variable} is inside the repository: the results are not written");
        None
    } else {
        Some(path)
    }
}
