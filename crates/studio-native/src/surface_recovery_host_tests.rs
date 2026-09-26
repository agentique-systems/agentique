//! The real host update/save/reopen path with renderer callbacks injected before
//! input routing. These tests do not claim GPU or physical device qualification.
use super::*;
use crate::{app::StudioApp, automation, commands::CommandId, session::Session};
use agq_modeling_repository::ProjectRevisionId;
use agq_studio_scene::{Point, SceneTarget};
use clap::Parser;
use eframe::App;

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "agq-graphics-checkpoint-{}",
            ProjectRevisionId::new()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn application(directory: &Directory, restore: bool) -> (StudioApp, egui::Context) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let database = directory.0.join("project.sqlite");
    let mut arguments = vec![
        "studio",
        "--root",
        root.to_str().unwrap(),
        "--database",
        database.to_str().unwrap(),
    ];
    if !restore {
        arguments.extend(["--fixture", "architecture", "--no-restore"]);
    }
    let context = egui::Context::default();
    let creation = eframe::CreationContext::_new_kittest(context.clone());
    let app = StudioApp::new(&creation, crate::Args::parse_from(arguments)).unwrap();
    (app, context)
}

fn frame(
    app: &mut StudioApp,
    context: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |context| app.update(context, &mut eframe::Frame::_new_kittest()),
    )
}

fn open_edit(app: &mut StudioApp, context: &egui::Context) -> egui::Pos2 {
    let owner = app
        .scene
        .nodes
        .iter()
        .find(|node| node.semantic.name == "ModelingPlatform")
        .unwrap()
        .id();
    app.select(SceneTarget::Node(owner), false);
    app.open_part_edit(CommandId::CreatePart);
    app.new_part_name = "mustNotPrepareWhileGraphicsAreUnavailable".into();
    assert!(app.part_edit_ready());
    frame(app, context, Vec::new());
    frame(app, context, Vec::new());
    automation::target(context, automation::Target::CandidatePrepare)
        .unwrap()
        .center()
}

fn click(app: &mut StudioApp, context: &egui::Context, position: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            app,
            context,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

fn fault(context: &egui::Context, device: bool) -> Recovery {
    let recovery = Recovery::default();
    context.data_mut(|data| {
        data.insert_temp(egui::Id::new("native-surface-recovery"), recovery.clone())
    });
    if device {
        recovery.device_lost(
            wgpu::DeviceLostReason::Unknown,
            "host recovery regression; no physical GPU reset",
            Some(context),
        );
    } else {
        for _ in 0..6 {
            recovery.surface_error(wgpu::SurfaceError::Lost, Some(context));
        }
    }
    recovery
}

#[test]
fn terminal_surface_and_device_faults_checkpoint_reopen_and_reject_edit_input() {
    // Positive control: the same ordinary widget input really prepares a visual
    // candidate when the renderer is available. No semantic acceptance is claimed.
    let control_directory = Directory::new();
    let (mut control, context) = application(&control_directory, false);
    let button = open_edit(&mut control, &context);
    click(&mut control, &context, button);
    assert!(control.candidate.is_some(), "input must exercise Prepare");

    for device in [false, true] {
        let directory = Directory::new();
        let (mut app, context) = application(&directory, false);
        let button = open_edit(&mut app, &context);
        // The fixture initializer selects this object on reopen too. Keep that
        // independent existing behavior out of this renderer-stop regression.
        let selected = app
            .scene
            .nodes
            .iter()
            .find(|node| node.semantic.name == "ModelRepository")
            .unwrap()
            .id();
        app.select(SceneTarget::Node(selected), false);
        app.camera.center = Point::new(713.0, -219.0);
        app.camera.zoom = 0.83;
        let expected = app.capture_presentation();
        let revision = app.projection.revision_id;
        let binding = app.binding;
        let projection = app.projection.clone();
        let pending = app.pending.clone();
        fault(&context, device);
        click(&mut app, &context, button);

        assert!(app.candidate.is_none());
        assert!(app.preparation.is_none());
        assert!(!app.bridge.mutation_pending());
        assert_eq!(app.pending, pending);
        assert_eq!(app.binding, binding);
        assert_eq!(app.projection, projection);
        assert!(app.create_dialog, "fault input cannot submit the dialog");
        assert!(app.status.contains("Presentation state was saved"));
        assert!(
            !app.config.database.exists(),
            "presentation is not a model write"
        );
        let saved = Session::load(&app.session_path).expect("fault checkpoint");
        assert_eq!(saved.revision, revision);
        assert_eq!(
            serde_json::to_value(saved.presentation.unwrap()).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        drop(app);

        let (reopened, _) = application(&directory, true);
        assert_eq!(reopened.projection.revision_id, revision);
        assert_eq!(reopened.world, expected.world);
        assert_eq!(reopened.focus, expected.definition.focus);
        assert_eq!(reopened.camera.center, expected.camera.center);
        assert_eq!(reopened.camera.zoom, expected.camera.zoom);
        assert_eq!(reopened.selection, expected.selection.unwrap());
        assert!(reopened.candidate.is_none());
        assert!(reopened.preparation.is_none());
        assert!(!reopened.create_dialog);
        assert!(!reopened.config.database.exists());
    }
}

#[test]
fn fault_retains_previous_checkpoint_until_pending_view_is_stable() {
    let directory = Directory::new();
    let (mut app, context) = application(&directory, false);
    app.save_session();
    let previous = std::fs::read(&app.session_path).unwrap();
    app.camera.center = Point::new(510.0, 420.0);
    app.pending_revision = Some(agq_studio_platform::RevisionBinding {
        project: agq_modeling_repository::ProjectId::new(),
        revision: ProjectRevisionId::new(),
    });
    fault(&context, false);
    frame(&mut app, &context, Vec::new());
    assert_eq!(std::fs::read(&app.session_path).unwrap(), previous);
    assert!(app.status.contains("waiting for the current view"));
    // The in-flight projection completion may make saving possible during the
    // same graphics incident. It must not wait for a new incident or lose state.
    app.pending_revision = None;
    frame(&mut app, &context, Vec::new());
    let saved = Session::load(&app.session_path).unwrap();
    assert_eq!(saved.camera.center, app.camera.center);
    assert!(app.status.contains("Presentation state was saved"));
}

#[test]
fn checkpoint_failure_stays_actionable_and_does_not_retry_on_every_fault_frame() {
    let directory = Directory::new();
    let (mut app, context) = application(&directory, false);
    app.save_session();
    let previous_path = app.session_path.clone();
    let previous = std::fs::read(&previous_path).unwrap();
    // Atomic replacement of a directory must fail on every supported platform.
    let blocked_path = directory.0.join("blocked-presentation.json");
    std::fs::create_dir(&blocked_path).unwrap();
    app.session_path = blocked_path;
    fault(&context, true);
    let output = frame(&mut app, &context, Vec::new());
    assert!(app.status.contains("Presentation state was not saved"));
    assert!(
        app.status
            .contains("last successful presentation checkpoint")
    );
    assert!(output.viewport_output.values().any(|viewport| {
        viewport.commands.iter().any(|command| {
            matches!(command, egui::ViewportCommand::Title(title) if title.contains("presentation not saved") && title.contains("restart"))
        })
    }));
    let attempted = app.last_saved;
    frame(&mut app, &context, Vec::new());
    assert_eq!(
        app.last_saved, attempted,
        "no per-frame failing disk writes"
    );
    assert!(app.status.contains("Presentation state was not saved"));
    assert_eq!(std::fs::read(&previous_path).unwrap(), previous);
    assert!(Session::load(&previous_path).is_some());
    assert!(!app.bridge.mutation_pending());
    assert!(!app.config.database.exists());
}
