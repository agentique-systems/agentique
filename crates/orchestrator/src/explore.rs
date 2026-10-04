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
//! them across runs. [`Instance`] is the one place tests replace the GUI.

use crate::control::{Client, TestInstance};
use crate::decide::{self, Answers, Decision, Failure, Question, Source, Way};
use crate::findings::{self, Check, Failed, Finding, Outcome};
use crate::knowledge::Knowledge;
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
    /// A fresh start of the same build in the same start state.
    fn restart(&mut self) -> Result<(), String>;
    /// Whether it still runs.
    fn alive(&mut self) -> bool;
    /// The folder its files belong in, if it has one: a dialog holding an
    /// absolute path outside it is never confirmed.
    fn folder(&self) -> Option<PathBuf>;
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

/// How long the explorer waits for an answer: an observation or an action
/// takes well under a second, so a Studio silent this long has stopped
/// answering.
const ANSWER: Duration = Duration::from_secs(30);

/// A test instance of a Studio executable, started fresh each time from a
/// copy of the start project, in its own folder (its data, its working
/// folder and the repository it is told about are all in there).
pub struct LiveInstance {
    exe: PathBuf,
    /// The start project: a project folder, or its model folder.
    start: PathBuf,
    folder: PathBuf,
    starts: u32,
    // The connection closes before the process ends.
    client: Option<Client>,
    instance: Option<TestInstance>,
}

impl LiveInstance {
    pub fn new(exe: &Path, start: &Path, folder: &Path) -> LiveInstance {
        LiveInstance {
            exe: exe.to_path_buf(),
            start: start.to_path_buf(),
            folder: folder.to_path_buf(),
            starts: 0,
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

/// Copies the start project's model files into `to` (a fresh model folder),
/// leaving out its repository and lock.
fn copy_model(start: &Path, to: &Path) -> Result<(), String> {
    let from = if start.join("model").is_dir() {
        start.join("model")
    } else {
        start.to_path_buf()
    };
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for entry in std::fs::read_dir(&from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let text = name.to_string_lossy();
        if text.starts_with('.') || text == "agentique.lock" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy_model(&path, &to.join(&name))?;
        } else {
            std::fs::copy(&path, to.join(&name)).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
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
        client.call(json!({
            "op": "act",
            "agent": act.agent,
            "why": act.why,
            "goal": act.goal,
            "expect": { "instance": instance },
            "observed": act.observed,
            "action": act.action,
        }))
    }

    fn restart(&mut self) -> Result<(), String> {
        self.client = None;
        self.instance = None;
        let _ = std::fs::remove_dir_all(self.folder.join(format!("start-{}", self.starts)));
        self.starts += 1;
        let base = self.folder.join(format!("start-{}", self.starts));
        let project = base.join("project");
        copy_model(&self.start, &project.join("model"))?;
        // Its repository is the project's copy, so nothing it opens or
        // writes lies outside its folder.
        let mut instance =
            TestInstance::start(&self.exe, &base.join("instance"), &project, &project)?;
        let mut client = instance.connect(Duration::from_secs(120))?;
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
    /// `explorer` (chosen), `check` (the undo check's undo and redo) or
    /// `recovery` (a dialog cancelled by rule, Escape out of a dead end).
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
}

/// The Operator's own, until every observation says so with `operatorOnly`:
/// Settings and the Conversation (regions), the Objectives panel's controls,
/// the agents' Pause, Step and Resume, and the system's folder picker, which
/// no agent can see (id prefixes).
const OPERATORS_REGIONS: [&str; 2] = ["settings", "conversation"];
const OPERATORS_PREFIXES: [&str; 3] = ["objective-", "agents-", "browse-"];
/// The Operator's commands (the Studio's `OPERATORS_COMMANDS`).
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

/// Whether `value` is an absolute path outside `folder`.
fn outside(value: &str, folder: &Path) -> bool {
    let absolute = Path::new(value).is_absolute()
        || value.starts_with(['/', '\\'])
        || value.get(1..3).is_some_and(|s| s == ":\\" || s == ":/");
    let comparable = |s: &str| s.to_lowercase().replace('/', "\\");
    absolute && !comparable(value).starts_with(&comparable(&folder.display().to_string()))
}

/// The name of an element the model has, for the duplicate input: the
/// first card in view or the selection's first (without its package).
fn existing_name(observation: &Value) -> Option<String> {
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
        )
        .find(|name| !name.is_empty())?;
    let last = qualified.rsplit("::").next().unwrap_or(qualified);
    Some(last.trim_matches('\'').to_string())
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
                .any(|c| outside(c["value"].as_str().unwrap_or_default(), folder))
        });
    let existing = existing_name(observation);
    // What a dialog's confirm (or Enter) would confirm: the input class of
    // each of its fields, so confirming other inputs is other coverage.
    let mut confirming: Vec<String> = observation["controls"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| scope == Some("dialog") && c["region"] == "dialog" && c["role"] == "field")
        .map(|c| {
            let value = c["value"].as_str().unwrap_or_default();
            format!(
                "{}={}",
                c["id"].as_str().unwrap_or_default(),
                Input::of(value, existing.as_deref()).name()
            )
        })
        .collect();
    confirming.sort();
    let confirming = confirming.join(",");
    let with_inputs = |key: String, about: String| {
        if confirming.is_empty() {
            (key, about)
        } else {
            (
                format!("{key}|{confirming}"),
                format!("{about} with {confirming}"),
            )
        }
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
            || control["operatorOnly"] == true
            || OPERATORS_REGIONS.contains(&region)
            || OPERATORS_PREFIXES.iter().any(|p| id.starts_with(p))
            || id.starts_with(WINDOW_CONTROLS)
            || id.is_empty()
            || (guarded && id == "dialog-confirm")
        {
            continue;
        }
        let area = area(observation, control);
        let label = control["label"].as_str().unwrap_or_default().to_string();
        if role == "field" {
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
                });
            }
        } else {
            let key = format!("{screen}|{region}|{id}|click");
            let about = format!("click the {role} “{label}” ({area})");
            let (key, about) = if id == "dialog-confirm" {
                with_inputs(key, about)
            } else {
                (key, about)
            };
            list.push(Candidate {
                action: json!({ "kind": "click", "control": id }),
                key,
                about,
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
                || command["operatorOnly"] == true
                || OPERATORS_COMMANDS.contains(&id)
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
            });
        }
    }
    if let Some((region, area, label)) = focused
        && !guarded
    {
        let (key, about) = with_inputs(
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
/// passes its `explorer`, `decisions` and `escalation` roles).
pub struct Deciding<'a> {
    /// Jev (its model and deadline are its own) and the models: a
    /// [`decide::Decider`], or a stand-in in tests.
    pub answers: &'a dyn Answers,
    /// The explorer's model and effort: the Model way.
    pub explorer: ModelRef,
    pub effort: Option<String>,
    /// The model Jev escalates to, and its effort.
    pub escalation: ModelRef,
    pub escalation_effort: Option<String>,
    /// Jev's confidence at or above which its answer is used.
    pub threshold: f64,
}

/// How many of the rules' best actions Jev chooses among, and the model.
const JEV_OPTIONS: usize = 8;
const MODEL_OPTIONS: usize = 24;

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
    /// model call's too).
    pub decision: Decision,
}

const INSTRUCTIONS: &str = "An explorer tests the Agentique application by operating it toward a goal, to find problems. Choose its next action among the options; each is valid on this screen and safe in this test instance. Prefer behaviour not covered before, toward the areas the goal names; do not repeat the last actions; leave a dialog or panel once its behaviour is covered.";

const FORMAT: &str = "{\"choice\": \"<one option id>\", \"why\": \"<one short line>\"}, with \"input\": \"<the text to type>\" added only when the option types into a field (it replaces the option's text), and \"expect\": {...} added only when the action must make something true in the next observation, with any of: screen, dialog (a kind, or null), statusContains, selectionContains, control (an id or label) with labelContains, valueContains or enabled, anyLabelContains";

/// The typed question for the next action: the best `n` of the rules'
/// order as options `a1`, `a2`, …
fn question(
    candidates: &[Candidate],
    ranked: &[usize],
    n: usize,
    state: &Value,
    knowledge: &Knowledge,
    covered: &BTreeMap<String, u32>,
) -> (Question, Vec<usize>) {
    let shown: Vec<usize> = ranked.iter().copied().take(n).collect();
    let mut options = BTreeMap::new();
    let mut coverage = serde_json::Map::new();
    for (i, &index) in shown.iter().enumerate() {
        let c = &candidates[index];
        let id = format!("a{}", i + 1);
        let before = knowledge.count(&c.key);
        let here = covered.get(&c.key).copied().unwrap_or(0);
        let note = match (before, here) {
            (0, 0) => "never covered".to_string(),
            (_, 0) => "covered before, not in this run".to_string(),
            (_, n) => format!("covered {n} time(s) in this run"),
        };
        options.insert(id.clone(), Some(format!("{} — {note}", c.about)));
        coverage.insert(id, json!({ "before": before, "thisRun": here }));
    }
    let mut state = state.clone();
    state["coverage"] = Value::Object(coverage);
    (
        Question {
            instructions: INSTRUCTIONS.into(),
            state,
            options,
        },
        shown,
    )
}

/// Reads the model's answer: an option, text only for a field, an
/// expectation in the grammar of observation criteria, and a reason.
type Said = (Option<String>, Option<Value>, String);

fn read_answer(
    said: &str,
    shown: &[usize],
    candidates: &[Candidate],
) -> Result<(String, Said), String> {
    let answer = decide::answer_object(said)?;
    let choice = answer["choice"].as_str().unwrap_or_default().to_string();
    let index = choice
        .strip_prefix('a')
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|n| (1..=shown.len()).contains(n))
        .ok_or_else(|| format!("the model chose `{choice}`, which is not an option"))?;
    let candidate = &candidates[shown[index - 1]];
    let input = match &answer["input"] {
        Value::Null => None,
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
            crate::control::expectation(expect).map_err(|e| format!("the expectation: {e}"))?;
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

/// The rules' choice, carrying the time and cost of what was tried first.
fn by_rule(ranked: &[usize], spent: Option<(u64, Option<f64>)>, note: String) -> (usize, Chosen) {
    let (millis, usd) = spent.unwrap_or((0, Some(0.0)));
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
                millis,
                usd,
                note,
            },
        },
    )
}

fn add(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    Some(a? + b?)
}

/// Chooses the next action by `way`: the rules' first; Jev among the
/// rules' best, used when confident; the model among more of them, its
/// answer checked; Jev escalating to the model. Whatever fails falls back
/// to the rules, with its time and cost kept.
#[allow(clippy::too_many_arguments)]
fn choose(
    way: Way,
    deciding: &Deciding,
    candidates: &[Candidate],
    ranked: &[usize],
    state: &Value,
    knowledge: &Knowledge,
    covered: &BTreeMap<String, u32>,
) -> (usize, Chosen) {
    if way == Way::Rules || way == Way::Cancel || candidates.len() == 1 {
        return by_rule(ranked, None, String::new());
    }
    let model = |model: &ModelRef,
                 effort: Option<&str>,
                 spent: (u64, Option<f64>),
                 note: String,
                 source: Source| {
        let (q, shown) = question(candidates, ranked, MODEL_OPTIONS, state, knowledge, covered);
        let read = |said: &str| read_answer(said, &shown, candidates);
        match decide::ask_model(deciding.answers, model, effort, &q, FORMAT, &read) {
            Ok((decision, (input, expect, why))) => {
                let index = decision.choice[1..].parse::<usize>().unwrap_or(1);
                (
                    shown[index - 1],
                    Chosen {
                        input,
                        expect,
                        why,
                        decision: Decision {
                            source,
                            millis: spent.0 + decision.millis,
                            usd: add(spent.1, decision.usd),
                            note: [note, decision.note]
                                .into_iter()
                                .filter(|n| !n.is_empty())
                                .collect::<Vec<_>>()
                                .join("; "),
                            ..decision
                        },
                    },
                )
            }
            Err(Failure {
                error, millis, usd, ..
            }) => by_rule(
                ranked,
                Some((spent.0 + millis, add(spent.1, usd))),
                [note, format!("the model failed: {error}")]
                    .into_iter()
                    .filter(|n| !n.is_empty())
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        }
    };
    if way == Way::Model {
        return model(
            &deciding.explorer,
            deciding.effort.as_deref(),
            (0, Some(0.0)),
            String::new(),
            Source::Model,
        );
    }
    let (q, shown) = question(candidates, ranked, JEV_OPTIONS, state, knowledge, covered);
    let (spent, note) = match deciding.answers.ask_jev(&q) {
        Ok(decision) => {
            let index = decision
                .choice
                .strip_prefix('a')
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| (1..=shown.len()).contains(n));
            match index {
                Some(index) if decision.confidence.unwrap_or(0.0) >= deciding.threshold => {
                    return (
                        shown[index - 1],
                        Chosen {
                            input: None,
                            expect: None,
                            why: String::new(),
                            decision,
                        },
                    );
                }
                _ => (
                    (decision.millis, decision.usd),
                    format!(
                        "Jev chose {} with confidence {:.2}",
                        decision.choice,
                        decision.confidence.unwrap_or(0.0)
                    ),
                ),
            }
        }
        Err(failure) => ((failure.millis, failure.usd), failure.error),
    };
    if way == Way::Escalating {
        model(
            &deciding.escalation,
            deciding.escalation_effort.as_deref(),
            spent,
            note,
            Source::Escalated,
        )
    } else {
        by_rule(ranked, Some(spent), note)
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
    /// Budgets: the explorer's actions, seconds and US dollars.
    pub steps: u32,
    pub seconds: u64,
    pub usd: f64,
    #[serde(default)]
    pub changes: Changes,
    /// The start state, as findings name it (the start project).
    pub start: String,
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
}

/// A way out of a state exploration did not choose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    /// The explorer's step it happened at.
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
}

impl Unwanted {
    pub fn total(&self) -> u32 {
        self.refused + self.stale + self.ended
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
    /// The explorer's actions (its budget counts these).
    pub chosen: u32,
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
    /// Spent on decisions (known prices), and decisions with unknown cost.
    pub usd: f64,
    pub unpriced: u32,
    pub notes: Vec<String>,
    /// Why it ended.
    pub ended: String,
    pub seconds: f64,
}

impl Run {
    /// A latency percentile of the decisions.
    pub fn latency(&self, p: f64) -> u64 {
        let mut sorted = self.latencies.clone();
        sorted.sort_unstable();
        if sorted.is_empty() {
            return 0;
        }
        sorted[((sorted.len() - 1) as f64 * p).round() as usize]
    }
}

/// How often the instance may be started again in one run before the run
/// gives up on it.
const RESTARTS: usize = 4;

/// The explorer's state during a run.
struct Explorer<'a> {
    instance: &'a mut dyn Instance,
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
    /// Keys whose change was checked with undo, and whether undo can be
    /// checked here at all.
    undo_checked: BTreeSet<String>,
    undo_unavailable: bool,
    reported: BTreeSet<String>,
    restarts: usize,
    started: Instant,
}

/// Explores `instance` as `plan` says, deciding by `plan.way` with
/// `deciding`, preferring what `knowledge` has not covered. `hold` is asked
/// before each step whether to go on (it may wait while the run is
/// paused). Starts the instance fresh, so every finding's steps begin at
/// the start.
pub fn explore(
    instance: &mut dyn Instance,
    plan: &Plan,
    deciding: &Deciding,
    knowledge: &Knowledge,
    hold: &mut dyn FnMut() -> bool,
) -> Run {
    let folder = instance.folder();
    let mut explorer = Explorer {
        instance,
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
            chosen: 0,
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
            notes: Vec::new(),
            ended: String::new(),
            seconds: 0.0,
        },
        since: Vec::new(),
        now: Value::Null,
        expected_dialog: Value::Null,
        avoid: BTreeSet::new(),
        undo_checked: BTreeSet::new(),
        undo_unavailable: false,
        reported: BTreeSet::new(),
        restarts: 0,
        started: Instant::now(),
    };
    explorer.run.ended = match explorer.go(hold) {
        Ok(()) => "the step budget was used".into(),
        Err(why) => why,
    };
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
    fn go(&mut self, hold: &mut dyn FnMut() -> bool) -> Result<(), String> {
        if way_is_for_dialogs(self.plan.way) {
            return Err("the cancelling rule decides dialogs in the way, not exploration".into());
        }
        self.start(false)?;
        while self.run.chosen < self.plan.steps {
            if !hold() {
                return Err("stopped".into());
            }
            if self.started.elapsed().as_secs() >= self.plan.seconds {
                return Err("the time budget was used".into());
            }
            if self.run.usd >= self.plan.usd {
                return Err("the spend budget was used".into());
            }
            self.step()?;
        }
        Ok(())
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
        self.instance
            .restart()
            .map_err(|e| format!("the test instance did not start: {e}"))?;
        let now = self
            .instance
            .observe()
            .map_err(|e| format!("the test instance does not answer: {e}"))?;
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
        Ok(())
    }

    /// The areas an observation shows.
    fn seen(&mut self, observation: &Value) {
        for control in observation["controls"].as_array().into_iter().flatten() {
            if control["hidden"] != true {
                self.run.areas.insert(area(observation, control));
            }
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

    /// The instance ended or stopped answering after `steps`: a finding,
    /// and a fresh start.
    fn ended(&mut self, error: &str) -> Result<(), String> {
        let message = findings::ended(self.instance);
        let control = self
            .since
            .last()
            .map(|s| s.target().to_string())
            .unwrap_or_default();
        let steps = self.since.clone();
        self.report(
            vec![Failed {
                check: Check::Answers,
                control,
                message,
                evidence: json!({ "error": error }),
            }],
            &steps,
        );
        self.recover("restarted", format!("the instance ended: {error}"));
        self.start(true)
    }

    fn recover(&mut self, kind: &str, detail: String) {
        self.run.recoveries.push(Recovery {
            at: self.run.chosen,
            kind: kind.into(),
            detail,
        });
    }

    fn observe(&mut self) -> Result<Option<Value>, String> {
        match self.instance.observe() {
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
        let mut candidates = candidates(&self.now, self.folder.as_deref());
        candidates.retain(|c| !self.avoid.contains(&c.key));
        if candidates.is_empty() {
            return self.dead_end();
        }
        let ranked = rank(
            &candidates,
            self.knowledge,
            &self.run.covered,
            &self.focus,
            self.plan.seed,
        );
        let state = self.state();
        let (index, chosen) = choose(
            self.plan.way,
            self.deciding,
            &candidates,
            &ranked,
            &state,
            self.knowledge,
            &self.run.covered,
        );
        self.run.chosen += 1;
        self.run.latencies.push(chosen.decision.millis);
        match chosen.decision.usd {
            Some(usd) => self.run.usd += usd,
            None => self.run.unpriced += 1,
        }
        let candidate = candidates[index].clone();
        let mut action = candidate.action.clone();
        let mut key = candidate.key.clone();
        if let Some(text) = &chosen.input {
            action["text"] = json!(text);
            let class = Input::of(text, existing_name(&self.now).as_deref());
            key = format!(
                "{}|{}",
                key.rsplit_once('|').map(|(k, _)| k).unwrap_or(&key),
                class.name()
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
            by: "explorer".into(),
        };
        self.take(step, Some(chosen), &why)
    }

    /// Acts, checks what the action did, and records it.
    fn take(&mut self, step: Step, chosen: Option<Chosen>, why: &str) -> Result<(), String> {
        let mut retried = false;
        loop {
            let act = Act {
                agent: AGENT,
                goal: &self.plan.goal,
                why,
                observed: self.now["screenRevision"].as_u64().unwrap_or_default(),
                action: &step.action,
            };
            let answer = match self.instance.act(&act) {
                Ok(answer) => answer,
                Err(error) => {
                    self.run.unwanted.ended += 1;
                    self.record(&step, chosen, "ended", error.clone(), None);
                    self.since.push(step);
                    return self.ended(&error);
                }
            };
            let error = answer["error"].as_str().unwrap_or_default().to_string();
            if answer["ok"] == false && error.starts_with("refused") {
                // The Operator's own, by rule: never a finding, never again.
                self.run.unwanted.refused += 1;
                self.avoid.insert(step.key.clone());
                self.record(&step, chosen, "refused", error, None);
                return Ok(());
            }
            if answer["ok"] == false
                && (error.starts_with("stale") || findings::refused_as_gone(&error))
            {
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
            // Carried out (or failed partway, which may have done part of
            // it): a step a replay takes again.
            let outcome = if answer["ok"] == false {
                "failed"
            } else {
                "ok"
            };
            let took = answer["tookMs"].as_u64();
            self.record(&step, chosen, outcome, error, took);
            *self.run.covered.entry(step.key.clone()).or_default() += 1;
            self.since.push(step.clone());
            let Some(after) = self.observe()? else {
                return Ok(());
            };
            let failed = findings::failures(&Outcome {
                before: &self.now,
                step: Some(&step),
                answer: Some(&answer),
                after: &after,
            });
            let steps = self.since.clone();
            self.report(failed, &steps);
            let before = std::mem::replace(&mut self.now, after);
            self.expected_dialog = self.now["dialog"].clone();
            if answer["ok"] != false
                && !self.undo_unavailable
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
            Err(reason) => {
                // Refused by rule or unavailable: no finding, and not tried
                // again in this run. Whether it did anything is unknown, so
                // the state is observed again.
                self.undo_unavailable = true;
                self.run
                    .notes
                    .push(format!("undo could not be checked: {reason}"));
                if let Some(now) = self.observe()? {
                    self.expected_dialog = now["dialog"].clone();
                    self.now = now;
                }
                Ok(())
            }
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
        });
    }

    /// A dialog in the way: cancelled by rule, or a fresh start when it
    /// cannot be (an approval, or no Cancel).
    fn cancel_dialog(&mut self) -> Result<(), String> {
        let dialog = self.now["dialog"].as_str().unwrap_or_default().to_string();
        let decision = decide::Situation::from_observation(&self.plan.goal, &self.now)
            .map(|situation| decide::cancel(&situation));
        match decision {
            Some(decision) if decision.choice != decide::WAIT => {
                self.recover("cancelled", format!("the {dialog} dialog was in the way"));
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

    /// Nothing valid to do: Escape, then a fresh start.
    fn dead_end(&mut self) -> Result<(), String> {
        let escaped = self
            .run
            .recoveries
            .last()
            .is_some_and(|r| r.kind == "escape" && r.at == self.run.chosen);
        if escaped || !self.now["approval"].is_null() {
            self.recover("restarted", "no valid action".into());
            return self.start(true);
        }
        self.recover("escape", "no valid action".into());
        let step = Step {
            action: json!({ "kind": "key", "keys": "escape" }),
            key: format!("{}|keys|escape|key", screen_of(&self.now)),
            screen: screen_of(&self.now),
            label: "Escape".into(),
            expect: None,
            by: "recovery".into(),
        };
        self.take(step, None, "no valid action: Escape")
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
            "lastActions": last,
        })
    }
}

fn way_is_for_dialogs(way: Way) -> bool {
    way == Way::Cancel
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
                { "id": "Message", "label": "Message", "role": "field", "region": "conversation" },
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
        // Inside the instance's folder, it may be confirmed.
        o["controls"][1]["value"] = json!("c:/explore/run/start-1/Shop");
        let list = candidates(&o, Some(folder));
        assert!(list.iter().any(|c| c.action["control"] == "dialog-confirm"));
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
        let shown = vec![button, field];
        let read = |said: &str| read_answer(said, &shown, &list);
        let (choice, (input, expect, why)) = read(
            r#"{"choice": "a2", "input": "Shop", "expect": {"dialog": null}, "why": "try a name"}"#,
        )
        .unwrap();
        assert_eq!(
            (choice.as_str(), input.as_deref(), why.as_str()),
            ("a2", Some("Shop"), "try a name")
        );
        assert_eq!(expect, Some(json!({ "dialog": null })));
        assert!(read(r#"{"choice": "a3"}"#).is_err(), "not an option");
        assert!(
            read(r#"{"choice": "a1", "input": "x"}"#).is_err(),
            "text for a button"
        );
        // An empty text for a button is no input.
        let (_, (input, _, _)) = read(r#"{"choice": "a1", "input": ""}"#).unwrap();
        assert_eq!(input, None);
        assert!(
            read(r#"{"choice": "a1", "expect": {"pixels": 3}}"#).is_err(),
            "not checkable"
        );
        assert!(read("no answer").is_err());
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
