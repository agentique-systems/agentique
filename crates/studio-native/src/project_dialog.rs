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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::Reply;
    use agq_studio_platform::BootstrapPhase;
    use clap::Parser;

    #[test]
    fn empty_authenticated_repository_can_create_but_fixture_progress_and_stale_ready_cannot() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let args = crate::Args::parse_from([
            "studio",
            "--fixture",
            "architecture",
            "--no-restore",
            "--root",
            root.to_str().unwrap(),
        ]);
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(&creation, args).unwrap();
        assert!(
            app.ready,
            "fixture is displayed without an authenticated host"
        );
        app.new_project_dialog();
        assert!(!app.project_dialog.open);
        app.fixture = None;
        app.ready = false;
        app.binding = None;
        assert!(app.projects.is_empty());

        // Exercise reply routing with explicit test messages. The actual worker
        // is given a missing runtime, so this test grants no semantic authority
        // and cannot create any repository or accepted publication.
        let temporary = std::env::temp_dir().join(format!(
            "agentique-empty-repository-test-{}",
            agq_modeling_repository::ProjectId::new()
        ));
        let mut config = app.config.clone();
        config.database = temporary.join("empty.sqlite");
        config.runtime.bundle = Some(temporary.join("missing.agq-runtime"));
        let first = app.bridge.open(config.clone(), None).unwrap();
        let response = |request, terminal, result| Reply {
            request,
            epoch: request,
            context: None,
            read: None,
            mutation: false,
            terminal,
            result,
        };
        app.receive_replies([response(
            first,
            false,
            Ok(Output::Progress(BootstrapPhase::Ready)),
        )]);
        app.new_project_dialog();
        assert!(!app.project_dialog.open);
        app.receive_replies([response(first, false, Ok(Output::Ready(vec![])))]);
        assert!(!app.authenticated_runtime_ready());

        app.receive_replies([response(first, true, Ok(Output::Ready(vec![])))]);
        assert!(app.authenticated_runtime_ready());
        assert!(app.projects.is_empty() && app.binding.is_none());
        let output = ctx.run(egui::RawInput::default(), |ctx| app.setup(ctx));
        let text: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text()),
                _ => None,
            })
            .collect();
        assert!(text.contains(&"Create your first project"));
        assert!(!text.contains(&"Authenticate and install runtime"));
        app.new_project_dialog();
        assert!(app.project_dialog.open);
        app.project_dialog.open = false;

        let next = app.bridge.open(config.clone(), None).unwrap();
        assert!(!app.authenticated_runtime_ready());
        app.receive_replies([response(first, true, Ok(Output::Ready(vec![])))]);
        app.new_project_dialog();
        assert!(!app.project_dialog.open);
        app.receive_replies([response(next, true, Err("Runtime unavailable".into()))]);
        assert_eq!(app.runtime_ready_epoch, None);
        assert!(!app.authenticated_runtime_ready());
        assert!(!config.database.exists());
    }
}

impl StudioApp {
    pub fn authenticated_runtime_ready(&self) -> bool {
        self.runtime_ready_epoch == Some(self.bridge.epoch())
    }

    pub fn new_project_dialog(&mut self) {
        if self.authenticated_runtime_ready() && self.allow_context_change() {
            self.project_dialog.open = true;
            self.project_dialog.repository = self.config.database.display().to_string();
        }
    }

    pub fn project_dialogs(&mut self, ctx: &egui::Context) {
        let mut create = false;
        let runtime_ready = self.authenticated_runtime_ready();
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
                create = ui.add_enabled(runtime_ready && self.pending.is_empty() && !self.project_dialog.name.trim().is_empty(), egui::Button::new("Create project")).clicked();
            });
        if create && self.authenticated_runtime_ready() && self.allow_context_change() {
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
