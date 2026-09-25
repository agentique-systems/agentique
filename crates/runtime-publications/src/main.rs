//! Explicit distribution lifecycle commands; normal Studio launch is separate.
use agq_runtime_publications::{
    RuntimeConfig, RuntimePhase, accepted_bundle_identity, accepted_contracts, discover,
    install_bundle, pack_bundle, runtime_directory, verify_bundle,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Install and authenticate Agentique's accepted semantic runtime"
)]
struct Args {
    /// Checkout root containing exact pinned library source bytes.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    /// Runtime store override; defaults to AGENTIQUE_RUNTIME_DIR or ~/.agentique.
    #[arg(long, global = true)]
    runtime_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Copy and authenticate a local directory or .agq-runtime package atomically.
    Install {
        #[arg(long)]
        bundle: PathBuf,
    },
    /// Authenticate an existing package without installing or publishing it.
    Verify {
        #[arg(long)]
        bundle: PathBuf,
    },
    /// Package two existing accepted caches. Never rebuilds a publication.
    Pack {
        #[arg(long)]
        kerml_cache: PathBuf,
        #[arg(long)]
        systems_cache: PathBuf,
        /// Directory or portable .agq-runtime ZIP; must not exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Show located files only. Use verify to establish authentication.
    Status,
    /// Show the checked-in authority references used by the bundle contract.
    Contract,
}

fn progress(phase: RuntimePhase) {
    eprintln!(
        "{}",
        serde_json::to_string(&phase).expect("phase serialization")
    );
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Agentique runtime: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let value = match args.command {
        Command::Install { bundle } => serde_json::to_value(install_bundle(
            &bundle,
            &runtime_directory(args.runtime_dir.as_deref())?,
            &args.root,
            progress,
        )?)?,
        Command::Verify { bundle } => {
            serde_json::to_value(verify_bundle(&bundle, &args.root, progress)?)?
        }
        Command::Pack {
            kerml_cache,
            systems_cache,
            output,
        } => serde_json::to_value(pack_bundle(
            &kerml_cache,
            &systems_cache,
            &output,
            &args.root,
            progress,
        )?)?,
        Command::Status => serde_json::to_value(discover(&RuntimeConfig {
            runtime_dir: args.runtime_dir,
            ..Default::default()
        })?)?,
        Command::Contract => {
            let (kerml, systems) = accepted_contracts()?;
            serde_json::json!({
                "format": "agq-accepted-publication-bundle/1",
                "identity": accepted_bundle_identity()?,
                "kerml": kerml,
                "systems": systems,
                "cache_transport_digests": "Issued by pack only after authenticating existing accepted cache bytes",
            })
        }
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
