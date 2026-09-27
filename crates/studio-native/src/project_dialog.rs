//! New project and Open project.
use crate::{
    edit::modal,
    targets::{Target, record},
    theme::Theme,
};
use eframe::egui::{self, Key};
use std::path::PathBuf;

/// `Some(true)` creates, `Some(false)` cancels, `None` keeps the dialog open.
pub fn new_project(
    ctx: &egui::Context,
    theme: Theme,
    folder: &mut String,
    name: &mut String,
) -> Option<bool> {
    let mut result = None;
    modal(ctx, "New project", |ui| {
        ui.label(crate::app::muted(
            "A project is a folder; the model is saved in it as SysML text.",
            theme,
        ));
        ui.label("Name");
        let name_field = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
        record(ui.ctx(), Target::Field("Project name"), name_field.rect);
        ui.label("Folder");
        let folder_field = ui.add(egui::TextEdit::singleline(folder).desired_width(f32::INFINITY));
        record(ui.ctx(), Target::Field("Project folder"), folder_field.rect);
        let enter = (name_field.lost_focus() || folder_field.lost_focus())
            && ui.input(|i| i.key_pressed(Key::Enter));
        let ready = !name.trim().is_empty() && !folder.trim().is_empty();
        ui.horizontal(|ui| {
            let create = ui.add_enabled(ready, egui::Button::new("Create project"));
            record(ui.ctx(), Target::Button("Create project"), create.rect);
            if create.clicked() || (enter && ready) {
                result = Some(true);
            }
            if ui.button("Cancel").clicked() {
                result = Some(false);
            }
        });
    });
    result
}

/// `Some(Some(folder))` opens, `Some(None)` cancels, `None` keeps it open.
pub fn open_project(
    ctx: &egui::Context,
    theme: Theme,
    folder: &mut String,
    recent: &[PathBuf],
) -> Option<Option<PathBuf>> {
    let mut result = None;
    modal(ctx, "Open project", |ui| {
        ui.label("Folder");
        let field = ui.add(
            egui::TextEdit::singleline(folder)
                .hint_text("C:\\Users\\you\\Agentique\\MySystem")
                .desired_width(f32::INFINITY),
        );
        record(ui.ctx(), Target::Field("Open folder"), field.rect);
        if !field.has_focus() && !field.lost_focus() && folder.is_empty() {
            field.request_focus();
        }
        let enter = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        ui.horizontal(|ui| {
            let open = ui.add_enabled(!folder.trim().is_empty(), egui::Button::new("Open"));
            record(ui.ctx(), Target::Button("Open"), open.rect);
            if open.clicked() || (enter && !folder.trim().is_empty()) {
                result = Some(Some(PathBuf::from(folder.trim())));
            }
            if ui.button("Cancel").clicked() {
                result = Some(None);
            }
        });
        if !recent.is_empty() {
            theme.section(ui, "RECENT PROJECTS");
            for path in recent {
                if ui.button(path.display().to_string()).clicked() {
                    result = Some(Some(path.clone()));
                }
            }
        }
    });
    result
}
