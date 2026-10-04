//! The agents of a cycle (C-53, ROADMAP §4.16): what each role is told and
//! which tools it has. Each role is its own Claude Agent runtime session;
//! the SDK's own tools work under the role's permission policy (set by the
//! driver), Agentique's model and control tools beside them, and one tool
//! with which the role hands its result to the Orchestrator, which checks it.

use crate::record::{Attempt, Objective, Proposal};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Lead,
    Implementer,
    Reviewer,
    Evaluator,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Lead => "lead",
            Role::Implementer => "implementer",
            Role::Reviewer => "reviewer",
            Role::Evaluator => "evaluator",
        }
    }
}

pub const SUBMIT_PROPOSAL: &str = "submit_proposal";
pub const SUBMIT_IMPLEMENTATION: &str = "submit_implementation";
pub const SUBMIT_REVIEW: &str = "submit_review";
pub const SUBMIT_EVALUATION: &str = "submit_evaluation";

/// Agentique's model tools a role may call, by name.
fn model_tools(write: bool) -> Vec<&'static str> {
    let mut names = vec![
        "read_model",
        "find_elements",
        "get_problems",
        "inspect_behaviour",
        "list_scenarios",
    ];
    if write {
        names.push("apply_changes");
    }
    names
}

/// The definitions of Agentique's tools named, as the Assistant defines them.
fn agentique(names: &[&str]) -> Vec<Value> {
    agq_assistant::tools::definitions()
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| names.contains(&d["name"].as_str().unwrap_or_default()))
        .cloned()
        .collect()
}

fn submit_proposal() -> Value {
    json!({
        "name": SUBMIT_PROPOSAL,
        "description": "Hand the Orchestrator one improvement for this cycle. It is checked (at least one criterion with a deterministic check; ids unique) and then frozen: later attempts are judged by exactly these criteria.",
        "input_schema": {
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "What the improvement does, in a few words." },
                "kind": { "type": "string", "enum": ["correctness", "usability", "comprehension", "other"] },
                "why": { "type": "string", "description": "The problem it solves, with the evidence you found (file:line, an observation)." },
                "parts": { "type": "array", "items": { "type": "string" }, "description": "The parts of the self-model it affects (qualified names)." },
                "plan": { "type": "array", "items": { "type": "string" }, "description": "The steps, in order." },
                "criteria": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "statement": { "type": "string", "description": "What must be true after the change." },
                            "check": {
                                "type": "object",
                                "description": "How it is checked: {kind: command, program: [\"cargo\", \"test\", \"-p\", \"agq-x\", \"name_of_test\"]} (a test run: cargo test with -p, --test, --features, --lib and one filter, node --test <file>.test.<ext>, or python -m unittest <module>; it must run at least one test, fail before the change and pass after), {kind: observation, setup: [actions], expect: {screen, dialog, statusContains, selectionContains, control, labelContains, valueContains, enabled, anyLabelContains}} (in a test instance of the change; at least one of these keys, and no others), or {kind: judgment} (the evaluator decides from observations). Each observation criterion starts with no dialog open (the Orchestrator cancels one left open; an approval left open means the criteria after it are not run): put every action it needs in its own setup.",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["command", "observation", "judgment"] },
                                    "program": { "type": "array", "items": { "type": "string" } },
                                    "setup": { "type": "array", "items": { "type": "object" } },
                                    "expect": { "type": "object" }
                                },
                                "required": ["kind"]
                            }
                        },
                        "required": ["id", "statement", "check"]
                    }
                },
                "intended_test_changes": {
                    "type": "array",
                    "description": "Existing tests, checks or budgets the change must alter, each with why (the reviewer judges them). Adding tests needs no entry.",
                    "items": {
                        "type": "object",
                        "properties": { "path": { "type": "string" }, "why": { "type": "string" } },
                        "required": ["path", "why"]
                    }
                }
            },
            "required": ["title", "kind", "why", "parts", "plan", "criteria"],
            "additionalProperties": false
        }
    })
}

fn submit_implementation() -> Value {
    json!({
        "name": SUBMIT_IMPLEMENTATION,
        "description": "Tell the Orchestrator the implementation is ready: it commits the worktree and runs the required checks and the criteria on a clean checkout.",
        "input_schema": {
            "type": "object",
            "properties": {
                "summary": { "type": "string", "description": "What changed and why, and what you checked yourself." }
            },
            "required": ["summary"],
            "additionalProperties": false
        }
    })
}

fn submit_review() -> Value {
    json!({
        "name": SUBMIT_REVIEW,
        "description": "Your verdict on the change. Request changes for any defect, scope creep, slop or unjustified change to tests or checks; findings go to the implementer.",
        "input_schema": {
            "type": "object",
            "properties": {
                "verdict": { "type": "string", "enum": ["approve", "request_changes"] },
                "findings": { "type": "array", "items": { "type": "string" }, "description": "Concrete findings, file:line, what is wrong and why." },
                "test_changes_accepted": { "type": "boolean", "description": "Whether the changes to tests, checks or budgets that the proposal named are justified. False when there are none." }
            },
            "required": ["verdict", "findings", "test_changes_accepted"],
            "additionalProperties": false
        }
    })
}

fn submit_evaluation() -> Value {
    json!({
        "name": SUBMIT_EVALUATION,
        "description": "Each behavioural criterion's outcome in the test instance, with the observations it rests on.",
        "input_schema": {
            "type": "object",
            "properties": {
                "criteria": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "outcome": { "type": "string", "enum": ["passed", "failed", "not run"] },
                            "observations": { "type": "string", "description": "What you saw in the observations (controls, labels, values, status), not what you expected." }
                        },
                        "required": ["id", "outcome", "observations"]
                    }
                }
            },
            "required": ["criteria"],
            "additionalProperties": false
        }
    })
}

/// The tools a role has (Agentique's; the SDK's own come with the session).
pub fn tools(role: Role) -> Value {
    let mut list = match role {
        Role::Lead => {
            let mut names = model_tools(false);
            names.push("observe_app");
            agentique(&names)
        }
        Role::Implementer => agentique(&model_tools(true)),
        Role::Reviewer => agentique(&model_tools(false)),
        // The test instance has the checkout's model open (one writer at a
        // time): the evaluator works through the application.
        Role::Evaluator => agentique(&["observe_app", "act_in_app"]),
    };
    list.push(match role {
        Role::Lead => submit_proposal(),
        Role::Implementer => submit_implementation(),
        Role::Reviewer => submit_review(),
        Role::Evaluator => submit_evaluation(),
    });
    Value::Array(list)
}

const COMMON: &str = "You are one of the agents Agentique's Orchestrator runs to improve Agentique itself (decision C-53, ROADMAP §4.16). The Operator gave the objective below and watches; nobody approves your steps, so decide yourself and say what you decided. Follow the project's instructions (AGENTS.md, loaded with CLAUDE.md), above all rule 10. The model of the system changes only through Agentique's tools (apply_changes); never edit model files. You never see pixels: the application is observed as text. Deterministic checks and an independent reviewer decide whether your work is merged; your own judgment never overrides a failing check. Be concise.";

fn criteria_text(proposal: &Proposal) -> String {
    proposal
        .criteria
        .iter()
        .map(|c| {
            format!(
                "- {} — {} (check: {})",
                c.id,
                c.statement,
                serde_json::to_string(&c.check).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A proposal as the other roles read it (and the thread shows it).
pub fn proposal_text(proposal: &Proposal) -> String {
    format!(
        "Title: {}\nKind: {}\nWhy: {}\nParts: {}\nPlan:\n{}\nAcceptance criteria (frozen):\n{}\nIntended changes to tests or checks: {}",
        proposal.title,
        proposal.kind,
        proposal.why,
        proposal.parts.join(", "),
        proposal
            .plan
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        criteria_text(proposal),
        if proposal.intended_test_changes.is_empty() {
            "none".to_string()
        } else {
            proposal
                .intended_test_changes
                .iter()
                .map(|c| format!("{} ({})", c.path, c.why))
                .collect::<Vec<_>>()
                .join("; ")
        }
    )
}

fn earlier(objective: &Objective) -> String {
    let done: Vec<String> = objective
        .cycles
        .iter()
        .filter(|c| c.proposal.is_some())
        .map(|c| {
            format!(
                "- cycle {}: {} — {}",
                c.n,
                c.proposal.as_ref().map(|p| p.title.as_str()).unwrap_or(""),
                if c.adopted {
                    "adopted".to_string()
                } else {
                    c.blocker
                        .clone()
                        .unwrap_or_else(|| format!("{:?}", c.phase).to_lowercase())
                }
            )
        })
        .collect();
    if done.is_empty() {
        "none".into()
    } else {
        done.join("\n")
    }
}

/// The system prompt (appended to the SDK's development instructions).
pub fn instructions(role: Role) -> String {
    let specific = match role {
        Role::Lead => {
            "Your role: lead. Find one genuine, bounded improvement that serves the objective, and hand it over with submit_proposal. Look before you choose: the self-model (read_model; model/Agentique.sysml), the code (read and search files; you run no commands), docs/stages.md and ROADMAP §5.6 (known problems), and the running application (observe_app). Choose something small (a few files), real (evidence: a failing case, a wrong result, a confusing screen), and checkable: at least one criterion must be a command that fails before the change and passes after (usually a new test: `cargo test -p <crate> <test name>`); a usability or comprehension improvement also gets an observation or judgment criterion in the running application. Leave locked parts and Agentique's safeguards alone unless the objective names them: the code of locked parts (crates/language, crates/system-state, crates/history, crates/execution, crates/implementation, crates/launcher, crates/orchestrator, claude-agent and the Assistant's Claude Agent runtime) and the safeguards (crates/assistant/src/policy.rs and model_tools.rs, crates/studio-native/src/control, objectives.rs and panels/objectives.rs, crates/implementation/src/task.rs); a change there fails the gates. Do not repeat an earlier cycle's improvement. You work in a throwaway checkout: change nothing there."
        }
        Role::Implementer => {
            "Your role: implementer. Implement the frozen proposal in this worktree, and only it. Add tests for the criteria; keep every existing test and check (rule 10). Run what you need yourself: `cargo fmt --all`, `cargo clippy -p <crate> --all-targets --offline -- -D warnings`, `cargo test -p <crate> --offline`, and the criteria's commands (a shared CARGO_TARGET_DIR is set). Model changes go through apply_changes. When done, call submit_implementation; the Orchestrator commits and checks a clean checkout. If you are repairing, fix exactly the failures and findings listed, without weakening a check."
        }
        Role::Reviewer => {
            "Your role: independent reviewer. You did not write this change. Review it against the frozen proposal and its criteria, the check results and the evaluation below, reading the code in this checkout (you write nothing). Judge correctness, scope (nothing unrelated), simplicity and naming (ROADMAP §1.3, §8.4), whether the tests really check the criteria, and every change to tests, checks or budgets the baseline guard lists. Then call submit_review: approve only a change you would merge as it is."
        }
        Role::Evaluator => {
            "Your role: evaluator. A test instance of Agentique built from the change is running; observe_app and act_in_app operate it (not the Operator's Studio). Check each behavioural criterion (observation and judgment ones) by operating it as the Operator would, and report each with submit_evaluation: the outcome and the observations it rests on (controls, labels, values, status). If a criterion cannot be checked, say not run and why."
        }
    };
    format!("{COMMON}\n\n{specific}")
}

/// The first message of a role's session.
pub fn brief(role: Role, objective: &Objective, context: &str) -> String {
    let cycle = objective.cycle();
    let proposal = cycle.and_then(|c| c.proposal.as_ref());
    let mut text = format!(
        "Objective: {}\n\nEarlier cycles:\n{}\n",
        objective.intent,
        earlier(objective)
    );
    if let Some(proposal) = proposal
        && role != Role::Lead
    {
        text.push_str(&format!(
            "\nThe cycle's frozen proposal:\n{}\n",
            proposal_text(proposal)
        ));
    }
    if !context.is_empty() {
        text.push_str(&format!("\n{context}\n"));
    }
    text
}

/// What a repair round is told: what failed and what the reviewer found.
pub fn repair_context(attempt: Option<&Attempt>, findings: &[String]) -> String {
    let mut text = String::from("Repair round. Fix these, without weakening any check:\n");
    for failure in attempt.map(Attempt::failures).unwrap_or_default() {
        text.push_str(&format!("- failed: {failure}\n"));
    }
    if let Some(attempt) = attempt {
        for outcome in attempt
            .checks
            .iter()
            .chain(&attempt.criteria)
            .chain(&attempt.gates)
        {
            if !outcome.passed() && !outcome.detail.is_empty() {
                let detail: String = outcome
                    .detail
                    .chars()
                    .rev()
                    .take(3000)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                text.push_str(&format!("\n{}:\n{}\n", outcome.name, detail));
            }
        }
    }
    for finding in findings {
        text.push_str(&format!("- review: {finding}\n"));
    }
    text
}

/// Whether `program` is a test run a criterion may name: `cargo test` with
/// packages, test targets, features and one filter; Node's test runner on a
/// test file; Python's unittest. Nothing else runs as a criterion, so a
/// proposal cannot run a program of its choosing.
pub fn test_command(program: &[String]) -> Result<(), String> {
    let words: Vec<&str> = program.iter().map(String::as_str).collect();
    let name = |w: &str| {
        !w.is_empty()
            && !w.starts_with('-')
            && w.chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-:.".contains(c))
    };
    // A path inside the checkout: relative, never a drive, a share or a
    // parent folder.
    let path = |w: &str| {
        !w.is_empty()
            && !w.starts_with(['-', '/', '\\'])
            && !w.contains("..")
            && !w.contains(':')
            && w.chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-./\\".contains(c))
    };
    match words.as_slice() {
        ["cargo", "test", rest @ ..] => {
            let mut filter = false;
            let mut i = 0;
            while i < rest.len() {
                match rest[i] {
                    "-p" | "--package" | "--test" | "--features"
                        if rest.get(i + 1).is_some_and(|w| name(w)) =>
                    {
                        i += 2
                    }
                    "--lib" | "--doc" | "--offline" | "--locked" => i += 1,
                    "--" => {
                        let tail = &rest[i + 1..];
                        if tail.iter().all(|w| {
                            *w == "--exact"
                                || *w == "--nocapture"
                                || w.starts_with("--test-threads=")
                                || (name(w) && !filter)
                        }) {
                            return Ok(());
                        }
                        return Err(format!(
                            "`{}` after `--` is not a test option",
                            tail.join(" ")
                        ));
                    }
                    w if name(w) && !filter => {
                        filter = true;
                        i += 1;
                    }
                    w => {
                        return Err(format!(
                            "`{w}` is not an option a criterion's cargo test may use"
                        ));
                    }
                }
            }
            Ok(())
        }
        ["node", rest @ ..] => {
            let flags: Vec<&&str> = rest.iter().filter(|w| w.starts_with('-')).collect();
            let files: Vec<&&str> = rest.iter().filter(|w| !w.starts_with('-')).collect();
            let known = flags.iter().all(|f| {
                matches!(
                    **f,
                    "--test" | "--experimental-strip-types" | "--no-warnings"
                )
            });
            if known
                && flags.iter().any(|f| **f == "--test")
                && !files.is_empty()
                && files.iter().all(|f| path(f) && f.contains(".test."))
            {
                Ok(())
            } else {
                Err("a Node criterion is `node --test <file>.test.<ext>`".into())
            }
        }
        ["python", "-m", "unittest", tests @ ..]
            if !tests.is_empty() && tests.iter().all(|t| path(t)) =>
        {
            Ok(())
        }
        [] => Err("a command check without a program".into()),
        _ => Err(
            "a criterion's command is a test run: cargo test, node --test or python -m unittest"
                .into(),
        ),
    }
}

/// A proposal from `submit_proposal`'s input, checked: at least one
/// criterion with a deterministic check, unique ids, commands as words.
pub fn read_proposal(input: &Value) -> Result<Proposal, String> {
    let mut proposal: Proposal = serde_json::from_value(json!({
        "title": input["title"],
        "kind": input["kind"],
        "why": input["why"],
        "parts": input["parts"],
        "plan": input["plan"],
        "criteria": input["criteria"],
        "intendedTestChanges": input.get("intended_test_changes").cloned().unwrap_or(json!([])),
    }))
    .map_err(|e| format!("the proposal cannot be read: {e}"))?;
    proposal.title = proposal.title.trim().to_string();
    if proposal.title.is_empty() || proposal.why.trim().is_empty() {
        return Err("a proposal needs a title and why".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for criterion in &proposal.criteria {
        if !ids.insert(criterion.id.clone()) {
            return Err(format!("the criterion id {} is used twice", criterion.id));
        }
        match &criterion.check {
            crate::record::Check::Command { program } => test_command(program)
                .map_err(|problem| format!("criterion {}: {problem}", criterion.id))?,
            crate::record::Check::Observation { expect, .. } => crate::control::expectation(expect)
                .map_err(|problem| format!("criterion {}: {problem}", criterion.id))?,
            crate::record::Check::Judgment => {}
        }
    }
    if !proposal.criteria.iter().any(|c| {
        matches!(
            c.check,
            crate::record::Check::Command { .. } | crate::record::Check::Observation { .. }
        )
    }) {
        return Err(
            "at least one criterion needs a deterministic check (a command, or an observation)"
                .into(),
        );
    }
    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_role_has_its_tools_and_one_way_to_hand_over() {
        let names = |role| -> Vec<String> {
            tools(role)
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["name"].as_str().unwrap().to_string())
                .collect()
        };
        assert!(names(Role::Implementer).contains(&"apply_changes".to_string()));
        assert!(!names(Role::Reviewer).contains(&"apply_changes".to_string()));
        assert!(names(Role::Evaluator).contains(&"act_in_app".to_string()));
        assert!(!names(Role::Lead).contains(&"act_in_app".to_string()));
        assert!(names(Role::Lead).contains(&SUBMIT_PROPOSAL.to_string()));
        for role in [
            Role::Lead,
            Role::Implementer,
            Role::Reviewer,
            Role::Evaluator,
        ] {
            let submits = names(role)
                .iter()
                .filter(|n| n.starts_with("submit_"))
                .count();
            assert_eq!(submits, 1, "{role:?}");
            assert!(instructions(role).contains("C-53"));
        }
    }

    #[test]
    fn a_proposal_needs_a_deterministic_criterion_and_unique_ids() {
        let base = json!({
            "title": "Fix the gap", "kind": "correctness", "why": "It is wrong at x.rs:3",
            "parts": ["AgentiqueArchitecture::Studio"], "plan": ["Write the test", "Fix it"],
            "criteria": [{ "id": "c1", "statement": "The gap is gone", "check": { "kind": "command", "program": ["cargo", "test", "-p", "agq-x", "gap"] } }]
        });
        let proposal = read_proposal(&base).unwrap();
        assert_eq!(proposal.criteria.len(), 1);
        let mut judgment_only = base.clone();
        judgment_only["criteria"] =
            json!([{ "id": "c1", "statement": "Looks better", "check": { "kind": "judgment" } }]);
        assert!(read_proposal(&judgment_only).is_err());
        let mut twice = base.clone();
        twice["criteria"] = json!([base["criteria"][0], base["criteria"][0]]);
        assert!(read_proposal(&twice).is_err());
    }

    #[test]
    fn a_criterion_runs_tests_only() {
        let words = |w: &[&str]| w.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        for allowed in [
            &["cargo", "test", "-p", "agq-orchestrator", "a_cycle_goes"][..],
            &[
                "cargo", "test", "-p", "agq-x", "--test", "cycle", "--", "--exact", "name",
            ],
            &[
                "cargo",
                "test",
                "--offline",
                "-p",
                "agq-studio-native",
                "--features",
                "automation",
                "control::",
            ],
            &[
                "node",
                "--experimental-strip-types",
                "--no-warnings",
                "--test",
                "test/policy.test.ts",
            ],
            &["python", "-m", "unittest", "tools.test_check_architecture"],
        ] {
            assert!(test_command(&words(allowed)).is_ok(), "{allowed:?}");
        }
        for refused in [
            &["git", "push", "--force", "origin", "HEAD:main"][..],
            &["powershell", "-c", "anything"],
            &["cargo", "run", "-p", "x"],
            &["cargo", "test", "--config", "target.x.runner='cmd'"],
            &["cargo", "+nightly", "test"],
            &["cargo", "test", "-Z", "unstable-options"],
            &["cargo", "test", "a", "b"],
            &["node", "script.js"],
            &["node", "--test", "../outside.test.js"],
            &["node", "--test", r"\\attacker.example\share\x.test.js"],
            &["node", "--test", r"\Windows\x.test.js"],
            &["node", "--test", "C:/x.test.js"],
            &["python", "tools/check_architecture.py"],
            &[],
        ] {
            assert!(test_command(&words(refused)).is_err(), "{refused:?}");
        }
        let base = json!({
            "title": "Fix the gap", "kind": "correctness", "why": "x.rs:3",
            "parts": [], "plan": ["a"],
            "criteria": [{ "id": "c1", "statement": "s", "check": { "kind": "command", "program": ["git", "push", "origin", "HEAD:main"] } }]
        });
        assert!(read_proposal(&base).is_err(), "a criterion cannot push");
        let mut empty = base.clone();
        empty["criteria"] = json!([{ "id": "c1", "statement": "s", "check": { "kind": "observation", "expect": {} } }]);
        assert!(
            read_proposal(&empty).is_err(),
            "an observation must expect something"
        );
        let mut misspelt = base.clone();
        misspelt["criteria"] = json!([{ "id": "c1", "statement": "s", "check": { "kind": "observation", "expect": { "status_contains": "x" } } }]);
        assert!(
            read_proposal(&misspelt).is_err(),
            "unknown keys are refused"
        );
    }
}
