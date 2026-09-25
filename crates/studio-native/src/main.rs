//! Agentique Native Studio. Semantic work crosses the in-process platform boundary.
#![forbid(unsafe_code)]
mod actions;
mod agents;
mod app;
mod automation;
mod bridge;
mod commands;
mod gpu;
mod inspector;
mod navigation;
mod palette_ui;
mod panels;
mod presentation;
mod real_automation;
mod real_targets;
mod saved_views;
mod scene_build;
mod selection;
mod session;
mod stress_automation;
mod theme;
mod timing;
mod updates;
mod viewport;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Clone, Debug)]
#[command(about = "Agentique Native Studio — spatial engineering")]
pub struct Args {
    /// Explicit visual fixture. Never substitutes for an authenticated project.
    #[arg(long, value_parser = ["architecture", "ports", "requirements", "diff", "stress1000", "stress10000"])]
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
    #[arg(long)]
    light: bool,
    #[arg(long)]
    no_restore: bool,
    /// Exercise native input routing over explicit fixtures or the accepted real model.
    #[arg(long, value_parser = ["vertical", "stress", "real", "real-restart"])]
    scenario: Option<String>,
    #[arg(
        long,
        default_value = "verification/generated/native-studio/interaction-report.json"
    )]
    scenario_report: PathBuf,
    /// Retain native screenshots and revision-qualified state at journey checkpoints.
    #[arg(long, requires = "scenario", conflicts_with = "screenshot")]
    gallery: Option<PathBuf>,
    /// Prior real journey report, required only for the separate durable restart check.
    #[arg(long, requires = "scenario")]
    restart_report: Option<PathBuf>,
    /// Real acceptance wall-time deadline including runtime restore and semantic work.
    #[arg(long, default_value_t = 14400)]
    scenario_timeout_seconds: u64,
}

fn main() -> eframe::Result {
    let args = Args::parse();
    if let Err(error) = real_automation::validate_launch(&args) {
        eprintln!("Native real acceptance launch refused: {error}");
        std::process::exit(2);
    }
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            on_surface_error: std::sync::Arc::new(surface_error),
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
            gpu::install(cc).map_err(std::io::Error::other)?;
            Ok(Box::new(app::StudioApp::new(cc, args)?))
        }),
    )
}

fn surface_error(error: wgpu::SurfaceError) -> egui_wgpu::SurfaceErrorAction {
    // Eframe owns the surface and applies this action before the next frame.
    // Timeout is transient; lost/outdated surfaces need reconfiguration.
    match error {
        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
            egui_wgpu::SurfaceErrorAction::RecreateSurface
        }
        other => {
            eprintln!("Native surface frame unavailable: {other}");
            egui_wgpu::SurfaceErrorAction::SkipFrame
        }
    }
}
