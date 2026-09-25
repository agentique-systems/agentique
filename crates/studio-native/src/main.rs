//! Agentique Native Studio. Semantic work crosses the in-process platform boundary.
#![forbid(unsafe_code)]
mod actions;
mod agents;
mod app;
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
mod presentation_automation;
mod read_lane;
mod real_automation;
mod real_targets;
mod revision_reads;
mod saved_views;
mod scene_build;
mod selection;
mod session;
mod stress_automation;
mod surface_recovery;
mod theme;
mod timing;
mod updates;
#[cfg(test)]
mod view_intent_tests;
mod viewport;

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
    /// Exercise native input routing over explicit fixtures or the accepted real model.
    #[arg(long, value_parser = ["vertical", "keyboard", "stress", "real", "real-restart", "presentation", "presentation-restart"])]
    scenario: Option<String>,
    #[arg(
        long,
        default_value = "verification/generated/native-studio/interaction-report.json"
    )]
    scenario_report: PathBuf,
    /// Retain native screenshots and revision-qualified state at journey checkpoints.
    #[arg(long, requires = "scenario", conflicts_with = "screenshot")]
    gallery: Option<PathBuf>,
    /// Prior journey report for a separate real or presentation restart check.
    #[arg(long, requires = "scenario")]
    restart_report: Option<PathBuf>,
    /// Repeat the full real journey from an untouched baseline in a failed report.
    #[arg(long, requires = "scenario", conflicts_with = "restart_report")]
    resume_report: Option<PathBuf>,
    /// Real acceptance wall-time deadline including runtime restore and semantic work.
    #[arg(long, default_value_t = 14400)]
    scenario_timeout_seconds: u64,
}

fn main() -> eframe::Result {
    let args = Args::parse();
    if let Err(error) = presentation_automation::validate_launch(&args) {
        eprintln!("Native presentation qualification launch refused: {error}");
        std::process::exit(2);
    }
    if let Err(error) = real_automation::validate_launch(&args) {
        eprintln!("Native real acceptance launch refused: {error}");
        std::process::exit(2);
    }
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
