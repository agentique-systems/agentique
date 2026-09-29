//! The Library in the Studio (C-49, ROADMAP §4.13, Scenario H): the Library
//! panel's state and the Operator's Library actions. Each action is planned
//! by `agq-library` and applied through the Studio's one change path
//! (`apply_change`), exactly like the Assistant's `use_library_block`: locks
//! ask, undo reverts it in one step, history names it in words.
//!
//! Opening a definition ("drill in") moves the Surface to the definition's
//! inside and keeps where it came from, so Back returns there; the
//! breadcrumb shows the way in.
use crate::{
    edit::{Dialog, Outcome},
    navigation::SurfaceView,
    selection::Selection,
    studio::{Dirty, LeftTab, PaletteMode, Studio},
};
use agq_language::{ElementId, ElementKind, Parent, Tree};
use agq_library::{
    BlockRef, Conflict, ConnectTo, Extraction, Fit, Index, Library, Override, PlanError,
    Resolution, Scope, Use,
};
use agq_studio_scene::{Camera2D, SceneTarget};
use agq_system_state::Actor;

/// The Library panel's state and the Library's sources.
pub struct LibraryState {
    /// The built-in blocks and My Library.
    pub source: Library,
    /// Every block, rebuilt when the model or My Library changes.
    pub index: Index,
    /// The block the panel shows in its preview.
    pub selected: Option<BlockRef>,
    /// Only this scope (all when `None`).
    pub scope: Option<Scope>,
    /// Only these kinds (all when empty).
    pub kinds: Vec<ElementKind>,
    /// "What can connect here?": only blocks that fit this port.
    pub fit: Option<FitFilter>,
    /// Blocks used lately, most recent first.
    pub recent: Vec<BlockRef>,
    /// The panel's search box takes the keyboard when it is next drawn.
    pub focus_search: bool,
}

/// Blocks that fit a port, and where a block chosen from them goes.
#[derive(Clone, Debug)]
pub struct FitFilter {
    pub card: ElementId,
    pub port: ElementId,
    /// The port as the Operator reads it: `front.backend`.
    pub label: String,
    pub fits: Vec<Fit>,
}

/// The Surface before a definition was opened, to return to.
#[derive(Clone)]
pub struct Drill {
    pub focus: Option<ElementId>,
    pub view: SurfaceView,
    pub camera: Camera2D,
    pub selection: Selection,
    /// What was opened, as the breadcrumb names it: `api : RateLimitedApi`.
    pub label: String,
}

/// How many recent blocks the panel remembers.
const RECENT: usize = 8;

impl LibraryState {
    /// The Library with My Library at `path` (beside the session file, so
    /// tests and journeys keep away from the Operator's own).
    pub fn new(path: std::path::PathBuf) -> Self {
        LibraryState {
            source: Library::with_mine(path),
            index: Index::default(),
            selected: None,
            scope: None,
            kinds: Vec::new(),
            fit: None,
            recent: Vec::new(),
            focus_search: false,
        }
    }

    fn used(&mut self, block: &BlockRef) {
        self.recent.retain(|b| b != block);
        self.recent.insert(0, block.clone());
        self.recent.truncate(RECENT);
    }
}

/// Whether an element is a usage typed by a definition with parts of its
/// own or inherited (double-click opens it).
pub fn is_composite_usage(tree: &Tree, element: ElementId) -> bool {
    let Some(e) = tree.get(element) else {
        return false;
    };
    if !e.kind.is_usage() {
        return false;
    }
    let Some(definition) = definition_of(tree, element) else {
        return false;
    };
    let semantics = agq_language::Semantics::new(tree);
    semantics
        .features(definition)
        .iter()
        .any(|f| semantics.element(*f).map(|e| e.kind) == Some(ElementKind::Part))
}

/// The type of a usage, or the definition itself: what "Open definition"
/// opens.
pub fn definition_of(tree: &Tree, element: ElementId) -> Option<ElementId> {
    let e = tree.get(element)?;
    if e.kind.is_definition() {
        return Some(element);
    }
    e.typed_by
        .iter()
        .find_map(|r| r.target())
        .filter(|t| tree.get(*t).is_some_and(|d| d.kind.is_definition()))
}

impl Studio {
    /// The index for the current model and My Library, rebuilt only when
    /// either changed since it was last built.
    pub fn library_index(&mut self) -> &Index {
        let project = self
            .project
            .as_ref()
            .map(|p| (p.state().tree(), p.state().revision()));
        let revision = project.map(|(_, r)| r);
        if !self
            .library
            .index
            .is_current(&self.library.source, revision)
        {
            let library = &mut self.library;
            library.index.refresh(&library.source, project);
        }
        &self.library.index
    }

    /// Shows the Library beside the Surface, its search ready for typing.
    pub fn show_library(&mut self) {
        self.left = LeftTab::Library;
        self.outline_hidden = false;
        self.panels_hidden = false;
        self.library.focus_search = true;
        self.mark(Dirty::LAYOUT | Dirty::OVERLAY);
    }

    /// The model's own top-level package (not the copied `Library`), where
    /// a block dropped on the empty Surface goes.
    pub fn model_package(&self) -> Parent {
        self.project
            .as_ref()
            .map(|p| p.state().tree())
            .and_then(|tree| {
                tree.roots().find(|r| {
                    tree[*r].kind == ElementKind::Package
                        && tree.effective_name(*r) != Some(agq_library::ROOT)
                })
            })
            .map_or(Parent::Document(0), Parent::Element)
    }

    /// Where a new block goes when nothing more specific is given: inside
    /// the selected card, else the model's first package.
    fn block_parent(&self) -> Parent {
        self.create_parent()
    }

    /// Uses a block: plans it (copying what it needs into the project's
    /// `Library` package) and applies it as the Operator's change. The new
    /// usage is selected with its name ready to edit. Returns it when the
    /// change applied.
    pub fn insert_block(
        &mut self,
        block: BlockRef,
        parent: Option<Parent>,
        connect: Option<ConnectTo>,
    ) -> Option<ElementId> {
        let parent = parent.unwrap_or_else(|| self.block_parent());
        let mut request = Use::new(block, parent);
        request.connect = connect;
        self.use_block(request)
    }

    /// Plans and applies a use of a block (see [`Studio::insert_block`]).
    pub fn use_block(&mut self, request: Use) -> Option<ElementId> {
        let project = self.project.as_ref()?;
        if !self.editable() {
            self.status = "Open or create a project to edit".into();
            return None;
        }
        let plan = match self
            .library
            .source
            .plan_use(project.state(), &request, Actor::Operator)
        {
            Ok(plan) => plan,
            Err(PlanError::Conflicts(conflicts)) => {
                self.dialog = Some(Dialog::LibraryConflict { request, conflicts });
                self.mark(Dirty::OVERLAY);
                return None;
            }
            Err(error) => {
                self.status = format!("Not added: {error}");
                self.mark(Dirty::STATUS);
                return None;
            }
        };
        let usage = plan.created;
        match self.apply_change(plan.change) {
            Outcome::Applied(_) => {
                self.library.used(&request.block);
                let usage = usage?;
                self.reveal_new(usage);
                if let Some(tree) = self.project.as_ref().map(|p| p.state().tree())
                    && self
                        .scene
                        .target_bounds(&SceneTarget::Node(usage))
                        .is_some()
                {
                    let name = tree.effective_name(usage).unwrap_or_default().to_string();
                    self.dialog = Some(Dialog::Rename {
                        element: usage,
                        name,
                    });
                }
                self.mark(Dirty::MODEL | Dirty::SELECTION | Dirty::OVERLAY);
                Some(usage)
            }
            _ => {
                self.mark(Dirty::ALL);
                None
            }
        }
    }

    /// Selects a new element's card (or shows it in the Inspector) and
    /// brings it into view.
    fn reveal_new(&mut self, element: ElementId) {
        let target = SceneTarget::Node(element);
        if self.scene.target_bounds(&target).is_some() {
            self.select(target.clone(), false);
            self.frame_target(&target);
        } else {
            self.reveal(element);
        }
        self.panel = crate::studio::Panel::Inspector;
    }

    /// Settles a conflict the Operator was asked about, and uses the block.
    pub fn resolve_conflict(&mut self, mut request: Use, resolution: Resolution) {
        request.resolution = resolution;
        self.use_block(request);
    }

    /// "What can connect here?": the Library shows only blocks with a port
    /// that fits the selected port, by the model's own rule; a block chosen
    /// then goes beside the port's part and is connected to it.
    pub fn fit_selected_port(&mut self) {
        let Some(SceneTarget::Port(card, port)) = self.selection.primary.clone() else {
            self.status = "Select a port first".into();
            return;
        };
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree().clone()) else {
            return;
        };
        let label = format!(
            "{}.{}",
            tree.effective_name(card).unwrap_or("?"),
            tree.effective_name(port).unwrap_or("?")
        );
        self.library_index();
        let fits = self
            .library
            .source
            .compatible(&self.library.index, &tree, port, false);
        self.status = match fits.len() {
            0 => format!("No building block fits {label}"),
            1 => format!("1 building block fits {label}"),
            n => format!("{n} building blocks fit {label}"),
        };
        self.library.fit = Some(FitFilter {
            card,
            port,
            label,
            fits,
        });
        self.show_library();
    }

    /// Where a block chosen under "What can connect here?" goes: beside the
    /// port's part, in the part's owner.
    pub fn fit_parent(&self, fit: &FitFilter) -> Option<Parent> {
        let tree = self.project.as_ref()?.state().tree();
        tree.get(fit.card)?.owner().map(Parent::Element)
    }

    /// Opens the definition of a usage (or a definition itself): the
    /// Surface shows its inside, and Back returns here.
    pub fn open_definition(&mut self, element: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let Some(definition) = definition_of(tree, element) else {
            self.status = "This element has no definition to open".into();
            return;
        };
        let label = match tree.get(element) {
            Some(e) if e.kind.is_usage() => format!(
                "{} : {}",
                tree.effective_name(element).unwrap_or(""),
                tree.effective_name(definition).unwrap_or("")
            ),
            _ => tree.effective_name(definition).unwrap_or("").to_string(),
        };
        self.drill_into(definition, label);
    }

    /// Shows only `element` and what it owns, remembering the view to go
    /// back to.
    pub fn drill_into(&mut self, element: ElementId, label: String) {
        self.drill.push(Drill {
            focus: self.focus,
            view: self.view,
            camera: self.camera,
            selection: self.selection.clone(),
            label,
        });
        self.view = SurfaceView::Architecture;
        self.focus = Some(element);
        self.selection.clear();
        self.rebuild();
        let target = SceneTarget::Container(element);
        if self.scene.target_bounds(&target).is_some() {
            self.select(target, false);
        } else if self
            .scene
            .target_bounds(&SceneTarget::Node(element))
            .is_some()
        {
            self.select(SceneTarget::Node(element), false);
        }
        self.frame_all();
        self.mark(Dirty::MODEL | Dirty::SELECTION | Dirty::CAMERA);
    }

    /// Back to what the Surface showed before the last definition was
    /// opened (or leaves a focus with nothing before it).
    pub fn back(&mut self) {
        match self.drill.pop() {
            Some(before) => {
                self.focus = before.focus;
                self.view = before.view;
                self.rebuild();
                self.camera = before.camera;
                self.camera_target = None;
                self.camera_move = None;
                self.selection = before.selection;
                self.selection.reconcile(&self.scene);
            }
            None => {
                self.focus = None;
                self.rebuild();
                self.frame_all();
            }
        }
        self.mark(Dirty::MODEL | Dirty::SELECTION | Dirty::CAMERA);
    }

    /// Back to the breadcrumb at `depth` (0: the whole model).
    pub fn back_to(&mut self, depth: usize) {
        while self.drill.len() > depth {
            self.back();
        }
    }

    /// The breadcrumb: the whole model, then each definition opened.
    pub fn breadcrumbs(&self) -> Vec<String> {
        let root = self
            .project
            .as_ref()
            .and_then(|p| p.folder().file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Model".into());
        std::iter::once(root)
            .chain(self.drill.iter().map(|d| d.label.clone()))
            .collect()
    }

    /// Lists the usages of a definition (or of a usage's type) in the
    /// palette.
    pub fn find_usages(&mut self, element: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        match definition_of(tree, element) {
            Some(definition) => self.palette = Some(PaletteMode::Usages(definition)),
            None => self.status = "This element has no definition".into(),
        }
    }

    /// Usages typed by `definition` and definitions specialising it, in
    /// model order.
    pub fn usages_of(&self, definition: ElementId) -> Vec<ElementId> {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return Vec::new();
        };
        let mut found: Vec<ElementId> = tree
            .references_to(definition)
            .into_iter()
            .filter(|(holder, role)| {
                matches!(
                    role,
                    agq_language::Role::TypedBy | agq_language::Role::Specializes
                ) && tree.get(*holder).is_some()
            })
            .map(|(holder, _)| holder)
            .collect();
        found.dedup();
        found
    }

    /// Asks for the name of a specialisation of the selected definition (or
    /// of the selected usage's type, which the usage then uses).
    pub fn start_specialize(&mut self, element: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let Some(definition) = definition_of(tree, element) else {
            self.status = "Only a definition, or a usage of one, can be specialised".into();
            return;
        };
        let usage = (element != definition).then_some(element);
        let base = tree.effective_name(definition).unwrap_or("Definition");
        self.dialog = Some(Dialog::Specialize {
            definition,
            usage,
            name: format!("Special{base}"),
        });
        self.mark(Dirty::OVERLAY);
    }

    /// Creates the specialisation, types the usage by it, and opens it so
    /// its inherited features can be overridden.
    pub fn specialize(&mut self, definition: ElementId, name: &str, usage: Option<ElementId>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        match self.library.source.plan_specialize(
            project.state(),
            definition,
            name,
            usage,
            Actor::Operator,
        ) {
            Ok(plan) => {
                let created = plan.created;
                if let Outcome::Applied(_) = self.apply_change(plan.change)
                    && let Some(created) = created
                {
                    let label = name.trim().to_string();
                    self.drill_into(created, label);
                }
            }
            Err(error) => self.status = format!("Not specialised: {error}"),
        }
        self.mark(Dirty::ALL);
    }

    /// Overrides an inherited feature of `owner` only there (a redefinition).
    pub fn override_feature(&mut self, owner: ElementId, path: &[ElementId], what: Override) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        match self
            .library
            .source
            .plan_override(project.state(), owner, path, what, Actor::Operator)
        {
            Ok(plan) => {
                self.apply_change(plan.change);
            }
            Err(error) => self.status = format!("Not overridden: {error}"),
        }
        self.mark(Dirty::ALL);
    }

    /// Proposes a building block made of the selected parts: shows the
    /// boundary before anything changes.
    pub fn start_extract(&mut self) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let selected = self.selection.elements(&self.scene);
        match Extraction::analyse(project.state(), &selected) {
            Ok(extraction) => {
                self.dialog = Some(Dialog::ExtractBlock {
                    extraction: Box::new(extraction),
                    definition: String::new(),
                    usage: String::new(),
                });
                self.mark(Dirty::OVERLAY);
            }
            Err(reason) => self.status = format!("Cannot create a building block: {reason}"),
        }
    }

    /// Creates the block and replaces the parts with a usage of it, in one
    /// change.
    pub fn extract(&mut self, extraction: &Extraction, definition: &str, usage: &str) -> bool {
        let Some(project) = self.project.as_ref() else {
            return false;
        };
        let usage = if usage.trim().is_empty() {
            agq_library::default_usage_name(
                project.state().tree(),
                Parent::Element(extraction.owner),
                definition.trim(),
            )
        } else {
            usage.trim().to_string()
        };
        match extraction.plan(project.state(), definition, &usage, Actor::Operator) {
            Ok(plan) => {
                let created = plan.created;
                if let Outcome::Applied(_) = self.apply_change(plan.change)
                    && let Some(created) = created
                {
                    self.reveal_new(created);
                }
                self.mark(Dirty::ALL);
                true
            }
            Err(error) => {
                self.status = format!("Not created: {error}");
                self.mark(Dirty::STATUS);
                false
            }
        }
    }

    /// Asks where in My Library a definition goes.
    pub fn start_save(&mut self, element: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let Some(definition) = definition_of(tree, element) else {
            self.status = "Only a definition can be saved to My Library".into();
            return;
        };
        let category = tree
            .get(definition)
            .and_then(|e| e.owner())
            .and_then(|o| tree.effective_name(o))
            .filter(|name| *name != agq_library::ROOT)
            .unwrap_or("Blocks")
            .to_string();
        self.dialog = Some(Dialog::SaveToLibrary {
            definition,
            category,
            conflicts: Vec::new(),
        });
        self.mark(Dirty::OVERLAY);
    }

    /// Saves a definition to My Library. `replace` replaces My Library's
    /// different blocks of the same names; without it they are reported and
    /// nothing is saved.
    pub fn save_to_library(
        &mut self,
        definition: ElementId,
        category: &str,
        replace: bool,
    ) -> Result<(), Vec<Conflict>> {
        let Some(project) = self.project.as_ref() else {
            return Ok(());
        };
        match self
            .library
            .source
            .plan_save(project.state().tree(), definition, category, replace)
        {
            Ok(plan) => {
                let block = plan.block.clone();
                let added = plan.added.len();
                match self.library.source.save(plan) {
                    Ok(()) => {
                        self.status = format!(
                            "Saved to My Library as {} ({added} definition(s) added)",
                            block.qualified_name
                        );
                        self.library.selected = Some(block);
                        self.library.scope = Some(Scope::Mine);
                    }
                    Err(error) => self.status = format!("My Library could not be saved: {error}"),
                }
                self.mark(Dirty::ALL);
                Ok(())
            }
            Err(PlanError::Conflicts(conflicts)) => Err(conflicts),
            Err(error) => {
                self.status = format!("Not saved: {error}");
                self.mark(Dirty::STATUS);
                Ok(())
            }
        }
    }

    /// Shows a block in the Library panel (a link from the Conversation):
    /// the project's copy when it has one.
    pub fn show_block(&mut self, block: BlockRef) {
        self.library_index();
        let index = &self.library.index;
        let shown = index.find(&block).map(|found| {
            index
                .project_copy(found)
                .and_then(|copy| index.get(copy))
                .map_or(block.clone(), |copy| copy.reference.clone())
        });
        match shown {
            Some(reference) => {
                self.library.scope = None;
                self.library.kinds.clear();
                self.library.fit = None;
                self.library.selected = Some(reference);
                self.show_library();
                self.library.focus_search = false;
            }
            None => self.status = format!("The Library has no {block}"),
        }
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
    }

    /// Removes a block from My Library; projects keep their copies.
    pub fn remove_from_library(&mut self, block: &BlockRef) {
        match self.library.source.remove_mine(&block.qualified_name) {
            Ok(()) => {
                self.status = format!("Removed {} from My Library", block.qualified_name);
                if self.library.selected.as_ref() == Some(block) {
                    self.library.selected = None;
                }
            }
            Err(reason) => self.status = format!("Not removed: {reason}"),
        }
        self.mark(Dirty::ALL);
    }
}

#[cfg(test)]
mod tests;
