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
/// never waits. A closure goes on while it returns true and stops only
/// between steps.
pub trait Supervisor {
    fn go_on(&mut self) -> bool;
    fn stopped(&mut self) -> bool {
        false
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
        copy_model(&self.start, &project.join("model"))?;
        // Its repository is the project's copy, so nothing it opens or
        // writes lies outside its folder.
        let mut instance =
            TestInstance::start(&self.exe, &base.join("instance"), &project, &project)?;
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
    /// `explorer` (chosen), `check` (the undo check's undo and redo, Stop
    /// after a turn ran past its budget), `wait` (for a turn of the
    /// Assistant to end: `label` is `the turn`, or `the stop` after Stop) or
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
    /// Jev (its model, threshold and deadline are its own) and the models:
    /// a [`decide::Decider`], or a stand-in in tests.
    pub answers: &'a dyn Answers,
    /// The explorer's model and effort: the Model way.
    pub explorer: ModelRef,
    pub effort: Option<String>,
    /// The model Jev escalates to, and its effort.
    pub escalation: ModelRef,
    pub escalation_effort: Option<String>,
}

/// How many of the rules' best actions Jev chooses among, and the model.
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

const FORMAT: &str = "{\"choice\": \"<one option id>\", \"why\": \"<one short line>\"}, with \"input\": \"<the text to type>\" added only when the option types into a field (it replaces the option's text; for the Conversation, the request to send; never a path that leads elsewhere), and \"expect\": {...} added only when the action must make something true in the next observation, with any of: screen, dialog (a kind, or null), statusContains, selectionContains, control (an id or label) with labelContains, valueContains or enabled, anyLabelContains, and replyContains (text the Assistant's reply will hold)";

/// The typed question for the next action: the best `n` of the rules'
/// order as options `a01`, `a02`, … (zero-padded, so they read in order),
/// with the ids and the candidates they stand for.
fn question(
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
    let mut coverage = serde_json::Map::new();
    for (id, index) in &shown {
        let c = &candidates[*index];
        let before = knowledge.count(&c.key);
        let here = covered.get(&c.key).copied().unwrap_or(0);
        let note = match (before, here) {
            (0, 0) => "never covered".to_string(),
            (_, 0) => "covered before, not in this run".to_string(),
            (_, n) => format!("covered {n} time(s) in this run"),
        };
        options.insert(id.clone(), Some(format!("{} — {note}", c.about)));
        coverage.insert(id.clone(), json!({ "before": before, "thisRun": here }));
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
/// rules' best, used when confident; the model among more of them, its
/// answer checked; Jev escalating to the model. Whatever fails falls back
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
            failure.error,
        ),
    };
    if way == Way::Escalating {
        choosing.model(
            &deciding.escalation,
            deciding.escalation_effort.as_deref(),
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
    };
    explorer.run.ended = match explorer.go() {
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
    fn go(&mut self) -> Result<(), String> {
        if self.plan.way == Way::Cancel {
            return Err("the cancelling rule decides dialogs in the way, not exploration".into());
        }
        self.start(false)?;
        while self.run.actions < self.plan.steps {
            if !self.supervisor.go_on() {
                return Err("stopped".into());
            }
            self.within_budgets()?;
            self.step()?;
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
        let restarted = self
            .instance
            .restart(&mut || halted(supervisor, started, seconds));
        if let Err(error) = restarted {
            if halted(self.supervisor, started, seconds) {
                return Err(self.why_halted());
            }
            return Err(format!("the test instance did not start: {error}"));
        }
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
        // A turn of the Assistant runs (a request was sent): its end is
        // waited for before anything else.
        if observed::turn_running(&self.now) {
            return self.await_turn();
        }
        let mut candidates = candidates(&self.now, self.folder.as_deref());
        candidates.retain(|c| !self.avoid.contains(&c.key));
        if !self.plan.conversation {
            candidates.retain(|c| c.area != "conversation");
        }
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
        let choosing = Choosing {
            deciding: self.deciding,
            candidates: &candidates,
            ranked: &ranked,
            state: &state,
            knowledge: self.knowledge,
            covered: &self.run.covered,
        };
        let (supervisor, started, seconds) =
            (&mut *self.supervisor, self.started, self.plan.seconds);
        let (index, chosen) = choose(self.plan.way, &choosing, &mut || {
            halted(supervisor, started, seconds)
        });
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
            let refusal = observed::refusal(&answer);
            if refusal == Refusal::Rule {
                // The Operator's own, by rule: never a finding, never again.
                self.run.unwanted.refused += 1;
                self.avoid.insert(step.key.clone());
                self.record(&step, chosen, "refused", error, None);
                return Ok(());
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
            // Carried out (or failed partway, which may have done part of
            // it): a step a replay takes again.
            let outcome = if answer["ok"] == false {
                "failed"
            } else {
                "ok"
            };
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
        let waited = findings::await_turn(self.instance, budget_ms, &mut || {
            halted(supervisor, started, seconds)
        });
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
        let answer = match self.instance.act(&act) {
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
        })
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
