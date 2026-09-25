use crate::{
    Args,
    bridge::Bridge,
    gpu::{Batch, GpuStats},
    navigation::{Navigation, World},
    selection::Selection,
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
}

pub struct StudioApp {
    pub args: Args,
    pub theme: Theme,
    pub reduced_motion: bool,
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
    pub comparison: ComparisonMode,
    pub compare_before: Option<ViewProjection>,
    pub dependencies: Option<BTreeSet<ElementId>>,
    pub show_agent: bool,
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
        let projection = fixture_projection(fixture.as_deref().unwrap_or("architecture"));
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
            comparison: ComparisonMode::Current,
            compare_before: None,
            dependencies: None,
            show_agent: false,
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
        let started = Instant::now();
        if self.layout_world != self.world {
            self.layouts.insert(self.layout_world, self.layout.clone());
            self.layout = self.layouts.get(&self.world).cloned().unwrap_or_default();
            self.layout_world = self.world;
        }
        let mut projection = self.active_projection().clone();
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
        match SemanticScene::from_projection(&projection, &options, Some(&self.layout)) {
            Ok(mut scene) => {
                if self.comparison == ComparisonMode::Diff {
                    let before = self
                        .candidate
                        .as_ref()
                        .map(|c| &c.before)
                        .or(self.compare_before.as_ref());
                    if let Some(before) = before
                        && let Ok(parent) =
                            SemanticScene::from_projection(before, &options, Some(&self.layout))
                    {
                        scene.apply_diff(&parent);
                    }
                }
                self.layout = scene.memory().clone();
                self.spatial = SpatialIndex::build(&scene);
                self.lookup = SceneLookup::build(&scene);
                self.outliner_order = hierarchy_order(&scene);
                self.selection.reconcile(&scene);
                self.scene = scene;
                self.generation += 1;
                self.batch_key = None;
                self.inspector = None;
                self.explanation = None;
                self.source = None;
                self.timing.scene_ms = started.elapsed().as_secs_f64() * 1000.0;
                self.timing.layout_ms = self.timing.scene_ms;
            }
            Err(error) => self.status = error.to_string(),
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
            } else {
                ctx.request_repaint();
            }
        }
    }
    pub fn save_session(&mut self) {
        if !self.ready || self.args.screenshot.is_some() || self.args.frames.is_some() {
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
        };
        if let Err(error) = session.save(&self.session_path) {
            self.status = format!("Presentation state was not saved: {error}");
        }
        self.last_saved = Instant::now();
    }
}

impl eframe::App for StudioApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if let Some(scenario)=&self.args.scenario {
            match crate::automation::drive(self,ctx,input,scenario,&self.args.scenario_report) {
                Ok(crate::automation::ScenarioStatus::Running)=>{},
                Ok(crate::automation::ScenarioStatus::Complete)=>ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error)=>{eprintln!("Native fixture interaction FAILED: {error}");std::process::exit(2);}
            }
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_number += 1;
        self.timing.frame();
        self.receive();
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

fn hierarchy_order(scene: &SemanticScene) -> Vec<usize> {
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
