//! A real interaction journey through the control interface (C-53, W11.3;
//! C-54, W12.2): the actual Studio binary, started with `--control`, is
//! driven only through its local endpoint, as an agent drives it:
//! observations as text, actions by control id, label or command, and waits.
//! It exercises input, focus, navigation, dialogs, the palette and rendering
//! in the visible window, the refusal of stale actions and of what is the
//! Operator's own (Settings, locking, the Objectives panel, text without a
//! focused field), the label rule, one agent holding the window at a time,
//! Pause and Step, and the event trace; a second journey watches observer
//! mode: typing a character at a time, Pause and Step between characters,
//! and Stop.
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
    port: u16,
    instance: String,
}

impl Drop for Studio {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Studio {
    /// The Studio, with agents' actions at `speed`.
    fn start(folder: &Path, speed: &str) -> Studio {
        let control = folder.join("control.json");
        let child = Command::new(env!("CARGO_BIN_EXE_agq-studio-native"))
            .args(["--no-restore", "--control-speed", speed, "--session"])
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
        let port = file["port"].as_u64().unwrap() as u16;
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        let mut studio = Studio {
            child,
            stream,
            reader,
            token: file["token"].as_str().unwrap().to_string(),
            port,
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

    /// An action request against an observation.
    fn body(&self, agent: &str, observed: &Value, action: Value, why: &str) -> Value {
        json!({
            "op": "act",
            "agent": agent,
            "why": why,
            "goal": "the control journey",
            "expect": { "instance": self.instance },
            "observed": observed["screenRevision"],
            "action": action,
        })
    }

    /// An action against the latest observation; returns the answer.
    fn act(&mut self, observed: &Value, action: Value, why: &str) -> Value {
        self.act_as("journey", observed, action, why)
    }

    fn act_as(&mut self, agent: &str, observed: &Value, action: Value, why: &str) -> Value {
        let answer = self.call(self.body(agent, observed, action, why));
        eprintln!("{agent}: {why}: {answer}");
        answer
    }

    /// Acts and requires success, observing first.
    fn must(&mut self, action: Value, why: &str) -> Value {
        let observed = self.observe();
        let answer = self.act(&observed, action, why);
        assert_eq!(answer["ok"], true, "{why}: {answer}");
        answer
    }

    /// Sends `body` on a connection of its own and waits for the answer in
    /// a thread, so the journey can watch (and pause) the action meanwhile.
    fn beside(&self, mut body: Value) -> std::thread::JoinHandle<(Value, Duration)> {
        body["token"] = json!(self.token);
        let port = self.port;
        std::thread::spawn(move || {
            let started = Instant::now();
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            writeln!(stream, "{body}").unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            (
                serde_json::from_str::<Value>(&line).unwrap(),
                started.elapsed(),
            )
        })
    }

    fn gate(&mut self, mode: &str) {
        let answer = self.call(json!({ "op": "gate", "mode": mode }));
        assert_eq!(answer["ok"], true, "{answer}");
    }
}

fn control<'a>(observation: &'a Value, id: &str) -> Option<&'a Value> {
    observation["controls"]
        .as_array()?
        .iter()
        .find(|c| c["id"] == id || c["label"] == id)
}

/// The label rule (W12.2, C-54): a control an agent can operate (a button,
/// field, tab, option, item, switch or link) has a label a person can read:
/// not empty, and not just its machine id (`objective-usd`). Returns the
/// controls that break it.
fn unreadable(observation: &Value) -> Vec<String> {
    const INTERACTIVE: [&str; 7] = ["button", "field", "tab", "option", "item", "switch", "link"];
    let machine_id = |text: &str| {
        !text.is_empty()
            && text.contains(['-', '_'])
            && text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    };
    observation["controls"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| INTERACTIVE.contains(&c["role"].as_str().unwrap_or_default()))
        .filter(|c| {
            let label = c["label"].as_str().unwrap_or_default().trim();
            let id = c["id"].as_str().unwrap_or_default();
            label.is_empty() || (label == id && machine_id(id))
        })
        .map(|c| c.to_string())
        .collect()
}

fn assert_readable(observation: &Value, screen: &str) {
    let found = unreadable(observation);
    assert!(
        found.is_empty(),
        "{screen}: controls without a readable label: {found:#?}"
    );
}

fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("agq-control-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn the_label_rule_finds_unlabelled_controls() {
    let observation = json!({ "controls": [
        { "id": "objective-usd", "role": "field", "label": "objective-usd" },
        { "id": "objective-cycles", "role": "field", "label": "Improvements" },
        { "id": "Settings", "role": "button", "label": "Settings" },
        { "id": "x", "role": "switch", "label": " " },
        { "id": "Viewport", "role": "area", "label": "" },
    ] });
    let found = unreadable(&observation);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found[0].contains("objective-usd") && found[1].contains("switch"));
}

#[test]
#[ignore = "opens a window: run on a desktop with --ignored"]
fn an_agent_drives_the_visible_studio_through_the_control_interface() {
    let folder = folder("journey");
    let mut studio = Studio::start(&folder, "instant");

    // The welcome screen, as text.
    let welcome = studio.observe();
    assert_eq!(welcome["screen"], "welcome", "{welcome}");
    assert_eq!(welcome["agents"]["speed"], "instant");
    assert!(control(&welcome, "welcome-sample").is_some(), "{welcome}");
    assert!(
        welcome["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "new-project")
    );
    assert_readable(&welcome, "the welcome screen");
    // The title bar's own controls are observed, and the window's buttons
    // are the Operator's.
    for (id, label) in [
        ("title-search", "Search or run a command"),
        ("window-minimize", "Minimize"),
        ("window-close", "Close"),
    ] {
        let found = control(&welcome, id).unwrap_or_else(|| panic!("{id}: {welcome}"));
        assert_eq!(found["label"], label);
    }
    assert_eq!(
        control(&welcome, "window-close").unwrap()["operatorOnly"],
        true
    );

    // A dialog through the pointer, its fields through the keyboard.
    studio.must(
        json!({ "kind": "click", "control": "welcome-sample" }),
        "open the sample's dialog",
    );
    let dialog = studio.observe();
    assert_eq!(dialog["dialog"], "NewProject", "{dialog}");
    assert!(control(&dialog, "Project folder").is_some(), "{dialog}");
    assert_readable(&dialog, "the New project dialog");
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
    assert_readable(&surface, "the Surface with a project");
    // The model's digest: a short hash of its text.
    let digest = surface["project"]["digest"].as_str().unwrap_or_default();
    assert_eq!(digest.len(), 16, "{surface}");
    let lock = surface["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "lock")
        .unwrap();
    assert_eq!(lock["operatorOnly"], true, "{lock}");

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
    assert_readable(&selected, "the Inspector with an element");
    for field in ["Name", "Docs"] {
        let found = control(&selected, field).unwrap_or_else(|| panic!("{field}: {selected}"));
        assert_eq!(found["role"], "field", "{found}");
    }

    // A control below its panel's fold is listed as hidden; clicking it
    // scrolls the panel to it first, as the Operator would. "Find usages"
    // only reads.
    let usages = control(&selected, "reuse-usages").expect("the Inspector offers Find usages");
    assert_eq!(
        usages["hidden"], true,
        "below the Inspector's fold: {usages}"
    );
    studio.must(
        json!({ "kind": "click", "control": "reuse-usages" }),
        "find the link store's usages",
    );
    let after = studio.observe();
    let now = control(&after, "reuse-usages").expect("still on screen");
    assert!(now["hidden"].is_null(), "scrolled into view: {now}");

    // A command opens a dialog; its field is focused; Enter confirms.
    studio.must(
        json!({ "kind": "command", "id": "checkpoint" }),
        "record a checkpoint",
    );
    let checkpoint = studio.observe();
    assert_eq!(checkpoint["dialog"], "Checkpoint", "{checkpoint}");
    assert_readable(&checkpoint, "the Checkpoint dialog");
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

    // Settings and back, by keys; inside, Settings are the Operator's, and
    // say so in the observation; a key field never tells its value.
    studio.must(
        json!({ "kind": "command", "id": "settings" }),
        "open Settings",
    );
    let settings = studio.observe();
    assert_eq!(settings["screen"], "settings");
    assert_readable(&settings, "Settings");
    let inside: Vec<&Value> = settings["controls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["region"] == "settings")
        .collect();
    assert!(!inside.is_empty(), "Settings has controls");
    for c in &inside {
        assert_eq!(c["operatorOnly"], true, "{c}");
        if c["id"].as_str().unwrap_or_default().starts_with("key-") {
            assert!(c["value"].is_null(), "a key field tells nothing: {c}");
        }
    }
    let refused = studio.act(
        &settings,
        json!({ "kind": "click", "control": inside[0]["id"] }),
        "press something in Settings",
    );
    assert_eq!(refused["ok"], false, "{refused}");
    assert!(
        refused["error"].as_str().unwrap().contains("Operator"),
        "{refused}"
    );
    studio.must(json!({ "kind": "key", "keys": "escape" }), "close Settings");
    let surface = studio.observe();
    assert_eq!(surface["screen"], "surface");

    // Text with no field focused would reach the shortcuts: refused. So is
    // locking (the Operator's), filling a button, and an action that does
    // not say which observation it rests on.
    let typed = studio.act(
        &surface,
        json!({ "kind": "type", "text": "p" }),
        "type with nothing focused",
    );
    assert_eq!(typed["ok"], false, "{typed}");
    assert!(
        typed["error"].as_str().unwrap().contains("focus"),
        "{typed}"
    );
    let locked = studio.act(
        &surface,
        json!({ "kind": "command", "id": "lock" }),
        "lock the selection",
    );
    assert_eq!(locked["ok"], false, "{locked}");
    assert!(
        locked["error"].as_str().unwrap().contains("Operator"),
        "{locked}"
    );
    // Other spellings and routes reach the same refusal: a key GPUI reads
    // as ctrl-i, and the palette choosing an Operator-only command.
    let now = studio.observe();
    let spelled = studio.act(
        &now,
        json!({ "kind": "key", "keys": "secondary-i" }),
        "insert the selection into the Operator's message",
    );
    assert_eq!(spelled["ok"], false, "{spelled}");

    // The palette: its search field takes typing (W12.2), and its rows are
    // options named by their commands.
    studio.must(
        json!({ "kind": "command", "id": "palette" }),
        "open the palette",
    );
    let palette = studio.observe();
    let search = control(&palette, "palette-search").expect("the palette's search field");
    assert_eq!(search["label"], "Search commands", "{search}");
    assert_eq!(search["focused"], true, "{search}");
    assert_readable(&palette, "the palette");
    studio.must(
        json!({ "kind": "type", "text": "New conversation" }),
        "find a command in the palette",
    );
    let found = studio.observe();
    assert_eq!(
        control(&found, "palette-search").unwrap()["value"],
        "New conversation"
    );
    let row = control(&found, "palette-new-conversation").expect("its row");
    assert_eq!(row["role"], "option", "{row}");
    assert_eq!(row["label"], "New conversation", "{row}");
    assert_eq!(row["selected"], true, "the highlighted row: {row}");
    assert_eq!(row["operatorOnly"], true, "{row}");
    // Choosing an Operator-only command is refused: the click up front,
    // Enter where it acts.
    let clicked = studio.act(
        &found,
        json!({ "kind": "click", "control": "palette-new-conversation" }),
        "start a new conversation",
    );
    assert_eq!(clicked["ok"], false, "{clicked}");
    let entered = studio.act(
        &found,
        json!({ "kind": "key", "keys": "enter" }),
        "start a new conversation by Enter",
    );
    assert_eq!(entered["ok"], false, "{entered}");
    assert!(
        entered["error"].as_str().unwrap().contains("Operator"),
        "{entered}"
    );
    // An ordinary command through the palette.
    if studio.observe()["palette"].is_null() {
        studio.must(
            json!({ "kind": "command", "id": "palette" }),
            "open the palette again",
        );
    }
    studio.must(
        json!({ "kind": "fill", "control": "palette-search", "text": "Fit to view" }),
        "find Fit to view",
    );
    let fit = studio.observe();
    assert_eq!(
        control(&fit, "palette-fit").unwrap()["label"],
        "Fit to view",
        "{fit}"
    );
    studio.must(
        json!({ "kind": "click", "control": "palette-fit" }),
        "run it",
    );
    let surface = studio.observe();
    assert!(
        surface["palette"].is_null(),
        "the palette closed: {surface}"
    );

    let button = surface["controls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["role"] == "button"
                && c["enabled"] != false
                && c["hidden"].is_null()
                && c["operatorOnly"].is_null()
                && !matches!(c["region"].as_str(), Some("title" | "status"))
        })
        .expect("a button on screen")["id"]
        .clone();
    let filled = studio.act(
        &surface,
        json!({ "kind": "fill", "control": button, "text": "x" }),
        "fill a button",
    );
    assert_eq!(filled["ok"], false, "{filled}");
    assert!(
        filled["error"].as_str().unwrap().contains("not a field"),
        "{filled}"
    );
    let unobserved = studio.call(json!({
        "op": "act", "agent": "journey", "why": "no observation",
        "expect": { "instance": studio.instance.clone() },
        "action": { "kind": "command", "id": "fit" },
    }));
    assert_eq!(unobserved["ok"], false, "{unobserved}");
    assert!(
        unobserved["error"].as_str().unwrap().contains("observed"),
        "{unobserved}"
    );
    assert_eq!(studio.observe()["screen"], "surface", "nothing happened");

    // The Objectives panel: its fields and switches are labelled as the
    // Operator reads them, and are the Operator's.
    studio.must(
        json!({ "kind": "click", "control": "Objectives" }),
        "look at the Objectives panel",
    );
    let objectives = studio.observe();
    assert_readable(&objectives, "the Objectives panel");
    for (id, label) in [
        (
            "objective-intent",
            "What should Agentique improve in itself?",
        ),
        ("objective-usd", "Spend budget (USD)"),
        ("objective-cycles", "Improvements"),
        (
            "objective-merge",
            "Merge reviewed changes that pass every check",
        ),
    ] {
        let found = control(&objectives, id).unwrap_or_else(|| panic!("{id}: {objectives}"));
        assert_eq!(found["label"], label, "{found}");
        assert_eq!(found["operatorOnly"], true, "{found}");
    }
    let merge = control(&objectives, "objective-merge").unwrap();
    assert_eq!(merge["role"], "switch");
    assert_eq!(merge["value"], "on");

    // One agent acts in the window at a time (C-54): `journey` holds it;
    // another agent's action is refused with who holds it (not as stale),
    // observing goes on, and the supervisor's own steps by rule pass; once
    // released, the other agent acts.
    let held = studio.observe();
    assert_eq!(held["agents"]["holder"], "journey", "{held}");
    let intruder = studio.act_as(
        "intruder",
        &held,
        json!({ "kind": "command", "id": "fit" }),
        "fit the view",
    );
    assert_eq!(intruder["ok"], false, "{intruder}");
    assert_eq!(
        intruder["error"],
        "refused: the window is in use by journey; act in your own test instance, or wait"
    );
    let supervisor = studio.act_as(
        "orchestrator",
        &held,
        json!({ "kind": "command", "id": "fit" }),
        "a step by rule",
    );
    assert_eq!(supervisor["ok"], true, "{supervisor}");
    let released = studio.call(json!({ "op": "release", "agent": "journey" }));
    assert_eq!(released, json!({ "ok": true, "released": true }));
    let now = studio.observe();
    let intruder = studio.act_as(
        "intruder",
        &now,
        json!({ "kind": "command", "id": "fit" }),
        "fit the view",
    );
    assert_eq!(intruder["ok"], true, "{intruder}");
    let taken = studio.act(&now, json!({ "kind": "command", "id": "fit" }), "fit");
    assert!(
        taken["error"]
            .as_str()
            .unwrap()
            .contains("in use by intruder"),
        "{taken}"
    );
    let released = studio.call(json!({ "op": "release", "agent": "intruder" }));
    assert_eq!(released["released"], true);

    // Pause holds actions (observing goes on); Step lets one through.
    studio.gate("pause");
    let before = studio.observe();
    assert_eq!(before["view"], "architecture");
    let held = studio.beside(studio.body(
        "journey",
        &before,
        json!({ "kind": "command", "id": "graph" }),
        "switch to the graph view",
    ));
    std::thread::sleep(Duration::from_millis(800));
    let paused = studio.observe();
    assert_eq!(paused["agents"]["held"], 1, "{paused}");
    assert_eq!(
        paused["view"], "architecture",
        "nothing happens while paused"
    );
    studio.gate("step");
    let (answer, _) = held.join().unwrap();
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(studio.observe()["view"], "graph");
    assert_eq!(
        studio.observe()["agents"]["gate"],
        "pause",
        "after one step it holds again"
    );
    studio.gate("run");

    // The trace has every action, refusals included, with the reason, the
    // goal and who held the window.
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
    let refused = events
        .iter()
        .find(|e| e["agent"] == "intruder" && e["ok"] == false)
        .expect("the refused intruder");
    assert_eq!(refused["holder"], "journey", "{refused}");
    assert_eq!(refused["why"], "fit the view");
    assert_eq!(refused["goal"], "the control journey");
    eprintln!("{} events in the trace", events.len());
}

/// Observer mode (C-54): at `observe`, an agent's text goes in about twelve
/// characters a second; Pause holds it between characters, Step lets one
/// through, and Stop ends the action at once and refuses agents until
/// Resume.
#[test]
#[ignore = "opens a window: run on a desktop with --ignored"]
fn the_operator_watches_agents_act_at_the_observer_speed() {
    let folder = folder("observer");
    let mut studio = Studio::start(&folder, "observe");
    let welcome = studio.observe();
    assert_eq!(welcome["agents"]["speed"], "observe");
    let started = Instant::now();
    studio.must(
        json!({ "kind": "click", "control": "welcome-sample" }),
        "open the sample's dialog",
    );
    assert!(
        started.elapsed() >= Duration::from_millis(300),
        "the target is shown before the click"
    );
    let dialog = studio.observe();
    assert_eq!(dialog["dialog"], "NewProject", "{dialog}");
    let typed = |studio: &mut Studio| {
        control(&studio.observe(), "Project folder").unwrap()["value"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .count()
    };
    // Character by character, and Pause holds between them.
    let text = "C:/agentique/observer/one-character-at-a-time";
    let fill = studio.beside(studio.body(
        "journey",
        &dialog,
        json!({ "kind": "fill", "control": "Project folder", "text": text }),
        "type where the project goes",
    ));
    std::thread::sleep(Duration::from_millis(1500));
    let busy = studio.observe();
    let stop = control(&busy, "agents-stop").expect("the chip offers Stop while agents act");
    assert_eq!(stop["operatorOnly"], true, "{stop}");
    studio.gate("pause");
    std::thread::sleep(Duration::from_millis(300));
    let at_pause = typed(&mut studio);
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(typed(&mut studio), at_pause, "held between characters");
    assert!(
        at_pause > 0 && at_pause < text.chars().count(),
        "part typed: {at_pause}"
    );
    studio.gate("step");
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(typed(&mut studio), at_pause + 1, "one character through");
    studio.gate("run");
    let (answer, took) = fill.join().unwrap();
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(
        control(&studio.observe(), "Project folder").unwrap()["value"],
        text
    );
    // About twelve a second: the text took at least most of that.
    let least = Duration::from_millis(83 * text.chars().count() as u64 * 3 / 4);
    assert!(
        took >= least,
        "{took:?} for {} characters",
        text.chars().count()
    );
    eprintln!(
        "{} characters in {took:?}, with a pause",
        text.chars().count()
    );

    // Stop ends the action at once and refuses agents until Resume.
    let now = studio.observe();
    let fill = studio.beside(studio.body(
        "journey",
        &now,
        json!({ "kind": "fill", "control": "Project folder", "text": text }),
        "type it again",
    ));
    std::thread::sleep(Duration::from_millis(1200));
    studio.gate("stop");
    let (answer, _) = fill.join().unwrap();
    assert_eq!(
        answer,
        json!({ "ok": false, "error": "refused: stopped by the Operator" })
    );
    let stopped_at = typed(&mut studio);
    assert!(
        stopped_at < text.chars().count(),
        "ended part way: {stopped_at}"
    );
    let stopped = studio.observe();
    assert_eq!(stopped["agents"]["gate"], "stop");
    let refused = studio.act(
        &stopped,
        json!({ "kind": "key", "keys": "escape" }),
        "leave",
    );
    assert_eq!(refused["error"], "refused: stopped by the Operator");
    studio.gate("run");
    let resumed = studio.observe();
    let answer = studio.act(
        &resumed,
        json!({ "kind": "key", "keys": "escape" }),
        "leave",
    );
    assert_eq!(answer["ok"], true, "{answer}");
    assert!(studio.observe()["dialog"].is_null());

    let events = studio.call(json!({ "op": "events", "since": 0 }));
    let fill = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["what"].as_str().is_some_and(|w| w.starts_with("fill")) && e["ok"] == true)
        .expect("the fill in the trace");
    assert_eq!(fill["why"], "type where the project goes");
    assert_eq!(fill["goal"], "the control journey");
    assert_eq!(fill["holder"], "journey");
}
