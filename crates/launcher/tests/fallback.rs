//! The launcher with real processes (ROADMAP §4.15, gate E): a stand-in
//! Studio, compiled here with `rustc`, that either reports ready or exits at
//! once. A build that does not start falls back to the last known good one,
//! which is told what happened; a build that starts becomes current; a build
//! whose executable changed after it was built is not started.

use agq_launcher::{Entry, Manifest, Registry, STUDIO, Started, State, file_digest, start};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FAKE: &str = r#"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exe = std::path::PathBuf::from(&args[0]);
    let folder = exe.parent().unwrap();
    std::fs::write(folder.join("args.txt"), args[1..].join(" ")).unwrap();
    if folder.join("broken").exists() {
        eprintln!("the stand-in Studio fails on purpose");
        std::process::exit(3);
    }
    let at = args.iter().position(|a| a == "--ready-file").unwrap();
    std::fs::write(&args[at + 1], "ready").unwrap();
}
"#;

/// Compiles the stand-in once into `dir`.
fn compile(dir: &Path) -> Option<PathBuf> {
    let source = dir.join("fake.rs");
    std::fs::write(&source, FAKE).unwrap();
    let out = dir.join(STUDIO);
    let status = std::process::Command::new("rustc")
        .args(["--edition", "2021", "-O"])
        .arg(&source)
        .arg("-o")
        .arg(&out)
        .status()
        .ok()?;
    status.success().then_some(out)
}

fn install(root: &Path, studio: &Path, id: &str, broken: bool) {
    let folder = root.join(id);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(studio, folder.join(STUDIO)).unwrap();
    if broken {
        std::fs::write(folder.join("broken"), "").unwrap();
    }
    Manifest {
        format: 1,
        id: id.into(),
        executables: vec![(STUDIO.into(), file_digest(&folder.join(STUDIO)).unwrap())],
        ..Manifest::default()
    }
    .save(&folder)
    .unwrap();
    let mut registry = Registry::load(root).unwrap();
    registry.add(Entry {
        id: id.into(),
        created: agq_launcher::now(),
        commit: format!("commit-{id}"),
        state: State::Built,
    });
    registry.save(root).unwrap();
}

#[test]
fn a_build_that_does_not_start_falls_back_to_the_last_known_good_one() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "good", false);
    install(&root, &studio, "bad", true);
    // The good one starts: current and last known good.
    let args = vec!["--project".to_string(), "C:\\agentique".to_string()];
    assert_eq!(
        start(&root, "good", &args, Duration::from_secs(30)),
        Started::Ready { id: "good".into() }
    );
    let registry = Registry::load(&root).unwrap();
    assert_eq!(registry.current.as_deref(), Some("good"));
    assert_eq!(registry.last_known_good.as_deref(), Some("good"));
    // Adopting the bad one: it exits, the good one runs again and is told.
    match start(&root, "bad", &args, Duration::from_secs(30)) {
        Started::FellBack {
            failed,
            reason,
            running,
        } => {
            assert_eq!(failed, "bad");
            assert_eq!(running, "good");
            assert!(reason.contains("exited"), "{reason}");
            assert!(reason.contains("fails on purpose"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    let registry = Registry::load(&root).unwrap();
    assert_eq!(registry.current.as_deref(), Some("good"));
    assert!(matches!(
        registry.entry("bad").unwrap().state,
        State::Failed { .. }
    ));
    let told = std::fs::read_to_string(root.join("good/args.txt")).unwrap();
    assert!(told.contains("--recovered-from bad"), "{told}");
    assert!(told.contains("--project C:\\agentique"), "{told}");
    // A build changed after it was built is not started.
    std::fs::write(root.join("good").join(STUDIO), b"tampered").unwrap();
    assert!(matches!(
        start(&root, "good", &args, Duration::from_secs(10)),
        Started::Failed { .. }
    ));
    let log = std::fs::read_to_string(root.join("launcher.log")).unwrap();
    assert!(log.contains("not the one that was built"), "{log}");
}
