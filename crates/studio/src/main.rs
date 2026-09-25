//! Local host binary. Standards acquisition is deliberately not a startup/build action.
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Agentique Studio — immutable semantic worlds")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long, default_value_t = 7332)]
    port: u16,
    #[arg(long)]
    database: Option<PathBuf>,
    /// Agentique runtime store. Defaults to ~/.agentique (or AGENTIQUE_RUNTIME_DIR).
    #[arg(long, global = true)]
    runtime_dir: Option<PathBuf>,
    /// Direct bundle override. Installed accepted publications are discovered by default.
    #[arg(long)]
    bundle: Option<PathBuf>,
    #[arg(long)]
    kerml_cache: Option<PathBuf>,
    #[arg(long)]
    systems_cache: Option<PathBuf>,
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Authenticate and install an accepted runtime bundle once, including offline bundles.
    Setup {
        #[arg(long)]
        bundle: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let runtime_dir = agq_runtime_publications::runtime_directory(args.runtime_dir.as_deref())?;
    if let Some(Command::Setup { bundle }) = args.command {
        agq_runtime_publications::install_bundle(&bundle, &runtime_dir, &args.root, |phase| {
            eprintln!("{phase:?}");
        })?;
        println!(
            "Authenticated runtime installed in {}",
            runtime_dir.display()
        );
        println!("Launch with cargo run --locked --offline -p agq-studio");
        return Ok(());
    }
    let config = agq_studio::StudioConfig {
        database: args
            .database
            .unwrap_or_else(|| runtime_dir.join("projects/agentique.sqlite")),
        root: args.root,
        kerml_cache: args.kerml_cache,
        systems_cache: args.systems_cache,
        runtime_dir: args.runtime_dir,
        bundle: args.bundle,
    };
    let app = agq_studio::router(config);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", args.port)).await?;
    println!("Agentique Studio http://127.0.0.1:{}/studio", args.port);
    axum::serve(listener, app).await?;
    Ok(())
}
