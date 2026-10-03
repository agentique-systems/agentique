//! A real interaction journey through the control interface (C-53, W11.3):
//! the actual Studio binary, started with `--control`, is driven only
//! through its local endpoint, as an agent drives it: observations as text,
//! actions by control id, label or command, and waits. It exercises input,
//! focus, navigation, dialogs and rendering in the visible window, the
//! refusal of stale actions, Pause and Step, and the event trace.
//!
//! It needs a desktop session (it opens a window), so it is ignored by
//! default:
//!
//! ```text
//! cargo test -p agq-studio-native --test control_journey -- --ignored --nocapture
//! ```

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

struct Studio {
    child: Child,
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    token: String,
    instance: String,
}

impl Drop for Studio {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Studio {
    fn start(folder: &Path) -> Studio {
        let control = folder.join("control.json");
        let child = Command::new(env!("CARGO_BIN_EXE_agq-studio-native"))
            .args(["--no-restore", "--session"])
            .arg(folder.join("session").join("studio-session.json"))
            .arg("--control")
            .arg(&control)
            .spawn()
            .expect("the Studio starts");
        let deadline = Instant::now() + Duration::from_secs(60);
        let text = loop {
            if let Ok(text) = std::fs::read_to_string(&control)
                && !text.trim().is_empty()
            {
                break text;
            }
            assert!(
                Instant::now() < deadline,
                "no control endpoint within a minute"
            );
            std::thread::sleep(Duration::from_millis(200));
        };
        let file: Value = serde_json::from_str(&text).unwrap();
        let stream =
            TcpStream::connect(("127.0.0.1", file["port"].as_u64().unwrap() as u16)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        let mut studio = Studio {
            child,
            stream,
            reader,
            token: file["token"].as_str().unwrap().to_string(),
            instance: String::new(),
        };
        let hello = studio.call(json!({ "op": "hello" }));
        studio.instance = hello["identity"]["instance"].as_str().unwrap().to_string();
        studio
    }

    fn call(&mut self, mut body: Value) -> Value {
        body["token"] = json!(self.token);
        writeln!(self.stream, "{body}").unwrap();
        self.stream.flush().unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line}"))
    }

    fn observe(&mut self) -> Value {
        let o = self.call(json!({ "op": "observe", "detail": "full" }));
        assert_eq!(o["ok"], true, "{o}");
        o
    }

    /// An action against the latest observation; returns the answer.
    fn act(&mut self, observed: &Value, action: Value, why: &str) -> Value {
        let answer = self.call(json!({
            "op": "act",
            "agent": "journey",
            "why": why,
            "expect": { "instance": self.instance },
            "observed": observed["screenRevision"],
            "action": action,
        }));
        eprintln!("{why}: {answer}");
        answer
    }

    /// Acts and requires success, observing first.
    fn must(&mut self, action: Value, why: &str) -> Value {
        let observed = self.observe();
        let answer = self.act(&observed, action, why);
        assert_eq!(answer["ok"], true, "{why}: {answer}");
        answer
    }
}

fn control<'a>(observation: &'a Value, id: &str) -> Option<&'a Value> {
    observation["controls"]
        .as_array()?
        .iter()
        .find(|c| c["id"] == id || c["label"] == id)
}

fn folder() -> PathBuf {
    let folder = std::env::temp_dir().join(format!("agq-control-journey-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
#[ignore = "opens a window: run on a desktop with --ignored"]
fn an_agent_drives_the_visible_studio_through_the_control_interface() {
    let folder = folder();
    let mut studio = Studio::start(&folder);

    // The welcome screen, as text.
    let welcome = studio.observe();
    assert_eq!(welcome["screen"], "welcome", "{welcome}");
    assert!(control(&welcome, "welcome-sample").is_some(), "{welcome}");
    assert!(
        welcome["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "new-project")
    );

    // A dialog through the pointer, its fields through the keyboard.
    studio.must(
        json!({ "kind": "click", "control": "welcome-sample" }),
        "open the sample's dialog",
    );
    let dialog = studio.observe();
    assert_eq!(dialog["dialog"], "NewProject", "{dialog}");
    assert!(control(&dialog, "Project folder").is_some(), "{dialog}");
    let project = folder.join("Shortener");
    studio.must(
        json!({ "kind": "fill", "control": "Project folder", "text": project.display().to_string() }),
        "say where the project goes",
    );
    let filled = studio.observe();
    assert_eq!(
        control(&filled, "Project folder").unwrap()["value"],
        project.display().to_string(),
        "the field shows what was typed"
    );
    // A stale action: observed on the welcome screen, which has changed
    // since. Refused, and nothing happens.
    let refused = studio.act(
        &welcome,
        json!({ "kind": "click", "control": "welcome-new" }),
        "a stale click",
    );
    assert_eq!(refused["ok"], false);
    assert!(
        refused["error"].as_str().unwrap().starts_with("stale"),
        "{refused}"
    );
    assert_eq!(
        studio.observe()["dialog"],
        "NewProject",
        "the stale click changed nothing"
    );
    // Another instance's observation is refused too.
    let mut other = filled.clone();
    let answer = studio.call(json!({
        "op": "act", "agent": "journey", "why": "wrong instance",
        "expect": { "instance": "not-this-one" }, "observed": other["screenRevision"].take(),
        "action": { "kind": "key", "keys": "escape" },
    }));
    assert_eq!(answer["ok"], false);
    assert!(
        answer["error"].as_str().unwrap().contains("instance"),
        "{answer}"
    );

    studio.must(
        json!({ "kind": "click", "control": "dialog-confirm" }),
        "create the project",
    );
    studio.must(
        json!({ "kind": "wait", "until": { "dialog": null, "screen": "surface" }, "timeoutMs": 30000 }),
        "wait for the project",
    );
    let surface = studio.observe();
    assert_eq!(
        surface["project"]["folder"].as_str().map(PathBuf::from),
        Some(project.clone())
    );
    assert!(
        surface["cards"].as_array().is_some_and(|c| !c.is_empty()),
        "cards in view: {surface}"
    );

    // Selecting an element shows it in the Inspector.
    studio.must(
        json!({ "kind": "select", "element": "UrlShortener::LinkStore" }),
        "look at the link store",
    );
    let selected = studio.observe();
    assert!(
        selected["selection"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == "UrlShortener::LinkStore"),
        "{selected}"
    );

    // A command opens a dialog; its field is focused; Enter confirms.
    studio.must(
        json!({ "kind": "command", "id": "checkpoint" }),
        "record a checkpoint",
    );
    let checkpoint = studio.observe();
    assert_eq!(checkpoint["dialog"], "Checkpoint", "{checkpoint}");
    studio.must(
        json!({ "kind": "type", "text": "Agent's first checkpoint" }),
        "describe it",
    );
    studio.must(json!({ "kind": "key", "keys": "enter" }), "confirm");
    studio.must(
        json!({ "kind": "wait", "until": { "dialog": null }, "timeoutMs": 10000 }),
        "wait",
    );
    let history = studio.observe();
    assert!(
        history["status"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("checkpoint"),
        "{history}"
    );

    // An unavailable command is refused with the Studio's reason.
    let undo_nothing = studio.observe();
    let redo = studio.act(
        &undo_nothing,
        json!({ "kind": "command", "id": "redo" }),
        "redo nothing",
    );
    assert_eq!(redo["ok"], false, "{redo}");

    // Settings and back, by keys.
    studio.must(
        json!({ "kind": "command", "id": "settings" }),
        "open Settings",
    );
    assert_eq!(studio.observe()["screen"], "settings");
    studio.must(json!({ "kind": "key", "keys": "escape" }), "close Settings");
    assert_eq!(studio.observe()["screen"], "surface");

    // Pause holds actions (observing goes on); Step lets one through.
    assert_eq!(
        studio.call(json!({ "op": "gate", "mode": "pause" }))["ok"],
        true
    );
    let before = studio.observe();
    assert_eq!(before["view"], "architecture");
    let token = studio.token.clone();
    let port = studio.stream.peer_addr().unwrap().port();
    let instance = studio.instance.clone();
    let revision = before["screenRevision"].clone();
    let held =
        std::thread::spawn(move || {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            writeln!(stream, "{}", json!({
            "token": token, "op": "act", "agent": "journey", "why": "switch to the graph view",
            "expect": { "instance": instance }, "observed": revision,
            "action": { "kind": "command", "id": "graph" },
        })).unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            serde_json::from_str::<Value>(&line).unwrap()
        });
    std::thread::sleep(Duration::from_millis(800));
    let paused = studio.observe();
    assert_eq!(paused["agents"]["held"], 1, "{paused}");
    assert_eq!(
        paused["view"], "architecture",
        "nothing happens while paused"
    );
    assert_eq!(
        studio.call(json!({ "op": "gate", "mode": "step" }))["ok"],
        true
    );
    let answer = held.join().unwrap();
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(studio.observe()["view"], "graph");
    assert_eq!(
        studio.observe()["agents"]["gate"],
        "pause",
        "after one step it holds again"
    );
    assert_eq!(
        studio.call(json!({ "op": "gate", "mode": "run" }))["ok"],
        true
    );

    // The trace has every action, refusals included.
    let events = studio.call(json!({ "op": "events", "since": 0 }));
    let events = events["events"].as_array().unwrap();
    assert!(events.iter().any(|e| {
        e["ok"] == false
            && e["detail"]["error"]
                .as_str()
                .is_some_and(|t| t.starts_with("stale"))
    }));
    assert!(
        events
            .iter()
            .any(|e| e["what"] == "command checkpoint" && e["ok"] == true)
    );
    eprintln!("{} events in the trace", events.len());
}
