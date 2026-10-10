//! Exploration (C-54, ROADMAP §4.16 "Exploration"; Scenario K2): an
//! explorer operates a test instance of one build through its control
//! interface toward a goal. Each step: observe; check the invariants
//! (`findings`); list the actions that are valid there and that the
//! observation offers to agents, fields with typed inputs from fixed input
//! classes ([`candidates`]); choose one by a way of deciding (the rules,
//! Jev, the explorer's model, or Jev escalating when unsure); act as the
//! explorer, with the goal and a one-line reason for observer mode; record.
//! A stale refusal is observed again, a dialog the explorer did not open is
//! cancelled by rule, a dead end is left by Escape or a restart, and an exit
//! or a hang is a finding, after which the instance starts again from the
//! same start. The run returns its steps, coverage, findings, decisions,
//! cost, latency and recoveries; the testing knowledge (`knowledge`) keeps
//! them across runs. [`Instance`] is the one place tests replace the GUI;
//! what the explorer reads from it is in `observed`.
//!
//! What a run explores is the project its objective's lead planned (the
//! W13.7 repair: [`Target`]), at the revision of the build explored. The
//! instance opens a copy of that project, and before its first action the
//! run checks the copy against the plan's [`Provenance`] (the folder copied,
//! the digest of its model files, the project the observation shows,
//! [`check_copy`]); a mismatch ends the run without acting, never a
//! substitution. Replays check their copies the same way.

use crate::control::{Client, TestInstance};
use crate::decide::{self, Answers, Decision, Question, Source, Way};
use crate::findings::{self, Check, Failed, Finding, Outcome};
use crate::knowledge::Knowledge;
use crate::observed::{self, Refusal};
use agq_providers::ModelRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Who acts in the test instance.
pub const AGENT: &str = "explorer";

/// The application under test as exploration sees it: a test instance
/// through its control interface ([`LiveInstance`]), or a stand-in in tests.
pub trait Instance {
    /// What it shows now: a full observation.
    fn observe(&mut self) -> Result<Value, String>;
    /// Asks it to carry out an action; its answer (`ok`, or the refusal).
    /// `Err` when it did not answer.
    fn act(&mut self, act: &Act) -> Result<Value, String>;
    /// A fresh start of the same build in the same start state; `stop` is
    /// asked while it starts.
    fn restart(&mut self, stop: &mut dyn FnMut() -> bool) -> Result<(), String>;
    /// Whether it still runs.
    fn alive(&mut self) -> bool;
    /// The folder its files belong in, if it has one: a dialog holding a
    /// path that leads outside it is never confirmed.
    fn folder(&self) -> Option<PathBuf>;
    /// The copy of a project it opened at its latest start, if it opens
    /// one: what a run checks against its plan's [`Provenance`].
    fn opened(&self) -> Option<Opened> {
        None
    }
}

/// An action as an agent asks for it, with the goal and the reason observer
/// mode shows.
pub struct Act<'a> {
    pub agent: &'a str,
    pub goal: &'a str,
    pub why: &'a str,
    /// The screen revision of the observation it rests on.
    pub observed: u64,
    pub action: &'a Value,
}

/// What supervises a run: before each step, whether to go on (it may wait
/// while the run is paused); during a long wait (a turn of the Assistant, a
/// model's answer, a fresh start, a replay), whether it was stopped, which
/// never waits; and what the run is doing, for whoever watches it. A
/// closure goes on while it returns true and stops only between steps.
pub trait Supervisor {
    fn go_on(&mut self) -> bool;
    fn stopped(&mut self) -> bool {
        false
    }
    /// What the run is doing (the W13.7 repair): nothing by default.
    fn progress(&mut self, _progress: &Progress) {}
    /// How often a wait for a model is said to go on: [`HEARTBEAT`].
    fn heartbeat(&self) -> Duration {
        HEARTBEAT
    }
}

/// How often a run says it still waits for a model's answer.
pub const HEARTBEAT: Duration = Duration::from_secs(10);

/// What a run reports while it goes (the W13.7 repair), for whoever
/// watches it: only what happened and what it waits for, never a model's
/// reasoning. `step` counts the run's actions, `of` is its step budget,
/// `seconds` its time so far.
#[derive(Debug)]
pub enum Progress<'a> {
    /// A model is being asked for step `step` (said once its call starts,
    /// not for a step the rules or Jev decide); `by` says which.
    Deciding {
        step: u32,
        of: u32,
        by: String,
        seconds: u64,
    },
    /// Still waiting for `by`'s answer, `waited` seconds so far.
    Waiting {
        step: u32,
        of: u32,
        by: String,
        waited: u64,
    },
    /// An action was taken (or refused, or failed).
    Took {
        step: u32,
        of: u32,
        taken: &'a Taken,
        seconds: u64,
    },
}

/// Seconds as a person reads them: `41 s`, `2 min 5 s`.
fn duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

/// Milliseconds as seconds with a tenth: `41.2 s`.
fn tenths(ms: u64) -> String {
    format!("{:.1} s", ms as f64 / 1000.0)
}

impl Progress<'_> {
    /// Its line in the thread, and what folds under it.
    pub fn entry(&self) -> (String, String) {
        match self {
            Progress::Deciding {
                step,
                of,
                by,
                seconds,
            } => (
                format!("Step {step}/{of}: asks {by} · {} in", duration(*seconds)),
                String::new(),
            ),
            Progress::Waiting {
                step,
                of,
                by,
                waited,
            } => (
                format!("Step {step}/{of}: waiting for {by} · {}", duration(*waited)),
                String::new(),
            ),
            Progress::Took {
                step,
                of,
                taken,
                seconds,
            } => (
                format!("Step {step}/{of}: {}", taken.line()),
                taken.details(*seconds),
            ),
        }
    }
}

/// How a decision's note begins when Jev failed (an error, its deadline)
/// rather than answered unsure.
const JEV_FAILED: &str = "Jev failed: ";

/// Who chose a step: the rules, Jev with its confidence, or a model (the
/// one `timing` says was asked: since the W13.7 repair a step Jev is unsure
/// of, or Jev failed on, goes to the explorer's model, not to the
/// escalation role's).
fn decided_by(chosen: &Chosen, timing: &Timing) -> String {
    let d = &chosen.decision;
    let model = || match (&timing.model, &timing.effort) {
        (Some(model), Some(effort)) => format!("{model} at {effort}"),
        (Some(model), None) => model.clone(),
        (None, _) => "the explorer's model".to_string(),
    };
    match d.source {
        Source::Rules if d.note.is_empty() => "rules".to_string(),
        Source::Rules => format!("rules ({})", d.note),
        Source::Jev => format!("Jev {:.2}", d.confidence.unwrap_or(0.0)),
        Source::Model => model(),
        Source::Escalated if d.note.starts_with(JEV_FAILED) => {
            format!("Jev failed, then {}", model())
        }
        Source::Escalated => format!("Jev unsure, then {}", model()),
    }
}

/// What a step did, in words, with what it acted on.
fn acted(step: &Step) -> String {
    let action = &step.action;
    let label = if step.label.is_empty() {
        step.target().to_string()
    } else {
        step.label.clone()
    };
    match action["kind"].as_str().unwrap_or_default() {
        "click" => format!("clicks “{label}”"),
        "command" => format!("runs “{label}”"),
        "fill" => format!(
            "types “{}” into “{label}”",
            action["text"]
                .as_str()
                .unwrap_or_default()
                .chars()
                .take(24)
                .collect::<String>()
        ),
        "key" => format!("presses {}", action["keys"].as_str().unwrap_or_default()),
        "select" => format!("selects {label}"),
        "await-turn" => format!("waits for {label}"),
        other => other.to_string(),
    }
}

/// Where one step's time went (the W13.7 repair), in milliseconds:
/// deciding (Jev, the models) and the instance (carrying the action out,
/// observing after); with the model asked, at its effort, its calls (a
/// reply that could not be read is asked again), the characters of its
/// prompts and the tokens the provider reported.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    #[serde(default)]
    pub jev_ms: u64,
    #[serde(default)]
    pub model_ms: u64,
    #[serde(default)]
    pub act_ms: u64,
    #[serde(default)]
    pub observe_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default)]
    pub calls: u32,
    #[serde(default)]
    pub prompt_chars: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Tokens>,
}

impl Timing {
    /// With `other`'s time, calls, prompts and tokens added (a step's
    /// decision, its action and what was observed after are measured
    /// apart); the model asked last is kept.
    fn add(&mut self, other: Timing) {
        self.jev_ms += other.jev_ms;
        self.model_ms += other.model_ms;
        self.act_ms += other.act_ms;
        self.observe_ms += other.observe_ms;
        if other.model.is_some() {
            self.model = other.model;
            self.effort = other.effort;
        }
        self.calls += other.calls;
        self.prompt_chars += other.prompt_chars;
        if let Some(t) = other.tokens {
            let tokens = self.tokens.get_or_insert_with(Tokens::default);
            tokens.input += t.input;
            tokens.output += t.output;
            tokens.reasoning += t.reasoning;
        }
    }
}

/// Tokens a provider reported: input (with the cache's), output, and the
/// output that was reasoning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
}

/// One step's typed decisions, asked through `answers` and measured where
/// they are asked: Jev's time, the models' time, calls, prompts and the
/// tokens reported.
struct Measured<'a> {
    answers: &'a dyn Answers,
    timing: std::cell::RefCell<Timing>,
}

impl<'a> Measured<'a> {
    fn new(answers: &'a dyn Answers) -> Measured<'a> {
        Measured {
            answers,
            timing: Default::default(),
        }
    }
}

impl Answers for Measured<'_> {
    fn jev_model(&self) -> ModelRef {
        self.answers.jev_model()
    }

    fn threshold(&self) -> f64 {
        self.answers.threshold()
    }

    fn ask_jev(&self, question: &Question) -> Result<Decision, decide::Failure> {
        let started = Instant::now();
        let answer = self.answers.ask_jev(question);
        self.timing.borrow_mut().jev_ms += started.elapsed().as_millis() as u64;
        answer
    }

    fn ask_jev_all(
        &self,
        state: &Value,
        questions: BTreeMap<String, agq_providers::jev::Question>,
    ) -> Result<(BTreeMap<String, agq_providers::jev::Answer>, Option<f64>), decide::Failure> {
        let started = Instant::now();
        let answer = self.answers.ask_jev_all(state, questions);
        self.timing.borrow_mut().jev_ms += started.elapsed().as_millis() as u64;
        answer
    }

    /// (`ask_model` asks `stop` before each call: whoever watches learns
    /// the call starts, and a stop that came first sends nothing.)
    fn chat(
        &self,
        model: &ModelRef,
        effort: Option<&str>,
        prompt: &str,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<decide::Answered, String> {
        let started = Instant::now();
        let answer = self.answers.chat(model, effort, prompt, stop);
        let mut timing = self.timing.borrow_mut();
        timing.model_ms += started.elapsed().as_millis() as u64;
        timing.model = Some(model.to_string());
        timing.effort = effort.map(str::to_string);
        timing.calls += 1;
        timing.prompt_chars += prompt.chars().count() as u64;
        if let Ok(decide::Answered {
            usage: Some(usage), ..
        }) = &answer
        {
            let tokens = timing.tokens.get_or_insert_with(Tokens::default);
            tokens.input += usage.input_tokens + usage.cache_read_tokens + usage.cache_write_tokens;
            tokens.output += usage.output_tokens;
            tokens.reasoning += usage.reasoning_tokens;
        }
        answer
    }
}

impl<F: FnMut() -> bool> Supervisor for F {
    fn go_on(&mut self) -> bool {
        self()
    }
}

/// How long the explorer waits for an answer beyond what the action itself
/// may take: an observation or a click takes well under a second, so a
/// Studio silent this long has stopped answering.
const ANSWER: Duration = Duration::from_secs(30);

/// How long a test instance may take to start.
const START: Duration = Duration::from_secs(120);

/// How long an answer to `action` may take before the instance counts as
/// not answering: what the action may take by the checks' budgets (a wait
/// its own time; typing a keystroke's budget for each character, so a long
/// request is not taken for a hang), and [`ANSWER`] more.
pub fn answer_time(action: &Value) -> Duration {
    let wait = action["timeoutMs"].as_u64().unwrap_or(0);
    let typed = action["text"]
        .as_str()
        .map_or(0, |t| t.chars().count() as u64);
    Duration::from_millis(wait + findings::ACTION_BUDGET_MS + findings::TYPED_CHARACTER_MS * typed)
        + ANSWER
}

/// A test instance of a Studio executable, started fresh each time from a
/// copy of the start project, in its own folder (its data, its working
/// folder and the repository it is told about are all in there), which is
/// removed when it is done with.
pub struct LiveInstance {
    exe: PathBuf,
    /// The start project: a project folder, or its model folder.
    start: PathBuf,
    folder: PathBuf,
    starts: u32,
    options: crate::control::Options,
    /// The copy it opened at its latest start.
    opened: Option<Opened>,
    // The connection closes before the process ends.
    client: Option<Client>,
    instance: Option<TestInstance>,
}

impl LiveInstance {
    pub fn new(exe: &Path, start: &Path, folder: &Path) -> LiveInstance {
        LiveInstance::with(exe, start, folder, crate::control::Options::default())
    }

    /// One started with `options` each time (C-54: its speed, its
    /// Assistant's stand-in or key, a stated condition).
    pub fn with(
        exe: &Path,
        start: &Path,
        folder: &Path,
        options: crate::control::Options,
    ) -> LiveInstance {
        LiveInstance {
            exe: exe.to_path_buf(),
            start: start.to_path_buf(),
            folder: folder.to_path_buf(),
            starts: 0,
            options,
            opened: None,
            client: None,
            instance: None,
        }
    }

    fn client(&mut self) -> Result<&mut Client, String> {
        self.client
            .as_mut()
            .ok_or_else(|| "the test instance is not running".to_string())
    }
}

/// The folder a copy of `start` takes its files from: its model folder
/// when it is a project folder that has one, else `start` itself (a folder
/// of model files, such as Agentique's `model`).
pub fn model_folder(start: &Path) -> PathBuf {
    if start.join("model").is_dir() {
        start.join("model")
    } else {
        start.to_path_buf()
    }
}

/// The files of a model folder a copy takes, by their paths relative to it
/// (`/` between folders), in order: all but hidden ones and the lock.
fn model_files(folder: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut files = Vec::new();
    let mut pending = vec![(String::new(), folder.to_path_buf())];
    while let Some((prefix, dir)) = pending.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "agentique.lock" {
                continue;
            }
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let path = entry.path();
            if path.is_dir() {
                pending.push((relative, path));
            } else {
                files.push((relative, path));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The folder `project` (as the repository names it, `/` between folders)
/// of `root`.
pub fn within(root: &Path, project: &str) -> PathBuf {
    project
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

/// A digest of a model folder's files as a copy takes them (their relative
/// paths and contents; FNV-1a, 64 bits): it tells a copy from one of
/// another project or revision, which is all it is for (it is no security
/// check). A folder without a `.sysml` file holds no model.
pub fn model_digest(folder: &Path) -> Result<String, String> {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut add = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    let files = model_files(folder)?;
    if !files
        .iter()
        .any(|(relative, _)| relative.ends_with(".sysml"))
    {
        return Err(format!("{} holds no model files", folder.display()));
    }
    for (relative, path) in files {
        let content = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        add(relative.as_bytes());
        add(&[0]);
        add(&(content.len() as u64).to_le_bytes());
        add(&content);
    }
    Ok(format!("{hash:016x}"))
}

/// Copies the start project's model files into `to` (a fresh model folder),
/// leaving out its repository and lock; the digest of what it copied.
pub(crate) fn copy_model(start: &Path, to: &Path) -> Result<String, String> {
    let from = model_folder(start);
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for (relative, path) in model_files(&from)? {
        let copy = to.join(&relative);
        if let Some(folder) = copy.parent() {
            std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        }
        std::fs::copy(&path, &copy).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    model_digest(to)
}

/// Whether two paths name the same folder: as the system resolves them
/// when they exist, compared part by part (case-folded on Windows).
pub fn same_folder(a: &Path, b: &Path) -> bool {
    let parted = |path: &Path| {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let text = path.display().to_string();
        parts(text.strip_prefix(r"\\?\").unwrap_or(&text))
    };
    parted(a) == parted(b)
}

/// What an objective explores (C-54, ROADMAP §4.16 "Exploration"), as its
/// lead planned it (`submit_exploration`): a project of the repository at
/// the revision of the build explored, the engineering goal, and where
/// useful the elements it is about and where the explorer starts. Recorded
/// on the objective once planned, so its later cycles, its children, the
/// replays and the evaluation's exploration explore the same project;
/// another only when `vary` permits it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    /// A folder of the repository that holds a model's files, as the
    /// repository names it: `model` (Agentique's own model),
    /// `models/url-shortener`.
    pub project: String,
    /// The commit the project is taken from: the commit of the build
    /// explored.
    pub revision: String,
    /// What the exploration should find out.
    pub goal: String,
    /// Elements of the project's model it is about, by qualified name, each
    /// resolved in the model at the revision when it was planned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Start::is_empty")]
    pub start: Start,
    /// Other projects a later plan of the same objective may name, as the
    /// lead allowed explicitly; empty: every exploration explores
    /// `project`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vary: Vec<String>,
    /// What the exploration tests, first (the W13.7 repair, E3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hypotheses: Vec<Hypothesis>,
}

/// An engineering hypothesis an exploration tests (the W13.7 repair, E3):
/// what should hold, the requirement of the project's model that governs
/// it (resolved at the revision, its text read from the model) or the
/// intended behaviour in plain words, the workflow in the screens' words
/// that tests it, and what the explorer should then observe. The explorer
/// works through its workflow and states a checkable expectation; the
/// check on the next observation answers it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hypothesis {
    pub claim: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    /// The requirement as the model prints it (bounded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behaviour: Option<String>,
    pub workflow: String,
    pub expected: String,
}

/// How an exploration answered a hypothesis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub claim: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    pub verdict: Verdict,
}

/// Whether the application agrees with a hypothesis: the expectation the
/// explorer stated for it held, it failed (the finding it became, by
/// identity), or no expectation was checked (with why: never a finding).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "kebab-case")]
pub enum Verdict {
    Agrees,
    Contradicted { finding: String },
    NotAnswered { why: String },
}

impl Answer {
    /// In a line, for the thread and the lead: the verdict, then the claim.
    pub fn line(&self, finding_id: impl Fn(&str) -> Option<String>) -> String {
        let verdict = match &self.verdict {
            Verdict::Agrees => "agrees".to_string(),
            Verdict::Contradicted { finding } => match finding_id(finding) {
                Some(id) => format!("contradicted (finding {id})"),
                None => "contradicted".to_string(),
            },
            Verdict::NotAnswered { why } => format!("not answered ({why})"),
        };
        format!(
            "{verdict}: {}{}",
            self.claim,
            self.requirement
                .as_ref()
                .map(|r| format!(" [{r}]"))
                .unwrap_or_default()
        )
    }
}

impl Target {
    /// The projects its objective may explore: its own, then those `vary`
    /// permits.
    pub fn projects(&self) -> Vec<&str> {
        std::iter::once(self.project.as_str())
            .chain(self.vary.iter().map(String::as_str))
            .collect()
    }

    /// In a line, for the thread: the project, its revision, and its scope
    /// and start when it has them.
    pub fn line(&self) -> String {
        let mut text = format!(
            "{} at {}",
            self.project,
            crate::builds::short(&self.revision)
        );
        if !self.scope.is_empty() {
            text.push_str(&format!("; about {}", self.scope.join(", ")));
        }
        if let Some(view) = &self.start.view {
            text.push_str(&format!("; starts in {view}"));
        }
        if let Some(element) = &self.start.select {
            text.push_str(&format!("; selects {element}"));
        }
        if !self.vary.is_empty() {
            text.push_str(&format!("; may also explore {}", self.vary.join(", ")));
        }
        text
    }
}

/// Where the explorer starts, before it chooses anything: a view to open
/// (a command's id, such as `requirements-view`), then an element to
/// select (a qualified name). Carried out by rule as the run's first steps
/// after each start, so a replay takes them too.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Start {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
}

impl Start {
    pub fn is_empty(&self) -> bool {
        self.view.is_none() && self.select.is_none()
    }
}

/// Where a run's project comes from (C-54): the folder of a checkout of a
/// revision its model files are copied from, with their digest there, and
/// the project's git tree at the revision when it is known. The checkout is
/// clean at the revision (it is made again when anything in it changed), so
/// its files are the revision's, but for any git ignores. Explorations and
/// replays check the copy their instance opened against it before their
/// first action ([`check_copy`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// The folder its files are copied from.
    pub folder: PathBuf,
    pub revision: String,
    /// [`model_digest`] of the files there.
    pub digest: String,
    /// `git rev-parse <revision>:<project>`: the project's identity in the
    /// repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<String>,
}

impl Provenance {
    /// Project `project` of `checkout`, a checkout of `revision`, with the
    /// digest of its model files; why not, when the checkout has no model
    /// files there.
    pub fn of(checkout: &Path, project: &str, revision: &str) -> Result<Provenance, String> {
        let folder = model_folder(&within(checkout, project));
        let digest = model_digest(&folder).map_err(|e| {
            format!(
                "the project {project} has no model at {}: {e}",
                crate::builds::short(revision)
            )
        })?;
        Ok(Provenance {
            folder,
            revision: revision.to_string(),
            digest,
            tree: None,
        })
    }
}

/// The copy of a project a test instance opened at a start: the folder it
/// opened as its project, the folder its files were copied from, and their
/// digest ([`model_digest`] of the copy).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub folder: PathBuf,
    pub from: PathBuf,
    pub digest: String,
}

/// Why a test instance is not on a copy of the planned project (the W13.7
/// repair).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "reason", rename_all = "lowercase")]
pub enum Mismatch {
    /// The copy is not the project's (copied from elsewhere, other files, or
    /// not said): the Orchestrator's own copying went wrong, not the build.
    Copy(String),
    /// The instance shows another project, or none: the build under test
    /// did not open what it was given.
    Shown(String),
}

impl Mismatch {
    /// Why, in a phrase.
    pub fn reason(&self) -> &str {
        let (Mismatch::Copy(reason) | Mismatch::Shown(reason)) = self;
        reason
    }
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the test instance did not open the planned project: {}",
            self.reason()
        )
    }
}

/// Whether `instance` opened a copy of `source` (the W13.7 repair): copied
/// from its folder, with its files' digest, and shown as the project in
/// `observation`, its first after the start. Returns what the instance
/// said it opened, with why it does not match, if it does not. Explorations
/// and replays check it before their first action.
pub fn check_copy(
    instance: &dyn Instance,
    source: &Provenance,
    observation: &Value,
) -> (Option<Opened>, Result<(), Mismatch>) {
    let Some(opened) = instance.opened() else {
        return (
            None,
            Err(Mismatch::Copy(
                "the instance does not say which copy it opened, so it cannot be checked".into(),
            )),
        );
    };
    let revision = crate::builds::short(&source.revision);
    let checked = if !same_folder(&opened.from, &source.folder) {
        Err(Mismatch::Copy(format!(
            "it copied {}, not {} of {revision}",
            opened.from.display(),
            source.folder.display()
        )))
    } else if opened.digest != source.digest {
        Err(Mismatch::Copy(format!(
            "its copy's files (digest {}) are not those of {revision} (digest {})",
            opened.digest, source.digest
        )))
    } else {
        match observation["project"]["folder"]
            .as_str()
            .or_else(|| observation["identity"]["project"].as_str())
        {
            Some(folder) if same_folder(Path::new(folder), &opened.folder) => Ok(()),
            Some(folder) => Err(Mismatch::Shown(format!(
                "it shows the project {folder}, not its copy {}",
                opened.folder.display()
            ))),
            None => Err(Mismatch::Shown("it shows no project open".into())),
        }
    };
    (Some(opened), checked)
}

impl Instance for LiveInstance {
    fn observe(&mut self) -> Result<Value, String> {
        let observation = self.client()?.observe(true)?;
        if observation["ok"] == false {
            return Err(observation["error"]
                .as_str()
                .unwrap_or_default()
                .to_string());
        }
        Ok(observation)
    }

    fn act(&mut self, act: &Act) -> Result<Value, String> {
        let client = self.client()?;
        let instance = client.instance.clone();
        client.answer_within(answer_time(act.action))?;
        let answer = client.call(json!({
            "op": "act",
            "agent": act.agent,
            "why": act.why,
            "goal": act.goal,
            "expect": { "instance": instance },
            "observed": act.observed,
            "action": act.action,
        }));
        client.answer_within(ANSWER)?;
        answer
    }

    fn restart(&mut self, stop: &mut dyn FnMut() -> bool) -> Result<(), String> {
        self.client = None;
        self.instance = None;
        let _ = std::fs::remove_dir_all(self.folder.join(format!("start-{}", self.starts)));
        self.starts += 1;
        let base = self.folder.join(format!("start-{}", self.starts));
        let project = base.join("project");
        self.opened = None;
        let digest = copy_model(&self.start, &project.join("model"))?;
        self.opened = Some(Opened {
            folder: project.clone(),
            from: model_folder(&self.start),
            digest,
        });
        // Its repository is the project's copy, so nothing it opens or
        // writes lies outside its folder.
        let mut instance = TestInstance::start_with(
            &self.exe,
            &base.join("instance"),
            &project,
            &project,
            &self.options,
        )?;
        // Connected in short slices, so a stop is heard while it starts.
        let started = Instant::now();
        let mut client = loop {
            match instance.connect(Duration::from_millis(500)) {
                Ok(client) => break client,
                Err(error) if !instance.alive() || started.elapsed() >= START => return Err(error),
                Err(_) if stop() => return Err("stopped".into()),
                Err(_) => {}
            }
        };
        client.answer_within(ANSWER)?;
        self.instance = Some(instance);
        self.client = Some(client);
        Ok(())
    }

    fn alive(&mut self) -> bool {
        self.instance.as_mut().is_some_and(TestInstance::alive)
    }

    fn folder(&self) -> Option<PathBuf> {
        Some(self.folder.clone())
    }

    fn opened(&self) -> Option<Opened> {
        self.opened.clone()
    }
}

impl Drop for LiveInstance {
    fn drop(&mut self) {
        self.client = None;
        self.instance = None;
        crate::control::remove_folder(&self.folder);
    }
}

/// One action taken, as a replay takes it again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// The action as it was sent.
    pub action: Value,
    /// Its coverage key.
    pub key: String,
    /// The screen it was taken on (and the dialog or palette open there).
    pub screen: String,
    /// The label of what it acted on.
    pub label: String,
    /// What the explorer expected it to show.
    #[serde(default)]
    pub expect: Option<Value>,
    /// `explorer` (chosen), `hypothesis` (chosen to test one, by the
    /// escalation role's model: E3), `check` (the undo check's undo and redo, Stop
    /// after a turn ran past its budget), `wait` (for a turn of the
    /// Assistant to end: `label` is `the turn`, or `the stop` after Stop),
    /// `recovery` (a dialog cancelled by rule, Escape out of a dead end) or
    /// `start` (the plan's start: its view, its element, by rule).
    pub by: String,
}

impl Step {
    /// The control, command or keys it acts on.
    pub fn target(&self) -> &str {
        self.action["control"]
            .as_str()
            .or_else(|| self.action["id"].as_str())
            .or_else(|| self.action["keys"].as_str())
            .unwrap_or_default()
    }
}

/// The screen an observation shows, as coverage keys and steps name it:
/// `surface`, `surface:Create` (a dialog), `surface:palette`.
pub fn screen_of(observation: &Value) -> String {
    let mut screen = observation["screen"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    if let Some(dialog) = observation["dialog"].as_str() {
        screen = format!("{screen}:{dialog}");
    } else if !observation["palette"].is_null() {
        screen.push_str(":palette");
    }
    screen
}

/// The Studio area a control is in: the panel shown in its column, the
/// dialog's kind, or its region.
pub fn area(observation: &Value, control: &Value) -> String {
    match control["region"].as_str().unwrap_or_default() {
        "inspector" => observation["panels"]["right"]
            .as_str()
            .unwrap_or("inspector")
            .to_string(),
        "left-body" => observation["panels"]["left"]
            .as_str()
            .unwrap_or("outline")
            .to_string(),
        "dialog" => format!(
            "dialog:{}",
            observation["dialog"].as_str().unwrap_or_default()
        ),
        region => region.to_string(),
    }
}

/// The fixed input classes a field is tried with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// A valid name.
    Name,
    Empty,
    Whitespace,
    /// 300 characters.
    Long,
    NonAscii,
    /// The name of an element the model has.
    Duplicate,
    /// A KerML/SysML keyword.
    Keyword,
    /// A path, with both separators.
    Path,
}

impl Input {
    pub const ALL: [Input; 8] = [
        Input::Name,
        Input::Empty,
        Input::Whitespace,
        Input::Long,
        Input::NonAscii,
        Input::Duplicate,
        Input::Keyword,
        Input::Path,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Input::Name => "name",
            Input::Empty => "empty",
            Input::Whitespace => "whitespace",
            Input::Long => "long",
            Input::NonAscii => "non-ascii",
            Input::Duplicate => "duplicate",
            Input::Keyword => "keyword",
            Input::Path => "path",
        }
    }

    /// The text it types; a duplicate needs an existing name.
    pub fn text(self, existing: Option<&str>) -> Option<String> {
        Some(match self {
            Input::Name => "Probe".into(),
            Input::Empty => String::new(),
            Input::Whitespace => "   ".into(),
            Input::Long => format!("Probe{}", "x".repeat(295)),
            Input::NonAscii => "Größe_名前".into(),
            Input::Duplicate => existing?.to_string(),
            Input::Keyword => "part".into(),
            Input::Path => "a/b\\c".into(),
        })
    }

    /// The class of a text a model chose to type.
    pub fn of(text: &str, existing: Option<&str>) -> Input {
        if text.is_empty() {
            Input::Empty
        } else if text.trim().is_empty() {
            Input::Whitespace
        } else if existing == Some(text) {
            Input::Duplicate
        } else if text.chars().count() >= 100 {
            Input::Long
        } else if text.contains(['/', '\\']) {
            Input::Path
        } else if !text.is_ascii() {
            Input::NonAscii
        } else if KEYWORDS.contains(&text) {
            Input::Keyword
        } else {
            Input::Name
        }
    }
}

/// KerML/SysML keywords a name could collide with.
const KEYWORDS: [&str; 12] = [
    "part",
    "port",
    "item",
    "attribute",
    "action",
    "state",
    "requirement",
    "interface",
    "connection",
    "package",
    "def",
    "import",
];

/// The fixed requests the explorer makes in the Conversation: what a person
/// asks Agentique's Assistant, and what nobody should have to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Explain the selected element.
    Explain,
    /// Add a part with a given name.
    AddPart,
    /// Connect two parts of the model.
    Connect,
    /// Something unrelated to the model.
    OffTopic,
    Empty,
    Whitespace,
    /// About 2,500 characters.
    Long,
}

impl Request {
    pub const ALL: [Request; 7] = [
        Request::Explain,
        Request::AddPart,
        Request::Connect,
        Request::OffTopic,
        Request::Empty,
        Request::Whitespace,
        Request::Long,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Request::Explain => "explain",
            Request::AddPart => "add-part",
            Request::Connect => "connect",
            Request::OffTopic => "off-topic",
            Request::Empty => "empty",
            Request::Whitespace => "whitespace",
            Request::Long => "long",
        }
    }

    /// Its text; connecting names two elements the model has, when it has
    /// two.
    pub fn text(self, names: &[String]) -> String {
        match self {
            Request::Explain => "Explain what the selected element does, in two sentences.".into(),
            Request::AddPart => "Add a part named Probe to the model.".into(),
            Request::Connect => match names {
                [a, b, ..] => format!("Connect {a} to {b}."),
                _ => "Connect the first two parts of the model.".into(),
            },
            Request::OffTopic => "What is a good recipe for pancakes?".into(),
            Request::Empty => String::new(),
            Request::Whitespace => "   ".into(),
            Request::Long => {
                "Describe every part of the model and how the parts work together. ".repeat(38)
            }
        }
    }

    /// The class of a request's text: one of the fixed ones, or `written`
    /// (a model wrote it).
    pub fn of(text: &str, names: &[String]) -> &'static str {
        Request::ALL
            .into_iter()
            .find(|r| r.text(names) == text)
            .map_or("written", Request::name)
    }
}

/// An action that is valid on the screen observed.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The action as it is sent.
    pub action: Value,
    /// Its coverage key: `screen|region|control|action[|input class]`.
    pub key: String,
    /// The Studio area it is in ([`area`]): for progress and relevance.
    pub area: String,
    /// What it does, in words.
    pub about: String,
    /// The label of what it acts on.
    pub label: String,
    /// A field: a model may give its own text.
    pub field: bool,
    /// It opens a view or a panel (a tab or an option outside a dialog, or
    /// a command that shows a view): navigation, which changes nothing.
    pub navigates: bool,
}

/// The Operator's own where an observation does not say with `operatorOnly`
/// (a Studio that marks nothing): Settings and the Conversation (regions)
/// and the Operator's commands (the Studio's `OPERATORS_COMMANDS`). Where
/// the observation marks controls and commands, its marks decide.
const OPERATORS_REGIONS: [&str; 2] = ["settings", "conversation"];
/// Never the explorer's, whatever the marks say: the Objectives panel
/// (objectives are the Operator's; a lead's delegation is not the
/// explorer's), the agents' Pause, Step and Resume (the supervisor's), and
/// the system's folder picker, which no agent can see.
const NEVER_PREFIXES: [&str; 3] = ["objective-", "agents-", "browse-"];
/// Undo and redo are the undo check's, never chosen.
const CHECKS_COMMANDS: [&str; 2] = ["undo", "redo"];
const OPERATORS_COMMANDS: [&str; 10] = [
    "lock",
    "trust-local",
    "ask-assistant",
    "insert-selection",
    "new-conversation",
    "undo",
    "redo",
    "theme",
    "contrast",
    "reduced-motion",
];
/// The window's own minimize, maximize and close: they hide or end the
/// instance, which is the operating system's business, not the Studio's.
const WINDOW_CONTROLS: &str = "window-";

/// A path's parts, split at either separator, without empty and `.` parts
/// (case-folded on Windows, whose paths ignore case).
fn parts(path: &str) -> Vec<String> {
    path.split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .map(|p| {
            if cfg!(windows) {
                p.to_lowercase()
            } else {
                p.to_string()
            }
        })
        .collect()
}

/// Whether `value` starts with a drive (`C:`).
fn drive(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Whether a text, taken as a path, could lead anywhere a plain relative
/// path cannot: it climbs (`..`), or is rooted, on a drive or a UNC path.
pub fn climbs(value: &str) -> bool {
    let value = value.trim();
    value.starts_with(['/', '\\']) || drive(value) || parts(value).iter().any(|p| p == "..")
}

/// Whether a path in a dialog's field could make it write or open outside
/// `folder`: one that climbs (`..`), one that is root-relative,
/// drive-relative or UNC, and an absolute one not inside `folder`
/// (compared part by part, so `run2` is not inside `run`). A plain relative
/// path resolves in the instance's working folder, which is inside
/// `folder`.
pub fn escapes(value: &str, folder: &Path) -> bool {
    let value = value.trim();
    if !climbs(value) {
        return false;
    }
    if parts(value).iter().any(|p| p == "..") {
        return true;
    }
    let absolute = (drive(value) && value[2..].starts_with(['/', '\\']))
        || (!cfg!(windows) && value.starts_with('/') && !value.starts_with("//"));
    !absolute || !parts(value).starts_with(&parts(&folder.display().to_string()))
}

/// The names of elements the model has (without their packages): the cards
/// in view, then the selection. The first is the duplicate input; requests
/// that name two take the first two.
fn existing_names(observation: &Value) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let qualified = observation["cards"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c["element"].as_str())
        .chain(
            observation["selection"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        );
    for q in qualified {
        let name = q.rsplit("::").next().unwrap_or(q).trim_matches('\'');
        if !name.is_empty() && !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// The actions that are valid on the screen observed and that it offers to
/// agents: enabled, visible controls with interactive roles (a field with
/// each input class), the available commands, Escape, and Enter in a
/// focused field. Behind a dialog or the palette only it is reachable; a
/// dialog asking for the Operator's approval leaves nothing; on the Settings
/// screen an agent may only leave. A dialog holding an absolute path
/// outside `folder` is not confirmed (the New project dialog proposes one in
/// the user's home). Deterministic: no model ever sees an invalid option.
pub fn candidates(observation: &Value, folder: Option<&Path>) -> Vec<Candidate> {
    if !observation["approval"].is_null() {
        return Vec::new();
    }
    let screen = screen_of(observation);
    let escape = Candidate {
        action: json!({ "kind": "key", "keys": "escape" }),
        key: format!("{screen}|keys|escape|key"),
        area: "keys".into(),
        about: "press Escape".into(),
        label: "Escape".into(),
        field: false,
        navigates: false,
    };
    if observation["screen"] == "settings" {
        return vec![escape];
    }
    let scope = if observation["dialog"].is_string() {
        Some("dialog")
    } else if !observation["palette"].is_null() {
        Some("palette")
    } else {
        None
    };
    let guarded = scope == Some("dialog")
        && folder.is_some_and(|folder| {
            observation["controls"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["region"] == "dialog" && c["role"] == "field")
                .any(|c| escapes(c["value"].as_str().unwrap_or_default(), folder))
        });
    let names = existing_names(observation);
    let existing = names.first().cloned();
    let marked = observed::marks(observation);
    // What submitting would submit, by region: the input class of each of a
    // dialog's fields, the request class of the Conversation's composer. So
    // confirming or sending other inputs is other coverage.
    let mut submitting: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for c in observation["controls"].as_array().into_iter().flatten() {
        let region = c["region"].as_str().unwrap_or_default();
        if c["role"] != "field" || !matches!(region, "dialog" | "conversation") {
            continue;
        }
        let value = c["value"].as_str().unwrap_or_default();
        let class = if region == "conversation" {
            Request::of(value, &names)
        } else {
            Input::of(value, existing.as_deref()).name()
        };
        submitting
            .entry(region)
            .or_default()
            .push(format!("{}={class}", c["id"].as_str().unwrap_or_default()));
    }
    let with_inputs = |region: &str, key: String, about: String| match submitting.get(region) {
        Some(inputs) => {
            let mut inputs = inputs.clone();
            inputs.sort();
            let inputs = inputs.join(",");
            (format!("{key}|{inputs}"), format!("{about} with {inputs}"))
        }
        None => (key, about),
    };
    let mut list = Vec::new();
    let mut focused = None;
    for control in observation["controls"].as_array().into_iter().flatten() {
        let id = control["id"].as_str().unwrap_or_default();
        let role = control["role"].as_str().unwrap_or_default();
        let region = control["region"].as_str().unwrap_or_default();
        if scope.is_some_and(|s| s != region)
            || !findings::INTERACTIVE.contains(&role)
            || control["enabled"] == false
            || control["hidden"] == true
            || observed::operator_only(control)
            || (!marked && OPERATORS_REGIONS.contains(&region))
            || NEVER_PREFIXES.iter().any(|p| id.starts_with(p))
            || id.starts_with(WINDOW_CONTROLS)
            || id.is_empty()
            || (guarded && id == observed::CONFIRM)
        {
            continue;
        }
        let area = area(observation, control);
        let label = control["label"].as_str().unwrap_or_default().to_string();
        if role == "field" && region == "conversation" {
            // The composer: requests of the fixed classes.
            if control["focused"] == true {
                focused = Some((region.to_string(), area.clone(), label.clone()));
            }
            for request in Request::ALL {
                list.push(Candidate {
                    action: json!({ "kind": "fill", "control": id, "text": request.text(&names) }),
                    key: format!("{screen}|{region}|{id}|fill|{}", request.name()),
                    about: format!(
                        "write the {} request into the Conversation's “{label}”",
                        request.name()
                    ),
                    area: area.clone(),
                    label: label.clone(),
                    field: true,
                    navigates: false,
                });
            }
        } else if role == "field" {
            if control["focused"] == true {
                focused = Some((region.to_string(), area.clone(), label.clone()));
            }
            for input in Input::ALL {
                let Some(text) = input.text(existing.as_deref()) else {
                    continue;
                };
                list.push(Candidate {
                    action: json!({ "kind": "fill", "control": id, "text": text }),
                    key: format!("{screen}|{region}|{id}|fill|{}", input.name()),
                    about: format!(
                        "type {} text into the field “{label}” ({area})",
                        input.name()
                    ),
                    area: area.clone(),
                    label: label.clone(),
                    field: true,
                    navigates: false,
                });
            }
        } else {
            let key = format!("{screen}|{region}|{id}|click");
            let about = format!("click the {role} “{label}” ({area})");
            let submits =
                id == observed::CONFIRM || (region == "conversation" && id == observed::SEND);
            let (key, about) = if submits {
                with_inputs(region, key, about)
            } else {
                (key, about)
            };
            list.push(Candidate {
                action: json!({ "kind": "click", "control": id }),
                key,
                about,
                // A panel's tab; never an option (the Inspector's type
                // matches change the model, the palette's rows run
                // commands), nor a dialog's or the palette's tab.
                navigates: role == "tab" && region != "dialog" && region != "palette",
                area,
                label,
                field: false,
            });
        }
    }
    if scope.is_none() {
        for command in observation["commands"].as_array().into_iter().flatten() {
            let id = command["id"].as_str().unwrap_or_default();
            if command["available"] == false
                || observed::operator_only(command)
                || (!marked && OPERATORS_COMMANDS.contains(&id))
                || CHECKS_COMMANDS.contains(&id)
                || id.is_empty()
            {
                continue;
            }
            let label = command["label"].as_str().unwrap_or_default().to_string();
            list.push(Candidate {
                action: json!({ "kind": "command", "id": id }),
                key: format!("{screen}|command|{id}|command"),
                about: format!("run the command “{label}”"),
                area: "command".into(),
                label,
                field: false,
                navigates: id.ends_with("-view") || id.starts_with("show-"),
            });
        }
    }
    if let Some((region, area, label)) = focused
        && !guarded
    {
        let (key, about) = with_inputs(
            &region,
            format!("{screen}|{region}|enter|key"),
            format!("press Enter in the field “{label}” ({area})"),
        );
        list.push(Candidate {
            action: json!({ "kind": "key", "keys": "enter" }),
            key,
            about,
            area,
            label,
            field: false,
            navigates: false,
        });
    }
    list.push(escape);
    list
}

/// What has changed recently: exploration prefers the areas it touches.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Changes {
    /// Changed file paths, relative to the repository.
    #[serde(default)]
    pub paths: Vec<String>,
    /// The subjects of the commits that changed them.
    #[serde(default)]
    pub subjects: Vec<String>,
}

/// Where a change to the Studio's code shows: the areas exploring it reaches.
/// A path that matches none points to no area.
const CHANGED_AREAS: [(&str, &str); 22] = [
    ("panels/objectives.rs", "objectives"),
    ("src/objectives.rs", "objectives"),
    ("panels/history.rs", "history"),
    ("src/history.rs", "history"),
    ("panels/inspector.rs", "inspector"),
    ("panels/outline.rs", "outline"),
    ("panels/library.rs", "library"),
    ("src/library", "library"),
    ("panels/scenarios.rs", "scenarios"),
    ("panels/requirements.rs", "requirements"),
    ("src/requirements.rs", "requirements"),
    ("panels/problems.rs", "problems"),
    ("panels/run.rs", "run"),
    ("src/runs.rs", "run"),
    ("src/dialogs.rs", "dialog"),
    ("src/edit.rs", "dialog"),
    ("src/palette.rs", "palette"),
    ("src/welcome.rs", "welcome"),
    ("src/surface", "surface"),
    ("src/workspace.rs", "title"),
    ("src/conversation", "conversation"),
    ("src/settings", "settings"),
];

impl Changes {
    /// The Studio areas the changed paths touch.
    pub fn areas(&self) -> BTreeSet<String> {
        self.paths
            .iter()
            .map(|p| p.replace('\\', "/"))
            .filter(|p| p.contains("studio-native/"))
            .flat_map(|p| {
                CHANGED_AREAS
                    .iter()
                    .filter(move |(pattern, _)| p.contains(pattern))
                    .map(|(_, area)| area.to_string())
            })
            .collect()
    }
}

/// Words that say nothing about where to go.
const STOP_WORDS: [&str; 24] = [
    "with", "that", "this", "from", "into", "then", "them", "each", "what", "when", "where",
    "which", "their", "there", "have", "make", "more", "some", "than", "also", "only", "over",
    "about", "after",
];

/// The words of a text that can name a place: lowercase, four letters or
/// more, without a plural `s`.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 4 && !STOP_WORDS.contains(&w.as_str()))
        .map(|w| match w.strip_suffix('s') {
            Some(stem) if stem.chars().count() >= 4 => stem.to_string(),
            _ => w,
        })
        .collect()
}

/// What makes an action relevant: the goal's words, and the areas and
/// commit subjects of recent changes.
struct Focus {
    goal: BTreeSet<String>,
    changed: BTreeSet<String>,
}

impl Focus {
    fn new(goal: &str, changes: &Changes) -> Focus {
        let mut changed = changes.areas();
        for subject in &changes.subjects {
            changed.extend(words(subject));
        }
        Focus {
            goal: words(goal),
            changed,
        }
    }

    /// Whether the candidate meets the goal's words, and whether it meets a
    /// recent change.
    fn relevance(&self, candidate: &Candidate) -> (bool, bool) {
        let mut own = words(&candidate.label);
        own.extend(words(&candidate.area));
        own.extend(words(candidate.action["id"].as_str().unwrap_or_default()));
        let area = candidate.area.split(':').next().unwrap_or_default();
        (
            !own.is_disjoint(&self.goal),
            self.changed.contains(area) || !own.is_disjoint(&self.changed),
        )
    }
}

/// FNV-1a: a stable hash, so a seed orders equally good actions the same
/// way on every machine and differently from another seed.
fn seeded(seed: u64, key: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// The view or panel `goal` names, if one is offered and not opened yet in
/// this run (the W13.7 repair): a navigation whose label the goal names as
/// a view, a panel or a tab (“the Requirements view”, “the History
/// panel”). The rules take it, and no model is asked.
fn route(candidates: &[Candidate], goal: &str, covered: &BTreeMap<String, u32>) -> Option<usize> {
    let goal = goal.to_lowercase();
    candidates.iter().position(|c| {
        let label = c.label.trim().to_lowercase();
        let named = if label.ends_with(" view") {
            goal.contains(&label)
        } else {
            ["view", "panel", "tab"]
                .iter()
                .any(|place| goal.contains(&format!("{label} {place}")))
        };
        c.navigates && !label.is_empty() && named && !covered.contains_key(&c.key)
    })
}

/// The choice of a route the goal names, by rule.
fn named_route(index: usize) -> (usize, Chosen) {
    (
        index,
        Chosen {
            input: None,
            expect: None,
            why: String::new(),
            decision: Decision {
                choice: String::new(),
                source: Source::Rules,
                confidence: None,
                millis: 0,
                usd: Some(0.0),
                note: "the goal names it".into(),
            },
            counted: 0.0,
        },
    )
}

/// The rules' order: actions never covered in the testing knowledge, then
/// those not covered in this run, then the least covered; within each,
/// those meeting the goal's words, then recent changes; then by the seed.
fn rank(
    candidates: &[Candidate],
    knowledge: &Knowledge,
    covered: &BTreeMap<String, u32>,
    focus: &Focus,
    seed: u64,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by_key(|&i| {
        let c = &candidates[i];
        let here = covered.get(&c.key).copied().unwrap_or(0);
        let tier = match (knowledge.count(&c.key), here) {
            (0, 0) => 0,
            (_, 0) => 1,
            (_, n) => 1 + n,
        };
        let (goal, changed) = focus.relevance(c);
        (tier, !goal, !changed, seeded(seed, &c.key))
    });
    order
}

/// The models a run may ask, as the caller resolved them (the Orchestrator
/// passes its `explorer` and `decisions` roles).
pub struct Deciding<'a> {
    /// Jev (its model, threshold and deadline are its own) and the models:
    /// a [`decide::Decider`], or a stand-in in tests.
    pub answers: &'a dyn Answers,
    /// The explorer's model and effort: the Model way, and the model Jev
    /// escalates a step to when it is unsure (the W13.7 repair: a step is
    /// the explorer's choice, so not the escalation role's reasoning
    /// model, which took a minute or more a step).
    pub explorer: ModelRef,
    pub effort: Option<String>,
    /// The escalation role's reasoning model and effort: a step testing a
    /// hypothesis (an engineering question: which workflow shows the
    /// answer, what the requirement says it must show) is its to decide
    /// (the W13.7 repair, E3).
    pub escalation: ModelRef,
    pub escalation_effort: Option<String>,
}

impl Deciding<'_> {
    /// The model a step asks, at its effort: the escalation role's for a
    /// step testing a hypothesis, else the explorer's.
    pub fn asked(&self, hypothesis: bool) -> (&ModelRef, Option<&str>) {
        if hypothesis {
            (&self.escalation, self.escalation_effort.as_deref())
        } else {
            (&self.explorer, self.effort.as_deref())
        }
    }
}

/// How many of the rules' best actions Jev chooses among, and the model.
/// (Fewer for the model hid what follows an action, such as Send after
/// typing a request, so a step's prompt is made smaller by its state.)
const JEV_OPTIONS: usize = 8;
const MODEL_OPTIONS: usize = 24;

/// The output a Jev call may write (a choice and its distribution), and the
/// requests one Jev decision may send (it retries twice).
const JEV_OUTPUT_TOKENS: u64 = 200;
const JEV_REQUESTS: f64 = 3.0;

/// What the explorer chose, and how.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chosen {
    /// Text a model chose for a field.
    #[serde(default)]
    pub input: Option<String>,
    /// What a model said the next observation would show.
    #[serde(default)]
    pub expect: Option<Value>,
    /// The model's reason, when it gave one.
    #[serde(default)]
    pub why: String,
    /// Who decided, with the confidence, time and cost (a failed Jev or
    /// model call's too; `None` when a call's cost is unknown).
    pub decision: Decision,
    /// What it counts toward the spend budget: its known cost, and for
    /// each call of unknown cost the most that call could have cost.
    #[serde(default)]
    pub counted: f64,
}

const INSTRUCTIONS: &str = "An explorer tests the Agentique application by operating it toward a goal, to find problems. Choose its next action among the options; each is valid on this screen and safe in this test instance. Prefer behaviour not covered before, toward the areas the goal names; do not repeat the last actions; leave a dialog or panel once its behaviour is covered.";

/// The instructions of a step that tests a hypothesis (E3).
const HYPOTHESIS: &str = "An explorer tests one engineering hypothesis about the Agentique application in a test instance: the state's `hypothesis` gives the claim, the requirement that governs it (with its text) or the intended behaviour, the workflow in the GUI that tests it, and what should then be observed. Choose the next action among the options that carries the workflow forward. When the next observation will show whether the claim holds, add \"expect\" with what it must show if the application behaves as the requirement says (concrete text, counts or states, in the grammar given): that check answers the hypothesis. Never state an expectation the state lists as judged wrong before.";

const FORMAT: &str = "{\"choice\": \"<one option id>\", \"why\": \"<one short line>\"}, with \"input\": \"<the text to type>\" added only when the option types into a field (it replaces the option's text; for the Conversation, the request to send; never a path that leads elsewhere), and \"expect\": {...} added only when the action must make something true in the next observation, with any of: screen, dialog (a kind, or null), statusContains, selectionContains, control (an id or label) with labelContains, valueContains or enabled, anyLabelContains, and replyContains (text the Assistant's reply will hold)";

/// The typed question for the next action: the best `n` of the rules'
/// order as options `a01`, `a02`, … (zero-padded, so they read in order),
/// with the ids and the candidates they stand for.
fn question(
    instructions: &str,
    candidates: &[Candidate],
    ranked: &[usize],
    n: usize,
    state: &Value,
    knowledge: &Knowledge,
    covered: &BTreeMap<String, u32>,
) -> (Question, Vec<(String, usize)>) {
    let shown: Vec<(String, usize)> = ranked
        .iter()
        .copied()
        .take(n)
        .enumerate()
        .map(|(i, index)| (format!("a{:02}", i + 1), index))
        .collect();
    let mut options = BTreeMap::new();
    for (id, index) in &shown {
        let c = &candidates[*index];
        let before = knowledge.count(&c.key);
        let here = covered.get(&c.key).copied().unwrap_or(0);
        let note = match (before, here) {
            (0, 0) => "never covered".to_string(),
            (_, 0) => "covered before, not in this run".to_string(),
            (_, n) => format!("covered {n} time(s) in this run"),
        };
        // What it covered is said here once (the W13.7 repair: no longer
        // also in the state).
        options.insert(id.clone(), Some(format!("{} — {note}", c.about)));
    }
    (
        Question {
            instructions: instructions.into(),
            state: state.clone(),
            options,
        },
        shown,
    )
}

/// The candidate an option id stands for.
fn shown_as(shown: &[(String, usize)], choice: &str) -> Option<usize> {
    shown.iter().find(|(id, _)| id == choice).map(|(_, i)| *i)
}

/// Reads the model's answer: an option, text only for a field, an
/// expectation in the grammar of observation criteria, and a reason.
type Said = (Option<String>, Option<Value>, String);

fn read_answer(
    said: &str,
    shown: &[(String, usize)],
    candidates: &[Candidate],
) -> Result<(String, Said), String> {
    let answer = decide::answer_object(said)?;
    let choice = answer["choice"].as_str().unwrap_or_default().to_string();
    let candidate = &candidates[shown_as(shown, &choice)
        .ok_or_else(|| format!("the model chose `{choice}`, which is not an option"))?];
    let input = match &answer["input"] {
        Value::Null => None,
        // A dialog's field never gets a path that leads elsewhere.
        Value::String(text)
            if candidate.field && candidate.area.starts_with("dialog:") && climbs(text) =>
        {
            return Err(format!(
                "`input` for {choice} is a path that could lead outside the test instance"
            ));
        }
        Value::String(text) if candidate.field => Some(text.chars().take(1000).collect()),
        // Nothing to type is no input, whatever the option.
        Value::String(text) if text.is_empty() => None,
        Value::String(_) => {
            return Err(format!(
                "`input` is only for a field, and {choice} is not one"
            ));
        }
        _ => return Err("`input` must be text".into()),
    };
    let expect = match &answer["expect"] {
        Value::Null => None,
        expect => {
            // Its part about the reply is the explorer's own; the rest is
            // in the grammar of observation criteria.
            if !expect[findings::REPLY].is_string() && expect.get(findings::REPLY).is_some() {
                return Err(format!("the expectation: `{}` is text", findings::REPLY));
            }
            if let Some(rest) = findings::deterministic(expect) {
                crate::control::expectation(&rest).map_err(|e| format!("the expectation: {e}"))?;
            } else if expect.get(findings::REPLY).is_none() {
                return Err(
                    "the expectation: an observation criterion must expect something".into(),
                );
            }
            Some(expect.clone())
        }
    };
    let why = answer["why"]
        .as_str()
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect();
    Ok((choice, (input, expect, why)))
}

/// What a decision took: its time, its cost (`None` once a call's cost is
/// unknown) and what it counts toward the spend budget.
#[derive(Clone, Copy, Debug)]
struct Spent {
    millis: u64,
    usd: Option<f64>,
    counted: f64,
}

impl Spent {
    const NOTHING: Spent = Spent {
        millis: 0,
        usd: Some(0.0),
        counted: 0.0,
    };

    /// With a part that took `millis` and cost `usd`, or at most `bound`
    /// when its cost is unknown.
    fn and(self, millis: u64, usd: Option<f64>, bound: f64) -> Spent {
        Spent {
            millis: self.millis + millis,
            usd: usd.and_then(|usd| Some(self.usd? + usd)),
            counted: self.counted + usd.unwrap_or(bound),
        }
    }
}

/// The most a call of unknown cost could have cost (a request that was sent
/// may be billed): its prompt, at two characters a token, and all the
/// output it may write, at the model's price, or without one at the high
/// price the Orchestrator charges unpriced usage ($15 and $75 a million
/// tokens).
fn at_most(model: &ModelRef, prompt: usize, output: u64) -> f64 {
    let usage = agq_providers::Usage {
        input_tokens: (prompt / 2) as u64,
        output_tokens: output,
        ..Default::default()
    };
    usage
        .cost_usd(model)
        .unwrap_or((usage.input_tokens as f64 * 15.0 + usage.output_tokens as f64 * 75.0) / 1e6)
}

/// The rules' choice, carrying the time and cost of what was tried first.
fn by_rule(ranked: &[usize], spent: Spent, note: String) -> (usize, Chosen) {
    (
        ranked[0],
        Chosen {
            input: None,
            expect: None,
            why: String::new(),
            decision: Decision {
                choice: String::new(),
                source: Source::Rules,
                confidence: None,
                millis: spent.millis,
                usd: spent.usd,
                note,
            },
            counted: spent.counted,
        },
    )
}

fn joined(notes: [String; 2]) -> String {
    notes
        .into_iter()
        .filter(|n| !n.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

/// What `choose` decides with.
struct Choosing<'a> {
    /// What a model is told the step is for.
    instructions: &'a str,
    deciding: &'a Deciding<'a>,
    candidates: &'a [Candidate],
    ranked: &'a [usize],
    state: &'a Value,
    knowledge: &'a Knowledge,
    covered: &'a BTreeMap<String, u32>,
}

impl Choosing<'_> {
    /// The model's choice among the rules' best, its answer checked; the
    /// rules' when it fails.
    fn model(
        &self,
        model: &ModelRef,
        effort: Option<&str>,
        spent: Spent,
        note: String,
        source: Source,
        stop: &mut dyn FnMut() -> bool,
    ) -> (usize, Chosen) {
        let (q, shown) = question(
            self.instructions,
            self.candidates,
            self.ranked,
            MODEL_OPTIONS,
            self.state,
            self.knowledge,
            self.covered,
        );
        // One call, and one more when the answer cannot be read.
        let bound = at_most(model, q.prompt(FORMAT).len(), decide::MODEL_OUTPUT_TOKENS) * 2.0;
        let read = |said: &str| read_answer(said, &shown, self.candidates);
        match decide::ask_model(
            self.deciding.answers,
            model,
            effort,
            &q,
            FORMAT,
            &read,
            stop,
        ) {
            Ok((decision, (input, expect, why))) => {
                let spent = spent.and(decision.millis, decision.usd, bound);
                let index = shown_as(&shown, &decision.choice).unwrap_or(self.ranked[0]);
                (
                    index,
                    Chosen {
                        input,
                        expect,
                        why,
                        decision: Decision {
                            source,
                            millis: spent.millis,
                            usd: spent.usd,
                            note: joined([note, decision.note.clone()]),
                            ..decision
                        },
                        counted: spent.counted,
                    },
                )
            }
            Err(failure) => by_rule(
                self.ranked,
                spent.and(failure.millis, failure.usd, bound),
                joined([note, format!("the model failed: {}", failure.error)]),
            ),
        }
    }
}

/// Chooses the next action by `way`: the rules' first; Jev among the
/// rules' best, used when confident; the explorer's model among more of
/// them, its answer checked; Jev escalating to the explorer's model. Whatever fails falls back
/// to the rules, with its time and cost kept; `stop` is asked while a model
/// is waited for.
fn choose(way: Way, choosing: &Choosing, stop: &mut dyn FnMut() -> bool) -> (usize, Chosen) {
    let deciding = choosing.deciding;
    if matches!(way, Way::Rules | Way::Cancel) || choosing.candidates.len() == 1 {
        return by_rule(choosing.ranked, Spent::NOTHING, String::new());
    }
    if way == Way::Model {
        return choosing.model(
            &deciding.explorer,
            deciding.effort.as_deref(),
            Spent::NOTHING,
            String::new(),
            Source::Model,
            stop,
        );
    }
    let (q, shown) = question(
        choosing.instructions,
        choosing.candidates,
        choosing.ranked,
        JEV_OPTIONS,
        choosing.state,
        choosing.knowledge,
        choosing.covered,
    );
    let answers = deciding.answers;
    let bound = at_most(&answers.jev_model(), q.prompt("").len(), JEV_OUTPUT_TOKENS) * JEV_REQUESTS;
    let (spent, note) = match answers.ask_jev(&q) {
        Ok(decision) => {
            let spent = Spent::NOTHING.and(decision.millis, decision.usd, bound);
            match shown_as(&shown, &decision.choice) {
                Some(index) if decision.confidence.unwrap_or(0.0) >= answers.threshold() => {
                    return (
                        index,
                        Chosen {
                            input: None,
                            expect: None,
                            why: String::new(),
                            decision,
                            counted: spent.counted,
                        },
                    );
                }
                _ => (
                    spent,
                    format!(
                        "Jev chose {} with confidence {:.2}",
                        decision.choice,
                        decision.confidence.unwrap_or(0.0)
                    ),
                ),
            }
        }
        Err(failure) => (
            Spent::NOTHING.and(failure.millis, failure.usd, bound),
            format!("{JEV_FAILED}{}", failure.error),
        ),
    };
    if way == Way::Escalating {
        choosing.model(
            &deciding.explorer,
            deciding.effort.as_deref(),
            spent,
            note,
            Source::Escalated,
            stop,
        )
    } else {
        by_rule(choosing.ranked, spent, note)
    }
}

/// What a run is asked to do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub goal: String,
    /// Rules, Jev, Model or Escalating.
    pub way: Way,
    /// Successive runs take different paths among equally good actions.
    pub seed: u64,
    /// Budgets: actions (the explorer's and its recoveries'), seconds and
    /// US dollars.
    pub steps: u32,
    pub seconds: u64,
    pub usd: f64,
    #[serde(default)]
    pub changes: Changes,
    /// The start state, as findings name it (the start project).
    pub start: String,
    /// Where the explorer begins, as the lead planned it: a view opened,
    /// an element selected, by rule after each start.
    #[serde(default, skip_serializing_if = "Start::is_empty")]
    pub begin: Start,
    /// The hypotheses it tests first, in order, each with a share of its
    /// steps (the W13.7 repair, E3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hypotheses: Vec<Hypothesis>,
    /// Where its project comes from: the run checks its instance's copy
    /// against it before its first action, and ends when it does not match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Provenance>,
    /// Whether the explorer may send requests to the instance's Assistant.
    /// Off by default: until exploration instances are given their own
    /// credentials (W12.5), a request would spend on whatever key the
    /// instance has.
    #[serde(default)]
    pub conversation: bool,
    /// How long a turn of the Assistant may take, and Stop to end one.
    #[serde(default = "turn_budget")]
    pub turn_ms: u64,
    #[serde(default = "stop_budget")]
    pub stop_ms: u64,
}

fn turn_budget() -> u64 {
    findings::TURN_BUDGET_MS
}

fn stop_budget() -> u64 {
    findings::STOP_BUDGET_MS
}

/// A step the run took: the action, how it was chosen and what came of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taken {
    pub step: Step,
    /// How the explorer chose it (none for a check's or a recovery's).
    #[serde(default)]
    pub chosen: Option<Chosen>,
    /// `ok`, `refused` (by rule), `stale`, `failed` or `ended`.
    pub outcome: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub took_ms: Option<u64>,
    /// Where its time went (the W13.7 repair).
    #[serde(default)]
    pub timing: Timing,
}

impl Taken {
    /// The step in a line: what it did, how it came out, who chose it, and
    /// its time.
    pub fn line(&self) -> String {
        let t = &self.timing;
        let who = match &self.chosen {
            Some(chosen) => decided_by(chosen, t),
            None => self.step.by.clone(),
        };
        let mut line = format!("{} — {} · {who}", acted(&self.step), self.outcome);
        if t.jev_ms + t.model_ms > 0 {
            line.push_str(&format!(" · {} deciding", tenths(t.jev_ms + t.model_ms)));
        }
        line.push_str(&format!(
            " · {} in the instance",
            tenths(t.act_ms + t.observe_ms)
        ));
        line
    }

    /// What folds under its line: why it was chosen, what was expected,
    /// the instance's answer, where its time went, the model's calls and
    /// tokens, and its cost; `seconds` is the run's time so far.
    pub fn details(&self, seconds: u64) -> String {
        let t = &self.timing;
        let mut details = Vec::new();
        if let Some(chosen) = &self.chosen
            && !chosen.why.is_empty()
        {
            details.push(format!("Why: {}", chosen.why));
        }
        if let Some(expect) = &self.step.expect {
            details.push(format!("Expected: {expect}"));
        }
        if !self.detail.is_empty() {
            details.push(format!("Answer: {}", self.detail));
        }
        details.push(format!(
            "Time: Jev {}, model {}, acting {}, observing {}; {} into the run",
            tenths(t.jev_ms),
            tenths(t.model_ms),
            tenths(t.act_ms),
            tenths(t.observe_ms),
            duration(seconds)
        ));
        if let Some(model) = &t.model {
            details.push(format!(
                "Model: {model}{}, {} call(s), {} prompt characters, {}",
                t.effort
                    .as_ref()
                    .map(|e| format!(" at {e}"))
                    .unwrap_or_default(),
                t.calls,
                t.prompt_chars,
                match &t.tokens {
                    Some(tokens) => format!(
                        "{} input and {} output tokens ({} reasoning)",
                        tokens.input, tokens.output, tokens.reasoning
                    ),
                    None => "no tokens reported".to_string(),
                }
            ));
        }
        if let Some(chosen) = &self.chosen {
            details.push(match chosen.decision.usd {
                Some(usd) => format!("Cost: ${usd:.4}"),
                None => format!("Cost: unknown, counted as at most ${:.4}", chosen.counted),
            });
            if !chosen.decision.note.is_empty() {
                details.push(format!("Note: {}", chosen.decision.note));
            }
        }
        details.join("\n")
    }
}

/// A way out of a state exploration did not choose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    /// The run's action count it happened at.
    pub at: u32,
    /// `observed again`, `cancelled`, `escape` or `restarted`.
    pub kind: String,
    pub detail: String,
}

/// Actions that did nothing or ended the instance.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unwanted {
    /// Refused as the Operator's own.
    pub refused: u32,
    /// Refused as stale (the screen changed, a control gone).
    pub stale: u32,
    /// Ended the instance (it exited or stopped answering).
    pub ended: u32,
    /// Refused as malformed: the explorer's own fault, never the Studio's.
    #[serde(default)]
    pub invalid: u32,
}

impl Unwanted {
    pub fn total(&self) -> u32 {
        self.refused + self.stale + self.ended + self.invalid
    }
}

/// What a run did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub plan: Plan,
    pub build: String,
    pub commit: String,
    pub started: String,
    /// The actions its step budget counts: the explorer's, and those it
    /// took to recover (Escape, a dialog cancelled).
    pub actions: u32,
    pub steps: Vec<Taken>,
    /// Each coverage key covered, with how often.
    pub covered: BTreeMap<String, u32>,
    /// Keys the testing knowledge had not seen.
    pub new_coverage: Vec<String>,
    /// The areas observed during the run.
    pub areas: BTreeSet<String>,
    pub findings: Vec<Finding>,
    pub recoveries: Vec<Recovery>,
    pub unwanted: Unwanted,
    /// Each decision's time.
    pub latencies: Vec<u64>,
    /// What the run counts as spent: its decisions' known costs, the most
    /// each call of unknown cost could have cost, and what the instance's
    /// Assistant spent where the observation says. The spend budget is
    /// held against this.
    pub usd: f64,
    /// Decisions whose cost is unknown (counted at most).
    pub unpriced: u32,
    /// What the instance's Assistant spent, where the observation says.
    #[serde(default)]
    pub assistant_usd: f64,
    pub notes: Vec<String>,
    /// Conditions of the environment the run met, such as the Assistant
    /// needing a key: never findings.
    #[serde(default)]
    pub conditions: Vec<String>,
    /// Why it ended.
    pub ended: String,
    pub seconds: f64,
    /// The copy its instance opened at its first start, as the instance
    /// reported it: with the plan's source, where what it explored came
    /// from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<Opened>,
    /// Why the copy its instance opened is not what the plan's source says:
    /// the run then took no action (never a substitution).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<Mismatch>,
    /// Milliseconds spent starting its instance, fresh starts included (the
    /// W13.7 repair).
    #[serde(default)]
    pub start_ms: u64,
    /// How it answered its plan's hypotheses, in order (the W13.7 repair,
    /// E3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<Answer>,
}

/// The value at percentile `p` (0 to 1) of `values`; 0 when there are none.
pub fn percentile(values: &[u64], p: f64) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

impl Run {
    /// A latency percentile of the decisions.
    pub fn latency(&self, p: f64) -> u64 {
        percentile(&self.latencies, p)
    }

    /// Where its time went, in a line (the W13.7 repair): deciding by Jev
    /// and by the models, the instance (acting, observing), starting it,
    /// and the rest, as shares of its time; the models' calls with their
    /// median and slowest step, and what they were asked and reported.
    pub fn time(&self) -> String {
        let total = ((self.seconds * 1000.0) as u64).max(1);
        let sum =
            |part: fn(&Timing) -> u64| -> u64 { self.steps.iter().map(|t| part(&t.timing)).sum() };
        let (jev, model) = (sum(|t| t.jev_ms), sum(|t| t.model_ms));
        let (act, observe) = (sum(|t| t.act_ms), sum(|t| t.observe_ms));
        let other = total.saturating_sub(jev + model + act + observe + self.start_ms);
        let share = |ms: u64| ms * 100 / total;
        let asked: Vec<u64> = self
            .steps
            .iter()
            .filter(|t| t.timing.calls > 0)
            .map(|t| t.timing.model_ms)
            .collect();
        let calls: u32 = self.steps.iter().map(|t| t.timing.calls).sum();
        let prompts = sum(|t| t.prompt_chars);
        let tokens = self.steps.iter().filter_map(|t| t.timing.tokens).fold(
            None,
            |all: Option<Tokens>, t| {
                let all = all.unwrap_or_default();
                Some(Tokens {
                    input: all.input + t.input,
                    output: all.output + t.output,
                    reasoning: all.reasoning + t.reasoning,
                })
            },
        );
        let mut line = format!(
            "time {}: model {}%, Jev {}%, the instance {}% (acting {}%, observing {}%), starting it {}%, other {}%",
            duration(total / 1000),
            share(model),
            share(jev),
            share(act + observe),
            share(act),
            share(observe),
            share(self.start_ms),
            share(other)
        );
        if calls > 0 {
            line.push_str(&format!(
                "; {calls} model call(s) in {} step(s), p50 {}, slowest {}; {prompts} prompt characters",
                asked.len(),
                tenths(percentile(&asked, 0.5)),
                tenths(percentile(&asked, 1.0)),
            ));
            match tokens {
                Some(t) => line.push_str(&format!(
                    ", {} output tokens ({} reasoning)",
                    t.output, t.reasoning
                )),
                None => line.push_str(", no tokens reported"),
            }
        }
        line
    }
}

/// How often the instance may be started again in one run before the run
/// gives up on it.
const RESTARTS: usize = 4;

/// Escapes in a row, with nothing carried out between them, before the run
/// starts the instance again.
const ESCAPES: u32 = 2;

/// Whether a run must stop now: its supervisor stopped it, or its time is
/// up. Asked during long waits.
fn halted(supervisor: &mut dyn Supervisor, started: Instant, seconds: u64) -> bool {
    supervisor.stopped() || started.elapsed().as_secs() >= seconds
}

/// The explorer's state during a run.
struct Explorer<'a> {
    instance: &'a mut dyn Instance,
    supervisor: &'a mut dyn Supervisor,
    plan: &'a Plan,
    deciding: &'a Deciding<'a>,
    knowledge: &'a Knowledge,
    folder: Option<PathBuf>,
    focus: Focus,
    run: Run,
    /// The steps since the instance last started.
    since: Vec<Step>,
    /// The latest observation, which the next action rests on.
    now: Value,
    /// The dialog the explorer's own last action left open.
    expected_dialog: Value,
    /// Keys refused by rule in this run: not chosen again.
    avoid: BTreeSet<String>,
    /// Keys whose change was checked with undo, and whether undo is the
    /// Operator's here (then it is not checked again).
    undo_checked: BTreeSet<String>,
    undo_refused: bool,
    /// What the explorer expected of a request it sent: checked when its
    /// turn has ended (on the wait for it), not while it runs.
    pending_expect: Option<Value>,
    /// What the instance's Assistant had spent at the last observation.
    assistant_seen: Option<f64>,
    /// Escapes out of dead ends since an action was last carried out.
    escapes: u32,
    reported: BTreeSet<String>,
    restarts: usize,
    started: Instant,
    /// The time of the step being taken, until it is recorded.
    pending: Timing,
    /// The hypothesis being tested (E3), the steps spent on it, the one
    /// whose expectation the next check answers, and that check's result
    /// (the identity of the expectation that failed, if it failed).
    hypothesis: usize,
    spent_on: u32,
    expecting: Option<usize>,
    checked: Option<Option<String>>,
}

/// Explores `instance` as `plan` says, deciding by `plan.way` with
/// `deciding`, preferring what `knowledge` has not covered. `supervisor` is
/// asked before each step whether to go on (it may wait while the run is
/// paused) and during long waits whether it was stopped. Starts the
/// instance fresh, so every finding's steps begin at the start.
pub fn explore(
    instance: &mut dyn Instance,
    plan: &Plan,
    deciding: &Deciding,
    knowledge: &Knowledge,
    supervisor: &mut dyn Supervisor,
) -> Run {
    let folder = instance.folder();
    let mut explorer = Explorer {
        instance,
        supervisor,
        plan,
        deciding,
        knowledge,
        folder,
        focus: Focus::new(&plan.goal, &plan.changes),
        run: Run {
            plan: plan.clone(),
            build: String::new(),
            commit: String::new(),
            started: agq_launcher::now(),
            actions: 0,
            steps: Vec::new(),
            covered: BTreeMap::new(),
            new_coverage: Vec::new(),
            areas: BTreeSet::new(),
            findings: Vec::new(),
            recoveries: Vec::new(),
            unwanted: Unwanted::default(),
            latencies: Vec::new(),
            usd: 0.0,
            unpriced: 0,
            assistant_usd: 0.0,
            notes: Vec::new(),
            conditions: Vec::new(),
            ended: String::new(),
            seconds: 0.0,
            opened: None,
            mismatch: None,
            start_ms: 0,
            answers: Vec::new(),
        },
        since: Vec::new(),
        now: Value::Null,
        expected_dialog: Value::Null,
        avoid: BTreeSet::new(),
        undo_checked: BTreeSet::new(),
        undo_refused: false,
        pending_expect: None,
        assistant_seen: None,
        escapes: 0,
        reported: BTreeSet::new(),
        restarts: 0,
        started: Instant::now(),
        pending: Timing::default(),
        hypothesis: 0,
        spent_on: 0,
        expecting: None,
        checked: None,
    };
    explorer.run.ended = match explorer.go() {
        Ok(()) => "the step budget was used".into(),
        Err(why) => why,
    };
    // What it did not get to is not answered, never a finding.
    let ended = explorer.run.ended.clone();
    while explorer.hypothesis < plan.hypotheses.len() {
        explorer.answer(
            explorer.hypothesis,
            Verdict::NotAnswered {
                why: format!("the run ended: {ended}"),
            },
        );
    }
    let mut run = explorer.run;
    run.seconds = explorer.started.elapsed().as_secs_f64();
    run.new_coverage = run
        .covered
        .keys()
        .filter(|k| knowledge.count(k) == 0)
        .cloned()
        .collect();
    run
}

impl Explorer<'_> {
    fn go(&mut self) -> Result<(), String> {
        if self.plan.way == Way::Cancel {
            return Err("the cancelling rule decides dialogs in the way, not exploration".into());
        }
        // The rules state no expectation: they answer no hypothesis.
        if matches!(self.plan.way, Way::Rules) {
            while self.hypothesis < self.plan.hypotheses.len() {
                self.answer(
                    self.hypothesis,
                    Verdict::NotAnswered {
                        why: "the rules state no expectation".into(),
                    },
                );
            }
        }
        self.start(false)?;
        while self.run.actions < self.plan.steps {
            if !self.supervisor.go_on() {
                return Err("stopped".into());
            }
            self.within_budgets()?;
            self.step()?;
            self.settle();
        }
        // A request sent last: its turn ends (and is checked) within the run.
        if observed::turn_running(&self.now) {
            self.await_turn()?;
        }
        Ok(())
    }

    /// Whether time and money are left.
    fn within_budgets(&self) -> Result<(), String> {
        if self.started.elapsed().as_secs() >= self.plan.seconds {
            return Err("the time budget was used".into());
        }
        if self.run.usd >= self.plan.usd {
            return Err("the spend budget was used".into());
        }
        Ok(())
    }

    /// Why a long wait ended early: stopped, or out of time.
    fn why_halted(&mut self) -> String {
        if self.supervisor.stopped() {
            "stopped".into()
        } else {
            "the time budget was used".into()
        }
    }

    /// Starts the instance (again), from the same start.
    fn start(&mut self, again: bool) -> Result<(), String> {
        if again {
            self.restarts += 1;
            if self.restarts > RESTARTS {
                return Err(format!(
                    "the instance was started again {RESTARTS} times; it keeps ending"
                ));
            }
        }
        self.since.clear();
        self.expected_dialog = Value::Null;
        self.assistant_seen = None;
        self.escapes = 0;
        let (supervisor, started, seconds) =
            (&mut *self.supervisor, self.started, self.plan.seconds);
        let starting = Instant::now();
        let restarted = self
            .instance
            .restart(&mut || halted(supervisor, started, seconds));
        if let Err(error) = restarted {
            if halted(self.supervisor, started, seconds) {
                return Err(self.why_halted());
            }
            return Err(format!("the test instance did not start: {error}"));
        }
        let now = self.instance.observe();
        self.run.start_ms += starting.elapsed().as_millis() as u64;
        let now = now.map_err(|e| format!("the test instance does not answer: {e}"))?;
        // What it opened is the plan's project, or the run ends here.
        if let Err(mismatch) = self.check_opened(&now) {
            let ended = mismatch.to_string();
            self.run.mismatch = Some(mismatch);
            return Err(ended);
        }
        if self.run.build.is_empty() {
            self.run.build = now["identity"]["build"]
                .as_str()
                .unwrap_or("development")
                .to_string();
            self.run.commit = now["identity"]["commit"]
                .as_str()
                .unwrap_or_default()
                .to_string();
        }
        self.seen(&now);
        let found = findings::failures(&Outcome {
            before: &now,
            step: None,
            answer: None,
            after: &now,
        });
        self.report(found, &[]);
        self.now = now;
        self.start_steps()
    }

    /// Whether the instance opened a copy of the plan's source: copied from
    /// its folder, with its files' digest, and shown as the project in the
    /// observation `now`. A plan without a source checks nothing.
    fn check_opened(&mut self, now: &Value) -> Result<(), Mismatch> {
        let Some(source) = &self.plan.source else {
            return Ok(());
        };
        let (opened, checked) = check_copy(self.instance, source, now);
        // The first copy that matched, or the one that did not.
        if checked.is_err() || self.run.opened.is_none() {
            self.run.opened = opened;
        }
        checked
    }

    /// The plan's start, by rule, as the first steps after each start (a
    /// replay takes them too): its view opened, then its element selected.
    /// A view the instance does not offer is noted, not forced.
    fn start_steps(&mut self) -> Result<(), String> {
        let start = self.plan.begin.clone();
        if let Some(view) = &start.view {
            let offered = candidates(&self.now, self.folder.as_deref())
                .into_iter()
                .find(|c| c.action["kind"] == "command" && c.action["id"] == view.as_str());
            match offered {
                Some(command) => {
                    self.run.actions += 1;
                    let step = Step {
                        action: command.action,
                        key: command.key,
                        screen: screen_of(&self.now),
                        label: command.label,
                        expect: None,
                        by: "start".into(),
                    };
                    self.take(step, None, "the plan's start: its view")?;
                }
                None => self.note(format!(
                    "the plan's start view `{view}` is not a command this instance offers here"
                )),
            }
        }
        if let Some(element) = &start.select {
            self.run.actions += 1;
            let screen = screen_of(&self.now);
            let step = Step {
                action: json!({ "kind": "select", "element": element }),
                key: format!("{screen}|selection|element|select"),
                screen,
                label: element.clone(),
                expect: None,
                by: "start".into(),
            };
            self.take(step, None, "the plan's start: its element")?;
        }
        Ok(())
    }

    /// A note of the run's, once.
    fn note(&mut self, note: String) {
        if !self.run.notes.contains(&note) {
            self.run.notes.push(note);
        }
    }

    /// The areas an observation shows, the conditions it reports, and what
    /// the instance's Assistant spent since the last one.
    fn seen(&mut self, observation: &Value) {
        for control in observation["controls"].as_array().into_iter().flatten() {
            if control["hidden"] != true {
                self.run.areas.insert(area(observation, control));
            }
        }
        if let Some(condition) = observed::needs_key(observation)
            && !self.run.conditions.contains(&condition)
        {
            self.run.conditions.push(condition);
        }
        if let Some(spent) = observed::assistant_spend(observation) {
            let since = (spent - self.assistant_seen.unwrap_or(0.0)).max(0.0);
            self.run.usd += since;
            self.run.assistant_usd += since;
            self.assistant_seen = Some(spent);
        }
    }

    /// Records the failures not reported yet in this run, with the steps
    /// since the start.
    fn report(&mut self, failed: Vec<Failed>, steps: &[Step]) {
        for failed in failed {
            if self.reported.insert(failed.identity()) {
                self.run.findings.push(Finding::new(
                    failed,
                    steps.to_vec(),
                    &self.run.build,
                    &self.run.commit,
                    &self.plan.start,
                ));
            }
        }
    }

    /// The instance ended or stopped answering after the steps since the
    /// start: a finding, and a fresh start.
    fn ended(&mut self, error: &str) -> Result<(), String> {
        let control = self
            .since
            .last()
            .map(|s| s.target().to_string())
            .unwrap_or_default();
        let failed = findings::ended(self.instance, &control, error);
        let steps = self.since.clone();
        self.report(vec![failed], &steps);
        self.recover("restarted", format!("the instance ended: {error}"));
        self.start(true)
    }

    fn recover(&mut self, kind: &str, detail: String) {
        self.run.recoveries.push(Recovery {
            at: self.run.actions,
            kind: kind.into(),
            detail,
        });
    }

    fn observe(&mut self) -> Result<Option<Value>, String> {
        let observing = Instant::now();
        let observed = self.instance.observe();
        self.pending.observe_ms += observing.elapsed().as_millis() as u64;
        match observed {
            Ok(now) => {
                self.seen(&now);
                Ok(Some(now))
            }
            Err(error) => {
                self.ended(&error)?;
                Ok(None)
            }
        }
    }

    /// One action of the explorer's, or a recovery.
    fn step(&mut self) -> Result<(), String> {
        // A dialog the explorer's own action did not leave open is in its
        // way: cancelled by rule (an approval cannot be: a fresh start).
        if self.now["dialog"].is_string() && self.now["dialog"] != self.expected_dialog {
            return self.cancel_dialog();
        }
        // A turn of the Assistant runs (a request was sent): its end is
        // waited for before anything else.
        if observed::turn_running(&self.now) {
            return self.await_turn();
        }
        let mut candidates = candidates(&self.now, self.folder.as_deref());
        candidates.retain(|c| !self.avoid.contains(&c.key));
        // Requests to the instance's Assistant only where the run may send
        // them and the observation shows what the Assistant spends (so its
        // spend counts toward the budget); otherwise none is sent.
        if !self.plan.conversation || observed::assistant_spend(&self.now).is_none() {
            if self.plan.conversation {
                let note = "no request was sent: the observation does not show what the instance's Assistant spends (`conversation.usd`)".to_string();
                if !self.run.notes.contains(&note) {
                    self.run.notes.push(note);
                }
            }
            candidates.retain(|c| c.area != "conversation");
        }
        if candidates.is_empty() {
            return self.dead_end();
        }
        // A hypothesis first (E3): toward its workflow, decided by the
        // escalation role's model (an engineering question; a model, since
        // Jev states no expectation).
        let testing = self.testing();
        let (way, aim) = match &testing {
            Some(h) => (Way::Model, format!("{} {}", h.workflow, h.claim)),
            None => (self.plan.way, self.plan.goal.clone()),
        };
        if testing.is_some() {
            self.expecting = None;
        }
        let toward = testing
            .as_ref()
            .map(|_| Focus::new(&aim, &self.plan.changes));
        let ranked = rank(
            &candidates,
            self.knowledge,
            &self.run.covered,
            toward.as_ref().unwrap_or(&self.focus),
            self.plan.seed,
        );
        let mut state = self.state();
        if let Some(h) = &testing {
            state["hypothesis"] = json!({
                "claim": h.claim,
                "requirement": h.requirement,
                "requirementText": h.requirement_text,
                "behaviour": h.behaviour,
                "workflow": h.workflow,
                "expected": h.expected,
                "stepsLeft": self.share().saturating_sub(self.spent_on),
            });
        }
        // A view or panel the goal (or the workflow) names, not opened yet
        // in this run: the rules take it, and no model is asked (the W13.7
        // repair).
        let routed = (!matches!(way, Way::Rules | Way::Cancel))
            .then(|| route(&candidates, &aim, &self.run.covered))
            .flatten();
        let step = self.run.actions + 1;
        // Asked through a measure of where the decision's time goes.
        let measured = Measured::new(self.deciding.answers);
        let (model_asked, effort) = self.deciding.asked(testing.is_some());
        let deciding = Deciding {
            answers: &measured,
            explorer: model_asked.clone(),
            effort: effort.map(str::to_string),
            escalation: self.deciding.escalation.clone(),
            escalation_effort: self.deciding.escalation_effort.clone(),
        };
        let choosing = Choosing {
            instructions: if testing.is_some() {
                HYPOTHESIS
            } else {
                INSTRUCTIONS
            },
            deciding: &deciding,
            candidates: &candidates,
            ranked: &ranked,
            state: &state,
            knowledge: self.knowledge,
            covered: &self.run.covered,
        };
        let (index, chosen) = match routed {
            Some(index) => named_route(index),
            None => {
                let (by, model) = self.asking(testing.is_some());
                let (supervisor, started, seconds, of) = (
                    &mut *self.supervisor,
                    self.started,
                    self.plan.seconds,
                    self.plan.steps,
                );
                let heartbeat = (supervisor.heartbeat().as_millis() as u64).max(1);
                let mut asked: Option<Instant> = None;
                let mut beats = 0;
                choose(way, &choosing, &mut || {
                    // A model's call starts (Jev, the rules and a named
                    // route ask no stop): said once, and its wait counted
                    // from here.
                    let since = match asked {
                        Some(since) => since,
                        None => {
                            supervisor.progress(&Progress::Deciding {
                                step,
                                of,
                                by: by.clone(),
                                seconds: started.elapsed().as_secs(),
                            });
                            *asked.insert(Instant::now())
                        }
                    };
                    // Still waiting: said at most every heartbeat.
                    let waited = since.elapsed().as_millis() as u64;
                    if waited / heartbeat > beats {
                        beats = waited / heartbeat;
                        supervisor.progress(&Progress::Waiting {
                            step,
                            of,
                            by: model.clone(),
                            waited: waited / 1000,
                        });
                    }
                    halted(supervisor, started, seconds)
                })
            }
        };
        // Added: what was measured of this step before (an observation)
        // stays in it.
        self.pending.add(measured.timing.into_inner());
        self.run.actions += 1;
        self.run.latencies.push(chosen.decision.millis);
        self.run.usd += chosen.counted;
        if chosen.decision.usd.is_none() {
            self.run.unpriced += 1;
        }
        let candidate = candidates[index].clone();
        let mut action = candidate.action.clone();
        let mut key = candidate.key.clone();
        if let Some(text) = &chosen.input {
            action["text"] = json!(text);
            // A request for the Conversation's composer, an input for any
            // other field.
            let names = existing_names(&self.now);
            let class = if candidate.area == "conversation" {
                Request::of(text, &names)
            } else {
                Input::of(text, names.first().map(String::as_str)).name()
            };
            key = format!(
                "{}|{class}",
                key.rsplit_once('|').map(|(k, _)| k).unwrap_or(&key),
            );
        }
        let before = self.knowledge.count(&key);
        let here = self.run.covered.get(&key).copied().unwrap_or(0);
        let why = summary(before, here, &chosen);
        let step = Step {
            action,
            key,
            screen: screen_of(&self.now),
            label: candidate.label.clone(),
            expect: chosen.expect.clone(),
            by: if testing.is_some() {
                "hypothesis"
            } else {
                "explorer"
            }
            .into(),
        };
        // A stop, or the time budget, that came while the decision was made
        // takes no more action; the decision is recorded (what it cost
        // counts).
        if halted(self.supervisor, self.started, self.plan.seconds) {
            let why = self.why_halted();
            self.record(&step, Some(chosen), "not taken", why.clone(), None);
            return Err(why);
        }
        if testing.is_some() {
            self.spent_on += 1;
            if step.expect.is_some() {
                self.expecting = Some(self.hypothesis);
            }
        }
        self.take(step, Some(chosen), &why)
    }

    /// The hypothesis the next step tests, if any is left and a model
    /// decides (the rules state no expectation).
    fn testing(&self) -> Option<Hypothesis> {
        if matches!(self.plan.way, Way::Rules | Way::Cancel) {
            return None;
        }
        self.plan.hypotheses.get(self.hypothesis).cloned()
    }

    /// A hypothesis's share of the steps: the plan's divided among them and
    /// what is not covered after, three at least.
    fn share(&self) -> u32 {
        (self.plan.steps / (self.plan.hypotheses.len() as u32 + 1)).max(3)
    }

    /// Answers the hypothesis being tested (E3): by the check of the
    /// expectation stated for it (it agrees, or the expectation that failed
    /// is its finding), or not answered once its share of steps is spent
    /// without one.
    fn settle(&mut self) {
        let checked = self.checked.take();
        if let (Some(i), Some(failed)) = (self.expecting, checked) {
            self.expecting = None;
            let verdict = match failed {
                None => Verdict::Agrees,
                Some(identity) => {
                    let h = &self.plan.hypotheses[i];
                    if let Some(finding) = self
                        .run
                        .findings
                        .iter_mut()
                        .find(|f| f.identity == identity && f.hypothesis.is_none())
                    {
                        finding.hypothesis = Some(h.claim.clone());
                        finding.requirement = h.requirement.clone();
                    }
                    Verdict::Contradicted { finding: identity }
                }
            };
            self.answer(i, verdict);
        } else if self.expecting.is_none()
            && self.testing().is_some()
            && self.spent_on >= self.share()
        {
            let share = self.share();
            self.answer(
                self.hypothesis,
                Verdict::NotAnswered {
                    why: format!("no expectation was checked within its {share} steps"),
                },
            );
        }
    }

    /// Records how hypothesis `i` was answered; the next is tested.
    fn answer(&mut self, i: usize, verdict: Verdict) {
        let h = &self.plan.hypotheses[i];
        self.run.answers.push(Answer {
            claim: h.claim.clone(),
            requirement: h.requirement.clone(),
            verdict,
        });
        self.hypothesis = i + 1;
        self.spent_on = 0;
        self.expecting = None;
    }

    /// Acts, checks what the action did, and records it.
    fn take(&mut self, step: Step, chosen: Option<Chosen>, why: &str) -> Result<(), String> {
        let mut retried = false;
        let mut held = 0;
        loop {
            let act = Act {
                agent: AGENT,
                goal: &self.plan.goal,
                why,
                observed: self.now["screenRevision"].as_u64().unwrap_or_default(),
                action: &step.action,
            };
            let acting = Instant::now();
            let answered = self.instance.act(&act);
            self.pending.act_ms += acting.elapsed().as_millis() as u64;
            let answer = match answered {
                Ok(answer) => answer,
                Err(error) => {
                    self.run.unwanted.ended += 1;
                    self.record(&step, chosen, "ended", error.clone(), None);
                    self.since.push(step);
                    return self.ended(&error);
                }
            };
            let error = answer["error"].as_str().unwrap_or_default().to_string();
            let refusal = observed::refusal(&answer);
            match refusal {
                Refusal::Rule => {
                    // The Operator's own, by rule: never a finding, never
                    // again.
                    self.run.unwanted.refused += 1;
                    self.avoid.insert(step.key.clone());
                    self.record(&step, chosen, "refused", error, None);
                    return Ok(());
                }
                Refusal::Held => {
                    // Another agent holds the window: waited for a little,
                    // never a finding.
                    if held < findings::HELD_TRIES
                        && !halted(self.supervisor, self.started, self.plan.seconds)
                    {
                        held += 1;
                        std::thread::sleep(findings::HELD_WAIT);
                        if let Some(now) = self.observe()? {
                            self.now = now;
                        }
                        continue;
                    }
                    self.record(&step, chosen, "held", error, None);
                    return Ok(());
                }
                Refusal::Stopped => {
                    // The Operator pressed Stop in that window.
                    self.record(&step, chosen, "stopped", error, None);
                    return Err("stopped".into());
                }
                Refusal::Invalid => {
                    // The explorer's own malformed action: unwanted, never
                    // the Studio's finding, never again.
                    self.run.unwanted.invalid += 1;
                    self.avoid.insert(step.key.clone());
                    self.record(&step, chosen, "invalid", error, None);
                    return Ok(());
                }
                _ => {}
            }
            if matches!(refusal, Refusal::Stale | Refusal::Gone) {
                let Some(after) = self.observe()? else {
                    return Ok(());
                };
                let failed = findings::failures(&Outcome {
                    before: &self.now,
                    step: Some(&step),
                    answer: Some(&answer),
                    after: &after,
                });
                if failed.iter().any(|f| f.check == Check::OfferedActs) {
                    let mut steps = self.since.clone();
                    steps.push(step.clone());
                    self.report(failed, &steps);
                    // Found once; it would be refused again.
                    self.avoid.insert(step.key.clone());
                    self.record(&step, chosen, "failed", error, None);
                    self.now = after;
                    return Ok(());
                }
                // Stale: observed again, and tried once more if it is still
                // valid there.
                self.run.unwanted.stale += 1;
                self.recover("observed again", error.clone());
                let still = candidates(&after, self.folder.as_deref())
                    .iter()
                    .any(|c| c.action == step.action);
                self.expected_dialog = after["dialog"].clone();
                self.now = after;
                if retried || !still {
                    self.record(&step, chosen, "stale", error, None);
                    return Ok(());
                }
                retried = true;
                continue;
            }
            // Carried out (or failed partway, or its handler failed, or it
            // was not carried out in time, any of which may have done part
            // of it): a step a replay takes again. A handler that failed or
            // an action out of time is a finding (`failures`), not tried
            // again in this run.
            let outcome = if answer["ok"] == false {
                "failed"
            } else {
                "ok"
            };
            if matches!(refusal, Refusal::Failed | Refusal::Timeout) {
                self.avoid.insert(step.key.clone());
            }
            if outcome == "ok" {
                self.escapes = 0;
            }
            let took = answer["tookMs"].as_u64();
            *self.run.covered.entry(step.key.clone()).or_default() += 1;
            self.since.push(step.clone());
            let Some(after) = self.observe()? else {
                self.record(&step, chosen, outcome, error, took);
                return Ok(());
            };
            // A request sent: what the explorer expected of it is checked
            // when its turn has ended, on the wait for it.
            let mut step = step;
            if observed::turn_running(&after) && step.expect.is_some() {
                self.pending_expect = step.expect.take();
                if let Some(last) = self.since.last_mut() {
                    last.expect = None;
                }
            }
            self.record(&step, chosen, outcome, error, took);
            let failed = findings::failures(&Outcome {
                before: &self.now,
                step: Some(&step),
                answer: Some(&answer),
                after: &after,
            });
            self.note_checked(&step, &failed);
            let steps = self.since.clone();
            self.report(failed, &steps);
            self.note_reply(&step, &after);
            let before = std::mem::replace(&mut self.now, after);
            self.expected_dialog = self.now["dialog"].clone();
            if answer["ok"] != false
                && !self.undo_refused
                && !observed::turn_running(&self.now)
                // Undo waits for an open dialog (a rename after an insert).
                && self.now["dialog"].is_null()
                && findings::changed_model(&before, &self.now)
                && self.undo_checked.insert(step.key.clone())
            {
                self.check_undo(&step, &before)?;
            }
            return Ok(());
        }
    }

    /// The undo check after `step` changed the model.
    fn check_undo(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        let after = self.now.clone();
        match findings::undo_restores(self.instance, step, before, &after, &self.plan.goal) {
            Ok(undone) => {
                // Found after the action that changed the model: a replay
                // takes the steps to it and makes the check again.
                let steps = self.since.clone();
                self.report(undone.failed.into_iter().collect(), &steps);
                for taken in undone.steps {
                    self.record(&taken, None, "ok", String::new(), None);
                    self.since.push(taken);
                }
                self.seen(&undone.now);
                self.expected_dialog = undone.now["dialog"].clone();
                self.now = undone.now;
                Ok(())
            }
            Err(findings::Unchecked::Ended { error, steps }) => {
                // The instance ended on undo or redo: found with that step.
                self.run.unwanted.ended += 1;
                for taken in steps {
                    self.record(&taken, None, "ended", error.clone(), None);
                    self.since.push(taken);
                }
                self.ended(&error)
            }
            Err(findings::Unchecked::Refused { reason, by_rule }) => {
                // No finding. Refused as the Operator's, it is not tried
                // again in this run. Whether it did anything is unknown, so
                // the state is observed again.
                self.undo_refused |= by_rule;
                let note = format!("undo could not be checked: {reason}");
                if !self.run.notes.contains(&note) {
                    self.run.notes.push(note);
                }
                if let Some(now) = self.observe()? {
                    self.expected_dialog = now["dialog"].clone();
                    self.now = now;
                }
                Ok(())
            }
        }
    }

    /// Waits for the running turn to end, observing only the
    /// Conversation's state, within the plan's turn budget. A turn past its
    /// budget is a finding, and Stop must then end it within the stop
    /// budget; one that will not stop leaves a fresh start.
    fn await_turn(&mut self) -> Result<(), String> {
        let ended = self.await_and_check("the turn", self.plan.turn_ms)?;
        if ended != Some(false) {
            return Ok(());
        }
        let stop = self.now["controls"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| {
                c["region"] == "conversation" && c["id"] == observed::STOP && c["enabled"] != false
            });
        if stop {
            let screen = screen_of(&self.now);
            let step = Step {
                action: json!({ "kind": "click", "control": observed::STOP }),
                key: format!("{screen}|conversation|{}|click", observed::STOP),
                screen,
                label: "Stop".into(),
                expect: None,
                by: "check".into(),
            };
            if self.act_and_check(step, "stop a turn past its budget")? == Some(true)
                && self.await_and_check("the stop", self.plan.stop_ms)? != Some(false)
            {
                return Ok(());
            }
        }
        self.recover("restarted", "a turn of the Assistant would not end".into());
        self.start(true)
    }

    /// Waits for a turn to end (`label`: `the turn`, or `the stop` after
    /// Stop) as a step a replay waits for too, and checks it: whether it
    /// ended in time, `None` when the instance ended (and was started
    /// again).
    fn await_and_check(&mut self, label: &str, budget_ms: u64) -> Result<Option<bool>, String> {
        let screen = screen_of(&self.now);
        let step = Step {
            action: json!({ "kind": "await-turn", "timeoutMs": budget_ms }),
            key: format!("{screen}|conversation|turn|await"),
            screen,
            label: label.into(),
            expect: if label == "the turn" {
                self.pending_expect.take()
            } else {
                None
            },
            by: "wait".into(),
        };
        let (supervisor, started, seconds) =
            (&mut *self.supervisor, self.started, self.plan.seconds);
        let waiting = Instant::now();
        let waited = findings::await_turn(self.instance, budget_ms, &mut || {
            halted(supervisor, started, seconds)
        });
        self.pending.act_ms += waiting.elapsed().as_millis() as u64;
        match waited {
            findings::Waited::Stopped => Err(self.why_halted()),
            findings::Waited::Gone(error) => {
                self.record(&step, None, "ended", error.clone(), None);
                self.since.push(step);
                self.ended(&error)?;
                Ok(None)
            }
            findings::Waited::Done { answer, after } => {
                let ok = answer["ok"] != false;
                let error = answer["error"].as_str().unwrap_or_default().to_string();
                let took = answer["tookMs"].as_u64();
                self.record(&step, None, if ok { "ok" } else { "failed" }, error, took);
                self.since.push(step.clone());
                self.seen(&after);
                let failed = findings::failures(&Outcome {
                    before: &self.now,
                    step: Some(&step),
                    answer: Some(&answer),
                    after: &after,
                });
                self.note_checked(&step, &failed);
                let steps = self.since.clone();
                self.report(failed, &steps);
                self.note_reply(&step, &after);
                self.expected_dialog = after["dialog"].clone();
                self.now = after;
                Ok(Some(ok))
            }
        }
    }

    /// Takes a step the explorer did not choose (Stop) and checks what it
    /// did: whether it was carried out, `None` when the instance ended (and
    /// was started again).
    fn act_and_check(&mut self, step: Step, why: &str) -> Result<Option<bool>, String> {
        let act = Act {
            agent: AGENT,
            goal: &self.plan.goal,
            why,
            observed: self.now["screenRevision"].as_u64().unwrap_or_default(),
            action: &step.action,
        };
        let acting = Instant::now();
        let answered = self.instance.act(&act);
        self.pending.act_ms += acting.elapsed().as_millis() as u64;
        let answer = match answered {
            Ok(answer) => answer,
            Err(error) => {
                self.record(&step, None, "ended", error.clone(), None);
                self.since.push(step);
                self.ended(&error)?;
                return Ok(None);
            }
        };
        let ok = answer["ok"] != false;
        let error = answer["error"].as_str().unwrap_or_default().to_string();
        self.record(&step, None, if ok { "ok" } else { "failed" }, error, None);
        self.since.push(step.clone());
        let Some(after) = self.observe()? else {
            return Ok(None);
        };
        let failed = findings::failures(&Outcome {
            before: &self.now,
            step: Some(&step),
            answer: Some(&answer),
            after: &after,
        });
        let steps = self.since.clone();
        self.report(failed, &steps);
        self.expected_dialog = after["dialog"].clone();
        self.now = after;
        Ok(Some(ok))
    }

    /// The result of checking what `step` was expected to show, when it had
    /// an expectation: the identity of the expectation that failed, or none.
    fn note_checked(&mut self, step: &Step, failed: &[Failed]) {
        if step.expect.is_some() {
            self.checked = Some(
                failed
                    .iter()
                    .find(|f| f.check == Check::Expectation)
                    .map(Failed::identity),
            );
        }
    }

    /// What the explorer expected the Assistant's reply to say, checked
    /// after `step`: recorded, never a finding (a reply varies between
    /// runs).
    fn note_reply(&mut self, step: &Step, after: &Value) {
        if let Some(Err(problem)) = step
            .expect
            .as_ref()
            .and_then(|e| findings::reply_holds(after, e))
        {
            self.run.notes.push(format!(
                "after {}: {problem} (a reply varies between runs: recorded, not a finding)",
                describe(&step.action)
            ));
        }
    }

    fn record(
        &mut self,
        step: &Step,
        chosen: Option<Chosen>,
        outcome: &str,
        detail: String,
        took_ms: Option<u64>,
    ) {
        self.run.steps.push(Taken {
            step: step.clone(),
            chosen,
            outcome: outcome.into(),
            detail,
            took_ms,
            timing: std::mem::take(&mut self.pending),
        });
        let taken = self.run.steps.last().expect("just recorded");
        self.supervisor.progress(&Progress::Took {
            step: self.run.actions,
            of: self.plan.steps,
            taken,
            seconds: self.started.elapsed().as_secs(),
        });
    }

    /// A dialog in the way: cancelled by rule (an action of the run's), or
    /// a fresh start when it cannot be (an approval, or no Cancel).
    fn cancel_dialog(&mut self) -> Result<(), String> {
        let dialog = self.now["dialog"].as_str().unwrap_or_default().to_string();
        let decision = decide::Situation::from_observation(&self.plan.goal, &self.now)
            .map(|situation| decide::cancel(&situation));
        match decision {
            Some(decision) if decision.choice != decide::WAIT => {
                self.recover("cancelled", format!("the {dialog} dialog was in the way"));
                self.run.actions += 1;
                let step = Step {
                    action: json!({ "kind": "click", "control": decision.choice }),
                    key: format!("{}|dialog|{}|click", screen_of(&self.now), decision.choice),
                    screen: screen_of(&self.now),
                    label: "Cancel".into(),
                    expect: None,
                    by: "recovery".into(),
                };
                let why = format!("a {dialog} dialog is in the way: cancelled by rule");
                // Its dialog is the one being cancelled, so a dialog left
                // open after it is still in the way.
                self.expected_dialog = Value::Null;
                self.take(step, None, &why)?;
                if self.now["dialog"].as_str() == Some(dialog.as_str()) {
                    self.recover("restarted", format!("the {dialog} dialog did not close"));
                    self.start(true)?;
                }
                Ok(())
            }
            _ => {
                self.recover(
                    "restarted",
                    format!("the {dialog} dialog cannot be cancelled by an agent"),
                );
                self.start(true)
            }
        }
    }

    /// Nothing valid to do: Escape (an action of the run's), and after
    /// [`ESCAPES`] of them with nothing carried out between, a fresh start.
    fn dead_end(&mut self) -> Result<(), String> {
        if self.escapes >= ESCAPES || !self.now["approval"].is_null() {
            self.recover("restarted", "no valid action".into());
            return self.start(true);
        }
        self.escapes += 1;
        self.run.actions += 1;
        self.recover("escape", "no valid action".into());
        let step = Step {
            action: json!({ "kind": "key", "keys": "escape" }),
            key: format!("{}|keys|escape|key", screen_of(&self.now)),
            screen: screen_of(&self.now),
            label: "Escape".into(),
            expect: None,
            by: "recovery".into(),
        };
        let escapes = self.escapes;
        self.take(step, None, "no valid action: Escape")?;
        // Escape "carried out" is no progress out of a dead end.
        self.escapes = escapes;
        Ok(())
    }

    /// Who a step's model call asks, as its progress says it, and the
    /// model waited for: the escalation role's for a step testing a
    /// hypothesis (an engineering question, E3), else the explorer's (only
    /// a model call says so).
    fn asking(&self, testing: bool) -> (String, String) {
        let (model, effort) = self.deciding.asked(testing);
        let model = match effort {
            Some(effort) => format!("{} at {effort}", model.model),
            None => model.model.clone(),
        };
        match self.plan.way {
            _ if testing => (format!("{model} for a hypothesis"), model),
            Way::Escalating => (format!("{model}, Jev being unsure"), model),
            _ => (model.clone(), model),
        }
    }

    /// The state a typed decision reads.
    fn state(&self) -> Value {
        let now = &self.now;
        let last: Vec<String> = self
            .run
            .steps
            .iter()
            .rev()
            .take(4)
            .map(|t| {
                format!(
                    "{} on {} ({})",
                    describe(&t.step.action),
                    t.step.screen,
                    t.outcome
                )
            })
            .collect();
        json!({
            "goal": self.plan.goal,
            "screen": now["screen"],
            "view": now["view"],
            "dialog": now["dialog"],
            "panels": now["panels"],
            "selection": now["selection"],
            "status": now["status"].as_str().unwrap_or_default().chars().take(200).collect::<String>(),
            "conversation": {
                "running": observed::turn_running(now),
                "lastReply": observed::last_reply(now).chars().take(300).collect::<String>(),
            },
            "lastActions": last,
            "judgedWrong": self.judged_wrong(),
        })
    }

    /// Expectations the lead judged wrong before (C-55, from the testing
    /// knowledge), so the explorer does not state them again (E3).
    fn judged_wrong(&self) -> Vec<String> {
        self.knowledge
            .findings
            .iter()
            .rev()
            .filter(|f| {
                f.check == Check::Expectation
                    && f.disposition
                        .as_ref()
                        .is_some_and(|d| d.kind == findings::DispositionKind::WrongExpectation)
            })
            .map(|f| f.message.chars().take(200).collect())
            .take(8)
            .collect()
    }
}

/// An action in words.
fn describe(action: &Value) -> String {
    match action["kind"].as_str().unwrap_or_default() {
        "click" => format!("click {}", action["control"].as_str().unwrap_or_default()),
        "fill" => format!(
            "type “{}” into {}",
            action["text"]
                .as_str()
                .unwrap_or_default()
                .chars()
                .take(24)
                .collect::<String>(),
            action["control"].as_str().unwrap_or_default()
        ),
        "command" => format!("command {}", action["id"].as_str().unwrap_or_default()),
        "key" => format!("press {}", action["keys"].as_str().unwrap_or_default()),
        other => other.to_string(),
    }
}

/// The one-line reason observer mode shows: coverage, then who decided.
fn summary(before: u64, here: u32, chosen: &Chosen) -> String {
    let coverage = match (before, here) {
        (0, 0) => "not covered".to_string(),
        (_, 0) => "not covered in this run".to_string(),
        (_, n) => format!("covered {n}× in this run"),
    };
    let d = &chosen.decision;
    let who = match d.source {
        Source::Rules => "rules".to_string(),
        Source::Jev => format!("Jev {:.2}", d.confidence.unwrap_or(0.0)),
        Source::Model | Source::Escalated => {
            let who = if d.source == Source::Model {
                "model"
            } else {
                "escalated"
            };
            if chosen.why.is_empty() {
                who.to_string()
            } else {
                format!("{who}: {}", chosen.why)
            }
        }
    };
    format!("{coverage} · {who}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> Value {
        json!({
            "screen": "surface", "screenRevision": 3, "dialog": null, "approval": null, "palette": null,
            "panels": { "left": "library", "right": "history" },
            "selection": ["Shop::Store"], "status": "",
            "cards": [{ "element": "Shop::'Cache'", "category": "part" }],
            "controls": [
                { "id": "Message", "label": "Message", "role": "field", "region": "conversation", "operatorOnly": true },
                { "id": "History", "label": "History", "role": "tab", "region": "inspector", "selected": true },
                { "id": "checkpoint", "label": "Checkpoint…", "role": "button", "region": "inspector" },
                { "id": "show-changes", "label": "Show changes", "role": "button", "region": "inspector", "enabled": false },
                { "id": "far", "label": "Far away", "role": "button", "region": "inspector", "hidden": true },
                { "id": "objective-start", "label": "Start", "role": "button", "region": "inspector" },
                { "id": "agents-pause", "label": "Pause", "role": "button", "region": "title" },
                { "id": "window-close", "label": "Close", "role": "button", "region": "title" },
                { "id": "lock-it", "label": "Lock", "role": "button", "region": "title", "operatorOnly": true },
                { "id": "Library search", "label": "Library search", "role": "field", "region": "left-body", "focused": true },
                { "id": "create", "label": "Create part", "role": "dialog", "region": "dialog" }
            ],
            "commands": [
                { "id": "graph", "label": "Graph view" },
                { "id": "rename", "label": "Rename", "available": false },
                { "id": "undo", "label": "Undo" },
                { "id": "fit", "label": "Fit", "operatorOnly": true }
            ]
        })
    }

    /// The review of the W13.7 repair: an escalated step says why it was
    /// escalated (Jev unsure, or Jev failed) and the model it went to.
    #[test]
    fn an_escalated_step_says_why_and_to_which_model() {
        let chosen = |note: &str| Chosen {
            input: None,
            expect: None,
            why: String::new(),
            decision: Decision {
                choice: "a01".into(),
                source: Source::Escalated,
                confidence: None,
                millis: 0,
                usd: Some(0.0),
                note: note.into(),
            },
            counted: 0.0,
        };
        let timing = Timing {
            model: Some("deepseek/deepseek-flash".into()),
            effort: Some("low".into()),
            calls: 1,
            ..Timing::default()
        };
        assert_eq!(
            decided_by(&chosen("Jev chose a02 with confidence 0.40"), &timing),
            "Jev unsure, then deepseek/deepseek-flash at low"
        );
        assert_eq!(
            decided_by(
                &chosen(&format!("{JEV_FAILED}its deadline passed")),
                &timing
            ),
            "Jev failed, then deepseek/deepseek-flash at low"
        );
    }

    /// The review of the W13.7 repair: only a panel's tab or a view's
    /// command is a route the rules take for a goal; never an option (the
    /// Inspector's type matches change the model, the palette's rows run
    /// commands), a dialog's tab, nor a label the goal names without
    /// saying it is a view, a panel or a tab.
    #[test]
    fn only_a_panels_tab_or_a_views_command_is_a_route() {
        let screen = |controls: Value| {
            json!({
                "screen": "surface", "screenRevision": 1, "dialog": null, "approval": null,
                "palette": null, "panels": {}, "selection": [], "status": "",
                "controls": controls, "commands": [{ "id": "requirements-view", "label": "Requirements view" }]
            })
        };
        let goal = "Open the History panel and read its buttons";
        let none = BTreeMap::new();
        let routed = |observation: &Value, goal: &str| {
            let list = candidates(observation, None);
            route(&list, goal, &none).map(|i| list[i].key.clone())
        };
        // The Inspector's type match, a palette row, a dialog's tab.
        let not_routes = screen(json!([
            { "id": "type-History", "label": "History", "role": "option", "region": "inspector" },
            { "id": "palette-History", "label": "History", "role": "option", "region": "palette" },
            { "id": "dialog-History", "label": "History", "role": "tab", "region": "dialog" }
        ]));
        assert_eq!(routed(&not_routes, goal), None);
        // The panel's tab is one; not for a goal that only says "history".
        let tab = screen(json!([
            { "id": "History", "label": "History", "role": "tab", "region": "inspector" }
        ]));
        assert_eq!(
            routed(&tab, goal).as_deref(),
            Some("surface|inspector|History|click")
        );
        assert_eq!(routed(&tab, "Look through the history of changes"), None);
        // A view's command, by its name.
        assert_eq!(
            routed(&tab, "Count the rows in the Requirements view").as_deref(),
            Some("surface|command|requirements-view|command")
        );
    }

    #[test]
    fn candidates_are_what_an_agent_may_do_on_the_screen() {
        let list = candidates(&observation(), None);
        let keys: Vec<&str> = list.iter().map(|c| c.key.as_str()).collect();
        assert!(keys.contains(&"surface|inspector|History|click"));
        assert!(keys.contains(&"surface|inspector|checkpoint|click"));
        assert!(keys.contains(&"surface|command|graph|command"));
        assert!(keys.contains(&"surface|keys|escape|key"));
        assert!(
            keys.contains(&"surface|left-body|enter|key"),
            "Enter in the focused field"
        );
        // Every input class for the field, the duplicate named after a card.
        let fills: Vec<&Candidate> = list.iter().filter(|c| c.field).collect();
        assert_eq!(fills.len(), Input::ALL.len());
        assert!(
            fills
                .iter()
                .any(|c| c.key.ends_with("|duplicate") && c.action["text"] == "Cache")
        );
        // Disabled, hidden, the Operator's own, the window's, unavailable,
        // and what is not interactive are not offered.
        for left_out in [
            "Message",
            "show-changes",
            "far",
            "objective-start",
            "agents-pause",
            "window-close",
            "lock-it",
            "create",
        ] {
            assert!(
                !list.iter().any(|c| c.action["control"] == left_out),
                "{left_out}"
            );
        }
        for left_out in ["rename", "undo", "fit"] {
            assert!(
                !list.iter().any(|c| c.action["id"] == left_out),
                "{left_out}"
            );
        }
    }

    #[test]
    fn without_marks_the_operators_own_is_known_by_its_place() {
        // A Studio that marks nothing: Settings, the Conversation and the
        // Operator's commands are left out by where they are.
        let mut o = observation();
        for list in ["controls", "commands"] {
            for c in o[list].as_array_mut().unwrap() {
                c.as_object_mut().unwrap().remove("operatorOnly");
            }
        }
        o["commands"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "id": "theme", "label": "Switch light or dark theme" }));
        let list = candidates(&o, None);
        assert!(!list.iter().any(|c| c.action["control"] == "Message"));
        assert!(!list.iter().any(|c| c.action["id"] == "theme"));
        // Marked: the marks decide, the Conversation's composer included.
        o["controls"][0]["operatorOnly"] = json!(false);
        let list = candidates(&o, None);
        assert!(list.iter().any(|c| c.action["control"] == "Message"));
        // Undo and redo stay the undo check's.
        assert!(!list.iter().any(|c| c.action["id"] == "undo"));
    }

    #[test]
    fn the_composer_takes_requests_and_sending_names_the_request() {
        let mut o = observation();
        o["cards"] = json!([{ "element": "Shop::'Cache'" }, { "element": "Shop::Store" }]);
        o["controls"] = json!([
            { "id": "Message", "label": "Message", "role": "field", "region": "conversation", "value": "Add a part named Probe to the model.", "focused": true, "operatorOnly": false },
            { "id": "send", "label": "Send", "role": "button", "region": "conversation", "operatorOnly": false },
            { "id": "History", "label": "History", "role": "tab", "region": "inspector" }
        ]);
        let list = candidates(&o, None);
        let fills: Vec<&str> = list
            .iter()
            .filter(|c| c.action["control"] == "Message")
            .map(|c| c.key.as_str())
            .collect();
        assert_eq!(fills.len(), Request::ALL.len());
        assert!(fills.contains(&"surface|conversation|Message|fill|add-part"));
        let connect = list
            .iter()
            .find(|c| c.key.ends_with("|fill|connect"))
            .unwrap();
        assert_eq!(
            connect.action["text"], "Connect Cache to Store.",
            "two names in view"
        );
        let keys: Vec<&str> = list.iter().map(|c| c.key.as_str()).collect();
        assert!(keys.contains(&"surface|conversation|send|click|Message=add-part"));
        assert!(keys.contains(&"surface|conversation|enter|key|Message=add-part"));
        assert_eq!(Request::of("Tell me a joke", &[]), "written");
        assert_eq!(Request::of("   ", &[]), "whitespace");
    }

    #[test]
    fn a_dialog_limits_what_is_reachable_and_a_path_outside_is_never_confirmed() {
        let mut o = observation();
        o["dialog"] = json!("NewProject");
        o["controls"] = json!([
            { "id": "checkpoint", "label": "Checkpoint…", "role": "button", "region": "inspector" },
            { "id": "Project folder", "label": "Project folder", "role": "field", "region": "dialog", "value": "C:\\Users\\someone\\Agentique\\NewSystem", "focused": true },
            { "id": "dialog-cancel", "label": "Cancel", "role": "button", "region": "dialog" },
            { "id": "dialog-confirm", "label": "Create project", "role": "button", "region": "dialog" }
        ]);
        let folder = Path::new("C:\\explore\\run");
        let list = candidates(&o, Some(folder));
        assert!(
            list.iter()
                .all(|c| c.area.starts_with("dialog:") || c.area == "keys")
        );
        assert!(!list.iter().any(|c| c.action["control"] == "dialog-confirm"));
        assert!(!list.iter().any(|c| c.action["keys"] == "enter"));
        assert!(list.iter().any(|c| c.action["control"] == "dialog-cancel"));
        // Inside the instance's folder, it may be confirmed, whichever
        // separator the path uses.
        o["controls"][1]["value"] = json!("C:/explore/run/start-1/Shop");
        let list = candidates(&o, Some(folder));
        assert!(list.iter().any(|c| c.action["control"] == "dialog-confirm"));
        // Windows paths ignore case; other hosts' do not.
        o["controls"][1]["value"] = json!("c:/explore/run/start-1/Shop");
        let list = candidates(&o, Some(folder));
        assert_eq!(
            list.iter().any(|c| c.action["control"] == "dialog-confirm"),
            cfg!(windows)
        );
        // An approval leaves nothing; Settings only Escape.
        o["approval"] = json!("live run");
        assert!(candidates(&o, Some(folder)).is_empty());
        let mut settings = observation();
        settings["screen"] = json!("settings");
        let list = candidates(&settings, None);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].action["keys"], "escape");
    }

    #[test]
    fn a_models_text_is_classed_and_its_answer_must_be_an_option() {
        assert_eq!(Input::of("", None), Input::Empty);
        assert_eq!(Input::of("  ", None), Input::Whitespace);
        assert_eq!(Input::of("Cache", Some("Cache")), Input::Duplicate);
        assert_eq!(Input::of("part", None), Input::Keyword);
        assert_eq!(Input::of("x/y", None), Input::Path);
        assert_eq!(Input::of("Größe", None), Input::NonAscii);
        assert_eq!(Input::of("Shop", None), Input::Name);
        let list = candidates(&observation(), None);
        let field = list.iter().position(|c| c.field).unwrap();
        let button = list.iter().position(|c| !c.field).unwrap();
        let shown = vec![("a01".to_string(), button), ("a02".to_string(), field)];
        let read = |said: &str| read_answer(said, &shown, &list);
        let (choice, (input, expect, why)) = read(
            r#"{"choice": "a02", "input": "Shop", "expect": {"dialog": null}, "why": "try a name"}"#,
        )
        .unwrap();
        assert_eq!(
            (choice.as_str(), input.as_deref(), why.as_str()),
            ("a02", Some("Shop"), "try a name")
        );
        assert!(
            read(r#"{"choice": "a2"}"#).is_err(),
            "only the ids as given"
        );
        assert_eq!(expect, Some(json!({ "dialog": null })));
        assert!(read(r#"{"choice": "a03"}"#).is_err(), "not an option");
        assert!(
            read(r#"{"choice": "a01", "input": "x"}"#).is_err(),
            "text for a button"
        );
        // An empty text for a button is no input.
        let (_, (input, _, _)) = read(r#"{"choice": "a01", "input": ""}"#).unwrap();
        assert_eq!(input, None);
        assert!(
            read(r#"{"choice": "a01", "expect": {"pixels": 3}}"#).is_err(),
            "not checkable"
        );
        assert!(read("no answer").is_err());
    }

    #[test]
    fn a_path_that_leads_outside_the_instance_is_never_confirmed_or_typed() {
        let folder = Path::new("C:\\explore\\run");
        // Plain relative paths stay in the instance's working folder.
        for inside in [
            "Shop",
            "a/b\\c",
            "Probe/x",
            "",
            "C:\\explore\\run\\start-1\\Shop",
        ] {
            assert!(!escapes(inside, folder), "{inside}");
        }
        for outside in [
            "..\\..\\..\\..\\..\\Desktop\\X",
            "Shop/../../x",
            "C:\\explore\\run2\\Shop",
            "C:\\explore",
            "\\\\server\\share\\x",
            "//server/share/x",
            "\\Users\\x",
            "C:x",
            "D:\\explore\\run\\x",
        ] {
            assert!(escapes(outside, folder), "{outside}");
        }
        // The guard on a dialog's confirm and Enter.
        let mut o = observation();
        o["dialog"] = json!("OpenProject");
        o["controls"] = json!([
            { "id": "Project folder", "label": "Project folder", "role": "field", "region": "dialog", "value": "..\\..\\Desktop\\X", "focused": true },
            { "id": "dialog-confirm", "label": "Open", "role": "button", "region": "dialog" }
        ]);
        let list = candidates(&o, Some(folder));
        assert!(!list.iter().any(|c| c.action["control"] == "dialog-confirm"));
        assert!(!list.iter().any(|c| c.action["keys"] == "enter"));
        // A model may not type one into a dialog's field.
        let field = list.iter().position(|c| c.field).unwrap();
        let shown = vec![("a01".to_string(), field)];
        for text in ["..\\..\\Desktop\\X", "\\\\server\\share", "C:\\x", "/etc"] {
            let said = json!({ "choice": "a01", "input": text }).to_string();
            assert!(read_answer(&said, &shown, &list).is_err(), "{text}");
        }
    }

    #[test]
    fn an_answer_may_take_as_long_as_its_typing_and_its_wait() {
        let click = json!({ "kind": "click", "control": "x" });
        assert_eq!(answer_time(&click), Duration::from_millis(2000) + ANSWER);
        // A long request typed at a keystroke's budget a character is not a
        // hang.
        let long = json!({ "kind": "fill", "control": "Message", "text": "x".repeat(2500) });
        assert!(answer_time(&long) >= Duration::from_secs(250));
        let wait = json!({ "kind": "wait", "timeoutMs": 60000 });
        assert!(answer_time(&wait) >= Duration::from_secs(90));
    }

    /// The W13.7 repair: a copy takes a model folder's files but its hidden
    /// ones and lock, a project folder's model folder when it has one, and
    /// its digest is its source's; other files, or the same files elsewhere,
    /// are told apart.
    #[test]
    fn a_copy_has_its_sources_digest_and_another_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("repo").join("model");
        std::fs::create_dir_all(model.join("parts")).unwrap();
        std::fs::write(model.join("A.sysml"), "package A;\n").unwrap();
        std::fs::write(model.join("parts").join("B.sysml"), "package B;\n").unwrap();
        std::fs::write(model.join(".gitignore"), "*.tmp\n").unwrap();
        std::fs::write(model.join("agentique.lock"), "").unwrap();
        let source = Provenance::of(&dir.path().join("repo"), "model", "abc").unwrap();
        assert!(same_folder(&source.folder, &model));
        let copy = dir.path().join("copy").join("model");
        let copied = copy_model(&model, &copy).unwrap();
        assert_eq!(copied, source.digest);
        assert!(copy.join("parts").join("B.sysml").is_file());
        assert!(!copy.join(".gitignore").exists() && !copy.join("agentique.lock").exists());
        // A project folder: its model folder is what is copied.
        let project = dir.path().join("copy");
        assert!(same_folder(&model_folder(&project), &copy));
        assert_eq!(model_digest(&model_folder(&project)).unwrap(), copied);
        // One byte, or one name, more: another digest.
        std::fs::write(copy.join("A.sysml"), "package A; \n").unwrap();
        assert_ne!(model_digest(&copy).unwrap(), source.digest);
        std::fs::write(copy.join("A.sysml"), "package A;\n").unwrap();
        std::fs::rename(copy.join("parts"), copy.join("other")).unwrap();
        assert_ne!(model_digest(&copy).unwrap(), source.digest);
        // A folder without model files is no project.
        assert!(Provenance::of(dir.path(), "nothing", "abc").is_err());
    }

    #[test]
    fn changed_files_point_to_the_studios_areas() {
        let changes = Changes {
            paths: vec![
                "crates/studio-native/src/panels/objectives.rs".into(),
                "crates\\studio-native\\src\\library\\index.rs".into(),
                "crates/orchestrator/src/run.rs".into(),
            ],
            subjects: vec![],
        };
        let areas: Vec<String> = changes.areas().into_iter().collect();
        assert_eq!(areas, vec!["library".to_string(), "objectives".to_string()]);
    }
}
