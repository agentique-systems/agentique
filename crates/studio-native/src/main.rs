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
mod panels;
mod selection;
mod session;
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
    /// Exercise real native pointer/keyboard routing against the architecture fixture.
    #[arg(long, value_parser = ["vertical"])]
    scenario: Option<String>,
    #[arg(long, default_value = "verification/generated/native-studio/interaction-report.json")]
    scenario_report: PathBuf,
}

fn main() -> eframe::Result {
    let args = Args::parse();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
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
