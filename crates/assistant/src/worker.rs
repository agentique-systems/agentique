//! An implementation worker (ROADMAP §4.15, W8.4): the Assistant with code
//! tools instead of architecture tools, working in one worktree for one
//! task. It reads and writes files only inside that worktree and never in
//! its protected paths; builds and tests go through the executor, which
//! refuses them unless the Operator allowed trusted-local execution. It
//! cannot change the model: when the model is wrong or not enough, it asks
//! the Operator for a contract change and stops.
//!
//! Repair is bounded: after [`MAX_ROUNDS`] rounds of checks, or
//! [`NO_PROGRESS`] rounds in a row without fewer failures, the checks tell
//! it to finish and say what still fails. The Studio verifies the working
//! copy itself afterwards; nothing the worker says about the checks is
//! taken on trust.

use crate::conversation::{Conversation, Entry, ToolResult};
use crate::model::Model;
use crate::turn::{ToolCall, Toolset, TurnEvent, run_with};
use agq_execution::Executor;
use agq_implementation::task::{Brief, verify};
use agq_implementation::{Link, LinkKind, Links};
use agq_language::Tree;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub const LIST_FILES: &str = "list_files";
pub const READ_CODE: &str = "read_code";
pub const WRITE_CODE: &str = "write_code";
pub const RUN_CHECKS: &str = "run_checks";
pub const LINK_CODE: &str = "link_code";
pub const REQUEST_CONTRACT_CHANGE: &str = "request_contract_change";
pub const FINISH_IMPLEMENTATION: &str = "finish_implementation";

/// Rounds of checks a task may take.
pub const MAX_ROUNDS: usize = 6;
/// Rounds in a row without fewer failures before the worker must stop.
pub const NO_PROGRESS: usize = 3;
/// Model calls a task may take.
pub const MAX_WORKER_CALLS: usize = 80;

/// The worker's tools.
pub fn definitions() -> Value {
    let name = |description: &str| json!({ "type": "string", "description": description });
    let kinds: Vec<&str> = LinkKind::ALL.iter().map(|k| k.label()).collect();
    json!([
        {
            "name": LIST_FILES,
            "description": "The files in a folder of the repository (recursively, at most 400).",
            "input_schema": { "type": "object", "properties": { "folder": name("A folder relative to the repository root; empty for the root.") }, "additionalProperties": false }
        },
        {
            "name": READ_CODE,
            "description": "A file's text.",
            "input_schema": { "type": "object", "properties": { "path": name("Relative to the repository root, e.g. \"src/store.rs\".") }, "required": ["path"], "additionalProperties": false }
        },
        {
            "name": WRITE_CODE,
            "description": "Write a file's whole text (created if it does not exist). Protected paths cannot be written.",
            "input_schema": { "type": "object", "properties": { "path": name("Relative to the repository root."), "text": name("The file's complete new text.") }, "required": ["path", "text"], "additionalProperties": false }
        },
        {
            "name": RUN_CHECKS,
            "description": "Build the code, run the implementation checks (module boundaries, contract shapes, linked tests) and the task's scenarios through the harness. Says what fails and why.",
            "input_schema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": LINK_CODE,
            "description": "Link code to a model element, so checks and the Operator can find it: the module that implements a part, the type that implements an item or enum def, a test that checks an element.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "element": name("The model element's qualified name, e.g. \"UrlShortener::ShortLink\"."),
                    "kind": { "type": "string", "enum": kinds },
                    "path": name("The file, relative to the repository root."),
                    "symbol": name("The type, function or test in it, e.g. \"ShortLink\".")
                },
                "required": ["element", "kind", "path"],
                "additionalProperties": false
            }
        },
        {
            "name": REQUEST_CONTRACT_CHANGE,
            "description": "Ask the Operator to change the model: the contract is wrong, contradictory or not enough to implement. Then stop and finish; do not work around it in code.",
            "input_schema": { "type": "object", "properties": { "element": name("The element concerned."), "reason": name("What is wrong and what change would fix it.") }, "required": ["reason"], "additionalProperties": false }
        },
        {
            "name": FINISH_IMPLEMENTATION,
            "description": "End the task with an honest summary: what was implemented, what passes, what still fails and why.",
            "input_schema": { "type": "object", "properties": { "summary": name("The summary for the Operator.") }, "required": ["summary"], "additionalProperties": false }
        }
    ])
}

/// The worker's system prompt and tools.
pub fn toolset() -> Toolset {
    Toolset {
        system: include_str!("../skills/worker.md").to_string(),
        definitions: definitions(),
    }
}

/// One task's worker: its working copy, and what it proposed.
pub struct Worker {
    tree: Tree,
    base: Links,
    pub brief: Brief,
    executor: Executor,
    cancel: Arc<AtomicBool>,
    /// Links it added, applied to the model only when the Operator
    /// integrates the patch.
    pub proposed: Vec<Link>,
    pub contract_requests: Vec<String>,
    pub summary: Option<String>,
    pub rounds: usize,
    best: Option<usize>,
    stale: usize,
}

impl Worker {
    /// A worker for `brief` in the executor's (writable) scope.
    pub fn new(
        tree: Tree,
        links: Links,
        brief: Brief,
        executor: Executor,
        cancel: Arc<AtomicBool>,
    ) -> Worker {
        Worker {
            tree,
            base: links,
            brief,
            executor,
            cancel,
            proposed: Vec::new(),
            contract_requests: Vec::new(),
            summary: None,
            rounds: 0,
            best: None,
            stale: 0,
        }
    }

    /// The links with those it proposed.
    pub fn links(&self) -> Links {
        let mut links = self.base.clone();
        for link in &self.proposed {
            if !links.links.contains(link) {
                links.links.push(link.clone());
            }
        }
        links
    }

    /// Carries out one checked call.
    pub fn execute(&mut self, call: &ToolCall) -> ToolResult {
        if self.summary.is_some() {
            return ToolResult::error("Not run: the task is finished.");
        }
        let text = |field: &str| {
            call.input
                .get(field)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        match call.name.as_str() {
            LIST_FILES => match self.executor.list(&text("folder"), 400) {
                Ok(files) if files.is_empty() => ToolResult::answer("No files there."),
                Ok(files) => ToolResult::answer(files.join("\n")),
                Err(refusal) => ToolResult::error(refusal.to_string()),
            },
            READ_CODE => match self.executor.read(&text("path")) {
                Ok(code) => ToolResult::answer(crate::tools::cap(code)),
                Err(refusal) => ToolResult::error(refusal.to_string()),
            },
            WRITE_CODE => {
                let path = text("path");
                let body = text("text");
                if self.brief.protected.iter().any(|p| {
                    let path = path.replace('\\', "/");
                    path == *p || path.starts_with(&format!("{p}/"))
                }) {
                    return ToolResult::error(format!("Not written: `{path}` is protected."));
                }
                match self.executor.write(&path, &body) {
                    Ok(()) => ToolResult::answer(format!(
                        "Wrote {} lines to {path}.",
                        body.lines().count()
                    )),
                    Err(refusal) => ToolResult::error(format!("Not written: {refusal}")),
                }
            }
            RUN_CHECKS => self.run_checks(),
            LINK_CODE => self.link(call),
            REQUEST_CONTRACT_CHANGE => {
                let element = text("element");
                let reason = text("reason");
                self.contract_requests.push(if element.is_empty() {
                    reason
                } else {
                    format!("{element}: {reason}")
                });
                ToolResult::answer(
                    "Recorded for the Operator, who decides on the model. Stop here: call finish_implementation and say what is blocked.",
                )
            }
            FINISH_IMPLEMENTATION => {
                self.summary = Some(text("summary"));
                ToolResult::answer(
                    "Finished. The Studio verifies the working copy itself and shows the patch to the Operator.",
                )
            }
            other => ToolResult::error(format!("there is no tool called `{other}`")),
        }
    }

    fn run_checks(&mut self) -> ToolResult {
        if self.rounds >= MAX_ROUNDS {
            return ToolResult::error(format!(
                "Not run: the repair budget ({MAX_ROUNDS} rounds) is used up. Call finish_implementation and say what still fails."
            ));
        }
        self.rounds += 1;
        let verification = verify(
            &self.tree,
            &self.links(),
            &self.brief,
            &self.executor,
            self.cancel.clone(),
        );
        let failures = verification.failures();
        let mut text = format!(
            "Round {} of {MAX_ROUNDS}.\n{}",
            self.rounds,
            verification.describe()
        );
        if failures == 0 {
            text.push_str("\nEverything passes: call finish_implementation with a summary.");
            return ToolResult::answer(text);
        }
        if self.best.is_none_or(|best| failures < best) {
            self.best = Some(failures);
            self.stale = 0;
        } else {
            self.stale += 1;
        }
        if self.stale >= NO_PROGRESS {
            text.push_str(&format!(
                "\nNo progress in {NO_PROGRESS} rounds: stop, call finish_implementation and say what still fails and why."
            ));
        } else if self.rounds >= MAX_ROUNDS {
            text.push_str("\nThat was the last round: call finish_implementation now and say what still fails.");
        }
        ToolResult::answer(crate::tools::cap(text))
    }

    fn link(&mut self, call: &ToolCall) -> ToolResult {
        let text = |field: &str| {
            call.input
                .get(field)
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        let Some(element) = text("element").and_then(|name| self.tree.find(&name)) else {
            return ToolResult::error(
                "Not linked: there is no such element; use its qualified name.",
            );
        };
        let Some(kind) =
            text("kind").and_then(|k| LinkKind::ALL.into_iter().find(|l| l.label() == k))
        else {
            return ToolResult::error("Not linked: unknown kind.");
        };
        let path = text("path").unwrap_or_default();
        if let Err(refusal) = self.executor.read(&path) {
            return ToolResult::error(format!("Not linked: {refusal}"));
        }
        let mut links = Links::default();
        links.add(&self.tree, element, kind, &path, text("symbol").as_deref());
        let link = links.links.remove(0);
        if !self.proposed.contains(&link) {
            self.proposed.push(link);
        }
        ToolResult::answer(format!(
            "Linked {} to {path} (applied to the model when the Operator integrates the patch).",
            self.tree.qualified_name(element)
        ))
    }
}

/// Runs the task: the brief is the first message, and the worker calls its
/// tools until it finishes, is stopped, or runs out of model calls.
pub fn run_worker(
    model: &mut dyn Model,
    worker: &mut Worker,
    on_event: &mut dyn FnMut(TurnEvent),
    stop: &AtomicBool,
) -> Conversation {
    let mut conversation = Conversation::default();
    conversation.entries.push(Entry::Operator {
        text: worker.brief.text.clone(),
    });
    let mut execute = |call: &ToolCall| worker.execute(call);
    run_with(
        model,
        &mut conversation,
        &toolset(),
        MAX_WORKER_CALLS,
        &mut execute,
        on_event,
        stop,
    );
    conversation
}
