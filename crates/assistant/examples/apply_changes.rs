//! Changes a project's model through Agentique's own operations, without a
//! window (C-55, W13.1): for agents working outside the Studio, such as
//! Claude Code during a bootstrap, which must not edit model files by hand.
//!
//! ```text
//! cargo run -p agq-assistant --example apply_changes -- <project folder> <changes.json> [--confirm <qualified name>]... [--lock <qualified name>]...
//! ```
//!
//! `changes.json` holds the input of the Assistant's `apply_changes` tool
//! (`{"description": ..., "operations": [...]}`), or a non-empty list of
//! such inputs. Each input is checked against the tool's schema and prepared
//! exactly as the tool prepares it, then applied as one System State change
//! by the Assistant's actor (its description says an external agent made it)
//! and saved, so identities, validation and locks work as in the Studio.
//!
//! Locks are the Operator's (§4.2): `--confirm` and `--lock` are given only
//! on the Operator's explicit instruction for that change. A change is first
//! tried without any confirmation; only when it is refused for locked
//! elements, every one of them named by `--confirm`, is it applied with the
//! confirmation of exactly those elements, and its description says the lock
//! was confirmed on the command line. A `--confirm` naming an element that
//! carries no lock is refused. `--lock` locks the named elements after the
//! changes, as one more change.
//!
//! The changes are saved, not committed (a checkpoint or git commits them;
//! git reverts them). Exits with 1 when an input is invalid or refused (the
//! changes before it stay saved), with 3 when the changes added problems to
//! the model, and with 0 otherwise.

use agq_assistant::tools::{self, Prepared};
use agq_library::Library;
use agq_system_state::{Actor, ApplyError, Change, Operation, Project, Rejection};
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(folder), Some(file)) = (args.next(), args.next()) else {
        eprintln!(
            "usage: apply_changes <project folder> <changes.json> [--confirm <qualified name>]... [--lock <qualified name>]..."
        );
        return ExitCode::FAILURE;
    };
    let mut confirm = Vec::new();
    let mut lock = Vec::new();
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--confirm", Some(name)) => confirm.push(name),
            ("--lock", Some(name)) => lock.push(name),
            ("--confirm" | "--lock", None) => {
                eprintln!("{arg} needs the qualified name of an element");
                return ExitCode::FAILURE;
            }
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
    if inputs.is_empty() {
        eprintln!("{file} holds no change");
        return ExitCode::FAILURE;
    }
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
            Some(id) if project.state().locks().contains(&id) => confirmed.push(id),
            Some(_) => {
                eprintln!("--confirm: `{name}` carries no lock");
                return ExitCode::FAILURE;
            }
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
        let description = std::mem::take(&mut change.description);
        change.description = format!("{description} (by an external agent)");
        let result = match project.apply(change.clone()) {
            Err(ApplyError::Rejection(Rejection::Locked { elements }))
                if !elements.is_empty() && elements.iter().all(|id| confirmed.contains(id)) =>
            {
                let names: Vec<String> = elements
                    .iter()
                    .map(|id| project.state().tree().qualified_name(*id))
                    .collect();
                change.description = format!(
                    "{description} (by an external agent; the lock of {} confirmed on the command line, --confirm)",
                    names.join(", ")
                );
                change.confirmed = elements;
                project.apply(change)
            }
            other => other,
        };
        if !report(&project, &label, result) {
            return ExitCode::FAILURE;
        }
    }
    if !lock.is_empty() {
        let mut operations = Vec::new();
        for name in &lock {
            match project.state().tree().find(name) {
                Some(element) => operations.push(Operation::Lock { element }),
                None => {
                    eprintln!("--lock: there is no element `{name}`");
                    return ExitCode::FAILURE;
                }
            }
        }
        let change = Change::new(
            Actor::Operator,
            &format!(
                "Lock {} (on the Operator's instruction, given on the command line, --lock)",
                lock.join(", ")
            ),
            operations,
        );
        let result = project.apply(change);
        if !report(&project, "locks", result) {
            return ExitCode::FAILURE;
        }
    }
    let state = project.state();
    let after = state.diagnostics().len();
    println!("{after} problem(s) in the model (before: {before})");
    for diagnostic in state.diagnostics() {
        println!(
            "  {} [{}]: {}",
            state.tree().qualified_name(diagnostic.element),
            diagnostic.code,
            diagnostic.message
        );
    }
    if after > before {
        ExitCode::from(3)
    } else {
        ExitCode::SUCCESS
    }
}

/// Prints what a change did, or why it was not applied; whether it was.
fn report(
    project: &Project,
    label: &str,
    result: Result<agq_system_state::ChangeEvent, ApplyError>,
) -> bool {
    match result {
        Ok(event) => {
            println!(
                "{label}: {}",
                tools::describe_event(project.state(), &event)
            );
            true
        }
        Err(ApplyError::Rejection(rejection)) => {
            eprintln!(
                "{label}: refused: {}",
                tools::describe_rejection(project.state(), &rejection)
            );
            false
        }
        Err(error) => {
            eprintln!("{label}: not saved: {error}");
            false
        }
    }
}
