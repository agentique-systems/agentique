//! Scenario A steps A1–A4, A8 and A9 live and without the Studio's window
//! (ROADMAP §2.1, §2.2): a real project in git, the real turn loop, the
//! Operator's part scripted.
//!
//! ```text
//! cargo run -p agq-assistant --example scenario_a -- <folder outside the repository>
//! ```
//!
//! A1 creates an empty project and describes the URL shortener; A2 counts
//! what appears and what the Assistant asked; A3 edits by hand and in words;
//! A4 locks the API and the store; A8 asks for expiring links and refuses
//! every lock confirmation; A9 checkpoints, closes, reopens and compares the
//! model, locks, history and conversation. Every call passes the spend guard
//! (`support/spend.rs`). Writes `report.md` into the folder; exits with 1 if
//! a check fails.

#[path = "support/spend.rs"]
mod spend;

use agq_assistant::tools::{self, Prepared};
use agq_assistant::{
    Conversation, Entry, ModelChoice, StreamEvent, ToolCall, ToolResult, TurnEvent, turn,
};
use agq_language::{ElementId, ElementKind, print};
use agq_library::Library;
use agq_system_state::{Actor, ApplyError, Change, Operation, Project, Rejection};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[derive(Default)]
struct Record {
    applied: usize,
    questions: Vec<String>,
    lock_prompts: usize,
    replies: Vec<String>,
}

struct Operator {
    /// Allow changes to locked elements.
    allow_locks: bool,
}

fn main() {
    let folder = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("give a folder outside the repository"),
    );
    let choice = ModelChoice::from_env();
    if !choice.has_key() {
        println!("{}", choice.missing_key_message());
        std::process::exit(2);
    }
    let guard = match spend::SpendGuard::start("scenario-a", &choice.model, 120, 16_000) {
        Ok(guard) => guard,
        Err(why) => {
            println!("{why}");
            std::process::exit(2);
        }
    };
    let project_folder = folder.join("url-shortener");
    let _ = std::fs::remove_dir_all(&project_folder);
    std::fs::create_dir_all(&project_folder).expect("the project folder can be made");
    let mut model = spend::GuardedModel {
        inner: {
            let mut model =
                agq_assistant::ProviderModel::new(choice.model.clone(), choice.effort.clone());
            model.max_output_tokens = 16_000;
            Box::new(model)
        },
        guard: guard.clone(),
        failures: 0,
    };
    let mut lines = vec![
        format!("# Scenario A, headless, on {}", choice.label()),
        String::new(),
    ];
    let mut checks: Vec<(String, bool)> = Vec::new();

    // A1: a new project and an idea in ordinary words.
    let mut project = Project::create(&project_folder, "UrlShortener").expect("A1: create");
    let mut conversation = Conversation::default();
    let mut record = Record::default();
    let say = |text: &str,
               project: &mut Project,
               conversation: &mut Conversation,
               record: &mut Record,
               operator: &Operator,
               model: &mut spend::GuardedModel| {
        conversation.entries.push(Entry::Operator {
            text: text.to_string(),
        });
        let mut reply = String::new();
        let mut replies = Vec::new();
        turn::run(
            model,
            conversation,
            &mut |call| execute(project, record, operator, call),
            &mut |event| match event {
                TurnEvent::Stream(StreamEvent::Text(text)) => reply.push_str(&text),
                TurnEvent::Entry(Entry::Assistant { .. }) if !reply.trim().is_empty() => {
                    replies.push(std::mem::take(&mut reply))
                }
                _ => {}
            },
            &AtomicBool::new(false),
        );
        if !reply.trim().is_empty() {
            replies.push(reply);
        }
        record.replies.extend(replies);
    };
    let operator = Operator { allow_locks: true };
    say(
        "I want a URL shortener: an HTTP API that shortens long URLs and resolves short codes, a store for the links, and click statistics. Keep it small.",
        &mut project,
        &mut conversation,
        &mut record,
        &operator,
        &mut model,
    );
    // A2: what appeared, and whether it asked.
    let parts = count(&project, ElementKind::PartDef);
    lines.push(format!(
        "A1–A2: {} changes applied, {} part defs, {} problems; questions: {:?}",
        record.applied,
        parts,
        project.state().diagnostics().len(),
        record.questions
    ));
    checks.push((
        "A2: the architecture appears (changes applied, 3+ part defs)".into(),
        record.applied > 0 && parts >= 3,
    ));
    checks.push((
        "A2: no problems left".into(),
        project.state().diagnostics().is_empty(),
    ));

    // A3: by hand (a doc on the first part def) and in words.
    let first_part = project
        .state()
        .tree()
        .walk()
        .into_iter()
        .find(|id| project.state().tree()[*id].kind == ElementKind::PartDef)
        .expect("a part def");
    let by_hand = Change::new(
        Actor::Operator,
        "Describe the first part",
        vec![Operation::Set {
            element: first_part,
            property: agq_system_state::Property::Doc(Some(
                "Edited by hand in Scenario A, step A3.".into(),
            )),
        }],
    );
    let hand = project.apply(by_hand).is_ok();
    let before = record.applied;
    say(
        "Add a requirement that every short code maps to exactly one long URL, and say which part satisfies it.",
        &mut project,
        &mut conversation,
        &mut record,
        &operator,
        &mut model,
    );
    let requirements =
        count(&project, ElementKind::RequirementDef) + count(&project, ElementKind::Requirement);
    lines.push(format!(
        "A3: edit by hand applied: {hand}; in words: {} changes, {requirements} requirements, {} problems",
        record.applied - before,
        project.state().diagnostics().len()
    ));
    checks.push(("A3: the edit by hand applies".into(), hand));
    checks.push(("A3: a requirement added in words".into(), requirements > 0));

    // A4: lock the API and the store (every part def whose name says so).
    let locked: Vec<ElementId> = project
        .state()
        .tree()
        .walk()
        .into_iter()
        .filter(|id| project.state().tree()[*id].kind == ElementKind::PartDef)
        .filter(|id| {
            project
                .state()
                .tree()
                .effective_name(*id)
                .is_some_and(|name| {
                    let name = name.to_lowercase();
                    ["api", "store", "storage", "repository"]
                        .iter()
                        .any(|word| name.contains(word))
                })
        })
        .collect();
    let lock = Change::new(
        Actor::Operator,
        "Lock the API and the store",
        locked
            .iter()
            .map(|element| Operation::Lock { element: *element })
            .collect(),
    );
    let locked_ok = !locked.is_empty() && project.apply(lock).is_ok();
    let locked_text: Vec<String> = locked
        .iter()
        .map(|id| agq_language::print_element(project.state().tree(), *id).unwrap_or_default())
        .collect();
    lines.push(format!(
        "A4: locked {:?}: {locked_ok}",
        locked
            .iter()
            .map(|id| project.state().tree().qualified_name(*id))
            .collect::<Vec<_>>()
    ));
    checks.push(("A4: the API and the store are locked".into(), locked_ok));

    // A8: a loosely worded idea; every lock confirmation is refused.
    let refusing = Operator { allow_locks: false };
    let (before, prompts) = (record.applied, record.lock_prompts);
    say(
        "Add expiring links.",
        &mut project,
        &mut conversation,
        &mut record,
        &refusing,
        &mut model,
    );
    let unchanged = locked.iter().zip(&locked_text).all(|(id, text)| {
        agq_language::print_element(project.state().tree(), *id).is_some_and(|now| now == *text)
    });
    let expiry = project.state().tree().walk().into_iter().any(|id| {
        project
            .state()
            .tree()
            .effective_name(id)
            .is_some_and(|name| name.to_lowercase().contains("expir"))
    });
    lines.push(format!(
        "A8: {} changes, {} lock prompts (all refused), locked elements unchanged: {unchanged}, expiry modelled: {expiry}; last reply: {}",
        record.applied - before,
        record.lock_prompts - prompts,
        record.replies.last().map(|reply| reply.chars().take(600).collect::<String>()).unwrap_or_default()
    ));
    checks.push((
        "A8: refused locks leave the locked elements unchanged".into(),
        unchanged,
    ));

    // A9: checkpoint, close, reopen, compare.
    let text_before: String = print(project.state().tree())
        .into_iter()
        .map(|s| s.text)
        .collect();
    let locks_before = project.state().locks().clone();
    let _ = project.checkpoint("Scenario A, headless");
    let conversation_file = folder.join("conversation.json");
    conversation
        .save(&conversation_file)
        .expect("A9: save the conversation");
    let history = project.checkpoints().map(|c| c.len()).unwrap_or_default();
    drop(project);
    let reopened = Project::open(&project_folder).expect("A9: reopen");
    let text_after: String = print(reopened.state().tree())
        .into_iter()
        .map(|s| s.text)
        .collect();
    let conversation_after =
        Conversation::load(&conversation_file).expect("A9: read the conversation");
    let same = text_before == text_after
        && *reopened.state().locks() == locks_before
        && reopened.checkpoints().map(|c| c.len()).unwrap_or_default() == history
        && conversation_after == conversation;
    lines.push(format!(
        "A9: reopened with the same model, locks, {history} checkpoints and conversation: {same}"
    ));
    checks.push(("A9: everything as left after reopening".into(), same));

    let spent = guard.lock().unwrap().run_usd;
    lines.push(String::new());
    lines.push("| Check | Result |".into());
    lines.push("|---|---|".into());
    for (name, ok) in &checks {
        lines.push(format!(
            "| {name} | {} |",
            if *ok { "pass" } else { "FAIL" }
        ));
    }
    lines.push(String::new());
    lines.push(format!("Estimated cost: ${spent:.4}. Final model:"));
    lines.push(String::new());
    lines.push(format!("```\n{text_after}\n```"));
    let report = lines.join("\n");
    println!("{report}");
    std::fs::write(folder.join("report.md"), &report).expect("the report can be written");
    if checks.iter().any(|(_, ok)| !ok) {
        std::process::exit(1);
    }
}

fn count(project: &Project, kind: ElementKind) -> usize {
    let tree = project.state().tree();
    tree.walk()
        .into_iter()
        .filter(|id| tree[*id].kind == kind)
        .count()
}

/// The Studio's executor, with the Operator scripted: questions get "a
/// separate part" for statistics and "decide as you think best" otherwise.
fn execute(
    project: &mut Project,
    record: &mut Record,
    operator: &Operator,
    call: &ToolCall,
) -> ToolResult {
    match tools::prepare(
        project.state(),
        &Library::built_in_only(),
        &call.name,
        &call.input,
    ) {
        Prepared::Answer(text) => ToolResult::answer(text),
        Prepared::Invalid(message) => ToolResult::error(message),
        Prepared::Question { question, options } => {
            let answer = if question.to_lowercase().contains("statistic") {
                options
                    .iter()
                    .find(|option| option.to_lowercase().contains("separate"))
                    .cloned()
                    .unwrap_or_else(|| "A separate part.".to_string())
            } else if question.to_lowercase().contains("expir") {
                "Links expire after a duration chosen when they are created.".to_string()
            } else {
                "Decide as you think best, and tell me what you chose.".to_string()
            };
            record.questions.push(question);
            ToolResult::answer(answer)
        }
        Prepared::SaveToLibrary { .. } => {
            ToolResult::error("Not saved: this run keeps no My Library.")
        }
        Prepared::Change(mut change) => match project.apply(change.clone()) {
            Ok(event) => {
                record.applied += 1;
                ToolResult::applied(project.state(), &event)
            }
            Err(ApplyError::Rejection(Rejection::Locked { elements })) => {
                record.lock_prompts += 1;
                if operator.allow_locks {
                    change.confirmed = elements;
                    match project.apply(change) {
                        Ok(event) => {
                            record.applied += 1;
                            ToolResult::applied(project.state(), &event)
                        }
                        Err(error) => ToolResult::error(error.to_string()),
                    }
                } else {
                    ToolResult::rejected(project.state(), &Rejection::Locked { elements })
                }
            }
            Err(ApplyError::Rejection(rejection)) => {
                ToolResult::rejected(project.state(), &rejection)
            }
            Err(error) => ToolResult::error(error.to_string()),
        },
    }
}
