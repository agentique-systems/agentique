//! The Studio application: owns the open project and the Surface, and
//! updates the Surface from every change event.
use crate::{
    Args,
    commands::{self, CommandContext, CommandId},
    gpu::{Batch, GpuStats},
    navigation::SurfaceView,
    selection::{CanvasClicks, Selection},
    session::{ProjectView, Session},
    theme::Theme,
    timing::FrameTiming,
};
use agq_studio_scene::{
    Camera2D, EdgeKind, ElementId, LayoutKind, LayoutMemory, LodController, Scene, SceneInput,
    SceneLookup, SceneOptions, SceneTarget, SpatialIndex, fixtures,
};
use agq_system_state::{ChangeEvent, Project, ProjectError};
use eframe::egui;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Definitions a type can be chosen from: kind, name and reference.
pub type TypeOptions = Vec<(agq_language::ElementKind, String, agq_language::Reference)>;

/// The Panel shown on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Inspector,
    Requirements,
    History,
}

pub struct StudioApp {
    pub args: Args,
    /// Windows' theme as last applied (`follow_windows`).
    system_theme: Option<egui::Theme>,
    /// Settings, and the view that edits them (Ctrl+,).
    pub settings: crate::settings_ui::SettingsView,
    /// Today's estimated cost of the Assistant (R-42).
    pub daily_cost: crate::cost::DailyCost,
    pub theme: Theme,
    pub reduced_motion: bool,
    pub adapter: String,
    /// The open project. `None` shows a read-only fixture or the start screen.
    pub project: Option<Project>,
    pub fixture: Option<String>,
    pub view: SurfaceView,
    /// What the Surface shows, built from the model after every change.
    pub input: SceneInput,
    pub scene: Scene,
    pub spatial: SpatialIndex,
    pub lookup: SceneLookup,
    pub generation: u64,
    pub camera: Camera2D,
    pub camera_target: Option<Camera2D>,
    /// How the camera moves towards `camera_target`.
    pub camera_move: Option<crate::motion::CameraMove>,
    pub lod: LodController,
    pub layouts: BTreeMap<SurfaceView, LayoutMemory>,
    pub collapsed: BTreeSet<ElementId>,
    pub focus: Option<ElementId>,
    pub selection: Selection,
    /// An element chosen in the Inspector that has no card, with the
    /// selection it was chosen from.
    pub inspected: Option<(Option<SceneTarget>, ElementId)>,
    pub canvas_clicks: CanvasClicks,
    /// Recently created or changed elements, highlighted where they are.
    /// Start of each highlight on `gpu::clock()`; the fade runs on the GPU.
    pub highlights: BTreeMap<ElementId, f32>,
    /// A "what changed" comparison shown on the Surface.
    pub comparison: Option<crate::history::Comparison>,
    pub gesture: Option<crate::viewport::Gesture>,
    pub batch: Arc<Batch>,
    pub batch_key: Option<u64>,
    pub gpu_stats: Arc<Mutex<GpuStats>>,
    pub palette: bool,
    pub palette_query: String,
    pub palette_focus: bool,
    pub dialog: Option<crate::edit::Dialog>,
    pub panel: Panel,
    pub history: crate::history::HistoryPanel,
    pub status: String,
    /// Whether the last change was saved, with the reason when it was not.
    pub saved: Result<(), String>,
    /// Problem messages at each element, for the current model.
    pub problems: BTreeMap<ElementId, Vec<String>>,
    /// Definitions a type can be chosen from, for the current model.
    pub type_options: Option<(u64, TypeOptions)>,
    /// The Requirements Panel's rows, for the current model.
    pub requirement_rows: Option<(u64, Vec<crate::requirements::Row>)>,
    pub timing: FrameTiming,
    pub frame_number: u64,
    pub capture_requested: bool,
    pub capture_done: bool,
    pub session: Session,
    pub session_path: PathBuf,
    pub fit_pending: bool,
    /// The Surface's size in the last frame.
    pub surface_size: Option<egui::Vec2>,
    /// The Conversation with the Assistant, for the open project.
    pub conversation: crate::conversation::ConversationPanel,
    last_saved: Instant,
    saved_layout: crate::surface_recovery::SavedLayout,
}

impl StudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>, args: Args) -> Self {
        let session_path = args.session.clone().unwrap_or_else(Session::default_path);
        let loaded = Session::load(&session_path);
        let session_loaded = loaded.is_some();
        let session = loaded.unwrap_or(Session {
            version: Session::VERSION,
            dark: !args.light,
            ..Default::default()
        });
        // Settings sit beside the session, so `--session` keeps tests and
        // journeys away from the Operator's own.
        let settings =
            crate::settings_ui::SettingsView::load(session_path.with_file_name("settings.json"));
        let daily_cost = crate::cost::DailyCost::load(&session_path);
        let mut settings = settings;
        let mut session = session;
        // Appearance lives in Settings; a session from Stage 4 gives its
        // theme, contrast and reduced motion to Settings once.
        if !session.appearance_in_settings {
            let chosen = ["appearance.theme", "appearance.reducedMotion"]
                .iter()
                .any(|id| settings.settings.changed(id));
            if session_loaded && !chosen {
                settings.adopt_appearance(
                    session.dark,
                    session.high_contrast,
                    session.reduced_motion,
                );
            }
            session.appearance_in_settings = true;
        }
        let (dark, contrast, reduced) = appearance(&settings, &cc.egui_ctx);
        let theme = Theme::new(dark && !args.light, contrast);
        theme.install(&cc.egui_ctx);
        if let Some(scale) = ui_scale(&args, &settings) {
            cc.egui_ctx.set_zoom_factor(scale);
        }
        // Scripted journeys run without animation so positions are final.
        let reduced_motion = reduced || args.scenario_running();
        // Both the dark and the light style, so switching theme keeps it.
        cc.egui_ctx.all_styles_mut(|style| {
            style.animation_time = if reduced_motion {
                0.0
            } else {
                crate::theme::HOVER_SECONDS
            };
        });
        let adapter = cc
            .wgpu_render_state
            .as_ref()
            .map(|s| format!("{:?}", s.adapter.get_info()))
            .unwrap_or_default();
        let input = SceneInput::default();
        let scene = Scene::build(&input, &SceneOptions::default(), None)
            .expect("an empty scene always builds");
        let mut app = Self {
            spatial: SpatialIndex::build(&scene),
            lookup: SceneLookup::build(&scene),
            scene,
            input,
            args,
            theme,
            reduced_motion,
            adapter,
            project: None,
            fixture: None,
            view: SurfaceView::Architecture,
            generation: 0,
            camera: Camera2D::default(),
            camera_target: None,
            camera_move: None,
            lod: LodController::default(),
            layouts: BTreeMap::new(),
            collapsed: BTreeSet::new(),
            focus: None,
            selection: Selection::default(),
            inspected: None,
            canvas_clicks: CanvasClicks::default(),
            highlights: BTreeMap::new(),
            comparison: None,
            gesture: None,
            batch: Arc::new(Batch::default()),
            batch_key: None,
            gpu_stats: Arc::new(Mutex::new(GpuStats::default())),
            palette: false,
            palette_query: String::new(),
            palette_focus: false,
            dialog: None,
            panel: Panel::Inspector,
            history: Default::default(),
            status: String::new(),
            saved: Ok(()),
            problems: BTreeMap::new(),
            type_options: None,
            requirement_rows: None,
            timing: FrameTiming::default(),
            frame_number: 0,
            capture_requested: false,
            capture_done: false,
            session,
            session_path,
            fit_pending: true,
            surface_size: None,
            conversation: crate::conversation::ConversationPanel::with_choice(
                settings.model_choice(),
            ),
            daily_cost,
            settings,
            system_theme: None,
            last_saved: Instant::now(),
            saved_layout: Default::default(),
        };
        #[cfg(feature = "automation")]
        if app.args.scenario.as_deref() == Some("a-assistant") {
            crate::automation::script_assistant(&mut app);
        }
        if let Some(name) = app.args.fixture.clone() {
            app.show_fixture(&name);
        } else if let Some(folder) = app.args.project.clone() {
            if !app.args.creates_project() {
                app.open_project(&folder);
            }
        } else if !app.args.no_restore
            && let Some(folder) = app.session.project.clone()
        {
            app.open_project(&folder);
        }
        app
    }

    /// Whether the model can be edited now: a project is open and the
    /// Surface is not showing an earlier checkpoint.
    pub fn editable(&self) -> bool {
        self.project.is_some()
            && self
                .comparison
                .as_ref()
                .is_none_or(|comparison| comparison.after_is_now)
    }

    pub fn context(&self) -> CommandContext {
        let container = self.selected_card().is_some();
        let pair = self.selection.targets.len() == 2
            && self.selection.targets.iter().all(|t| {
                matches!(
                    t,
                    SceneTarget::Port(..) | SceneTarget::Node(_) | SceneTarget::Container(_)
                )
            });
        let state = self.project.as_ref().map(Project::state);
        CommandContext {
            busy: self.dialog.is_some() || self.settings.open,
            project: self.project.is_some(),
            editable: self.editable(),
            selected: self.selection.primary.is_some(),
            pair,
            container,
            can_undo: state.is_some_and(|s| s.undo_description().is_some()),
            can_redo: state.is_some_and(|s| s.redo_description().is_some()),
            graph_view: self.view == SurfaceView::Graph,
            focused: self.focus.is_some(),
        }
    }

    /// The card of the primary selection: a selected card, or the card a
    /// selected port is shown on.
    pub fn selected_card(&self) -> Option<ElementId> {
        match self.selection.primary.as_ref()? {
            SceneTarget::Node(id) | SceneTarget::Container(id) => Some(*id),
            SceneTarget::Port(card, _) => Some(*card),
            SceneTarget::Edge(_) => None,
        }
    }

    /// The element the Inspector shows: one chosen there without a card, or
    /// the selected element.
    pub fn inspected_element(&self) -> Option<ElementId> {
        self.inspected
            .as_ref()
            .filter(|(from, _)| *from == self.selection.primary)
            .map(|(_, id)| *id)
            .or_else(|| self.selection.element(&self.scene))
    }

    /// The elements the Operator is working on: the element chosen in the
    /// Inspector (such as an attribute line), else every selected element.
    pub fn working_elements(&self) -> Vec<ElementId> {
        match self
            .inspected
            .as_ref()
            .filter(|(from, _)| *from == self.selection.primary)
        {
            Some((_, id)) => vec![*id],
            None => self.selection.elements(&self.scene),
        }
    }

    /// The definition that owns the port the Operator is working on, when the
    /// port is shown on a usage through its type.
    pub fn shared_definition(&self) -> Option<ElementId> {
        let SceneTarget::Port(card, port) = self.selection.primary.as_ref()? else {
            return None;
        };
        if self.inspected_element() != Some(*port) {
            return None;
        }
        self.lookup.port(&self.scene, *card, *port)?;
        let tree = self.project.as_ref()?.state().tree();
        tree.get(*port)?.owner().filter(|owner| owner != card)
    }

    pub fn select(&mut self, target: SceneTarget, extend: bool) {
        self.selection.select(target, extend);
        self.batch_key = None;
    }

    // Projects and fixtures.

    pub fn open_project(&mut self, folder: &Path) {
        // Opening the project that is open reads it again from disk: this
        // window's project must let go of the folder first.
        let same = |a: &Path, b: &Path| match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => a == b,
        };
        if self
            .project
            .as_ref()
            .is_some_and(|project| same(project.folder(), folder))
        {
            self.save_session();
            self.project = None;
        }
        match Project::open(folder) {
            Ok(project) => self.install_project(project),
            Err(ProjectError::Locked) => {
                self.status = format!("{} is open in another Agentique window", folder.display());
            }
            Err(error) => {
                self.status = format!("Could not open {}: {error}", folder.display());
                self.session.recent.retain(|recent| recent != folder);
                if self.session.project.as_deref() == Some(folder) {
                    self.session.project = None;
                }
            }
        }
    }

    /// Creates a project holding the URL shortener sample (R-46): a new
    /// project whose model is then replaced by the sample and read again.
    pub fn create_sample(&mut self, folder: &Path, name: &str) {
        self.create_project(folder, name);
        let Some(project) = &self.project else { return };
        let file = project.folder().join("model").join(format!("{name}.sysml"));
        let text = SAMPLE.replace(
            &format!("package {SAMPLE_NAME}"),
            &format!("package {name}"),
        );
        if let Err(error) = std::fs::write(&file, text) {
            self.status = format!("The sample could not be written: {error}");
            return;
        }
        let folder = project.folder().to_path_buf();
        self.open_project(&folder);
    }

    pub fn create_project(&mut self, folder: &Path, name: &str) {
        match Project::create(folder, name) {
            Ok(project) => self.install_project(project),
            Err(ProjectError::Locked) => {
                self.status = format!("{} is open in another Agentique window", folder.display())
            }
            Err(error) => self.status = format!("Could not create the project: {error}"),
        }
    }

    fn install_project(&mut self, project: Project) {
        self.save_session();
        let folder = project.folder().to_path_buf();
        let remembered = self.session.views.get(&folder).cloned().unwrap_or_default();
        self.fixture = None;
        self.load_conversation(&folder);
        self.project = Some(project);
        self.view = remembered.view;
        self.layouts = remembered.layouts;
        self.collapsed.clear();
        self.focus = None;
        self.selection.clear();
        self.comparison = None;
        self.highlights.clear();
        self.history = Default::default();
        match remembered.camera {
            Some(camera) => {
                self.camera = camera;
                self.fit_pending = false;
            }
            None => self.fit_pending = true,
        }
        self.camera_target = None;
        self.session.remember(&folder);
        let unmatched: Vec<String> = self
            .project
            .as_ref()
            .map(|p| p.unmatched().to_vec())
            .unwrap_or_default();
        self.status = if unmatched.is_empty() {
            format!("Opened {}", folder.display())
        } else {
            let listed: Vec<&str> = unmatched.iter().take(3).map(String::as_str).collect();
            format!(
                "Opened {}. The model text was edited outside Agentique: {} element(s) could not be matched to their saved identity and were treated as new ({}{}). Locks and history links on them were not carried over.",
                folder.display(),
                unmatched.len(),
                listed.join(", "),
                if unmatched.len() > 3 { ", …" } else { "" }
            )
        };
        self.refresh();
        self.save_session();
    }

    pub fn show_fixture(&mut self, name: &str) {
        self.end_turn();
        self.project = None;
        self.fixture = Some(name.to_string());
        self.view = match name {
            "requirements" => SurfaceView::Requirements,
            "stress1000" | "stress10000" | "ports" => SurfaceView::Graph,
            _ => SurfaceView::Architecture,
        };
        self.fit_pending = true;
        self.refresh();
        if name == "diff" {
            let (before, after) = fixtures::change_trees();
            self.comparison = Some(crate::history::Comparison::of_trees(
                "URL shortener",
                before,
                "edited",
                &after,
                self.input.clone(),
                false,
            ));
            self.rebuild();
        }
    }

    // Building the Surface.

    /// Rebuilds the scene input from the model (or fixture), then the scene.
    pub fn refresh(&mut self) {
        self.generation += 1;
        self.input = match (&self.project, self.fixture.as_deref()) {
            (Some(project), _) => {
                let state = project.state();
                let mut counts = BTreeMap::new();
                self.problems.clear();
                for diagnostic in state.diagnostics() {
                    *counts.entry(diagnostic.element).or_insert(0) += 1;
                    self.problems
                        .entry(diagnostic.element)
                        .or_default()
                        .push(diagnostic.message.clone());
                }
                self.history.uncommitted = project.has_uncommitted_changes().ok();
                SceneInput::from_tree(state.tree(), state.locks(), &counts, self.generation)
            }
            (None, Some(name)) => {
                let mut input = match name {
                    "typography" => fixtures::typography(),
                    "ports" => fixtures::dense_ports(),
                    "stress1000" => fixtures::stress(1000, 2000),
                    "stress10000" => fixtures::stress(10000, 20000),
                    "diff" => fixtures::change().1,
                    // The gallery shows no Surface.
                    "components" => SceneInput::default(),
                    _ => fixtures::architecture(),
                };
                input.generation = self.generation;
                input
            }
            (None, None) => SceneInput {
                generation: self.generation,
                ..Default::default()
            },
        };
        if let (Some(comparison), Some(project)) = (&mut self.comparison, &self.project)
            && comparison.after_is_now
        {
            comparison.update_now(project.state().tree(), &self.input);
        }
        self.rebuild();
    }

    /// Lays out the current input for the current view.
    pub fn rebuild(&mut self) {
        let started = Instant::now();
        let after = match &self.comparison {
            Some(comparison) if !comparison.after_is_now => comparison.after.clone(),
            _ => self.input.clone(),
        };
        let shown = self.view_input(&after);
        let options = self.scene_options();
        let memory = self.layouts.get(&self.view);
        let built = match &self.comparison {
            // Removed cards are laid out with the current ones, never on top.
            Some(comparison) => Scene::comparison(
                &self.view_input(&comparison.before),
                &shown,
                &comparison.changed,
                &options,
                memory,
            ),
            // An edit lays out the cards again and routes only the edges it
            // touches (S5.1, R-28).
            None => self.scene.update(&shown, &options, memory),
        };
        let scene = match built {
            Ok(scene) => scene,
            Err(error) => {
                self.status = format!("The Surface could not be laid out: {error}");
                return;
            }
        };
        self.timing.layout_ms = started.elapsed().as_secs_f64() * 1000.0;
        let indexed = Instant::now();
        // A comparison's extra cards do not change the remembered layout.
        if self.comparison.is_none() {
            self.layouts.insert(self.view, scene.memory().clone());
        }
        self.spatial = SpatialIndex::build(&scene);
        self.lookup = SceneLookup::build(&scene);
        self.selection.reconcile(&scene);
        self.scene = scene;
        self.batch_key = None;
        self.timing.index_ms = indexed.elapsed().as_secs_f64() * 1000.0;
        self.timing.scene_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    fn view_input(&self, input: &SceneInput) -> SceneInput {
        match self.view {
            SurfaceView::Architecture => {
                let mut input = input.clone();
                input.retain_edges(&[EdgeKind::Connection, EdgeKind::Interface, EdgeKind::Satisfy]);
                input
            }
            SurfaceView::Graph => input.clone(),
            SurfaceView::Requirements => input.requirements_view(),
        }
    }

    fn scene_options(&self) -> SceneOptions {
        SceneOptions {
            collapsed: self.collapsed.clone(),
            focus: self
                .focus
                .filter(|_| self.view == SurfaceView::Architecture),
            layout: match self.view {
                SurfaceView::Architecture => LayoutKind::Hierarchy,
                SurfaceView::Graph => LayoutKind::Graph,
                SurfaceView::Requirements => LayoutKind::Requirements,
            },
        }
    }

    /// Updates the Surface after a change, undo or redo, and highlights what
    /// was created or changed where it is.
    pub fn changed(&mut self, event: &ChangeEvent) {
        self.refresh();
        let now = crate::gpu::clock();
        let tree = self.project.as_ref().map(|p| p.state().tree());
        // An owner changes when a member is added or removed; highlight the
        // member, not the whole owner.
        let owners: std::collections::BTreeSet<ElementId> = event
            .created
            .iter()
            .filter_map(|id| tree.and_then(|t| t.get(*id)).and_then(|e| e.owner()))
            .collect();
        // After a delete, the containers that lost a member are not highlighted.
        let only_deleted = !event.deleted.is_empty() && event.created.is_empty();
        let updated = event.updated.iter().filter(|id| {
            let container = self
                .lookup
                .node(&self.scene, **id)
                .is_some_and(|n| n.is_container);
            let skipped = owners.contains(id) || (only_deleted && container);
            !skipped
        });
        for id in event.created.iter().chain(updated) {
            // Highlight the element's card, or the nearest card that owns it.
            let mut current = Some(*id);
            while let Some(element) = current {
                let shown = self.lookup.node(&self.scene, element).is_some()
                    || self.scene.ports.iter().any(|p| p.id == element)
                    || self
                        .scene
                        .edges
                        .iter()
                        .any(|e| e.semantic.element == Some(element));
                if shown {
                    self.highlights.insert(element, now);
                    break;
                }
                current = tree.and_then(|t| t.get(element)).and_then(|e| e.owner());
            }
        }
        self.batch_key = None;
        // Bring new cards into view.
        let visible = self.camera.visible_rect();
        let outside = event.created.iter().any(|id| {
            self.lookup
                .node(&self.scene, *id)
                .is_some_and(|n| !visible.contains_rect(n.bounds))
        });
        if outside {
            self.frame_all();
        }
    }

    pub fn set_view(&mut self, view: SurfaceView) {
        if self.view != view {
            self.view = view;
            self.fit_pending = !self.layouts.contains_key(&view);
            self.rebuild();
            if self.fit_pending {
                self.frame_all();
            }
        }
    }

    /// Frames the model. The architecture view frames its structure (usages,
    /// connections and the definitions that contain them); definitions that
    /// own nothing are below it.
    pub fn frame_all(&mut self) {
        let mut target = self.camera;
        let has_secondary = self.scene.nodes.iter().any(|n| n.secondary);
        let bounds =
            if self.view == SurfaceView::Architecture && self.comparison.is_none() && has_secondary
            {
                // The structure, with the title of the package around it.
                let mut structure = self.scene.structure_bounds();
                if let Some(top) = self
                    .scene
                    .nodes
                    .iter()
                    .filter(|n| n.category == agq_studio_scene::NodeCategory::Package)
                    .filter(|n| n.bounds.contains_rect(structure))
                    .map(|n| n.bounds.min.y)
                    .reduce(f32::max)
                {
                    structure = agq_studio_scene::Rect::new(
                        structure.min.x,
                        top,
                        structure.width(),
                        structure.max.y - top,
                    );
                }
                structure
            } else {
                self.scene.bounds()
            };
        target.fit(bounds, 42.0);
        target.zoom = target.zoom.min(1.3);
        // When more follows below the structure, show the structure at the top.
        let whole = self.scene.bounds();
        let half = target.viewport.height * 0.5 / target.zoom;
        if whole.max.y > bounds.max.y + 1.0 && bounds.height() < 2.0 * half {
            target.center.y = bounds.min.y - 42.0 / target.zoom + half;
        }
        // The first fit, while the window opens, is instant.
        if self.frame_number > 4 && !self.reduced_motion {
            self.camera_target = Some(target);
            self.camera_move = Some(crate::motion::CameraMove::tween(
                self.camera,
                crate::gpu::clock(),
                self.reduced_motion,
            ));
        } else {
            self.camera = target;
            self.camera_target = None;
        }
        self.fit_pending = false;
    }

    pub fn frame_target(&mut self, target: &SceneTarget) {
        if let Some(bounds) = self.scene.target_bounds(target) {
            let mut camera = self.camera;
            camera.fit(bounds, 120.0);
            camera.zoom = camera.zoom.clamp(0.4, 1.4);
            if self.reduced_motion {
                self.camera = camera;
                self.camera_target = None;
            } else {
                self.camera_move = Some(crate::motion::CameraMove::follow(
                    self.camera_move,
                    self.camera,
                ));
                self.camera_target = Some(camera);
            }
        }
    }

    pub fn animate(&mut self, ctx: &egui::Context) {
        // Direct manipulation clears `camera_target`; the move goes with it.
        match self.camera_target {
            Some(target) => {
                let dt = ctx.input(|i| i.stable_dt).min(0.05);
                let camera = self.camera;
                let motion = self
                    .camera_move
                    .get_or_insert_with(|| crate::motion::CameraMove::follow(None, camera));
                if motion.step(
                    &mut self.camera,
                    target,
                    crate::gpu::clock(),
                    dt,
                    self.reduced_motion,
                ) {
                    self.camera_target = None;
                    self.camera_move = None;
                } else {
                    ctx.request_repaint();
                }
            }
            None => self.camera_move = None,
        }
        // Highlights fade on the GPU; the app only repaints while one lasts
        // and rebuilds the batch when one ends.
        let now = crate::gpu::clock();
        let before = self.highlights.len();
        self.highlights
            .retain(|_, started| now - *started < crate::theme::CHANGED_SECONDS);
        if self.highlights.len() != before {
            self.batch_key = None;
        }
        if !self.highlights.is_empty() {
            ctx.request_repaint();
        }
    }

    /// Runs a command from the keyboard, the palette or a menu.
    pub fn execute(&mut self, id: CommandId, ctx: &egui::Context) {
        if let Some(reason) = commands::unavailable(id, &self.context()) {
            self.status = reason.to_string();
            return;
        }
        use CommandId::*;
        match id {
            Architecture => self.set_view(SurfaceView::Architecture),
            Graph => self.set_view(SurfaceView::Graph),
            Requirements => {
                self.set_view(SurfaceView::Requirements);
                self.panel = Panel::Requirements;
            }
            Fit => self.frame_all(),
            ZoomToSelection => {
                if let Some(target) = self.selection.primary.clone() {
                    self.frame_target(&target);
                }
            }
            GoToElement => {
                // The palette, searching elements only.
                self.palette = true;
                self.palette_focus = true;
                self.palette_query = "focus: ".into();
            }
            ShortcutHelp => self.settings.show(crate::settings_ui::Section::Keyboard),
            Focus => {
                if let Some(card) = self.selected_card() {
                    self.view = SurfaceView::Architecture;
                    self.focus = Some(card);
                    self.rebuild();
                    self.frame_all();
                }
            }
            LeaveFocus => {
                self.focus = None;
                self.rebuild();
                self.frame_all();
            }
            Collapse => {
                if let Some(card) = self.selected_card() {
                    if !self.collapsed.remove(&card) {
                        self.collapsed.insert(card);
                    }
                    self.rebuild();
                }
            }
            Pin | Unpin => {
                if let Some(card) = self.selected_card()
                    && let Some(node) = self.lookup.node(&self.scene, card)
                {
                    let bounds = node.bounds;
                    let memory = self.layouts.entry(self.view).or_default();
                    if id == Pin {
                        let _ = memory.pin(card, bounds);
                    } else {
                        memory.unpin(card);
                    }
                    self.rebuild();
                }
            }
            History => self.panel = Panel::History,
            Palette => {
                self.palette = true;
                self.palette_focus = true;
                self.palette_query.clear();
            }
            NewProject => self.dialog = Some(crate::edit::Dialog::new_project()),
            OpenProject => {
                self.dialog = Some(crate::edit::Dialog::OpenProject {
                    folder: String::new(),
                })
            }
            // The appearance commands change the setting, so Settings shows
            // it and it holds at the next start.
            Theme => {
                let theme = if self.theme.dark { "light" } else { "dark" };
                self.settings
                    .set_value("appearance.theme", serde_json::json!(theme));
                self.apply_appearance(ctx);
            }
            Contrast => {
                let theme = match (self.theme.contrast, self.theme.dark) {
                    (false, _) => "high-contrast",
                    (true, true) => "dark",
                    (true, false) => "light",
                };
                self.settings
                    .set_value("appearance.theme", serde_json::json!(theme));
                self.apply_appearance(ctx);
            }
            ReducedMotion => {
                let reduced = if self.reduced_motion { "off" } else { "on" };
                self.settings
                    .set_value("appearance.reducedMotion", serde_json::json!(reduced));
                self.apply_appearance(ctx);
            }
            Settings => {
                if self.settings.open {
                    self.settings.close();
                } else {
                    self.settings.show(crate::settings_ui::Section::Providers);
                }
            }
            CreatePart | CreatePort | CreateItem | CreateAttribute | CreateInterface
            | CreateRequirement | Rename | Delete | Connect | MoveTo | Lock | Undo | Redo
            | Checkpoint => self.edit(id),
            AskAssistant => {
                self.conversation.shown = true;
                self.conversation.focus_input = true;
            }
            InsertSelection => self.insert_selection(),
            NewConversation => self.new_conversation(),
            ShowConversation => self.conversation.shown = !self.conversation.shown,
        }
    }

    /// Runs the command bound to a key pressed on the Surface.
    fn keyboard(&mut self, ctx: &egui::Context) {
        if self.settings.open {
            // Settings takes the keyboard; Ctrl+, or Escape closes it.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
                self.settings.close();
            }
            return;
        }
        if ctx.egui_wants_keyboard_input() || self.palette || self.dialog.is_some() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.selection.clear();
            self.batch_key = None;
            return;
        }
        // Shift+1 and Shift+2 by their place on the keyboard, since the
        // characters they type differ between layouts; Home fits too.
        if consume_shifted(ctx, egui::Key::Num1)
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home))
        {
            self.execute(CommandId::Fit, ctx);
            return;
        }
        if consume_shifted(ctx, egui::Key::Num2) {
            self.execute(CommandId::ZoomToSelection, ctx);
            return;
        }
        // Redo also answers to Ctrl+Shift+Z.
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )
        }) {
            self.execute(CommandId::Redo, ctx);
            return;
        }
        for command in commands::COMMANDS {
            if let Some((modifiers, key)) = command.key
                && ctx.input_mut(|i| i.consume_key(modifiers, key))
            {
                self.execute(command.id, ctx);
                return;
            }
        }
    }

    // The session.

    pub fn save_session(&mut self) {
        if let Err(error) = self.write_session() {
            self.status = format!("The session was not saved: {error}");
        }
    }

    fn write_session(&mut self) -> Result<(), String> {
        self.last_saved = Instant::now();
        if self.args.screenshot.is_some() || self.args.frames.is_some() {
            return Ok(());
        }
        if let Some(project) = &self.project {
            self.session.views.insert(
                project.folder().to_path_buf(),
                ProjectView {
                    view: self.view,
                    camera: Some(self.camera_target.unwrap_or(self.camera)),
                    layouts: self.layouts.clone(),
                },
            );
        }

        self.session
            .save(&self.session_path)
            .map_err(|error| error.to_string())
    }

    fn graphics_fault(&mut self, ctx: &egui::Context, message: &str, device: bool) {
        if self.saved_layout.needs_attempt(message) {
            let result = self.write_session().map(|()| true);
            self.saved_layout.record(result);
            crate::surface_recovery::checkpoint_title(ctx, device, self.saved_layout.failed());
        }
        self.status = self.saved_layout.status(message);
        ctx.request_repaint_after(Duration::from_secs(8));
    }

    pub fn capture(&mut self, ctx: &egui::Context) {
        // After the camera has settled, too (a fit view moves for 300 ms).
        if self.args.screenshot.is_some()
            && !self.capture_requested
            && self.frame_number >= 20
            && self.camera_target.is_none()
        {
            self.capture_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot
            && let Some(path) = &self.args.screenshot
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            match image::save_buffer(
                path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            ) {
                Ok(()) => println!("Screenshot saved: {}", path.display()),
                Err(error) => eprintln!("Screenshot failed: {error}"),
            }
            self.capture_done = true;
        }
        if self.args.screenshot.is_some() && !self.capture_done {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        let finished = self
            .args
            .frames
            .is_some_and(|frames| self.frame_number >= frames)
            || (self.args.frames.is_none() && self.args.screenshot.is_some() && self.capture_done);
        if finished
            && !self.args.scenario_running()
            && (self.args.screenshot.is_none() || self.capture_done)
        {
            let emitted = egui::Id::new("studio-metrics-emitted");
            if ctx
                .data(|data| data.get_temp::<bool>(emitted))
                .unwrap_or(false)
            {
                return;
            }
            let report = self.metrics_report();
            if let Some(path) = &self.args.metrics
                && let Err(error) = std::fs::write(path, report.to_string())
            {
                eprintln!("Cannot write the metrics: {error}");
            }
            println!("{report}");
            ctx.data_mut(|data| data.insert_temp(emitted, true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub fn metrics_report(&self) -> serde_json::Value {
        let stats = self.gpu_stats.lock().ok();
        let frames = self.timing.frame_summary();
        serde_json::json!({
            "fixture": self.fixture,
            "adapter": self.adapter,
            "frame_count": self.frame_number,
            "start_to_first_update_ms": self.timing.start_to_first_update_ms,
            // The start budget holds for the start screen; a fixture's scene
            // is built before the first frame.
            "budgets": if self.fixture.is_none() {
                vec![crate::budgets::result(
                    "start to first update (warm)",
                    crate::budgets::START_TO_FIRST_UPDATE_MS,
                    self.timing.start_to_first_update_ms,
                )]
            } else {
                Vec::new()
            },
            "frame_interval_median_ms": frames.median,
            "frame_interval_p95_ms": frames.p95,
            "scene_build_ms": self.timing.scene_ms,
            "layout_ms": self.timing.layout_ms,
            "index_ms": self.timing.index_ms,
            "edit_to_frame_ms": self.timing.edits(),
            "edges_routed": self.scene.routing().routed,
            "edges_kept": self.scene.routing().kept,
            "warmup_frame_intervals_discarded": self.timing.discarded_frame_intervals(),
            "input_pipeline": self.timing.latency_report(),
            "hit_test_us": self.timing.hit_summary(),
            "input_to_next_update_ms": {
                "pan": self.timing.input_summary(crate::timing::InputKind::Pan),
                "zoom": self.timing.input_summary(crate::timing::InputKind::Zoom),
                "selection": self.timing.input_summary(crate::timing::InputKind::Selection),
            },
            "gpu_upload_cpu_ms": stats.as_ref().filter(|s| s.uploads > 0).map(|s| s.upload_ms),
            "gpu_instances": stats.as_ref().map(|s| s.instances),
            "visible_nodes": self.timing.visible_nodes,
            "total_nodes": self.scene.nodes.len(),
            "total_edges": self.scene.edges.len(),
            "gpu_timestamp_ms": stats.as_ref().map(|s| s.timestamp_ms.summary()),
        })
    }
}

impl eframe::App for StudioApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        #[cfg(feature = "automation")]
        crate::automation::raw_input(self, _ctx, input);
        self.timing.raw_input(input);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        self.frame_number += 1;
        self.timing.frame();
        if let Some(message) = crate::surface_recovery::device_fault(ctx) {
            // No edits while the Surface cannot show them.
            self.graphics_fault(ctx, &message, true);
            self.timing.ui_complete();
            return;
        }
        if let Some(message) = crate::surface_recovery::surface_fault(ctx) {
            self.graphics_fault(ctx, &message, false);
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("Graphics surface unavailable");
                ui.label(&self.status);
            });
            crate::surface_recovery::paint_heartbeat(ctx);
            self.timing.ui_complete();
            return;
        }
        self.saved_layout = Default::default();
        self.follow_windows(ctx);
        self.animate(ctx);
        self.poll_conversation();
        if self.conversation.running() && self.conversation.waiting.is_none() {
            // Streamed text and tool calls arrive from the Assistant's
            // thread; while it waits for the Operator, nothing arrives.
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        self.keyboard(ctx);
        self.shell(ui, ctx);
        self.dialogs(ctx);
        if self.palette {
            self.command_palette(ctx);
        }
        crate::surface_recovery::paint_heartbeat(ctx);
        self.capture(ctx);
        if self.args.frames.is_some() {
            ctx.request_repaint();
        } else if self.args.scenario_running() {
            // The scripted input clock runs in the UI pass.
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

impl StudioApp {
    /// Applies what Settings changed: the appearance at once, the model
    /// from the next turn.
    pub fn apply_settings(&mut self, ctx: &egui::Context, changed: crate::settings_ui::Changed) {
        if changed.appearance {
            self.apply_appearance(ctx);
        }
        if changed.assistant {
            self.conversation.use_choice(self.settings.model_choice());
        }
    }

    /// "Follow Windows" follows it: egui learns Windows' theme at the first
    /// frame and whenever it changes.
    pub fn follow_windows(&mut self, ctx: &egui::Context) {
        let system = ctx.system_theme();
        if system != self.system_theme {
            self.system_theme = system;
            self.apply_appearance(ctx);
        }
    }

    fn apply_appearance(&mut self, ctx: &egui::Context) {
        let (dark, contrast, reduced) = appearance(&self.settings, ctx);
        self.theme = Theme::new(dark && !self.args.light, contrast);
        self.theme.install(ctx);
        self.batch_key = None;
        self.reduced_motion = reduced || self.args.scenario_running();
        let animation_time = if self.reduced_motion {
            0.0
        } else {
            crate::theme::HOVER_SECONDS
        };
        ctx.all_styles_mut(|style| style.animation_time = animation_time);
        if let Some(scale) = ui_scale(&self.args, &self.settings) {
            ctx.set_zoom_factor(scale);
        }
    }
}

/// Dark, high contrast and reduced motion as Settings has them. "Follow
/// Windows" follows its light or dark theme; egui does not report Windows'
/// animation setting, so reduced motion is off unless chosen.
fn appearance(
    settings: &crate::settings_ui::SettingsView,
    ctx: &egui::Context,
) -> (bool, bool, bool) {
    let system_dark = ctx.system_theme() != Some(egui::Theme::Light);
    let (dark, contrast) = match settings.text("appearance.theme").as_str() {
        "light" => (false, false),
        "dark" => (true, false),
        "high-contrast" => (system_dark, true),
        _ => (system_dark, false),
    };
    let reduced = settings.text("appearance.reducedMotion") == "on";
    (dark, contrast, reduced)
}

/// `--ui-scale` wins over the setting (100% when not set).
fn ui_scale(args: &Args, settings: &crate::settings_ui::SettingsView) -> Option<f32> {
    if let Some(scale) = args.ui_scale.filter(|scale| scale.is_finite()) {
        return Some(scale.clamp(0.5, 3.0));
    }
    let scale = settings
        .settings
        .get("appearance.uiScale")
        .as_f64()
        .unwrap_or(1.0);
    Some((scale as f32).clamp(1.0, 2.0))
}

/// Takes a key pressed with Shift (and no other modifier) by its physical
/// place, whatever character it types.
fn consume_shifted(ctx: &egui::Context, physical: egui::Key) -> bool {
    ctx.input_mut(|input| {
        let found = input.events.iter().position(|event| {
            matches!(
                event,
                egui::Event::Key { physical_key: Some(key), pressed: true, modifiers, .. }
                    if *key == physical && modifiers.shift && !modifiers.command && !modifiers.alt
            )
        });
        found.map(|index| input.events.remove(index)).is_some()
    })
}

/// The URL shortener of Scenario A, offered on the first run (R-46).
pub const SAMPLE: &str = include_str!("../../../models/url-shortener/UrlShortener.sysml");
pub const SAMPLE_NAME: &str = "UrlShortener";

pub fn muted(text: impl Into<String>, theme: Theme) -> egui::RichText {
    egui::RichText::new(text).color(theme.muted)
}
