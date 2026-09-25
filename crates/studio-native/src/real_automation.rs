//! Opt-in, two-process native acceptance over authenticated real semantic data.
//!
//! This driver can only inject ordinary input and inspect application responses.
//! It never fabricates projections, mutates a graph, or calls a commit bypass.
use crate::{
    Args,
    app::{ComparisonMode, StudioApp},
    automation::{self, ScenarioStatus},
    navigation::World,
    real_targets::{self, Target},
};
use agq_kernel::ElementId;
use agq_modeling_repository::{
    BranchId, ContentDigest, ProjectId, ProjectRevisionId, RevisionManifest, ValidationState,
};
use agq_modeling_view::{ExplanationNodeKind, ViewKind, ViewOrigin};
use agq_studio_platform::{CandidatePhase, RevisionBinding};
use agq_studio_scene::{DiffMark, NodeCategory, Point, SceneTarget};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, time::Instant};

const FORMAT: &str = "agentique-native-real-acceptance/1";
const PART_NAME: &str = "alphaStudioObserver";

/// Run before StudioApp starts a worker: real acceptance may write only to an
/// explicitly selected fresh database. Restart reads that same isolated database.
pub fn validate_launch(args: &Args) -> Result<(), String> {
    let real = matches!(args.scenario.as_deref(), Some("real" | "real-restart"));
    if !real {
        return if args.restart_report.is_some() {
            Err("--restart-report is only valid with --scenario real-restart".into())
        } else {
            Ok(())
        };
    }
    if args.fixture.is_some() || !args.no_restore || args.frames.is_some() {
        return Err(
            "Real acceptance requires --no-restore and refuses --fixture or --frames".into(),
        );
    }
    if args.screenshot.is_some() || args.gallery.is_none() {
        return Err(
            "Real acceptance requires --gallery and refuses single-frame --screenshot".into(),
        );
    }
    if args.scenario_timeout_seconds == 0 {
        return Err("Real acceptance needs a nonzero wall-time deadline".into());
    }
    let database = args
        .database
        .as_ref()
        .ok_or("An explicit isolated --database is required")?;
    if !database.is_absolute() {
        return Err("The acceptance database path must be absolute".into());
    }
    if entry_exists(&args.scenario_report) {
        return Err(
            "Choose a new --scenario-report; prior acceptance evidence is preserved".into(),
        );
    }
    if args.gallery.as_ref().is_some_and(|path| entry_exists(path)) {
        return Err("Choose a new --gallery directory; prior captures are preserved".into());
    }
    if args.scenario.as_deref() == Some("real") {
        if args.restart_report.is_some()
            || entry_exists(database)
            || entry_exists(&database.with_extension("views.sqlite"))
            || entry_exists(&database.with_extension("native-session.json"))
            || ["-wal", "-shm"].iter().any(|suffix| {
                let mut path = database.as_os_str().to_os_string();
                path.push(suffix);
                entry_exists(Path::new(&path))
            })
        {
            return Err("First real run requires a new database with no sidecar presentation state and no restart report".into());
        }
    } else {
        let report = read_restart(args)?;
        if !database.is_file() || Path::new(&report.database) != database {
            return Err(
                "Restart must open the exact existing database from the successful first report"
                    .into(),
            );
        }
    }
    Ok(())
}

fn entry_exists(path: &Path) -> bool {
    // Includes broken symbolic links: a missing target is not a fresh entry.
    std::fs::symlink_metadata(path).is_ok()
}

fn read_restart(args: &Args) -> Result<Report, String> {
    let path = args
        .restart_report
        .as_ref()
        .ok_or("--scenario real-restart requires --restart-report")?;
    let bytes =
        std::fs::read(path).map_err(|e| format!("Cannot read first-process report: {e}"))?;
    let report: Report =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid first-process report: {e}"))?;
    if report.format != FORMAT
        || report.scenario != "real"
        || !report.passed
        || report.outcome != "journey_passed_restart_pending"
        || report.restart_verified
        || report.committed.is_none()
        || report.added_element.is_none()
        || report.gallery.len() < 8
    {
        return Err("Restart requires a successful real first-process journey with committed identity and complete gallery".into());
    }
    Ok(report)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct State {
    frame: u64,
    binding: Option<RevisionBinding>,
    branch: Option<BranchId>,
    scene_revision: ProjectRevisionId,
    selection_revision: ProjectRevisionId,
    selected: Vec<String>,
    focus: Option<ElementId>,
    world: String,
    comparison: String,
    candidate_revision: Option<ProjectRevisionId>,
    candidate_phase: Option<String>,
    producer_completeness: String,
    projection_nodes: usize,
    projection_edges: usize,
    scene_nodes: usize,
    scene_edges: usize,
    camera: [f32; 3],
    pending_requests: usize,
    mutation_pending: bool,
    inspector_revision: Option<ProjectRevisionId>,
    inspector_element: Option<ElementId>,
    explanation_revision: Option<ProjectRevisionId>,
    explanation_subject: Option<ElementId>,
    explanation_origin: Option<ViewOrigin>,
    explanation_rule: Option<String>,
    explanation_evidence: Option<usize>,
    agent_view: bool,
    status: String,
}
impl State {
    fn of(app: &StudioApp) -> Self {
        Self {
            frame: app.frame_number,
            binding: app.binding,
            branch: app.branch,
            scene_revision: app.scene.revision_id,
            selection_revision: app.selection.revision,
            selected: app
                .selection
                .targets
                .iter()
                .map(|t| format!("{t:?}"))
                .collect(),
            focus: app.focus,
            world: format!("{:?}", app.world),
            comparison: format!("{:?}", app.comparison),
            candidate_revision: app.candidate.as_ref().map(|c| c.after.revision_id),
            candidate_phase: app
                .candidate
                .as_ref()
                .and_then(|c| c.phase.map(|p| format!("{p:?}"))),
            producer_completeness: app
                .active_projection()
                .metadata
                .producer_completeness
                .clone(),
            projection_nodes: app.active_projection().nodes.len(),
            projection_edges: app.active_projection().edges.len(),
            scene_nodes: app.scene.nodes.len(),
            scene_edges: app.scene.edges.len(),
            camera: [app.camera.center.x, app.camera.center.y, app.camera.zoom],
            pending_requests: app.pending.len(),
            mutation_pending: app.bridge.mutation_pending(),
            inspector_revision: app.inspector.as_ref().map(|i| i.revision_id),
            inspector_element: app.inspector.as_ref().map(|i| i.element.id),
            explanation_revision: app.explanation.as_ref().map(|e| e.revision_id),
            explanation_subject: app.explanation.as_ref().map(|e| e.subject_id),
            explanation_origin: app.explanation.as_ref().map(|e| e.origin),
            explanation_rule: app
                .explanation
                .as_ref()
                .and_then(|e| e.rule_id.map(|id| id.to_string())),
            explanation_evidence: app.explanation.as_ref().map(|e| e.evidence_count),
            agent_view: app.show_agent,
            status: app.status.clone(),
        }
    }
    fn same_capture_context(&self, other: &Self) -> bool {
        self.binding == other.binding
            && self.scene_revision == other.scene_revision
            && self.selection_revision == other.selection_revision
            && self.selected == other.selected
            && self.focus == other.focus
            && self.world == other.world
            && self.comparison == other.comparison
            && self.candidate_revision == other.candidate_revision
            && self.candidate_phase == other.candidate_phase
            && self.explanation_subject == other.explanation_subject
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Assertion {
    name: String,
    passed: bool,
    elapsed_ms: u128,
    since_start_ms: u128,
    before: State,
    after: State,
    inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Report {
    format: String,
    scenario: String,
    scope: String,
    database: String,
    root: String,
    outcome: String,
    passed: bool,
    restart_verified: bool,
    previous_report_digest: Option<ContentDigest>,
    project: Option<ProjectId>,
    branch: Option<BranchId>,
    baseline: Option<RevisionManifest>,
    committed: Option<RevisionManifest>,
    added_element: Option<ElementId>,
    added_name: String,
    assertions: Vec<Assertion>,
    gallery: Vec<String>,
    failure: Option<String>,
    elapsed_ms: u128,
    background_frames: u64,
    background_pan_observed: bool,
    metrics: serde_json::Value,
    last_state: State,
}

#[derive(Clone, Copy, Debug)]
struct Named {
    name: &'static str,
    kind: &'static str,
}
const PLATFORM: Named = Named {
    name: "ModelingPlatform",
    kind: "PartDefinition",
};
const REPOSITORY: Named = Named {
    name: "ModelRepository",
    kind: "PartDefinition",
};
const PART: Named = Named {
    name: PART_NAME,
    kind: "PartUsage",
};

fn named(app: &StudioApp, named: Named) -> Result<ElementId, String> {
    let ids: Vec<_> = app
        .active_projection()
        .nodes
        .iter()
        .filter(|n| n.name == named.name && n.semantic_kind == named.kind)
        .map(|n| n.id)
        .collect();
    match ids.as_slice() {
        [id] => Ok(*id),
        _ => Err(format!(
            "Expected one real {} named {}; found {} in the current projection",
            named.kind,
            named.name,
            ids.len()
        )),
    }
}

#[derive(Clone, Debug)]
enum Action {
    OpenProject,
    Select(Named),
    Palette(&'static str),
    Key(Key),
    Standards,
    DerivedEdge,
    Prepare,
    Mode(ComparisonMode),
    HistoryBaseline,
}
impl Action {
    fn frames(&self) -> u64 {
        match self {
            Self::Select(_) | Self::Palette(_) => 12,
            Self::Prepare => 7,
            Self::Key(_) => 1,
            _ => 2,
        }
    }
}

#[derive(Clone, Debug)]
enum Check {
    Baseline,
    Selected(Named),
    FocusedPlatform,
    RepositoryInterfaces,
    Home,
    World(World),
    Dependencies,
    Standards,
    DerivedEdge,
    Explanation,
    ExplanationClosed,
    History,
    ParentDiff,
    CreateDialog,
    CandidateWorking,
    Mode(ComparisonMode),
    Validated,
    Committed,
    Restart,
}

#[derive(Clone, Debug)]
struct Step {
    name: &'static str,
    action: Action,
    check: Check,
    capture: Option<&'static str>,
}
fn steps(restart: bool) -> Vec<Step> {
    let step = |name, action, check, capture| Step {
        name,
        action,
        check,
        capture,
    };
    if restart {
        return vec![
            step(
                "restart opens the same durable real project",
                Action::OpenProject,
                Check::Restart,
                Some("10-restarted-system-world"),
            ),
            step(
                "restart inspects the same committed canonical element",
                Action::Select(PART),
                Check::Selected(PART),
                Some("11-restarted-committed-element"),
            ),
            step(
                "restart confirms durable revision history",
                Action::Key(Key::Num4),
                Check::History,
                Some("12-restarted-history"),
            ),
        ];
    }
    vec![
        step(
            "open authenticated Agentique with Validated semantic closure",
            Action::OpenProject,
            Check::Baseline,
            Some("01-system-world"),
        ),
        step(
            "select ModelingPlatform through the semantic outliner",
            Action::Select(PLATFORM),
            Check::Selected(PLATFORM),
            None,
        ),
        step(
            "focus ModelingPlatform through native keyboard input",
            Action::Key(Key::F),
            Check::FocusedPlatform,
            Some("02-focused-subsystem"),
        ),
        step(
            "return through project architecture to its component definitions",
            Action::Palette("Home"),
            Check::Home,
            None,
        ),
        step(
            "inspect ModelRepository and inherited interfaces",
            Action::Select(REPOSITORY),
            Check::RepositoryInterfaces,
            None,
        ),
        step(
            "agent shows revision-bound dependencies",
            Action::Palette("Show dependencies"),
            Check::Dependencies,
            Some("07-agent-view"),
        ),
        step(
            "open Graph World over real relationships",
            Action::Key(Key::Num2),
            Check::World(World::Graph),
            Some("03-graph-world"),
        ),
        step(
            "include authenticated adjacent standard dependencies",
            Action::Standards,
            Check::Standards,
            None,
        ),
        step(
            "select an actual derived relationship by hit-tested geometry",
            Action::DerivedEdge,
            Check::DerivedEdge,
            None,
        ),
        step(
            "Explain renders actual derived causal evidence",
            Action::Key(Key::E),
            Check::Explanation,
            Some("05-explain"),
        ),
        step(
            "dismiss Explain",
            Action::Key(Key::Escape),
            Check::ExplanationClosed,
            None,
        ),
        step(
            "open real Requirements World",
            Action::Key(Key::Num3),
            Check::World(World::Requirements),
            Some("04-requirements-world"),
        ),
        step(
            "open immutable real design history",
            Action::Key(Key::Num4),
            Check::History,
            None,
        ),
        step(
            "open System World for a matching-lens comparison",
            Action::Key(Key::Num1),
            Check::World(World::System),
            None,
        ),
        step(
            "compare actual parent revision in System World",
            Action::Palette("Compare with parent"),
            Check::ParentDiff,
            Some("06-history-diff"),
        ),
        step(
            "select current revision in History to leave comparison",
            Action::Key(Key::Num4),
            Check::History,
            None,
        ),
        step(
            "restore current revision atomically",
            Action::HistoryBaseline,
            Check::Baseline,
            None,
        ),
        step(
            "return to System World before direct manipulation",
            Action::Key(Key::Num1),
            Check::Home,
            None,
        ),
        step(
            "select nested part owner",
            Action::Select(PLATFORM),
            Check::Selected(PLATFORM),
            None,
        ),
        step(
            "open Create Part through contextual palette",
            Action::Palette("Create: nested Part"),
            Check::CreateDialog,
            None,
        ),
        step(
            "prepare real source-backed candidate while current remains responsive",
            Action::Prepare,
            Check::CandidateWorking,
            None,
        ),
        step(
            "review Candidate revision",
            Action::Mode(ComparisonMode::Candidate),
            Check::Mode(ComparisonMode::Candidate),
            None,
        ),
        step(
            "select real candidate part",
            Action::Select(PART),
            Check::Selected(PART),
            Some("08-candidate"),
        ),
        step(
            "review immutable Current revision",
            Action::Mode(ComparisonMode::Current),
            Check::Mode(ComparisonMode::Current),
            None,
        ),
        step(
            "return to Candidate revision",
            Action::Mode(ComparisonMode::Candidate),
            Check::Mode(ComparisonMode::Candidate),
            None,
        ),
        step(
            "review exact candidate difference",
            Action::Mode(ComparisonMode::Diff),
            Check::Mode(ComparisonMode::Diff),
            Some("09-candidate-diff"),
        ),
        step(
            "validate the retained semantic candidate",
            Action::Palette("Validate candidate"),
            Check::Validated,
            None,
        ),
        step(
            "commit only the validated candidate",
            Action::Palette("Commit validated candidate"),
            Check::Committed,
            None,
        ),
        step(
            "observe committed revision in immutable History",
            Action::Key(Key::Num4),
            Check::History,
            Some("10-committed-history"),
        ),
    ]
}

#[derive(Clone)]
struct Capture {
    name: &'static str,
    state: State,
    started: Instant,
}

#[derive(Clone)]
struct Runner {
    report: Report,
    steps: Vec<Step>,
    index: usize,
    age: u64,
    started: Instant,
    step_started: Instant,
    before: Option<State>,
    point: Option<Pos2>,
    events: Vec<String>,
    capture: Option<Capture>,
    dirty: bool,
    last_write: Instant,
    background_pan: Option<(u8, Pos2, Point)>,
    candidate_id: Option<agq_studio_platform::CandidateId>,
    candidate_revision: Option<ProjectRevisionId>,
}
impl Runner {
    fn new(app: &StudioApp) -> Result<Self, String> {
        let restart = app.args.scenario.as_deref() == Some("real-restart");
        let previous = if restart {
            Some(read_restart(&app.args)?)
        } else {
            None
        };
        let previous_report_digest = app
            .args
            .restart_report
            .as_ref()
            .map(|p| {
                std::fs::read(p)
                    .map(|bytes| ContentDigest::of(&bytes))
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        Ok(Self {
            report: Report {
                format: FORMAT.into(), scenario: app.args.scenario.clone().unwrap(),
                scope: "Actual native input, authenticated runtime, real models/agentique and ordinary in-process service workers. First process alone does not establish durable restart or overall alpha acceptance.".into(),
                database: app.config.database.display().to_string(), root: app.config.root.display().to_string(),
                outcome: "running".into(), passed: false, restart_verified: false,
                previous_report_digest,
                project: previous.as_ref().and_then(|p| p.project),
                branch: previous.as_ref().and_then(|p| p.branch),
                baseline: previous.as_ref().and_then(|p| p.baseline.clone()),
                committed: previous.as_ref().and_then(|p| p.committed.clone()),
                added_element: previous.as_ref().and_then(|p| p.added_element),
                added_name: PART_NAME.into(), assertions: vec![], gallery: vec![], failure: None,
                elapsed_ms: 0, background_frames: 0, background_pan_observed: false,
                metrics: serde_json::Value::Null, last_state: State::of(app),
            },
            steps: steps(restart), index: 0, age: 0, started: Instant::now(), step_started: Instant::now(),
            before: None, point: None, events: vec![], capture: None, dirty: true, last_write: Instant::now(),
            background_pan: None, candidate_id: None, candidate_revision: None,
        })
    }

    fn advance(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        if app.fixture.is_some() || app.args.fixture.is_some() {
            return Err("Real runner refuses every fixture and fixture service fallback".into());
        }
        if self.started.elapsed().as_secs() > app.args.scenario_timeout_seconds {
            return Err(format!(
                "Real acceptance exceeded {} wall seconds at step {}",
                app.args.scenario_timeout_seconds,
                self.index + 1
            ));
        }
        input
            .events
            .retain(|e| matches!(e, Event::Screenshot { .. } | Event::WindowFocused(_)));
        input.focused = true;
        input.modifiers = Modifiers::NONE;
        if self.capture.is_some() {
            self.receive_capture(app, input)?;
            return Ok(ScenarioStatus::Running);
        }
        if self.index == self.steps.len() {
            return Ok(ScenarioStatus::Complete);
        }
        let step = self.steps[self.index].clone();
        if self.before.is_none() {
            if !idle(app) {
                return Ok(ScenarioStatus::Running);
            }
            if matches!(step.action, Action::OpenProject) && app.projects.is_empty() {
                if app.frame_number > 2 {
                    return Err(format!(
                        "Authenticated bootstrap produced no project: {}",
                        app.setup_reason
                    ));
                }
                return Ok(ScenarioStatus::Running);
            }
            if !matches!(step.action, Action::OpenProject) {
                assert_bound(app)?;
            }
            self.before = Some(State::of(app));
            self.step_started = Instant::now();
            self.age = 0;
            self.point = None;
            self.events.clear();
        }
        // Distinct clicks are separated in ordinary native time. This prevents
        // accidental outliner double-click focus across unrelated steps.
        const LEAD: u64 = 30;
        if self.age >= LEAD && self.age < LEAD + step.action.frames() {
            let start = input.events.len();
            self.inject(&step.action, self.age - LEAD, app, ctx, input)?;
            self.events
                .extend(input.events[start..].iter().map(|e| format!("{e:?}")));
        }
        if matches!(step.action, Action::Prepare) && app.bridge.mutation_pending() {
            self.background_input(app, ctx, input)?;
        }
        self.age += 1;
        if self.age < LEAD + step.action.frames() + 18 || !idle(app) {
            return Ok(ScenarioStatus::Running);
        }
        if self.background_pan.is_some() {
            self.background_input(app, ctx, input)?;
            return Ok(ScenarioStatus::Running);
        }
        assert_bound(app)?;
        let result = self.check(&step.check, app, ctx);
        self.report.assertions.push(Assertion {
            name: step.name.into(),
            passed: result.is_ok(),
            elapsed_ms: self.step_started.elapsed().as_millis(),
            since_start_ms: self.started.elapsed().as_millis(),
            before: self.before.take().expect("step initialized"),
            after: State::of(app),
            inputs: self.events.clone(),
        });
        self.dirty = true;
        result.map_err(|error| {
            format!(
                "Step {} '{}': {error}; status={}",
                self.index + 1,
                step.name,
                app.status
            )
        })?;
        self.index += 1;
        if let Some(name) = step.capture {
            self.capture = Some(Capture {
                name,
                state: State::of(app),
                started: Instant::now(),
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                name.to_string(),
            )));
        }
        Ok(ScenarioStatus::Running)
    }

    fn receive_capture(&mut self, app: &StudioApp, input: &egui::RawInput) -> Result<(), String> {
        let capture = self.capture.as_ref().expect("pending capture");
        if !capture.state.same_capture_context(&State::of(app)) || !idle(app) {
            return Err(format!(
                "Semantic or presentation context changed while capturing {}",
                capture.name
            ));
        }
        let image = input.events.iter().find_map(|e| match e {
            Event::Screenshot {
                user_data, image, ..
            } if user_data
                .data
                .as_ref()
                .and_then(|v| v.downcast_ref::<String>())
                .is_some_and(|name| name == capture.name) =>
            {
                Some(image)
            }
            _ => None,
        });
        if let Some(image) = image {
            let directory = app
                .args
                .gallery
                .as_ref()
                .ok_or("Real gallery directory missing")?;
            std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            let path = directory.join(format!("{}.png", capture.name));
            if path.exists() {
                return Err(format!("Refusing to overwrite capture {}", path.display()));
            }
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            image::save_buffer(
                &path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .map_err(|e| e.to_string())?;
            let evidence = serde_json::json!({
                "format": "agentique-native-real-gallery/1", "semantic_data": "real authenticated models/agentique",
                "fixture": null, "checkpoint": capture.name, "state": capture.state,
                "baseline": self.report.baseline, "committed": self.report.committed,
                "image_size": image.size, "image_digest": ContentDigest::of(&std::fs::read(&path).map_err(|e| e.to_string())?),
                "adapter": app.adapter, "metrics": app.metrics_report(),
            });
            std::fs::write(
                path.with_extension("json"),
                serde_json::to_vec_pretty(&evidence).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            self.report.gallery.push(path.display().to_string());
            self.capture = None;
            self.dirty = true;
        } else if capture.started.elapsed().as_secs() > 30 {
            return Err(format!(
                "Native screenshot {} was not delivered within 30 seconds",
                capture.name
            ));
        }
        Ok(())
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
            Action::OpenProject => {
                let project = app
                    .projects
                    .iter()
                    .find(|p| {
                        self.report
                            .project
                            .map_or(p.name == "Agentique", |id| p.id == id)
                    })
                    .ok_or("Expected real Agentique project is absent")?;
                click(
                    input,
                    real_targets::target(ctx, Target::Project(project.id))?.center(),
                    frame == 0,
                );
            }
            Action::Select(wanted) => match frame {
                0 | 1 => click(
                    input,
                    real_targets::target(ctx, Target::ExplorerSearch)?.center(),
                    frame == 0,
                ),
                2 => key(input, Key::A, Modifiers::COMMAND),
                3 => input.events.push(Event::Text(wanted.name.into())),
                8 | 9 => {
                    if app.search != wanted.name {
                        return Err("Explorer did not retain native text input".into());
                    }
                    let id = named(app, *wanted)?;
                    click(
                        input,
                        real_targets::target(ctx, Target::ExplorerElement(id))?.center(),
                        frame == 8,
                    );
                }
                _ => {}
            },
            Action::Palette(query) => match frame {
                0 => key(input, Key::K, Modifiers::COMMAND),
                5 | 6 => click(
                    input,
                    automation::target(ctx, automation::Target::PaletteInput)?.center(),
                    frame == 5,
                ),
                7 => key(input, Key::A, Modifiers::COMMAND),
                8 => input.events.push(Event::Text((*query).into())),
                10 => {
                    if app.palette_query != *query {
                        return Err("Palette did not retain native text input".into());
                    }
                    key(input, Key::Enter, Modifiers::NONE);
                }
                _ => {}
            },
            Action::Key(k) => key(input, *k, Modifiers::NONE),
            Action::Standards => click(
                input,
                real_targets::target(ctx, Target::Standards)?.center(),
                frame == 0,
            ),
            Action::DerivedEdge => {
                let point = if let Some(point) = self.point {
                    point
                } else {
                    let point = derived_point(app, ctx)?;
                    self.point = Some(point);
                    point
                };
                click(input, point, frame == 0);
            }
            Action::Prepare => match frame {
                0 | 1 => click(
                    input,
                    automation::target(ctx, automation::Target::CandidateName)?.center(),
                    frame == 0,
                ),
                2 => key(input, Key::A, Modifiers::COMMAND),
                3 => input.events.push(Event::Text(PART_NAME.into())),
                5 | 6 => {
                    if frame == 5 && app.new_part_name != PART_NAME {
                        return Err("Candidate name did not retain native text input".into());
                    }
                    click(
                        input,
                        automation::target(ctx, automation::Target::CandidatePrepare)?.center(),
                        frame == 5,
                    );
                }
                _ => {}
            },
            Action::Mode(mode) => {
                let target = match mode {
                    ComparisonMode::Current => Target::ComparisonCurrent,
                    ComparisonMode::Candidate => Target::ComparisonCandidate,
                    ComparisonMode::Diff => Target::ComparisonDiff,
                };
                click(
                    input,
                    real_targets::target(ctx, target)?.center(),
                    frame == 0,
                );
            }
            Action::HistoryBaseline => {
                let revision = self
                    .report
                    .baseline
                    .as_ref()
                    .ok_or("No retained baseline")?
                    .revision_id;
                click(
                    input,
                    automation::target(ctx, automation::Target::HistoryRevision(revision))?
                        .center(),
                    frame == 0,
                );
            }
        }
        Ok(())
    }

    fn background_input(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<(), String> {
        if app.bridge.mutation_pending() {
            self.report.background_frames += 1;
            if app.binding.map(|b| b.revision)
                != self.report.baseline.as_ref().map(|m| m.revision_id)
                || app.scene.revision_id
                    != self
                        .report
                        .baseline
                        .as_ref()
                        .ok_or("Baseline absent")?
                        .revision_id
            {
                return Err(
                    "Candidate reconstruction replaced the current world before completion".into(),
                );
            }
        }
        if self.report.background_pan_observed {
            return Ok(());
        }
        if self.background_pan.is_none() {
            let rect = automation::target(ctx, automation::Target::Viewport)?;
            self.background_pan = Some((0, rect.center(), app.camera.center));
        }
        let (frame, point, before) = self.background_pan.as_mut().expect("initialized pan");
        match *frame {
            0 => click(input, *point, true),
            1 | 2 => input.events.push(Event::PointerMoved(
                *point + Vec2::new(35.0 * *frame as f32, 20.0 * *frame as f32),
            )),
            3 => click(input, *point + Vec2::new(70.0, 40.0), false),
            _ => {
                if app.camera.center.distance(*before) < 1.0 {
                    return Err(
                        "Native pan did not move the current camera during background preparation"
                            .into(),
                    );
                }
                self.report.background_pan_observed = app.bridge.mutation_pending();
                self.events.push(format!(
                    "Background native pan: camera {:?} -> {:?}, mutation_pending={}",
                    before,
                    app.camera.center,
                    app.bridge.mutation_pending()
                ));
                self.background_pan = None;
                return Ok(());
            }
        }
        *frame += 1;
        Ok(())
    }

    fn check(&mut self, check: &Check, app: &StudioApp, ctx: &egui::Context) -> Result<(), String> {
        let require = |ok: bool, reason: &str| if ok { Ok(()) } else { Err(reason.to_string()) };
        match check {
            Check::Baseline => {
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                if let Some(baseline) = &self.report.baseline {
                    require(
                        manifest == baseline && app.comparison == ComparisonMode::Current,
                        "Baseline revision or comparison mode changed",
                    )?;
                } else {
                    assert_self_model_sources(app, manifest)?;
                    self.report.project = Some(manifest.project_id);
                    self.report.branch = app.branch;
                    self.report.baseline = Some(manifest.clone());
                }
                require(
                    app.gpu_stats.lock().is_ok_and(|s| s.draw_calls > 0),
                    "Native GPU scene has not drawn",
                )
            }
            Check::Selected(wanted) => {
                let id = named(app, *wanted)?;
                require(
                    app.selected_element() == Some(id)
                        && app.inspector.as_ref().is_some_and(|i| {
                            i.element.id == id && i.revision_id == app.scene.revision_id
                        }),
                    "Selection and real Inspector are not bound to the chosen canonical object",
                )?;
                if wanted.name == PART_NAME {
                    if let Some(expected) = self.report.added_element {
                        require(
                            id == expected,
                            "Created canonical identity changed across candidate/restart",
                        )?;
                    } else {
                        self.report.added_element = Some(id);
                    }
                }
                Ok(())
            }
            Check::FocusedPlatform => require(
                app.world == World::System && app.focus == Some(named(app, PLATFORM)?),
                "System focus did not enter ModelingPlatform",
            ),
            Check::RepositoryInterfaces => {
                self.check(&Check::Selected(REPOSITORY), app, ctx)?;
                let inspector = app
                    .inspector
                    .as_ref()
                    .ok_or("Repository Inspector unavailable")?;
                require(
                    inspector
                        .owned_features
                        .iter()
                        .chain(&inspector.effective_features)
                        .any(|f| {
                            matches!(f.semantic_kind.as_str(), "PortUsage" | "PortDefinition")
                        }),
                    "ModelRepository has no modeled interface in its effective Inspector",
                )
            }
            Check::Home => require(
                app.world == World::System
                    && app.focus.is_none()
                    && app.comparison == ComparisonMode::Current,
                "System World did not return to the current architecture",
            ),
            Check::World(world) => {
                require(app.world == *world, "Requested World did not open")?;
                if *world == World::Requirements {
                    require(
                        app.active_projection().view.kind == ViewKind::Requirements
                            && app
                                .scene
                                .nodes
                                .iter()
                                .any(|n| n.category == NodeCategory::Requirement),
                        "Requirements World has no actual modeled requirements",
                    )?;
                }
                require(
                    !app.scene.nodes.is_empty() && !app.scene.edges.is_empty(),
                    "Real World has no inspectable semantic relationships",
                )
            }
            Check::Dependencies => require(
                app.world == World::Graph
                    && app.show_agent
                    && app.focus.is_some()
                    && app.projection.view.focus == app.focus
                    && !app.projection.edges.is_empty(),
                "Agent action did not return an actual revision-bound dependency view",
            ),
            Check::Standards => require(
                app.include_standard
                    && app.active_projection().view.include_standard_library
                    && app.scene.edges.iter().any(|e| {
                        e.semantic.origin == ViewOrigin::Derived
                            && e.semantic.relationship_id.is_some()
                    }),
                "No derived relationship from authenticated standards is visible",
            ),
            Check::DerivedEdge => require(
                app.selection.primary.as_ref().is_some_and(|t| match t {
                    SceneTarget::Edge(id) => app.lookup.edge(&app.scene, id).is_some_and(|e| {
                        e.semantic.origin == ViewOrigin::Derived
                            && e.semantic.relationship_id.is_some()
                    }),
                    _ => false,
                }),
                "Pointer did not select a canonical derived relationship",
            ),
            Check::Explanation => {
                self.check(&Check::DerivedEdge, app, ctx)?;
                let evidence = app
                    .explanation
                    .as_ref()
                    .ok_or("Real explanation worker returned no evidence")?;
                require(
                    app.show_explain
                        && evidence.subject_id
                            == app.selected_element().ok_or("No explanation subject")?
                        && evidence.origin == ViewOrigin::Derived
                        && evidence.rule_id.is_some()
                        && evidence.evidence_count > 0
                        && !evidence.edges.is_empty()
                        && evidence
                            .nodes
                            .iter()
                            .any(|n| n.kind == ExplanationNodeKind::Rule)
                        && automation::target(ctx, automation::Target::ExplainWindow).is_ok(),
                    "Explain lacks actual derived rule/evidence or a native visible window",
                )
            }
            Check::ExplanationClosed => require(!app.show_explain, "Explain did not dismiss"),
            Check::History => {
                require(app.world == World::History, "History World did not open")?;
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                let expected = self
                    .report
                    .committed
                    .as_ref()
                    .or(self.report.baseline.as_ref())
                    .ok_or("No expected durable revision")?;
                require(
                    manifest == expected,
                    "History is not showing the expected durable revision",
                )
            }
            Check::ParentDiff => {
                let manifest = current_manifest(app)?;
                require(
                    app.comparison == ComparisonMode::Diff
                        && app.compare_before.as_ref().map(|p| p.revision_id)
                            == manifest.parent_revision_id
                        && app
                            .compare_before
                            .as_ref()
                            .is_some_and(|before| before.view == app.projection.view)
                        && app
                            .scene
                            .nodes
                            .iter()
                            .any(|n| n.diff != DiffMark::Unchanged),
                    "Parent difference lacks exact parent, matching lens, or structural changes",
                )
            }
            Check::CreateDialog => require(
                app.create_dialog && !app.palette,
                "Create Part dialog did not open",
            ),
            Check::CandidateWorking => {
                let candidate = app
                    .candidate
                    .as_ref()
                    .ok_or("No real candidate was returned")?;
                require(
                    candidate.id.is_some()
                        && candidate.phase == Some(CandidatePhase::Working)
                        && candidate.before.revision_id
                            == self
                                .report
                                .baseline
                                .as_ref()
                                .ok_or("No baseline")?
                                .revision_id
                        && candidate.after.revision_id != candidate.before.revision_id
                        && candidate.source.contains(&format!("part {PART_NAME};")),
                    "Working candidate lacks actual identity/source/base binding",
                )?;
                require(
                    current_manifest(app)?.revision_id == candidate.before.revision_id,
                    "Working candidate advanced durable history",
                )?;
                self.candidate_id = candidate.id;
                self.candidate_revision = Some(candidate.after.revision_id);
                if self.report.background_frames >= 10 {
                    require(
                        self.report.background_pan_observed,
                        "Long-running preparation did not demonstrate responsive native panning",
                    )?;
                }
                Ok(())
            }
            Check::Mode(mode) => {
                let candidate = app
                    .candidate
                    .as_ref()
                    .ok_or("Candidate disappeared during review")?;
                require(
                    candidate.id == self.candidate_id
                        && Some(candidate.after.revision_id) == self.candidate_revision
                        && app.comparison == *mode,
                    "Candidate identity or review mode is wrong",
                )?;
                let expected = if *mode == ComparisonMode::Current {
                    candidate.before.revision_id
                } else {
                    candidate.after.revision_id
                };
                require(
                    app.scene.revision_id == expected,
                    "Candidate mode mixed revisions",
                )?;
                if *mode == ComparisonMode::Diff {
                    require(
                        app.scene
                            .nodes
                            .iter()
                            .any(|n| n.semantic.name == PART_NAME && n.diff == DiffMark::Added),
                        "Added part is absent from semantic difference",
                    )?;
                }
                Ok(())
            }
            Check::Validated => require(
                app.candidate.as_ref().is_some_and(|c| {
                    c.id == self.candidate_id
                        && Some(c.after.revision_id) == self.candidate_revision
                        && c.phase == Some(CandidatePhase::Validated)
                }),
                "Candidate was not validated by the real platform",
            ),
            Check::Committed => {
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                require(
                    app.candidate.is_none()
                        && Some(manifest.revision_id) == self.candidate_revision
                        && manifest.parent_revision_id
                            == self.report.baseline.as_ref().map(|m| m.revision_id)
                        && app.comparison == ComparisonMode::Current,
                    "Commit did not durably advance exactly the validated candidate",
                )?;
                require(
                    app.history.as_ref().is_some_and(|h| {
                        h.branches.iter().any(|b| {
                            Some(b.id) == self.report.branch && b.head == manifest.revision_id
                        })
                    }),
                    "Branch history did not confirm the committed head",
                )?;
                self.report.committed = Some(manifest.clone());
                Ok(())
            }
            Check::Restart => {
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                require(
                    self.report.committed.as_ref() == Some(manifest)
                        && app.branch == self.report.branch
                        && app.binding.map(|b| b.project) == self.report.project,
                    "Reauthenticated durable revision/receipt differs from the first process",
                )?;
                require(
                    Some(named(app, PART)?) == self.report.added_element,
                    "Restarted canonical part identity differs",
                )?;
                require(
                    app.history.as_ref().is_some_and(|h| {
                        h.branches.iter().any(|b| {
                            Some(b.id) == self.report.branch && b.head == manifest.revision_id
                        })
                    }),
                    "Restarted branch head differs from committed revision",
                )
            }
        }
    }
}

fn idle(app: &StudioApp) -> bool {
    app.pending.is_empty()
        && !app.bridge.mutation_pending()
        && !app.scene_builder.busy
        && (!app.ready || (!app.fit_pending && app.camera_target.is_none()))
}

fn assert_bound(app: &StudioApp) -> Result<(), String> {
    let binding = app.binding.ok_or("No real repository revision is bound")?;
    let projection = app.active_projection();
    if !app.ready
        || app.fixture.is_some()
        || app.branch.is_none()
        || app.projection.revision_id != binding.revision
        || app.scene.revision_id != projection.revision_id
        || app.selection.revision != app.scene.revision_id
        || projection
            .nodes
            .iter()
            .any(|n| n.revision_id != projection.revision_id)
        || projection
            .edges
            .iter()
            .any(|e| e.revision_id != projection.revision_id)
        || projection.metadata.producer_completeness != "Complete"
    {
        return Err("Real view has incomplete closure or mixed project/projection/scene/selection revision bindings".into());
    }
    if let Some(inspector) = &app.inspector
        && !app.selected_context().is_some_and(|(b, _, id)| {
            b.revision == inspector.revision_id && id == inspector.element.id
        })
    {
        return Err("Inspector is stale or bound to another selection/revision".into());
    }
    if let Some(explanation) = &app.explanation
        && !app.selected_context().is_some_and(|(b, _, id)| {
            b.revision == explanation.revision_id && id == explanation.subject_id
        })
    {
        return Err("Explain is stale or bound to another selection/revision".into());
    }
    Ok(())
}

fn current_manifest(app: &StudioApp) -> Result<&RevisionManifest, String> {
    let binding = app.binding.ok_or("No durable binding")?;
    app.history
        .as_ref()
        .filter(|h| h.project.id == binding.project)
        .and_then(|h| {
            h.revisions
                .iter()
                .find(|m| m.revision_id == binding.revision)
        })
        .ok_or_else(|| "Bound revision has no matching actual repository manifest".into())
}
fn assert_validated(manifest: &RevisionManifest) -> Result<(), String> {
    if matches!(manifest.validation, ValidationState::Validated(_)) {
        Ok(())
    } else {
        Err("Actual durable revision is Working, not Validated".into())
    }
}
fn assert_self_model_sources(app: &StudioApp, manifest: &RevisionManifest) -> Result<(), String> {
    let entries =
        std::fs::read_dir(app.config.root.join("models/agentique")).map_err(|e| e.to_string())?;
    let mut expected = BTreeMap::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "sysml") {
            expected.insert(
                path.file_name()
                    .ok_or("Invalid source filename")?
                    .to_string_lossy()
                    .to_string(),
                ContentDigest::of(&std::fs::read(path).map_err(|e| e.to_string())?),
            );
        }
    }
    if expected.is_empty()
        || manifest.documents.len() != expected.len()
        || manifest
            .documents
            .iter()
            .any(|d| expected.get(&d.path) != Some(&d.content_digest))
    {
        return Err(
            "Repository source identities do not exactly match current models/agentique/*.sysml"
                .into(),
        );
    }
    Ok(())
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
fn screen(app: &StudioApp, viewport: Rect, world: Point) -> Option<Pos2> {
    let point = app.camera.world_to_screen(world);
    let point = viewport.min + Vec2::new(point.x, point.y);
    viewport.shrink(4.0).contains(point).then_some(point)
}
fn derived_point(app: &StudioApp, ctx: &egui::Context) -> Result<Pos2, String> {
    let viewport = automation::target(ctx, automation::Target::Viewport)?;
    for edge in app.scene.edges.iter().filter(|e| {
        e.semantic.origin == ViewOrigin::Derived && e.semantic.relationship_id.is_some()
    }) {
        for pair in edge.points.windows(2) {
            for t in [0.5, 0.25, 0.75] {
                let p = Point::new(
                    pair[0].x + (pair[1].x - pair[0].x) * t,
                    pair[0].y + (pair[1].y - pair[0].y) * t,
                );
                if app.spatial.hit_test(p, 6.0 / app.camera.zoom)
                    == Some(SceneTarget::Edge(edge.semantic.id.clone()))
                    && let Some(point) = screen(app, viewport, p)
                {
                    return Ok(point);
                }
            }
        }
    }
    Err("No visible unoccluded actual derived relationship can be selected; no semantic shortcut substituted".into())
}

pub fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    report_path: &Path,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("agentique-native-real-scenario");
    let mut runner = match ctx.data(|data| data.get_temp::<Runner>(id)) {
        Some(runner) => runner,
        None => Runner::new(app)?,
    };
    let result = runner.advance(app, ctx, input);
    runner.report.elapsed_ms = runner.started.elapsed().as_millis();
    runner.report.last_state = State::of(app);
    match &result {
        Ok(ScenarioStatus::Complete) => {
            runner.report.passed = true;
            runner.report.restart_verified = runner.report.scenario == "real-restart";
            runner.report.outcome = if runner.report.restart_verified {
                "passed"
            } else {
                "journey_passed_restart_pending"
            }
            .into();
            runner.report.metrics = app.metrics_report();
        }
        Err(error) => {
            runner.report.outcome = "failed".into();
            runner.report.failure = Some(error.clone());
        }
        Ok(ScenarioStatus::Running) => {}
    }
    if runner.dirty
        || runner.last_write.elapsed().as_secs() >= 10
        || !matches!(result, Ok(ScenarioStatus::Running))
    {
        if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            report_path,
            serde_json::to_vec_pretty(&runner.report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        runner.dirty = false;
        runner.last_write = Instant::now();
    }
    ctx.data_mut(|data| data.insert_temp(id, runner));
    if matches!(result, Ok(ScenarioStatus::Running)) {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn isolated_args() -> Args {
        let unique = format!(
            "agq-real-launch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        Args::try_parse_from([
            "agq-studio-native",
            "--scenario",
            "real",
            "--no-restore",
            "--database",
            base.with_extension("sqlite").to_str().unwrap(),
            "--scenario-report",
            base.with_extension("json").to_str().unwrap(),
            "--gallery",
            base.to_str().unwrap(),
        ])
        .unwrap()
    }

    #[test]
    fn real_launch_refuses_fixture_default_database_and_session_restoration() {
        let mut args = isolated_args();
        assert!(validate_launch(&args).is_ok());
        args.fixture = Some("architecture".into());
        assert!(validate_launch(&args).unwrap_err().contains("fixture"));
        args.fixture = None;
        args.no_restore = false;
        assert!(validate_launch(&args).is_err());
        args.no_restore = true;
        args.database = None;
        assert!(
            validate_launch(&args)
                .unwrap_err()
                .contains("isolated --database")
        );
    }

    #[test]
    fn real_launch_preserves_existing_database_bytes_before_any_worker_starts() {
        let args = isolated_args();
        let database = args.database.as_ref().unwrap();
        std::fs::write(database, b"existing repository sentinel").unwrap();
        let result = validate_launch(&args);
        let bytes = std::fs::read(database).unwrap();
        std::fs::remove_file(database).unwrap();
        assert!(result.unwrap_err().contains("new database"));
        assert_eq!(bytes, b"existing repository sentinel");
    }

    #[test]
    fn restart_cannot_proceed_from_missing_or_invalid_first_report() {
        let mut args = isolated_args();
        args.scenario = Some("real-restart".into());
        assert!(
            validate_launch(&args)
                .unwrap_err()
                .contains("restart-report")
        );
        let path = args
            .database
            .as_ref()
            .unwrap()
            .with_extension("previous.json");
        args.restart_report = Some(path.clone());
        std::fs::write(&path, br#"{"format":"fixture","passed":true}"#).unwrap();
        let result = validate_launch(&args);
        std::fs::remove_file(path).unwrap();
        assert!(result.unwrap_err().contains("Invalid first-process report"));
    }
}
