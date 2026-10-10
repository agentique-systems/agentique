//! The agents of a cycle (C-53, ROADMAP §4.16): what each role is told and
//! which tools it has. Each role is its own Claude Agent runtime session;
//! the SDK's own tools work under the role's permission policy (set by the
//! driver), Agentique's model and control tools beside them, and one tool
//! with which the role hands its result to the Orchestrator, which checks it.

use crate::explore::Target;
use crate::findings::{Disposition, DispositionKind};
use crate::record::{Attempt, Objective, Proposal};
use crate::traceability::{self, Elements, Resolution};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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
pub const SUBMIT_EXPLORATION: &str = "submit_exploration";
pub const DELEGATE: &str = "delegate";
pub const SUBMIT_IMPLEMENTATION: &str = "submit_implementation";
pub const SUBMIT_REVIEW: &str = "submit_review";
pub const SUBMIT_EVALUATION: &str = "submit_evaluation";
pub const ADJUDICATE_FINDING: &str = "adjudicate_finding";

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
        "description": "Hand the Orchestrator one improvement for this cycle. It is checked (at least one criterion with a deterministic check; ids unique; `serves` and `parts` resolved in the base commit's model) and then frozen: later attempts are judged by exactly these criteria.",
        "input_schema": {
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "What the improvement does, in a few words." },
                "kind": { "type": "string", "enum": ["correctness", "usability", "comprehension", "other"] },
                "why": { "type": "string", "description": "The problem it solves, with the evidence you found (file:line, an observation)." },
                "serves": { "type": "array", "minItems": 1, "items": { "type": "string" }, "description": "The requirements of the project's model the change serves, by qualified name (a requirement def or usage, such as AgentiqueArchitecture::GatesDecide): the engineering capability or root requirement it is for. Each must be a requirement of the base commit's model." },
                "benefit": { "type": "string", "description": "The benefit the Operator is expected to see, in one or two sentences." },
                "complexity": { "type": "string", "description": "Its effect on root complexity, reuse and dependencies: what it adds, removes or generalises, and whether a root part, a dependency or a crate changes." },
                "parts": { "type": "array", "items": { "type": "string" }, "description": "The elements and contracts of the model it affects (qualified names), each an element of the base commit's model; for elements it creates, the element that will own them. The review compares them with what the commit changes." },
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
                                "description": "How it is checked: {kind: command, program: [\"cargo\", \"test\", \"-p\", \"agq-x\", \"name_of_test\"]} (a test run: cargo test with -p, --test, --features, --lib and one filter, node --test <file>.test.<ext>, or python -m unittest <module>; it must run at least one test, fail before the change and pass after; on the base the Orchestrator brings over the change's new and changed test files, so a test that shows the defect there is one in a test file that compiles against the base), {kind: observation, setup: [actions], expect: {screen, dialog, statusContains, selectionContains, control, labelContains, valueContains, enabled, anyLabelContains}, condition?} (in a test instance, on the base and on the change; at least one of these keys, and no others; `condition` starts it in a stated condition: `recovered` after a build that did not start, or `with an objective` recorded with its thread), or {kind: judgment} (the evaluator decides from observations; never evidence). Each observation criterion starts with no dialog open (the Orchestrator cancels one left open; an approval left open means the criteria after it are not run): put every action it needs in its own setup.",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["command", "observation", "judgment"] },
                                    "program": { "type": "array", "items": { "type": "string" } },
                                    "setup": { "type": "array", "items": { "type": "object" } },
                                    "expect": { "type": "object" },
                                    "condition": { "type": "string", "enum": ["recovered", "with an objective"] }
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
                },
                "finding": { "type": "string", "description": "The reproduced finding it fixes, by its id in your brief (f1, f2, …), adjudicated a defect first (adjudicate_finding): required while the brief lists a finding not judged other than a defect. Its replay becomes a frozen criterion: it must fail on the original build and pass on the change." }
            },
            "required": ["title", "kind", "why", "serves", "benefit", "complexity", "parts", "plan", "criteria"],
            "additionalProperties": false
        }
    })
}

fn adjudicate_finding() -> Value {
    json!({
        "name": ADJUDICATE_FINDING,
        "description": "Record your judgment of a reproduced finding before anything is fixed (C-55). Judge it against the requirements and the intended semantics, not by how often its check failed: an explorer's expectation is a model's guess, and failing it again and again does not make a defect. `defect`: the application is wrong; only a defect is fixed (choose it with submit_proposal's `finding`). `wrong-expectation`: what the check expected is wrong; it is not offered again. `ambiguous-requirement`: the requirements do not say what is right; it goes to the Operator as a question and is not proposed. `unreliable-reproduction`: it does not reproduce reliably; it is not offered again. The Orchestrator records it in the testing knowledge, kept across objectives, and shows it in the thread.",
        "input_schema": {
            "type": "object",
            "properties": {
                "finding": { "type": "string", "description": "The finding, by its id in your brief (f1, f2, …)." },
                "disposition": { "type": "string", "enum": ["defect", "wrong-expectation", "ambiguous-requirement", "unreliable-reproduction"] },
                "reason": { "type": "string", "description": "Why: what the requirement or the intended behaviour says, and what the finding shows." },
                "requirement": { "type": "string", "description": "The requirement of the model it was judged against (qualified name), if there is one." }
            },
            "required": ["finding", "disposition", "reason"],
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
                "test_changes_accepted": { "type": "boolean", "description": "Whether the changes to tests, checks or budgets that the proposal named are justified. False when there are none." },
                "traceability": { "type": "string", "description": "Your judgment of each change listed as changed but not named in `parts` (does it belong to the proposal, or is it scope creep?) and each element named but not changed (does the proposal still hold?); `none listed` when both lists are empty." },
                "purpose": { "type": "string", "description": "Your judgment of the cumulative change since the approved baseline: does it still serve Agentique's purpose (ROADMAP §1.1) and the requirements the proposal serves, or do small changes add up to redefining the product? The numbers inform; they do not decide." }
            },
            "required": ["verdict", "findings", "test_changes_accepted", "traceability", "purpose"],
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

fn submit_exploration() -> Value {
    json!({
        "name": SUBMIT_EXPLORATION,
        "description": "Hand the explorer its target for this cycle's exploration of the running build: the project it opens (a copy of it, at the build's commit) and its goal (it acts in a test instance, choosing each action among the valid ones, and checks invariants after each). The Orchestrator checks the project and the names, records the target on the objective, and keeps every later exploration, child and replay of the objective to it. Without it, the explorer's goal is the objective.",
        "input_schema": {
            "type": "object",
            "properties": {
                "project": { "type": "string", "description": "The project to explore: a folder of the repository holding a model's files, as the brief lists them for this commit (`model` is Agentique's own model; `models/url-shortener` a sample). Name the one the objective is about; once recorded, later plans keep to it." },
                "goal": { "type": "string", "description": "What to find out, in the words of the screens and panels (the explorer prefers actions whose labels and areas meet its words)." },
                "scope": { "type": "array", "items": { "type": "string" }, "description": "Elements of the project's model the exploration is about, by qualified name; each must be an element of the project's model at this commit." },
                "start": {
                    "type": "object",
                    "description": "Where the explorer starts, by rule, before it chooses anything: `view`, the id of a command that opens a view or panel (such as `requirements-view`; observe_app lists the commands), and `select`, an element of the project's model to select (qualified name).",
                    "properties": {
                        "view": { "type": "string" },
                        "select": { "type": "string" }
                    },
                    "additionalProperties": false
                },
                "vary": { "type": "array", "items": { "type": "string" }, "description": "Only when the objective asks for several projects: other projects later explorations of this objective may explore. Leave it out to explore `project` every time." },
                "hypotheses": {
                    "type": "array",
                    "maxItems": 5,
                    "description": "What the exploration tests first, each with a share of its steps (then it explores what is not covered): engineering hypotheses about whether what the application shows agrees with the project's model and its requirements. The explorer works through each workflow and states an expectation the next observation is checked against: one that fails is a finding of that hypothesis; one it could not state is reported as not answered, never a finding.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "claim": { "type": "string", "description": "What should hold, in one sentence." },
                            "requirement": { "type": "string", "description": "The requirement of the project's model that governs it, by qualified name (read_model shows them); its text is given to the explorer." },
                            "behaviour": { "type": "string", "description": "The intended behaviour in plain words, when no requirement states it." },
                            "workflow": { "type": "string", "description": "The steps in the GUI that test it, in the screens' words (such as: open the Requirements view; select the requirement X)." },
                            "expected": { "type": "string", "description": "What should then be observed (text, counts, a state shown), in words." }
                        },
                        "required": ["claim", "workflow", "expected"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["project", "goal"],
            "additionalProperties": false
        }
    })
}

fn delegate() -> Value {
    json!({
        "name": DELEGATE,
        "description": "Delegate a child objective that explores one area of the running build and reproduces what it finds, within this objective's budget and permissions (it never pushes, merges or adopts). The Orchestrator validates it (budget within what is left, at most two deep, one child at a time) and records it as your directive; your turn then ends, and the child's result (its findings, coverage and spend) comes back to you as your next message.",
        "input_schema": {
            "type": "object",
            "properties": {
                "instruction": { "type": "string", "description": "What the child explores and why." },
                "focus": { "type": "string", "description": "The area, in the words of the screens and panels." },
                "project": { "type": "string", "description": "The project the child explores, one this objective may explore. Needed before you plan this exploration with submit_exploration (a project the brief lists); after, leave it out to give the child the project you planned." },
                "usd": { "type": "number", "description": "Its spend budget in US dollars: more than nothing, and within what is left of this objective's when this objective has a spend limit." },
                "steps": { "type": "integer", "description": "Its exploration's actions, at most this objective's step budget." }
            },
            "required": ["instruction", "usd", "steps"],
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
    if role == Role::Lead {
        list.push(adjudicate_finding());
    }
    Value::Array(list)
}

/// The lead's tools for a cycle that explores (C-54): planning the
/// exploration (`submit_exploration`) or proposing, with `delegate` when it
/// may delegate (its objective is less than two deep).
pub fn lead_tools(planning: bool, may_delegate: bool) -> Value {
    let mut list = tools(Role::Lead);
    let items = list.as_array_mut().expect("a list");
    if planning {
        items.retain(|d| d["name"] != SUBMIT_PROPOSAL && d["name"] != ADJUDICATE_FINDING);
        items.push(submit_exploration());
    }
    if may_delegate {
        items.push(delegate());
    }
    list
}

/// The lead's instructions while it plans an exploration.
pub fn planning_instructions() -> String {
    format!(
        "{COMMON}\n\nYour role: lead, planning this cycle's exploration (C-54). The Orchestrator is about to explore the running build in a test instance, to find problems a deterministic check shows (an invariant of the application, or an expectation stated before an action); what reproduces goes to you to choose one to fix. Look at what the brief says was covered and found before and at what changed recently (reading only; you run no commands), then hand the explorer its target with submit_exploration: the project the objective is about (the brief lists the projects at this commit; `model` is Agentique's own model) and the goal, in the words of the screens and panels, with the elements it is about and where to start when that helps. The project is recorded on the objective and every later exploration keeps to it; name others in `vary` only when the objective asks for several. Give it `hypotheses` to test: what the application should show if it agrees with the project's model and its requirements, each with the requirement that governs it (read the model: read_model, find_elements), the workflow in the screens' words and the observation you expect. Good hypotheses test meaning, not controls: whether counts and summaries agree with the rows they summarise, whether what one element contains is counted with it, whether evidence or a result refers to the configuration it claims, whether something unsupported, unknown or out of date is said to be so. Do not repeat an expectation the brief lists as judged wrong before. If one area deserves a deeper look of its own, delegate it as a child objective first (its result comes back to you). Be brief: this is planning, not the fix."
    )
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
        "Title: {}\nKind: {}\nWhy: {}\nServes: {}\nBenefit: {}\nComplexity: {}\nParts: {}\nIn the base commit's model: {}\nPlan:\n{}\nAcceptance criteria (frozen):\n{}\nIntended changes to tests or checks: {}",
        proposal.title,
        proposal.kind,
        proposal.why,
        proposal.serves.join(", "),
        proposal.benefit,
        proposal.complexity,
        proposal.parts.join(", "),
        proposal
            .resolved
            .as_ref()
            .map(Resolution::text)
            .unwrap_or_else(|| "not resolved".into()),
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
            "Your role: lead. Find one genuine, bounded improvement that serves the objective, and hand it over with submit_proposal. Say what it is for (C-55): `serves` names the requirements of the project's model it serves (requirement defs or usages, by qualified name; read_model shows them), `benefit` what the Operator will see, `complexity` what it adds, removes or generalises at the root (a root part, a dependency, a crate); `parts` names the existing elements and contracts it affects (for elements it creates, their future owner), and the review compares them with what the change touches; its evidence is its acceptance criteria, frozen with it (and the replay of the finding it fixes). The purpose and the model's requirements bound what any cycle changes: in a project whose model declares a root purpose requirement (`Purpose`, `purpose`; Agentique's does, for ROADMAP §1.1), that requirement, everything it owns and ROADMAP.md are the Operator's, and a change to them fails the gates even when the objective names them; describe a change you think the purpose needs in `why`, for the Operator. When the brief lists reproduced findings (C-54), judge each you consider with adjudicate_finding before choosing one (C-55): against the requirements and the intended semantics, never by how often its check failed (an explorer's expectation is a model's guess, and repeated disagreement with it does not establish a defect). Only a finding judged a defect is fixed (`finding`), and its replay becomes a frozen criterion; a finding that contradicts a hypothesis is judged against the requirement that governs it, and a proposal fixing it names that requirement in `serves`; a wrong expectation or an unreliable reproduction is set aside, and an ambiguous requirement goes to the Operator as a question; when you judge none a defect, end without a proposal. No criterion may pass on the original build, and at least one must fail there with evidence: the replay, an observation, or a test that compiles and runs on the base (the change's new and changed test files are brought over, so put a new test in a test file that compiles against the base, such as a crate's tests/ folder). A change to the Studio (a part named Studio) needs a behavioural criterion (an observation or judgment, or the replay). Look before you choose: the self-model (read_model; model/Agentique.sysml), the code (read and search files; you run no commands), docs/stages.md and ROADMAP §5.6 (known problems), and the running application (observe_app). Choose something small (a few files), real (evidence: a failing case, a wrong result, a confusing screen), and checkable: at least one criterion must be a command that fails before the change and passes after (usually a new test: `cargo test -p <crate> <test name>`); a usability or comprehension improvement also gets an observation or judgment criterion in the running application. Leave locked parts and Agentique's safeguards alone unless the objective names them: the code of locked parts (crates/language, crates/system-state, crates/history, crates/execution, crates/implementation, crates/launcher, crates/orchestrator, claude-agent and the Assistant's Claude Agent runtime) and the safeguards (crates/assistant/src/policy.rs and model_tools.rs, crates/studio-native/src/control, objectives.rs and panels/objectives.rs, crates/implementation/src/task.rs); a change there fails the gates. Do not repeat an earlier cycle's improvement. You work in a throwaway checkout: change nothing there."
        }
        Role::Implementer => {
            "Your role: implementer. Implement the frozen proposal in this worktree, and only it: the review compares what you change with the elements it names. Add tests for the criteria; keep every existing test and check (rule 10). Never change the model's root purpose requirement (`Purpose`, `purpose`) or, in a project that declares one (Agentique does), ROADMAP.md (C-55): that fails the gates, whatever the objective names. Run what you need yourself: `cargo fmt --all`, `cargo clippy -p <crate> --all-targets --offline -- -D warnings`, `cargo test -p <crate> --offline`, and the criteria's commands (a shared CARGO_TARGET_DIR is set). Model changes go through apply_changes. When done, call submit_implementation; the Orchestrator commits and checks a clean checkout. If you are repairing, fix exactly the failures and findings listed, without weakening a check."
        }
        Role::Reviewer => {
            "Your role: independent reviewer. You did not write this change. Review it against the frozen proposal and its criteria, the check results and the evaluation below, reading the code in this checkout (you write nothing). Judge correctness, scope (nothing unrelated), simplicity and naming (ROADMAP §1.3, §8.4), whether the tests really check the criteria, and every change to tests, checks or budgets the baseline guard lists. For each criterion counted as evidence on the original build, check that its failing test asserts the defect itself, not merely that the change exists (a test asserting that a new file, function or control exists fails on the base for any change); request changes when it does not. Judge traceability explicitly, as you judge test changes (C-55): each change listed as changed but not named in the proposal's `parts` either belongs to the proposal or is scope creep (request changes), and each element named but not changed says whether the proposal still holds. Judge the cumulative change since the approved baseline: does it still serve the project's purpose (Agentique's is ROADMAP §1.1, its model's requirement `purpose`) and the requirements the proposal serves, or do small changes add up to redefining the product? When the proposal fixes a finding, the brief says how the lead judged it: check that judgment against the requirement too. The counts inform; they do not decide, and neither does any agent's approval. Then call submit_review with both judgments: approve only a change you would merge as it is."
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

/// What a lead's submissions are checked against (C-54, C-55).
pub struct Given<'a> {
    /// The reproduced findings the lead was given: `(id, identity)`.
    pub findings: &'a [(String, String)],
    /// What each finding was judged to be, by identity: the testing
    /// knowledge's, and the lead's own in this turn.
    pub dispositions: &'a BTreeMap<String, Disposition>,
    /// The base commit's model, or why there is none to resolve names in.
    pub model: &'a Result<Elements, String>,
}

/// The finding `id` of the brief (`f1`, …), by its identity.
fn finding_of(given: &Given, id: &str) -> Result<String, String> {
    if given.findings.is_empty() {
        return Err(format!(
            "there are no reproduced findings; `finding` {id} is not one"
        ));
    }
    given
        .findings
        .iter()
        .find(|(f, _)| f == id.trim())
        .map(|(_, identity)| identity.clone())
        .ok_or_else(|| {
            format!(
                "`finding` {id} is not one of the reproduced findings ({})",
                given
                    .findings
                    .iter()
                    .map(|(f, _)| f.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// What a lead's plan of an exploration and a child's project are checked
/// against (C-54, the W13.7 repair): the projects of the base commit (the
/// folders that hold a model's files, as the repository lists them), that
/// commit, and what the objective explores so far: its recorded target, or
/// a plan accepted earlier in the lead's turn, which [`Planning::accept`]
/// makes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Planning {
    pub projects: Vec<String>,
    pub revision: String,
    pub target: Option<Target>,
    /// The projects children were given before there was a target: the
    /// first plan keeps them among its projects, so what they find is the
    /// objective's.
    pub given: Vec<String>,
}

impl Planning {
    /// The projects a plan or a child may name: the target's, or before
    /// there is one, every project of the base commit.
    pub fn permitted(&self) -> Vec<String> {
        match &self.target {
            Some(target) => target.projects().into_iter().map(str::to_string).collect(),
            None => self.projects.clone(),
        }
    }

    /// `named` as the repository lists it (a project folder, or the one of
    /// its model folder), if a plan may name it; else why not: outside the
    /// target's projects, or holding no model at the commit.
    pub fn project(&self, named: &str) -> Result<String, String> {
        let named = named.trim().replace('\\', "/");
        let named = named.trim_end_matches('/').to_string();
        if named.is_empty() {
            return Err("name the project to explore".into());
        }
        if crate::explore::climbs(&named) {
            return Err(format!(
                "`{named}` is not a folder of the repository: name one as the brief lists it"
            ));
        }
        let listed = [named.clone(), format!("{named}/model")]
            .into_iter()
            .find(|form| self.projects.contains(form));
        let held = || {
            format!(
                "`{named}` holds no model at {}: the projects are {}",
                crate::builds::short(&self.revision),
                if self.projects.is_empty() {
                    "none".to_string()
                } else {
                    self.projects.join(", ")
                }
            )
        };
        match &self.target {
            Some(target) => {
                let form = listed.clone().unwrap_or(named.clone());
                let permitted = self.permitted();
                if !permitted.contains(&form) {
                    return Err(format!(
                        "this objective explores {}, as its first plan recorded: name {}",
                        target.line(),
                        permitted.join(" or ")
                    ));
                }
                listed.ok_or_else(held)
            }
            None => listed.ok_or_else(held),
        }
    }

    /// Makes `planned` what the objective explores, and returns it as it is
    /// recorded: the projects it may explore stay those of its first plan
    /// (`vary` holds the others).
    pub fn accept(&mut self, planned: Target) -> Target {
        let permitted: Vec<String> = match &self.target {
            Some(target) => target.projects(),
            None => planned
                .projects()
                .into_iter()
                .chain(self.given.iter().map(String::as_str))
                .collect(),
        }
        .into_iter()
        .map(str::to_string)
        .collect();
        let mut seen = Vec::new();
        let permitted: Vec<String> = permitted
            .into_iter()
            .filter(|p| {
                let first = !seen.contains(p);
                seen.push(p.clone());
                first
            })
            .collect();
        let vary = permitted
            .into_iter()
            .filter(|p| *p != planned.project)
            .collect();
        let accepted = Target { vary, ..planned };
        self.target = Some(accepted.clone());
        accepted
    }

    /// Notes that a child was given `project`: before there is a target,
    /// the first plan keeps it among its projects.
    pub fn gave(&mut self, project: &str) {
        if self.target.is_none() && !self.given.iter().any(|p| p == project) {
            self.given.push(project.to_string());
        }
    }

    /// The target of a child the lead delegates to explore `instruction`:
    /// on the project it names (one the objective may explore) or else the
    /// objective's, without other projects. Before the objective has a
    /// target, the lead must name the project.
    pub fn child(&self, named: Option<&str>, instruction: &str) -> Result<Target, String> {
        let named = named.map(str::trim).filter(|n| !n.is_empty());
        let project = match (named, &self.target) {
            (Some(named), _) => self.project(named)?,
            (None, Some(target)) => target.project.clone(),
            (None, None) => {
                return Err(format!(
                    "this objective has no project to explore yet: name the child's `project` ({}), or plan the exploration with submit_exploration first",
                    if self.projects.is_empty() {
                        "none at this commit".to_string()
                    } else {
                        self.projects.join(", ")
                    }
                ));
            }
        };
        // What the plan said about its own project only.
        let (scope, start, hypotheses) = match &self.target {
            Some(target) if target.project == project => (
                target.scope.clone(),
                target.start.clone(),
                target.hypotheses.clone(),
            ),
            _ => (Vec::new(), Default::default(), Vec::new()),
        };
        Ok(Target {
            project,
            revision: self.revision.clone(),
            goal: instruction.to_string(),
            scope,
            start,
            vary: Vec::new(),
            hypotheses,
        })
    }
}

/// A plan from `submit_exploration`'s input, checked against `planning`
/// (C-54, the W13.7 repair): a project the objective may explore (one of
/// the base commit's; once it has a target, one of the target's), a goal,
/// other projects only among those, a view as a command's id, and its
/// hypotheses (E3), each a claim, a requirement or the intended behaviour,
/// a workflow and an expected observation, at most [`HYPOTHESES`]. The
/// names of `scope`, `start.select` and the hypotheses' requirements are
/// the caller's to resolve in the project's model. Its revision is the base
/// commit.
pub fn read_exploration(input: &Value, planning: &Planning) -> Result<Target, String> {
    let project = planning.project(input["project"].as_str().unwrap_or_default())?;
    let goal = input["goal"].as_str().unwrap_or_default().trim();
    if goal.is_empty() {
        return Err("a plan needs its goal".into());
    }
    let texts = |field: &Value| -> Vec<String> {
        field
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    };
    let mut vary = Vec::new();
    for other in texts(&input["vary"]) {
        let other = planning
            .project(&other)
            .map_err(|problem| format!("`vary`: {problem}"))?;
        if other != project && !vary.contains(&other) {
            vary.push(other);
        }
    }
    let text = |field: &str| {
        input["start"][field]
            .as_str()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    };
    let start = crate::explore::Start {
        view: text("view"),
        select: text("select"),
    };
    if let Some(view) = &start.view
        && !view
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!(
            "`start.view` is a command's id, such as `requirements-view`, not `{view}`"
        ));
    }
    let hypotheses = input["hypotheses"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, h)| {
            read_hypothesis(h).map_err(|problem| format!("hypothesis {}: {problem}", i + 1))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if hypotheses.len() > HYPOTHESES {
        return Err(format!(
            "a plan tests at most {HYPOTHESES} hypotheses, each with a share of the steps"
        ));
    }
    Ok(Target {
        project,
        revision: planning.revision.clone(),
        goal: goal.to_string(),
        scope: texts(&input["scope"]),
        start,
        vary,
        hypotheses,
    })
}

/// The hypotheses a plan tests at most.
pub const HYPOTHESES: usize = 5;

/// A hypothesis of a plan: its claim, its requirement (a qualified name) or
/// the intended behaviour, its workflow and its expected observation.
fn read_hypothesis(input: &Value) -> Result<crate::explore::Hypothesis, String> {
    let text = |field: &str| {
        input[field]
            .as_str()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    };
    let needed = |field: &str| text(field).ok_or(format!("`{field}` is needed"));
    let hypothesis = crate::explore::Hypothesis {
        claim: needed("claim")?,
        requirement: text("requirement"),
        requirement_text: None,
        behaviour: text("behaviour"),
        workflow: needed("workflow")?,
        expected: needed("expected")?,
    };
    if hypothesis.requirement.is_none() && hypothesis.behaviour.is_none() {
        return Err(
            "name the `requirement` of the project's model that governs it, or state the intended `behaviour`"
                .into(),
        );
    }
    Ok(hypothesis)
}

/// A review from `submit_review`'s input, checked (C-55): its verdict one
/// of the two, and its judgments of traceability and of the cumulative
/// change stated, never blank (a judgment left out is no judgment).
pub fn read_review(input: &Value) -> Result<(), String> {
    if !matches!(
        input["verdict"].as_str(),
        Some("approve" | "request_changes")
    ) {
        return Err("`verdict` is approve or request_changes".into());
    }
    for field in ["traceability", "purpose"] {
        if input[field].as_str().is_none_or(|t| t.trim().is_empty()) {
            return Err(format!(
                "`{field}` states your judgment: it cannot be left blank"
            ));
        }
    }
    Ok(())
}

/// A disposition from `adjudicate_finding`'s input (C-55), by the lead of
/// `objective`'s cycle `cycle`: the finding one the lead was given, its
/// kind known, a reason, and the requirement it was judged against, if
/// named, a requirement of the base commit's model. Returns the finding's
/// identity with it.
pub fn read_disposition(
    input: &Value,
    given: &Given,
    objective: &str,
    cycle: u32,
) -> Result<(String, Disposition), String> {
    let identity = finding_of(given, input["finding"].as_str().unwrap_or_default())?;
    let kind: DispositionKind = serde_json::from_value(input["disposition"].clone()).map_err(|_| {
        "`disposition` is one of defect, wrong-expectation, ambiguous-requirement, unreliable-reproduction"
            .to_string()
    })?;
    let reason = input["reason"].as_str().unwrap_or_default().trim();
    if reason.is_empty() {
        return Err("a disposition needs its reason".into());
    }
    let requirement = input["requirement"]
        .as_str()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(str::to_string);
    if let (Some(name), Ok(model)) = (&requirement, given.model) {
        traceability::requirement(model, name)
            .map_err(|problem| format!("`requirement`: {problem}"))?;
    }
    Ok((
        identity,
        Disposition {
            kind,
            reason: reason.to_string(),
            requirement,
            objective: objective.to_string(),
            cycle,
            role: Role::Lead.name().to_string(),
            at: agq_launcher::now(),
            build: None,
        },
    ))
}

/// A proposal from `submit_proposal`'s input, checked against what the lead
/// was `given`: at least one criterion with a deterministic check (the
/// replay of the finding it chooses counts), unique ids, commands as words,
/// conditions known; while a reproduced finding the lead was given is not
/// judged other than a defect, one of them chosen, and the one chosen judged
/// a defect (C-55); a change to the part `Studio` with a behavioural
/// criterion (C-54); and (C-55) the requirements it serves, its benefit and
/// its effect on complexity stated, `serves` resolved to requirements and
/// `parts` to elements of the base commit's model (when it has one; else
/// the resolution says why not).
pub fn read_proposal(input: &Value, given: &Given) -> Result<Proposal, String> {
    let mut proposal: Proposal = serde_json::from_value(json!({
        "title": input["title"],
        "kind": input["kind"],
        "why": input["why"],
        "parts": input["parts"],
        "plan": input["plan"],
        "criteria": input["criteria"],
        "intendedTestChanges": input.get("intended_test_changes").cloned().unwrap_or(json!([])),
        "serves": input.get("serves").cloned().unwrap_or(json!([])),
        "benefit": input.get("benefit").cloned().unwrap_or(json!("")),
        "complexity": input.get("complexity").cloned().unwrap_or(json!("")),
    }))
    .map_err(|e| format!("the proposal cannot be read: {e}"))?;
    // The findings still to be fixed: none judged other than a defect.
    let open: Vec<&str> = given
        .findings
        .iter()
        .filter(|(_, identity)| {
            given
                .dispositions
                .get(identity)
                .is_none_or(|d| d.kind == DispositionKind::Defect)
        })
        .map(|(id, _)| id.as_str())
        .collect();
    proposal.finding = match input["finding"].as_str() {
        None if open.is_empty() => None,
        None => {
            return Err(format!(
                "choose the reproduced finding it fixes: `finding` is one of {}, judged a defect first with adjudicate_finding (a finding judged other than a defect is not fixed)",
                open.join(", ")
            ));
        }
        Some(id) => {
            let identity = finding_of(given, id)?;
            match given.dispositions.get(&identity) {
                Some(d) if d.kind == DispositionKind::Defect => Some(identity),
                Some(d) => {
                    return Err(format!(
                        "finding {id} was judged {}, not a defect: it is not fixed",
                        d.kind.name()
                    ));
                }
                None => {
                    return Err(format!(
                        "finding {id} is not adjudicated: judge it with adjudicate_finding first (only a defect is fixed)"
                    ));
                }
            }
        }
    };
    proposal.title = proposal.title.trim().to_string();
    if proposal.title.is_empty() || proposal.why.trim().is_empty() {
        return Err("a proposal needs a title and why".into());
    }
    proposal.serves = proposal
        .serves
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if proposal.serves.is_empty()
        || proposal.benefit.trim().is_empty()
        || proposal.complexity.trim().is_empty()
    {
        return Err(
            "a proposal names the requirements of the model it serves (`serves`), the benefit the Operator will see (`benefit`) and its effect on complexity (`complexity`)"
                .into(),
        );
    }
    proposal.resolved = Some(match given.model {
        Ok(model) => traceability::resolve(&proposal.serves, &proposal.parts, model)?,
        Err(why) => Resolution {
            skipped: Some(why.clone()),
            ..Resolution::default()
        },
    });
    let mut ids = std::collections::BTreeSet::new();
    for criterion in &proposal.criteria {
        if !ids.insert(criterion.id.clone()) {
            return Err(format!("the criterion id {} is used twice", criterion.id));
        }
        match &criterion.check {
            crate::record::Check::Command { program } => test_command(program)
                .map_err(|problem| format!("criterion {}: {problem}", criterion.id))?,
            crate::record::Check::Observation {
                expect, condition, ..
            } => {
                crate::control::expectation(expect)
                    .map_err(|problem| format!("criterion {}: {problem}", criterion.id))?;
                if let Some(condition) = condition
                    && !crate::control::CONDITIONS.contains(&condition.as_str())
                {
                    return Err(format!(
                        "criterion {}: `{condition}` is not a condition a test instance starts in ({})",
                        criterion.id,
                        crate::control::CONDITIONS.join(", ")
                    ));
                }
            }
            crate::record::Check::Judgment => {}
        }
    }
    if criterion_ids_taken(&proposal) {
        return Err(format!(
            "`{}` is the replay's criterion id: use another",
            crate::record::REPLAY
        ));
    }
    if proposal.finding.is_none()
        && !proposal.criteria.iter().any(|c| {
            matches!(
                c.check,
                crate::record::Check::Command { .. } | crate::record::Check::Observation { .. }
            )
        })
    {
        return Err(
            "at least one criterion needs a deterministic check (a command, or an observation)"
                .into(),
        );
    }
    if user_facing(&proposal) && !behavioural(&proposal) {
        return Err(
            "a change to the Studio needs a behavioural criterion: an observation or a judgment in a test instance, or the replay of a finding"
                .into(),
        );
    }
    Ok(proposal)
}

/// Whether a proposal that fixes the finding of a hypothesis serves the
/// requirement governing it (the W13.7 repair, E3): that a finding
/// reproduced is no authority to change; what the change is for is the
/// requirement the finding showed the application contradicts.
pub fn serves_the_finding(proposal: &Proposal, requirement: Option<&str>) -> Result<(), String> {
    match requirement {
        Some(requirement)
            if !proposal
                .serves
                .iter()
                .any(|s| s.trim() == requirement.trim()) =>
        {
            Err(format!(
                "the finding it fixes contradicts {requirement}: `serves` names that requirement"
            ))
        }
        _ => Ok(()),
    }
}

/// Whether a criterion of the lead's takes the id of the replay's.
fn criterion_ids_taken(proposal: &Proposal) -> bool {
    proposal
        .criteria
        .iter()
        .any(|c| c.id == crate::record::REPLAY)
}

/// Whether the proposal changes the part `Studio` (user-facing code).
pub fn user_facing(proposal: &Proposal) -> bool {
    proposal.parts.iter().any(|p| {
        p.rsplit("::")
            .next()
            .is_some_and(|name| name.trim() == "Studio")
    })
}

/// Whether the proposal has a criterion checked by behaviour in a test
/// instance: an observation, a judgment, or the replay of its finding.
pub fn behavioural(proposal: &Proposal) -> bool {
    proposal.finding.is_some()
        || proposal.criteria.iter().any(|c| {
            matches!(
                c.check,
                crate::record::Check::Observation { .. } | crate::record::Check::Judgment
            )
        })
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

    /// A proposal read as a lead without a model (nothing to resolve names
    /// in) is given `findings`, judged as `dispositions` say.
    fn judged(
        input: &Value,
        findings: &[(String, String)],
        dispositions: &BTreeMap<String, Disposition>,
    ) -> Result<Proposal, String> {
        let model = Err("the project has no model".to_string());
        read_proposal(
            input,
            &Given {
                findings,
                dispositions,
                model: &model,
            },
        )
    }

    fn read(input: &Value, findings: &[(String, String)]) -> Result<Proposal, String> {
        judged(input, findings, &BTreeMap::new())
    }

    fn disposition(kind: DispositionKind) -> Disposition {
        Disposition {
            kind,
            reason: "r".into(),
            requirement: None,
            objective: "objective-1".into(),
            cycle: 1,
            role: "lead".into(),
            at: "t".into(),
            build: None,
        }
    }

    /// The alignment fields every proposal states (C-55), added to `input`.
    fn aligned(mut input: Value) -> Value {
        input["serves"] = json!(["AgentiqueArchitecture::GatesDecide"]);
        input["benefit"] = json!("The Operator sees the gap closed");
        input["complexity"] = json!("Adds one check inside the Orchestrator; nothing at the root");
        input
    }

    #[test]
    fn a_proposal_needs_a_deterministic_criterion_and_unique_ids() {
        let base = aligned(json!({
            "title": "Fix the gap", "kind": "correctness", "why": "It is wrong at x.rs:3",
            "parts": ["AgentiqueArchitecture::Orchestrator"], "plan": ["Write the test", "Fix it"],
            "criteria": [{ "id": "c1", "statement": "The gap is gone", "check": { "kind": "command", "program": ["cargo", "test", "-p", "agq-x", "gap"] } }]
        }));
        let proposal = read(&base, &[]).unwrap();
        assert_eq!(proposal.criteria.len(), 1);
        let mut judgment_only = base.clone();
        judgment_only["criteria"] =
            json!([{ "id": "c1", "statement": "Looks better", "check": { "kind": "judgment" } }]);
        assert!(read(&judgment_only, &[]).is_err());
        let mut twice = base.clone();
        twice["criteria"] = json!([base["criteria"][0], base["criteria"][0]]);
        assert!(read(&twice, &[]).is_err());
    }

    /// C-54: a proposal chooses one of the reproduced findings it was given
    /// (whose replay is then its criterion), a change to the Studio needs a
    /// behavioural criterion, and a condition is one a test instance knows.
    #[test]
    fn a_proposal_chooses_a_reproduced_finding_and_a_studio_change_is_behavioural() {
        let base = aligned(json!({
            "title": "Label the filter", "kind": "usability", "why": "The filter has no readable label",
            "parts": ["AgentiqueArchitecture::Studio"], "plan": ["Label it"],
            "criteria": [{ "id": "c1", "statement": "It has a test", "check": { "kind": "command", "program": ["cargo", "test", "-p", "agq-x", "label"] } }]
        }));
        let findings = vec![
            (
                "f1".to_string(),
                "readable-labels|filter|no label".to_string(),
            ),
            ("f2".to_string(), "answers||the instance exited".to_string()),
        ];
        let defect =
            BTreeMap::from([(findings[1].1.clone(), disposition(DispositionKind::Defect))]);
        assert!(read(&base, &findings).unwrap_err().contains("f1, f2"));
        let mut chosen = base.clone();
        chosen["finding"] = json!("f2");
        let proposal = judged(&chosen, &findings, &defect).unwrap();
        assert_eq!(
            proposal.finding.as_deref(),
            Some("answers||the instance exited")
        );
        let mut unknown = base.clone();
        unknown["finding"] = json!("f9");
        assert!(judged(&unknown, &findings, &defect).is_err());
        assert!(read(&chosen, &[]).is_err(), "no findings to choose");
        // The Studio without a behavioural criterion: refused.
        assert!(read(&base, &[]).unwrap_err().contains("behavioural"));
        let mut observed = base.clone();
        observed["criteria"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "id": "c2", "statement": "Labelled", "check": { "kind": "observation", "expect": { "anyLabelContains": "Filter" }, "condition": "recovered" } }));
        assert!(read(&observed, &[]).is_ok());
        observed["criteria"][1]["check"]["condition"] = json!("on fire");
        assert!(read(&observed, &[]).is_err());
        let mut taken = base.clone();
        taken["parts"] = json!(["AgentiqueArchitecture::Orchestrator"]);
        taken["criteria"][0]["id"] = json!("replay");
        assert!(read(&taken, &[]).is_err(), "the replay's id is taken");
        // A finding's replay alone is a deterministic, behavioural criterion.
        let mut replay_only = chosen.clone();
        replay_only["criteria"] =
            json!([{ "id": "c1", "statement": "Looks right", "check": { "kind": "judgment" } }]);
        assert!(judged(&replay_only, &findings, &defect).is_ok());
    }

    /// C-55: a proposal states the requirements it serves, its benefit and
    /// its effect on complexity; `serves` resolves to requirements and
    /// `parts` to elements of the base commit's model, by identity, or the
    /// proposal is refused with the reason; without a model, the
    /// resolution says why nothing was resolved.
    #[test]
    fn a_proposal_names_requirements_it_serves_resolved_in_the_base_model() {
        use agq_assistant::model_tools::Described;
        let element = |id: u64, name: &str, kind: &str, owners: &[u64]| Described {
            id,
            name: name.into(),
            kind: kind.into(),
            owners: owners.to_vec(),
            locked: false,
        };
        let model: Result<Elements, String> = Ok([
            element(1, "AgentiqueArchitecture", "package", &[]),
            element(2, "AgentiqueArchitecture::Orchestrator", "part def", &[1]),
            element(
                3,
                "AgentiqueArchitecture::GatesDecide",
                "requirement def",
                &[1],
            ),
            element(4, "AgentiqueArchitecture::gatesDecide", "requirement", &[1]),
        ]
        .into_iter()
        .map(|e| (e.id, e))
        .collect());
        let none = BTreeMap::new();
        let given = Given {
            findings: &[],
            dispositions: &none,
            model: &model,
        };
        let base = aligned(json!({
            "title": "Fix the gap", "kind": "correctness", "why": "It is wrong at x.rs:3",
            "parts": ["AgentiqueArchitecture::Orchestrator"], "plan": ["Fix it"],
            "criteria": [{ "id": "c1", "statement": "The gap is gone", "check": { "kind": "command", "program": ["cargo", "test", "-p", "agq-x", "gap"] } }]
        }));
        // Valid: the ids recorded.
        let proposal = read_proposal(&base, &given).unwrap();
        let resolved = proposal.resolved.as_ref().unwrap();
        assert_eq!(resolved.serves[0].element, 3);
        assert_eq!(resolved.parts[0].element, 2);
        assert!(resolved.skipped.is_none());
        assert!(proposal_text(&proposal).contains("AgentiqueArchitecture::GatesDecide (#3)"));
        // Unresolved `serves`: refused, with the requirements there are.
        let mut unknown = base.clone();
        unknown["serves"] = json!(["AgentiqueArchitecture::Purpose"]);
        let refused = read_proposal(&unknown, &given).unwrap_err();
        assert!(
            refused.contains("`AgentiqueArchitecture::Purpose` is not an element"),
            "{refused}"
        );
        assert!(
            refused.contains("AgentiqueArchitecture::gatesDecide"),
            "{refused}"
        );
        // `serves` naming a part: refused.
        let mut part = base.clone();
        part["serves"] = json!(["AgentiqueArchitecture::Orchestrator"]);
        let refused = read_proposal(&part, &given).unwrap_err();
        assert!(
            refused.contains("is a part def, not a requirement"),
            "{refused}"
        );
        // Unresolved `parts`: refused.
        let mut parts = base.clone();
        parts["parts"] = json!(["AgentiqueArchitecture::Conductor"]);
        let refused = read_proposal(&parts, &given).unwrap_err();
        assert!(
            refused.contains("`parts`: `AgentiqueArchitecture::Conductor`"),
            "{refused}"
        );
        // Each alignment field is needed.
        for field in ["serves", "benefit", "complexity"] {
            let mut missing = base.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(read_proposal(&missing, &given).is_err(), "{field}");
        }
        let mut blank = base.clone();
        blank["benefit"] = json!("  ");
        assert!(read_proposal(&blank, &given).is_err());
        // No model: accepted, and the record says why nothing was resolved.
        let proposal = read(&unknown, &[]).unwrap();
        let resolved = proposal.resolved.unwrap();
        assert_eq!(
            resolved.skipped.as_deref(),
            Some("the project has no model")
        );
        assert!(resolved.serves.is_empty());
        // The tool states them as required.
        let schema = submit_proposal();
        let required = schema["input_schema"]["required"].as_array().unwrap();
        for field in ["serves", "benefit", "complexity"] {
            assert!(required.contains(&json!(field)), "{field}");
        }
    }

    /// C-55: a finding is adjudicated before it is fixed: choosing one not
    /// adjudicated (an explorer's expectation, say) is refused, as is one
    /// judged other than a defect; when every finding is judged other than
    /// a defect, a proposal needs none. A disposition names a finding the
    /// lead was given, a known kind, a reason, and a requirement of the
    /// model if any.
    #[test]
    fn a_finding_is_adjudicated_before_it_is_chosen() {
        let base = aligned(json!({
            "title": "Fix the dialog", "kind": "correctness", "why": "It does not close",
            "parts": ["AgentiqueArchitecture::Orchestrator"], "plan": ["Fix it"],
            "criteria": [{ "id": "c1", "statement": "It closes", "check": { "kind": "command", "program": ["cargo", "test", "-p", "agq-x", "closes"] } }]
        }));
        let findings = vec![
            (
                "f1".to_string(),
                "expectation|save|the status says saved".to_string(),
            ),
            ("f2".to_string(), "dialogs-close|dialog|x".to_string()),
        ];
        let mut chosen = base.clone();
        chosen["finding"] = json!("f1");
        let refused = read(&chosen, &findings).unwrap_err();
        assert!(refused.contains("not adjudicated"), "{refused}");
        let mut dispositions = BTreeMap::from([(
            findings[0].1.clone(),
            disposition(DispositionKind::WrongExpectation),
        )]);
        let refused = judged(&chosen, &findings, &dispositions).unwrap_err();
        assert!(
            refused.contains("a wrong expectation, not a defect"),
            "{refused}"
        );
        // f2 is still open: a proposal must choose it.
        let refused = judged(&base, &findings, &dispositions).unwrap_err();
        assert!(refused.contains("one of f2"), "{refused}");
        dispositions.insert(
            findings[1].1.clone(),
            disposition(DispositionKind::AmbiguousRequirement),
        );
        let mut ambiguous = base.clone();
        ambiguous["finding"] = json!("f2");
        assert!(judged(&ambiguous, &findings, &dispositions).is_err());
        // Nothing judged a defect: a proposal of its own needs no finding.
        assert!(
            judged(&base, &findings, &dispositions)
                .unwrap()
                .finding
                .is_none()
        );
        dispositions.insert(findings[0].1.clone(), disposition(DispositionKind::Defect));
        assert_eq!(
            judged(&chosen, &findings, &dispositions).unwrap().finding,
            Some(findings[0].1.clone())
        );
        // The disposition itself.
        let model: Result<Elements, String> = Ok(BTreeMap::from([(
            7,
            agq_assistant::model_tools::Described {
                id: 7,
                name: "Shop::Store".into(),
                kind: "part def".into(),
                owners: Vec::new(),
                locked: false,
            },
        )]));
        let none = BTreeMap::new();
        let given = Given {
            findings: &findings,
            dispositions: &none,
            model: &model,
        };
        let input = json!({ "finding": "f1", "disposition": "wrong-expectation", "reason": "The requirement says the status stays" });
        let (identity, read) = read_disposition(&input, &given, "objective-1", 3).unwrap();
        assert_eq!(identity, findings[0].1);
        assert_eq!(read.kind, DispositionKind::WrongExpectation);
        assert_eq!(
            (read.objective.as_str(), read.cycle, read.role.as_str()),
            ("objective-1", 3, "lead")
        );
        for (wrong, why) in [
            (
                json!({ "finding": "f9", "disposition": "defect", "reason": "x" }),
                "not one of",
            ),
            (
                json!({ "finding": "f1", "disposition": "bug", "reason": "x" }),
                "one of defect",
            ),
            (
                json!({ "finding": "f1", "disposition": "defect", "reason": " " }),
                "reason",
            ),
            (
                json!({ "finding": "f1", "disposition": "defect", "reason": "x", "requirement": "Shop::Store" }),
                "not a requirement",
            ),
        ] {
            let refused = read_disposition(&wrong, &given, "objective-1", 3).unwrap_err();
            assert!(refused.contains(why), "{refused}");
        }
        let empty = Given {
            findings: &[],
            dispositions: &none,
            model: &model,
        };
        assert!(read_disposition(&input, &empty, "o", 1).is_err());
        // The lead plans without it, and proposes with it.
        let names = |tools: Value| -> Vec<String> {
            tools
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["name"].as_str().unwrap().to_string())
                .collect()
        };
        assert!(names(lead_tools(false, false)).contains(&ADJUDICATE_FINDING.to_string()));
        assert!(!names(lead_tools(true, false)).contains(&ADJUDICATE_FINDING.to_string()));
    }

    /// C-55: ids are places among the cycle's findings, never renumbered:
    /// once the first finding is set aside, a later session is offered `f2`
    /// and `f3`, so `f1` binds nothing and `f2` still binds the second.
    #[test]
    fn finding_ids_bind_the_same_finding_in_every_session() {
        use crate::findings::{Check, Failed, Finding, State};
        use crate::knowledge::{Knowledge, finding_id};
        let findings: Vec<Finding> = ["a", "b", "c"]
            .iter()
            .map(|control| {
                let mut f = Finding::new(
                    Failed {
                        check: Check::ReadableLabels,
                        control: control.to_string(),
                        message: "no label".into(),
                        evidence: json!({}),
                    },
                    Vec::new(),
                    "b1",
                    "abc",
                    "model",
                );
                f.state = State::Reproduced;
                f
            })
            .collect();
        let mut knowledge = Knowledge::new("p");
        knowledge.adjudicate(&findings[0], disposition(DispositionKind::WrongExpectation));
        let offered: Vec<(String, String)> = knowledge
            .offered(&findings)
            .into_iter()
            .map(|(i, f)| (finding_id(i), f.identity.clone()))
            .collect();
        assert_eq!(
            offered
                .iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>(),
            vec!["f2", "f3"]
        );
        let none = BTreeMap::new();
        let model = Err("the project has no model".to_string());
        let given = Given {
            findings: &offered,
            dispositions: &none,
            model: &model,
        };
        let judge = |id: &str| {
            read_disposition(
                &json!({ "finding": id, "disposition": "defect", "reason": "x" }),
                &given,
                "o",
                1,
            )
        };
        assert!(
            judge("f1").is_err(),
            "set aside, not renumbered onto another"
        );
        assert_eq!(judge("f2").unwrap().0, findings[1].identity);
        assert_eq!(judge("f3").unwrap().0, findings[2].identity);
    }

    /// The W13.7 repair: a plan names a project of the base commit (as the
    /// repository lists it, `X` meaning `X/model` where that is listed), a
    /// goal, other projects only among those, and a view as a command's id;
    /// once the objective has a target, a later plan keeps to its projects
    /// and cannot widen them, and is told so first.
    #[test]
    fn a_plan_names_a_project_of_the_commit_and_keeps_to_its_target() {
        let projects = vec![
            "examples/shop/model".to_string(),
            "model".into(),
            "models/garden".into(),
            "models/shop".into(),
        ];
        let mut planning = Planning {
            projects: projects.clone(),
            revision: "abc1234".into(),
            target: None,
            given: Vec::new(),
        };
        let plan = read_exploration(
            &json!({
                "project": r"models\shop/", "goal": " Look at the Requirements panel ",
                "scope": ["Shop::Store", " "],
                "start": { "view": "requirements-view", "select": "Shop::Store" },
                "vary": ["model", "models/shop"]
            }),
            &planning,
        )
        .unwrap();
        assert_eq!(
            (
                plan.project.as_str(),
                plan.goal.as_str(),
                plan.revision.as_str()
            ),
            ("models/shop", "Look at the Requirements panel", "abc1234")
        );
        assert_eq!(plan.scope, vec!["Shop::Store".to_string()]);
        assert_eq!(plan.vary, vec!["model".to_string()]);
        assert_eq!(plan.start.view.as_deref(), Some("requirements-view"));
        assert_eq!(plan.projects(), vec!["models/shop", "model"]);
        // A project folder whose model folder is listed: as listed.
        let examples = read_exploration(
            &json!({ "project": "examples/shop", "goal": "Look" }),
            &planning,
        )
        .unwrap();
        assert_eq!(examples.project, "examples/shop/model");
        for (input, why) in [
            (json!({ "goal": "Look" }), "name the project"),
            (
                json!({ "project": "models/none", "goal": "Look" }),
                "holds no model at abc1234: the projects are examples/shop/model, model, models/garden, models/shop",
            ),
            (
                json!({ "project": "../model", "goal": "Look" }),
                "not a folder",
            ),
            (
                json!({ "project": "C:/model", "goal": "Look" }),
                "not a folder",
            ),
            (json!({ "project": "model", "goal": " " }), "goal"),
            (
                json!({ "project": "model", "goal": "Look", "start": { "view": "Requirements view" } }),
                "command's id",
            ),
            (
                json!({ "project": "model", "goal": "Look", "vary": ["models/none"] }),
                "`vary`",
            ),
        ] {
            let refused = read_exploration(&input, &planning).unwrap_err();
            assert!(refused.contains(why), "{refused}");
        }
        // Accepted: the projects it may explore are its first plan's.
        let recorded = planning.accept(plan.clone());
        assert_eq!(recorded, plan);
        planning.revision = "def5678".into();
        let later =
            read_exploration(&json!({ "project": "model", "goal": "Again" }), &planning).unwrap();
        assert_eq!(
            (later.project.as_str(), later.revision.as_str()),
            ("model", "def5678")
        );
        assert_eq!(
            planning.accept(later).vary,
            vec!["models/shop".to_string()],
            "the others stay permitted"
        );
        // Outside them, existing or not, the target is what it is told.
        for other in ["models/garden", "models/none"] {
            let refused =
                read_exploration(&json!({ "project": other, "goal": "Elsewhere" }), &planning)
                    .unwrap_err();
            assert!(
                refused.contains("this objective explores model at def5678")
                    && refused.contains("name model or models/shop"),
                "{refused}"
            );
        }
        assert!(
            read_exploration(
                &json!({ "project": "model", "goal": "Wider", "vary": ["models/garden"] }),
                &planning
            )
            .is_err(),
            "a later plan cannot widen its projects"
        );
        // The tool asks for both.
        let schema = submit_exploration();
        let required = schema["input_schema"]["required"].as_array().unwrap();
        assert!(required.contains(&json!("project")) && required.contains(&json!("goal")));
    }

    /// The W13.7 repair: a child explores a project its parent may explore:
    /// the one the lead names, or else the parent's target's (its scope and
    /// start with it, on that project only, and no other projects); before
    /// the parent has a target the lead must name it.
    #[test]
    fn a_child_explores_its_parents_target_or_a_project_the_lead_names() {
        let projects = vec!["model".to_string(), "models/shop".into()];
        let mut planning = Planning {
            projects,
            revision: "abc1234".into(),
            target: None,
            given: Vec::new(),
        };
        let unbound = planning.child(None, "Look closer").unwrap_err();
        assert!(
            unbound.contains("name the child's `project` (model, models/shop)")
                && unbound.contains("submit_exploration first"),
            "{unbound}"
        );
        let named = planning.child(Some("models/shop"), "Look closer").unwrap();
        assert_eq!(
            (named.project.as_str(), named.goal.as_str()),
            ("models/shop", "Look closer")
        );
        // Given to a child before the first plan: kept among the plan's
        // projects, so what the child finds is the objective's.
        planning.gave("models/shop");
        planning.accept(Target {
            project: "model".into(),
            revision: "abc1234".into(),
            goal: "Look".into(),
            scope: vec!["A::b".into()],
            start: crate::explore::Start {
                view: Some("requirements-view".into()),
                select: None,
            },
            vary: Vec::new(),
            hypotheses: Vec::new(),
        });
        assert_eq!(
            planning.target.as_ref().unwrap().vary,
            vec!["models/shop".to_string()]
        );
        let inherited = planning.child(None, "Look closer").unwrap();
        assert_eq!(inherited.project, "model");
        assert_eq!(inherited.scope, vec!["A::b".to_string()]);
        assert!(inherited.vary.is_empty(), "a child explores one project");
        let other = planning.child(Some("models/shop"), "Look there").unwrap();
        assert!(other.scope.is_empty() && other.start.is_empty());
        assert!(planning.child(Some("models/none"), "x").is_err());
    }

    /// The W13.7 repair, E3: a proposal fixing a hypothesis's finding
    /// serves the requirement that governs it; a hypothesis names its
    /// requirement or the intended behaviour, its workflow and what is
    /// expected.
    #[test]
    fn a_hypothesis_names_what_governs_it_and_a_fix_serves_it() {
        let proposal = Proposal {
            serves: vec!["Shop::Fast".into()],
            ..read(
                &aligned(json!({
                    "title": "Fix it", "kind": "correctness", "why": "x.rs:3",
                    "parts": [], "plan": ["a"],
                    "criteria": [{ "id": "c1", "statement": "s", "check": { "kind": "command", "program": ["cargo", "test", "-p", "agq-x", "t"] } }]
                })),
                &[],
            )
            .unwrap()
        };
        assert!(serves_the_finding(&proposal, None).is_ok());
        assert!(serves_the_finding(&proposal, Some("Shop::Fast")).is_ok());
        let other = serves_the_finding(&proposal, Some("Shop::Counted")).unwrap_err();
        assert!(other.contains("contradicts Shop::Counted"), "{other}");
        let planning = Planning {
            projects: vec!["model".into()],
            revision: "abc".into(),
            target: None,
            given: Vec::new(),
        };
        let plan = |hypothesis: Value| {
            read_exploration(
                &json!({ "project": "model", "goal": "Check the counts", "hypotheses": [hypothesis] }),
                &planning,
            )
        };
        let read = plan(json!({
            "claim": "The headline counts agree with the rows",
            "requirement": " Shop::Counted ",
            "workflow": "Open the Requirements view",
            "expected": "3 of 4 hold",
        }))
        .unwrap();
        let hypothesis = &read.hypotheses[0];
        assert_eq!(hypothesis.requirement.as_deref(), Some("Shop::Counted"));
        assert!(
            hypothesis.requirement_text.is_none(),
            "the Orchestrator reads it"
        );
        assert!(
            plan(json!({ "claim": "c", "behaviour": "b", "workflow": "w", "expected": "e" }))
                .is_ok()
        );
        for (wrong, why) in [
            (
                json!({ "claim": "c", "workflow": "w", "expected": "e" }),
                "requirement",
            ),
            (
                json!({ "claim": "c", "behaviour": "b", "expected": "e" }),
                "`workflow`",
            ),
            (
                json!({ "claim": " ", "behaviour": "b", "workflow": "w", "expected": "e" }),
                "`claim`",
            ),
            (
                json!({ "claim": "c", "behaviour": "b", "workflow": "w" }),
                "`expected`",
            ),
        ] {
            let refused = plan(wrong).unwrap_err();
            assert!(
                refused.contains("hypothesis 1") && refused.contains(why),
                "{refused}"
            );
        }
        let six: Vec<Value> = (0..6)
            .map(|_| json!({ "claim": "c", "behaviour": "b", "workflow": "w", "expected": "e" }))
            .collect();
        assert!(
            read_exploration(
                &json!({ "project": "model", "goal": "g", "hypotheses": six }),
                &planning
            )
            .unwrap_err()
            .contains("at most 5")
        );
    }

    /// C-55: a review states both judgments; a blank one is refused.
    #[test]
    fn a_review_states_its_judgments() {
        let review = json!({
            "verdict": "approve", "findings": [], "test_changes_accepted": false,
            "traceability": "none listed", "purpose": "It still serves the purpose."
        });
        assert!(read_review(&review).is_ok());
        for (field, value) in [
            ("traceability", json!("  ")),
            ("purpose", json!("")),
            ("purpose", Value::Null),
            ("verdict", json!("maybe")),
        ] {
            let mut wrong = review.clone();
            wrong[field] = value;
            assert!(read_review(&wrong).unwrap_err().contains(field), "{field}");
        }
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
        let base = aligned(json!({
            "title": "Fix the gap", "kind": "correctness", "why": "x.rs:3",
            "parts": [], "plan": ["a"],
            "criteria": [{ "id": "c1", "statement": "s", "check": { "kind": "command", "program": ["git", "push", "origin", "HEAD:main"] } }]
        }));
        assert!(
            read(&base, &[]).unwrap_err().contains("criterion c1"),
            "a criterion cannot push"
        );
        let mut empty = base.clone();
        empty["criteria"] = json!([{ "id": "c1", "statement": "s", "check": { "kind": "observation", "expect": {} } }]);
        assert!(
            read(&empty, &[]).unwrap_err().contains("criterion c1"),
            "an observation must expect something"
        );
        let mut misspelt = base.clone();
        misspelt["criteria"] = json!([{ "id": "c1", "statement": "s", "check": { "kind": "observation", "expect": { "status_contains": "x" } } }]);
        assert!(
            read(&misspelt, &[]).unwrap_err().contains("criterion c1"),
            "unknown keys are refused"
        );
        let mut fine = base.clone();
        fine["criteria"] = json!([{ "id": "c1", "statement": "s", "check": { "kind": "observation", "expect": { "statusContains": "x" } } }]);
        assert!(read(&fine, &[]).is_ok());
    }
}
