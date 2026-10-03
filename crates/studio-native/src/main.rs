//! Agentique Studio: the Operator builds and changes an architecture on the
//! Surface and with the Assistant. Every model change goes through the
//! System State's typed operations; the project saves it. The Studio draws
//! with GPUI (C-48).
#![forbid(unsafe_code)]
#[cfg(feature = "automation")]
mod automation;
// Asserted by the stress harness (feature `automation`); its tests run in every build.
mod agent_runtime;
#[cfg_attr(not(feature = "automation"), allow(dead_code))]
mod budgets;
mod commands;
mod control;
mod conversation;
mod conversation_view;
mod cost;
mod develop;
mod dialogs;
mod edit;
mod gallery;
mod history;
mod implementation;
mod library;
mod live;
mod motion;
mod navigation;
mod palette;
mod panels;
mod relationship_labels;
mod requirements;
mod runs;
mod selection;
mod session;
// The settings table and settings.json (interface 2 of ROADMAP §6.2).
#[allow(
    dead_code,
    reason = "Scope and the default path are for later settings and tools"
)]
mod settings;
mod settings_view;
#[cfg(feature = "automation")]
mod stress_automation;
mod studio;
mod surface;
mod tasks;
mod timing;
// Design tokens and the component list (interface 3 of ROADMAP §6.2).
mod tokens;
mod ui;
mod welcome;
mod workspace;

use clap::Parser;
use gpui::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
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
    #[arg(long)]
    light: bool,
    /// Scale the UI, as display scaling does (1.5 is 150%); for checking
    /// text and layout at 100%, 150% and 200% on one display.
    #[arg(long, value_parser = clap::value_parser!(f32))]
    ui_scale: Option<f32>,
    /// Start without reopening the last project.
    #[arg(long)]
    no_restore: bool,
    /// Start without the Claude Agent runtime, whatever Settings say (the
    /// launcher's recovery start).
    #[arg(long)]
    safe_mode: bool,
    /// Write this file once the window is up (the launcher waits for it).
    #[arg(long)]
    ready_file: Option<PathBuf>,
    /// Open the control interface's local endpoint and describe it in this
    /// file (port and token), so an agent can operate this Studio (C-53).
    #[arg(long)]
    control: Option<PathBuf>,
    /// Started by the supervising launcher (C-53): hand over to another
    /// build by exiting with its handover code.
    #[arg(long)]
    supervised: bool,
    /// Started for the adoption of this build: report ready only after the
    /// check after adoption (C-53, ROADMAP §4.16).
    #[arg(long)]
    adopted: Option<String>,
    /// The launcher started this build because that one did not start.
    #[arg(long)]
    recovered_from: Option<String>,
    /// Print what this build is (its data formats and companion) as JSON,
    /// and end.
    #[arg(long)]
    describe: bool,
    /// Drive the UI through a scripted journey (a-build, a-crash, a-reopen, a-assistant, c-understand, d-daily, e-settings, h-library, i-scenarios, i-code), the camera benchmark (stress) or the Conversation benchmark (chat, in a new project at `--project`).
    #[cfg(feature = "automation")]
    #[arg(long, value_parser = ["a-build", "a-crash", "a-reopen", "a-assistant", "c-understand", "d-daily", "e-settings", "h-library", "i-scenarios", "i-code", "stress", "chat"])]
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
    /// The `a-build`, `a-assistant`, `d-daily`, `e-settings`, `h-library`,
    /// `i-scenarios` and `chat` journeys create the project at `--project`
    /// through the UI.
    #[cfg(feature = "automation")]
    fn creates_project(&self) -> bool {
        matches!(
            self.scenario.as_deref(),
            Some(
                "a-build"
                    | "a-assistant"
                    | "d-daily"
                    | "e-settings"
                    | "h-library"
                    | "i-scenarios"
                    | "i-code"
                    | "chat"
            )
        )
    }
    #[cfg(not(feature = "automation"))]
    fn creates_project(&self) -> bool {
        false
    }
    /// A run that measures: frames are never throttled while the window is
    /// in the background.
    pub(crate) fn measuring(&self) -> bool {
        self.frames.is_some() || self.screenshot.is_some() || self.scenario_running()
    }
}

fn main() {
    timing::mark_process_start();
    let args = Args::parse();
    if args.describe {
        println!("{}", develop::describe());
        return;
    }
    gpui_platform::application()
        .with_assets(ui::icon::Assets)
        .run(move |cx: &mut App| {
            if let Err(error) = cx.text_system().add_fonts(ui::icon::fonts()) {
                eprintln!("The Studio's fonts could not be loaded: {error}");
            }
            gpui_base::init(cx);
            commands::bind(cx);
            ui::menu::bind(cx);
            palette::bind(cx);
            panels::library::bind(cx);
            dialogs::bind(cx);
            conversation_view::bind(cx);
            settings_view::bind(cx);
            workspace::bind(cx);
            let bounds = Bounds::centered(None, size(px(1600.0), px(1000.0)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Agentique Studio".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_min_size: Some(size(px(1080.0), px(720.0))),
                app_id: Some("systems.agentique.studio".into()),
                inactive_frame_interval: if args.measuring() {
                    None
                } else {
                    WindowOptions::default().inactive_frame_interval
                },
                ..Default::default()
            };
            let args = args.clone();
            let opened = cx.open_window(options, move |window, cx| {
                let workspace = cx.new(|cx| workspace::Workspace::new(args, window, cx));
                #[cfg(feature = "automation")]
                automation::start(workspace.clone(), window, cx);
                cx.new(|cx| gpui_base::Root::new(workspace, window, cx))
            });
            if let Err(error) = opened {
                eprintln!("The Studio's window could not be opened: {error}");
                cx.quit();
            }
            cx.activate(true);
        });
}
