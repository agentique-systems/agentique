//! Local project/source input; semantic construction stays on the modeling worker.
use crate::{
    app::{PendingPreparation, StudioApp},
    bridge::Output,
};
use eframe::egui;
use std::path::PathBuf;

#[derive(Default)]
pub struct ProjectDialog {
    pub open: bool,
    pub name: String,
    pub repository: String,
    pub import_open: bool,
    pub source_path: String,
}

impl StudioApp {
    pub fn new_project_dialog(&mut self) {
        if self.allow_context_change() && !self.projects.is_empty() {
            self.project_dialog.open = true;
            self.project_dialog.repository = self.config.database.display().to_string();
        }
    }

    pub fn project_dialogs(&mut self, ctx: &egui::Context) {
        let mut create = false;
        egui::Window::new("New project")
            .open(&mut self.project_dialog.open)
            .collapsible(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut self.project_dialog.name);
                ui.label("Repository location (.sqlite)");
                ui.add(egui::TextEdit::singleline(&mut self.project_dialog.repository).desired_width(f32::INFINITY));
                ui.label("Creates an empty Working project. Add source, review, and validate when ready.");
                create = ui.add_enabled(self.pending.is_empty() && !self.project_dialog.name.trim().is_empty(), egui::Button::new("Create project")).clicked();
            });
        if create && self.allow_context_change() {
            let name = self.project_dialog.name.trim().to_owned();
            let database = PathBuf::from(&self.project_dialog.repository);
            if !database.is_absolute() {
                self.status = "Enter an absolute repository file path".into();
            } else {
                let request = self.enqueue_mutation(Box::new(move |platform| {
                    let (project, projects) = platform.create_project_at(&name, &database)?;
                    Ok(Output::ProjectCreated {
                        project,
                        projects,
                        database,
                    })
                }));
                if request != 0 {
                    self.project_dialog.open = false;
                }
            }
        }

        let mut import = false;
        egui::Window::new("Add source document")
            .open(&mut self.project_dialog.import_open)
            .collapsible(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label("Local .sysml or .kerml file");
                ui.add(egui::TextEdit::singleline(&mut self.project_dialog.source_path).desired_width(f32::INFINITY));
                ui.label("The file's exact text becomes a Working candidate. Review and validate before committing.");
                import = ui.add_enabled(self.pending.is_empty() && !self.project_dialog.source_path.is_empty(), egui::Button::new("Prepare source candidate")).clicked();
            });
        if import && self.allow_context_change() {
            let (Some(binding), Some(branch)) = (self.binding, self.branch) else {
                return;
            };
            if self.fixture.is_some() {
                return;
            }
            let path = PathBuf::from(&self.project_dialog.source_path);
            let context = agq_modeling_agent::AgentContext {
                project: binding.project,
                branch,
                revision: binding.revision,
                selection: vec![],
            };
            let definition = self.definition();
            let intent = format!(
                "Add {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            let request = self.enqueue_mutation(Box::new(move |platform| {
                let invalid =
                    |message: &str| agq_studio_platform::PlatformError::Invalid(message.into());
                if !path.is_absolute() {
                    return Err(invalid("Choose an absolute local source file path"));
                }
                let language = match path.extension().and_then(|value| value.to_str()) {
                    Some("sysml") => agq_studio_platform::SourceLanguage::SysMl,
                    Some("kerml") => agq_studio_platform::SourceLanguage::KerMl,
                    _ => return Err(invalid("Choose a .sysml or .kerml file")),
                };
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| invalid("Source filename must be UTF-8"))?
                    .to_owned();
                let source = std::fs::read_to_string(&path).map_err(|error| {
                    agq_studio_platform::PlatformError::Invalid(format!(
                        "Could not read {}: {error}",
                        path.display()
                    ))
                })?;
                platform
                    .propose(
                        context,
                        agq_modeling_agent::ModelCommand::AddSourceDocument {
                            path: name,
                            source,
                            language,
                        },
                        &definition,
                    )
                    .map(Output::Candidate)
            }));
            if request != 0 {
                self.preparation = Some(PendingPreparation {
                    request,
                    started: std::time::Instant::now(),
                    cancelled: false,
                    intent,
                });
                self.project_dialog.import_open = false;
                self.status = "Preparing source candidate in the background".into();
            }
        }
    }
}
