//! Crash safety: an interrupted save leaves the old or the new state, never
//! a mix, and never touches committed history.
use agq_history::{Error, History, Identities, Snapshot};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn snapshot(documents: &[(&str, &str)], elements: &[(&str, &str)], locks: &[&str]) -> Snapshot {
    Snapshot {
        documents: documents
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
        identities: Identities {
            elements: elements
                .iter()
                .map(|(id, locator)| (id.to_string(), locator.to_string()))
                .collect(),
            locks: locks.iter().map(|id| id.to_string()).collect(),
        },
    }
}

const SHOP: &str = "package Shop {\n    part def Store;\n}\n";

fn old() -> Snapshot {
    snapshot(
        &[("Shop.sysml", SHOP), ("Stats.sysml", "package Stats;\n")],
        &[
            ("1", "package Shop"),
            ("2", "part def Shop::Store"),
            ("3", "package Stats"),
        ],
        &["2"],
    )
}

/// Changes a document, deletes one, adds one in a subfolder and changes
/// identities and locks.
fn new() -> Snapshot {
    snapshot(
        &[
            ("Shop.sysml", "package Shop {\n    part def LinkStore;\n}\n"),
            ("sub/Extra.sysml", "package Extra;\n"),
        ],
        &[
            ("1", "package Shop"),
            ("2", "part def Shop::LinkStore"),
            ("4", "package Extra"),
        ],
        &[],
    )
}

/// A project whose model folder holds `old()`, committed.
fn committed() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let (mut history, _) = History::create(dir.path()).unwrap();
    history.save(&old()).unwrap();
    history.commit("Add the shop").unwrap().unwrap();
    dir
}

/// The files Agentique writes in the model folder, without `.gitignore` and
/// the lock file.
fn listing(dir: &Path, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().unwrap().is_dir() {
            out.extend(listing(&entry.path(), &format!("{prefix}{name}/")));
        } else if name != ".gitignore" && name != "agentique.lock" {
            out.push(format!("{prefix}{name}"));
        }
    }
    out.sort();
    out
}

#[test]
fn an_interrupted_save_leaves_the_old_or_the_new_state() {
    let dir = committed();
    let (old, new) = (old(), new());
    let (mut saw_old, mut saw_new) = (0, 0);
    for steps in 0.. {
        let (mut history, _) = History::open(dir.path()).unwrap();
        history.save(&old).unwrap();
        let finished = history.save_steps(&new, steps).unwrap();
        drop(history); // the process dies here

        let (history, loaded) = History::open(dir.path()).unwrap();
        if loaded == old {
            saw_old += 1;
        } else {
            assert_eq!(loaded, new, "after {steps} steps: neither old nor new");
            saw_new += 1;
        }
        let expected: BTreeSet<String> = loaded
            .documents
            .keys()
            .cloned()
            .chain(["agentique.json".to_owned()])
            .collect();
        let expected: Vec<String> = expected.into_iter().collect();
        assert_eq!(listing(&dir.path().join("model"), ""), expected);
        // Committed history is untouched.
        assert_eq!(history.log().unwrap().len(), 1);
        assert_eq!(history.load_commit("HEAD").unwrap(), old);
        if finished {
            break;
        }
    }
    // Four steps write temporary files and the journal; the rest finish.
    assert_eq!((saw_old, saw_new), (4, 6));
}

#[test]
fn torn_temporary_files_are_discarded() {
    let dir = committed();
    let folder = dir.path().join("model");
    // A save died while writing its temporary files, before its journal.
    fs::write(
        folder.join("Shop.sysml.tmp"),
        &new().documents["Shop.sysml"][..20],
    )
    .unwrap();
    fs::write(folder.join("agentique.pending.tmp"), "{\"format\":1,\"wri").unwrap();

    let (_history, loaded) = History::open(dir.path()).unwrap();
    assert_eq!(loaded, old());
    assert_eq!(
        listing(&folder, ""),
        ["Shop.sysml", "Stats.sysml", "agentique.json"]
    );
}

#[test]
fn unknown_formats_are_refused_and_left_alone() {
    let dir = committed();
    let folder = dir.path().join("model");

    let newer = "{\n  \"format\": 2,\n  \"elements\": {},\n  \"locks\": [],\n  \"groups\": []\n}\n";
    fs::write(folder.join("agentique.json"), newer).unwrap();
    assert!(matches!(
        History::open(dir.path()),
        Err(Error::UnknownFormat { found: 2, .. })
    ));
    assert_eq!(
        fs::read_to_string(folder.join("agentique.json")).unwrap(),
        newer
    );

    // A save journal from a newer version is not applied either.
    fs::write(folder.join("agentique.json"), old().identities.to_text()).unwrap();
    let journal = r#"{"format":2,"write":["Shop.sysml"],"delete":[]}"#;
    fs::write(folder.join("agentique.pending"), journal).unwrap();
    fs::write(folder.join("Shop.sysml.tmp"), "package Other;\n").unwrap();
    assert!(matches!(
        History::open(dir.path()),
        Err(Error::UnknownFormat { found: 2, .. })
    ));
    assert_eq!(fs::read_to_string(folder.join("Shop.sysml")).unwrap(), SHOP);
}

#[test]
fn an_unfinished_git_merge_is_explained() {
    let dir = committed();
    let path = dir.path().join("model/agentique.json");
    let text = fs::read_to_string(&path).unwrap().replace(
        "    \"2\": \"part def Shop::Store\"",
        "<<<<<<< HEAD\n    \"2\": \"part def Shop::Store\"\n=======\n    \"2\": \"part def Shop::Warehouse\"\n>>>>>>> idea",
    );
    fs::write(&path, &text).unwrap();
    let error = History::open(dir.path()).err().unwrap();
    assert!(matches!(&error, Error::ConflictMarkers { path } if path == "agentique.json"));
    assert!(error.to_string().contains("git merge --abort"), "{error}");
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
}
