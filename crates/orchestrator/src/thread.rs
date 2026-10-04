//! An objective's thread (C-54, ROADMAP §4.16 "The Conversation, one
//! place"): what happened, in time order, as the Conversation and the
//! Objectives panel show it: the Operator's messages, agents' directives and
//! results, the Orchestrator's events (phases, checks, gates, pull requests,
//! merges, builds, adoptions, recoveries) and each agent's tool activity,
//! which folds under the entry before it that is not activity (the step it
//! belongs to), with a bounded diff for a file it changed or the command
//! line it ran. Each entry says who wrote it: the Operator, Agentique, or an
//! agent with its role and model.
//!
//! Kept with the objective's records (`objectives/<id>/thread.jsonl`, under
//! the `objectives` data format 1, a file the previous build does not read):
//! one JSON line per entry, appended whole under the folder's lock file, so
//! writers (the Orchestrator's thread, the Studio) take turns and number
//! entries in order (`seq`); a line cut short by a crash is skipped when
//! read and the next one starts on a line of its own. Read back from any
//! point with [`Store::thread`]. Bounded: each entry's text and details are
//! capped ([`TEXT_CHARS`], [`DETAILS_CHARS`]), and a file that reaches
//! [`FILE_BYTES`] is kept as `thread.1.jsonl` (replacing the one before) while
//! a new one starts, so an objective's thread takes at most twice that. No
//! model receives it whole.

use crate::record::Store;
use agq_providers::ModelRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// A thread file's size at which it is kept as the previous one and a new
/// one starts. An entry is at most about 10 KB (its text and details
/// capped) and usually a few hundred bytes, so a file holds over ten
/// thousand usual entries: more than a six-hour objective (the default time
/// budget) writes with a tool call every few seconds. With the previous file
/// kept, the latest 4 to 8 MB of an objective's thread can always be read.
pub const FILE_BYTES: u64 = 4 << 20;

/// The most characters of an entry's text, and of its details.
pub const TEXT_CHARS: usize = 2000;
pub const DETAILS_CHARS: usize = 8000;

/// What an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The Operator's message (the intent is the first).
    Human,
    /// One agent's instruction to another, as recorded
    /// (`record::Directive`).
    Directive,
    /// What an agent handed over: a submission, a verdict, a child
    /// objective's result.
    Result,
    /// What happened: a phase, a check, a gate, a pull request, a merge, a
    /// build, an adoption, a recovery, the Operator's command.
    Event,
    /// An agent's tool call, folded under the step it belongs to.
    Activity,
}

/// Who wrote an entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "lowercase")]
pub enum Author {
    /// The Operator ("you").
    Operator,
    /// The Orchestrator, or the Studio for it.
    Agentique,
    /// An agent: its role, and the model it runs on (as the objective
    /// recorded it; none for a role without one).
    Agent {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<ModelRef>,
    },
}

impl Author {
    /// The agent of `role`.
    pub fn agent(role: &str, model: Option<ModelRef>) -> Author {
        Author::Agent {
            role: role.to_string(),
            model,
        }
    }

    /// `you`, `Agentique`, or the role and its model (`lead ·
    /// claude-opus-5-5`).
    pub fn label(&self) -> String {
        match self {
            Author::Operator => "you".into(),
            Author::Agentique => "Agentique".into(),
            Author::Agent { role, model: None } => role.clone(),
            Author::Agent {
                role,
                model: Some(model),
            } => format!("{role} · {}", model.model),
        }
    }
}

/// One entry of an objective's thread.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEntry {
    /// Its place in the thread, from 1.
    pub seq: u64,
    pub at: String,
    /// The objective whose thread it is.
    pub objective: String,
    pub kind: Kind,
    pub author: Author,
    pub text: String,
    /// What folds under it: a tool call's input, a bounded diff of a file
    /// changed, a command line, a proposal in full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// The directive it is (for a directive) or belongs to (the work done
    /// for it, its result); a directive's id names its objective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directive: Option<String>,
}

impl ThreadEntry {
    /// A new entry; its number, time and objective are given when it is
    /// added to a thread.
    pub fn new(kind: Kind, author: Author, text: impl Into<String>) -> ThreadEntry {
        ThreadEntry {
            seq: 0,
            at: String::new(),
            objective: String::new(),
            kind,
            author,
            text: text.into(),
            details: None,
            directive: None,
        }
    }

    /// Agentique's event.
    pub fn event(text: impl Into<String>) -> ThreadEntry {
        ThreadEntry::new(Kind::Event, Author::Agentique, text)
    }

    pub fn with_details(mut self, details: impl Into<String>) -> ThreadEntry {
        let details = details.into();
        self.details = (!details.trim().is_empty()).then_some(details);
        self
    }

    pub fn for_directive(mut self, id: Option<&str>) -> ThreadEntry {
        self.directive = id.map(str::to_string);
        self
    }
}

/// `text` cut to `most` characters, saying how much was left out.
pub fn capped(text: &str, most: usize) -> String {
    let count = text.chars().count();
    if count <= most {
        return text.to_string();
    }
    let kept: String = text.chars().take(most.saturating_sub(40)).collect();
    format!(
        "{kept}\n… ({} more characters)",
        count - kept.chars().count()
    )
}

/// A tool call as the thread shows it: a line naming the tool and what it
/// acts on, and what folds under it: for a file changed (`Edit`, `Write`,
/// `MultiEdit`), its diff (old lines `-`, new lines `+`); for a command, its
/// command line; for Agentique's tools, their input.
pub fn activity(tool: &str, input: &Value) -> (String, Option<String>) {
    let tool = tool.strip_prefix("mcp__agentique__").unwrap_or(tool);
    let field = |name: &str| input[name].as_str().unwrap_or_default();
    let lines = |prefix: &str, text: &str| -> String {
        text.lines()
            .map(|line| format!("{prefix} {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    match tool {
        "Edit" => (
            format!("Edit {}", field("file_path")),
            Some(format!(
                "{}\n{}",
                lines("-", field("old_string")),
                lines("+", field("new_string"))
            )),
        ),
        "MultiEdit" => {
            let edits: Vec<String> = input["edits"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|e| {
                    format!(
                        "{}\n{}",
                        lines("-", e["old_string"].as_str().unwrap_or_default()),
                        lines("+", e["new_string"].as_str().unwrap_or_default())
                    )
                })
                .collect();
            (
                format!("Edit {} ({} changes)", field("file_path"), edits.len()),
                Some(edits.join("\n…\n")),
            )
        }
        "Write" => (
            format!("Write {}", field("file_path")),
            Some(lines("+", field("content"))),
        ),
        "Bash" | "PowerShell" => {
            let command = field("command");
            let first = command.lines().next().unwrap_or_default();
            (format!("{tool}: {first}"), Some(command.to_string()))
        }
        "Read" | "NotebookRead" => (format!("Read {}", field("file_path")), None),
        "Glob" | "Grep" => (
            format!("{tool} {} {}", field("pattern"), field("path"))
                .trim()
                .to_string(),
            None,
        ),
        _ => {
            let shown = match input {
                Value::Object(o) if o.is_empty() => None,
                Value::Null => None,
                other => serde_json::to_string_pretty(other).ok(),
            };
            (tool.to_string(), shown)
        }
    }
}

impl Store {
    fn thread_files(&self, id: &str) -> (PathBuf, PathBuf) {
        let dir = self.folder.join(id);
        (dir.join("thread.1.jsonl"), dir.join("thread.jsonl"))
    }

    /// Adds `entry` to the thread of objective `id`, numbered after the
    /// last one, and returns it as added.
    pub fn append_thread(&self, id: &str, mut entry: ThreadEntry) -> Result<ThreadEntry, String> {
        if id.contains(['/', '\\']) || id.contains("..") {
            return Err(format!("{id} is not an objective's id"));
        }
        let (previous, current) = self.thread_files(id);
        let dir = self.folder.join(id);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        // One writer at a time, across threads and processes.
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("thread.lock"))
            .map_err(|e| format!("the thread's lock: {e}"))?;
        lock.lock().map_err(|e| format!("the thread's lock: {e}"))?;
        let size = std::fs::metadata(&current).map(|m| m.len()).unwrap_or(0);
        let last = last_seq(&current)?.or(last_seq(&previous)?).unwrap_or(0);
        if size >= FILE_BYTES {
            std::fs::rename(&current, &previous)
                .map_err(|e| format!("{}: {e}", current.display()))?;
        }
        entry.seq = last + 1;
        entry.at = agq_launcher::now();
        entry.objective = id.to_string();
        entry.text = capped(&entry.text, TEXT_CHARS);
        entry.details = entry.details.map(|d| capped(&d, DETAILS_CHARS));
        let mut line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&current)
            .map_err(|e| format!("{}: {e}", current.display()))?;
        // A line a crash cut short is left alone; this one starts anew.
        if ends_unfinished(&mut file)? {
            line.insert(0, '\n');
        }
        file.write_all(line.as_bytes())
            .and_then(|_| file.sync_data())
            .map_err(|e| format!("{}: {e}", current.display()))?;
        Ok(entry)
    }

    /// The entries of objective `id`'s thread after `since` (0 for all
    /// that are kept), in order.
    pub fn thread(&self, id: &str, since: u64) -> Vec<ThreadEntry> {
        let (previous, current) = self.thread_files(id);
        let mut entries: Vec<ThreadEntry> = [previous, current]
            .iter()
            .flat_map(|path| {
                std::fs::read_to_string(path)
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|line| serde_json::from_str::<ThreadEntry>(line).ok())
                    .filter(|e| e.seq > since)
                    .collect::<Vec<_>>()
            })
            .collect();
        // In order even if a writer's clock or a copy disagreed.
        entries.sort_by_key(|e| e.seq);
        entries.dedup_by_key(|e| e.seq);
        entries
    }
}

/// The number of the last whole entry in `path`, read from its end.
fn last_seq(path: &std::path::Path) -> Result<Option<u64>, String> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    // An entry is at most about 10 KB: the last whole one is in the last 64.
    let from = size.saturating_sub(64 * 1024);
    file.seek(SeekFrom::Start(from))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&tail)
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<ThreadEntry>(line).ok())
        .map(|e| e.seq)
        .next())
}

/// Whether the file's last line was cut short (it does not end a line).
fn ends_unfinished(file: &mut std::fs::File) -> Result<bool, String> {
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(size - 1))
        .map_err(|e| e.to_string())?;
    let mut last = [0u8; 1];
    file.read_exact(&mut last).map_err(|e| e.to_string())?;
    Ok(last[0] != b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_providers::Provider;
    use serde_json::json;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("objectives"));
        (dir, store)
    }

    #[test]
    fn entries_are_numbered_in_order_and_read_back_from_any_point() {
        let (_dir, store) = store();
        let lead = Author::agent(
            "lead",
            Some(ModelRef::new(Provider::Anthropic, "claude-opus-5-5")),
        );
        let added = [
            ThreadEntry::new(Kind::Human, Author::Operator, "Find and fix problems"),
            ThreadEntry::event("Cycle 1: Proposing an improvement"),
            ThreadEntry::new(Kind::Activity, lead.clone(), "Read model/links.json"),
            ThreadEntry::new(Kind::Directive, lead.clone(), "Add a label")
                .with_details("Title: Add a label")
                .for_directive(Some("objective-1/d1")),
        ]
        .into_iter()
        .map(|e| store.append_thread("objective-1", e).unwrap())
        .collect::<Vec<_>>();
        assert_eq!(
            added.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert!(added.iter().all(|e| e.objective == "objective-1"));
        let all = store.thread("objective-1", 0);
        assert_eq!(all, added, "read back as written");
        let later = store.thread("objective-1", 2);
        assert_eq!(later.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![3, 4]);
        assert_eq!(later[1].directive.as_deref(), Some("objective-1/d1"));
        assert_eq!(later[1].author.label(), "lead · claude-opus-5-5");
        assert!(store.thread("objective-2", 0).is_empty());
        assert!(
            store
                .append_thread("../x", ThreadEntry::event("x"))
                .is_err()
        );
        // The line as written: plain names, no empty fields.
        let text =
            std::fs::read_to_string(store.folder.join("objective-1").join("thread.jsonl")).unwrap();
        let first: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(first["kind"], "human");
        assert_eq!(first["author"], json!({ "by": "operator" }));
        assert!(first.get("details").is_none() && first.get("directive").is_none());
    }

    #[test]
    fn writers_take_turns_and_no_number_is_used_twice() {
        let (_dir, store) = store();
        let writers: Vec<_> = (0..4)
            .map(|n| {
                let store = store.clone();
                std::thread::spawn(move || {
                    for i in 0..25 {
                        store
                            .append_thread("o", ThreadEntry::event(format!("{n}-{i}")))
                            .unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        let all = store.thread("o", 0);
        assert_eq!(
            all.iter().map(|e| e.seq).collect::<Vec<_>>(),
            (1..=100).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_line_cut_short_is_skipped_and_the_next_starts_anew() {
        let (_dir, store) = store();
        store.append_thread("o", ThreadEntry::event("one")).unwrap();
        let file = store.folder.join("o").join("thread.jsonl");
        let mut text = std::fs::read_to_string(&file).unwrap();
        text.push_str("{\"seq\":2,\"at\":\"x\",\"obj");
        std::fs::write(&file, text).unwrap();
        let next = store.append_thread("o", ThreadEntry::event("two")).unwrap();
        assert_eq!(next.seq, 2);
        let all = store.thread("o", 0);
        assert_eq!(
            all.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
            vec!["one", "two"]
        );
    }

    #[test]
    fn a_thread_stays_within_its_bounds() {
        let (_dir, store) = store();
        let long = "x".repeat(TEXT_CHARS * 3);
        let entry = store
            .append_thread(
                "o",
                ThreadEntry::event(long.clone()).with_details("y".repeat(DETAILS_CHARS * 3)),
            )
            .unwrap();
        assert!(entry.text.chars().count() <= TEXT_CHARS + 40);
        assert!(entry.text.ends_with("more characters)"));
        assert!(entry.details.unwrap().chars().count() <= DETAILS_CHARS + 40);
        // A full file is kept as the previous one, and a new one starts;
        // numbers go on, and both are read. (Filled here in one write, as
        // appending entry by entry would.)
        let current = store.folder.join("o").join("thread.jsonl");
        let fill = |from: u64| -> u64 {
            let mut text = std::fs::read_to_string(&current).unwrap_or_default();
            let mut seq = from;
            while (text.len() as u64) < FILE_BYTES {
                seq += 1;
                let mut entry = ThreadEntry::event(capped(&long, TEXT_CHARS));
                entry.seq = seq;
                entry.objective = "o".into();
                text.push_str(&serde_json::to_string(&entry).unwrap());
                text.push('\n');
            }
            std::fs::write(&current, text).unwrap();
            seq
        };
        let n = fill(1);
        let after = store
            .append_thread("o", ThreadEntry::event("after"))
            .unwrap();
        assert_eq!(after.seq, n + 1);
        assert!(store.folder.join("o").join("thread.1.jsonl").is_file());
        assert!(std::fs::metadata(&current).unwrap().len() < 1000);
        let read = store.thread("o", n - 1);
        assert_eq!(
            read.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![n, n + 1]
        );
        // Once more: the oldest file goes, so at most two are kept.
        let m = fill(n + 1);
        let last = store
            .append_thread("o", ThreadEntry::event("last"))
            .unwrap();
        assert_eq!(last.seq, m + 1);
        let kept = store.thread("o", 0);
        assert!(kept.first().unwrap().seq > n, "the oldest entries went");
        assert_eq!(kept.last().unwrap().text, "last");
    }

    #[test]
    fn tool_calls_show_what_they_act_on_with_diffs_and_command_lines() {
        let (text, details) = activity(
            "Edit",
            &json!({ "file_path": "src/a.rs", "old_string": "let a = 1;\nlet b = 2;", "new_string": "let a = 3;" }),
        );
        assert_eq!(text, "Edit src/a.rs");
        assert_eq!(details.unwrap(), "- let a = 1;\n- let b = 2;\n+ let a = 3;");
        let (text, details) = activity(
            "MultiEdit",
            &json!({ "file_path": "b.rs", "edits": [
                { "old_string": "x", "new_string": "y" },
                { "old_string": "p", "new_string": "q" },
            ] }),
        );
        assert_eq!(text, "Edit b.rs (2 changes)");
        assert_eq!(details.unwrap(), "- x\n+ y\n…\n- p\n+ q");
        let (text, details) = activity(
            "Write",
            &json!({ "file_path": "n.md", "content": "one\ntwo" }),
        );
        assert_eq!(
            (text.as_str(), details.unwrap().as_str()),
            ("Write n.md", "+ one\n+ two")
        );
        let (text, details) = activity(
            "Bash",
            &json!({ "command": "cargo test -p agq-x\necho done", "description": "test" }),
        );
        assert_eq!(text, "Bash: cargo test -p agq-x");
        assert_eq!(details.unwrap(), "cargo test -p agq-x\necho done");
        assert_eq!(
            activity("Read", &json!({ "file_path": "c.rs" })),
            ("Read c.rs".into(), None)
        );
        let (text, details) = activity("mcp__agentique__read_model", &json!({}));
        assert_eq!((text.as_str(), details), ("read_model", None));
        let (text, details) = activity("mcp__agentique__apply_changes", &json!({ "changes": [1] }));
        assert_eq!(text, "apply_changes");
        assert!(details.unwrap().contains("changes"));
    }
}
