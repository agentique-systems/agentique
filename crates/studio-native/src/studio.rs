//! The Studio's state: the open project, the Surface's scene, camera and
//! selection, the Panels and the Conversation, apart from how they are
//! drawn. Every model change goes through the project's typed operations
//! (`edit.rs`); the views read this state and mark what they change, so a
//! pan draws only the Surface again.
use crate::{
    Args,
    commands::{self, CommandContext, CommandId},
    navigation::SurfaceView,
    selection::{CanvasClicks, Selection},
    session::{ProjectView, Session},
    settings::{Section, SettingsStore},
    timing::FrameTiming,
};
use agq_studio_scene::{
    Camera2D, EdgeKind, ElementId, LayoutKind, LayoutMemory, LodController, Scene, SceneInput,
    SceneLookup, SceneOptions, SceneTarget, SpatialIndex, fixtures,
};
use agq_system_state::{ChangeEvent, Project, ProjectError};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

/// Definitions a type can be chosen from: kind, name and reference.
pub type TypeOptions = Vec<(agq_language::ElementKind, String, agq_language::Reference)>;

/// The Panel shown in the Inspector column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Inspector,
    /// The selected scenario, its runs and its trace (C-50).
    Run,
    Requirements,
    History,
    Problems,
}

/// What changed in the Studio since the views last heard, so each view is
/// drawn again only for what it shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dirty(u16);

impl Dirty {
    /// The camera moved (pan, zoom, a camera move running).
    pub const CAMERA: Dirty = Dirty(1);
    pub const SELECTION: Dirty = Dirty(1 << 1);
    /// The model, the scene or the view changed.
    pub const MODEL: Dirty = Dirty(1 << 2);
    pub const CONVERSATION: Dirty = Dirty(1 << 3);
    /// Panels shown, hidden or resized.
    pub const LAYOUT: Dirty = Dirty(1 << 4);
    pub const APPEARANCE: Dirty = Dirty(1 << 5);
    /// The status line, the saved state.
    pub const STATUS: Dirty = Dirty(1 << 6);
    /// A dialog, the palette or Settings opened or closed.
    pub const OVERLAY: Dirty = Dirty(1 << 7);
    pub const ALL: Dirty = Dirty(u16::MAX);

    pub fn intersects(self, other: Dirty) -> bool {
        self.0 & other.0 != 0
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Dirty {
    type Output = Dirty;
    fn bitor(self, other: Dirty) -> Dirty {
        Dirty(self.0 | other.0)
    }
}

impl std::ops::BitOrAssign for Dirty {
    fn bitor_assign(&mut self, other: Dirty) {
        self.0 |= other.0;
    }
}

/// What the views must draw again after an update of the Studio.
#[derive(Clone, Copy, Debug)]
pub struct StudioEvent(pub Dirty);

/// Which palette is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    /// Every command and element (Ctrl+K).
    Commands,
    /// Elements only (Ctrl+P, "go to element").
    Elements,
    /// Building blocks to insert ("Insert from Library…", Shift+A); with
    /// `fit`, only those that fit the Library's chosen port, connected to it.
    Library { fit: bool },
    /// The usages and specialisations of a definition ("Find usages").
    Usages(agq_language::ElementId),
}

/// What the left column shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LeftTab {
    #[default]
    Outline,
    Library,
    /// The project's scenarios (C-50).
    Scenarios,
}

/// The theme the Studio shows: dark or light, and high contrast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appearance {
    pub dark: bool,
    pub contrast: bool,
}

/// What Windows says about appearance, read by the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct System {
    pub dark: bool,
    /// Windows' "Animation effects" is off.
    pub reduced_motion: bool,
}

pub struct Studio {
    pub args: Args,
    pub settings: SettingsStore,
    /// Settings is shown in place of the Surface (Ctrl+,), at this section.
    pub settings_open: bool,
    pub settings_section: Section,
    /// Today's estimated cost of the Assistant (R-42).
    pub daily_cost: crate::cost::DailyCost,
    pub system: System,
    pub appearance: Appearance,
    pub reduced_motion: bool,
    /// The open project. `None` shows a read-only fixture or the start screen.
    pub project: Option<Project>,
    pub fixture: Option<String>,
    pub view: SurfaceView,
    /// What the Surface shows, built from the model after every change.
    pub input: SceneInput,
    /// Shared with the Surface's paint, which runs after the views are built.
    pub scene: Rc<Scene>,
    pub spatial: Rc<SpatialIndex>,
    pub lookup: Rc<SceneLookup>,
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
    /// Recently created or changed elements, highlighted where they are,
    /// with when on `motion::clock()` and by whom.
    pub highlights: BTreeMap<ElementId, (f32, agq_system_state::Actor)>,
    /// A "what changed" comparison shown on the Surface.
    pub comparison: Option<crate::history::Comparison>,
    pub gesture: Option<crate::surface::Gesture>,
    pub palette: Option<PaletteMode>,
    /// Commands run from the palette, most recent first (at most five).
    pub recent_commands: Vec<CommandId>,
    /// Focus mode (Ctrl+\): the Surface takes the whole window.
    pub panels_hidden: bool,
    /// The Outline (Ctrl+B) and the Inspector column (Ctrl+Alt+B) collapsed.
    pub outline_hidden: bool,
    pub inspector_hidden: bool,
    /// Widths of the docked columns, remembered per project.
    pub widths: crate::session::Widths,
    pub dialog: Option<crate::edit::Dialog>,
    pub panel: Panel,
    /// The left column: the Outline or the Library.
    pub left: LeftTab,
    /// The Library of building blocks (C-49).
    pub library: crate::library::LibraryState,
    /// Definitions opened and elements focused, to go back to (breadcrumb).
    pub drill: Vec<crate::library::Drill>,
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
    /// Frames the Surface has drawn.
    pub frame_number: u64,
    pub session: Session,
    pub session_path: PathBuf,
    pub fit_pending: bool,
    /// The Surface's size in the last frame, in pixels.
    pub surface_size: Option<(f32, f32)>,
    /// The Conversation with the Assistant, for the open project.
    pub conversation: crate::conversation::ConversationPanel,
    /// Scenarios, runs and their traces (C-50).
    pub runs: crate::runs::RunsState,
    /// The Claude Agent runtime's setup and health check (C-51).
    pub runtime: crate::agent_runtime::RuntimeState,
    /// Started in safe mode (`--safe-mode`, the launcher's recovery): the
    /// Claude Agent runtime is not used, whatever Settings say.
    pub safe_mode: bool,
    /// Agentique's own builds: building, trying, adopting (C-51).
    pub develop: crate::develop::BuildsState,
    /// The control interface (C-53): observations and agents' actions.
    pub control: crate::control::ControlState,
    /// Implementation links, checks and drift (C-50).
    pub implementation: crate::implementation::ImplementationState,
    last_saved: Instant,
    /// What changed since the views last heard.
    dirty: Dirty,
}

impl Studio {
    pub fn new(args: Args, system: System) -> Self {
        let session_path = args.session.clone().unwrap_or_else(Session::default_path);
        let loaded = Session::load(&session_path);
        let session_loaded = loaded.is_some();
        let mut session = loaded.unwrap_or(Session {
            version: Session::VERSION,
            dark: !args.light,
            ..Default::default()
        });
        // Settings sit beside the session, so `--session` keeps tests and
        // journeys away from the Operator's own.
        let mut settings = SettingsStore::load(session_path.with_file_name("settings.json"));
        let daily_cost = crate::cost::DailyCost::load(&session_path);
        // My Library beside the session too.
        let library = crate::library::LibraryState::new(
            session_path
                .with_file_name("library")
                .join("My Library.sysml"),
        );
        // Appearance lives in Settings; a session from Stage 4 gives its
        // theme, contrast and reduced motion to Settings once.
        if !session.appearance_in_settings {
            let chosen = ["appearance.theme", "appearance.reducedMotion"]
                .iter()
                .any(|id| settings.changed(id));
            if session_loaded && !chosen {
                settings.adopt_appearance(
                    session.dark,
                    session.high_contrast,
                    session.reduced_motion,
                );
            }
            session.appearance_in_settings = true;
        }
        let input = SceneInput::default();
        let scene = Scene::build(&input, &SceneOptions::default(), None)
            .expect("an empty scene always builds");
        let conversation =
            crate::conversation::ConversationPanel::with_choice(settings.model_choice());
        let args_safe_mode = args.safe_mode;
        let mut studio = Self {
            spatial: Rc::new(SpatialIndex::build(&scene)),
            lookup: Rc::new(SceneLookup::build(&scene)),
            scene: Rc::new(scene),
            input,
            args,
            settings,
            settings_open: false,
            settings_section: Section::Providers,
            daily_cost,
            system,
            appearance: Appearance {
                dark: true,
                contrast: false,
            },
            reduced_motion: false,
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
            palette: None,
            recent_commands: Vec::new(),
            panels_hidden: false,
            outline_hidden: false,
            inspector_hidden: false,
            widths: Default::default(),
            dialog: None,
            panel: Panel::Inspector,
            left: LeftTab::Outline,
            library,
            drill: Vec::new(),
            history: Default::default(),
            status: String::new(),
            saved: Ok(()),
            problems: BTreeMap::new(),
            type_options: None,
            requirement_rows: None,
            timing: FrameTiming::default(),
            frame_number: 0,
            session,
            session_path,
            fit_pending: true,
            surface_size: None,
            conversation,
            runs: Default::default(),
            runtime: Default::default(),
            safe_mode: args_safe_mode,
            develop: Default::default(),
            control: Default::default(),
            implementation: Default::default(),
            last_saved: Instant::now(),
            dirty: Dirty::ALL,
        };
        studio.apply_appearance();
        studio.apply_runtime_choice();
        studio.note_recovery();
        #[cfg(feature = "automation")]
        if studio.args.scenario.as_deref() == Some("a-assistant") {
            crate::automation::script_assistant(&mut studio);
        }
        #[cfg(feature = "automation")]
        if studio.args.scenario.as_deref() == Some("h-library") {
            crate::automation::script_library_assistant(&mut studio);
        }
        if let Some(name) = studio.args.fixture.clone() {
            studio.show_fixture(&name);
        } else if let Some(folder) = studio.args.project.clone() {
            #[cfg(feature = "automation")]
            if studio.args.scenario.as_deref() == Some("c-understand")
                && let Err(why) = crate::automation::copy_self_model(&folder)
            {
                studio.status = why;
            }
            if !studio.args.creates_project() {
                studio.open_project(&folder);
            }
        } else if !studio.args.no_restore
            && let Some(folder) = studio.session.project.clone()
        {
            studio.open_project(&folder);
        }
        studio
    }

    // What changed.

    /// Marks what an update changed. An update that marks nothing is taken
    /// to have changed everything.
    /// Opens the control interface's endpoint when `--control` names a
    /// file (C-53).
    pub fn open_control_endpoint(&mut self) {
        let Some(file) = self.args.control.clone() else {
            return;
        };
        match crate::control::server::start(&file, &self.control.instance, self.control.sender()) {
            Ok(endpoint) => self.control.endpoint = Some(endpoint),
            Err(error) => self.status = error,
        }
    }

    /// The Settings section shown, by name.
    pub fn settings_section_name(&self) -> String {
        format!("{:?}", self.settings_section).to_lowercase()
    }

    pub fn mark(&mut self, dirty: Dirty) {
        self.dirty |= dirty;
    }

    /// What changed since the last call; everything when nothing was marked.
    pub fn take_dirty(&mut self) -> Dirty {
        match std::mem::take(&mut self.dirty) {
            dirty if dirty.is_empty() => Dirty::ALL,
            dirty => dirty,
        }
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
        let tree = state.map(|s| s.tree());
        let element = self.inspected_element();
        let definition = tree
            .zip(element)
            .is_some_and(|(tree, e)| crate::library::definition_of(tree, e).is_some());
        let elements = self.selection.elements(&self.scene);
        let parts = !elements.is_empty()
            && tree.is_some_and(|tree| {
                elements.iter().all(|e| {
                    tree.get(*e)
                        .is_some_and(|e| e.kind == agq_language::ElementKind::Part)
                })
            });
        CommandContext {
            port: matches!(self.selection.primary, Some(SceneTarget::Port(..))),
            definition,
            parts,
            busy: self.dialog.is_some() || self.settings_open,
            project: self.project.is_some(),
            editable: self.editable(),
            selected: self.selection.primary.is_some(),
            pair,
            container,
            can_undo: state.is_some_and(|s| s.undo_description().is_some()),
            can_redo: state.is_some_and(|s| s.redo_description().is_some()),
            graph_view: self.view == SurfaceView::Graph,
            focused: self.focus.is_some() || !self.drill.is_empty(),
            scenario: self.runs.selected.is_some(),
            running: self.runs.running()
                || self.implementation.checking()
                || self.implementation.task.is_some(),
            trace: self
                .runs
                .result
                .as_ref()
                .is_some_and(|r| !r.trace.is_empty()),
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

    /// Selects the nearest card in `direction` from the selected one (from
    /// the middle of the view when nothing is selected), and brings it into
    /// view at once (keyboard-invoked, §8.5 rule 5). The selection ring is
    /// the focus ring (§3.5).
    pub fn select_neighbour(&mut self, direction: (f32, f32)) {
        let cards: Vec<_> = self
            .scene
            .nodes
            .iter()
            .filter(|node| node.category != agq_studio_scene::NodeCategory::Package)
            .collect();
        let from_card = self
            .selection
            .primary
            .as_ref()
            .and_then(|target| match target {
                SceneTarget::Node(id) | SceneTarget::Container(id) => {
                    cards.iter().find(|node| node.id() == *id)
                }
                _ => None,
            });
        let from = from_card.map_or(self.camera.center, |node| node.bounds.center());
        let best = cards
            .iter()
            .filter(|node| from_card.is_none_or(|current| current.id() != node.id()))
            .filter_map(|node| {
                let center = node.bounds.center();
                let (dx, dy) = (center.x - from.x, center.y - from.y);
                let along = dx * direction.0 + dy * direction.1;
                let across = (dx * direction.1 - dy * direction.0).abs();
                // Ahead, and within 60 degrees of the direction.
                (from_card.is_none() || (along > 0.0 && across <= along * 1.8))
                    .then_some((along.abs() + 2.0 * across, *node))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, node)| node);
        let Some(node) = best else { return };
        let (id, bounds, container) = (node.id(), node.bounds, node.is_container);
        let target = if container {
            SceneTarget::Container(id)
        } else {
            SceneTarget::Node(id)
        };
        self.select(target, false);
        if !self.camera.visible_rect().contains_rect(bounds) {
            self.camera.center = bounds.center();
            self.camera_target = None;
            self.camera_move = None;
        }
    }

    pub fn select(&mut self, target: SceneTarget, extend: bool) {
        self.selection.select(target, extend);
    }

    // Projects and fixtures.

    pub fn open_project(&mut self, folder: &Path) {
        // Opening the project that is open reads it again from disk: this
        // window's project must let go of the folder first.
        let same = |a: &Path, b: &Path| match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => a == b,
        };
        let reopening = self
            .project
            .as_ref()
            .is_some_and(|project| same(project.folder(), folder));
        if reopening {
            self.save_session();
            self.project = None;
        }
        let mut opened = Project::open(folder);
        // The folder's lock was this window's own. On Linux a process started
        // at that moment by another thread holds a copy of the lock file until
        // it has started, so the lock may outlive the project by a moment:
        // wait for it briefly rather than call our own folder taken.
        let deadline = Instant::now() + Duration::from_secs(2);
        while reopening && matches!(opened, Err(ProjectError::Locked)) && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(20));
            opened = Project::open(folder);
        }
        match opened {
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
    pub fn create_sample(&mut self, folder: &Path, name: &str, sample: Sample) {
        self.create_project(folder, name);
        let Some(project) = &self.project else { return };
        let file = project.folder().join("model").join(format!("{name}.sysml"));
        let text = sample.text().replace(
            &format!("package {SAMPLE_NAME}"),
            &format!("package {name}"),
        );
        if let Err(error) = std::fs::write(&file, text) {
            self.status = format!("The sample could not be written: {error}");
            return;
        }
        let folder = project.folder().to_path_buf();
        self.open_project(&folder);
        if sample == Sample::ScreeningWithCode
            && self.project.is_some()
            && let Err(why) = self.create_sample_code(name)
        {
            self.status = format!("The sample's code could not be written: {why}");
            return;
        }
        if self.project.is_some() {
            // The camera remembered for the empty project does not fit it.
            self.fit_pending = true;
            // The sample's elements are new to the project: nothing was
            // edited outside Agentique, whatever reading it again reported.
            self.status = "Started from the URL shortener sample; checkpoint it (Ctrl+S) to keep it in the history.".into();
        }
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
        self.panels_hidden = remembered.panels_hidden;
        self.outline_hidden = remembered.outline_hidden;
        self.inspector_hidden = remembered.inspector_hidden;
        self.widths = remembered.widths;
        self.conversation.shown = !remembered.conversation_hidden;
        self.collapsed.clear();
        self.collapse_library();
        self.focus = None;
        self.drill.clear();
        self.library.fit = None;
        self.selection.clear();
        self.comparison = None;
        self.highlights.clear();
        self.history = Default::default();
        self.runs = Default::default();
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
        self.load_checks();
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
        // Results and checks describe a version of the model: say so when
        // it changed, or changed back (undo).
        if self.project.is_some() {
            self.supersede_live_run();
            self.refresh_run_freshness();
            self.refresh_check_freshness_for_model();
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
        self.spatial = Rc::new(SpatialIndex::build(&scene));
        self.lookup = Rc::new(SceneLookup::build(&scene));
        self.selection.reconcile(&scene);
        self.scene = Rc::new(scene);
        self.timing.index_ms = indexed.elapsed().as_secs_f64() * 1000.0;
        self.timing.scene_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    fn view_input(&self, input: &SceneInput) -> SceneInput {
        match self.view {
            SurfaceView::Architecture => {
                let mut input = input.clone();
                input.retain_edges(&[EdgeKind::Connection, EdgeKind::Interface, EdgeKind::Satisfy]);
                self.show_inherited(&mut input);
                input
            }
            SurfaceView::Graph => input.clone(),
            SurfaceView::Requirements => input.requirements_view(),
        }
    }

    /// Inside a definition opened on the Surface (C-49), what it inherits
    /// is shown too: the general's parts inside it (dashed), and the
    /// general's connections between them, ending at its overrides where it
    /// redefines a part. Its own members and overrides are as they are.
    fn show_inherited(&self, input: &mut SceneInput) {
        let (Some(focus), Some(project)) = (self.focus, self.project.as_ref()) else {
            return;
        };
        let tree = project.state().tree();
        if !tree.get(focus).is_some_and(|e| e.kind.is_definition()) {
            return;
        }
        let semantics = agq_language::Semantics::new(tree);
        let generals: BTreeSet<ElementId> = {
            let mut all = vec![focus];
            let mut i = 0;
            while i < all.len() {
                for g in semantics.generals(all[i]) {
                    if !all.contains(&g) {
                        all.push(g);
                    }
                }
                i += 1;
            }
            all.into_iter().filter(|g| *g != focus).collect()
        };
        if generals.is_empty() {
            return;
        }
        // Inherited parts move inside the opened definition; redefined ones
        // are stood for by their overrides.
        let mut stands_for: BTreeMap<ElementId, ElementId> =
            generals.iter().map(|g| (*g, focus)).collect();
        for feature in semantics.features(focus) {
            let Some(element) = tree.get(feature) else {
                continue;
            };
            if semantics.is_inherited(focus, feature) {
                if element.kind == agq_language::ElementKind::Part
                    && let Some(node) = input.nodes.iter_mut().find(|n| n.id == feature)
                {
                    let from = element
                        .owner()
                        .and_then(|o| tree.effective_name(o))
                        .unwrap_or("its general")
                        .to_string();
                    node.owner = Some(focus);
                    node.origin = agq_studio_scene::NodeOrigin::Inherited;
                    node.detail = format!("{} · from {from}", node.detail)
                        .trim_start_matches(" · ")
                        .to_string();
                }
            } else {
                for general in semantics.generals(feature) {
                    if tree.get(general).is_some_and(|g| g.kind.is_usage()) {
                        stands_for.insert(general, feature);
                    }
                }
            }
        }
        for edge in &mut input.edges {
            for end in [&mut edge.source, &mut edge.target] {
                if let Some(other) = stands_for.get(&end.node) {
                    end.node = *other;
                }
            }
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

    /// The project's copies of Library blocks (the top-level `Library`
    /// package) start collapsed on the Surface: the model's own structure
    /// leads, and the copies are one card until expanded (X, or the
    /// Outline's arrow).
    fn collapse_library(&mut self) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        if let Some(library) = tree.roots().find(|r| {
            tree[*r].kind == agq_language::ElementKind::Package
                && tree.effective_name(*r) == Some(agq_library::ROOT)
        }) {
            self.collapsed.insert(library);
        }
    }

    /// Updates the Surface after a change, undo or redo, and highlights what
    /// was created or changed where it is, in the colour of who changed it.
    pub fn changed(&mut self, event: &ChangeEvent) {
        // A Library package the change created starts collapsed.
        if let Some(tree) = self.project.as_ref().map(|p| p.state().tree())
            && event.created.iter().any(|id| {
                tree.get(*id).is_some_and(|e| {
                    e.kind == agq_language::ElementKind::Package && e.owner().is_none()
                })
            })
        {
            self.collapse_library();
        }
        self.refresh();
        let now = crate::motion::clock();
        let tree = self.project.as_ref().map(|p| p.state().tree());
        // An owner changes when a member is added or removed; highlight the
        // member, not the whole owner.
        let owners: BTreeSet<ElementId> = event
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
                    self.highlights.insert(element, (now, event.actor));
                    break;
                }
                current = tree.and_then(|t| t.get(element)).and_then(|e| e.owner());
            }
        }
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
        target.fit(bounds, 48.0);
        target.zoom = target.zoom.min(1.3);
        // When more follows below the structure, show the structure at the top.
        let whole = self.scene.bounds();
        let half = target.viewport.height * 0.5 / target.zoom;
        if whole.max.y > bounds.max.y + 1.0 && bounds.height() < 2.0 * half {
            target.center.y = bounds.min.y - 48.0 / target.zoom + half;
        }
        // The first fit, while the window opens, is instant.
        if self.frame_number > 4 && !self.reduced_motion {
            self.camera_target = Some(target);
            self.camera_move = Some(crate::motion::CameraMove::tween(
                self.camera,
                crate::motion::clock(),
                self.reduced_motion,
            ));
        } else {
            self.camera = target;
            self.camera_target = None;
        }
        self.fit_pending = false;
    }

    /// Zooms around the middle of the Surface at once, ending any camera
    /// move.
    pub fn zoom_by(&mut self, factor: f32) {
        let middle = agq_studio_scene::Point::new(
            self.camera.viewport.width * 0.5,
            self.camera.viewport.height * 0.5,
        );
        self.camera.zoom_at(middle, factor);
        self.camera_target = None;
        self.camera_move = None;
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

    /// Moves the camera towards its target and ends highlights that are
    /// over. True while something still moves, so the Surface asks for the
    /// next frame.
    pub fn animate(&mut self, dt: f32) -> bool {
        let now = crate::motion::clock();
        let mut moving = false;
        // Direct manipulation clears `camera_target`; the move goes with it.
        match self.camera_target {
            Some(target) => {
                let camera = self.camera;
                let motion = self
                    .camera_move
                    .get_or_insert_with(|| crate::motion::CameraMove::follow(None, camera));
                if motion.step(
                    &mut self.camera,
                    target,
                    now,
                    dt.min(0.05),
                    self.reduced_motion,
                ) {
                    self.camera_target = None;
                    self.camera_move = None;
                } else {
                    moving = true;
                }
            }
            None => self.camera_move = None,
        }
        self.highlights
            .retain(|_, (started, _)| now - *started < crate::motion::CHANGED_SECONDS);
        moving || !self.highlights.is_empty()
    }

    /// Runs a command from the keyboard, the palette or a menu.
    pub fn execute(&mut self, id: CommandId) {
        if let Some(reason) = commands::unavailable(id, &self.context()) {
            self.status = reason.to_string();
            return;
        }
        // Whatever route reached it (a key, the palette, a menu), what is
        // the Operator's own is refused to agents (C-53); opening another
        // project would end the Assistant's own turn.
        if crate::control::OPERATORS_COMMANDS.contains(&id)
            && self.refused_to_agents(&format!("`{}`", crate::control::command_name(id)))
        {
            return;
        }
        if matches!(
            id,
            CommandId::NewProject | CommandId::OpenProject | CommandId::DevelopAgentique
        ) && self.control.acting.as_deref() == Some("Assistant")
            && self.refused_to_agents("opening another project (it ends your own turn)")
        {
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
            // Keyboard zoom is instant (§8.5 rule 5), around the middle.
            ZoomIn => self.zoom_by(1.25),
            ZoomOut => self.zoom_by(0.8),
            ZoomReset => self.zoom_by(1.0 / self.camera.zoom),
            ZoomToSelection => {
                if let Some(target) = self.selection.primary.clone() {
                    self.frame_target(&target);
                }
            }
            GoToElement => self.palette = Some(PaletteMode::Elements),
            ShortcutHelp => self.show_settings(Section::Keyboard),
            Focus => {
                if let Some(card) = self.selected_card() {
                    let label = self
                        .project
                        .as_ref()
                        .and_then(|p| p.state().tree().effective_name(card).map(str::to_string))
                        .or_else(|| {
                            self.lookup
                                .node(&self.scene, card)
                                .map(|n| n.semantic.name.clone())
                        })
                        .unwrap_or_else(|| "Focus".into());
                    self.drill_into(card, label);
                }
            }
            LeaveFocus => self.back(),
            ShowLibrary => self.show_library(),
            InsertFromLibrary => {
                self.library.fit = None;
                self.palette = Some(PaletteMode::Library { fit: false });
            }
            ConnectFromLibrary => {
                self.fit_selected_port();
                if self.library.fit.is_some() {
                    // The palette takes the keyboard, not the Library's search.
                    self.library.focus_search = false;
                    self.palette = Some(PaletteMode::Library { fit: true });
                }
            }
            OpenDefinition => {
                if let Some(element) = self.inspected_element() {
                    self.open_definition(element)
                }
            }
            FindUsages => {
                if let Some(element) = self.inspected_element() {
                    self.find_usages(element)
                }
            }
            Specialize => {
                if let Some(element) = self.inspected_element() {
                    self.start_specialize(element)
                }
            }
            CreateBlock => self.start_extract(),
            SaveToLibrary => {
                if let Some(element) = self.inspected_element() {
                    self.start_save(element)
                }
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
            Palette => self.palette = Some(PaletteMode::Commands),
            NewProject => {
                let mut dialog = crate::edit::Dialog::new_project();
                // Settings' folder for new projects, when set.
                let base = self.settings.text("projects.defaultFolder");
                if let crate::edit::Dialog::NewProject { folder, .. } = &mut dialog
                    && !base.trim().is_empty()
                {
                    *folder = std::path::Path::new(base.trim())
                        .join("NewSystem")
                        .display()
                        .to_string();
                }
                self.dialog = Some(dialog);
            }
            OpenProject => {
                self.dialog = Some(crate::edit::Dialog::OpenProject {
                    folder: String::new(),
                })
            }
            DevelopAgentique => self.develop_agentique(),
            // The appearance commands change the setting, so Settings shows
            // it and it holds at the next start.
            Theme => {
                let theme = if self.appearance.dark {
                    "light"
                } else {
                    "dark"
                };
                let _ = self
                    .settings
                    .set("appearance.theme", serde_json::json!(theme));
                self.apply_appearance();
            }
            Contrast => {
                let theme = match (self.appearance.contrast, self.appearance.dark) {
                    (false, _) => "high-contrast",
                    (true, true) => "dark",
                    (true, false) => "light",
                };
                let _ = self
                    .settings
                    .set("appearance.theme", serde_json::json!(theme));
                self.apply_appearance();
            }
            ReducedMotion => {
                let reduced = if self.reduced_motion { "off" } else { "on" };
                let _ = self
                    .settings
                    .set("appearance.reducedMotion", serde_json::json!(reduced));
                self.apply_appearance();
            }
            HidePanels => self.panels_hidden = !self.panels_hidden,
            ShowOutline => self.outline_hidden = !self.outline_hidden,
            ShowInspector => self.inspector_hidden = !self.inspector_hidden,
            ShowScenarios => {
                self.left = LeftTab::Scenarios;
                self.outline_hidden = false;
                self.panels_hidden = false;
            }
            NewScenario => self.start_new_scenario(),
            RunScenario => {
                let mode = self.runs.mode();
                if mode == agq_simulation::Mode::Live {
                    // The plan is shown first and frozen when confirmed.
                    self.runs.live_plan = Some(match self.runs.selected {
                        Some(scenario) => crate::live::plan(self, scenario),
                        None => Err("Choose a scenario to run.".into()),
                    });
                    self.dialog = Some(crate::edit::Dialog::ConfirmLive);
                } else {
                    self.start_run(mode);
                }
            }
            StopRun => {
                self.stop_run();
                self.stop_checks();
                self.stop_task();
            }
            TraceFirst => {
                self.runs.playing = false;
                let first = self.runs.visible_events().first().copied();
                self.set_cursor(first);
            }
            TraceBack => {
                self.runs.playing = false;
                self.step_cursor(false);
            }
            TracePlay => self.toggle_playback(),
            TraceForward => {
                self.runs.playing = false;
                self.step_cursor(true);
            }
            TraceLast => {
                self.runs.playing = false;
                let last = self.runs.visible_events().last().copied();
                self.set_cursor(last);
            }
            CheckImplementation => self.start_checks(),
            TrustLocal => self.dialog = Some(crate::edit::Dialog::TrustLocal),
            Settings => {
                if self.settings_open {
                    self.settings_open = false;
                } else {
                    // The last section viewed (§3.7).
                    self.show_settings(self.settings_section);
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

    /// Opens Settings at a section (deep links from errors, §3.7).
    pub fn show_settings(&mut self, section: Section) {
        self.settings_open = true;
        self.settings_section = section;
        self.palette = None;
    }

    // The session.

    pub fn save_session(&mut self) {
        if let Err(error) = self.write_session() {
            self.status = format!("The session was not saved: {error}");
        }
    }

    /// Saves the session every few seconds, as the layout and camera change.
    pub fn save_session_now_and_then(&mut self) {
        if self.last_saved.elapsed() > Duration::from_secs(8) {
            self.save_session();
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
                    panels_hidden: self.panels_hidden,
                    outline_hidden: self.outline_hidden,
                    inspector_hidden: self.inspector_hidden,
                    conversation_hidden: !self.conversation.shown,
                    widths: self.widths,
                },
            );
        }
        self.session
            .save(&self.session_path)
            .map_err(|error| error.to_string())
    }

    pub fn metrics_report(&self) -> serde_json::Value {
        let frames = self.timing.frame_summary();
        serde_json::json!({
            "fixture": self.fixture,
            "renderer": "GPUI (DirectX 11 on Windows)",
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
            "visible_nodes": self.timing.visible_nodes,
            "total_nodes": self.scene.nodes.len(),
            "total_edges": self.scene.edges.len(),
        })
    }

    // Appearance.

    /// Windows' appearance changed (or was read at start).
    pub fn follow_system(&mut self, system: System) {
        if system != self.system {
            self.system = system;
            self.apply_appearance();
        }
    }

    /// Applies Settings' appearance: dark, high contrast and reduced motion.
    /// "Follow Windows" follows its theme and its "Animation effects".
    pub fn apply_appearance(&mut self) {
        let system_dark = self.system.dark;
        let (dark, contrast) = match self.settings.text("appearance.theme").as_str() {
            "light" => (false, false),
            "dark" => (true, false),
            "high-contrast" => (system_dark, true),
            _ => (system_dark, false),
        };
        self.appearance = Appearance {
            dark: dark && !self.args.light,
            contrast,
        };
        let reduced = match self.settings.text("appearance.reducedMotion").as_str() {
            "on" => true,
            "off" => false,
            _ => self.system.reduced_motion,
        };
        // Scripted journeys run without animation so positions are final.
        self.reduced_motion = reduced || self.args.scenario_running();
        self.mark(Dirty::APPEARANCE);
    }

    /// `--ui-scale` wins over the setting (100% when not set).
    pub fn ui_scale(&self) -> f32 {
        if let Some(scale) = self.args.ui_scale.filter(|scale| scale.is_finite()) {
            return scale.clamp(0.5, 3.0);
        }
        let scale = self
            .settings
            .get("appearance.uiScale")
            .as_f64()
            .unwrap_or(1.0);
        (scale as f32).clamp(1.0, 2.0)
    }

    /// The model picker's choice: Settings' provider, with its default model
    /// and effort; the next turn uses it.
    pub fn choose_provider(&mut self, provider: &str) {
        let _ = self
            .settings
            .set("assistant.provider", serde_json::json!(provider));
        let _ = self.settings.set("assistant.model", serde_json::json!(""));
        let _ = self.settings.set("assistant.effort", serde_json::json!(""));
        self.apply_runtime_choice();
    }

    /// The Assistant is working without needing the Operator.
    pub fn assistant_working(&self) -> bool {
        self.conversation.running() && self.conversation.waiting.is_none() && self.dialog.is_none()
    }
}

/// Whether to call the Operator back: the Assistant stopped working (the turn
/// ended, or it waits for an answer) while the window was not focused.
pub fn needs_attention(was_working: bool, working: bool, focused: Option<bool>) -> bool {
    was_working && !working && focused == Some(false)
}

/// The URL shortener of Scenario A, offered on the first run (R-46).
pub const SAMPLE: &str = include_str!("../../../models/url-shortener/UrlShortener.sysml");
/// The URL shortener whose links an AI agent screens, with its scenarios:
/// Scenario I (C-50).
pub const SCREENING_SAMPLE: &str =
    include_str!("../../../models/link-screening/UrlShortener.sysml");
pub const SAMPLE_NAME: &str = "UrlShortener";

/// The samples a project can start from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sample {
    /// Scenario A's URL shortener (R-46).
    UrlShortener,
    /// The URL shortener with AI screening and its scenarios (C-50).
    Screening,
    /// The same, with its code in a repository beside the project, linked
    /// and with a harness, so its scenarios run against real code (C-50).
    ScreeningWithCode,
}

impl Sample {
    pub fn text(self) -> &'static str {
        match self {
            Sample::UrlShortener => SAMPLE,
            Sample::Screening | Sample::ScreeningWithCode => SCREENING_SAMPLE,
        }
    }
}

/// The URL shortener's code (the implementation fixture), file by file.
const SAMPLE_CODE: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../../implementation/tests/fixtures/url-shortener/Cargo.toml"),
    ),
    (
        "src/lib.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/lib.rs"),
    ),
    (
        "src/api.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/api.rs"),
    ),
    (
        "src/jev.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/jev.rs"),
    ),
    (
        "src/json.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/json.rs"),
    ),
    (
        "src/model.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/model.rs"),
    ),
    (
        "src/ports.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/ports.rs"),
    ),
    (
        "src/screening.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/screening.rs"),
    ),
    (
        "src/service.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/service.rs"),
    ),
    (
        "src/store.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/src/store.rs"),
    ),
    (
        "src/bin/agentique-harness.rs",
        include_str!(
            "../../implementation/tests/fixtures/url-shortener/src/bin/agentique-harness.rs"
        ),
    ),
    (
        "tests/behaviour.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/tests/behaviour.rs"),
    ),
    (
        "tests/jev_client.rs",
        include_str!("../../implementation/tests/fixtures/url-shortener/tests/jev_client.rs"),
    ),
    (
        "tests/decisions.json",
        include_str!("../../implementation/tests/fixtures/url-shortener/tests/decisions.json"),
    ),
];

/// Its links to the model, by name.
const SAMPLE_LINKS: &[(&str, &str, &str, Option<&str>)] =
    include!("../../implementation/tests/fixtures/url-shortener/links.in");

impl Studio {
    /// Writes the sample's code beside the project (`<name>-code`), commits
    /// it, and links it to the model.
    fn create_sample_code(&mut self, name: &str) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("No project is open.")?;
        let folder = project.folder().to_path_buf();
        let code_name = format!("{name}-code");
        let code = folder.with_file_name(&code_name);
        if code.exists() {
            return Err(format!(
                "{} already exists; the code was not written.",
                code.display()
            ));
        }
        for (path, text) in SAMPLE_CODE {
            let file = code.join(path);
            std::fs::create_dir_all(file.parent().expect("a file has a folder"))
                .map_err(|e| e.to_string())?;
            std::fs::write(&file, text).map_err(|e| e.to_string())?;
        }
        std::fs::write(code.join(".gitignore"), "/target\n").map_err(|e| e.to_string())?;
        agq_execution::git::init_and_commit(&code, "The URL shortener, implemented from its model")
            .map_err(|e| e.to_string())?;
        let tree = project.state().tree();
        let package = tree
            .documents()
            .first()
            .and_then(|d| d.members().first().copied());
        let package = package
            .and_then(|p| tree.effective_name(p))
            .unwrap_or(SAMPLE_NAME)
            .to_string();
        let mut links = agq_implementation::Links {
            repository: format!("../{code_name}"),
            language: "Rust".into(),
            harness: [
                "cargo",
                "run",
                "--quiet",
                "--offline",
                "--bin",
                "agentique-harness",
            ]
            .map(String::from)
            .to_vec(),
            protected: vec![
                "tests/behaviour.rs".into(),
                "tests/jev_client.rs".into(),
                "tests/decisions.json".into(),
            ],
            ..Default::default()
        };
        for (element, kind, path, symbol) in SAMPLE_LINKS {
            let Some(id) = tree.find(&format!("{package}::{element}")) else {
                continue;
            };
            if let Some(kind) = agq_implementation::LinkKind::ALL
                .into_iter()
                .find(|k| k.key() == *kind)
            {
                links.add(tree, id, kind, path, *symbol);
            }
        }
        self.save_implementation_links(&links)
    }
}

#[cfg(test)]
mod attention_tests {
    use super::needs_attention;

    #[test]
    fn the_operator_is_called_back_only_when_away_and_the_assistant_stops() {
        assert!(needs_attention(true, false, Some(false)));
        assert!(
            !needs_attention(true, false, Some(true)),
            "they are looking"
        );
        assert!(!needs_attention(true, true, Some(false)), "still working");
        assert!(!needs_attention(false, false, Some(false)), "nothing ran");
        assert!(!needs_attention(true, false, None), "focus not known");
    }
}
