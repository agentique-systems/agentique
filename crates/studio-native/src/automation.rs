//! Scripted UI journeys for Scenario A (`--features automation`). Every
//! action is ordinary input (pointer, keys, text) given to egui; checks read
//! the application state on the following frames.
//!
//! - `a-build --project <new folder>`: creates the project through the New
//!   project dialog, builds part of the URL shortener by hand (parts, ports,
//!   a connection, a rename, a type, a lock, a refused locked change) and
//!   records a checkpoint.
//! - `a-reopen --project <same folder>`: in a new process, checks that
//!   everything is as it was left, including the lock and the checkpoint.
//!
//! Screenshots go only to the `--gallery` directory given on the command line.
use crate::{
    app::StudioApp,
    edit::Dialog,
    targets::{Target, target},
};
use agq_language::{ElementId, ElementKind, Tree};
use agq_studio_scene::{Point, SceneTarget};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Vec2};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioStatus {
    Running,
    Complete,
}

/// Called from `raw_input_hook`. Closes the window when the journey
/// completes; exits with status 2 when a check fails.
pub fn raw_input(app: &StudioApp, ctx: &egui::Context, input: &mut egui::RawInput) {
    let Some(scenario) = &app.args.scenario else {
        return;
    };
    let report = app.args.scenario_report.as_deref();
    let outcome = if scenario == "stress" {
        crate::stress_automation::drive(app, ctx, input, report)
    } else {
        drive(app, ctx, input, scenario, report)
    };
    match outcome {
        Ok(ScenarioStatus::Running) => {}
        Ok(ScenarioStatus::Complete) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        Err(error) => {
            eprintln!("Journey FAILED: {error}");
            std::process::exit(2);
        }
    }
}

const PACKAGE: &str = "UrlShortener";

#[derive(Clone, Debug)]
enum Action {
    Idle,
    Key(Key, Modifiers),
    Text(&'static str),
    /// Click, select all, type.
    Fill(Target, String),
    Click(Target),
    /// Click the card of the element at this path.
    ClickCard(&'static str),
    /// Drag from a port on a card to a port on another card: (card, port).
    DragPort((&'static str, &'static str), (&'static str, &'static str)),
}
impl Action {
    fn frames(&self) -> u64 {
        match self {
            Self::Idle | Self::Key(..) | Self::Text(_) => 1,
            Self::Click(_) | Self::ClickCard(_) => 2,
            Self::Fill(..) => 5,
            Self::DragPort(..) => 12,
        }
    }
    /// Pointer actions wait so egui cannot join them into a double click.
    fn lead(&self) -> u64 {
        match self {
            Self::Click(_) | Self::ClickCard(_) | Self::Fill(..) | Self::DragPort(..) => 25,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug)]
enum Check {
    StartScreen,
    DialogNewProject,
    ProjectName(&'static str),
    ProjectOpen,
    CreateDialog,
    RenameDialog,
    CheckpointDialog,
    NoDialog,
    Exists(&'static str, ElementKind),
    Selected(&'static str),
    Port(&'static str, &'static str),
    Interface(&'static str, &'static str),
    TypedBy(&'static str, &'static str),
    Locked(&'static str),
    LockConfirmation,
    Checkpoints(usize),
    Comparison,
    NothingSelected,
    MoveDialog,
    /// A card with this name shows a problem.
    ProblemShown(&'static str),
    NoProblems,
}

#[derive(Clone, Debug)]
struct Step {
    name: &'static str,
    action: Action,
    check: Check,
    /// Frames to wait after the action before checking.
    settle: u64,
    screenshot: Option<&'static str>,
}
fn step(name: &'static str, action: Action, check: Check) -> Step {
    Step {
        name,
        action,
        check,
        settle: 3,
        screenshot: None,
    }
}

fn create(kind_key: Key, name: &'static str, path: &'static str, kind: ElementKind) -> Vec<Step> {
    vec![
        step(
            "open the create dialog",
            Action::Key(kind_key, Modifiers::NONE),
            Check::CreateDialog,
        ),
        step("type the name", Action::Text(name), Check::CreateDialog),
        step(
            "create",
            Action::Key(Key::Enter, Modifiers::NONE),
            Check::Exists(path, kind),
        ),
    ]
}

fn build(folder: &Path) -> Vec<Step> {
    let mut steps = vec![
        step("start screen", Action::Idle, Check::StartScreen),
        step(
            "New project…",
            Action::Click(Target::Button("New project…")),
            Check::DialogNewProject,
        ),
        step(
            "project name",
            Action::Fill(Target::Field("Project name"), PACKAGE.into()),
            Check::ProjectName(PACKAGE),
        ),
        step(
            "project folder",
            Action::Fill(
                Target::Field("Project folder"),
                folder.display().to_string(),
            ),
            Check::DialogNewProject,
        ),
        step(
            "Create project",
            Action::Click(Target::Button("Create project")),
            Check::ProjectOpen,
        ),
    ];
    for (name, path) in [
        ("api", "UrlShortener::api"),
        ("store", "UrlShortener::store"),
        ("stats", "UrlShortener::stats"),
    ] {
        steps.extend(create(Key::P, name, path, ElementKind::Part));
        steps.push(step(
            "clear the selection",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::NothingSelected,
        ));
    }
    for (card, port, name) in [
        ("UrlShortener::api", "UrlShortener::api::storage", "storage"),
        ("UrlShortener::api", "UrlShortener::api::clicks", "clicks"),
        ("UrlShortener::store", "UrlShortener::store::links", "links"),
        (
            "UrlShortener::stats",
            "UrlShortener::stats::clicks",
            "clicks",
        ),
    ] {
        steps.push(step(
            "select the part",
            Action::ClickCard(card),
            Check::Selected(card),
        ));
        steps.extend(create(Key::O, name, port, ElementKind::Port));
    }
    steps.push(step(
        "clear the selection",
        Action::Key(Key::Escape, Modifiers::NONE),
        Check::NothingSelected,
    ));
    steps.push(Step {
        screenshot: Some("01-parts-and-ports"),
        ..step(
            "drag from api.storage to store.links",
            Action::DragPort(
                ("UrlShortener::api", "UrlShortener::api::storage"),
                ("UrlShortener::store", "UrlShortener::store::links"),
            ),
            Check::Interface("api.storage", "store.links"),
        )
    });
    steps.extend([
        step(
            "select stats",
            Action::ClickCard("UrlShortener::stats"),
            Check::Selected("UrlShortener::stats"),
        ),
        step(
            "F2 renames in place",
            Action::Key(Key::F2, Modifiers::NONE),
            Check::RenameDialog,
        ),
        step(
            "select the old name",
            Action::Key(Key::A, Modifiers::COMMAND),
            Check::RenameDialog,
        ),
        step(
            "type the new name",
            Action::Text("statistics"),
            Check::RenameDialog,
        ),
        step(
            "Enter renames",
            Action::Key(Key::Enter, Modifiers::NONE),
            Check::Exists("UrlShortener::statistics", ElementKind::Part),
        ),
        step(
            "select store",
            Action::ClickCard("UrlShortener::store"),
            Check::Selected("UrlShortener::store"),
        ),
    ]);
    steps.extend(create(
        Key::A,
        "capacity",
        "UrlShortener::store::capacity",
        ElementKind::Attribute,
    ));
    steps.extend([
        step(
            "type Natural in the Inspector's type field",
            Action::Fill(Target::Field("Type"), "Natural".into()),
            Check::NoDialog,
        ),
        step(
            "Enter sets the type",
            Action::Key(Key::Enter, Modifiers::NONE),
            Check::TypedBy("UrlShortener::store::capacity", "Natural"),
        ),
        step(
            "select api",
            Action::ClickCard("UrlShortener::api"),
            Check::Selected("UrlShortener::api"),
        ),
        Step {
            screenshot: Some("02-locked"),
            ..step(
                "L locks api",
                Action::Key(Key::L, Modifiers::NONE),
                Check::Locked("UrlShortener::api"),
            )
        },
        step(
            "rename the locked part",
            Action::Key(Key::F2, Modifiers::NONE),
            Check::RenameDialog,
        ),
        step(
            "select the old name",
            Action::Key(Key::A, Modifiers::COMMAND),
            Check::RenameDialog,
        ),
        step(
            "type a new name",
            Action::Text("gateway"),
            Check::RenameDialog,
        ),
        Step {
            screenshot: Some("03-locked-confirmation"),
            ..step(
                "the change asks for confirmation",
                Action::Key(Key::Enter, Modifiers::NONE),
                Check::LockConfirmation,
            )
        },
        step(
            "Escape refuses; nothing changes",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::Exists("UrlShortener::api", ElementKind::Part),
        ),
        // Confirming a change to the locked part applies it.
        step(
            "create a port on the locked part",
            Action::Key(Key::O, Modifiers::NONE),
            Check::CreateDialog,
        ),
        step(
            "type the port name",
            Action::Text("shorten"),
            Check::CreateDialog,
        ),
        step(
            "the locked part asks first",
            Action::Key(Key::Enter, Modifiers::NONE),
            Check::LockConfirmation,
        ),
        step(
            "Enter confirms the change",
            Action::Key(Key::Enter, Modifiers::NONE),
            Check::Exists("UrlShortener::api::shorten", ElementKind::Port),
        ),
        // A second part with a taken name is flagged at the element.
        step(
            "clear the selection",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::NothingSelected,
        ),
        step(
            "create another part",
            Action::Key(Key::P, Modifiers::NONE),
            Check::CreateDialog,
        ),
        step(
            "with a taken name",
            Action::Text("store"),
            Check::CreateDialog,
        ),
        Step {
            screenshot: Some("03b-problem-at-element"),
            ..step(
                "the problem shows at the element",
                Action::Key(Key::Enter, Modifiers::NONE),
                Check::ProblemShown("store"),
            )
        },
        step(
            "Delete removes the new part",
            Action::Key(Key::Delete, Modifiers::NONE),
            Check::NoProblems,
        ),
        // Move a part into another, then undo and redo the move.
        step(
            "clear the selection",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::NothingSelected,
        ),
    ]);
    steps.extend(create(
        Key::P,
        "cache",
        "UrlShortener::cache",
        ElementKind::Part,
    ));
    steps.extend([
        step(
            "M moves the part",
            Action::Key(Key::M, Modifiers::NONE),
            Check::MoveDialog,
        ),
        step(
            "find the new owner",
            Action::Text("store"),
            Check::MoveDialog,
        ),
        step(
            "Enter moves it into store",
            Action::Key(Key::Enter, Modifiers::NONE),
            Check::Exists("UrlShortener::store::cache", ElementKind::Part),
        ),
        step(
            "Ctrl+Z undoes the move",
            Action::Key(Key::Z, Modifiers::COMMAND),
            Check::Exists("UrlShortener::cache", ElementKind::Part),
        ),
        step(
            "Ctrl+Y redoes it",
            Action::Key(Key::Y, Modifiers::COMMAND),
            Check::Exists("UrlShortener::store::cache", ElementKind::Part),
        ),
        step(
            "Ctrl+S asks for a checkpoint message",
            Action::Key(Key::S, Modifiers::COMMAND),
            Check::CheckpointDialog,
        ),
        step(
            "type the message",
            Action::Text("URL shortener skeleton"),
            Check::CheckpointDialog,
        ),
        Step {
            screenshot: Some("04-checkpoint"),
            ..step(
                "Enter records the checkpoint",
                Action::Key(Key::Enter, Modifiers::NONE),
                Check::Checkpoints(1),
            )
        },
    ]);
    steps
}

fn reopen() -> Vec<Step> {
    vec![
        step("the project opens", Action::Idle, Check::ProjectOpen),
        step(
            "parts",
            Action::Idle,
            Check::Exists("UrlShortener::api", ElementKind::Part),
        ),
        step(
            "renamed part",
            Action::Idle,
            Check::Exists("UrlShortener::statistics", ElementKind::Part),
        ),
        step(
            "ports",
            Action::Idle,
            Check::Port("UrlShortener::store", "links"),
        ),
        step(
            "connection",
            Action::Idle,
            Check::Interface("api.storage", "store.links"),
        ),
        step(
            "type",
            Action::Idle,
            Check::TypedBy("UrlShortener::store::capacity", "Natural"),
        ),
        Step {
            screenshot: Some("05-reopened"),
            ..step("lock", Action::Idle, Check::Locked("UrlShortener::api"))
        },
        step(
            "confirmed, moved and deleted changes",
            Action::Idle,
            Check::Exists("UrlShortener::store::cache", ElementKind::Part),
        ),
        step("no problems left", Action::Idle, Check::NoProblems),
        step("checkpoint", Action::Idle, Check::Checkpoints(1)),
        step(
            "History panel",
            Action::Click(Target::Button("History")),
            Check::Checkpoints(1),
        ),
        step(
            "select the checkpoint",
            Action::Click(Target::Button(crate::history::HISTORY_ROWS[0])),
            Check::Checkpoints(1),
        ),
        Step {
            screenshot: Some("06-what-changed"),
            ..step(
                "show what changed since then",
                Action::Click(Target::Button("Show changes")),
                Check::Comparison,
            )
        },
    ]
}

#[derive(Clone, Serialize)]
struct Report {
    scenario: String,
    passed: bool,
    steps: Vec<String>,
    failure: Option<String>,
    gallery: Vec<String>,
}

#[derive(Clone)]
struct Runner {
    steps: Vec<Step>,
    index: usize,
    age: u64,
    frames: u64,
    time: Option<f64>,
    point: Option<(Pos2, Pos2)>,
    capture: Option<(&'static str, u64)>,
    report: Report,
}

fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    scenario: &str,
    report_path: Option<&Path>,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("agentique-journey");
    let mut runner = match ctx.data(|d| d.get_temp::<Runner>(id)) {
        Some(runner) => runner,
        None => {
            let folder = app
                .args
                .project
                .clone()
                .ok_or("the journeys need --project <folder>")?;
            Runner {
                steps: if scenario == "a-build" {
                    build(&folder)
                } else {
                    reopen()
                },
                index: 0,
                age: 0,
                frames: 0,
                time: None,
                point: None,
                capture: None,
                report: Report {
                    scenario: scenario.into(),
                    passed: false,
                    steps: Vec::new(),
                    failure: None,
                    gallery: Vec::new(),
                },
            }
        }
    };
    let result = runner.advance(app, ctx, input);
    runner.report.passed = matches!(result, Ok(ScenarioStatus::Complete));
    if let Err(error) = &result {
        runner.report.failure = Some(error.clone());
    }
    if let Some(path) = report_path
        && !matches!(result, Ok(ScenarioStatus::Running))
    {
        let bytes = serde_json::to_vec_pretty(&runner.report).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| format!("Cannot write the report: {e}"))?;
    }
    ctx.data_mut(|d| d.insert_temp(id, runner));
    result
}

impl Runner {
    fn advance(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        self.frames += 1;
        if self.frames > 6000 {
            return Err("the journey took more than 6000 frames".into());
        }
        // Only scripted input reaches the app; one steady clock.
        input
            .events
            .retain(|e| matches!(e, Event::Screenshot { .. } | Event::WindowFocused(_)));
        input.focused = true;
        input.modifiers = Modifiers::NONE;
        let origin = *self.time.get_or_insert(input.time.unwrap_or(0.0));
        input.time = Some(origin + self.frames as f64 / 60.0);
        input.predicted_dt = 1.0 / 60.0;
        if let Some((name, waited)) = &mut self.capture {
            *waited += 1;
            if let Some(image) = input.events.iter().find_map(|event| match event {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            }) {
                let directory = app.args.gallery.as_ref().expect("a gallery was requested");
                std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
                let path = directory.join(format!("{}-{name}.png", self.report.scenario));
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                image::save_buffer(
                    &path,
                    &bytes,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                )
                .map_err(|e| format!("Cannot save the screenshot: {e}"))?;
                self.report.gallery.push(path.display().to_string());
                self.capture = None;
            } else if *waited > 120 {
                return Err(format!("screenshot {name} did not arrive"));
            }
            return Ok(ScenarioStatus::Running);
        }
        let Some(step) = self.steps.get(self.index).cloned() else {
            return Ok(ScenarioStatus::Complete);
        };
        let lead = step.action.lead();
        let acting = self.age >= lead && self.age < lead + step.action.frames();
        if acting {
            self.inject(&step.action, self.age - lead, app, ctx, input)
                .map_err(|e| format!("step {} “{}”: {e}", self.index + 1, step.name))?;
        }
        if self.age >= lead + step.action.frames() + step.settle {
            match check(&step.check, app) {
                Ok(()) => {
                    self.report
                        .steps
                        .push(format!("{} · {}", self.index + 1, step.name));
                    self.index += 1;
                    self.age = 0;
                    self.point = None;
                    if let (Some(name), Some(_)) = (step.screenshot, &app.args.gallery) {
                        self.capture = Some((name, 0));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                            egui::UserData::default(),
                        ));
                    }
                    return Ok(ScenarioStatus::Running);
                }
                Err(reason) if self.age > lead + step.action.frames() + 180 => {
                    return Err(format!(
                        "step {} “{}”: {reason} (status: {})",
                        self.index + 1,
                        step.name,
                        app.status
                    ));
                }
                Err(_) => {}
            }
        }
        self.age += 1;
        Ok(ScenarioStatus::Running)
    }

    fn inject(
        &mut self,
        action: &Action,
        frame: u64,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<(), String> {
        match action {
            Action::Idle => {}
            Action::Key(k, m) => key(input, *k, *m),
            Action::Text(text) => input.events.push(Event::Text((*text).into())),
            Action::Click(t) => {
                let p = target(ctx, *t)?.center();
                click(input, p, frame == 0);
            }
            Action::ClickCard(path) => {
                let p = match self.point {
                    Some((p, _)) => p,
                    None => {
                        let p = card_point(app, ctx, path)?;
                        self.point = Some((p, p));
                        p
                    }
                };
                click(input, p, frame == 0);
            }
            Action::Fill(t, text) => match frame {
                0 | 1 => {
                    let p = target(ctx, *t)?.center();
                    click(input, p, frame == 0);
                }
                2 => key(input, Key::A, Modifiers::COMMAND),
                3 => input.events.push(Event::Text(text.clone())),
                _ => {}
            },
            Action::DragPort(from, to) => {
                let (a, b) = match self.point {
                    Some(points) => points,
                    None => {
                        let points = (port_point(app, ctx, *from)?, port_point(app, ctx, *to)?);
                        self.point = Some(points);
                        points
                    }
                };
                let last = action.frames() - 1;
                if frame == 0 {
                    input.events.push(Event::PointerMoved(a));
                    input.events.push(Event::PointerButton {
                        pos: a,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    });
                } else if frame < last {
                    let t = frame as f32 / (last - 1) as f32;
                    input.events.push(Event::PointerMoved(a + (b - a) * t));
                } else {
                    input.events.push(Event::PointerMoved(b));
                    input.events.push(Event::PointerButton {
                        pos: b,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    });
                }
            }
        }
        Ok(())
    }
}

fn key(input: &mut egui::RawInput, key: Key, mut modifiers: Modifiers) {
    if modifiers.command {
        modifiers.ctrl = !cfg!(target_os = "macos");
        modifiers.mac_cmd = cfg!(target_os = "macos");
    }
    input.modifiers = modifiers;
    for pressed in [true, false] {
        input.events.push(Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        });
    }
}

fn click(input: &mut egui::RawInput, position: Pos2, pressed: bool) {
    input.events.push(Event::PointerMoved(position));
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

fn tree(app: &StudioApp) -> Result<&Tree, String> {
    app.project
        .as_ref()
        .map(|p| p.state().tree())
        .ok_or_else(|| "no project is open".to_string())
}

fn find(app: &StudioApp, path: &str) -> Result<ElementId, String> {
    tree(app)?
        .find(path)
        .ok_or_else(|| format!("{path} does not exist"))
}

fn screen(app: &StudioApp, ctx: &egui::Context, world: Point) -> Result<Pos2, String> {
    let viewport = target(ctx, Target::Viewport)?;
    let local = app.camera.world_to_screen(world);
    let p = viewport.min + Vec2::new(local.x, local.y);
    if !viewport.shrink(4.0).contains(p) {
        return Err(format!("{p:?} is outside the Surface {viewport:?}"));
    }
    Ok(p)
}

fn card_point(app: &StudioApp, ctx: &egui::Context, path: &str) -> Result<Pos2, String> {
    let id = find(app, path)?;
    let node = app
        .lookup
        .node(&app.scene, id)
        .ok_or_else(|| format!("{path} has no card on the Surface"))?;
    let point = if node.is_container {
        Point::new(node.bounds.center().x, node.bounds.min.y + 26.0)
    } else {
        Point::new(node.bounds.center().x, node.bounds.min.y + 40.0)
    };
    screen(app, ctx, point)
}

fn port_point(
    app: &StudioApp,
    ctx: &egui::Context,
    (card, port): (&str, &str),
) -> Result<Pos2, String> {
    let (card, port) = (find(app, card)?, find(app, port)?);
    let shown = app
        .lookup
        .port(&app.scene, card, port)
        .ok_or("the port is not shown on the card")?;
    screen(app, ctx, shown.position)
}

fn check(check: &Check, app: &StudioApp) -> Result<(), String> {
    let fail = |message: &str| Err(message.to_string());
    match check {
        Check::StartScreen => {
            if app.project.is_some() {
                return fail("a project is already open");
            }
        }
        Check::DialogNewProject => {
            if !matches!(app.dialog, Some(Dialog::NewProject { .. })) {
                return fail("the New project dialog is not open");
            }
        }
        Check::ProjectName(expected) => match &app.dialog {
            Some(Dialog::NewProject { name, .. }) if name == expected => {}
            _ => return fail("the project name was not typed"),
        },
        Check::ProjectOpen => {
            find(app, PACKAGE)?;
        }
        Check::CreateDialog => {
            if !matches!(app.dialog, Some(Dialog::Create { .. })) {
                return fail("the Create dialog is not open");
            }
        }
        Check::RenameDialog => {
            if !matches!(app.dialog, Some(Dialog::Rename { .. })) {
                return fail("the name is not being edited");
            }
        }
        Check::CheckpointDialog => {
            if !matches!(app.dialog, Some(Dialog::Checkpoint { .. })) {
                return fail("the Checkpoint dialog is not open");
            }
        }
        Check::NoDialog => {
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::Exists(path, kind) => {
            let id = find(app, path)?;
            if tree(app)?[id].kind != *kind {
                return fail("the element has the wrong kind");
            }
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::Selected(path) => {
            let id = find(app, path)?;
            if app.selection.primary != Some(SceneTarget::Node(id))
                && app.selection.primary != Some(SceneTarget::Container(id))
            {
                return fail("the card is not selected");
            }
        }
        Check::NothingSelected => {
            if app.selection.primary.is_some() {
                return fail("something is still selected");
            }
        }
        Check::Port(card, port) => {
            let tree = tree(app)?;
            let card = find(app, card)?;
            if !tree[card].children().iter().any(|c| {
                tree[*c].kind == ElementKind::Port && tree.effective_name(*c) == Some(port)
            }) {
                return fail("the port does not exist");
            }
        }
        Check::Interface(from, to) => {
            let tree = tree(app)?;
            let found = tree.walk().into_iter().any(|id| {
                let e = &tree[id];
                e.kind == ElementKind::Interface
                    && e.ends.len() == 2
                    && e.ends[0].is_linked()
                    && e.ends[1].is_linked()
                    && e.ends[0].to_string() == *from
                    && e.ends[1].to_string() == *to
            });
            if !found {
                return fail("no linked interface connects the two ports");
            }
        }
        Check::TypedBy(path, type_name) => {
            let tree = tree(app)?;
            let id = find(app, path)?;
            let typed = tree[id]
                .typed_by
                .first()
                .is_some_and(|r| r.is_linked() && r.last_name() == *type_name);
            if !typed {
                return fail("the type is not set and linked");
            }
        }
        Check::Locked(path) => {
            let id = find(app, path)?;
            let project = app.project.as_ref().expect("checked by find");
            if !project.state().locks().contains(&id) {
                return fail("the element is not locked");
            }
        }
        Check::LockConfirmation => {
            if !matches!(app.dialog, Some(Dialog::Confirm { .. })) {
                return fail("no confirmation was asked");
            }
        }
        Check::Checkpoints(count) => {
            let project = app.project.as_ref().ok_or("no project is open")?;
            let found = project.checkpoints().map_err(|e| e.to_string())?.len();
            if found < *count {
                return Err(format!("{found} checkpoint(s), expected {count}"));
            }
        }
        Check::Comparison => {
            if app.comparison.is_none() {
                return fail("no comparison is shown");
            }
        }
        Check::MoveDialog => {
            if !matches!(app.dialog, Some(Dialog::MoveTo { .. })) {
                return fail("the Move to dialog is not open");
            }
        }
        Check::ProblemShown(name) => {
            let shown = app
                .scene
                .nodes
                .iter()
                .any(|n| n.semantic.name == *name && n.semantic.problems > 0);
            if !shown {
                return fail("no card with that name shows a problem");
            }
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::NoProblems => {
            let project = app.project.as_ref().ok_or("no project is open")?;
            if let Some(problem) = project.state().diagnostics().first() {
                return Err(format!("a problem remains: {}", problem.message));
            }
        }
    }
    Ok(())
}
