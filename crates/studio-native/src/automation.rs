//! Opt-in native input smoke scenario. Every interaction enters egui RawInput;
//! assertions observe the resulting application state on subsequent real frames.
//! This module refuses live model bindings and cannot authorize a service write.
use crate::{
    app::{ComparisonMode, StudioApp},
    commands::{self, CommandId},
    navigation::World,
};
use agq_kernel::ElementId;
use agq_modeling_view::ViewOrigin;
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{DiffMark, Point, SceneTarget, fixtures};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use serde::Serialize;
use std::path::Path;

/// UI geometry recorded by ordinary widget construction, not alternate handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Viewport,
    PaletteInput,
    CandidateName,
    CandidatePrepare,
    HistoryRevision(ProjectRevisionId),
}
pub fn record(ctx: &egui::Context, target: Target, rect: Rect) {
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new(("native-interaction-target", target)), rect)
    });
}
pub(crate) fn target(ctx: &egui::Context, key: Target) -> Result<Rect, String> {
    ctx.data(|data| data.get_temp::<Rect>(egui::Id::new(("native-interaction-target", key))))
        .filter(|rect| rect.is_finite() && rect.is_positive())
        .ok_or_else(|| format!("UI geometry for {key:?} has not been recorded"))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioStatus {
    Running,
    Complete,
}

/// Call only from `eframe::App::raw_input_hook`. A returned error must cause a
/// nonzero process exit. The report is written as failed before returning it.
pub fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    scenario: &str,
    report_path: &Path,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("agentique-native-input-scenario");
    let mut runner = ctx
        .data(|data| data.get_temp::<Runner>(id))
        .unwrap_or_else(|| Runner::new(scenario));
    let result = runner.advance(app, ctx, input);
    match &result {
        Ok(ScenarioStatus::Complete) => runner.report.outcome = "passed".into(),
        Ok(ScenarioStatus::Running) => {}
        Err(error) => {
            runner.report.outcome = "failed".into();
            runner.report.failure = Some(error.clone());
        }
    }
    runner.report.passed = matches!(result, Ok(ScenarioStatus::Complete));
    // Intermediate evidence always says running/failed, never a premature pass.
    if runner.report_dirty || !matches!(result, Ok(ScenarioStatus::Running)) {
        if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create scenario report directory: {e}"))?;
        }
        let bytes = serde_json::to_vec_pretty(&runner.report).map_err(|e| e.to_string())?;
        std::fs::write(report_path, bytes)
            .map_err(|e| format!("Cannot write scenario report: {e}"))?;
        runner.report_dirty = false;
    }
    ctx.data_mut(|data| data.insert_temp(id, runner));
    if matches!(result, Ok(ScenarioStatus::Running)) {
        ctx.request_repaint();
    }
    result
}

#[derive(Clone, Debug)]
enum Action {
    Idle,
    ClickNode(u128, bool),
    DoubleClickContainer(u128),
    ClickNamedNode(&'static str),
    ClickDerivedEdge,
    Key(Key, Modifiers),
    Pan,
    Wheel,
    Palette(&'static str),
    PrepareNamedPart(&'static str),
    ClickTarget(Target),
}
impl Action {
    fn lead_frames(&self) -> u64 {
        // egui counts double/triple clicks globally by time, even across
        // different widgets. Separate independent semantic gestures while
        // keeping the two clicks inside DoubleClickContainer deliberately close.
        match self {
            Self::ClickNode(..)
            | Self::DoubleClickContainer(..)
            | Self::ClickNamedNode(..)
            | Self::ClickDerivedEdge
            | Self::ClickTarget(..) => 40,
            _ => 0,
        }
    }
    fn frames(&self) -> u64 {
        match self {
            Self::Idle | Self::Key(..) | Self::Wheel => 1,
            Self::ClickNode(..)
            | Self::ClickNamedNode(..)
            | Self::ClickDerivedEdge
            | Self::ClickTarget(..) => 2,
            Self::DoubleClickContainer(..) | Self::Pan => 4,
            Self::Palette(..) => 12,
            Self::PrepareNamedPart(..) => 6,
        }
    }
}
#[derive(Clone, Debug)]
enum Check {
    Ready,
    Selected(u128),
    MultiSelected,
    SelectionEmpty,
    Focused(u128),
    RootFocus,
    Panned,
    ZoomAnchored,
    World(World),
    DerivedSelected,
    ExplainOpen,
    ExplainClosed,
    Dependencies,
    Diff,
    CreateDialog,
    Candidate,
    NamedSelected(&'static str),
    Disabled(CommandId),
    PaletteClosed,
    Cancelled,
    Revision(ProjectRevisionId),
    ContrastChanged,
    ThemeChanged,
    ReducedMotion,
}
#[derive(Clone, Debug)]
struct Step {
    name: &'static str,
    action: Action,
    check: Check,
    settle: u64,
}
fn vertical() -> Vec<Step> {
    let step = |name, action, check| Step {
        name,
        action,
        check,
        settle: 3,
    };
    vec![
        Step {
            name: "native fixture and GPU ready",
            action: Action::Idle,
            check: Check::Ready,
            settle: 12,
        },
        step(
            "enable reduced-motion keyboard path",
            Action::Palette("Toggle reduced motion"),
            Check::ReducedMotion,
        ),
        step(
            "select ModelRepository by scene geometry",
            Action::ClickNode(21, false),
            Check::Selected(21),
        ),
        step(
            "shift-click adds ModelingService",
            Action::ClickNode(22, true),
            Check::MultiSelected,
        ),
        step(
            "Escape clears scene selection",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::SelectionEmpty,
        ),
        step(
            "double-click focuses ModelingPlatform",
            Action::DoubleClickContainer(2),
            Check::Focused(2),
        ),
        step(
            "Alt-Up navigates to owner context",
            Action::Key(Key::ArrowUp, Modifiers::ALT),
            Check::RootFocus,
        ),
        step(
            "select subsystem header",
            Action::ClickNode(2, false),
            Check::Selected(2),
        ),
        step(
            "F focuses selected subsystem",
            Action::Key(Key::F, Modifiers::NONE),
            Check::Focused(2),
        ),
        step(
            "Alt-Left restores root navigation",
            Action::Key(Key::ArrowLeft, Modifiers::ALT),
            Check::RootFocus,
        ),
        step("drag empty canvas pans camera", Action::Pan, Check::Panned),
        Step {
            name: "wheel zoom preserves pointer anchor",
            action: Action::Wheel,
            check: Check::ZoomAnchored,
            settle: 15,
        },
        step(
            "2 opens Graph World",
            Action::Key(Key::Num2, Modifiers::NONE),
            Check::World(World::Graph),
        ),
        step(
            "select derived relationship geometry",
            Action::ClickDerivedEdge,
            Check::DerivedSelected,
        ),
        step(
            "E opens semantic Explain",
            Action::Key(Key::E, Modifiers::NONE),
            Check::ExplainOpen,
        ),
        step(
            "Escape closes Explain",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::ExplainClosed,
        ),
        step(
            "select repository in Graph World",
            Action::ClickNode(21, false),
            Check::Selected(21),
        ),
        step(
            "command palette shows dependencies",
            Action::Palette("Show dependencies"),
            Check::Dependencies,
        ),
        step(
            "command palette compares revisions",
            Action::Palette("Compare with parent"),
            Check::Diff,
        ),
        step(
            "1 opens System World comparison",
            Action::Key(Key::Num1, Modifiers::NONE),
            Check::World(World::System),
        ),
        step(
            "select candidate parent",
            Action::ClickNode(2, false),
            Check::Selected(2),
        ),
        step(
            "palette opens nested-part dialog",
            Action::Palette("Create nested PartUsage"),
            Check::CreateDialog,
        ),
        step(
            "text entry prepares candidate preview",
            Action::PrepareNamedPart("ScenarioNestedPart"),
            Check::Candidate,
        ),
        step(
            "candidate element is selectable",
            Action::ClickNamedNode("ScenarioNestedPart"),
            Check::NamedSelected("ScenarioNestedPart"),
        ),
        step(
            "fixture validation stays disabled",
            Action::Palette("Validate candidate"),
            Check::Disabled(CommandId::Validate),
        ),
        step(
            "close disabled validation palette",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::PaletteClosed,
        ),
        step(
            "fixture commit stays disabled",
            Action::Palette("Commit validated candidate"),
            Check::Disabled(CommandId::Commit),
        ),
        step(
            "close disabled commit palette",
            Action::Key(Key::Escape, Modifiers::NONE),
            Check::PaletteClosed,
        ),
        step(
            "cancel drops candidate and stale selection",
            Action::Palette("Cancel candidate"),
            Check::Cancelled,
        ),
        step(
            "4 opens immutable history",
            Action::Key(Key::Num4, Modifiers::NONE),
            Check::World(World::History),
        ),
        step(
            "history baseline changes entire revision context",
            Action::ClickTarget(Target::HistoryRevision(fixtures::revision())),
            Check::Revision(fixtures::revision()),
        ),
        step(
            "high contrast command changes theme",
            Action::Palette("Toggle high contrast"),
            Check::ContrastChanged,
        ),
        step(
            "light-dark command changes theme",
            Action::Palette("Switch light"),
            Check::ThemeChanged,
        ),
        step(
            "restore theme through same command path",
            Action::Palette("Switch light"),
            Check::ThemeChanged,
        ),
        step(
            "restore ordinary contrast",
            Action::Palette("Toggle high contrast"),
            Check::ContrastChanged,
        ),
        step(
            "return to System World",
            Action::Key(Key::Num1, Modifiers::NONE),
            Check::World(World::System),
        ),
        step(
            "final engineering selection",
            Action::ClickNode(21, false),
            Check::Selected(21),
        ),
    ]
}

#[derive(Clone, Debug, Serialize)]
struct Snapshot {
    frame: u64,
    world: String,
    revision: String,
    selection_revision: String,
    selected: Vec<String>,
    focus: Option<String>,
    camera_center: [f32; 2],
    zoom: f32,
    nodes: usize,
    edges: usize,
    dependency_elements: Option<Vec<String>>,
    agent_overlay: bool,
    added_elements: Vec<String>,
    removed_elements: Vec<String>,
    candidate: bool,
    candidate_phase: Option<String>,
    palette: bool,
    palette_query: String,
    create_dialog: bool,
    explanation_open: bool,
    dark: bool,
    high_contrast: bool,
    reduced_motion: bool,
    status: String,
}
impl Snapshot {
    fn of(app: &StudioApp) -> Self {
        Self {
            frame: app.frame_number,
            world: format!("{:?}", app.world),
            revision: app.scene.revision_id.to_string(),
            selection_revision: app.selection.revision.to_string(),
            selected: app
                .selection
                .targets
                .iter()
                .map(|t| format!("{t:?}"))
                .collect(),
            focus: app.focus.map(|id| id.to_string()),
            camera_center: [app.camera.center.x, app.camera.center.y],
            zoom: app.camera.zoom,
            nodes: app.scene.nodes.len(),
            edges: app.scene.edges.len(),
            dependency_elements: app
                .dependencies
                .as_ref()
                .map(|ids| ids.iter().map(ToString::to_string).collect()),
            agent_overlay: app.show_agent,
            added_elements: app
                .scene
                .nodes
                .iter()
                .filter(|n| n.diff == DiffMark::Added)
                .map(|n| n.semantic.name.clone())
                .collect(),
            removed_elements: app
                .scene
                .nodes
                .iter()
                .filter(|n| n.diff == DiffMark::Removed)
                .map(|n| n.semantic.name.clone())
                .collect(),
            candidate: app.candidate.is_some(),
            candidate_phase: app
                .candidate
                .as_ref()
                .and_then(|c| c.phase.as_ref().map(|p| format!("{p:?}"))),
            palette: app.palette,
            palette_query: app.palette_query.clone(),
            create_dialog: app.create_dialog,
            explanation_open: app.show_explain,
            dark: app.theme.dark,
            high_contrast: app.theme.contrast,
            reduced_motion: app.reduced_motion,
            status: app.status.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
struct InputEvidence {
    frame: u64,
    palette_query_before: String,
    keyboard_focus: Option<String>,
    events: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
struct AssertionEvidence {
    name: String,
    expected: String,
    passed: bool,
    before: Snapshot,
    after: Snapshot,
    inputs: Vec<InputEvidence>,
}
#[derive(Clone, Debug, Serialize)]
struct Report {
    format: &'static str,
    scenario: String,
    scope: &'static str,
    outcome: String,
    passed: bool,
    adapter: String,
    assertions: Vec<AssertionEvidence>,
    failure: Option<String>,
}
#[derive(Clone, Debug)]
struct Runner {
    report: Report,
    steps: Vec<Step>,
    index: usize,
    age: u64,
    total_frames: u64,
    before: Option<Snapshot>,
    point: Option<Pos2>,
    anchor: Option<Point>,
    events: Vec<InputEvidence>,
    report_dirty: bool,
    time_origin: Option<f64>,
}
impl Runner {
    fn new(scenario: &str) -> Self {
        Self {
            report: Report {
                format: "agentique-native-interaction/1",
                scenario: scenario.into(),
                scope: "Actual native egui input over deterministic visual fixture; no semantic validation or durable commit acceptance",
                outcome: "running".into(),
                passed: false,
                adapter: String::new(),
                assertions: vec![],
                failure: None,
            },
            steps: vertical(),
            index: 0,
            age: 0,
            total_frames: 0,
            before: None,
            point: None,
            anchor: None,
            events: vec![],
            report_dirty: true,
            time_origin: None,
        }
    }
    fn advance(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        if self.report.scenario != "vertical" {
            return Err(format!("Unknown native scenario {}", self.report.scenario));
        }
        if app.fixture.as_deref() != Some("architecture")
            || app.binding.is_some()
            || app.branch.is_some()
        {
            return Err("Native input scenario requires --fixture architecture and refuses every live service binding".into());
        }
        self.report.adapter.clone_from(&app.adapter);
        if self.index == self.steps.len() {
            return Ok(ScenarioStatus::Complete);
        }
        self.total_frames += 1;
        if self.total_frames > 2400 {
            return Err("Native scenario exceeded its 2400-frame global deadline".into());
        }
        // Ignore human keyboard/pointer interference while this explicit scenario
        // owns input. Keep screenshot delivery and window lifecycle events.
        input
            .events
            .retain(|e| matches!(e, Event::Screenshot { .. } | Event::WindowFocused(_)));
        input.focused = true;
        input.modifiers = Modifiers::NONE;
        let origin = *self.time_origin.get_or_insert(input.time.unwrap_or(0.0));
        input.time = Some(origin + self.total_frames as f64 / 60.0);
        input.predicted_dt = 1.0 / 60.0;
        let step = self.steps[self.index].clone();
        if self.age == 0 {
            self.before = Some(Snapshot::of(app));
            self.events.clear();
            self.point = None;
            self.anchor = None;
        }
        let before = self.before.as_ref().expect("step snapshot initialized");
        if self.age >= step.action.lead_frames() + step.action.frames() + step.settle {
            let passed = check(&step.check, app, before, self.anchor, self.point, ctx);
            if passed || self.age > step.action.lead_frames() + step.action.frames() + 180 {
                self.report.assertions.push(AssertionEvidence {
                    name: step.name.into(),
                    expected: format!("{:?}", step.check),
                    passed,
                    before: before.clone(),
                    after: Snapshot::of(app),
                    inputs: self.events.clone(),
                });
                self.report_dirty = true;
                if !passed {
                    return Err(format!(
                        "Native interaction assertion failed at step {} '{}': expected {:?}; status={}",
                        self.index + 1,
                        step.name,
                        step.check,
                        app.status
                    ));
                }
                self.index += 1;
                self.age = 0;
                return Ok(if self.index == self.steps.len() {
                    ScenarioStatus::Complete
                } else {
                    ScenarioStatus::Running
                });
            }
        }
        if self.age >= step.action.lead_frames()
            && self.age - step.action.lead_frames() < step.action.frames()
        {
            let start = input.events.len();
            inject(
                &step.action,
                self.age - step.action.lead_frames(),
                app,
                ctx,
                input,
                &mut self.point,
                &mut self.anchor,
            )
            .map_err(|error| format!("Step {} '{}': {error}", self.index + 1, step.name))?;
            if input.events.len() > start {
                self.events.push(InputEvidence {
                    frame: app.frame_number,
                    palette_query_before: app.palette_query.clone(),
                    keyboard_focus: ctx
                        .memory(|memory| memory.focused().map(|id| format!("{id:?}"))),
                    events: input.events[start..]
                        .iter()
                        .map(|e| format!("{e:?}"))
                        .collect(),
                });
            }
        }
        self.age += 1;
        Ok(ScenarioStatus::Running)
    }
}

fn key(input: &mut egui::RawInput, key: Key, mut modifiers: Modifiers) {
    // Match the actual native platform modifier as well as egui's logical
    // command bit; text widgets may inspect Ctrl/mac_cmd directly.
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
fn click(input: &mut egui::RawInput, position: Pos2, pressed: bool, modifiers: Modifiers) {
    input.modifiers = modifiers;
    input.events.push(Event::PointerMoved(position));
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers,
    });
}
fn screen(app: &StudioApp, ctx: &egui::Context, world: Point) -> Result<Pos2, String> {
    let viewport = target(ctx, Target::Viewport)?;
    let local = app.camera.world_to_screen(world);
    let p = viewport.min + Vec2::new(local.x, local.y);
    if !viewport.contains(p) {
        return Err(format!(
            "Semantic click point {p:?} lies outside viewport {viewport:?}"
        ));
    }
    Ok(p)
}
fn node_point(app: &StudioApp, ctx: &egui::Context, id: ElementId) -> Result<Pos2, String> {
    let node = app
        .lookup
        .node(&app.scene, id)
        .ok_or_else(|| format!("Semantic node {id} is absent"))?;
    let point = if node.is_container {
        Point::new(node.bounds.center().x, node.bounds.min.y + 26.0)
    } else {
        node.bounds.center()
    };
    screen(app, ctx, point)
}
fn derived_point(app: &StudioApp, ctx: &egui::Context) -> Result<Pos2, String> {
    for edge in app
        .scene
        .edges
        .iter()
        .filter(|e| e.semantic.origin == ViewOrigin::Derived)
    {
        for pair in edge.points.windows(2) {
            for t in [0.5, 0.25, 0.75] {
                let p = Point::new(
                    pair[0].x + (pair[1].x - pair[0].x) * t,
                    pair[0].y + (pair[1].y - pair[0].y) * t,
                );
                if app.spatial.hit_test(p, 6.0 / app.camera.zoom)
                    == Some(SceneTarget::Edge(edge.semantic.id.clone()))
                    && let Ok(screen) = screen(app, ctx, p)
                {
                    return Ok(screen);
                }
            }
        }
    }
    Err("No unoccluded visible derived-edge segment was available for pointer selection".into())
}
fn inject(
    action: &Action,
    frame: u64,
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    point: &mut Option<Pos2>,
    anchor: &mut Option<Point>,
) -> Result<(), String> {
    match action {
        Action::Idle => {}
        Action::Key(k, m) => key(input, *k, *m),
        Action::ClickNode(id, shift) => {
            let p = if let Some(p) = *point {
                p
            } else {
                let p = node_point(app, ctx, fixtures::id(*id))?;
                *point = Some(p);
                p
            };
            click(
                input,
                p,
                frame == 0,
                if *shift {
                    Modifiers::SHIFT
                } else {
                    Modifiers::NONE
                },
            );
        }
        Action::DoubleClickContainer(id) => {
            let p = if let Some(p) = *point {
                p
            } else {
                let p = node_point(app, ctx, fixtures::id(*id))?;
                *point = Some(p);
                p
            };
            click(input, p, frame % 2 == 0, Modifiers::NONE);
        }
        Action::ClickNamedNode(name) => {
            let p = if let Some(p) = *point {
                p
            } else {
                let id = app
                    .scene
                    .nodes
                    .iter()
                    .find(|n| n.semantic.name == *name)
                    .map(|n| n.id())
                    .ok_or_else(|| format!("Expected named scene node {name} is absent"))?;
                let p = node_point(app, ctx, id)?;
                *point = Some(p);
                p
            };
            click(input, p, frame == 0, Modifiers::NONE);
        }
        Action::ClickDerivedEdge => {
            let p = if let Some(p) = *point {
                p
            } else {
                let p = derived_point(app, ctx)?;
                *point = Some(p);
                p
            };
            click(input, p, frame == 0, Modifiers::NONE);
        }
        Action::ClickTarget(which) => {
            let p = if let Some(p) = *point {
                p
            } else {
                let p = target(ctx, *which)?.center();
                *point = Some(p);
                p
            };
            click(input, p, frame == 0, Modifiers::NONE);
        }
        Action::Pan => {
            let p = if let Some(p) = *point {
                p
            } else {
                let p = target(ctx, Target::Viewport)?.left_top() + Vec2::new(22.0, 22.0);
                *point = Some(p);
                p
            };
            match frame {
                0 => click(input, p, true, Modifiers::NONE),
                1 | 2 => input.events.push(Event::PointerMoved(
                    p + Vec2::new(frame as f32 * 40.0, frame as f32 * 24.0),
                )),
                _ => click(input, p + Vec2::new(80.0, 48.0), false, Modifiers::NONE),
            }
        }
        Action::Wheel => {
            let viewport = target(ctx, Target::Viewport)?;
            // An off-center anchor catches implementations that accidentally
            // zoom about the camera center instead of the actual pointer.
            let p = viewport.min + Vec2::new(viewport.width() * 0.70, viewport.height() * 0.38);
            *point = Some(p);
            *anchor = Some(
                app.camera
                    .screen_to_world(Point::new(p.x - viewport.min.x, p.y - viewport.min.y)),
            );
            input.events.push(Event::PointerMoved(p));
            input.events.push(Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, 140.0),
                modifiers: Modifiers::NONE,
            });
        }
        Action::Palette(query) => match frame {
            0 => key(input, Key::K, Modifiers::COMMAND),
            // Native floating windows need their ordinary sizing/fade passes
            // before their previous-frame hit geometry becomes interactive.
            5 | 6 => click(
                input,
                target(ctx, Target::PaletteInput)?.center(),
                frame == 5,
                Modifiers::NONE,
            ),
            7 => key(input, Key::A, Modifiers::COMMAND),
            8 => input.events.push(Event::Text((*query).into())),
            10 => {
                if app.palette_query != *query {
                    return Err(format!(
                        "Palette text input did not retain query: expected {query:?}, observed {:?}, focused {:?}",
                        app.palette_query,
                        ctx.memory(|memory| memory.focused())
                    ));
                }
                key(input, Key::Enter, Modifiers::NONE);
            }
            _ => {}
        },
        Action::PrepareNamedPart(name) => match frame {
            0 | 1 => click(
                input,
                target(ctx, Target::CandidateName)?.center(),
                frame == 0,
                Modifiers::NONE,
            ),
            2 => key(input, Key::A, Modifiers::COMMAND),
            3 => input.events.push(Event::Text((*name).into())),
            _ => click(
                input,
                target(ctx, Target::CandidatePrepare)?.center(),
                frame == 4,
                Modifiers::NONE,
            ),
        },
    }
    Ok(())
}
fn check(
    check: &Check,
    app: &StudioApp,
    before: &Snapshot,
    anchor: Option<Point>,
    pointer: Option<Pos2>,
    ctx: &egui::Context,
) -> bool {
    match check {
        Check::Ready => {
            app.ready
                && !app.fit_pending
                && app.frame_number >= 12
                && app.gpu_stats.lock().is_ok_and(|s| s.draw_calls > 0)
        }
        Check::Selected(id) => {
            app.selected_element() == Some(fixtures::id(*id)) && app.selection.targets.len() == 1
        }
        Check::MultiSelected => {
            app.selection.targets.len() == 2
                && app.selection.contains(fixtures::id(21))
                && app.selection.contains(fixtures::id(22))
        }
        Check::SelectionEmpty => {
            app.selection.targets.is_empty() && app.selection.primary.is_none()
        }
        Check::Focused(id) => {
            app.focus == Some(fixtures::id(*id))
                && app.world == World::System
                && app.scene.nodes.iter().all(|n| {
                    n.id() == fixtures::id(*id) || n.semantic.owner == Some(fixtures::id(*id))
                })
        }
        Check::RootFocus => app.focus.is_none() && app.scene.nodes.len() == 12,
        Check::Panned => {
            Point::new(before.camera_center[0], before.camera_center[1]).distance(app.camera.center)
                > 20.0
                && (before.zoom - app.camera.zoom).abs() < 0.001
        }
        Check::ZoomAnchored => {
            if let (Some(anchor), Some(pointer), Ok(viewport)) =
                (anchor, pointer, target(ctx, Target::Viewport))
            {
                let after = app.camera.screen_to_world(Point::new(
                    pointer.x - viewport.min.x,
                    pointer.y - viewport.min.y,
                ));
                app.camera.zoom > before.zoom + 0.01 && anchor.distance(after) < 0.1
            } else {
                false
            }
        }
        Check::World(world) => app.world == *world,
        Check::DerivedSelected => app.selection.primary.as_ref().is_some_and(|t| match t {
            SceneTarget::Edge(id) => app
                .lookup
                .edge(&app.scene, id)
                .is_some_and(|e| e.semantic.origin == ViewOrigin::Derived),
            _ => false,
        }),
        Check::ExplainOpen => app.show_explain,
        Check::ExplainClosed => !app.show_explain,
        Check::Dependencies => {
            app.world == World::Graph
                && app.dependencies.as_ref().is_some_and(|ids| {
                    ids.contains(&fixtures::id(21))
                        && ids.contains(&fixtures::id(11))
                        && ids.contains(&fixtures::id(31))
                })
                && app.show_agent
        }
        Check::Diff => {
            app.comparison == ComparisonMode::Diff
                && [DiffMark::Added, DiffMark::Removed, DiffMark::Changed]
                    .into_iter()
                    .all(|mark| app.scene.nodes.iter().any(|n| n.diff == mark))
                && app.selection.revision == app.scene.revision_id
        }
        Check::CreateDialog => app.create_dialog && !app.palette,
        Check::Candidate => {
            app.candidate.as_ref().is_some_and(|c| {
                c.id.is_none()
                    && c.phase.is_none()
                    && c.after.nodes.iter().any(|n| n.name == "ScenarioNestedPart")
                    && !c
                        .before
                        .nodes
                        .iter()
                        .any(|n| n.name == "ScenarioNestedPart")
            }) && app.comparison == ComparisonMode::Diff
                && !app.create_dialog
        }
        Check::NamedSelected(name) => app
            .selected_element()
            .and_then(|id| app.lookup.node(&app.scene, id))
            .is_some_and(|n| n.semantic.name == *name),
        Check::Disabled(command) => {
            app.palette
                && commands::unavailable(*command, &app.context()).is_some()
                && app
                    .candidate
                    .as_ref()
                    .is_some_and(|c| c.id.is_none() && c.phase.is_none())
                && app.pending.is_empty()
        }
        Check::PaletteClosed => !app.palette,
        Check::Cancelled => {
            app.candidate.is_none()
                && app.comparison == ComparisonMode::Current
                && !app
                    .scene
                    .nodes
                    .iter()
                    .any(|n| n.semantic.name == "ScenarioNestedPart")
                && app.selection.targets.is_empty()
                && app.selection.primary.is_none()
                && app.selection.revision == app.scene.revision_id
        }
        Check::Revision(revision) => {
            app.projection.revision_id == *revision
                && app.scene.revision_id == *revision
                && app.selection.revision == *revision
                && app.selection.targets.is_empty()
                && app.inspector.is_none()
                && app.explanation.is_none()
        }
        Check::ContrastChanged => app.theme.contrast != before.high_contrast && !app.palette,
        Check::ThemeChanged => app.theme.dark != before.dark && !app.palette,
        Check::ReducedMotion => app.reduced_motion && !app.palette,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_and_pointer_injection_use_real_events() {
        let mut raw = egui::RawInput::default();
        key(&mut raw, Key::K, Modifiers::COMMAND);
        assert!(matches!(
            raw.events[0],
            Event::Key {
                key: Key::K,
                pressed: true,
                ..
            }
        ));
        assert!(matches!(raw.events[1], Event::Key { pressed: false, .. }));
        click(&mut raw, Pos2::new(42.0, 70.0), true, Modifiers::SHIFT);
        assert!(
            matches!(raw.events.last(),Some(Event::PointerButton {pressed:true,modifiers,..}) if modifiers.shift)
        );
    }
    #[test]
    fn scenario_covers_visual_edit_and_fixture_authority_boundary() {
        let steps = vertical();
        assert!(steps.iter().any(|s| matches!(s.check, Check::Candidate)));
        assert!(
            steps
                .iter()
                .any(|s| matches!(s.check, Check::Disabled(CommandId::Validate)))
        );
        assert!(
            steps
                .iter()
                .any(|s| matches!(s.check, Check::Disabled(CommandId::Commit)))
        );
        assert!(steps.iter().any(|s| matches!(s.check, Check::Cancelled)));
    }
}
