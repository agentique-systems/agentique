//! An implementation worker (ROADMAP §4.15, W8.4): the Assistant with code
//! tools instead of architecture tools, working in one worktree for one
//! task. It reads and writes files only inside that worktree and never in
//! its protected paths; builds and tests go through the executor, which
//! refuses them unless the Operator allowed trusted-local execution. Given
//! [`Worker::with_model`], it changes only its worktree's copy of the model,
//! through System State operations (locked elements refused), and those
//! changes stay proposed until the Operator integrates the task; a contract
//! change it cannot make there goes back to the Operator, and it stops.
//!
//! Repair is bounded: after [`MAX_ROUNDS`] rounds of checks, or
//! [`NO_PROGRESS`] rounds in a row without fewer failures, the checks tell
//! it to finish and say what still fails. The Studio verifies the working
//! copy itself afterwards; nothing the worker says about the checks is
//! taken on trust.

use crate::conversation::{Conversation, Entry, ToolResult};
use crate::turn::{ToolCall, Toolset, TurnEvent};
use agq_execution::Executor;
use agq_implementation::task::{Brief, verify};
use agq_implementation::{Link, LinkKind, Links};
use agq_language::Tree;
use agq_simulation::Verdict;
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
pub const SEARCH_CODE: &str = "search_code";
pub const EDIT_CODE: &str = "edit_code";
pub const RUN_PROGRAM: &str = "run_program";

/// Hits `search_code` returns at most.
const SEARCH_LIMIT: usize = 200;

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
            "description": "A file's text, or the lines from `from_line` to `to_line` (1-based, inclusive) with their numbers. Read a range of a long file instead of all of it.",
            "input_schema": { "type": "object", "properties": {
                "path": name("Relative to the repository root, e.g. \"src/store.rs\"."),
                "from_line": { "type": "number", "description": "First line to read (1-based)." },
                "to_line": { "type": "number", "description": "Last line to read (inclusive)." }
            }, "required": ["path"], "additionalProperties": false }
        },
        {
            "name": SEARCH_CODE,
            "description": "Find a text in the repository's files (not build output or installed packages): each hit as path:line: the line. At most 200 hits; narrow by folder.",
            "input_schema": { "type": "object", "properties": {
                "text": name("The text to find, as written."),
                "folder": name("A folder to search in; empty for the whole repository."),
                "ignore_case": { "type": "boolean", "description": "Match without regard to case." }
            }, "required": ["text"], "additionalProperties": false }
        },
        {
            "name": EDIT_CODE,
            "description": "Replace one passage of a file: `old_text` must occur exactly once (include enough lines to make it unique); it becomes `new_text`. Protected paths cannot be changed.",
            "input_schema": { "type": "object", "properties": {
                "path": name("Relative to the repository root."),
                "old_text": name("The passage as it is now, exactly."),
                "new_text": name("What it becomes.")
            }, "required": ["path", "old_text", "new_text"], "additionalProperties": false }
        },
        {
            "name": RUN_PROGRAM,
            "description": "Run one program in the repository for diagnostics: a Cargo build, check, test (with a filter), clippy or fmt, or one of the project's own check commands exactly as listed in the brief. It runs with the Operator's trusted-local permission, offline and without secrets. Says how it ended and the last lines of its output.",
            "input_schema": { "type": "object", "properties": {
                "program": { "type": "array", "items": { "type": "string" }, "description": "The program and its arguments, e.g. [\"cargo\", \"test\", \"-p\", \"agq-execution\", \"--offline\"]." }
            }, "required": ["program"], "additionalProperties": false }
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

/// The worker's system prompt and tools: its own, and the Assistant's model
/// tools (the same schemas) for the working copy's model.
pub fn toolset() -> Toolset {
    let mut definitions = definitions();
    let model_tools = [
        crate::tools::READ_MODEL,
        crate::tools::FIND_ELEMENTS,
        crate::tools::GET_PROBLEMS,
        crate::tools::APPLY_CHANGES,
    ];
    if let (Some(own), Some(assistant)) = (
        definitions.as_array_mut(),
        crate::tools::definitions().as_array(),
    ) {
        own.extend(
            assistant
                .iter()
                .filter(|d| model_tools.contains(&d["name"].as_str().unwrap_or_default()))
                .cloned(),
        );
    }
    Toolset {
        system: include_str!("../skills/worker.md").to_string(),
        definitions,
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
    /// The working copy's model, changed only through System State
    /// operations (the worker cannot write model files): its changes are
    /// proposed, reviewed with the code, and integrated with it.
    model: Option<crate::model_tools::WorkingModel>,
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
            model: None,
        }
    }

    /// Lets the worker read and change the model of the working copy at
    /// `folder` (the worktree's project folder) with the Assistant's model
    /// tools. Locked elements are refused, as for any unconfirmed change.
    pub fn with_model(mut self, folder: impl Into<std::path::PathBuf>) -> Worker {
        self.model = Some(crate::model_tools::WorkingModel::new(folder));
        self
    }

    /// A model tool on the working copy's model.
    fn model_tool(&mut self, call: &ToolCall) -> ToolResult {
        match &mut self.model {
            Some(model) => model.execute(call),
            None => crate::conversation::ToolResult::error(
                "This task has no model of its own to change: use request_contract_change.",
            ),
        }
    }

    /// Closes the working copy's model (its files are saved with each
    /// change; this releases its lock before the task commit), and returns
    /// it if the worker read or changed it: the model the task is checked
    /// against.
    pub fn close_model(&mut self) -> Option<Tree> {
        self.model.as_mut()?.close()
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
                Ok(code) => {
                    let line = |field: &str| call.input.get(field).and_then(Value::as_u64);
                    match (line("from_line"), line("to_line")) {
                        (None, None) => ToolResult::answer(crate::tools::cap(code)),
                        (from, to) => {
                            let from = from.unwrap_or(1).max(1) as usize;
                            let to = to.map_or(usize::MAX, |t| t as usize);
                            let total = code.lines().count();
                            let shown: Vec<String> = code
                                .lines()
                                .enumerate()
                                .skip(from - 1)
                                .take(to.saturating_sub(from - 1))
                                .map(|(i, l)| format!("{:>5} {l}", i + 1))
                                .collect();
                            ToolResult::answer(crate::tools::cap(format!(
                                "{} (lines {from} to {} of {total})\n{}",
                                text("path"),
                                (from - 1 + shown.len()).max(from),
                                shown.join("\n")
                            )))
                        }
                    }
                }
                Err(refusal) => ToolResult::error(refusal.to_string()),
            },
            SEARCH_CODE => self.search(
                &text("text"),
                &text("folder"),
                call.input
                    .get("ignore_case")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
            EDIT_CODE => {
                let path = text("path");
                if self.executor.scope().is_protected(&path) {
                    return ToolResult::error(format!("Not changed: `{path}` is protected."));
                }
                let current = match self.executor.read(&path) {
                    Ok(current) => current,
                    Err(refusal) => return ToolResult::error(format!("Not changed: {refusal}")),
                };
                let (old, new) = (text("old_text"), text("new_text"));
                match current.matches(old.as_str()).count() {
                    0 => ToolResult::error(format!(
                        "Not changed: the passage is not in {path}; read it again."
                    )),
                    1 if !old.is_empty() => {
                        match self.executor.write(&path, &current.replacen(&old, &new, 1)) {
                            Ok(()) => ToolResult::answer(format!(
                                "Changed {path}: {} line(s) became {} line(s).",
                                old.lines().count(),
                                new.lines().count()
                            )),
                            Err(refusal) => ToolResult::error(format!("Not changed: {refusal}")),
                        }
                    }
                    n => ToolResult::error(format!(
                        "Not changed: the passage occurs {n} times in {path}; include more lines so it occurs once."
                    )),
                }
            }
            RUN_PROGRAM => self.run_program(call),
            WRITE_CODE => {
                let path = text("path");
                let body = text("text");
                if self.executor.scope().is_protected(&path) {
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
            crate::tools::READ_MODEL
            | crate::tools::FIND_ELEMENTS
            | crate::tools::GET_PROBLEMS
            | crate::tools::APPLY_CHANGES => self.model_tool(call),
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

    /// Lines of the repository's files that contain `needle`.
    fn search(&self, needle: &str, folder: &str, ignore_case: bool) -> ToolResult {
        if needle.is_empty() {
            return ToolResult::error("Nothing to find: give the text to search for.");
        }
        let files = match self.executor.list(folder, 20_000) {
            Ok(files) => files,
            Err(refusal) => return ToolResult::error(refusal.to_string()),
        };
        let wanted = if ignore_case {
            needle.to_lowercase()
        } else {
            needle.to_string()
        };
        let mut hits = Vec::new();
        'files: for file in files {
            let Ok(text) = self.executor.read(&file) else {
                continue; // not text
            };
            for (number, line) in text.lines().enumerate() {
                let found = if ignore_case {
                    line.to_lowercase().contains(&wanted)
                } else {
                    line.contains(&wanted)
                };
                if found {
                    let shown: String = line.trim().chars().take(200).collect();
                    hits.push(format!("{file}:{}: {shown}", number + 1));
                    if hits.len() >= SEARCH_LIMIT {
                        hits.push(format!(
                            "… stopped at {SEARCH_LIMIT} hits: narrow the search by folder or text"
                        ));
                        break 'files;
                    }
                }
            }
        }
        if hits.is_empty() {
            ToolResult::answer(format!("`{needle}` is not in the files searched."))
        } else {
            ToolResult::answer(crate::tools::cap(hits.join("\n")))
        }
    }

    /// One allowed program, for diagnostics (not a verification).
    fn run_program(&mut self, call: &ToolCall) -> ToolResult {
        let list: Vec<String> = call
            .input
            .get("program")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let Some(program) = agq_execution::Program::from_list(&list) else {
            return ToolResult::error("Not run: give the program and its arguments.");
        };
        match self
            .executor
            .run(&program, "", std::time::Duration::from_secs(900))
        {
            Err(refusal) => ToolResult::error(format!("Not run: {refusal}")),
            Ok(finished) => {
                let output = format!("{}\n{}", finished.stdout, finished.stderr);
                ToolResult::answer(crate::tools::cap(format!(
                    "`{}` {}\n{}",
                    program.display(),
                    finished.summary(),
                    agq_execution::process::last_lines(output.trim(), 60)
                )))
            }
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
        // What the worker can repair: what failed or is blocked. Checks that
        // could not run here (nothing configured for them) are reported, and
        // stay not run in the Studio's own verification; they are no pass.
        let outcomes = verification.outcomes();
        let failures = outcomes
            .iter()
            .filter(|o| matches!(o.verdict, Verdict::Failed | Verdict::Blocked))
            .count()
            + verification
                .extra()
                .iter()
                .filter(|c| c.verdict == Verdict::Failed)
                .count();
        let mut text = format!(
            "Round {} of {MAX_ROUNDS}.\n{}",
            self.rounds,
            verification.describe()
        );
        if failures == 0 {
            if verification.passed() {
                text.push_str("\nEverything passes: call finish_implementation with a summary.");
            } else {
                let not_run: Vec<String> = outcomes
                    .iter()
                    .filter(|o| o.verdict != Verdict::Passed)
                    .map(|o| format!("{} ({})", o.name, o.message))
                    .collect();
                text.push_str(&format!(
                    "\nNothing that ran fails, but these required checks did not run: {}. You cannot make them run from here. Call finish_implementation and say plainly that they were not verified.",
                    not_run.join("; ")
                ));
            }
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
    runtime: &mut dyn crate::Runtime,
    worker: &mut Worker,
    on_event: &mut dyn FnMut(TurnEvent),
    stop: &AtomicBool,
) -> Conversation {
    let mut conversation = Conversation::default();
    conversation.entries.push(Entry::Operator {
        text: worker.brief.text.clone(),
    });
    let toolset = toolset();
    let mut execute = |call: &ToolCall| worker.execute(call);
    let mut execute = crate::runtime::checked(&toolset.definitions, &mut execute);
    runtime.run(
        &mut conversation,
        &toolset,
        MAX_WORKER_CALLS,
        &mut execute,
        on_event,
        stop,
    );
    conversation
}
