//! Changes a project's model through Agentique's own operations, without a
//! window (C-55, W13.1): for agents working outside the Studio, such as
//! Claude Code during a bootstrap, which must not edit model files by hand.
//!
//! ```text
//! cargo run -p agq-assistant --example apply_changes -- <project folder> <changes.json> [--confirm <qualified name>]...
//! ```
//!
//! `changes.json` holds the input of the Assistant's `apply_changes` tool
//! (`{"description": ..., "operations": [...]}`), or a list of such inputs.
//! Each input is checked against the tool's schema and prepared exactly as
//! the tool prepares it, then applied as one System State change by the
//! Assistant's actor (its description says an external agent made it) and
//! saved, so identities, validation and locks work as in the Studio. A
//! change touching a locked element is refused unless `--confirm` names that
//! element, as the Operator confirms it; the confirmation is written into
//! the change's description. Prints what each change did and the model's
//! problems afterwards; exits with 1 when an input is invalid or refused
//! (the changes before it stay applied, each one its own undo step).

use agq_assistant::tools::{self, Prepared};
use agq_library::Library;
use agq_system_state::{ApplyError, Project};
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(folder), Some(file)) = (args.next(), args.next()) else {
        eprintln!(
            "usage: apply_changes <project folder> <changes.json> [--confirm <qualified name>]..."
        );
        return ExitCode::FAILURE;
    };
    let mut confirm = Vec::new();
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--confirm", Some(name)) => confirm.push(name),
            _ => {
                eprintln!("unknown argument `{arg}`");
                return ExitCode::FAILURE;
            }
        }
    }
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read {file}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let inputs = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(inputs)) => inputs,
        Ok(input) => vec![input],
        Err(error) => {
            eprintln!("{file} is not JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut project = match Project::open(&PathBuf::from(&folder)) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("cannot open the project in {folder}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut confirmed = Vec::new();
    for name in &confirm {
        match project.state().tree().find(name) {
            Some(id) => confirmed.push(id),
            None => {
                eprintln!("--confirm: there is no element `{name}`");
                return ExitCode::FAILURE;
            }
        }
    }
    let library = Library::built_in_only();
    let before = project.state().diagnostics().len();
    for (index, input) in inputs.iter().enumerate() {
        let label = format!("change {} of {}", index + 1, inputs.len());
        if let Err(problem) = tools::check_input(tools::APPLY_CHANGES, input) {
            eprintln!("{label}: {problem}");
            return ExitCode::FAILURE;
        }
        let mut change =
            match tools::prepare(project.state(), &library, tools::APPLY_CHANGES, input) {
                Prepared::Change(change) => change,
                Prepared::Invalid(problem) => {
                    eprintln!("{label}: {problem}");
                    return ExitCode::FAILURE;
                }
                _ => {
                    eprintln!("{label}: apply_changes did not prepare a change");
                    return ExitCode::FAILURE;
                }
            };
        change.description = if confirm.is_empty() {
            format!("{} (by an external agent)", change.description)
        } else {
            format!(
                "{} (by an external agent; the Operator's confirmation given for {})",
                change.description,
                confirm.join(", ")
            )
        };
        change.confirmed = confirmed.clone();
        match project.apply(change) {
            Ok(event) => println!(
                "{label}: {}",
                tools::describe_event(project.state(), &event)
            ),
            Err(ApplyError::Rejection(rejection)) => {
                eprintln!(
                    "{label}: refused: {}",
                    tools::describe_rejection(project.state(), &rejection)
                );
                return ExitCode::FAILURE;
            }
            Err(error) => {
                eprintln!("{label}: not saved: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    let state = project.state();
    println!(
        "{} problem(s) in the model (before: {before})",
        state.diagnostics().len()
    );
    for diagnostic in state.diagnostics() {
        println!(
            "  {} [{}]: {}",
            state.tree().qualified_name(diagnostic.element),
            diagnostic.code,
            diagnostic.message
        );
    }
    ExitCode::SUCCESS
}
