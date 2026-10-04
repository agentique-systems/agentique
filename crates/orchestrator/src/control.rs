//! Test instances and the client of their control interface (C-53, ROADMAP
//! §4.16): a cycle starts a build of Agentique with its own app data and a
//! control endpoint, and its evaluator (and the Orchestrator's own
//! assertions) operate it through observations and actions, as an agent
//! operates the running Studio. The instance and its process tree end with
//! the value, and its folder is removed.
//!
//! Since C-54 a test instance starts as one (`--test-instance`: agents may
//! operate its Conversation and undo), at the observer speed it is given
//! (`--control-speed`), with its Assistant on the scripted stand-in
//! (`--assistant-stand-in`) when the build is unreviewed, or with the
//! explorer's provider key in its environment when it is a merged build
//! that explores; and in a stated condition when a criterion asks
//! ([`CONDITIONS`]). A flag the build does not know (an older build) is not
//! passed: what it supports is read from its `--help`.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long one request may take (a wait may last up to ten minutes).
const ANSWER: Duration = Duration::from_secs(660);

/// A client of a Studio's control endpoint, from its endpoint file.
pub struct Client {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    token: String,
    /// The instance's identity (its `hello`).
    pub instance: String,
}

impl Client {
    /// Connects to the endpoint the file describes, waiting up to `within`
    /// for the file to appear (a Studio that is starting).
    pub fn connect(file: &Path, within: Duration) -> Result<Client, String> {
        let deadline = Instant::now() + within;
        let text = loop {
            match std::fs::read_to_string(file) {
                Ok(text) if !text.trim().is_empty() => break text,
                _ if Instant::now() >= deadline => {
                    return Err(format!("no control endpoint at {}", file.display()));
                }
                _ => std::thread::sleep(Duration::from_millis(200)),
            }
        };
        let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let port = value["port"]
            .as_u64()
            .ok_or("the endpoint file has no port")? as u16;
        let token = value["token"]
            .as_str()
            .ok_or("the endpoint file has no token")?
            .to_string();
        let stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|e| format!("the Studio's endpoint refused the connection: {e}"))?;
        stream
            .set_read_timeout(Some(ANSWER))
            .map_err(|e| e.to_string())?;
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut client = Client {
            stream,
            reader,
            token,
            instance: String::new(),
        };
        let hello = client.call(json!({ "op": "hello" }))?;
        client.instance = hello["identity"]["instance"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        Ok(client)
    }

    /// One request and its answer.
    pub fn call(&mut self, mut body: Value) -> Result<Value, String> {
        body["token"] = json!(self.token);
        writeln!(self.stream, "{body}")
            .and_then(|_| self.stream.flush())
            .map_err(|e| format!("the Studio is gone: {e}"))?;
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| format!("no answer from the Studio: {e}"))?;
        if line.is_empty() {
            return Err("the Studio closed the connection".into());
        }
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }

    pub fn observe(&mut self, full: bool) -> Result<Value, String> {
        self.call(json!({ "op": "observe", "detail": if full { "full" } else { "summary" } }))
    }

    /// How long an answer may take from now on (an explorer does not wait
    /// ten minutes for a Studio that stopped answering).
    pub fn answer_within(&mut self, within: Duration) -> Result<(), String> {
        self.stream
            .set_read_timeout(Some(within))
            .map_err(|e| e.to_string())
    }

    /// An action against the latest observation of this instance.
    pub fn act(&mut self, agent: &str, why: &str, action: Value) -> Result<Value, String> {
        let observed = self.observe(false)?;
        self.call(json!({
            "op": "act",
            "agent": agent,
            "why": why,
            "expect": { "instance": self.instance },
            "observed": observed["screenRevision"],
            "action": action,
        }))
    }
}

/// The conditions a test instance can be started in (C-54, ROADMAP §4.16),
/// so criteria about such states are testable: `recovered` (its builds
/// registry holds a build that did not start, and it starts as the launcher
/// starts the last known good build after it, `--recovered-from`) and `with
/// an objective` (a recorded objective and its thread in its app data,
/// which the Studio shows at start).
pub const CONDITIONS: [&str; 2] = ["recovered", "with an objective"];

/// The build a `recovered` test instance was started after.
pub const FAILED_BUILD: &str = "000000000000-0000000000";

/// The provider key a test instance's Assistant may use, and its model.
#[derive(Clone, Debug)]
pub struct InstanceKey {
    pub model: agq_providers::ModelRef,
    pub secret: Arc<agq_providers::Secret>,
}

/// How a test instance starts besides its data and project (C-54).
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Observer mode's speed (`instant`, `fast`, `observe`); `instant` when
    /// none is given.
    pub speed: Option<String>,
    /// Its Assistant on the scripted stand-in (an unreviewed build's).
    pub stand_in: bool,
    /// A provider key for its Assistant, preset to that provider and model:
    /// only for a merged build that explores.
    pub key: Option<InstanceKey>,
    /// A stated condition ([`CONDITIONS`]).
    pub condition: Option<String>,
}

/// What a build's command line supports of what a test instance needs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub test_instance: bool,
    pub control_speed: bool,
    pub stand_in: bool,
}

impl Flags {
    /// Read from `exe --help` (a build before C-54 knows none of them).
    pub fn of(exe: &Path) -> Flags {
        let help = Command::new(exe)
            .arg("--help")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        Flags {
            test_instance: help.contains("--test-instance"),
            control_speed: help.contains("--control-speed"),
            stand_in: help.contains("--assistant-stand-in"),
        }
    }
}

/// Prepares `folder` (a test instance's, before it starts) for a stated
/// condition, and returns the arguments it starts with for it.
pub fn prepare(folder: &Path, condition: &str) -> Result<Vec<String>, String> {
    match condition {
        "recovered" => {
            let builds = folder.join("builds");
            std::fs::create_dir_all(&builds).map_err(|e| e.to_string())?;
            let mut registry = agq_launcher::Registry::load(&builds)?;
            registry.add(agq_launcher::Entry {
                id: FAILED_BUILD.into(),
                created: agq_launcher::now(),
                commit: String::new(),
                state: agq_launcher::State::Failed {
                    reason: "it did not report ready within 60 s".into(),
                },
            });
            registry.save(&builds)?;
            Ok(vec!["--recovered-from".into(), FAILED_BUILD.into()])
        }
        "with an objective" => {
            seed_objective(&folder.join("session").join("objectives"))?;
            Ok(Vec::new())
        }
        other => Err(format!(
            "`{other}` is not a condition a test instance starts in ({})",
            CONDITIONS.join(", ")
        )),
    }
}

/// A recorded objective with its thread, in the objectives folder `at`: a
/// finished one, so the instance shows it and its thread at start and
/// nothing goes on by itself.
pub fn seed_objective(at: &Path) -> Result<crate::record::Objective, String> {
    use crate::record::{DirectiveStatus, Recipient, Scope, State, Store};
    use crate::thread::{Author, Kind, ThreadEntry};
    let store = Store::new(at);
    let mut objective = store.create(
        "Find and fix problems in the Library panel",
        Path::new("."),
        "main",
        crate::record::Budgets::default(),
        crate::record::Permissions::default(),
    )?;
    let lead = Author::agent(
        "lead",
        Some(agq_providers::ModelRef::new(
            agq_providers::Provider::DeepSeek,
            "deepseek-v4-pro",
        )),
    );
    let implementer = Author::agent(
        "implementer",
        Some(agq_providers::ModelRef::new(
            agq_providers::Provider::DeepSeek,
            "deepseek-flash",
        )),
    );
    let id = objective.direct(
        "lead",
        Recipient::Role("implementer".into()),
        Scope {
            instruction: "Implement the frozen proposal “Label the Library's kind filter”".into(),
            focus: None,
            budgets: None,
            permissions: None,
        },
        Some("cycle-1/proposal".into()),
    );
    objective.settle(&id, DirectiveStatus::Done, Some("approved".into()));
    objective.cycles.push(crate::record::Cycle::new(1));
    if let Some(cycle) = objective.cycle_mut() {
        cycle.phase = crate::record::Phase::Done;
    }
    objective.state = State::Done;
    objective.note = Some("1 cycle(s) done, 0 adopted.".into());
    store.save(&objective)?;
    let post = |entry: ThreadEntry| store.append_thread(&objective.id, entry);
    post(ThreadEntry::new(
        Kind::Human,
        Author::Operator,
        objective.intent.clone(),
    ))?;
    post(ThreadEntry::event("Cycle 1: Proposing an improvement"))?;
    post(
        ThreadEntry::new(
            Kind::Directive,
            lead,
            "Proposes: Label the Library's kind filter",
        )
        .with_details("Title: Label the Library's kind filter\nAcceptance criteria (frozen):\n- c1 — the filter has a readable label")
        .for_directive(Some(&id)),
    )?;
    let step = post(
        ThreadEntry::new(
            Kind::Event,
            implementer.clone(),
            "Starts its session on deepseek-flash · DeepSeek",
        )
        .for_directive(Some(&id)),
    )?;
    let mut edit = ThreadEntry::new(
        Kind::Activity,
        implementer.clone(),
        "Edit crates/studio-native/src/panels/library.rs",
    )
    .with_details("- .placeholder(\"kind\")\n+ .placeholder(\"Filter by kind\")")
    .for_directive(Some(&id));
    edit.under = Some(step.seq);
    post(edit)?;
    post(
        ThreadEntry::new(Kind::Result, implementer, "Submits the implementation")
            .with_details("Labelled the filter; added a test.")
            .for_directive(Some(&id)),
    )?;
    post(ThreadEntry::event("Cycle 1: Done"))?;
    Ok(objective)
}

/// A running test instance: a Studio executable with its own session and
/// app data, its builds folder and its control endpoint.
pub struct TestInstance {
    child: Child,
    pub folder: PathBuf,
    pub endpoint: PathBuf,
}

impl TestInstance {
    /// Starts `exe` with its data in `folder` (made fresh), opening
    /// `project`, with Agentique's repository at `repository` (a checkout of
    /// the commit, never the Operator's), as a test instance at `instant`
    /// speed with no key and its own Assistant as the build has it.
    pub fn start(
        exe: &Path,
        folder: &Path,
        project: &Path,
        repository: &Path,
    ) -> Result<TestInstance, String> {
        TestInstance::start_with(exe, folder, project, repository, &Options::default())
    }

    /// [`TestInstance::start`] with `options`.
    pub fn start_with(
        exe: &Path,
        folder: &Path,
        project: &Path,
        repository: &Path,
        options: &Options,
    ) -> Result<TestInstance, String> {
        let _ = std::fs::remove_dir_all(folder);
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        let endpoint = folder.join("control.json");
        let flags = Flags::of(exe);
        let mut arguments = Vec::new();
        if flags.test_instance {
            arguments.push("--test-instance".to_string());
        }
        if flags.control_speed {
            arguments.push("--control-speed".into());
            arguments.push(options.speed.clone().unwrap_or_else(|| "instant".into()));
        }
        if options.stand_in && flags.stand_in {
            arguments.push("--assistant-stand-in".into());
        }
        if let Some(condition) = &options.condition {
            arguments.extend(prepare(folder, condition)?);
        }
        let session = folder.join("session");
        std::fs::create_dir_all(&session).map_err(|e| e.to_string())?;
        let errors = std::fs::File::create(folder.join("errors.log"))
            .map(Stdio::from)
            .unwrap_or_else(|_| Stdio::null());
        // Only what a process needs (as Execution passes it): the Studio's
        // keys and tokens stay with the Studio, since a test instance runs
        // code no reviewer has read yet.
        let mut environment = agq_execution::Executor::new(
            agq_execution::Scope::read_only(repository).map_err(|e| e.to_string())?,
        )
        .environment();
        // What a window needs on Linux.
        for name in ["DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR"] {
            if let Ok(value) = std::env::var(name) {
                environment.push((name.to_string(), value));
            }
        }
        // A merged build that explores: its Assistant on the explorer's
        // provider and model, with that key in its environment only (a test
        // instance reads keys from nowhere else).
        if let Some(key) = &options.key {
            environment.push((
                key.model.provider.key_variable().to_string(),
                key.secret.expose().to_string(),
            ));
            let settings = json!({
                "format": 1,
                "assistant.provider": key.model.provider.id(),
                "assistant.model": key.model.model,
            });
            agq_launcher::write_atomically(
                &session.join("settings.json"),
                settings.to_string().as_bytes(),
            )?;
        }
        // Its working folder is its own, so a relative path typed into one of
        // its fields (a project folder, say) stays inside it.
        let child = Command::new(exe)
            .current_dir(folder)
            .env_clear()
            .envs(environment)
            .arg("--no-restore")
            .arg("--session")
            .arg(session.join("studio-session.json"))
            .arg("--control")
            .arg(&endpoint)
            .arg("--project")
            .arg(project)
            .args(&arguments)
            .env("AGENTIQUE_BUILDS", folder.join("builds"))
            .env("AGENTIQUE_REPOSITORY", repository)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(errors)
            .spawn()
            .map_err(|e| format!("the test instance could not start: {e}"))?;
        Ok(TestInstance {
            child,
            folder: folder.to_path_buf(),
            endpoint,
        })
    }

    /// Whether its process still runs.
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Connects to it, waiting up to `within` for it to start; a test
    /// instance that exits first says what it wrote.
    pub fn connect(&mut self, within: Duration) -> Result<Client, String> {
        let deadline = Instant::now() + within;
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                let errors =
                    std::fs::read_to_string(self.folder.join("errors.log")).unwrap_or_default();
                return Err(format!(
                    "the test instance ended ({status}) before it was ready: {}",
                    agq_execution::process::last_lines(&errors, 8)
                ));
            }
            if self.endpoint.is_file() {
                return Client::connect(&self.endpoint, Duration::from_secs(10));
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "the test instance did not start within {} s",
                    within.as_secs()
                ));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

impl Drop for TestInstance {
    fn drop(&mut self) {
        agq_execution::process::kill_tree(&mut self.child);
        remove_folder(&self.folder);
    }
}

/// Removes a test instance's folder after use, trying again for a moment
/// while the system releases its files.
pub fn remove_folder(folder: &Path) {
    for _ in 0..20 {
        if std::fs::remove_dir_all(folder).is_ok() || !folder.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The keys an observation criterion may use.
const EXPECTATIONS: [&str; 9] = [
    "screen",
    "dialog",
    "statusContains",
    "selectionContains",
    "control",
    "labelContains",
    "valueContains",
    "enabled",
    "anyLabelContains",
];

/// Whether `expect` is an observation criterion that checks something: at
/// least one known key, no unknown one, and a control for the keys about
/// one.
pub fn expectation(expect: &Value) -> Result<(), String> {
    let object = expect
        .as_object()
        .ok_or("an observation criterion's expect is an object")?;
    if object.is_empty() {
        return Err("an observation criterion must expect something".into());
    }
    if let Some(unknown) = object.keys().find(|k| !EXPECTATIONS.contains(&k.as_str())) {
        return Err(format!(
            "`{unknown}` is not something an observation can expect"
        ));
    }
    if ["labelContains", "valueContains", "enabled"]
        .iter()
        .any(|k| object.contains_key(*k))
        && !object.contains_key("control")
    {
        return Err("labelContains, valueContains and enabled are about a `control`".into());
    }
    Ok(())
}

/// Whether an observation shows what `expect` asks (an Observation
/// criterion): every field given must hold. `screen`, `dialog` (a kind, or
/// null), `statusContains`, `selectionContains`, and `control` (an id or
/// label) with `labelContains`, `valueContains` and `enabled`. Returns what
/// did not hold.
pub fn holds(observation: &Value, expect: &Value) -> Result<(), String> {
    expectation(expect)?;
    let mut problems = Vec::new();
    if let Some(screen) = expect.get("screen")
        && observation["screen"] != *screen
    {
        problems.push(format!(
            "the screen is {}, not {screen}",
            observation["screen"]
        ));
    }
    if let Some(dialog) = expect.get("dialog")
        && observation["dialog"] != *dialog
    {
        problems.push(format!(
            "the dialog is {}, not {dialog}",
            observation["dialog"]
        ));
    }
    if let Some(text) = expect["statusContains"].as_str()
        && !observation["status"]
            .as_str()
            .unwrap_or_default()
            .contains(text)
    {
        problems.push(format!(
            "the status “{}” does not say “{text}”",
            observation["status"].as_str().unwrap_or_default()
        ));
    }
    if let Some(name) = expect["selectionContains"].as_str()
        && !observation["selection"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|s| s.as_str() == Some(name))
    {
        problems.push(format!("{name} is not selected"));
    }
    if let Some(name) = expect["control"].as_str() {
        let control = observation["controls"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["id"] == name || c["label"] == name);
        match control {
            None => problems.push(format!("no control {name} is on screen")),
            Some(control) => {
                if let Some(text) = expect["labelContains"].as_str()
                    && !control["label"].as_str().unwrap_or_default().contains(text)
                {
                    problems.push(format!("{name}'s label does not say “{text}”"));
                }
                if let Some(text) = expect["valueContains"].as_str()
                    && !control["value"].as_str().unwrap_or_default().contains(text)
                {
                    problems.push(format!("{name}'s value does not say “{text}”"));
                }
                if let Some(enabled) = expect["enabled"].as_bool()
                    && (control["enabled"] != false) != enabled
                {
                    problems.push(format!(
                        "{name} is {}",
                        if enabled { "disabled" } else { "enabled" }
                    ));
                }
            }
        }
    }
    if let Some(text) = expect["anyLabelContains"].as_str()
        && !observation["controls"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| c["label"].as_str().unwrap_or_default().contains(text))
    {
        problems.push(format!("no control on screen says “{text}”"));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_observation_criterion_checks_every_field_it_names() {
        let observation = json!({
            "screen": "surface",
            "dialog": null,
            "status": "Checkpoint: done",
            "selection": ["Shop::Store"],
            "controls": [
                { "id": "build-use-0", "label": "Use this build…", "enabled": false },
                { "id": "Message", "label": "Message", "value": "hello world" },
            ],
        });
        assert!(
            holds(
                &observation,
                &json!({ "screen": "surface", "dialog": null })
            )
            .is_ok()
        );
        assert!(
            holds(
                &observation,
                &json!({ "statusContains": "Checkpoint", "selectionContains": "Shop::Store" })
            )
            .is_ok()
        );
        assert!(
            holds(
                &observation,
                &json!({ "control": "Message", "valueContains": "world" })
            )
            .is_ok()
        );
        assert!(
            holds(
                &observation,
                &json!({ "control": "build-use-0", "enabled": false })
            )
            .is_ok()
        );
        let failed = holds(
            &observation,
            &json!({ "screen": "settings", "control": "build-use-0", "enabled": true }),
        )
        .unwrap_err();
        assert!(
            failed.contains("the screen is") && failed.contains("disabled"),
            "{failed}"
        );
        assert!(holds(&observation, &json!({ "anyLabelContains": "Use this" })).is_ok());
        assert!(holds(&observation, &json!({ "control": "Nothing" })).is_err());
    }

    #[test]
    fn an_observation_criterion_must_check_something_it_knows() {
        assert!(expectation(&json!({ "screen": "surface" })).is_ok());
        assert!(expectation(&json!({})).is_err());
        assert!(expectation(&json!({ "status_contains": "x" })).is_err());
        assert!(
            expectation(&json!({ "valueContains": "x" })).is_err(),
            "about which control?"
        );
        assert!(holds(&json!({ "screen": "surface" }), &json!({})).is_err());
    }
}
