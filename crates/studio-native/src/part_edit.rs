//! Reviewable part-edit intent, pinned to the object and revision at dialog open.
use crate::{
    app::{ComparisonMode, PendingPreparation, StudioApp, muted},
    commands::{self, CommandId},
    navigation::World,
};
use agq_kernel::ElementId;
use agq_modeling_view::ViewDefinition;
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_platform::RevisionBinding;
use eframe::egui::{self, Align2, Vec2};

pub struct EditTarget {
    command: CommandId,
    element: ElementId,
    revision: ProjectRevisionId,
    binding: Option<RevisionBinding>,
    fixture: Option<String>,
    name: String,
    /// The ordinary owner query must be installed before preparing a nested edit.
    owner_view: Option<ViewDefinition>,
}

impl StudioApp {
    pub fn open_part_edit(&mut self, command: CommandId) {
        let Some(element) = self.selected_element() else {
            return;
        };
        let Some(node) = self.scene.node(element) else {
            return;
        };
        self.new_part_name = if command == CommandId::RenamePart {
            node.semantic.name.clone()
        } else {
            "newPart".into()
        };
        self.edit_target = Some(EditTarget {
            command,
            element,
            revision: self.projection.revision_id,
            binding: self.binding,
            fixture: self.fixture.clone(),
            name: node.semantic.name.clone(),
            owner_view: None,
        });
        if command == CommandId::CreatePart {
            let view = self.open_part_owner_view(element);
            self.edit_target
                .as_mut()
                .expect("pinned edit target")
                .owner_view = Some(view);
        }
        self.create_dialog = true;
        self.create_dialog_focus = true;
    }

    fn open_part_owner_view(&mut self, owner: ElementId) -> ViewDefinition {
        // Creating inside a part explicitly enters that engineering context.
        // Record the old location before changing the lens so Back remains useful.
        self.cancel_revision_navigation();
        self.navigation.update_camera(
            [self.camera.center.x, self.camera.center.y],
            self.camera.zoom,
        );
        self.record_location();
        self.restore_world_filters(World::System);
        self.world = World::System;
        self.focus = Some(owner);
        self.search.clear();
        self.expanded = None;
        self.collapsed.remove(&owner);
        self.dependencies = None;
        self.compare_before = None;
        self.comparison = ComparisonMode::Current;
        self.show_agent = false;
        self.agent_activity = None;
        self.agent_return = None;
        self.restore = None;
        let view = ViewDefinition {
            focus: Some(owner),
            depth: 1,
            relationship_families: self.families.iter().copied().collect(),
            include_standard_library: self.include_standard,
            ..ViewDefinition::architecture()
        };
        self.request_projection_definition(view.clone());
        self.fit_pending = true;
        self.record_location();
        view
    }

    fn part_edit_unavailable(&self, command: CommandId) -> Option<&'static str> {
        let Some(target) = &self.edit_target else {
            return Some("Select a part and reopen the edit command");
        };
        if target.command != command
            || target.revision != self.projection.revision_id
            || target.binding != self.binding
            || target.fixture != self.fixture
        {
            return Some("The revision changed; reopen the edit command in the current view");
        }
        if let Some(view) = &target.owner_view {
            // A later World/focus/filter intent wins. The dialog never redirects
            // the operator or prepares an invisible child in a different lens.
            if self.world != World::System
                || self.focus != Some(target.element)
                || self.definition() != *view
                || self.expanded.is_some()
                || self.collapsed.contains(&target.element)
                || self.comparison != ComparisonMode::Current
            {
                return Some(
                    "The view changed. Close this dialog and reopen Create inside the intended part.",
                );
            }
            if self.pending_revision.is_some()
                || self.requested_definition.is_some()
                || self.deferred_definition.is_some()
                || self.scene_builder.busy
            {
                return Some(
                    "Opening the selected part's System view. Prepare will be available when it is ready.",
                );
            }
            if self.projection.view != *view
                || self.scene.revision_id != target.revision
                || self.layout_world != World::System
                || self.layout_focus != Some(target.element)
                || self
                    .scene
                    .node(target.element)
                    .is_none_or(|node| node.semantic.revision_id != target.revision)
            {
                return Some(
                    "The selected part's view is not ready. Close and reopen Create to retry.",
                );
            }
        }
        let mut context = self.context();
        context.selected = true;
        context.can_create = self.scene.node(target.element).is_some_and(|node| {
            commands::can_create_part(&node.semantic, self.fixture.is_some(), target.revision)
        });
        commands::unavailable(command, &context)
    }

    pub(crate) fn part_edit_ready(&self) -> bool {
        self.edit_target
            .as_ref()
            .is_some_and(|target| self.part_edit_unavailable(target.command).is_none())
    }

    pub fn part_edit_target(&mut self, command: CommandId) -> Option<ElementId> {
        if let Some(reason) = self.part_edit_unavailable(command) {
            self.status = reason.into();
            return None;
        }
        self.edit_target.as_ref().map(|target| target.element)
    }

    pub fn part_edit_dialog(&mut self, ctx: &egui::Context) {
        if !self.create_dialog {
            return;
        }
        let Some(target) = &self.edit_target else {
            self.create_dialog = false;
            return;
        };
        let rename = target.command == CommandId::RenamePart;
        let target_name = target.name.clone();
        let unavailable = self.part_edit_unavailable(target.command);
        let ready = self.part_edit_ready();
        let mut open = true;
        egui::Window::new(if rename { "Rename part" } else { "Create nested part" })
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(format!("{} {target_name}", if rename { "Selected:" } else { "Inside" }));
                ui.add_space(10.0);
                let label = ui.label(if rename { "New part name" } else { "Part name" });
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.new_part_name)
                        .desired_width(f32::INFINITY),
                ).labelled_by(label.id);
                if self.create_dialog_focus {
                    response.request_focus();
                    self.create_dialog_focus = false;
                }
                let submit = response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                crate::automation::record(ctx, crate::automation::Target::CandidateName, response.rect);
                ui.add_space(16.0);
                ui.label(muted(if let Some(reason) = unavailable {
                    reason
                } else if self.fixture.is_some() {
                    "This creates a visual preview of typed intent. Semantic reconstruction requires the accepted runtime."
                } else {
                    "A Working candidate will be reconstructed. Review and validate it before committing."
                }, self.theme));
                ui.add_space(16.0);
                let prepare = ui.add_enabled(ready, egui::Button::new("Prepare candidate"));
                crate::automation::record(ctx, crate::automation::Target::CandidatePrepare, prepare.rect);
                if ready && (prepare.clicked() || submit) {
                    if rename { self.prepare_rename(); } else { self.prepare_part(); }
                }
            });
        self.create_dialog &= open;
    }

    fn prepare_rename(&mut self) {
        let Some(element) = self.part_edit_target(CommandId::RenamePart) else {
            return;
        };
        let name = self.new_part_name.trim().to_owned();
        if name.is_empty() {
            self.status = "Enter a part name".into();
            return;
        }
        let (Some(binding), Some(branch)) = (self.binding, self.branch) else {
            return;
        };
        let original = &self.edit_target.as_ref().expect("checked edit target").name;
        let intent = format!("Rename {original} to {name}");
        let context = agq_modeling_agent::AgentContext {
            project: binding.project,
            branch,
            revision: binding.revision,
            selection: vec![element],
        };
        let request = self.enqueue_mutation(crate::bridge::propose(
            context,
            agq_modeling_agent::ModelCommand::RenameElement { element, name },
            self.definition(),
        ));
        if request != 0 {
            self.preparation = Some(PendingPreparation {
                request,
                started: std::time::Instant::now(),
                cancelled: false,
                intent,
            });
            self.create_dialog = false;
            self.status = "Constructing a Working name-change candidate in the background".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_studio_scene::SceneTarget;
    use clap::Parser;

    fn application() -> StudioApp {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let args = crate::Args::parse_from([
            "studio",
            "--fixture",
            "architecture",
            "--no-restore",
            "--root",
            root.to_str().unwrap(),
        ]);
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        StudioApp::new(&context, args).unwrap()
    }

    fn named(app: &StudioApp, name: &str) -> ElementId {
        app.scene
            .nodes
            .iter()
            .find(|node| node.semantic.name == name)
            .unwrap()
            .id()
    }

    #[test]
    fn dialog_intent_retains_original_owner_after_another_canvas_selection() {
        let mut app = application();
        let original = named(&app, "ModelingPlatform");
        let later = named(&app, "ModelRepository");
        app.select(SceneTarget::Node(original), false);
        app.open_part_edit(CommandId::CreatePart);
        assert!(
            app.scene.node(later).is_some(),
            "later selection is visible in the owner view"
        );
        assert!(app.part_edit_ready());
        app.select(SceneTarget::Node(later), false);
        app.new_part_name = "reviewedChild".into();
        app.prepare_part();
        let candidate = app.candidate.unwrap();
        let added = candidate
            .after
            .nodes
            .iter()
            .find(|node| node.name == "reviewedChild")
            .unwrap();
        assert_eq!(added.owner, Some(original));
        assert!(candidate.intent.contains("ModelingPlatform"));
        assert_eq!(candidate.before.view, candidate.after.view);
        assert_eq!(candidate.after.view.focus, Some(original));
        assert!(candidate.after.view.depth >= 1);
        assert_eq!(
            candidate.phase, None,
            "fixture is never semantic acceptance"
        );
    }

    #[test]
    fn create_enters_the_owner_view_and_keeps_the_previous_camera_for_back() {
        let mut app = application();
        let owner = named(&app, "ModelingPlatform");
        app.world = World::Graph;
        app.focus = None;
        app.projection.view = ViewDefinition::semantic_graph();
        assert!(app.rebuild_immediate());
        app.camera.center = agq_studio_scene::Point::new(310.0, -240.0);
        app.camera.zoom = 0.83;
        let previous_camera = app.camera;
        app.record_location();
        app.collapsed.insert(owner);
        app.expanded = Some(std::collections::BTreeSet::from([owner]));
        app.select(SceneTarget::Node(owner), false);
        let full_nodes = app.projection.nodes.clone();
        app.open_part_edit(CommandId::CreatePart);

        assert_eq!(app.world, World::System);
        assert_eq!(app.focus, Some(owner));
        assert!(!app.collapsed.contains(&owner));
        assert!(app.expanded.is_none());
        assert_eq!(
            app.projection.nodes, full_nodes,
            "opening Create never inserts semantic records"
        );
        assert_eq!(
            app.projection.view.kind,
            agq_modeling_view::ViewKind::Architecture
        );
        assert_eq!(app.projection.view.focus, Some(owner));
        assert_eq!(app.projection.view.depth, 1);
        assert!(app.part_edit_ready());
        let previous = app.navigation.back().expect("previous operator location");
        assert_eq!(previous.world, World::Graph);
        assert_eq!(previous.focus, None);
        assert_eq!(
            previous.center,
            [previous_camera.center.x, previous_camera.center.y]
        );
        assert_eq!(previous.zoom, previous_camera.zoom);
    }

    #[test]
    fn pending_or_mismatched_owner_scene_cannot_enable_preparation() {
        let mut app = application();
        let owner = named(&app, "ModelingPlatform");
        app.select(SceneTarget::Node(owner), false);
        app.open_part_edit(CommandId::CreatePart);
        assert!(app.part_edit_ready());
        let view = app.projection.view.clone();

        app.requested_definition = Some((91, view.clone()));
        app.scene_request = 91;
        assert!(!app.part_edit_ready());
        app.prepare_part();
        assert!(app.candidate.is_none());
        assert!(app.status.contains("Opening the selected part"));
        app.requested_definition = None;

        // No pending request is not proof of an installed matching query.
        app.projection.view.focus = None;
        assert!(!app.part_edit_ready());
        app.projection.view = view;
        app.layout_focus = None;
        assert!(!app.part_edit_ready());
        app.layout_focus = Some(owner);
        let revision = app.scene.revision_id;
        app.scene.revision_id = ProjectRevisionId::new();
        assert!(!app.part_edit_ready());
        app.scene.revision_id = revision;
        app.scene_builder.busy = true;
        assert!(!app.part_edit_ready());
        app.scene_builder.busy = false;
        assert!(app.part_edit_ready());
    }

    #[test]
    fn a_later_focus_or_world_change_blocks_create_without_redirecting_the_operator() {
        let mut app = application();
        let owner = named(&app, "ModelingPlatform");
        app.select(SceneTarget::Node(owner), false);
        app.open_part_edit(CommandId::CreatePart);
        assert!(app.part_edit_ready());
        app.focus = None;
        app.prepare_part();
        assert!(app.candidate.is_none());
        assert_eq!(app.focus, None);
        assert!(app.status.contains("view changed"));
        app.world = World::Graph;
        app.focus = Some(owner);
        app.prepare_part();
        assert!(app.candidate.is_none());
        assert_eq!(app.world, World::Graph);
        assert!(!app.bridge.mutation_pending());
    }

    #[test]
    fn a_superseded_live_owner_query_cannot_replace_a_newer_world_intent() {
        use crate::bridge::{Output, Reply};
        let mut app = application();
        let owner = named(&app, "ModelingPlatform");
        // Explicit fixture DTOs test native routing only. No accepted runtime,
        // writable service, or semantic edit authority is minted by this test.
        app.fixture = None;
        app.binding = Some(RevisionBinding {
            project: agq_modeling_repository::ProjectId::new(),
            revision: app.projection.revision_id,
        });
        app.select(SceneTarget::Node(owner), false);
        let context = app.work_context();
        app.open_part_edit(CommandId::CreatePart);
        let owner_request = app.scene_request;
        let mut owner_projection = app.projection.clone();
        owner_projection.view = app
            .edit_target
            .as_ref()
            .unwrap()
            .owner_view
            .clone()
            .unwrap();
        assert_ne!(owner_request, 0);
        assert!(
            app.part_edit_unavailable(CommandId::CreatePart)
                .unwrap()
                .contains("Opening")
        );

        app.world = World::Graph;
        app.focus = None;
        let graph = app.definition();
        app.request_projection_definition(graph.clone());
        let latest = app.scene_request;
        assert_ne!(latest, owner_request);
        let previous = app.projection.clone();
        app.receive_replies([Reply {
            request: owner_request,
            epoch: app.bridge.epoch(),
            context: Some(context),
            read: None,
            mutation: false,
            terminal: true,
            result: Ok(Output::Projection(owner_projection)),
        }]);
        assert_eq!(app.world, World::Graph);
        assert_eq!(app.focus, None);
        assert_eq!(app.projection, previous);
        assert_eq!(app.requested_definition, Some((latest, graph)));
        assert!(!app.pending.contains(&owner_request));
        assert!(!app.part_edit_ready());
        app.prepare_part();
        assert!(!app.bridge.mutation_pending());
        assert!(app.candidate.is_none());
    }

    #[test]
    fn rejected_owner_scene_does_not_enable_a_nested_edit_on_the_previous_scene() {
        use crate::bridge::{Output, Reply};
        let mut app = application();
        let owner = named(&app, "ModelingPlatform");
        app.fixture = None;
        app.binding = Some(RevisionBinding {
            project: agq_modeling_repository::ProjectId::new(),
            revision: app.projection.revision_id,
        });
        app.select(SceneTarget::Node(owner), false);
        let context = app.work_context();
        let previous = app.projection.clone();
        let generation = app.generation;
        app.open_part_edit(CommandId::CreatePart);
        let mut invalid = previous.clone();
        invalid.view = app
            .edit_target
            .as_ref()
            .unwrap()
            .owner_view
            .clone()
            .unwrap();
        invalid.nodes.push(invalid.nodes[0].clone());
        app.receive_replies([Reply {
            request: app.scene_request,
            epoch: app.bridge.epoch(),
            context: Some(context),
            read: None,
            mutation: false,
            terminal: true,
            result: Ok(Output::Projection(invalid)),
        }]);
        assert!(app.status.contains("duplicate element"));
        assert_eq!(app.projection, previous);
        assert_eq!(app.generation, generation);
        assert!(app.requested_definition.is_none());
        assert!(!app.part_edit_ready());
        app.prepare_part();
        assert!(app.candidate.is_none());
        assert!(!app.bridge.mutation_pending());
    }

    #[test]
    fn stale_dialog_cannot_prepare_a_candidate_for_a_new_revision() {
        let mut app = application();
        let selected = named(&app, "ModelRepository");
        app.select(SceneTarget::Node(selected), false);
        app.open_part_edit(CommandId::CreatePart);
        app.projection.revision_id = ProjectRevisionId::new();
        app.prepare_part();
        assert!(app.candidate.is_none());
        assert!(app.status.contains("revision changed"));
        assert!(!app.bridge.mutation_pending());
    }

    #[test]
    fn rename_cannot_use_visual_fixture_authority() {
        let mut app = application();
        let selected = named(&app, "ModelRepository");
        app.select(SceneTarget::Node(selected), false);
        assert!(commands::unavailable(CommandId::RenamePart, &app.context()).is_some());
        app.open_part_edit(CommandId::RenamePart);
        app.new_part_name = "renamed".into();
        app.prepare_rename();
        assert!(app.candidate.is_none());
        assert!(!app.bridge.mutation_pending());
        assert_eq!(
            app.scene.node(selected).unwrap().semantic.name,
            "ModelRepository"
        );
    }
}
