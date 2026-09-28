//! Agentique Studio: the Operator builds and changes an architecture on the
//! Surface. Every model change goes through the System State's typed
//! operations; the project saves it.
#![forbid(unsafe_code)]
mod accessibility;
mod app;
#[cfg(feature = "automation")]
mod automation;
// Asserted by the stress harness (feature `automation`); its tests run in every build.
#[cfg_attr(not(feature = "automation"), allow(dead_code))]
mod budgets;
mod commands;
mod conversation;
mod conversation_ui;
mod cost;
mod edit;
mod gallery;
mod gpu;
mod gpu_timing;
mod history;
mod inspector;
mod markdown;
mod motion;
mod navigation;
mod palette_ui;
mod panels;
mod project_dialog;
mod relationship_labels;
mod requirements;
mod selection;
mod session;
// The settings table and settings.json (interface 2 of ROADMAP §6.2).
#[allow(
    dead_code,
    reason = "Scope and the default path are for later settings and tools"
)]
mod settings;
mod settings_ui;
#[cfg(feature = "automation")]
mod stress_automation;
mod surface_recovery;
mod targets;
mod theme;
mod timing;
// Design tokens and the component list (interface 3 of ROADMAP §6.2).
#[allow(
    dead_code,
    reason = "the Studio's drawing moves onto the tokens in W5.2; the gallery shows them all now"
)]
mod tokens;
mod viewport;
mod zoom_input;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Clone, Debug)]
#[command(about = "Agentique Studio")]
pub struct Args {
    /// Show a read-only example instead of a project.
    #[arg(long, value_parser = ["architecture", "typography", "ports", "requirements", "diff", "stress1000", "stress10000", "components"])]
    fixture: Option<String>,
    /// Open this project folder.
    #[arg(long)]
    project: Option<PathBuf>,
    /// Where the Studio remembers the last project, recent projects and cameras.
    #[arg(long)]
    session: Option<PathBuf>,
    /// Save a screenshot of the window after it settles, then close.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Close after this many frames and print frame timings.
    #[arg(long)]
    frames: Option<u64>,
    #[arg(long)]
    metrics: Option<PathBuf>,
    /// Measure the Surface's GPU pass when the adapter supports timestamp queries.
    #[arg(long)]
    gpu_timestamps: bool,
    #[arg(long)]
    light: bool,
    /// Scale the UI, as display scaling does (1.5 is 150%); for checking
    /// text and layout at 100%, 150% and 200% on one display.
    #[arg(long, value_parser = clap::value_parser!(f32))]
    ui_scale: Option<f32>,
    /// Start without reopening the last project.
    #[arg(long)]
    no_restore: bool,
    /// Drive the UI through a scripted journey (a-build, a-crash, a-reopen, a-assistant, d-daily, e-settings), the camera benchmark (stress) or the Conversation benchmark (chat, in a new project at `--project`).
    #[cfg(feature = "automation")]
    #[arg(long, value_parser = ["a-build", "a-crash", "a-reopen", "a-assistant", "d-daily", "e-settings", "stress", "chat"])]
    scenario: Option<String>,
    /// Write the scenario report (JSON) to this path.
    #[cfg(feature = "automation")]
    #[arg(long, requires = "scenario")]
    scenario_report: Option<PathBuf>,
    /// Save screenshots at journey checkpoints into this directory.
    #[cfg(feature = "automation")]
    #[arg(long, requires = "scenario", conflicts_with = "screenshot")]
    gallery: Option<PathBuf>,
}

impl Args {
    /// Whether a scripted scenario drives this process's input.
    #[cfg(feature = "automation")]
    fn scenario_running(&self) -> bool {
        self.scenario.is_some()
    }
    #[cfg(not(feature = "automation"))]
    fn scenario_running(&self) -> bool {
        false
    }
    /// The `a-build`, `a-assistant` and `e-settings` journeys create the project at
    /// `--project` through the UI.
    #[cfg(feature = "automation")]
    fn creates_project(&self) -> bool {
        matches!(
            self.scenario.as_deref(),
            Some("a-build" | "a-assistant" | "d-daily" | "e-settings" | "chat")
        )
    }
    #[cfg(not(feature = "automation"))]
    fn creates_project(&self) -> bool {
        false
    }
}

fn main() -> eframe::Result {
    timing::mark_process_start();
    let args = Args::parse();
    let mut wgpu_setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    if args.gpu_timestamps {
        let original = wgpu_setup.device_descriptor.clone();
        wgpu_setup.device_descriptor = std::sync::Arc::new(move |adapter| {
            let mut descriptor = original(adapter);
            if adapter.features().contains(gpu_timing::features()) {
                descriptor.required_features |= gpu_timing::features();
            }
            descriptor
        });
    }
    let recovery = surface_recovery::Recovery::default();
    let surface_context = std::sync::Arc::new(std::sync::Mutex::new(None::<eframe::egui::Context>));
    let error_context = surface_context.clone();
    let error_recovery = recovery.clone();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            // eframe's own default since 0.35: one frame in flight.
            surface: egui_wgpu::SurfaceConfig::LOW_LATENCY,
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(wgpu_setup),
            on_surface_status: std::sync::Arc::new(move |status| {
                let context = error_context.lock().expect("surface context").clone();
                error_recovery.surface_status(status, context.as_ref())
            }),
        },
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Agentique Studio")
            .with_inner_size([1600.0, 1000.0])
            .with_min_inner_size([1080.0, 720.0])
            .with_app_id("systems.agentique.studio"),
        ..Default::default()
    };
    eframe::run_native(
        "Agentique Studio",
        options,
        Box::new(move |cc| {
            *surface_context.lock().expect("surface context") = Some(cc.egui_ctx.clone());
            if let Some(render_state) = &cc.wgpu_render_state {
                recovery.attach(&cc.egui_ctx, &render_state.device);
            }
            gpu::install(cc, args.gpu_timestamps, recovery).map_err(std::io::Error::other)?;
            Ok(Box::new(app::StudioApp::new(cc, args)))
        }),
    )
}
