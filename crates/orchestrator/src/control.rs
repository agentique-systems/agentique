//! Test instances and the client of their control interface (C-53, ROADMAP
//! §4.16): a cycle starts a build of Agentique with its own app data and a
//! control endpoint, and its evaluator (and the Orchestrator's own
//! assertions) operate it through observations and actions, as an agent
//! operates the running Studio. The instance and its process tree end with
//! the value.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
    /// the commit, never the Operator's).
    pub fn start(
        exe: &Path,
        folder: &Path,
        project: &Path,
        repository: &Path,
    ) -> Result<TestInstance, String> {
        let _ = std::fs::remove_dir_all(folder);
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        let endpoint = folder.join("control.json");
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
        // Its working folder is its own, so a relative path typed into one of
        // its fields (a project folder, say) stays inside it.
        let child = Command::new(exe)
            .current_dir(folder)
            .env_clear()
            .envs(environment)
            .arg("--no-restore")
            .arg("--session")
            .arg(folder.join("session").join("studio-session.json"))
            .arg("--control")
            .arg(&endpoint)
            .arg("--project")
            .arg(project)
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
