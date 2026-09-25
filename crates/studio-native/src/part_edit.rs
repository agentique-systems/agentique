//! Reviewable part-edit intent, pinned to the object and revision at dialog open.
use crate::{
    app::{PendingPreparation, StudioApp, muted},
    commands::{self, CommandId},
};
use agq_kernel::ElementId;
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
        });
        self.create_dialog = true;
        self.create_dialog_focus = true;
    }

    pub fn part_edit_target(&mut self, command: CommandId) -> Option<ElementId> {
        let Some(target) = &self.edit_target else {
            self.status = "Select a part and reopen the edit command".into();
            return None;
        };
        if target.command != command
            || target.revision != self.projection.revision_id
            || target.binding != self.binding
            || target.fixture != self.fixture
        {
            self.status =
                "The revision changed; reopen the edit command in the current view".into();
            return None;
        }
        let mut context = self.context();
        context.selected = true;
        context.can_create = self.scene.node(target.element).is_some_and(|node| {
            commands::can_create_part(&node.semantic, self.fixture.is_some(), target.revision)
        });
        if let Some(reason) = commands::unavailable(command, &context) {
            self.status = reason.into();
            return None;
        }
        Some(target.element)
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
        let current = target.revision == self.projection.revision_id
            && target.binding == self.binding
            && target.fixture == self.fixture;
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
                ui.label(muted(if !current {
                    "The revision changed. Close this dialog and reopen the command."
                } else if self.fixture.is_some() {
                    "This creates a visual preview of typed intent. Semantic reconstruction requires the accepted runtime."
                } else {
                    "A Working candidate will be reconstructed. Review and validate it before committing."
                }, self.theme));
                ui.add_space(16.0);
                let prepare = ui.add_enabled(current, egui::Button::new("Prepare candidate"));
                crate::automation::record(ctx, crate::automation::Target::CandidatePrepare, prepare.rect);
                if current && (prepare.clicked() || submit) {
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
        let original = named(&app, "ModelRepository");
        let later = named(&app, "ModelingService");
        app.select(SceneTarget::Node(original), false);
        app.open_part_edit(CommandId::CreatePart);
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
        assert!(candidate.intent.contains("ModelRepository"));
        assert_eq!(
            candidate.phase, None,
            "fixture is never semantic acceptance"
        );
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
