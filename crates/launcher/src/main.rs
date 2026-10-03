//! `agentique-launcher`: starts the current build of Agentique (ROADMAP
//! §4.15, C-51).
//!
//! ```text
//! agentique-launcher [args…]                        the current build
//! agentique-launcher --recover [args…]              the last known good build, in safe mode
//! agentique-launcher --adopt <id> --wait <lock> [args…]
//!                                                   after the running Agentique hands over:
//!                                                   build <id>, falling back if it does not start
//! ```
//!
//! Other arguments (such as `--project <folder>`) go to the build. If no
//! build starts, the builds folder opens, with `launcher.log` saying why.
#![cfg_attr(windows, windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use agq_launcher::{READY_WITHIN, Registry, Started, default_root, start, wait_for_release};
use std::time::Duration;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut take = |flag: &str, with_value: bool| -> Option<String> {
        let at = args.iter().position(|a| a == flag)?;
        if with_value {
            if at + 1 >= args.len() {
                args.remove(at);
                return None;
            }
            let value = args.remove(at + 1);
            args.remove(at);
            Some(value)
        } else {
            args.remove(at);
            Some(String::new())
        }
    };
    let recover = take("--recover", false).is_some();
    let adopt = take("--adopt", true);
    let wait = take("--wait", true);
    let root = default_root();
    let log = root.join("launcher.log");
    let note = |line: String| {
        use std::io::Write;
        let _ = std::fs::create_dir_all(&root);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
        {
            let _ = writeln!(file, "{} {line}", agq_launcher::now());
        }
    };
    if let Some(lock) = wait
        && let Err(reason) = wait_for_release(std::path::Path::new(&lock), Duration::from_secs(90))
    {
        note(format!("no handover: {reason}"));
        open_folder(&root);
        std::process::exit(1);
    }
    let registry = match Registry::load(&root) {
        Ok(registry) => registry,
        Err(reason) => {
            note(reason);
            open_folder(&root);
            std::process::exit(1);
        }
    };
    let id = if recover {
        args.push("--safe-mode".into());
        registry
            .last_known_good
            .clone()
            .or(registry.current.clone())
    } else if let Some(id) = adopt {
        Some(id)
    } else {
        registry
            .current
            .clone()
            .or(registry.last_known_good.clone())
    };
    let Some(id) = id else {
        note(
            "there is no build to start: install one from Agentique (Settings › About › Builds)"
                .into(),
        );
        open_folder(&root);
        std::process::exit(1);
    };
    match start(&root, &id, &args, READY_WITHIN) {
        Started::Ready { .. } | Started::FellBack { .. } => {}
        Started::Failed { .. } => {
            open_folder(&root);
            std::process::exit(2);
        }
    }
}

/// Shows the builds folder (with `launcher.log`) to the Operator.
fn open_folder(root: &std::path::Path) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(root).spawn();
    #[cfg(not(windows))]
    let _ = root;
}
