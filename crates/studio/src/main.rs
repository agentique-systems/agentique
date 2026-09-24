//! Local host binary. Standards acquisition is deliberately not a startup/build action.
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Agentique Studio — immutable semantic worlds")]
struct Args {
    #[arg(long, default_value_t = 7332)]
    port: u16,
    #[arg(long, default_value = ".workspaces/studio.sqlite")]
    database: PathBuf,
    #[arg(long)]
    kerml_cache: Option<PathBuf>,
    #[arg(long)]
    systems_cache: Option<PathBuf>,
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = agq_studio::StudioConfig {
        database: args.database,
        root: args.root,
        kerml_cache: args
            .kerml_cache
            .or_else(|| std::env::var_os("AGENTIQUE_KERML_CACHE").map(PathBuf::from)),
        systems_cache: args
            .systems_cache
            .or_else(|| std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").map(PathBuf::from)),
    };
    let app = agq_studio::router(config);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", args.port)).await?;
    println!("Agentique Studio http://127.0.0.1:{}/studio", args.port);
    axum::serve(listener, app).await?;
    Ok(())
}
