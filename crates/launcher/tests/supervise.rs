//! The supervising launcher with real processes (C-53, ROADMAP §4.15): a
//! stand-in Studio, compiled here with `rustc`, reports ready and then
//! closes, hands over to another build, or crashes, as marker files in its
//! folder say. The supervisor starts the build a handover names, falls back
//! to the last known good build when that one does not start or crashes at
//! once, and ends when the Studio closes.

use agq_launcher::{
    Entry, HANDOVER_EXIT, Manifest, Registry, SETTLED, STUDIO, State, Supervised, file_digest,
    supervise,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FAKE: &str = r#"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exe = std::path::PathBuf::from(&args[0]);
    let folder = exe.parent().unwrap();
    let root = folder.parent().unwrap();
    // Each start appends its arguments, so a test sees every start.
    let mut seen = std::fs::read_to_string(folder.join("args.txt")).unwrap_or_default();
    seen.push_str(&args[1..].join(" "));
    seen.push('\n');
    std::fs::write(folder.join("args.txt"), seen).unwrap();
    if folder.join("broken").exists() {
        eprintln!("the stand-in Studio fails on purpose");
        std::process::exit(3);
    }
    let at = args.iter().position(|a| a == "--ready-file").unwrap();
    std::fs::write(&args[at + 1], "ready").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    if let Ok(to) = std::fs::read_to_string(folder.join("handover-to")) {
        let _ = std::fs::remove_file(folder.join("handover-to"));
        let to = to.trim();
        let project = std::fs::read_to_string(folder.join("handover-project"))
            .map(|p| format!(",\"--project\",\"{}\"", p.trim()))
            .unwrap_or_default();
        std::fs::write(
            root.join("handover.json"),
            format!("{{\"build\":\"{to}\",\"args\":[\"--adopted\",\"{to}\"{project}]}}"),
        )
        .unwrap();
        std::process::exit(75);
    }
    if folder.join("crash").exists() {
        std::process::exit(1);
    }
    if folder.join("crash-late").exists() {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        std::process::exit(1);
    }
    if folder.join("handover-without-file").exists() {
        let _ = std::fs::remove_file(folder.join("handover-without-file"));
        std::process::exit(75);
    }
}
"#;

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

fn install(root: &Path, studio: &Path, id: &str) {
    let folder = root.join(id);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(studio, folder.join(STUDIO)).unwrap();
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

fn mark(root: &Path, id: &str, file: &str, text: &str) {
    std::fs::write(root.join(id).join(file), text).unwrap();
}

fn starts(root: &Path, id: &str) -> Vec<String> {
    std::fs::read_to_string(root.join(id).join("args.txt"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn set_current(root: &Path, id: &str) {
    let mut registry = Registry::load(root).unwrap();
    registry.current = Some(id.into());
    registry.last_known_good = Some(id.into());
    registry.save(root).unwrap();
}

const WITHIN: Duration = Duration::from_secs(30);

#[test]
fn a_handover_starts_the_named_build_which_becomes_the_last_known_good_one() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "a");
    install(&root, &studio, "b");
    set_current(&root, "a");
    mark(&root, "a", "handover-to", "b");
    let args = vec!["--project".to_string(), "C:\\agentique".to_string()];
    assert_eq!(
        supervise(&root, &args, WITHIN, SETTLED),
        Supervised::Closed { last: "b".into() }
    );
    let a = starts(&root, "a");
    assert_eq!(a.len(), 1);
    assert!(
        a[0].contains("--project C:\\agentique --supervised"),
        "{a:?}"
    );
    let b = starts(&root, "b");
    assert_eq!(b.len(), 1);
    assert!(b[0].contains("--supervised --adopted b"), "{b:?}");
    let registry = Registry::load(&root).unwrap();
    assert_eq!(registry.current.as_deref(), Some("b"));
    assert_eq!(registry.last_known_good.as_deref(), Some("b"));
    assert!(
        !root.join(agq_launcher::HANDOVER).exists(),
        "a handover is acted on once"
    );
    assert_eq!(HANDOVER_EXIT, 75);
}

#[test]
fn a_handover_to_a_build_that_does_not_start_returns_to_the_last_known_good_one() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "good");
    install(&root, &studio, "bad");
    mark(&root, "bad", "broken", "");
    set_current(&root, "good");
    mark(&root, "good", "handover-to", "bad");
    assert_eq!(
        supervise(&root, &[], WITHIN, SETTLED),
        Supervised::Closed {
            last: "good".into()
        }
    );
    let good = starts(&root, "good");
    assert_eq!(good.len(), 2, "{good:?}");
    // The fallback is not asked to check an adoption, and says what happened.
    assert!(!good[1].contains("--adopted"), "{good:?}");
    assert!(good[1].contains("--recovered-from bad"), "{good:?}");
    let registry = Registry::load(&root).unwrap();
    assert_eq!(registry.last_known_good.as_deref(), Some("good"));
    assert!(matches!(
        registry.entry("bad").unwrap().state,
        State::Failed { .. }
    ));
}

#[test]
fn a_build_that_crashes_soon_after_starting_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "good");
    install(&root, &studio, "new");
    set_current(&root, "good");
    mark(&root, "good", "handover-to", "new");
    mark(&root, "new", "crash", "");
    assert_eq!(
        supervise(&root, &[], WITHIN, SETTLED),
        Supervised::Closed {
            last: "good".into()
        }
    );
    let good = starts(&root, "good");
    assert_eq!(good.len(), 2);
    assert!(good[1].contains("--recovered-from new"), "{good:?}");
    let registry = Registry::load(&root).unwrap();
    // It had reported ready, so it was last known good for a moment; the
    // crash returns both marks to the build that works.
    assert_eq!(registry.current.as_deref(), Some("good"));
    assert!(matches!(
        registry.entry("new").unwrap().state,
        State::Failed { .. }
    ));
}

#[test]
fn a_handover_keeps_the_project_the_studio_had_open() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "a");
    install(&root, &studio, "b");
    set_current(&root, "a");
    // The Studio switched to project B before handing over: its handover
    // says so, and B (not the A supervising started with) opens.
    mark(&root, "a", "handover-to", "b");
    mark(&root, "a", "handover-project", "B");
    let args = vec!["--project".to_string(), "A".to_string()];
    assert_eq!(
        supervise(&root, &args, WITHIN, SETTLED),
        Supervised::Closed { last: "b".into() }
    );
    let b = starts(&root, "b");
    assert!(b[0].contains("--adopted b --project B"), "{b:?}");
    assert!(!b[0].contains("--project A"), "{b:?}");
    // A handover is written in this launcher's format, and one naming
    // something that is not a build is refused.
    let handover = agq_launcher::Handover::new("b", Vec::new());
    assert_eq!(handover.format, 1);
    handover.save(&root).unwrap();
    assert_eq!(agq_launcher::Handover::take(&root).unwrap(), handover);
    // A handover naming something that is not a build is refused.
    agq_launcher::Handover::new("..\\elsewhere", Vec::new())
        .save(&root)
        .unwrap();
    assert!(agq_launcher::Handover::take(&root).is_err());
    agq_launcher::Handover::new("unknown", Vec::new())
        .save(&root)
        .unwrap();
    assert!(agq_launcher::Handover::take(&root).is_err());
}

#[test]
fn a_build_that_crashes_again_after_settling_gives_way_to_the_build_before_it() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "good");
    install(&root, &studio, "new");
    set_current(&root, "good");
    mark(&root, "good", "handover-to", "new");
    mark(&root, "new", "crash-late", "");
    // Settled after one second: "new" crashes after it settled, starts once
    // more, crashes again, and "good" (known good before "new") takes over.
    assert_eq!(
        supervise(&root, &[], WITHIN, Duration::from_secs(1)),
        Supervised::Closed {
            last: "good".into()
        }
    );
    let new = starts(&root, "new");
    assert_eq!(new.len(), 2, "{new:?}");
    assert!(!new[1].contains("--recovered-from"), "{new:?}");
    let good = starts(&root, "good");
    assert_eq!(good.len(), 2, "{good:?}");
    assert!(good[1].contains("--recovered-from new"), "{good:?}");
    assert!(!good[1].contains("--adopted"), "{good:?}");
    let registry = Registry::load(&root).unwrap();
    assert_eq!(registry.last_known_good.as_deref(), Some("good"));
    assert!(matches!(
        registry.entry("new").unwrap().state,
        State::Failed { .. }
    ));
}

#[test]
fn an_exit_to_hand_over_without_a_handover_counts_as_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let Some(studio) = compile(dir.path()) else {
        eprintln!("rustc is not available: skipped");
        return;
    };
    let root = dir.path().join("builds");
    install(&root, &studio, "good");
    install(&root, &studio, "odd");
    set_current(&root, "good");
    mark(&root, "good", "handover-to", "odd");
    mark(&root, "odd", "handover-without-file", "");
    assert_eq!(
        supervise(&root, &[], WITHIN, SETTLED),
        Supervised::Closed {
            last: "good".into()
        }
    );
    let good = starts(&root, "good");
    assert!(good[1].contains("--recovered-from odd"), "{good:?}");
    let log = std::fs::read_to_string(root.join("launcher.log")).unwrap();
    assert!(
        log.contains("asked to hand over, but there is no handover"),
        "{log}"
    );
}
