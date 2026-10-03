//! An objective's record (C-53, ROADMAP §4.16): the state file, written
//! atomically after every step, and the journal, one line per side effect
//! before and after it, keyed so that resuming never repeats one that
//! completed. Kept in the app data (`objectives/<id>/`), never committed.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The record's format; a build that does not know it does not adopt.
pub const FORMAT: u32 = 1;

/// What an objective is doing as a whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Running,
    Paused,
    /// The Operator stopped it.
    Stopped,
    /// Its cycles are done.
    Done,
    /// A budget was used up, or a cycle could not go on: see `note`.
    Failed,
}

/// What an objective may spend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budgets {
    /// US dollars, at the models' own prices.
    pub usd: f64,
    /// Cycles (improvements) at most.
    pub cycles: u32,
    /// Attempts per cycle at most (the first and each repair).
    pub attempts: u32,
    /// Hours worked (not counting time paused) at most.
    pub hours: f64,
}

impl Default for Budgets {
    fn default() -> Self {
        Budgets {
            usd: 5.0,
            cycles: 1,
            attempts: 4,
            hours: 6.0,
        }
    }
}

/// What the objective allows without asking (§4.2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// Push the cycle's branch and open a pull request.
    pub push: bool,
    /// Merge the pull request when the gates pass.
    pub merge: bool,
    /// Build, try and adopt the merged commit.
    pub adopt: bool,
    /// Locked elements (qualified names) the cycles may change.
    #[serde(default)]
    pub locked: Vec<String>,
    /// Paths of the agent configuration (`.claude`, `CLAUDE.md`,
    /// `AGENTS.md`, `.mcp.json`) and of Agentique's safeguards
    /// (`gates::SAFEGUARDS`) the cycles may change.
    #[serde(default)]
    pub configuration: Vec<String>,
}

/// What it spent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spend {
    pub usd: f64,
    /// Some usage had no known price.
    #[serde(default)]
    pub unknown: bool,
    pub tokens: u64,
    /// Decisions about dialogs in a test instance's way (by rule or by
    /// model; a model's part is in `usd`).
    #[serde(default)]
    pub decisions: u32,
    /// Time worked, not counting time paused.
    #[serde(default)]
    pub seconds: f64,
}

/// Where a cycle is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Propose,
    Implement,
    Check,
    Evaluate,
    Review,
    Repair,
    Merge,
    Build,
    Try,
    Adopt,
    /// Started again in the adopted build: it checks itself, then the cycle
    /// is done.
    Resume,
    Done,
    Failed,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Propose => "Proposing an improvement",
            Phase::Implement => "Implementing",
            Phase::Check => "Running the required checks",
            Phase::Evaluate => "Evaluating in a test instance",
            Phase::Review => "Independent review",
            Phase::Repair => "Repairing",
            Phase::Merge => "Merging",
            Phase::Build => "Building the merged commit",
            Phase::Try => "Trying the build",
            Phase::Adopt => "Adopting the build",
            Phase::Resume => "Resuming in the adopted build",
            Phase::Done => "Done",
            Phase::Failed => "Stopped",
        }
    }
}

/// How an acceptance criterion is checked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Check {
    /// A command that must succeed in the cycle's checkout (a test filter, a
    /// script), as a list of words.
    Command { program: Vec<String> },
    /// What a test instance must show: every field of `expect` must hold in
    /// its observation (`screen`, `dialog`, `statusContains`,
    /// `selectionContains`, `control` with `labelContains`, `valueContains`,
    /// `enabled`, and `anyLabelContains`; at least one, no others).
    Observation {
        #[serde(default)]
        setup: Vec<serde_json::Value>,
        expect: serde_json::Value,
    },
    /// The evaluator's judgment, with the observations it rests on.
    Judgment,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Criterion {
    pub id: String,
    pub statement: String,
    pub check: Check,
}

/// A change to tests, checks or budgets the proposal intends, with why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestChange {
    pub path: String,
    pub why: String,
}

/// The improvement a cycle makes, as proposed and then frozen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub title: String,
    /// `correctness`, `usability`, `comprehension` or `other`.
    pub kind: String,
    pub why: String,
    pub parts: Vec<String>,
    pub plan: Vec<String>,
    pub criteria: Vec<Criterion>,
    #[serde(default)]
    pub intended_test_changes: Vec<TestChange>,
}

/// One check's outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub name: String,
    /// `passed`, `failed`, `not run`.
    pub verdict: String,
    pub detail: String,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        self.verdict == "passed"
    }
}

/// One round of implementation and checking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub n: u32,
    pub commit: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub checks: Vec<Outcome>,
    #[serde(default)]
    pub criteria: Vec<Outcome>,
    #[serde(default)]
    pub gates: Vec<Outcome>,
}

impl Attempt {
    /// What failed, as a fingerprint: the same failures twice in a row mean
    /// no progress. Each failure is its check and what its output says
    /// failed (failing tests, errors), without numbers such as timings and
    /// counts that change from run to run.
    pub fn failures(&self) -> Vec<String> {
        self.checks
            .iter()
            .chain(&self.criteria)
            .chain(&self.gates)
            .filter(|o| !o.passed())
            .map(|o| {
                let mut said: Vec<String> = o
                    .detail
                    .lines()
                    .map(str::trim)
                    .filter(|l| {
                        l.contains("FAILED")
                            || l.starts_with("error")
                            || l.starts_with("---- ")
                            || l.contains("panicked")
                            || l.contains(" failed")
                    })
                    .map(|l| {
                        l.chars()
                            .filter(|c| !c.is_ascii_digit())
                            .collect::<String>()
                    })
                    .collect();
                said.sort();
                said.dedup();
                if said.is_empty() {
                    o.name.clone()
                } else {
                    format!(
                        "{}: {}",
                        o.name,
                        said.join(" | ").chars().take(400).collect::<String>()
                    )
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    /// `approve` or `request_changes`.
    pub verdict: String,
    pub findings: Vec<String>,
    /// The reviewer accepted the changes to tests, checks or budgets the
    /// proposal named.
    #[serde(default)]
    pub test_changes_accepted: bool,
    pub commit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
}

/// One improvement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cycle {
    pub n: u32,
    pub phase: Phase,
    pub started: String,
    pub proposal: Option<Proposal>,
    /// The commit the cycle starts from.
    pub base: Option<String>,
    pub branch: Option<String>,
    pub worktree: Option<PathBuf>,
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    pub review: Option<Review>,
    pub pull_request: Option<PullRequest>,
    /// The merged commit on the default branch.
    pub merged: Option<String>,
    pub build: Option<String>,
    #[serde(default)]
    pub trial: Vec<Outcome>,
    /// The command criteria on the base, before the change: each must fail
    /// there (or run no test), so passing after shows the change.
    #[serde(default)]
    pub before: Vec<Outcome>,
    /// The commit pushed for review: the reviewed tree on the last pushed
    /// one (or the base), never the attempts that came before.
    #[serde(default)]
    pub pushed: Option<String>,
    #[serde(default)]
    pub adopted: bool,
    /// Why it stopped, when it did.
    pub blocker: Option<String>,
    /// The SDK session of each role, to resume after a restart.
    #[serde(default)]
    pub sessions: BTreeMap<String, String>,
}

impl Cycle {
    pub fn new(n: u32) -> Cycle {
        Cycle {
            n,
            phase: Phase::Propose,
            started: agq_launcher::now(),
            proposal: None,
            base: None,
            branch: None,
            worktree: None,
            attempts: Vec::new(),
            review: None,
            pull_request: None,
            merged: None,
            build: None,
            trial: Vec::new(),
            before: Vec::new(),
            pushed: None,
            adopted: false,
            blocker: None,
            sessions: BTreeMap::new(),
        }
    }

    pub fn attempt(&self) -> Option<&Attempt> {
        self.attempts.last()
    }
}

/// Where an objective goes on after its Studio hands over to an adopted
/// build.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Continuation {
    pub cycle: u32,
    pub build: String,
    pub at: String,
}

/// An objective and its cycles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Objective {
    pub format: u32,
    pub id: String,
    pub intent: String,
    pub created: String,
    pub state: State,
    pub budgets: Budgets,
    pub permissions: Permissions,
    /// The repository the cycles change (Agentique's own).
    pub repository: PathBuf,
    /// The branch changes are merged into.
    pub base_branch: String,
    #[serde(default)]
    pub cycles: Vec<Cycle>,
    #[serde(default)]
    pub spent: Spend,
    pub continuation: Option<Continuation>,
    /// Why it ended, in plain words.
    pub note: Option<String>,
}

impl Objective {
    pub fn cycle(&self) -> Option<&Cycle> {
        self.cycles.last()
    }

    pub fn cycle_mut(&mut self) -> Option<&mut Cycle> {
        self.cycles.last_mut()
    }

    /// Whether it is still going (running or paused).
    pub fn active(&self) -> bool {
        matches!(self.state, State::Running | State::Paused)
    }
}

/// One journal line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalLine {
    pub seq: u64,
    pub at: String,
    pub key: String,
    /// `started`, `completed` or `failed`.
    pub state: String,
    #[serde(default)]
    pub note: String,
}

/// The objectives in a folder of the app data.
#[derive(Clone, Debug)]
pub struct Store {
    pub folder: PathBuf,
}

impl Store {
    pub fn new(folder: impl Into<PathBuf>) -> Store {
        Store {
            folder: folder.into(),
        }
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.folder.join(id)
    }

    /// A new objective, saved before anything starts.
    pub fn create(
        &self,
        intent: &str,
        repository: &Path,
        base_branch: &str,
        budgets: Budgets,
        permissions: Permissions,
    ) -> Result<Objective, String> {
        let created = agq_launcher::now();
        let stamp = created.replace(['-', ':', 'T', 'Z'], "");
        let id = format!("objective-{}", stamp.get(..14).unwrap_or(&stamp));
        let objective = Objective {
            format: FORMAT,
            id,
            intent: intent.trim().to_string(),
            created,
            state: State::Running,
            budgets,
            permissions,
            repository: repository.to_path_buf(),
            base_branch: base_branch.to_string(),
            cycles: Vec::new(),
            spent: Spend::default(),
            continuation: None,
            note: None,
        };
        self.save(&objective)?;
        Ok(objective)
    }

    pub fn save(&self, objective: &Objective) -> Result<(), String> {
        let dir = self.dir(&objective.id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(objective).map_err(|e| e.to_string())?;
        agq_launcher::write_atomically(&dir.join("objective.json"), text.as_bytes())
    }

    pub fn load(&self, id: &str) -> Result<Objective, String> {
        if id.contains(['/', '\\']) || id.contains("..") {
            return Err(format!("{id} is not an objective's id"));
        }
        let text = std::fs::read_to_string(self.dir(id).join("objective.json"))
            .map_err(|e| format!("objective {id}: {e}"))?;
        let objective: Objective =
            serde_json::from_str(&text).map_err(|e| format!("objective {id}: {e}"))?;
        if objective.format != FORMAT {
            return Err(format!(
                "objective {id} is in format {}, which this build does not read",
                objective.format
            ));
        }
        Ok(objective)
    }

    /// Every objective, newest first; those that cannot be read are left out.
    pub fn list(&self) -> Vec<Objective> {
        let mut all: Vec<Objective> = std::fs::read_dir(&self.folder)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| self.load(&entry.file_name().to_string_lossy()).ok())
            .collect();
        all.sort_by(|a, b| b.created.cmp(&a.created));
        all
    }

    /// The objective still going, if any (one at a time).
    pub fn active(&self) -> Option<Objective> {
        self.list().into_iter().find(Objective::active)
    }

    /// Whether every objective's record can be read by this build: the
    /// check after adoption refuses a build that cannot.
    pub fn readable(&self) -> Result<(), String> {
        for entry in std::fs::read_dir(&self.folder)
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry.path().join("objective.json").is_file() {
                self.load(&entry.file_name().to_string_lossy())?;
            }
        }
        Ok(())
    }

    fn journal_path(&self, id: &str) -> PathBuf {
        self.dir(id).join("journal.jsonl")
    }

    /// The journal's lines.
    pub fn journal(&self, id: &str) -> Vec<JournalLine> {
        std::fs::read_to_string(self.journal_path(id))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    fn append(&self, id: &str, key: &str, state: &str, note: &str) -> Result<(), String> {
        let path = self.journal_path(id);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let seq = self.journal(id).last().map(|l| l.seq + 1).unwrap_or(1);
        let line = JournalLine {
            seq,
            at: agq_launcher::now(),
            key: key.to_string(),
            state: state.to_string(),
            note: note.to_string(),
        };
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        writeln!(
            file,
            "{}",
            serde_json::to_string(&line).map_err(|e| e.to_string())?
        )
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
    }

    /// Whether the side effect `key` completed (so it is not done again).
    pub fn completed(&self, id: &str, key: &str) -> Option<String> {
        self.journal(id)
            .into_iter()
            .rev()
            .find(|l| l.key == key && l.state == "completed")
            .map(|l| l.note)
    }

    /// Records a side effect before it happens.
    pub fn begin(&self, id: &str, key: &str) -> Result<(), String> {
        self.append(id, key, "started", "")
    }

    /// Records that it happened, with what came of it (an id, a commit).
    pub fn end(&self, id: &str, key: &str, note: &str) -> Result<(), String> {
        self.append(id, key, "completed", note)
    }

    pub fn fail(&self, id: &str, key: &str, note: &str) -> Result<(), String> {
        self.append(id, key, "failed", note)
    }

    /// Runs `effect` once under `key`: a completed key returns its recorded
    /// note without running it again (resuming never repeats it).
    pub fn once(
        &self,
        id: &str,
        key: &str,
        effect: impl FnOnce() -> Result<String, String>,
    ) -> Result<String, String> {
        if let Some(note) = self.completed(id, key) {
            return Ok(note);
        }
        self.begin(id, key)?;
        match effect() {
            Ok(note) => {
                self.end(id, key, &note)?;
                Ok(note)
            }
            Err(error) => {
                let _ = self.fail(id, key, &error);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_objective_is_saved_read_and_listed_and_a_later_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let mut objective = store
            .create(
                "Fix a correctness problem",
                Path::new("C:/agentique"),
                "main",
                Budgets::default(),
                Permissions::default(),
            )
            .unwrap();
        objective.cycles.push(Cycle::new(1));
        store.save(&objective).unwrap();
        assert_eq!(store.load(&objective.id).unwrap(), objective);
        assert_eq!(store.active().unwrap().id, objective.id);
        assert!(store.readable().is_ok());
        let mut later = objective.clone();
        later.format = FORMAT + 1;
        store.save(&later).unwrap();
        assert!(store.load(&objective.id).is_err());
        assert!(store.readable().is_err());
        assert!(store.load("../escape").is_err());
    }

    #[test]
    fn a_side_effect_is_done_once_whatever_happens_after() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let mut runs = 0;
        let first = store.once("o1", "cycle-1/commit", || {
            runs += 1;
            Ok("abc123".into())
        });
        assert_eq!(first.unwrap(), "abc123");
        // Resumed: the completed side effect is not repeated.
        let again = store.once("o1", "cycle-1/commit", || {
            runs += 1;
            Ok("other".into())
        });
        assert_eq!(again.unwrap(), "abc123");
        assert_eq!(runs, 1);
        // A failed one is tried again.
        let _ = store.once("o1", "cycle-1/push", || Err("refused".into()));
        let retried = store.once("o1", "cycle-1/push", || Ok("pushed".into()));
        assert_eq!(retried.unwrap(), "pushed");
        let states: Vec<String> = store.journal("o1").into_iter().map(|l| l.state).collect();
        assert_eq!(
            states,
            vec![
                "started",
                "completed",
                "started",
                "failed",
                "started",
                "completed"
            ]
        );
    }

    #[test]
    fn the_same_failures_twice_are_the_same_fingerprint() {
        let attempt = |detail: &str| Attempt {
            checks: vec![Outcome {
                name: "cargo test".into(),
                verdict: "failed".into(),
                detail: detail.into(),
            }],
            ..Attempt::default()
        };
        assert_eq!(
            attempt("\ntest a ... FAILED").failures(),
            attempt("\ntest a ... FAILED").failures()
        );
        assert_ne!(
            attempt("test a ... FAILED").failures(),
            attempt("test b ... FAILED").failures()
        );
    }
}
