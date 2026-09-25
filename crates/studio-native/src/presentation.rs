//! Operator presentation workflows; none of these operations changes semantics.
use crate::{
    app::{ComparisonMode, StudioApp, fixture_projection},
    navigation::World,
    saved_views::{PanelState, SavedPresentation, SavedView, SavedViews, ViewBinding},
    session::Session,
};
use agq_kernel::ElementId;
use agq_modeling_view::ViewProjection;
use agq_studio_platform::RevisionBinding;
use std::collections::{BTreeMap, BTreeSet};

impl StudioApp {
    /// Invoke once from the toolbar. Disk IO occurs when the menu opens or after
    /// an explicit save/rename/update, never in the ordinary viewport frame.
    pub fn local_views_menu(&mut self, ui: &mut eframe::egui::Ui) {
        use crate::presentation_automation::{Target, record};
        use eframe::egui::{self, RichText};
        let memory = egui::Id::new("local-view-editor");
        let mut editor = ui
            .ctx()
            .data(|data| data.get_temp::<ViewEditor>(memory))
            .unwrap_or_default();
        // This menu contains an editor and multi-step bookmark operations.
        // egui's default menu closes on every click, including inside TextEdit.
        let (menu, _) = egui::containers::menu::MenuButton::new("Local views")
            .config(egui::containers::menu::MenuConfig::new()
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
            ui.set_min_width(330.0);
            ui.set_max_width(460.0);
            let scope = self.view_binding().ok();
            if !editor.initialized || editor.last_frame + 1 != self.frame_number || editor.scope != scope {
                match self.list_local_views() {
                    Ok(views) => { editor.views = views; editor.error = None; }
                    Err(error) => { editor.views.clear(); editor.error = Some(error); }
                }
                editor.scope = scope;
                editor.initialized = true;
                if !editor.views.iter().any(|view| Some(view.id) == editor.selected) { editor.selected = None; }
            }
            editor.last_frame = self.frame_number;
            ui.label(RichText::new("Saved on this computer").strong());
            ui.label("Each view opens its saved revision. Saving a view does not change the model.");
            ui.separator();
            if editor.views.is_empty() { ui.label("No local views for this project yet."); }
            for view in editor.views.clone() {
                ui.horizontal(|ui| {
                    let selected = ui.selectable_label(editor.selected == Some(view.id), &view.name);
                    record(ui.ctx(), Target::Select(view.id), selected.rect);
                    if selected.clicked() {
                        editor.selected = Some(view.id);
                        editor.name = view.name.clone();
                    }
                    ui.label(crate::app::muted(crate::app::short_revision(view.binding.revision), self.theme).small());
                    let open = ui.small_button("Open");
                    record(ui.ctx(), Target::Open(view.id), open.rect);
                    if open.clicked() {
                        match self.open_local_view(view.id) {
                            Ok(()) => { editor.error = None; ui.close(); }
                            Err(error) => editor.error = Some(error),
                        }
                    }
                });
            }
            ui.separator();
            let name_label = ui.label("View name");
            let name = ui.add(egui::TextEdit::singleline(&mut editor.name).hint_text("e.g. Repository interfaces").desired_width(f32::INFINITY)).labelled_by(name_label.id);
            record(ui.ctx(), Target::Name, name.rect);
            let can_capture = self.bookmark_snapshot().is_ok();
            let has_name = !editor.name.trim().is_empty();
            let mut refresh = false;
            ui.horizontal_wrapped(|ui| {
                let save = ui.add_enabled(can_capture && has_name, egui::Button::new("Save current as new"));
                record(ui.ctx(), Target::Save, save.rect);
                if save.clicked() {
                    match self.save_local_view(&editor.name) {
                        Ok(id) => { editor.selected = Some(id); editor.error = None; refresh = true; self.status = "Local view saved; model unchanged".into(); }
                        Err(error) => editor.error = Some(error),
                    }
                }
                let rename = ui.add_enabled(editor.selected.is_some() && has_name, egui::Button::new("Rename selected"));
                record(ui.ctx(), Target::Rename, rename.rect);
                if rename.clicked()
                    && let Some(id) = editor.selected {
                    match self.rename_local_view(id, &editor.name) {
                        Ok(()) => { editor.error = None; refresh = true; self.status = "Local view renamed".into(); }
                        Err(error) => editor.error = Some(error),
                    }
                }
            });
            let update = ui.add_enabled(can_capture && editor.selected.is_some(), egui::Button::new("Update selected from current revision")).on_hover_text("Replace the selected bookmark's revision, camera, filters and layout with the current presentation");
            record(ui.ctx(), Target::Update, update.rect);
            if update.clicked()
                && let Some(id) = editor.selected {
                match self.update_local_view(id) {
                    Ok(()) => { editor.error = None; refresh = true; self.status = "Local view updated from the current revision".into(); }
                    Err(error) => editor.error = Some(error),
                }
            }
            if !can_capture {
                ui.label(crate::app::muted("Save from the current revision after candidate review and loading finish.", self.theme).small());
            }
            if refresh {
                match self.list_local_views() {
                    Ok(views) => editor.views = views,
                    Err(error) => editor.error = Some(error),
                }
            }
            if let Some(error) = &editor.error { ui.label(RichText::new(error).color(self.theme.amber)); }
        });
        record(ui.ctx(), Target::Menu, menu.rect);
        ui.ctx().data_mut(|data| data.insert_temp(memory, editor));
    }

    /// Breadcrumb labels are derived exclusively from available owner identities.
    /// An unavailable ancestor is acknowledged instead of invented from a name.
    pub fn breadcrumb_navigation(&mut self, ui: &mut eframe::egui::Ui) {
        let trail = breadcrumbs(self.active_projection(), self.focus);
        if trail.outside_projection {
            ui.label("…")
                .on_hover_text("Earlier ownership context is outside this loaded view");
        }
        for entry in trail.entries {
            ui.label("/");
            if ui
                .selectable_label(self.focus == Some(entry.id), &entry.name)
                .clicked()
            {
                self.navigation.update_camera(
                    [self.camera.center.x, self.camera.center.y],
                    self.camera.zoom,
                );
                self.focus = Some(entry.id);
                self.request_projection();
                self.fit_pending = true;
                self.record_location();
            }
        }
        if trail.cycle {
            ui.label("Ownership cycle")
                .on_hover_text("The projection contains a repeated ownership identity");
        }
    }

    pub fn capture_presentation(&self) -> SavedPresentation {
        let mut definition = self.definition();
        if definition.kind == self.active_projection().view.kind {
            definition.depth = self.active_projection().view.depth;
            definition.hidden_elements = self.active_projection().view.hidden_elements.clone();
        }
        SavedPresentation {
            world: self.world,
            definition,
            camera: self.camera,
            layout: self.layout.clone(),
            collapsed: self.collapsed.clone(),
            expanded: self.expanded.clone(),
            branch: self.branch,
            panels: PanelState {
                agent: self.show_agent,
                explain: self.show_explain,
                source: self.show_source,
            },
        }
    }

    fn view_binding(&self) -> Result<ViewBinding, String> {
        if !self.ready || (self.binding.is_none() && self.fixture.is_none()) {
            return Err("Open a project before saving a view".into());
        }
        Ok(ViewBinding {
            project: self.project_id(),
            revision: self.projection.revision_id,
            fixture: self.fixture.clone(),
        })
    }

    fn bookmark_snapshot(&self) -> Result<(ViewBinding, SavedPresentation), String> {
        if self.comparison != ComparisonMode::Current || self.candidate.is_some() {
            return Err(
                "Save views from a current durable revision after completing candidate review"
                    .into(),
            );
        }
        if self.scene_builder.busy
            || self.pending_revision.is_some()
            || self.bridge.mutation_pending()
            || self.scene.revision_id != self.projection.revision_id
            || self
                .binding
                .is_some_and(|binding| binding.revision != self.projection.revision_id)
        {
            return Err("Wait for the selected revision before saving this view".into());
        }
        Ok((self.view_binding()?, self.capture_presentation()))
    }

    fn local_views_path(&self) -> std::path::PathBuf {
        self.config.database.with_extension("native-views.json")
    }

    /// Read only when opening/refreshing the menu, rather than on every frame.
    pub fn list_local_views(&self) -> Result<Vec<SavedView>, String> {
        let binding = self.view_binding()?;
        let store =
            SavedViews::load(&self.local_views_path()).map_err(|error| error.to_string())?;
        Ok(store
            .views
            .into_iter()
            .filter(|view| view.binding.same_project(&binding))
            .collect())
    }

    pub fn save_local_view(&self, name: &str) -> Result<u64, String> {
        let (binding, presentation) = self.bookmark_snapshot()?;
        let path = self.local_views_path();
        let mut store = SavedViews::load(&path).map_err(|error| error.to_string())?;
        let id = store.insert(name, binding, presentation)?;
        store.save(&path).map_err(|error| error.to_string())?;
        Ok(id)
    }

    pub fn rename_local_view(&self, id: u64, name: &str) -> Result<(), String> {
        let binding = self.view_binding()?;
        let path = self.local_views_path();
        let mut store = SavedViews::load(&path).map_err(|error| error.to_string())?;
        if !store
            .views
            .iter()
            .any(|view| view.id == id && view.binding.same_project(&binding))
        {
            return Err("Saved view belongs to another project or is no longer available".into());
        }
        store.rename(id, name)?;
        store.save(&path).map_err(|error| error.to_string())
    }

    /// UI label: "Update from current revision". The bookmark can advance to a
    /// new revision only through this explicit operator action.
    pub fn update_local_view(&self, id: u64) -> Result<(), String> {
        let (binding, presentation) = self.bookmark_snapshot()?;
        let path = self.local_views_path();
        let mut store = SavedViews::load(&path).map_err(|error| error.to_string())?;
        store.update(id, binding, presentation)?;
        store.save(&path).map_err(|error| error.to_string())
    }

    pub fn open_local_view(&mut self, id: u64) -> Result<(), String> {
        let current = self.view_binding()?;
        let store =
            SavedViews::load(&self.local_views_path()).map_err(|error| error.to_string())?;
        let saved = store
            .views
            .into_iter()
            .find(|view| view.id == id)
            .ok_or("Saved view is no longer available")?;
        if !saved.binding.same_project(&current) {
            return Err("Open the saved view's project first".into());
        }
        if let Some(history) = &self.history
            && !history
                .revisions
                .iter()
                .any(|revision| revision.revision_id == saved.binding.revision)
        {
            return Err("This saved revision is unavailable in the open repository; the current view is unchanged".into());
        }
        if !self.allow_context_change() {
            return Err(self.status.clone());
        }
        let mut saved_fixture = None;
        if let Some(fixture) = &saved.binding.fixture {
            let projection = if saved.presentation.world == World::Requirements {
                agq_studio_scene::fixtures::requirements()
            } else {
                fixture_projection(fixture)
            };
            if projection.revision_id == saved.binding.revision {
                saved_fixture = Some(projection);
            } else {
                let (before, after) = agq_studio_scene::fixtures::revision_diff();
                saved_fixture = [before, after]
                    .into_iter()
                    .find(|projection| projection.revision_id == saved.binding.revision);
            }
            if saved_fixture.is_none() {
                return Err("Saved fixture revision is unavailable".into());
            }
        }
        let previous = self.display_snapshot();
        let presentation = saved.presentation;
        self.world = presentation.world;
        self.focus = presentation.definition.focus;
        self.families = presentation
            .definition
            .relationship_families
            .iter()
            .copied()
            .collect();
        self.include_standard = presentation.definition.include_standard_library;
        self.collapsed = presentation.collapsed.clone();
        self.expanded = presentation.expanded.clone();
        self.dependencies = None;
        self.compare_before = None;
        self.candidate = None;
        self.comparison = ComparisonMode::Current;
        self.invalidate_inspection();
        if let Some(project) = saved.binding.project {
            let binding = RevisionBinding {
                project,
                revision: saved.binding.revision,
            };
            self.pending_revision = Some(binding);
            let definition = presentation.definition.clone();
            self.restore = Some(Session {
                version: 1,
                project: Some(project),
                revision: saved.binding.revision,
                fixture: None,
                world: presentation.world,
                focus: presentation.definition.focus,
                camera: presentation.camera,
                layout: presentation.layout.clone(),
                dark: self.theme.dark,
                high_contrast: self.theme.contrast,
                reduced_motion: self.reduced_motion,
                presentation: Some(presentation),
            });
            self.scene_request = self.enqueue(Box::new(move |platform| {
                platform
                    .project(binding, &definition)
                    .map(crate::bridge::Output::Projection)
            }));
        } else if let Some(projection) = saved_fixture {
            self.projection = projection;
            self.apply_saved_presentation(presentation);
            if !self.rebuild_immediate() {
                let error = self.status.clone();
                self.restore_display(previous);
                return Err(error);
            }
            self.record_location();
        }
        self.status = format!("Opened local view ‘{}’ at its saved revision", saved.name);
        Ok(())
    }

    /// Call after the exact saved revision/lens has arrived, before rebuilding.
    /// Projection omissions are recovered as presentation omissions only.
    pub fn apply_saved_presentation(&mut self, mut presentation: SavedPresentation) {
        let recovery = presentation.reconcile(&self.projection);
        self.projection.view = presentation.definition.clone();
        self.world = presentation.world;
        self.focus = presentation.definition.focus;
        self.families = presentation
            .definition
            .relationship_families
            .into_iter()
            .collect();
        self.include_standard = presentation.definition.include_standard_library;
        self.collapsed = presentation.collapsed;
        self.expanded = presentation.expanded;
        self.layout = presentation.layout;
        self.layout_world = self.world;
        self.layout_focus = self.focus;
        // A saved window size cannot override the current monitor/viewport.
        let viewport = self.camera.viewport;
        self.camera = presentation.camera;
        self.camera.viewport = viewport;
        self.camera_target = None;
        self.fit_pending = recovery.focus_reset;
        self.show_agent = presentation.panels.agent;
        self.dependencies = self.show_agent.then(|| self.expanded.clone()).flatten();
        // Evidence/source require a revision-bound selection, which is not saved.
        self.show_explain = false;
        self.show_source = false;
        if let Some(branch) = presentation.branch
            && self
                .history
                .as_ref()
                .is_some_and(|history| history.branches.iter().any(|item| item.id == branch))
        {
            self.branch = Some(branch);
        }
        if recovery.focus_reset || recovery.omitted_references > 0 {
            self.status = format!(
                "Restored view; {} saved references were outside the loaded projection{}",
                recovery.omitted_references,
                if recovery.focus_reset {
                    "; focus reset"
                } else {
                    ""
                }
            );
        }
    }
}

#[derive(Clone, Default)]
struct ViewEditor {
    initialized: bool,
    last_frame: u64,
    scope: Option<ViewBinding>,
    selected: Option<u64>,
    name: String,
    views: Vec<SavedView>,
    error: Option<String>,
}

/// Read-only observation of the ordinary editor, used by native input assertions.
pub(crate) fn local_view_name(ctx: &eframe::egui::Context) -> Option<String> {
    ctx.data(|data| data.get_temp::<ViewEditor>(eframe::egui::Id::new("local-view-editor")))
        .map(|editor| editor.name)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breadcrumb {
    pub id: ElementId,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BreadcrumbTrail {
    pub entries: Vec<Breadcrumb>,
    /// The owning ancestor was not included in this bounded projection.
    pub outside_projection: bool,
    pub cycle: bool,
}

/// Root-to-focus ownership from canonical projection identities. Qualified-name
/// text is never split to invent ancestors, and cycles terminate safely.
pub fn breadcrumbs(projection: &ViewProjection, focus: Option<ElementId>) -> BreadcrumbTrail {
    let nodes: BTreeMap<_, _> = projection
        .nodes
        .iter()
        .map(|node| (node.id, node))
        .collect();
    let mut trail = BreadcrumbTrail::default();
    let mut seen = BTreeSet::new();
    let mut cursor = focus;
    while let Some(id) = cursor {
        if !seen.insert(id) {
            trail.cycle = true;
            break;
        }
        let Some(node) = nodes.get(&id) else {
            trail.outside_projection = true;
            break;
        };
        trail.entries.push(Breadcrumb {
            id,
            name: node.name.clone(),
        });
        cursor = node.owner;
    }
    trail.entries.reverse();
    trail
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breadcrumbs_use_ownership_and_stop_at_missing_ancestors_or_cycles() {
        let mut projection = agq_studio_scene::fixtures::architecture();
        let selected = projection
            .nodes
            .iter()
            .find(|node| node.owner.is_some())
            .unwrap()
            .id;
        let trail = breadcrumbs(&projection, Some(selected));
        assert_eq!(trail.entries.last().unwrap().id, selected);
        assert!(trail.entries.len() >= 2);
        assert!(!trail.cycle);
        let selected_node = projection
            .nodes
            .iter_mut()
            .find(|node| node.id == selected)
            .unwrap();
        selected_node.owner = Some(ElementId::from_u128(u128::MAX));
        let bounded = breadcrumbs(&projection, Some(selected));
        assert_eq!(bounded.entries.len(), 1);
        assert!(bounded.outside_projection);
        projection
            .nodes
            .iter_mut()
            .find(|node| node.id == selected)
            .unwrap()
            .owner = Some(selected);
        let cyclic = breadcrumbs(&projection, Some(selected));
        assert!(cyclic.cycle);
        assert_eq!(cyclic.entries.len(), 1);
    }
}
