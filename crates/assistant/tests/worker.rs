//! The implementation worker (C-50, W8.4) with a scripted model, in a real
//! git worktree of a throwaway Rust crate: it writes only inside the
//! worktree and never its protected paths, its links wait for the
//! Operator, a contract change goes back to the Operator, the repair rounds
//! are bounded, and the Studio's own verification is what counts.

use agq_assistant::model::{Reply, ScriptedModel};
use agq_assistant::worker::{self, Worker};
use agq_execution::{Executor, Scope, git};
use agq_implementation::Links;
use agq_implementation::task::{brief, verify};
use agq_language::{Source, Tree, parse};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const MODEL: &str = "package Jobs {
    enum def Priority { low; high; }
    item def Job {
        attribute size : ScalarValues::Integer;
        attribute priority : Priority;
    }
    port def JobPort { in item job : Job; }
    part def Worker {
        doc /* Takes jobs. */
        port jobs : JobPort;
    }
}";

struct Folder(PathBuf);
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tool(id: &str, name: &str, input: Value) -> Value {
    json!({ "type": "tool_use", "id": id, "name": name, "input": input })
}

fn reply(content: Vec<Value>) -> Reply {
    let stop = if content.iter().any(|c| c["type"] == "tool_use") {
        "tool_use"
    } else {
        "end_turn"
    };
    Reply {
        content,
        stop_reason: stop.into(),
    }
}

/// A throwaway crate, committed, and a worktree of it for the task.
fn repository(name: &str) -> (Folder, PathBuf, git::Worktree) {
    let root = std::env::temp_dir().join(format!("agq-worker-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"jobs\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(repo.join("src/lib.rs"), "//! Jobs.\n").unwrap();
    std::fs::write(repo.join("README.md"), "Protected.\n").unwrap();
    git::init_and_commit(&repo, "Start").unwrap();
    let worktree = git::create_worktree(&repo, "task", &root.join("task")).unwrap();
    (Folder(root), repo, worktree)
}

fn tree() -> Tree {
    parse(&[Source::new("Jobs.sysml", MODEL)])
}

fn executor(worktree: &Path, protected: &[String], target: &Path) -> Executor {
    let protected: Vec<&str> = protected.iter().map(String::as_str).collect();
    let scope = Scope::writable(worktree, &["src", "tests", "Cargo.toml"], &protected).unwrap();
    Executor::new(scope)
        .trusted(true)
        .target_dir(target.to_path_buf())
}

#[test]
fn a_worker_implements_links_and_finishes_and_the_studio_verifies() {
    let (folder, repo, worktree) = repository("implements");
    let tree = tree();
    let links = Links {
        protected: vec!["README.md".into()],
        ..Default::default()
    };
    let worker_def = tree.find("Jobs::Worker").unwrap();
    let brief = brief(&tree, &links, worker_def, "Keep it small.").unwrap();
    assert!(brief.text.contains("item def Job"), "{}", brief.text);
    assert!(brief.text.contains("enum def Priority"), "{}", brief.text);
    assert!(brief.text.contains("The Operator adds: Keep it small."));
    assert!(brief.protected.contains(&"README.md".to_string()));
    let target = folder.0.join("target");
    let cancel = Arc::new(AtomicBool::new(false));
    let mut w = Worker::new(
        tree.clone(),
        links.clone(),
        brief.clone(),
        executor(&worktree.path, &brief.protected, &target),
        cancel.clone(),
    );
    let code = "//! Jobs.\n\n/// Jobs::Priority\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum Priority {\n    Low,\n    High,\n}\n\n/// Jobs::Job\n#[derive(Clone, Debug, PartialEq)]\npub struct Job {\n    pub size: i64,\n    pub priority: Priority,\n}\n";
    let model = ScriptedModel::new([
        reply(vec![tool("t1", worker::LIST_FILES, json!({}))]),
        reply(vec![
            tool(
                "t2",
                worker::WRITE_CODE,
                json!({ "path": "README.md", "text": "mine" }),
            ),
            tool(
                "t3",
                worker::WRITE_CODE,
                json!({ "path": "../escape.rs", "text": "x" }),
            ),
            tool(
                "t4",
                worker::WRITE_CODE,
                json!({ "path": "src/lib.rs", "text": code }),
            ),
        ]),
        reply(vec![
            tool(
                "t5",
                worker::LINK_CODE,
                json!({ "element": "Jobs::Job", "kind": "type", "path": "src/lib.rs", "symbol": "Job" }),
            ),
            tool(
                "t6",
                worker::LINK_CODE,
                json!({ "element": "Jobs::Priority", "kind": "type", "path": "src/lib.rs", "symbol": "Priority" }),
            ),
            tool(
                "t7",
                worker::LINK_CODE,
                json!({ "element": "Jobs::Worker", "kind": "module", "path": "src/lib.rs" }),
            ),
        ]),
        reply(vec![tool("t8", worker::RUN_CHECKS, json!({}))]),
        reply(vec![tool(
            "t9",
            worker::FINISH_IMPLEMENTATION,
            json!({ "summary": "Job and Priority, checked." }),
        )]),
        reply(vec![json!({ "type": "text", "text": "Done." })]),
    ]);
    let mut results = Vec::new();
    let conversation = worker::run_worker(
        &mut agq_assistant::LoopRuntime(Box::new(model)),
        &mut w,
        &mut |event| {
            if let agq_assistant::TurnEvent::ToolFinished(result) = event {
                results.push(result);
            }
        },
        &AtomicBool::new(false),
    );
    assert!(conversation.entries.len() > 5);
    // The protected file and the path outside the worktree were refused.
    assert!(
        results[1].is_error && results[1].content.contains("protected"),
        "{:?}",
        results[1]
    );
    assert!(results[2].is_error, "{:?}", results[2]);
    assert!(!results[3].is_error, "{:?}", results[3]);
    assert_eq!(
        std::fs::read_to_string(repo.join("README.md")).unwrap(),
        "Protected.\n"
    );
    // Links wait for the Operator; the checks passed.
    assert_eq!(w.proposed.len(), 3);
    let checks = &results[7];
    assert!(
        checks.content.contains("- Build: passed."),
        "{}",
        checks.content
    );
    assert!(
        checks
            .content
            .contains("Passed: every one of the 4 required check(s) passed."),
        "{}",
        checks.content
    );
    assert_eq!(w.summary.as_deref(), Some("Job and Priority, checked."));
    // The Studio verifies the working copy itself.
    let verified = verify(
        &tree,
        &w.links(),
        &brief,
        &executor(&worktree.path, &brief.protected, &target),
        cancel,
    );
    assert_eq!(verified.failures(), 0, "{}", verified.describe());
    // The patch is in the worktree only; the repository is as it was.
    let patch = git::patch(&worktree.path, &worktree.base).unwrap();
    let paths: Vec<&str> = patch.files.iter().map(|f| f.path.as_str()).collect();
    // The build writes the lock file; nothing else changed.
    assert!(paths.contains(&"src/lib.rs"), "{paths:?}");
    assert!(
        paths
            .iter()
            .all(|p| ["src/lib.rs", "Cargo.lock"].contains(p)),
        "{paths:?}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("src/lib.rs")).unwrap(),
        "//! Jobs.\n"
    );
    // Integrating commits it to the repository.
    let commit = git::integrate(
        &repo,
        &worktree.path,
        &worktree.base,
        "Implement Jobs::Worker",
    )
    .unwrap()
    .expect("integrated");
    assert_eq!(git::head(&repo).unwrap().commit, commit);
    assert!(
        std::fs::read_to_string(repo.join("src/lib.rs"))
            .unwrap()
            .contains("pub struct Job")
    );
}

#[test]
fn a_wrong_contract_goes_back_to_the_operator_and_repair_is_bounded() {
    let (folder, _repo, worktree) = repository("bounded");
    let tree = tree();
    let links = Links::default();
    let brief = brief(&tree, &links, tree.find("Jobs::Worker").unwrap(), "").unwrap();
    let target = folder.0.join("target");
    let mut w = Worker::new(
        tree.clone(),
        links,
        brief.clone(),
        executor(&worktree.path, &brief.protected, &target),
        Arc::new(AtomicBool::new(false)),
    );
    // A shape the model disagrees with, checked again and again.
    let wrong = "pub struct Job { pub size: String }\n";
    let mut script = vec![
        reply(vec![tool(
            "w",
            worker::WRITE_CODE,
            json!({ "path": "src/lib.rs", "text": wrong }),
        )]),
        reply(vec![tool(
            "l",
            worker::LINK_CODE,
            json!({ "element": "Jobs::Job", "kind": "type", "path": "src/lib.rs", "symbol": "Job" }),
        )]),
    ];
    for i in 0..worker::MAX_ROUNDS + 1 {
        script.push(reply(vec![tool(
            &format!("c{i}"),
            worker::RUN_CHECKS,
            json!({}),
        )]));
    }
    script.push(reply(vec![tool(
        "r",
        worker::REQUEST_CONTRACT_CHANGE,
        json!({ "element": "Jobs::Job", "reason": "size should be text" }),
    )]));
    script.push(reply(vec![tool(
        "f",
        worker::FINISH_IMPLEMENTATION,
        json!({ "summary": "Blocked on the contract." }),
    )]));
    script.push(reply(vec![tool(
        "x",
        worker::WRITE_CODE,
        json!({ "path": "src/lib.rs", "text": "late" }),
    )]));
    script.push(reply(vec![json!({ "type": "text", "text": "Stopped." })]));
    let model = ScriptedModel::new(script);
    let mut results = Vec::new();
    worker::run_worker(
        &mut agq_assistant::LoopRuntime(Box::new(model)),
        &mut w,
        &mut |event| {
            if let agq_assistant::TurnEvent::ToolFinished(result) = event {
                results.push(result);
            }
        },
        &AtomicBool::new(false),
    );
    let rounds: Vec<&agq_assistant::ToolResult> =
        results[2..2 + worker::MAX_ROUNDS + 1].iter().collect();
    assert!(
        rounds[0].content.contains("contract shape") || rounds[0].content.contains("Job"),
        "{}",
        rounds[0].content
    );
    assert!(
        rounds[worker::NO_PROGRESS].content.contains("No progress"),
        "{}",
        rounds[worker::NO_PROGRESS].content
    );
    // After the budget, checks no longer run.
    assert!(
        rounds[worker::MAX_ROUNDS].is_error,
        "{}",
        rounds[worker::MAX_ROUNDS].content
    );
    assert_eq!(w.rounds, worker::MAX_ROUNDS);
    assert_eq!(w.contract_requests, ["Jobs::Job: size should be text"]);
    // Nothing runs after it finished.
    assert!(results.last().unwrap().is_error);
    assert_eq!(
        std::fs::read_to_string(worktree.path.join("src/lib.rs")).unwrap(),
        wrong
    );
}

/// The coding tools (W10.4): a search, a ranged read, an exact edit (refused
/// when the passage is not unique, and on a protected path in any
/// spelling), and one allowed program for diagnostics (another is refused).
#[test]
fn the_coding_tools_search_read_ranges_edit_exactly_and_run_only_allowed_programs() {
    let (_folder, _repo, worktree) = repository("tools");
    std::fs::write(
        worktree.path.join("src/lib.rs"),
        "//! Jobs.\npub fn size() -> u32 { 1 }\npub fn other() -> u32 { 1 }\n",
    )
    .unwrap();
    let tree = tree();
    let element = tree.find("Jobs::Worker").unwrap();
    let mut brief = brief(&tree, &Links::default(), element, "").unwrap();
    brief.protected.push("README.md".into());
    let target = worktree.path.join("../target");
    let cancel = Arc::new(AtomicBool::new(false));
    let mut w = Worker::new(
        tree,
        Links::default(),
        brief.clone(),
        executor(&worktree.path, &brief.protected, &target),
        cancel,
    );
    let call = |name: &str, input: Value| agq_assistant::turn::ToolCall {
        id: "x".into(),
        name: name.into(),
        input,
    };
    let found = w.execute(&call(worker::SEARCH_CODE, json!({ "text": "pub fn" })));
    assert!(
        found.content.contains("src/lib.rs:2: pub fn size()"),
        "{}",
        found.content
    );
    assert!(found.content.contains("src/lib.rs:3:"), "{}", found.content);
    let range = w.execute(&call(
        worker::READ_CODE,
        json!({ "path": "src/lib.rs", "from_line": 2, "to_line": 2 }),
    ));
    assert!(
        range.content.contains("    2 pub fn size()"),
        "{}",
        range.content
    );
    assert!(!range.content.contains("other"), "{}", range.content);
    // Not unique: refused, nothing changed.
    let ambiguous = w.execute(&call(
        worker::EDIT_CODE,
        json!({ "path": "src/lib.rs", "old_text": "{ 1 }", "new_text": "{ 2 }" }),
    ));
    assert!(
        ambiguous.is_error && ambiguous.content.contains("occurs 2 times"),
        "{}",
        ambiguous.content
    );
    let edited = w.execute(&call(
        worker::EDIT_CODE,
        json!({ "path": "src/lib.rs", "old_text": "size() -> u32 { 1 }", "new_text": "size() -> u32 { 2 }" }),
    ));
    assert!(!edited.is_error, "{}", edited.content);
    assert!(
        std::fs::read_to_string(worktree.path.join("src/lib.rs"))
            .unwrap()
            .contains("size() -> u32 { 2 }")
    );
    // A protected path, in another spelling: refused.
    let protected = w.execute(&call(
        worker::EDIT_CODE,
        json!({ "path": "readme.MD", "old_text": "Protected.", "new_text": "Weakened." }),
    ));
    assert!(
        protected.is_error && protected.content.contains("protected"),
        "{}",
        protected.content
    );
    let written = w.execute(&call(
        worker::WRITE_CODE,
        json!({ "path": "README.md", "text": "Weakened." }),
    ));
    assert!(written.is_error, "{}", written.content);
    // An allowed program runs; another is refused.
    let version = w.execute(&call(
        worker::RUN_PROGRAM,
        json!({ "program": ["cargo", "--version"] }),
    ));
    assert!(version.content.contains("succeeded"), "{}", version.content);
    let shell = w.execute(&call(
        worker::RUN_PROGRAM,
        json!({ "program": ["cmd", "/c", "dir"] }),
    ));
    assert!(
        shell.is_error && shell.content.contains("not a command this task may run"),
        "{}",
        shell.content
    );
}
