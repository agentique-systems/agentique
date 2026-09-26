use crate::{
    Args,
    bridge::Bridge,
    gpu::{Batch, GpuStats},
    navigation::{Navigation, World},
    selection::{CanvasClicks, Selection},
    session::Session,
    theme::Theme,
    timing::FrameTiming,
};
use agq_kernel::ElementId;
use agq_modeling_repository::{BranchId, Project, ProjectId};
use agq_modeling_view::{
    ElementInspector, ExplanationProjection, RelationshipFamily, ViewDefinition, ViewProjection,
};
use agq_studio_platform::{
    CandidateId, CandidatePhase, NativeConfig, ProjectHistory, RevisionBinding, SourceProjection,
};
use agq_studio_scene::{
    Camera2D, LayoutMemory, LodController, Point, SceneLookup, SceneOptions, SemanticScene,
    SpatialIndex, fixtures,
};
use eframe::egui;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonMode {
    Current,
    Candidate,
    Diff,
}

pub struct Candidate {
    pub id: Option<CandidateId>,
    pub phase: Option<CandidatePhase>,
    pub intent: String,
    pub actor: String,
    pub before: ViewProjection,
    pub after: ViewProjection,
    pub source: String,
    pub review_selection: Option<Selection>,
}

pub struct PendingPreparation {
    pub request: u64,
    pub started: Instant,
    pub cancelled: bool,
    pub cancel_requested_at: Option<Instant>,
    pub control: agq_studio_platform::CompilationControl,
    pub intent: String,
}
impl PendingPreparation {
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.cancel_requested_at.get_or_insert_with(Instant::now);
        self.control.cancel();
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PreparationCancellationReceipt {
    pub request: u64,
    pub epoch: u64,
    pub binding: Option<RevisionBinding>,
    pub preparation_elapsed_ms: u128,
    pub request_to_ack_ms: Option<u128>,
    pub last_stage: Option<String>,
}

/// Disposable display state only. Restoring this must never restore a lifecycle
/// phase, durable binding, receipt or operator authority from an older snapshot.
pub struct DisplayState {
    projection: ViewProjection,
    candidate: Option<CandidateDisplay>,
    comparison: ComparisonMode,
    compare_before: Option<ViewProjection>,
    world: World,
    focus: Option<ElementId>,
    selection: Selection,
    camera: Camera2D,
    camera_target: Option<Camera2D>,
    layout: LayoutMemory,
    layouts: BTreeMap<(World, Option<ElementId>), LayoutMemory>,
    layout_world: World,
    layout_focus: Option<ElementId>,
    collapsed: BTreeSet<ElementId>,
    expanded: Option<BTreeSet<ElementId>>,
    families: BTreeSet<RelationshipFamily>,
    include_standard: bool,
    world_filters: BTreeMap<World, (BTreeSet<RelationshipFamily>, bool)>,
    show_agent: bool,
    dependencies: Option<BTreeSet<ElementId>>,
    agent_activity: Option<crate::agents::DependencyActivity>,
    agent_return: Option<crate::agents::AgentReturn>,
    fit_pending: bool,
    focus_changes_pending: bool,
    requested_definition: Option<(u64, ViewDefinition)>,
    deferred_definition: Option<ViewDefinition>,
}

struct CandidateDisplay {
    id: Option<CandidateId>,
    before: ViewProjection,
    after: ViewProjection,
    review_selection: Option<Selection>,
}

pub struct StudioApp {
    pub args: Args,
    pub theme: Theme,
    pub reduced_motion: bool,
    pub ime_composing: bool,
    pub ready: bool,
    /// Successful worker host opening, distinct from a displayed scene or a nonempty project list.
    pub runtime_ready_epoch: Option<u64>,
    pub fixture: Option<String>,
    pub config: NativeConfig,
    pub setup_reason: String,
    pub opening: Option<crate::loading::OpeningProgress>,
    pub bundle_path: String,
    pub projects: Vec<Project>,
    pub binding: Option<RevisionBinding>,
    /// Requested revision becomes current only when its projection is ready.
    pub pending_revision: Option<RevisionBinding>,
    /// A durable acknowledgement survives failed or superseded view preparation.
    pub committed_receipt: Option<(ProjectId, agq_modeling_repository::CommitReceipt)>,
    pub revision_retry: Option<RevisionBinding>,
    pub branch: Option<BranchId>,
    pub history: Option<ProjectHistory>,
    pub bridge: Bridge,
    pub pending: BTreeSet<u64>,
    pub scene_request: u64,
    pub inspector_request: u64,
    pub explanation_request: u64,
    pub source_request: u64,
    pub project_request: u64,
    pub lifecycle_request: u64,
    pub lifecycle_unknown: bool,
    pub history_request: Option<(u64, ProjectId)>,
    pub projection: ViewProjection,
    pub scene: SemanticScene,
    pub scene_builder: crate::scene_build::SceneBuilder,
    pub spatial: SpatialIndex,
    pub lookup: SceneLookup,
    pub outliner_order: Vec<usize>,
    pub camera: Camera2D,
    pub camera_target: Option<Camera2D>,
    pub lod: LodController,
    pub layout: LayoutMemory,
    pub layouts: BTreeMap<(World, Option<ElementId>), LayoutMemory>,
    pub layout_world: World,
    pub layout_focus: Option<ElementId>,
    pub generation: u64,
    pub batch: Arc<Batch>,
    pub batch_key: Option<u64>,
    pub gpu_stats: Arc<Mutex<GpuStats>>,
    pub selection: Selection,
    pub canvas_clicks: CanvasClicks,
    pub explorer_clicks: CanvasClicks,
    pub navigation: Navigation,
    pub world: World,
    pub focus: Option<ElementId>,
    pub collapsed: BTreeSet<ElementId>,
    pub families: BTreeSet<RelationshipFamily>,
    pub expanded: Option<BTreeSet<ElementId>>,
    pub include_standard: bool,
    pub world_filters: BTreeMap<World, (BTreeSet<RelationshipFamily>, bool)>,
    pub inspector: Option<ElementInspector>,
    pub explanation: Option<ExplanationProjection>,
    pub source: Option<SourceProjection>,
    pub show_explain: bool,
    pub show_source: bool,
    pub palette: bool,
    pub palette_query: String,
    pub palette_focus: bool,
    pub create_dialog: bool,
    pub project_dialog: crate::project_dialog::ProjectDialog,
    pub edit_target: Option<crate::part_edit::EditTarget>,
    pub create_dialog_focus: bool,
    pub new_part_name: String,
    pub candidate: Option<Candidate>,
    pub preparation: Option<PendingPreparation>,
    pub last_preparation_cancellation: Option<PreparationCancellationReceipt>,
    pub comparison: ComparisonMode,
    pub compare_before: Option<ViewProjection>,
    pub dependencies: Option<BTreeSet<ElementId>>,
    pub show_agent: bool,
    pub agent_activity: Option<crate::agents::DependencyActivity>,
    pub agent_return: Option<crate::agents::AgentReturn>,
    pub search: String,
    pub status: String,
    pub fit_pending: bool,
    pub focus_changes_pending: bool,
    pub requested_definition: Option<(u64, ViewDefinition)>,
    pub deferred_definition: Option<ViewDefinition>,
    pub marquee_start: Option<Point>,
    pub marquee_end: Option<Point>,
    pub timing: FrameTiming,
    pub frame_number: u64,
    pub capture_requested: bool,
    pub capture_done: bool,
    pub session_path: PathBuf,
    pub restore: Option<Session>,
    pub last_saved: Instant,
    pub adapter: String,
}

impl StudioApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        args: Args,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        install_fonts(&cc.egui_ctx);
        let mut config = NativeConfig::for_root(args.root.clone(), args.runtime_dir.clone())?;
        if let Some(database) = &args.database {
            config.database = database.clone();
        }
        config.seed_agentique_on_empty =
            crate::real_automation::seed_isolated_project(&args).map_err(std::io::Error::other)?;
        let session_path = config.database.with_extension("native-session.json");
        let restore = if args.no_restore || args.fixture.is_some() {
            None
        } else {
            Session::load(&session_path)
        };
        let restored_fixture = restore.as_ref().and_then(restorable_fixture);
        let fixture = args
            .fixture
            .clone()
            .or_else(|| restored_fixture.as_ref().map(|(name, _)| name.clone()));
        let projection = restored_fixture
            .map(|(_, projection)| projection)
            .unwrap_or_else(|| initial_projection(fixture.as_deref()));
        let scene = SemanticScene::from_projection(&projection, &SceneOptions::default(), None)?;
        let spatial = SpatialIndex::build(&scene);
        let lookup = SceneLookup::build(&scene);
        let outliner_order = hierarchy_order(&scene);
        let layout = scene.memory().clone();
        let selection = Selection::new(projection.revision_id);
        let dark = restore.as_ref().map_or(!args.light, |r| r.dark);
        let theme = Theme::new(dark, restore.as_ref().is_some_and(|r| r.high_contrast));
        theme.install(&cc.egui_ctx);
        cc.egui_ctx.style_mut(|style| {
            style.animation_time = if restore.as_ref().is_some_and(|r| r.reduced_motion) {
                0.0
            } else {
                0.12
            };
        });
        let setup = agq_studio_platform::setup_surface(&config)?;
        let mut bridge = Bridge::new(cc.egui_ctx.clone());
        let mut pending = BTreeSet::new();
        let mut opening = None;
        if fixture.is_none() && setup.bundle_located {
            let request = bridge
                .open(config.clone(), None)
                .map_err(std::io::Error::other)?;
            pending.insert(request);
            opening = Some(crate::loading::OpeningProgress::new(request));
        }
        let adapter = cc
            .wgpu_render_state
            .as_ref()
            .map(|s| format!("{:?}", s.adapter.get_info()))
            .unwrap_or_default();
        let mut app = Self {
            args,
            theme,
            reduced_motion: restore.as_ref().is_some_and(|r| r.reduced_motion),
            ime_composing: false,
            ready: fixture.is_some(),
            runtime_ready_epoch: None,
            fixture,
            config,
            setup_reason: setup
                .reason
                .unwrap_or_else(|| "Authenticating accepted publications…".into()),
            opening,
            bundle_path: String::new(),
            projects: vec![],
            binding: None,
            pending_revision: None,
            committed_receipt: None,
            revision_retry: None,
            branch: None,
            history: None,
            bridge,
            pending,
            scene_request: 0,
            inspector_request: 0,
            explanation_request: 0,
            source_request: 0,
            project_request: 0,
            lifecycle_request: 0,
            lifecycle_unknown: false,
            history_request: None,
            projection,
            scene,
            scene_builder: crate::scene_build::SceneBuilder::new(cc.egui_ctx.clone())?,
            spatial,
            lookup,
            outliner_order,
            camera: Camera2D::default(),
            camera_target: None,
            lod: LodController::default(),
            layout,
            layouts: BTreeMap::new(),
            layout_world: World::System,
            layout_focus: None,
            generation: 1,
            batch: Arc::new(Batch::default()),
            batch_key: None,
            gpu_stats: Arc::new(Mutex::new(GpuStats::default())),
            selection,
            canvas_clicks: CanvasClicks::default(),
            explorer_clicks: CanvasClicks::default(),
            navigation: Navigation::default(),
            world: World::System,
            focus: None,
            collapsed: BTreeSet::new(),
            families: RelationshipFamily::all().into_iter().collect(),
            expanded: None,
            include_standard: false,
            world_filters: BTreeMap::new(),
            inspector: None,
            explanation: None,
            source: None,
            show_explain: false,
            show_source: false,
            palette: false,
            palette_query: String::new(),
            palette_focus: false,
            create_dialog: false,
            project_dialog: Default::default(),
            edit_target: None,
            create_dialog_focus: false,
            new_part_name: "newPart".into(),
            candidate: None,
            preparation: None,
            last_preparation_cancellation: None,
            comparison: ComparisonMode::Current,
            compare_before: None,
            dependencies: None,
            show_agent: false,
            agent_activity: None,
            agent_return: None,
            search: String::new(),
            status: "Ready".into(),
            fit_pending: true,
            focus_changes_pending: false,
            requested_definition: None,
            deferred_definition: None,
            marquee_start: None,
            marquee_end: None,
            timing: FrameTiming::default(),
            frame_number: 0,
            capture_requested: false,
            capture_done: false,
            session_path,
            restore,
            last_saved: Instant::now(),
            adapter,
        };
        if app.fixture.as_deref() == Some("requirements") {
            app.world = World::Requirements;
        }
        if matches!(
            app.fixture.as_deref(),
            Some("stress1000" | "stress10000" | "ports")
        ) {
            app.world = World::Graph;
        }
        if app.fixture.is_some()
            && let Some(session) = app.restore.take()
            && let Some(presentation) = session.presentation
        {
            app.apply_saved_presentation(presentation);
        }
        app.rebuild();
        if app.fixture.as_deref() == Some("diff") {
            app.show_fixture_diff();
        }
        if let Some(id) = app
            .projection
            .nodes
            .iter()
            .find(|n| n.name == "ModelRepository")
            .map(|n| n.id)
        {
            app.select(agq_studio_scene::SceneTarget::Node(id), false);
        }
        app.record_location();
        Ok(app)
    }
    pub fn project_id(&self) -> Option<ProjectId> {
        self.binding.map(|b| b.project)
    }
    pub fn display_snapshot(&self) -> DisplayState {
        DisplayState {
            projection: self.projection.clone(),
            candidate: self.candidate.as_ref().map(|candidate| CandidateDisplay {
                id: candidate.id,
                before: candidate.before.clone(),
                after: candidate.after.clone(),
                review_selection: candidate.review_selection.clone(),
            }),
            comparison: self.comparison,
            compare_before: self.compare_before.clone(),
            world: self.world,
            focus: self.focus,
            selection: self.selection.clone(),
            camera: self.camera,
            camera_target: self.camera_target,
            layout: self.layout.clone(),
            layouts: self.layouts.clone(),
            layout_world: self.layout_world,
            layout_focus: self.layout_focus,
            collapsed: self.collapsed.clone(),
            expanded: self.expanded.clone(),
            families: self.families.clone(),
            include_standard: self.include_standard,
            world_filters: self.world_filters.clone(),
            show_agent: self.show_agent,
            dependencies: self.dependencies.clone(),
            agent_activity: self.agent_activity.clone(),
            agent_return: self.agent_return.clone(),
            fit_pending: self.fit_pending,
            focus_changes_pending: self.focus_changes_pending,
            requested_definition: self.requested_definition.clone(),
            deferred_definition: self.deferred_definition.clone(),
        }
    }
    pub fn restore_display(&mut self, previous: DisplayState) {
        self.scene_builder.invalidate();
        self.projection = previous.projection;
        if let (Some(current), Some(old)) = (&mut self.candidate, previous.candidate)
            && current.id == old.id
        {
            current.before = old.before;
            current.after = old.after;
            current.review_selection = old.review_selection;
        }
        self.comparison = previous.comparison;
        self.compare_before = previous.compare_before;
        self.world = previous.world;
        self.focus = previous.focus;
        self.selection = previous.selection;
        self.camera = previous.camera;
        self.camera_target = previous.camera_target;
        self.layout = previous.layout;
        self.layouts = previous.layouts;
        self.layout_world = previous.layout_world;
        self.layout_focus = previous.layout_focus;
        self.collapsed = previous.collapsed;
        self.expanded = previous.expanded;
        self.families = previous.families;
        self.include_standard = previous.include_standard;
        self.world_filters = previous.world_filters;
        self.show_agent = previous.show_agent;
        self.dependencies = previous.dependencies;
        self.agent_activity = previous.agent_activity;
        self.agent_return = previous.agent_return;
        self.fit_pending = previous.fit_pending;
        self.focus_changes_pending = previous.focus_changes_pending;
        self.requested_definition = previous.requested_definition;
        self.deferred_definition = previous.deferred_definition;
        self.invalidate_inspection();
        self.batch_key = None;
    }
    pub fn rebuild(&mut self) -> bool {
        self.rebuild_with_background(true)
    }
    /// Revision/candidate/return swaps must finish before publishing matching DTOs.
    pub fn rebuild_immediate(&mut self) -> bool {
        self.rebuild_with_background(false)
    }
    fn rebuild_with_background(&mut self, background: bool) -> bool {
        if (self.layout_world, self.layout_focus) != (self.world, self.focus) {
            self.layouts
                .insert((self.layout_world, self.layout_focus), self.layout.clone());
            self.layout = self
                .layouts
                .get(&(self.world, self.focus))
                .cloned()
                .unwrap_or_default();
            // Retain every coordinate system referenced by bounded navigation
            // plus each world's Home. Key-order eviction could otherwise pair
            // an old Back camera with a newly packed layout.
            if self.layouts.len() > 24 {
                let retained = self.navigation.retained_views();
                self.layouts
                    .retain(|key, _| key.1.is_none() || retained.contains(key));
            }
            self.layout_world = self.world;
            self.layout_focus = self.focus;
        }
        // A loaded current-neighborhood ID set cannot classify removals. Diff
        // keeps the complete paired query scope, including genuine old ghosts.
        let expanded = (self.comparison != ComparisonMode::Diff)
            .then_some(self.expanded.as_ref())
            .flatten();
        let visible_projection = |projection: &ViewProjection| {
            crate::scene_build::presentation_projection(
                projection,
                &self.families,
                expanded,
                self.world == World::System,
            )
        };
        let projection = visible_projection(self.active_projection());
        let options = SceneOptions {
            collapsed: self.collapsed.clone(),
            focus: ownership_focus(&projection, self.world, self.focus, self.fixture.is_some()),
            hierarchy: matches!(self.world, World::System | World::History),
        };
        let input = crate::scene_build::SceneInput {
            projection,
            before: (self.comparison == ComparisonMode::Diff)
                .then(|| {
                    self.candidate
                        .as_ref()
                        .map(|c| &c.before)
                        .or(self.compare_before.as_ref())
                        .map(visible_projection)
                })
                .flatten(),
            options,
            memory: self.layout.clone(),
        };
        // Large same-revision presentation changes retain the explorable previous
        // scene while layout, routing and indexes build on a dedicated worker.
        // Revision swaps stay atomic until all revision-bound UI is staged.
        if background
            && input.projection.nodes.len() >= 750
            && input.projection.revision_id == self.scene.revision_id
            && self.pending_revision.is_none()
        {
            if let Err(error) = self.scene_builder.request(input) {
                self.status = error;
                return false;
            }
        } else {
            self.scene_builder.invalidate();
            match crate::scene_build::build(input) {
                Ok(built) => self.install_scene(built),
                Err(error) => {
                    self.status = error;
                    return false;
                }
            }
        }
        true
    }
    fn install_scene(&mut self, built: crate::scene_build::BuiltScene) {
        self.layout = built.scene.memory().clone();
        self.spatial = built.spatial;
        self.lookup = built.lookup;
        self.outliner_order = built.outliner;
        self.selection.reconcile(&built.scene);
        self.scene = built.scene;
        self.generation += 1;
        self.batch_key = None;
        self.invalidate_inspection();
        self.timing.scene_ms = built.build_ms;
        self.timing.layout_ms = built.scene_ms;
        self.timing.index_ms = built.index_ms;
    }
    fn receive_scene(&mut self) {
        if let Some(result) = self.scene_builder.poll() {
            match result {
                Ok(built) => {
                    self.install_scene(built);
                    self.request_inspection();
                }
                Err(error) => self.status = format!("View rebuild failed: {error}"),
            }
        }
    }
    pub fn active_projection(&self) -> &ViewProjection {
        if self.comparison != ComparisonMode::Current
            && let Some(candidate) = &self.candidate
        {
            &candidate.after
        } else {
            &self.projection
        }
    }
    pub fn animate(&mut self, ctx: &egui::Context) {
        if let Some(target) = self.camera_target {
            let dt = ctx.input(|i| i.stable_dt).min(0.05);
            let amount = if self.reduced_motion {
                1.0
            } else {
                1.0 - (-dt * 16.0).exp()
            };
            self.camera.center.x += (target.center.x - self.camera.center.x) * amount;
            self.camera.center.y += (target.center.y - self.camera.center.y) * amount;
            self.camera.zoom += (target.zoom - self.camera.zoom) * amount;
            if (target.zoom - self.camera.zoom).abs() < 0.001
                && (target.center.x - self.camera.center.x).abs()
                    + (target.center.y - self.camera.center.y).abs()
                    < 0.2
            {
                self.camera = target;
                self.camera_target = None;
                self.navigation
                    .update_camera([target.center.x, target.center.y], target.zoom);
            } else {
                ctx.request_repaint();
            }
        }
    }
    pub fn save_session(&mut self) {
        if !self.ready
            || self.pending_revision.is_some()
            || self.deferred_definition.is_some()
            || self
                .requested_definition
                .as_ref()
                .is_some_and(|(request, _)| {
                    *request == self.scene_request && self.pending.contains(request)
                })
            || self.args.screenshot.is_some()
            || self.args.frames.is_some()
            || self
                .args
                .scenario
                .as_deref()
                .is_some_and(|scenario| scenario != "presentation")
        {
            return;
        }
        // Never restore a process-local candidate as durable model state.
        let session = Session {
            version: 1,
            project: self.project_id(),
            revision: self.projection.revision_id,
            fixture: self.fixture.clone(),
            world: self.world,
            focus: self.focus,
            camera: self.camera,
            layout: self.layout.clone(),
            dark: self.theme.dark,
            high_contrast: self.theme.contrast,
            reduced_motion: self.reduced_motion,
            presentation: Some(self.capture_presentation()),
        };
        if let Err(error) = session.save(&self.session_path) {
            self.status = format!("Presentation state was not saved: {error}");
        }
        self.last_saved = Instant::now();
    }
}

impl eframe::App for StudioApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            match event {
                egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                    self.ime_composing = !text.is_empty()
                }
                egui::Event::Ime(egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled) => {
                    self.ime_composing = false
                }
                _ => {}
            }
        }
        if let Some(scenario) = &self.args.scenario {
            let outcome = if crate::presentation_automation::is_scenario(Some(scenario)) {
                crate::presentation_automation::drive(self, ctx, input)
            } else if matches!(scenario.as_str(), "real" | "real-restart") {
                crate::real_automation::drive(self, ctx, input, &self.args.scenario_report)
            } else if scenario == "stress" {
                crate::stress_automation::drive(self, ctx, input, &self.args.scenario_report)
            } else {
                crate::automation::drive(self, ctx, input, scenario, &self.args.scenario_report)
            };
            match outcome {
                Ok(crate::automation::ScenarioStatus::Running) => {}
                Ok(crate::automation::ScenarioStatus::Complete) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                }
                Err(error) => {
                    eprintln!("Native interaction FAILED: {error}");
                    std::process::exit(2);
                }
            }
        }
        self.timing.raw_input(input);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_number += 1;
        self.timing.frame();
        self.receive();
        self.receive_scene();
        if let Some(message) = crate::surface_recovery::device_fault(ctx) {
            // Keep processing in-flight semantic outcomes, but do not accept new
            // blind editor actions while its graphics device cannot show them.
            let first_notice = self.status != message;
            self.status = message;
            if first_notice || self.last_saved.elapsed() > Duration::from_secs(8) {
                self.save_session();
            }
            self.timing.ui_complete();
            return;
        }
        if let Some(message) = crate::surface_recovery::surface_fault(ctx) {
            self.status = message.clone();
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Graphics surface unavailable");
                ui.label(message);
            });
            // Resizing or a later input can acquire a surface again. The minimal
            // recovery view accepts no model-edit commands while pixels are stale.
            crate::surface_recovery::paint_heartbeat(ctx);
            self.timing.ui_complete();
            return;
        }
        self.animate(ctx);
        self.keyboard(ctx);
        if self.ready {
            self.shell(ctx);
        } else {
            self.setup(ctx);
        }
        self.dialogs(ctx);
        crate::surface_recovery::paint_heartbeat(ctx);
        self.capture(ctx);
        if self.args.frames.is_some() {
            ctx.request_repaint();
        } else if self.args.scenario.is_some() {
            // raw_input_hook runs before begin_pass, which can clear a delayed
            // repaint requested by a scenario. Keep its opt-in input clock in
            // the actual UI pass, including when every visible widget is idle.
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        if self.last_saved.elapsed() > Duration::from_secs(8) {
            self.save_session();
        }
        self.timing.ui_complete();
    }
    fn on_exit(&mut self) {
        self.save_session();
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.theme.canvas.to_normalized_gamma_f32()
    }
}

pub fn fixture_projection(name: &str) -> ViewProjection {
    match name {
        "typography" => {
            fixtures::adversarial()
                .into_iter()
                .find(|(name, _)| *name == "long-unicode-names")
                .expect("retained typography stress fixture")
                .1
        }
        "ports" => fixtures::dense_ports(),
        "requirements" => fixtures::requirements(),
        "stress1000" => fixtures::stress(1000, 2000),
        "stress10000" => fixtures::stress(10000, 20000),
        _ => fixtures::architecture(),
    }
}

/// Restore only known disposable fixtures at their exact retained revision.
/// A persisted fixture name can never select a real project or invent a model.
fn restorable_fixture(session: &Session) -> Option<(String, ViewProjection)> {
    let name = session.fixture.as_deref()?;
    if session.project.is_some()
        || session.presentation.is_none()
        || !matches!(
            name,
            "architecture" | "typography" | "ports" | "requirements" | "stress1000" | "stress10000"
        )
    {
        return None;
    }
    let projection = if session.world == World::Requirements {
        fixtures::requirements()
    } else {
        fixture_projection(name)
    };
    (projection.revision_id == session.revision).then(|| (name.to_owned(), projection))
}

fn initial_projection(fixture: Option<&str>) -> ViewProjection {
    if let Some(name) = fixture {
        return fixture_projection(name);
    }
    // An unloaded presentation has no semantic objects, including for command
    // discovery. This sentinel revision is never submitted to a model service.
    ViewProjection {
        revision_id: agq_modeling_workspace::ProjectRevisionId::from_u128(0),
        view: agq_modeling_view::ViewDefinition {
            name: "No project open".into(),
            ..Default::default()
        },
        nodes: vec![],
        edges: vec![],
        groups: vec![],
        metadata: agq_modeling_view::ViewMetadata {
            suggested_focus: None,
            scope: "No project open".into(),
            producer_completeness: "Unavailable".into(),
            local_element_count: 0,
            omitted_standard_endpoints: 0,
            warnings: vec![],
        },
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // Platform fonts are read locally, never redistributed. Built-in fonts remain fallback.
    for path in [
        "C:/Windows/Fonts/segoeui.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "studio-ui".into(),
                Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "studio-ui".into());
            break;
        }
    }
    for (name, paths) in [
        (
            "studio-symbols",
            vec![
                "C:/Windows/Fonts/seguisym.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
            ],
        ),
        (
            "studio-cjk",
            vec![
                "C:/Windows/Fonts/msyh.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            ],
        ),
    ] {
        for path in paths {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push(name.into());
                break;
            }
        }
    }
    ctx.set_fonts(fonts);
}

pub fn muted(text: impl Into<String>, theme: Theme) -> egui::RichText {
    egui::RichText::new(text).color(theme.muted)
}
pub fn short_revision(id: agq_modeling_workspace::ProjectRevisionId) -> String {
    format!(
        "{:04x}…{:04x}",
        (id.as_u128() >> 112) as u16,
        id.as_u128() as u16
    )
}

pub(crate) fn hierarchy_order(scene: &SemanticScene) -> Vec<usize> {
    hierarchy_order_with_focus(scene, None)
}

pub(crate) fn hierarchy_order_with_focus(
    scene: &SemanticScene,
    focus: Option<ElementId>,
) -> Vec<usize> {
    let ids: BTreeSet<_> = scene.nodes.iter().map(|n| n.id()).collect();
    let mut children = BTreeMap::<Option<ElementId>, Vec<usize>>::new();
    for (index, node) in scene.nodes.iter().enumerate() {
        children
            .entry(node.semantic.owner.filter(|id| ids.contains(id)))
            .or_default()
            .push(index);
    }
    let mut stack = children.get(&None).cloned().unwrap_or_default();
    stack.sort_by_key(|index| {
        (
            Some(scene.nodes[*index].id()) != focus,
            scene.nodes[*index].semantic.name.to_lowercase(),
        )
    });
    stack.reverse();
    let mut ordered = Vec::with_capacity(scene.nodes.len());
    let mut seen = BTreeSet::new();
    while let Some(index) = stack.pop() {
        if !seen.insert(index) {
            continue;
        }
        ordered.push(index);
        if let Some(owned) = children.get(&Some(scene.nodes[index].id())) {
            stack.extend(owned.iter().rev().copied());
        }
    }
    ordered
}

/// A real focused architecture projection has already crossed canonical ownership
/// and typing edges. A second lexical subtree filter would discard definitions
/// referenced by its parts (for example ModelRepository inside ModelingPlatform).
/// Fixtures and temporary local navigation still use presentation-only ownership.
fn ownership_focus(
    projection: &ViewProjection,
    world: World,
    focus: Option<ElementId>,
    fixture: bool,
) -> Option<ElementId> {
    if world != World::System
        || (!fixture
            && projection.view.kind == agq_modeling_view::ViewKind::Architecture
            && projection.view.focus == focus)
    {
        None
    } else {
        focus
    }
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

    #[test]
    fn focused_semantic_architecture_keeps_referenced_definitions_outside_lexical_owner() {
        let mut projection = fixture_projection("architecture");
        let focus = projection
            .nodes
            .iter()
            .find(|n| n.name == "ModelingPlatform")
            .unwrap()
            .id;
        let external = projection
            .nodes
            .iter_mut()
            .find(|n| n.name == "ModelRepository")
            .unwrap();
        external.owner = None;
        let external_id = external.id;
        projection.view.focus = Some(focus);
        let options = SceneOptions {
            focus: ownership_focus(&projection, World::System, Some(focus), false),
            ..Default::default()
        };
        let scene = SemanticScene::from_projection(&projection, &options, None).unwrap();
        assert!(scene.node(external_id).is_some());
        assert_eq!(scene.node(external_id).unwrap().semantic.owner, None);
        let fixture_options = SceneOptions {
            focus: ownership_focus(&projection, World::System, Some(focus), true),
            ..Default::default()
        };
        let local = SemanticScene::from_projection(&projection, &fixture_options, None).unwrap();
        assert!(local.node(external_id).is_none());
    }

    #[test]
    fn unloaded_workspace_has_no_hidden_fixture_objects_or_selection_targets() {
        let projection = initial_projection(None);
        assert!(projection.nodes.is_empty());
        assert!(projection.edges.is_empty());
        assert!(projection.groups.is_empty());
        assert_eq!(projection.metadata.producer_completeness, "Unavailable");
        let scene =
            SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
        assert!(scene.nodes.is_empty());
        assert!(scene.ports.is_empty());
        assert!(hierarchy_order(&scene).is_empty());
        assert_eq!(initial_projection(Some("architecture")).nodes.len(), 12);
    }

    #[test]
    fn fixture_session_restoration_requires_known_name_exact_revision_and_no_project() {
        let projection = fixture_projection("architecture");
        let presentation = crate::saved_views::SavedPresentation {
            world: World::System,
            definition: projection.view,
            camera: Camera2D::default(),
            layout: LayoutMemory::default(),
            collapsed: BTreeSet::new(),
            expanded: None,
            branch: None,
            panels: Default::default(),
            selection: None,
        };
        let mut session = Session {
            version: 1,
            project: None,
            revision: projection.revision_id,
            fixture: Some("architecture".into()),
            world: World::System,
            focus: None,
            camera: Camera2D::default(),
            layout: LayoutMemory::default(),
            dark: true,
            high_contrast: false,
            reduced_motion: false,
            presentation: Some(presentation),
        };
        assert!(restorable_fixture(&session).is_some());
        session.fixture = Some("unknown".into());
        assert!(restorable_fixture(&session).is_none());
        session.fixture = Some("architecture".into());
        session.project = Some(ProjectId::new());
        assert!(restorable_fixture(&session).is_none());
        session.project = None;
        session.revision = agq_modeling_repository::ProjectRevisionId::new();
        assert!(restorable_fixture(&session).is_none());
        session.revision = projection.revision_id;
        session.presentation = None;
        assert!(restorable_fixture(&session).is_none());
    }
}
