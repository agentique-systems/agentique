//! Agentique Native Studio. Semantic work crosses the in-process platform boundary.
#![forbid(unsafe_code)]
mod actions;
mod agents;
mod app;
#[cfg(feature = "automation")]
mod automation;
mod bridge;
mod commands;
mod gpu;
mod gpu_timing;
mod history;
mod inspector;
mod loading;
mod navigation;
mod palette_ui;
mod panels;
mod part_edit;
mod presentation;
mod project_dialog;
mod read_lane;
mod relationship_labels;
mod requirements;
mod revision_reads;
mod saved_views;
mod scene_build;
mod selection;
mod session;
#[cfg(feature = "automation")]
mod stress_automation;
mod surface_recovery;
mod targets;
mod theme;
mod timing;
mod updates;
#[cfg(test)]
mod view_intent_tests;
mod viewport;
mod zoom_input;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Clone, Debug)]
#[command(about = "Agentique Native Studio — spatial engineering")]
pub struct Args {
    /// Explicit visual fixture. Never substitutes for an authenticated project.
    #[arg(long, value_parser = ["architecture", "typography", "ports", "requirements", "diff", "stress1000", "stress10000"])]
    fixture: Option<String>,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    runtime_dir: Option<PathBuf>,
    #[arg(long)]
    database: Option<PathBuf>,
    /// Capture the actual native GPU surface after a settling period.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Benchmark frames then close; vsync stays enabled and timings are wall-frame intervals.
    #[arg(long)]
    frames: Option<u64>,
    #[arg(long)]
    metrics: Option<PathBuf>,
    /// Measure the custom scene GPU pass when the adapter supports timestamp queries.
    #[arg(long)]
    gpu_timestamps: bool,
    #[arg(long)]
    light: bool,
    #[arg(long)]
    no_restore: bool,
    /// Drive native input through a scripted fixture journey or the camera benchmark.
    #[cfg(feature = "automation")]
    #[arg(long, value_parser = ["vertical", "keyboard", "stress"])]
    scenario: Option<String>,
    /// Write the scenario report (JSON) to this path.
    #[cfg(feature = "automation")]
    #[arg(long, requires = "scenario")]
    scenario_report: Option<PathBuf>,
    /// Save native screenshots at journey checkpoints into this directory.
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
}

fn main() -> eframe::Result {
    let args = Args::parse();
    let mut wgpu_setup = egui_wgpu::WgpuSetupCreateNew::default();
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
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(wgpu_setup),
            on_surface_error: std::sync::Arc::new(move |error| {
                let context = error_context.lock().expect("surface context").clone();
                error_recovery.surface_error(error, context.as_ref())
            }),
            ..Default::default()
        },
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Agentique · Native Studio")
            .with_inner_size([1600.0, 1000.0])
            .with_min_inner_size([1080.0, 720.0])
            .with_app_id("systems.agentique.studio.native"),
        ..Default::default()
    };
    eframe::run_native(
        "Agentique Native Studio",
        options,
        Box::new(move |cc| {
            *surface_context.lock().expect("surface context") = Some(cc.egui_ctx.clone());
            if let Some(render_state) = &cc.wgpu_render_state {
                recovery.attach(&cc.egui_ctx, &render_state.device);
            }
            gpu::install(cc, args.gpu_timestamps, recovery).map_err(std::io::Error::other)?;
            Ok(Box::new(app::StudioApp::new(cc, args)?))
        }),
    )
}
