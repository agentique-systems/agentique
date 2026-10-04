//! A scripted stand-in for a test instance of the Studio (C-54, W12.4): a
//! surface with an Inspector and a History panel, an outline of elements, a
//! Create dialog, a model with undo and redo, and defects planted on
//! request, answering observations and actions in the control interface's
//! form. Without defects it behaves correctly, so exploration finds nothing
//! in it; with one, the check that defect breaks fails.

use agq_orchestrator::explore::{Act, Instance};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Defects to plant. Each control they concern is there either way and
/// works without its defect, so a replay passes on the "fixed" stand-in.
#[derive(Clone, Copy, Debug, Default)]
pub struct Defects {
    /// History's “Export” ends the instance.
    pub crash: bool,
    /// History's “Archive” button has no label.
    pub unlabelled: bool,
    /// Undo does nothing.
    pub undo: bool,
    /// “Recompute” takes 5 s.
    pub slow: bool,
    /// “Refresh” ends the instance the first time it is pressed, ever.
    pub flaky: bool,
    /// The Create dialog ignores its Cancel and Escape.
    pub stuck_dialog: bool,
    /// “Validate” puts an internal error in the status line.
    pub internal_error: bool,
    /// “Sync” is listed enabled but refused as disabled.
    pub refuses_offered: bool,
}

#[derive(Clone, Debug)]
struct Screen {
    panel: &'static str,
    dialog: bool,
    name: String,
    focused: bool,
    elements: Vec<String>,
    undo: Vec<Vec<String>>,
    redo: Vec<Vec<String>>,
    revision: u64,
    screen_revision: u64,
    status: String,
    selection: Vec<String>,
    view: &'static str,
}

impl Screen {
    fn fresh(dialog: bool) -> Screen {
        Screen {
            panel: "inspector",
            dialog,
            name: String::new(),
            focused: dialog,
            elements: vec!["Store".into(), "Gateway".into()],
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            screen_revision: 1,
            status: "Ready".into(),
            selection: Vec::new(),
            view: "architecture",
        }
    }
}

pub struct StandIn {
    pub defects: Defects,
    /// Undo and redo are the Operator's (today's Studio, before agents may
    /// undo in a test instance).
    pub undo_refused: bool,
    /// The observation publishes `project.digest`.
    pub digest: bool,
    /// The Create dialog is open when it starts.
    pub dialog_at_start: bool,
    /// Times the screen changes by itself just before an action.
    pub drift: u32,
    pub starts: u32,
    /// Every action asked for, with its agent.
    pub log: Vec<String>,
    flaky_fired: bool,
    alive: bool,
    s: Screen,
}

impl StandIn {
    pub fn new(defects: Defects) -> StandIn {
        StandIn {
            defects,
            undo_refused: false,
            digest: true,
            dialog_at_start: false,
            drift: 0,
            starts: 0,
            log: Vec::new(),
            flaky_fired: false,
            alive: false,
            s: Screen::fresh(false),
        }
    }

    fn controls(&self) -> Vec<Value> {
        let s = &self.s;
        let button = |id: &str, label: &str, region: &str| json!({ "id": id, "label": label, "role": "button", "region": region });
        let mut list = vec![
            json!({ "id": "Message", "label": "Message", "role": "field", "region": "conversation", "value": "" }),
            json!({ "id": "Inspector", "label": "Inspector", "role": "tab", "region": "inspector", "selected": s.panel == "inspector" }),
            json!({ "id": "History", "label": "History", "role": "tab", "region": "inspector", "selected": s.panel == "history" }),
        ];
        if s.panel == "history" {
            list.push(button("checkpoint", "Checkpoint…", "inspector"));
            list.push(button("export", "Export", "inspector"));
            list.push(button(
                "archive-x",
                if self.defects.unlabelled {
                    ""
                } else {
                    "Archive"
                },
                "inspector",
            ));
        } else {
            list.push(button("details", "Details", "inspector"));
        }
        for element in &s.elements {
            list.push(json!({ "id": element, "label": element, "role": "item", "region": "outline", "selected": s.selection.contains(element) }));
        }
        list.extend([
            button("create", "Create part", "title"),
            json!({ "id": "Architecture", "label": "Architecture", "role": "option", "region": "title", "selected": s.view == "architecture" }),
            json!({ "id": "Graph", "label": "Graph", "role": "option", "region": "title", "selected": s.view == "graph" }),
            button("recompute", "Recompute", "title"),
            button("refresh", "Refresh", "title"),
            button("validate", "Validate", "title"),
            button("sync", "Sync", "title"),
            json!({ "id": "lock-it", "label": "Lock", "role": "button", "region": "title", "operatorOnly": true }),
            button("window-close", "Close", "title"),
        ]);
        if s.dialog {
            list.extend([
                json!({ "id": "create-dialog", "label": "Create part", "role": "dialog", "region": "dialog" }),
                json!({ "id": "Name", "label": "Name", "role": "field", "region": "dialog", "value": s.name, "focused": s.focused }),
                button("dialog-cancel", "Cancel", "dialog"),
                button("dialog-confirm", "Create", "dialog"),
            ]);
        }
        list
    }

    fn digest_of(elements: &[String]) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in elements.join("|").bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{hash:016x}")
    }

    fn ok(&self, did: &str, took: u64) -> Value {
        json!({
            "ok": true, "did": did, "screen": "surface", "screenRevision": self.s.screen_revision,
            "dialog": if self.s.dialog { json!("Create") } else { Value::Null },
            "status": self.s.status, "tookMs": took,
        })
    }

    fn refuse(error: &str) -> Value {
        json!({ "ok": false, "error": error })
    }

    fn moved(&mut self) {
        self.s.screen_revision += 1;
    }

    fn confirm(&mut self) {
        let name = self.s.name.trim().to_string();
        if name.is_empty() {
            self.s.status = "A part needs a name".into();
            return;
        }
        if self.s.elements.contains(&name) {
            self.s.status = format!("“{name}” is taken");
            return;
        }
        self.s.undo.push(self.s.elements.clone());
        self.s.redo.clear();
        self.s.elements.push(name.clone());
        self.s.revision += 1;
        self.s.status = format!("Created part {name}");
        self.s.selection = vec![name];
        self.s.dialog = false;
        self.s.focused = false;
        self.moved();
    }

    fn close_dialog(&mut self) {
        if !self.defects.stuck_dialog {
            self.s.dialog = false;
            self.s.focused = false;
            self.moved();
        }
    }

    fn command(&mut self, id: &str) -> Value {
        match id {
            "undo" | "redo" if self.undo_refused => {
                Self::refuse(&format!("refused: `{id}` is the Operator's to use"))
            }
            "undo" => {
                let Some(previous) = self.s.undo.pop() else {
                    return Self::refuse("`undo` is not available: nothing to undo");
                };
                if self.defects.undo {
                    // Does nothing (and forgets the step).
                    return self.ok("command undo", 90);
                }
                self.s
                    .redo
                    .push(std::mem::replace(&mut self.s.elements, previous));
                self.s.revision += 1;
                self.ok("command undo", 90)
            }
            "redo" => {
                let Some(next) = self.s.redo.pop() else {
                    return Self::refuse("`redo` is not available: nothing to redo");
                };
                self.s
                    .undo
                    .push(std::mem::replace(&mut self.s.elements, next));
                self.s.revision += 1;
                self.ok("command redo", 90)
            }
            "graph" => {
                self.s.view = "graph";
                self.ok("command graph", 80)
            }
            "architecture" => {
                self.s.view = "architecture";
                self.ok("command architecture", 80)
            }
            other => Self::refuse(&format!("there is no command `{other}`")),
        }
    }

    fn click(&mut self, id: &str) -> Result<Value, String> {
        let controls = self.controls();
        let Some(control) = controls.iter().find(|c| c["id"] == id) else {
            return Ok(Self::refuse(&format!(
                "stale: no control `{id}` is on screen now; observe again"
            )));
        };
        if control["operatorOnly"] == true {
            return Ok(Self::refuse("refused: locking is the Operator's own"));
        }
        let label = control["label"].as_str().unwrap_or_default().to_string();
        if self.s.dialog && control["region"] != "dialog" {
            // Behind the dialog: the click lands on its backdrop.
            return Ok(self.ok(&format!("click “{id}”"), 100));
        }
        let mut took = 120;
        match id {
            "create" => {
                self.s.dialog = true;
                self.s.name.clear();
                self.s.focused = true;
                self.moved();
            }
            "dialog-cancel" => self.close_dialog(),
            "dialog-confirm" => self.confirm(),
            "Inspector" => self.s.panel = "inspector",
            "History" => self.s.panel = "history",
            "Architecture" => self.s.view = "architecture",
            "Graph" => self.s.view = "graph",
            "checkpoint" => self.s.status = "Checkpoint recorded".into(),
            "details" => self.s.status = "Details shown".into(),
            "archive-x" => self.s.status = "Archived".into(),
            "export" if self.defects.crash => {
                self.alive = false;
                return Err("the Studio closed the connection".into());
            }
            "export" => self.s.status = "Exported".into(),
            "refresh" if self.defects.flaky && !self.flaky_fired => {
                self.flaky_fired = true;
                self.alive = false;
                return Err("the Studio closed the connection".into());
            }
            "refresh" => self.s.status = "Refreshed".into(),
            "recompute" => {
                self.s.status = "Recomputed".into();
                if self.defects.slow {
                    took = 5000;
                }
            }
            "validate" if self.defects.internal_error => {
                self.s.status =
                    "Validation failed: internal error: index out of bounds: the len is 2 but the index is 7".into();
            }
            "validate" => self.s.status = "The model is valid".into(),
            "sync" if self.defects.refuses_offered => {
                return Ok(Self::refuse(&format!("`{label}` is disabled now")));
            }
            "sync" => self.s.status = "Synced".into(),
            element if self.s.elements.iter().any(|e| e == element) => {
                self.s.selection = vec![element.to_string()];
                self.moved();
            }
            _ => {}
        }
        Ok(self.ok(&format!("click “{id}”"), took))
    }
}

impl Instance for StandIn {
    fn observe(&mut self) -> Result<Value, String> {
        if !self.alive {
            return Err("the Studio closed the connection".into());
        }
        let s = &self.s;
        let mut project =
            json!({ "folder": "/stand-in/project", "revision": s.revision, "saved": true });
        if self.digest {
            project["digest"] = json!(Self::digest_of(&s.elements));
        }
        Ok(json!({
            "ok": true,
            "identity": { "instance": format!("stand-in-{}", self.starts), "build": "b1", "commit": "abc1234" },
            "screen": "surface",
            "screenRevision": s.screen_revision,
            "project": project,
            "view": s.view,
            "panels": { "left": "outline", "right": s.panel, "conversation": true },
            "dialog": if s.dialog { json!("Create") } else { Value::Null },
            "approval": null,
            "palette": null,
            "selection": s.selection,
            "status": s.status,
            "controls": self.controls(),
            "commands": [
                { "id": "graph", "label": "Graph view" },
                { "id": "architecture", "label": "Architecture view" },
                { "id": "undo", "label": "Undo", "available": !s.undo.is_empty() },
                { "id": "redo", "label": "Redo", "available": !s.redo.is_empty() },
                { "id": "lock", "label": "Lock or unlock" },
            ],
            "cards": s.elements.iter().map(|e| json!({ "element": format!("Shop::{e}"), "category": "part" })).collect::<Vec<_>>(),
        }))
    }

    fn act(&mut self, act: &Act) -> Result<Value, String> {
        if !self.alive {
            return Err("the Studio closed the connection".into());
        }
        self.log.push(format!("{}: {}", act.agent, act.action));
        if self.drift > 0 {
            self.drift -= 1;
            self.moved();
        }
        if act.observed != self.s.screen_revision {
            return Ok(Self::refuse(
                "stale: the screen changed since you observed it (now: surface); observe again",
            ));
        }
        let action = act.action;
        match action["kind"].as_str().unwrap_or_default() {
            "click" => self.click(action["control"].as_str().unwrap_or_default()),
            "fill" => {
                let id = action["control"].as_str().unwrap_or_default();
                if !(self.s.dialog && id == "Name") {
                    return Ok(Self::refuse(&format!(
                        "stale: no control `{id}` is on screen now; observe again"
                    )));
                }
                self.s.name = action["text"].as_str().unwrap_or_default().to_string();
                self.s.focused = true;
                Ok(self.ok("fill", 150))
            }
            "key" => {
                match action["keys"].as_str().unwrap_or_default() {
                    "escape" if self.s.dialog => self.close_dialog(),
                    "escape" if !self.s.selection.is_empty() => {
                        self.s.selection.clear();
                        self.moved();
                    }
                    "enter" if self.s.dialog && self.s.focused => self.confirm(),
                    _ => {}
                }
                Ok(self.ok("key", 60))
            }
            "command" => Ok(self.command(action["id"].as_str().unwrap_or_default())),
            other => Ok(Self::refuse(&format!("there is no action `{other}`"))),
        }
    }

    fn restart(&mut self) -> Result<(), String> {
        self.starts += 1;
        self.alive = true;
        self.s = Screen::fresh(self.dialog_at_start);
        self.log.push(format!("start {}", self.starts));
        Ok(())
    }

    fn alive(&mut self) -> bool {
        self.alive
    }

    fn folder(&self) -> Option<PathBuf> {
        None
    }
}
