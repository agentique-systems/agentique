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

/// What an objective may spend. Spend and time are unlimited unless set
/// (the Operator's amendment of C-54): its cycles, attempts, exploration
/// steps and model calls bound it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budgets {
    /// US dollars at most, at the models' own prices; `None` for no limit
    /// (left out of the record, which a previous build then skips).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd: Option<f64>,
    /// Cycles (improvements) at most.
    pub cycles: u32,
    /// Attempts per cycle at most (the first and each repair).
    pub attempts: u32,
    /// Hours worked (not counting time paused) at most; `None` for no
    /// limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<f64>,
    /// Actions an exploration run may take (C-54).
    #[serde(default = "default_steps")]
    pub steps: u32,
    /// Model calls a session of a role may make, by role (C-54); a role not
    /// named has its default ([`DEFAULT_CALLS`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub calls: BTreeMap<String, u32>,
}

/// The actions of an exploration run when the Operator sets none: the
/// step budget of the fixed exploration tasks (W12.4).
pub const DEFAULT_STEPS: u32 = 20;

/// Each session role's model calls when the Operator sets none (the bounds
/// Stage 11's cycles ran with).
pub const DEFAULT_CALLS: [(&str, u32); 4] = [
    ("lead", 80),
    ("implementer", 160),
    ("reviewer", 60),
    ("evaluator", 80),
];

fn default_steps() -> u32 {
    DEFAULT_STEPS
}

impl Default for Budgets {
    fn default() -> Self {
        Budgets {
            usd: None,
            cycles: 1,
            attempts: 4,
            hours: None,
            steps: DEFAULT_STEPS,
            calls: BTreeMap::new(),
        }
    }
}

impl Budgets {
    /// The defaults of an objective that explores (C-54): three cycles, so
    /// it explores, fixes and explores the adopted build again.
    pub fn exploring() -> Budgets {
        Budgets {
            cycles: 3,
            ..Budgets::default()
        }
    }

    /// US dollars left after `spent`; unbounded without a spend budget.
    pub fn usd_left(&self, spent: f64) -> f64 {
        self.usd.map_or(f64::INFINITY, |usd| (usd - spent).max(0.0))
    }

    /// Hours left after `seconds` worked; unbounded without a time budget.
    pub fn hours_left(&self, seconds: f64) -> f64 {
        self.hours
            .map_or(f64::INFINITY, |hours| hours - seconds / 3600.0)
    }

    /// What was spent against the spend budget, as the Operator reads it:
    /// `$1.20 of $5.00`, or `$1.20, no limit`.
    pub fn spent_text(&self, spent: f64) -> String {
        match self.usd {
            Some(usd) => format!("${spent:.2} of ${usd:.2}"),
            None => format!("${spent:.2}, no limit"),
        }
    }

    /// The model calls a session of `role` may make.
    pub fn calls_of(&self, role: &str) -> u32 {
        self.calls.get(role).copied().unwrap_or_else(|| {
            DEFAULT_CALLS
                .iter()
                .find(|(r, _)| *r == role)
                .map_or(80, |(_, n)| *n)
        })
    }

    /// Whether the Operator's budgets are within what an objective may set:
    /// a start form shows the problems and starts nothing.
    pub fn check(&self) -> Result<(), Vec<String>> {
        let mut problems = Vec::new();
        if self.usd.is_some_and(|usd| !(0.05..=100.0).contains(&usd)) {
            problems.push("a spend budget is between $0.05 and $100".to_string());
        }
        if !(1..=10).contains(&self.cycles) {
            problems.push("an objective makes 1 to 10 improvements".into());
        }
        if !(1..=10).contains(&self.attempts) {
            problems.push("a cycle has 1 to 10 attempts".into());
        }
        if self
            .hours
            .is_some_and(|hours| !(0.1..=48.0).contains(&hours))
        {
            problems.push("a time budget is between 0.1 and 48 hours".into());
        }
        if !(1..=500).contains(&self.steps) {
            problems.push("an exploration takes 1 to 500 steps".into());
        }
        for (role, calls) in &self.calls {
            if !DEFAULT_CALLS.iter().any(|(r, _)| r == role) {
                problems.push(format!("{role} is not a role with a session"));
            } else if !(1..=1000).contains(calls) {
                problems.push(format!("the {role} makes 1 to 1000 model calls"));
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
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
    /// Dialogs in a test instance's way decided (by rule, at no cost).
    #[serde(default)]
    pub decisions: u32,
    /// Time worked, not counting time paused.
    #[serde(default)]
    pub seconds: f64,
    /// The same by role, and by model within a role (`provider/model`), so
    /// the models of a session's SDK subagents show under its role (C-54).
    /// Included in the totals above.
    #[serde(default)]
    pub roles: BTreeMap<String, BTreeMap<String, Cost>>,
}

/// What some usage cost: US dollars at the model's own list price (for a
/// Claude subscription, what the API would have charged), and tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub usd: f64,
    pub tokens: u64,
    /// Some of it had no known price (counted at a high one).
    #[serde(default)]
    pub unknown: bool,
}

impl Cost {
    pub fn add(&mut self, other: Cost) {
        self.usd += other.usd;
        self.tokens += other.tokens;
        self.unknown |= other.unknown;
    }
}

impl Spend {
    /// Nothing spent.
    pub fn is_empty(&self) -> bool {
        self.usd == 0.0 && self.tokens == 0 && self.roles.is_empty()
    }

    /// Counts `cost` of `model`'s usage by `role`, in the totals too.
    pub fn add(&mut self, role: &str, model: &agq_providers::ModelRef, cost: Cost) {
        self.usd += cost.usd;
        self.tokens += cost.tokens;
        self.unknown |= cost.unknown;
        self.roles
            .entry(role.to_string())
            .or_default()
            .entry(model.to_string())
            .or_default()
            .add(cost);
    }

    /// What `role` spent, all its models together.
    pub fn role(&self, role: &str) -> Cost {
        let mut total = Cost::default();
        for cost in self.roles.get(role).into_iter().flat_map(BTreeMap::values) {
            total.add(*cost);
        }
        total
    }
}

/// Which credential a role's model runs on (C-54).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// The provider's API key, billed per token to its account.
    Key,
    /// The Operator's Claude subscription token, within the plan's limits;
    /// the Claude Agent runtime only.
    Subscription,
}

/// The model a role uses for the whole objective, resolved by the Studio
/// before it started (C-54): the configured one when its provider has a
/// credential Agentique may use, otherwise its fallback, with why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleModel {
    /// `lead`, `implementer`, `reviewer`, `evaluator`, `explorer`,
    /// `escalation` or `decisions` (`models::ROLES`).
    pub role: String,
    pub model: agq_providers::ModelRef,
    #[serde(default)]
    pub effort: Option<String>,
    pub access: Access,
    /// The model Settings name for the role.
    pub configured: agq_providers::ModelRef,
    /// Why the configured model is not used and its fallback is.
    #[serde(default)]
    pub fallback: Option<String>,
    /// Where the credential comes from: its environment variable, or the
    /// Windows Credential Manager. Never the credential.
    pub credential: String,
    /// Who pays for it.
    pub billed: String,
}

impl RoleModel {
    /// `claude-opus-5-5 · high · Claude subscription (your plan's limits
    /// apply)`, `deepseek-v4-pro · max · DeepSeek`.
    pub fn label(&self) -> String {
        let mut text = self.model.model.clone();
        if let Some(effort) = &self.effort {
            text.push_str(&format!(" · {effort}"));
        }
        text.push_str(&match self.access {
            Access::Subscription => " · Claude subscription (your plan's limits apply)".into(),
            Access::Key => format!(" · {}", self.model.provider.name()),
        });
        text
    }
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

/// The phases before Propose in a cycle that explores (C-54, ROADMAP §4.16):
/// kept beside `phase`, which stays `propose` meanwhile, so the previous
/// build reads the record (it would propose without exploring).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exploring {
    /// An explorer operates a test instance of the base build.
    Explore,
    /// The new findings are replayed from a fresh start and reduced.
    Reproduce,
}

impl Exploring {
    pub fn label(self) -> &'static str {
        match self {
            Exploring::Explore => "Exploring the running build",
            Exploring::Reproduce => "Reproducing what it found",
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
    /// `enabled`, and `anyLabelContains`; at least one, no others). With a
    /// `condition` (`control::CONDITIONS`), in a test instance of its own
    /// started in that condition (C-54).
    Observation {
        #[serde(default)]
        setup: Vec<serde_json::Value>,
        expect: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub title: String,
    /// `correctness`, `usability`, `comprehension` or `other`.
    pub kind: String,
    pub why: String,
    /// The elements and contracts of the model it affects (qualified
    /// names); for new elements, the element that will own them.
    pub parts: Vec<String>,
    pub plan: Vec<String>,
    pub criteria: Vec<Criterion>,
    #[serde(default)]
    pub intended_test_changes: Vec<TestChange>,
    /// The reproduced finding it fixes (its identity), whose replay the
    /// Orchestrator adds as a frozen criterion (C-54).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<String>,
    /// The requirements of the project's model it serves (qualified names;
    /// C-55): the engineering capability or root requirement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serves: Vec<String>,
    /// The benefit the Operator is expected to see.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub benefit: String,
    /// Its effect on root complexity, reuse and dependencies.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub complexity: String,
    /// What `serves` and `parts` resolved to in the base commit's model, by
    /// identity, or why they were not resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<crate::traceability::Resolution>,
}

/// One check's outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub name: String,
    /// `passed`, `failed`, `not run`; on the base, also `no evidence`.
    pub verdict: String,
    pub detail: String,
    /// The evaluator's judgment: its failure is identified by its criterion
    /// and verdict, never its wording (C-54).
    #[serde(default, skip_serializing_if = "is_false")]
    pub judged: bool,
}

impl Outcome {
    pub fn new(name: impl Into<String>, verdict: &str, detail: impl Into<String>) -> Outcome {
        Outcome {
            name: name.into(),
            verdict: verdict.to_string(),
            detail: detail.into(),
            judged: false,
        }
    }

    pub fn passed(&self) -> bool {
        self.verdict == "passed"
    }

    /// What identifies its failure from one attempt to the next (C-54): a
    /// judgment by its criterion and verdict, a reviewer's request for
    /// changes as [`REVIEW`], anything else by its name and what its output
    /// says failed (failing tests, errors), with what changes from run to run
    /// (numbers, long hexadecimal ids) normalised.
    pub fn failure(&self) -> String {
        if self.judged {
            return format!("{}: {}", self.name, self.verdict);
        }
        if self.name == REVIEW {
            return REVIEW.into();
        }
        let mut said: Vec<String> = self
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
            .map(normalised)
            .collect();
        said.sort();
        said.dedup();
        if said.is_empty() {
            self.name.clone()
        } else {
            format!(
                "{}: {}",
                self.name,
                said.join(" | ").chars().take(400).collect::<String>()
            )
        }
    }
}

/// The outcome a reviewer's request for changes is recorded as.
pub const REVIEW: &str = "review";

/// A line of output without what changes from run to run: digits, and
/// words of eight or more hexadecimal digits (commits, ids).
fn normalised(line: &str) -> String {
    line.split(' ')
        .map(|word| {
            let core = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if core.len() >= 8
                && core.chars().all(|c| c.is_ascii_hexdigit())
                && core.chars().any(|c| c.is_ascii_alphabetic())
            {
                "<id>".to_string()
            } else {
                word.chars().filter(|c| !c.is_ascii_digit()).collect()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    /// no progress. Each failure is identified as [`Outcome::failure`] says.
    pub fn failures(&self) -> Vec<String> {
        self.checks
            .iter()
            .chain(&self.criteria)
            .chain(&self.gates)
            .filter(|o| !o.passed())
            .map(Outcome::failure)
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
    /// Its judgment of what the commit changed against what the proposal
    /// named (C-55).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub traceability: String,
    /// Its judgment of the cumulative change since the approved baseline
    /// against the purpose and the requirements the proposal serves (C-55).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub purpose: String,
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
    /// Where an exploring cycle is before it proposes (C-54).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploring: Option<Exploring>,
    /// The build of its base: what it explored, and where its criteria are
    /// checked before the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_build: Option<BaseBuild>,
    /// Its explorations, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explorations: Vec<Exploration>,
    /// What they found that was new, or that failed again after its fix,
    /// with what became of each (reproduced or not, reduced).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<crate::findings::Finding>,
    /// The reproduced finding the proposal fixes, frozen with it: its replay
    /// is the criterion [`REPLAY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay: Option<crate::findings::Finding>,
    /// What the criteria's outcomes on the base (`before`) were made with:
    /// the commit whose test files were brought over, and those files. A
    /// commit with other test files has its test runs on the base again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    /// What the reviewed commit changed against what the proposal named
    /// (C-55), as the reviewer was given it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceability: Option<crate::traceability::Traced>,
    /// The cumulative change since the approved baseline to the reviewed
    /// commit (C-55), as the reviewer was given it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cumulative: Option<crate::traceability::Cumulative>,
}

/// What a cycle's evidence on the base was made with (C-54).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    /// The commit whose test files were brought over.
    pub commit: String,
    /// Those test files: each changed test file of the commit against the
    /// base, with its blob id (`deleted` for one it deletes).
    pub tests: Vec<TestFile>,
}

/// A test file as a commit has it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TestFile {
    pub path: String,
    pub blob: String,
}

/// The criterion the Orchestrator adds for the finding a proposal fixes.
pub const REPLAY: &str = "replay";

/// The build a cycle's base runs as: the running build when it is of the
/// base commit, else a debug build of the base checkout kept for the cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseBuild {
    /// The build's id, or `debug-<commit>`.
    pub build: String,
    pub exe: PathBuf,
    pub commit: String,
}

/// One exploration of a cycle as its record keeps it (the run itself goes
/// to the testing knowledge).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exploration {
    pub n: u32,
    pub build: String,
    /// The start project (`models/url-shortener`, `model`).
    pub start: String,
    pub way: crate::decide::Way,
    pub seed: u64,
    pub steps: u32,
    /// Coverage keys the testing knowledge had not seen.
    pub new_coverage: u32,
    /// The identities of what it found that was new, and of fixed findings
    /// that failed again when replayed first.
    #[serde(default)]
    pub found: Vec<String>,
    #[serde(default)]
    pub regressions: Vec<String>,
    /// How many of them reproduced.
    #[serde(default)]
    pub reproduced: u32,
    pub usd: f64,
    pub ended: String,
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
            exploring: None,
            base_build: None,
            explorations: Vec::new(),
            findings: Vec::new(),
            replay: None,
            evidence: None,
            traceability: None,
            cumulative: None,
        }
    }

    /// Where it is, in words.
    pub fn label(&self) -> &'static str {
        match (self.phase, self.exploring) {
            (Phase::Propose, Some(exploring)) => exploring.label(),
            (phase, _) => phase.label(),
        }
    }

    pub fn attempt(&self) -> Option<&Attempt> {
        self.attempts.last()
    }
}

/// How deep child objectives nest at most (C-54): the Operator's objective
/// is 0 deep, its child 1, a child's child 2.
pub const MAX_DEPTH: u8 = 2;

/// Times in a row an objective resumes by itself without getting further
/// before it stops (C-54: a resume that fails twice stops it).
pub const RESUMES: u32 = 2;

/// What a Studio that starts does with an objective that is not finished
/// (C-54, ROADMAP §4.16 "Durable, continuing work").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resuming {
    /// It goes on by itself, saying why in its thread.
    Continue(String),
    /// It waits for the Operator's Continue, saying why.
    Wait(String),
    /// It stops with its record: resumed twice without getting further.
    Stop(String),
}

/// An agent role of an objective: who wrote a directive, who asked for a
/// child objective (C-54).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleRef {
    pub role: String,
    pub objective: String,
}

/// Whom a directive is for: a role within the objective, or a child
/// objective (its id).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum Recipient {
    Role(String),
    Child(String),
}

/// What a directive asks: the instruction, and for a child objective its
/// budgets and permissions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub instruction: String,
    /// The area a child objective explores (the `delegate` tool's `focus`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budgets: Option<Budgets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
}

/// Where a directive is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum DirectiveStatus {
    Running,
    Done,
    Failed,
    Stopped,
    /// The Orchestrator did not accept it, with why.
    Refused {
        reason: String,
    },
}

/// One agent's instruction to another (C-54, ROADMAP §4.16 "Directives and
/// delegation"): created only by the Orchestrator (a handoff within a cycle,
/// or a tool call it validated), never by text alone, and kept in the
/// objective's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Directive {
    /// `<objective>/d<n>`.
    pub id: String,
    pub author: RoleRef,
    pub recipient: Recipient,
    /// The objective it belongs to.
    pub parent: String,
    pub scope: Scope,
    pub status: DirectiveStatus,
    /// What came of it, in a few lines.
    #[serde(default)]
    pub result: Option<String>,
    /// The record it hands over: `cycle-<n>/proposal`, `cycle-<n>/review-<attempt>`,
    /// `cycle-<n>/delegation` for a child.
    #[serde(default)]
    pub refers_to: Option<String>,
    /// For a child: what of its spend is counted in the parent's so far, so
    /// that after a restart only the rest is added.
    #[serde(default, skip_serializing_if = "Spend::is_empty")]
    pub counted: Spend,
    pub created: String,
    pub updated: String,
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
    /// Each role's model for every session of the objective (C-54), as the
    /// Studio resolved them before it started. A record of a build before
    /// C-54 has none.
    #[serde(default)]
    pub models: Vec<RoleModel>,
    /// The roles it does not need that had no model when it started, each
    /// with why (C-54): an objective that does not explore needs no
    /// explorer, escalation or typed decisions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub roles_unavailable: BTreeMap<String, String>,
    /// Its cycles start by exploring the running build (C-54).
    #[serde(default, skip_serializing_if = "is_false")]
    pub explore: bool,
    /// What it does as inferred from its intent before Start, and who
    /// inferred it (the Operator's amendment of C-54); what it was started
    /// with is `explore`, `budgets.cycles` and `permissions`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred: Option<crate::decide::Inferred>,
    /// The objective that delegated it, for a child objective (C-54).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// 0 for the Operator's; one more than its parent's for a child.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub depth: u8,
    /// The agent that asked for it, for a child objective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<RoleRef>,
    /// The directives of its agents, in the order they were made.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directives: Vec<Directive>,
    /// It was interrupted because the Operator closed Agentique, so it waits
    /// for their Continue (C-54); one that was running when its Studio ended
    /// otherwise goes on by itself after a recovered crash.
    #[serde(default, skip_serializing_if = "is_false")]
    pub interrupted: bool,
    /// Times in a row it was resumed by itself without getting further: a
    /// resume that fails twice stops it.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub resumes: u32,
    /// The last of the Operator's messages for the lead (thread entries to
    /// `lead`) given to it: the later ones go to its next turn.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub delivered: u64,
    /// The URL of the repository's remote `origin` when the objective was
    /// created (C-55): where the approved baseline is read, whatever an
    /// agent does to the remote later. None without a remote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// What its explorations explore, once its lead planned it (C-54, the
    /// W13.7 repair): later cycles, its children, the replays and the
    /// evaluation's exploration keep to its projects. None until planned,
    /// and for an objective that does not explore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<crate::explore::Target>,
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_zero(value: &u8) -> bool {
    *value == 0
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

    /// Records a new directive of `author` (a role of this objective) for
    /// `recipient`, running, and returns its id.
    pub fn direct(
        &mut self,
        author: &str,
        recipient: Recipient,
        scope: Scope,
        refers_to: Option<String>,
    ) -> String {
        let now = agq_launcher::now();
        let id = format!("{}/d{}", self.id, self.directives.len() + 1);
        self.directives.push(Directive {
            id: id.clone(),
            author: RoleRef {
                role: author.to_string(),
                objective: self.id.clone(),
            },
            recipient,
            parent: self.id.clone(),
            scope,
            status: DirectiveStatus::Running,
            result: None,
            refers_to,
            counted: Spend::default(),
            created: now.clone(),
            updated: now,
        });
        id
    }

    pub fn directive(&self, id: &str) -> Option<&Directive> {
        self.directives.iter().find(|d| d.id == id)
    }

    /// Moves directive `id` to `status`, with what came of it.
    pub fn settle(&mut self, id: &str, status: DirectiveStatus, result: Option<String>) {
        if let Some(directive) = self.directives.iter_mut().find(|d| d.id == id) {
            directive.status = status;
            if result.is_some() {
                directive.result = result;
            }
            directive.updated = agq_launcher::now();
        }
    }

    /// Settles every directive still running in it (it stopped, or its
    /// cycle ended), with what came of them.
    pub fn settle_running(&mut self, status: DirectiveStatus, result: &str) {
        let running: Vec<String> = self
            .directives
            .iter()
            .filter(|d| d.status == DirectiveStatus::Running)
            .map(|d| d.id.clone())
            .collect();
        for id in running {
            self.settle(&id, status.clone(), Some(result.to_string()));
        }
    }

    /// What a Studio that starts does with it while it is not finished:
    /// one handed over to an adopted build (its continuation), or running
    /// when its Studio did not start and the launcher started the last
    /// known good build instead (`recovered`: `--recovered-from`), goes on
    /// by itself; one interrupted because the Operator closed Agentique
    /// (`interrupted`, written as the Studio closes) waits for their
    /// Continue, as does any other; and one that resumed by itself twice
    /// without getting further stops. A plain start under the launcher is
    /// not a recovery.
    pub fn on_start(&self, recovered: bool) -> Resuming {
        let by_itself = if self.continuation.is_some() {
            Some("Going on in the adopted build")
        } else if self.interrupted {
            return Resuming::Wait(
                "Interrupted when Agentique closed: Continue goes on from where it was.".into(),
            );
        } else if recovered {
            Some(
                "Agentique ended unexpectedly while it ran, and started again: going on from where it was",
            )
        } else {
            None
        };
        match by_itself {
            Some(_) if self.resumes >= RESUMES => Resuming::Stop(format!(
                "It resumed by itself {} times without getting further, so it stops here.",
                self.resumes
            )),
            Some(why) => Resuming::Continue(why.into()),
            None => Resuming::Wait(
                "An objective is not finished: Continue goes on from where it was.".into(),
            ),
        }
    }

    /// The latest directive for `role` that is still running.
    pub fn running_for(&self, role: &str) -> Option<&Directive> {
        self.directives.iter().rev().find(|d| {
            d.status == DirectiveStatus::Running && d.recipient == Recipient::Role(role.into())
        })
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
            models: Vec::new(),
            roles_unavailable: BTreeMap::new(),
            explore: false,
            inferred: None,
            parent: None,
            depth: 0,
            requested_by: None,
            directives: Vec::new(),
            interrupted: false,
            resumes: 0,
            delivered: 0,
            origin: crate::forge::origin_url(repository),
            target: None,
        };
        self.save(&objective)?;
        Ok(objective)
    }

    pub fn save(&self, objective: &Objective) -> Result<(), String> {
        let dir = self.dir(&objective.id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(objective).map_err(|e| e.to_string())?;
        let _writing = self.writing(&objective.id)?;
        agq_launcher::write_atomically(&dir.join("objective.json"), text.as_bytes())
    }

    /// The lock of objective `id`'s record: one writer at a time.
    fn writing(&self, id: &str) -> Result<std::fs::File, String> {
        let path = self.dir(id).join("objective.lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        file.lock()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(file)
    }

    /// Marks objective `id` as interrupted because the Operator closed
    /// Agentique (C-54), as the Studio closes, whatever its run saved last:
    /// read, marked and written under the record's lock, so no newer save
    /// of the run is lost. One that is not going on is left alone.
    pub fn mark_interrupted(&self, id: &str) -> Result<(), String> {
        let mut objective = self.load(id)?;
        let _writing = self.writing(id)?;
        // Read again under the lock: the run may have saved meanwhile.
        objective = self.load(id).unwrap_or(objective);
        if !objective.active() || objective.interrupted {
            return Ok(());
        }
        objective.interrupted = true;
        let text = serde_json::to_string_pretty(&objective).map_err(|e| e.to_string())?;
        agq_launcher::write_atomically(&self.dir(id).join("objective.json"), text.as_bytes())
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

    /// The Operator's objective still going, if any (one at a time); a
    /// child goes on inside its parent's run.
    pub fn active(&self) -> Option<Objective> {
        self.list()
            .into_iter()
            .find(|o| o.active() && o.parent.is_none())
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

    /// Spend by role and by model within it (C-54): each model's usage is
    /// counted under its role and in the totals, so a session's subagents'
    /// models show.
    #[test]
    fn spend_is_counted_by_role_and_model() {
        use agq_providers::{ModelRef, Provider};
        let mut spent = Spend::default();
        let pro = ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro");
        let flash = ModelRef::new(Provider::DeepSeek, "deepseek-flash");
        let cost = |usd: f64, tokens: u64| Cost {
            usd,
            tokens,
            unknown: false,
        };
        spent.add("lead", &pro, cost(0.25, 1000));
        spent.add("lead", &flash, cost(0.01, 300));
        spent.add("lead", &pro, cost(0.25, 1000));
        spent.add(
            "reviewer",
            &pro,
            Cost {
                unknown: true,
                ..cost(0.5, 10)
            },
        );
        assert_eq!(spent.role("lead"), cost(0.51, 2300));
        assert_eq!(
            spent.roles["lead"]["deepseek/deepseek-v4-pro"],
            cost(0.5, 2000)
        );
        assert_eq!(spent.roles["lead"]["deepseek/deepseek-flash"].tokens, 300);
        assert!(spent.role("reviewer").unknown && spent.unknown);
        assert!((spent.usd - 1.01).abs() < 1e-9);
        assert_eq!(spent.tokens, 2310);
        assert_eq!(spent.role("evaluator"), Cost::default());
    }

    fn with_models() -> Objective {
        use agq_providers::{ModelRef, Provider};
        let mut objective = Objective {
            format: FORMAT,
            id: "objective-1".into(),
            intent: "Fix it".into(),
            created: "2026-10-04T00:00:00Z".into(),
            state: State::Running,
            // Limits set, as every objective had before the Operator's
            // amendment of C-54.
            budgets: Budgets {
                usd: Some(5.0),
                hours: Some(6.0),
                ..Budgets::default()
            },
            permissions: Permissions::default(),
            repository: PathBuf::from("C:/agentique"),
            base_branch: "main".into(),
            cycles: vec![Cycle::new(1)],
            spent: Spend::default(),
            continuation: None,
            note: None,
            models: vec![RoleModel {
                role: "lead".into(),
                model: ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro"),
                effort: Some("max".into()),
                access: Access::Key,
                configured: ModelRef::new(Provider::Anthropic, "claude-opus-5-5"),
                fallback: Some("no Anthropic API key or Claude subscription token".into()),
                credential: "DEEPSEEK_API_KEY".into(),
                billed: "per token, to the DeepSeek account of this key".into(),
            }],
            roles_unavailable: BTreeMap::from([(
                "decisions".to_string(),
                "typesafe/jev-1.13.0 needs a TypeSafe AI key".to_string(),
            )]),
            explore: false,
            inferred: None,
            parent: None,
            depth: 0,
            requested_by: None,
            directives: Vec::new(),
            interrupted: false,
            resumes: 0,
            delivered: 0,
            origin: None,
            target: None,
        };
        objective.spent.add(
            "lead",
            &ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro"),
            Cost {
                usd: 0.5,
                tokens: 2000,
                unknown: false,
            },
        );
        objective
    }

    /// `objective.json` stays format 1 (C-54, §7.6 locked part (4)): the
    /// fields W12.3 adds are optional, a record written before them is read
    /// with them empty, and the previous build reads a record that has them.
    /// The previous build's reader is these types without the new fields and
    /// with the same serde attributes (none in this file refuses unknown
    /// fields, the last assertion keeps it so, and serde's default is to skip
    /// fields a type does not know); `Previous` and
    /// `PreviousSpend` are exactly that reader for the two types that gained
    /// fields, and every other type is unchanged.
    #[test]
    fn a_record_with_models_and_spend_by_role_stays_format_1() {
        #[derive(Default, Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct PreviousSpend {
            usd: f64,
            #[serde(default)]
            unknown: bool,
            tokens: u64,
            #[serde(default)]
            decisions: u32,
            #[serde(default)]
            seconds: f64,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct Previous {
            format: u32,
            id: String,
            intent: String,
            created: String,
            state: State,
            budgets: Budgets,
            permissions: Permissions,
            repository: PathBuf,
            base_branch: String,
            #[serde(default)]
            cycles: Vec<Cycle>,
            #[serde(default)]
            spent: PreviousSpend,
            continuation: Option<Continuation>,
            note: Option<String>,
        }
        let objective = with_models();
        let text = serde_json::to_string_pretty(&objective).unwrap();
        assert!(
            text.contains("\"models\"")
                && text.contains("\"roles\"")
                && text.contains("\"rolesUnavailable\"")
        );
        // The previous build reads it, and sees what it knew.
        let previous: Previous = serde_json::from_str(&text).unwrap();
        assert_eq!(previous.format, 1);
        assert_eq!(previous.spent.tokens, 2000);
        assert_eq!(previous.cycles.len(), 1);
        // This build reads it back whole.
        assert_eq!(serde_json::from_str::<Objective>(&text).unwrap(), objective);
        // A record the previous build wrote reads with the new fields empty.
        let mut old: serde_json::Value = serde_json::from_str(&text).unwrap();
        old.as_object_mut().unwrap().remove("models");
        old.as_object_mut().unwrap().remove("rolesUnavailable");
        old["spent"].as_object_mut().unwrap().remove("roles");
        let read: Objective = serde_json::from_value(old).unwrap();
        assert!(
            read.models.is_empty()
                && read.spent.roles.is_empty()
                && read.roles_unavailable.is_empty()
        );
        assert_eq!(read.spent.tokens, 2000);
        // And the store keeps format 1.
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store.save(&objective).unwrap();
        assert_eq!(store.load(&objective.id).unwrap(), objective);
        assert!(!include_str!("record.rs").contains(concat!("deny_unknown", "_fields")));
    }

    /// `objective.json` stays format 1 with what W12.5 and W12.6 add (C-54,
    /// §7.6 locked part (4)): exploring, a child's parent, depth and
    /// requester, directives, and the step and call budgets are optional;
    /// the previous build (`main` with W12.3, whose reader is these types
    /// without them, `Previous` and `PreviousBudgets` here) reads a record
    /// that has them, and this build reads one it wrote with its defaults.
    #[test]
    fn a_record_with_directives_and_new_budgets_stays_format_1() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct PreviousBudgets {
            usd: f64,
            cycles: u32,
            attempts: u32,
            hours: f64,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct Previous {
            format: u32,
            id: String,
            intent: String,
            created: String,
            state: State,
            budgets: PreviousBudgets,
            permissions: Permissions,
            repository: PathBuf,
            base_branch: String,
            #[serde(default)]
            cycles: Vec<Cycle>,
            #[serde(default)]
            spent: Spend,
            continuation: Option<Continuation>,
            note: Option<String>,
            #[serde(default)]
            models: Vec<RoleModel>,
            #[serde(default)]
            roles_unavailable: BTreeMap<String, String>,
        }
        let mut objective = with_models();
        objective.explore = true;
        objective.parent = Some("objective-0".into());
        objective.depth = 1;
        objective.requested_by = Some(RoleRef {
            role: "lead".into(),
            objective: "objective-0".into(),
        });
        objective.budgets.steps = 40;
        objective.budgets.calls.insert("lead".into(), 30);
        let id = objective.direct(
            "lead",
            Recipient::Role("implementer".into()),
            Scope {
                instruction: "Implement the proposal".into(),
                focus: None,
                budgets: None,
                permissions: None,
            },
            Some("cycle-1/proposal".into()),
        );
        assert_eq!(id, "objective-1/d1");
        let child = objective.direct(
            "lead",
            Recipient::Child("objective-1-d2".into()),
            Scope {
                instruction: "Explore the History panel".into(),
                focus: None,
                budgets: Some(Budgets {
                    usd: Some(0.5),
                    ..Budgets::default()
                }),
                permissions: Some(Permissions::default()),
            },
            None,
        );
        objective.settle(
            &child,
            DirectiveStatus::Refused {
                reason: "over budget".into(),
            },
            None,
        );
        objective.settle(&id, DirectiveStatus::Done, Some("Implemented".into()));
        assert_eq!(objective.running_for("implementer"), None);
        let text = serde_json::to_string_pretty(&objective).unwrap();
        for field in [
            "\"explore\"",
            "\"parent\"",
            "\"depth\"",
            "\"requestedBy\"",
            "\"directives\"",
            "\"refersTo\"",
            "\"steps\"",
            "\"calls\"",
            "\"refused\"",
        ] {
            assert!(text.contains(field), "{field}");
        }
        // The previous build reads it, and sees what it knew.
        let previous: Previous = serde_json::from_str(&text).unwrap();
        assert_eq!(previous.format, 1);
        assert_eq!(previous.budgets.attempts, 4);
        assert_eq!(previous.models.len(), 1);
        // This build reads it back whole.
        let read: Objective = serde_json::from_str(&text).unwrap();
        assert_eq!(read, objective);
        assert_eq!(read.budgets.calls_of("lead"), 30);
        assert_eq!(read.budgets.calls_of("implementer"), 160);
        assert!(matches!(
            read.directive(&child).unwrap().status,
            DirectiveStatus::Refused { .. }
        ));
        // An exploring cycle as written (in a phase the previous build knows:
        // a cycle explores while its phase stays `propose`), with its
        // explorations, findings, replay, base build, conditions and judged
        // outcomes: the previous build reads it, and this one whole.
        let mut exploring = objective.clone();
        let cycle = exploring.cycle_mut().unwrap();
        cycle.exploring = Some(Exploring::Reproduce);
        cycle.base_build = Some(BaseBuild {
            build: "debug-abc".into(),
            exe: PathBuf::from("C:/work/base-studio/agentique-studio.exe"),
            commit: "abc".into(),
        });
        cycle.explorations.push(Exploration {
            n: 1,
            build: "debug-abc".into(),
            start: "model".into(),
            way: crate::decide::Way::Rules,
            seed: 7,
            steps: 20,
            new_coverage: 3,
            found: vec!["readable-labels|x|y".into()],
            regressions: Vec::new(),
            reproduced: 1,
            usd: 0.0,
            ended: "the step budget was used".into(),
        });
        let finding = crate::findings::Finding::new(
            crate::findings::Failed {
                check: crate::findings::Check::ReadableLabels,
                control: "x".into(),
                message: "y".into(),
                evidence: serde_json::json!({}),
            },
            Vec::new(),
            "b1",
            "abc",
            "model",
        );
        cycle.findings.push(finding.clone());
        cycle.replay = Some(finding);
        cycle.proposal = Some(Proposal {
            title: "t".into(),
            kind: "usability".into(),
            why: "w".into(),
            parts: Vec::new(),
            plan: Vec::new(),
            criteria: vec![Criterion {
                id: "c1".into(),
                statement: "s".into(),
                check: Check::Observation {
                    setup: Vec::new(),
                    expect: serde_json::json!({ "screen": "surface" }),
                    condition: Some("recovered".into()),
                },
            }],
            intended_test_changes: Vec::new(),
            finding: Some("readable-labels|x|y".into()),
            ..Proposal::default()
        });
        cycle
            .before
            .push(Outcome::new(REPLAY, "failed", "it fails on the base"));
        cycle.attempts.push(Attempt {
            n: 1,
            criteria: vec![Outcome {
                judged: true,
                ..Outcome::new("c2", "passed", "seen")
            }],
            ..Attempt::default()
        });
        exploring.interrupted = true;
        exploring.resumes = 1;
        exploring.delivered = 12;
        let text = serde_json::to_string_pretty(&exploring).unwrap();
        assert!(text.contains("\"phase\": \"propose\""));
        let previous: Previous = serde_json::from_str(&text).unwrap();
        assert_eq!(previous.cycles[0].attempts.len(), 1);
        assert_eq!(serde_json::from_str::<Objective>(&text).unwrap(), exploring);
        // A record the previous build wrote reads with the defaults, and a
        // record of an objective that does not explore writes none of them.
        let plain = with_models();
        let text = serde_json::to_string_pretty(&plain).unwrap();
        for field in [
            "\"explore\"",
            "\"parent\"",
            "\"depth\"",
            "\"directives\"",
            "\"calls\"",
        ] {
            assert!(!text.contains(field), "{field}");
        }
        let mut old: serde_json::Value = serde_json::from_str(&text).unwrap();
        old["budgets"].as_object_mut().unwrap().remove("steps");
        let read: Objective = serde_json::from_value(old).unwrap();
        assert_eq!(read.budgets.steps, DEFAULT_STEPS);
        assert!(!read.explore && read.parent.is_none() && read.directives.is_empty());
        assert_eq!(read, plain);
    }

    /// `objective.json` stays format 1 with what C-55 adds: a proposal's
    /// `serves`, benefit, complexity and resolution, a review's judgments,
    /// a cycle's traceability and cumulative change, a finding's
    /// disposition, all optional. The previous build's readers of the types
    /// that gained fields (`PreviousProposal`, `PreviousReview`, with the
    /// fields `main` at W12.6 knows and the same serde attributes) read what
    /// this build writes, and this build reads a record written without
    /// them.
    #[test]
    fn a_record_with_traceability_and_dispositions_stays_format_1() {
        use crate::findings::{Disposition, DispositionKind};
        use crate::traceability::{Cumulative, Group, Resolution, Resolved, Traced};
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct PreviousProposal {
            title: String,
            kind: String,
            why: String,
            parts: Vec<String>,
            plan: Vec<String>,
            criteria: Vec<Criterion>,
            #[serde(default)]
            intended_test_changes: Vec<TestChange>,
            #[serde(default)]
            finding: Option<String>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct PreviousReview {
            verdict: String,
            findings: Vec<String>,
            #[serde(default)]
            test_changes_accepted: bool,
            commit: String,
        }
        let mut objective = with_models();
        let cycle = objective.cycle_mut().unwrap();
        cycle.proposal = Some(Proposal {
            title: "t".into(),
            kind: "correctness".into(),
            why: "w".into(),
            parts: vec!["Shop::Store".into()],
            serves: vec!["Shop::Fast".into()],
            benefit: "Faster checkout".into(),
            complexity: "Adds nothing at the root".into(),
            resolved: Some(Resolution {
                serves: vec![Resolved {
                    name: "Shop::Fast".into(),
                    element: 4,
                    kind: "requirement def".into(),
                }],
                parts: Vec::new(),
                skipped: None,
            }),
            ..Proposal::default()
        });
        cycle.review = Some(Review {
            verdict: "approve".into(),
            findings: Vec::new(),
            test_changes_accepted: false,
            commit: "abc".into(),
            traceability: "Every change is within Store".into(),
            purpose: "It still serves the purpose".into(),
        });
        cycle.traceability = Some(Traced {
            commit: "abc".into(),
            base: "def".into(),
            changed: 2,
            not_named: vec!["Shop::Cart (part def, updated)".into()],
            not_changed: Vec::new(),
            skipped: None,
        });
        cycle.cumulative = Some(Cumulative {
            commit: "abc".into(),
            since: "0ld".into(),
            approved: true,
            created: 3,
            groups: vec![Group {
                what: "requirements".into(),
                added: vec!["Shop::Slow".into()],
                ..Group::default()
            }],
            ..Cumulative::default()
        });
        let mut finding = crate::findings::Finding::new(
            crate::findings::Failed {
                check: crate::findings::Check::Expectation,
                control: "x".into(),
                message: "y".into(),
                evidence: serde_json::json!({}),
            },
            Vec::new(),
            "b1",
            "abc",
            "model",
        );
        finding.disposition = Some(Disposition {
            kind: DispositionKind::WrongExpectation,
            reason: "the requirement says otherwise".into(),
            requirement: None,
            objective: "objective-1".into(),
            cycle: 1,
            role: "lead".into(),
            at: "t".into(),
            build: None,
        });
        cycle.findings.push(finding);
        let text = serde_json::to_string_pretty(&objective).unwrap();
        for field in [
            "\"serves\"",
            "\"benefit\"",
            "\"complexity\"",
            "\"resolved\"",
            "\"traceability\"",
            "\"cumulative\"",
            "\"purpose\"",
            "\"disposition\"",
            "\"wrong-expectation\"",
        ] {
            assert!(text.contains(field), "{field}");
        }
        // This build reads it back whole, and still as format 1.
        let read: Objective = serde_json::from_str(&text).unwrap();
        assert_eq!(read, objective);
        assert_eq!(read.format, 1);
        // The previous build reads the proposal and review it wrote.
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let previous: PreviousProposal =
            serde_json::from_value(value["cycles"][0]["proposal"].clone()).unwrap();
        assert_eq!(previous.parts, vec!["Shop::Store".to_string()]);
        let previous: PreviousReview =
            serde_json::from_value(value["cycles"][0]["review"].clone()).unwrap();
        assert_eq!(previous.verdict, "approve");
        // A record written without them reads with them empty.
        let mut old = value.clone();
        let cycle = &mut old["cycles"][0];
        for field in ["traceability", "cumulative"] {
            cycle.as_object_mut().unwrap().remove(field);
        }
        for field in ["serves", "benefit", "complexity", "resolved"] {
            cycle["proposal"].as_object_mut().unwrap().remove(field);
        }
        for field in ["traceability", "purpose"] {
            cycle["review"].as_object_mut().unwrap().remove(field);
        }
        cycle["findings"][0]
            .as_object_mut()
            .unwrap()
            .remove("disposition");
        let read: Objective = serde_json::from_value(old).unwrap();
        let cycle = read.cycle().unwrap();
        let proposal = cycle.proposal.as_ref().unwrap();
        assert!(proposal.serves.is_empty() && proposal.resolved.is_none());
        assert!(cycle.review.as_ref().unwrap().purpose.is_empty());
        assert!(cycle.traceability.is_none() && cycle.cumulative.is_none());
        assert!(cycle.findings[0].disposition.is_none());
    }

    /// Durable work (C-54): after an adoption or a recovered crash it goes
    /// on by itself; interrupted by the Operator closing Agentique, or after
    /// an ordinary start, it waits for Continue; resumed twice without
    /// getting further, it stops.
    #[test]
    fn an_unfinished_objective_goes_on_by_itself_only_when_it_should() {
        let mut objective = with_models();
        assert!(matches!(objective.on_start(false), Resuming::Wait(_)));
        assert!(matches!(objective.on_start(true), Resuming::Continue(_)));
        objective.interrupted = true;
        assert!(matches!(objective.on_start(true), Resuming::Wait(why) if why.contains("closed")));
        objective.interrupted = false;
        // Its note's wording means nothing: only the typed field does.
        objective.note = Some("Interrupted when Agentique closed; continue it.".into());
        assert!(matches!(objective.on_start(true), Resuming::Continue(_)));
        objective.note = None;
        objective.continuation = Some(Continuation {
            cycle: 1,
            build: "b2".into(),
            at: "t".into(),
        });
        assert!(
            matches!(objective.on_start(false), Resuming::Continue(why) if why.contains("adopted"))
        );
        objective.resumes = RESUMES;
        assert!(matches!(objective.on_start(false), Resuming::Stop(_)));
        // A child goes on inside its parent's run, never on its own.
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let mut child = with_models();
        child.id = "objective-1-c1".into();
        child.parent = Some("objective-1".into());
        child.created = "2026-10-05T00:00:00Z".into();
        store.save(&child).unwrap();
        assert!(store.active().is_none());
        store.save(&with_models()).unwrap();
        assert_eq!(store.active().unwrap().id, "objective-1");
        // The Studio marks it interrupted as it closes, keeping what the
        // run saved; a finished one is left alone.
        let mut saved = with_models();
        saved.spent.tokens = 99;
        store.save(&saved).unwrap();
        store.mark_interrupted("objective-1").unwrap();
        let read = store.load("objective-1").unwrap();
        assert!(read.interrupted && read.spent.tokens == 99);
        saved.state = State::Done;
        store.save(&saved).unwrap();
        store.mark_interrupted("objective-1").unwrap();
        assert!(!store.load("objective-1").unwrap().interrupted);
    }

    #[test]
    fn budgets_are_checked_before_an_objective_starts() {
        assert!(Budgets::default().check().is_ok());
        assert_eq!(Budgets::exploring().cycles, 3);
        assert!(Budgets::exploring().check().is_ok());
        let wrong = Budgets {
            usd: Some(0.0),
            cycles: 0,
            attempts: 11,
            hours: Some(0.0),
            steps: 0,
            calls: BTreeMap::from([("explorer".to_string(), 5), ("lead".to_string(), 0)]),
        };
        let problems = wrong.check().unwrap_err();
        assert_eq!(problems.len(), 7, "{problems:?}");
        assert_eq!(Budgets::default().calls_of("reviewer"), 60);
    }

    /// The Operator's amendment of C-54 (§7.6): spend and time unlimited,
    /// left out of the record, with what the intent was read as. The
    /// previous build's reader cannot parse such a record (so its list
    /// leaves it out and its check after adoption refuses, as §7.6 says);
    /// this build reads and lists it whole; a record with limits still
    /// parses in the previous build's reader.
    #[test]
    fn an_unlimited_record_is_unreadable_to_the_previous_build() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct PreviousBudgets {
            usd: f64,
            hours: f64,
        }
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Previous {
            budgets: PreviousBudgets,
        }
        let mut objective = with_models();
        objective.budgets.usd = None;
        objective.budgets.hours = None;
        objective.inferred = Some(crate::decide::Inferred {
            shape: crate::decide::Shape {
                explore: true,
                cycles: 2,
                merge: true,
                adopt: true,
            },
            source: crate::decide::Source::Jev,
            confidence: Some(0.91),
            millis: 800,
            usd: Some(0.001),
            note: String::new(),
        });
        let text = serde_json::to_string_pretty(&objective).unwrap();
        assert!(!text.contains("\"usd\": null") && !text.contains("\"hours\""));
        assert!(serde_json::from_str::<Previous>(&text).is_err());
        assert_eq!(serde_json::from_str::<Objective>(&text).unwrap(), objective);
        let limited = serde_json::to_string(&with_models()).unwrap();
        assert!(serde_json::from_str::<Previous>(&limited).is_ok());

        let folder = std::env::temp_dir().join(format!("agq-unlimited-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let store = Store::new(folder.clone());
        store.save(&objective).unwrap();
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.load(&objective.id).unwrap().budgets.usd, None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_same_failures_twice_are_the_same_fingerprint() {
        let attempt = |detail: &str| Attempt {
            checks: vec![Outcome {
                name: "cargo test".into(),
                verdict: "failed".into(),
                detail: detail.into(),
                judged: false,
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

    /// Failure identity (C-54): a judgment by its criterion and verdict,
    /// whatever its wording; a review's request for changes as `review`;
    /// numbers and long hexadecimal ids normalised.
    #[test]
    fn a_failure_is_identified_by_what_failed_not_its_wording() {
        let judged = |detail: &str| Outcome {
            judged: true,
            ..Outcome::new("c2", "failed", detail)
        };
        assert_eq!(
            judged("The button reads Archive, not Store").failure(),
            judged("Still unlabelled; I saw no change").failure()
        );
        assert_eq!(judged("x").failure(), "c2: failed");
        assert_ne!(
            judged("x").failure(),
            Outcome {
                judged: true,
                ..Outcome::new("c2", "not run", "x")
            }
            .failure()
        );
        assert_eq!(
            Outcome::new(REVIEW, "failed", "a.rs:3 is wrong").failure(),
            Outcome::new(REVIEW, "failed", "b.rs:9 is wrong in another way").failure()
        );
        assert_eq!(
            Outcome::new(
                "cargo test",
                "failed",
                "error: at deadbeef12 FAILED in 1.2s"
            )
            .failure(),
            Outcome::new(
                "cargo test",
                "failed",
                "error: at 0123abcdef FAILED in 9.9s"
            )
            .failure()
        );
    }
}
