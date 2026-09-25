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
    ElementInspector, ExplanationProjection, RelationshipFamily, ViewProjection,
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
    pub before: ViewProjection,
    pub after: ViewProjection,
    pub source: String,
    pub review_selection: Option<Selection>,
}

pub struct PendingPreparation {
    pub request: u64,
    pub started: Instant,
    pub cancelled: bool,
}

pub struct StudioApp {
    pub args: Args,
    pub theme: Theme,
    pub reduced_motion: bool,
    pub ime_composing: bool,
    pub ready: bool,
    pub fixture: Option<String>,
    pub config: NativeConfig,
    pub setup_reason: String,
    pub bundle_path: String,
    pub projects: Vec<Project>,
    pub binding: Option<RevisionBinding>,
    pub branch: Option<BranchId>,
    pub history: Option<ProjectHistory>,
    pub bridge: Bridge,
    pub pending: BTreeSet<u64>,
    pub scene_request: u64,
    pub inspector_request: u64,
    pub explanation_request: u64,
    pub source_request: u64,
    pub project_request: u64,
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
    pub layouts: BTreeMap<World, LayoutMemory>,
    pub layout_world: World,
    pub generation: u64,
    pub batch: Arc<Batch>,
    pub batch_key: Option<u64>,
    pub gpu_stats: Arc<Mutex<GpuStats>>,
    pub selection: Selection,
    pub canvas_clicks: CanvasClicks,
    pub navigation: Navigation,
    pub world: World,
    pub focus: Option<ElementId>,
    pub collapsed: BTreeSet<ElementId>,
    pub families: BTreeSet<RelationshipFamily>,
    pub expanded: Option<BTreeSet<ElementId>>,
    pub include_standard: bool,
    pub inspector: Option<ElementInspector>,
    pub explanation: Option<ExplanationProjection>,
    pub source: Option<SourceProjection>,
    pub show_explain: bool,
    pub show_source: bool,
    pub palette: bool,
    pub palette_query: String,
    pub palette_focus: bool,
    pub create_dialog: bool,
    pub new_part_name: String,
    pub candidate: Option<Candidate>,
    pub preparation: Option<PendingPreparation>,
    pub comparison: ComparisonMode,
    pub compare_before: Option<ViewProjection>,
    pub dependencies: Option<BTreeSet<ElementId>>,
    pub show_agent: bool,
    pub agent_activity: Option<crate::agents::DependencyActivity>,
    pub search: String,
    pub status: String,
    pub fit_pending: bool,
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
        let session_path = config.database.with_extension("native-session.json");
        let restore = if args.no_restore || args.fixture.is_some() {
            None
        } else {
            Session::load(&session_path)
        };
        let fixture = args.fixture.clone();
        let projection = initial_projection(fixture.as_deref());
        let scene = SemanticScene::from_projection(&projection, &SceneOptions::default(), None)?;
        let spatial = SpatialIndex::build(&scene);
        let lookup = SceneLookup::build(&scene);
        let outliner_order = hierarchy_order(&scene);
        let layout = scene.memory().clone();
        let selection = Selection::new(projection.revision_id);
        let dark = restore.as_ref().map_or(!args.light, |r| r.dark);
        let theme = Theme::new(dark, restore.as_ref().is_some_and(|r| r.high_contrast));
        theme.install(&cc.egui_ctx);
        let setup = agq_studio_platform::setup_surface(&config)?;
        let mut bridge = Bridge::new(cc.egui_ctx.clone());
        let mut pending = BTreeSet::new();
        if fixture.is_none() && setup.bundle_located {
            pending.insert(
                bridge
                    .open(config.clone(), None)
                    .map_err(std::io::Error::other)?,
            );
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
            fixture,
            config,
            setup_reason: setup
                .reason
                .unwrap_or_else(|| "Authenticating accepted publications…".into()),
            bundle_path: String::new(),
            projects: vec![],
            binding: None,
            branch: None,
            history: None,
            bridge,
            pending,
            scene_request: 0,
            inspector_request: 0,
            explanation_request: 0,
            source_request: 0,
            project_request: 0,
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
            generation: 1,
            batch: Arc::new(Batch::default()),
            batch_key: None,
            gpu_stats: Arc::new(Mutex::new(GpuStats::default())),
            selection,
            canvas_clicks: CanvasClicks::default(),
            navigation: Navigation::default(),
            world: World::System,
            focus: None,
            collapsed: BTreeSet::new(),
            families: RelationshipFamily::all().into_iter().collect(),
            expanded: None,
            include_standard: false,
            inspector: None,
            explanation: None,
            source: None,
            show_explain: false,
            show_source: false,
            palette: false,
            palette_query: String::new(),
            palette_focus: false,
            create_dialog: false,
            new_part_name: "newPart".into(),
            candidate: None,
            preparation: None,
            comparison: ComparisonMode::Current,
            compare_before: None,
            dependencies: None,
            show_agent: false,
            agent_activity: None,
            search: String::new(),
            status: "Ready".into(),
            fit_pending: true,
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
    pub fn rebuild(&mut self) {
        if self.layout_world != self.world {
            self.layouts.insert(self.layout_world, self.layout.clone());
            self.layout = self.layouts.get(&self.world).cloned().unwrap_or_default();
            self.layout_world = self.world;
        }
        let mut projection = self.active_projection().clone();
        let hidden: BTreeSet<_> = projection.view.hidden_elements.iter().copied().collect();
        projection.nodes.retain(|node| !hidden.contains(&node.id));
        projection
            .edges
            .retain(|edge| !hidden.contains(&edge.source) && !hidden.contains(&edge.target));
        projection
            .edges
            .retain(|e| self.families.contains(&e.family));
        if let Some(expanded) = &self.expanded {
            projection.nodes.retain(|n| expanded.contains(&n.id));
            projection
                .edges
                .retain(|e| expanded.contains(&e.source) && expanded.contains(&e.target));
        }
        let options = SceneOptions {
            collapsed: self.collapsed.clone(),
            focus: if self.world == World::System {
                self.focus
            } else {
                None
            },
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
                        .cloned()
                })
                .flatten(),
            options,
            memory: self.layout.clone(),
        };
        // Large same-revision presentation changes retain the explorable previous
        // scene while layout, routing and indexes build on a dedicated worker.
        // Revision swaps stay atomic until all revision-bound UI is staged.
        if input.projection.nodes.len() >= 750
            && input.projection.revision_id == self.scene.revision_id
        {
            if let Err(error) = self.scene_builder.request(input) {
                self.status = error;
            }
        } else {
            self.scene_builder.invalidate();
            match crate::scene_build::build(input) {
                Ok(built) => self.install_scene(built),
                Err(error) => self.status = error,
            }
        }
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
            || self.args.screenshot.is_some()
            || self.args.frames.is_some()
            || self.args.scenario.is_some()
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
            let outcome = if scenario == "stress" {
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
                    eprintln!("Native fixture interaction FAILED: {error}");
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
        self.animate(ctx);
        self.keyboard(ctx);
        if self.ready {
            self.shell(ctx);
        } else {
            self.setup(ctx);
        }
        self.dialogs(ctx);
        self.capture(ctx);
        if self.args.frames.is_some() {
            ctx.request_repaint();
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
        "ports" => fixtures::dense_ports(),
        "requirements" => fixtures::requirements(),
        "stress1000" => fixtures::stress(1000, 2000),
        "stress10000" => fixtures::stress(10000, 20000),
        _ => fixtures::architecture(),
    }
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
    let ids: BTreeSet<_> = scene.nodes.iter().map(|n| n.id()).collect();
    let mut children = BTreeMap::<Option<ElementId>, Vec<usize>>::new();
    for (index, node) in scene.nodes.iter().enumerate() {
        children
            .entry(node.semantic.owner.filter(|id| ids.contains(id)))
            .or_default()
            .push(index);
    }
    let mut stack = children.get(&None).cloned().unwrap_or_default();
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

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

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
}
