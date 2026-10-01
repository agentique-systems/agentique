//! The model folder in git: where the repository is, what a commit holds,
//! one writer at a time, edits made outside the app, and whole-repository
//! branch switching.
use agq_history::{Error, History, Identities, ModelFiles};
use std::fs;
use std::path::Path;

fn model(text: &str) -> ModelFiles {
    let mut identities = Identities::default();
    identities
        .elements
        .insert("1".into(), "package Shop".into());
    ModelFiles {
        documents: [("Shop.sysml".to_owned(), text.to_owned())].into(),
        identities,
        links: None,
    }
}

fn blob(repo: &git2::Repository, revision: &str, path: &str) -> Option<String> {
    let tree = repo
        .revparse_single(revision)
        .unwrap()
        .peel_to_tree()
        .unwrap();
    let entry = tree.get_path(Path::new(path)).ok()?;
    Some(String::from_utf8(repo.find_blob(entry.id()).unwrap().content().to_vec()).unwrap())
}

#[test]
fn a_project_folder_outside_any_repository_becomes_one() {
    let dir = tempfile::tempdir().unwrap();
    let (mut history, loaded) = History::create(dir.path()).unwrap();
    assert_eq!(loaded, ModelFiles::default());
    history.save(&model("package Shop;\n")).unwrap();
    let first = history.commit("Create Shop").unwrap().unwrap();
    assert_eq!(first.message, "Create Shop");

    let repo = git2::Repository::open(dir.path()).unwrap();
    assert_eq!(history.branch().unwrap(), "main");
    assert_eq!(history.branches().unwrap(), ["main"]);
    assert_eq!(
        blob(&repo, "HEAD", "model/Shop.sysml").as_deref(),
        Some("package Shop;\n")
    );
    // The ignore file is committed; the lock file is never.
    assert!(blob(&repo, "HEAD", "model/.gitignore").is_some());
    let status = |path: &str| repo.status_file(Path::new(path)).unwrap();
    assert_eq!(status("model/agentique.lock"), git2::Status::IGNORED);
    assert_eq!(status("model/agentique.json"), git2::Status::CURRENT);
    assert!(
        History::create(dir.path()).is_err(),
        "a model is already there"
    );
}

#[test]
fn a_commit_contains_only_the_model_folder() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/main.rs")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let me = git2::Signature::now("Operator", "operator@localhost").unwrap();
    repo.commit(Some("HEAD"), &me, &me, "Add main", &tree, &[])
        .unwrap();

    let (mut history, _) = History::create(dir.path()).unwrap();
    history.save(&model("package Shop;\n")).unwrap();
    history.commit("Add the shop model").unwrap().unwrap();
    // Unrelated code work in progress, as with the git command line: one
    // unstaged edit, one staged file.
    fs::write(dir.path().join("src/main.rs"), "fn main() { todo!() }\n").unwrap();
    fs::write(dir.path().join("src/lib.rs"), "\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/lib.rs")).unwrap();
    index.write().unwrap();

    for text in ["package Shop { part def A; }\n", "package Shop;\n"] {
        history.save(&model(text)).unwrap();
        history.commit("Change the shop model").unwrap().unwrap();
    }
    assert!(!history.has_uncommitted_changes().unwrap());
    assert!(history.commit("Nothing changed").unwrap().is_none());
    assert_eq!(
        blob(&repo, "HEAD", "src/main.rs").as_deref(),
        Some("fn main() {}\n")
    );
    assert!(blob(&repo, "HEAD", "src/lib.rs").is_none());
    assert!(blob(&repo, "HEAD", "model/agentique.json").is_some());
    let status = |path: &str| repo.status_file(Path::new(path)).unwrap();
    assert_eq!(status("src/main.rs"), git2::Status::WT_MODIFIED);
    assert_eq!(status("src/lib.rs"), git2::Status::INDEX_NEW);
    assert_eq!(status("model/Shop.sysml"), git2::Status::CURRENT);
    // The log lists only commits that changed the model folder.
    let log = history.log().unwrap();
    assert_eq!(log.len(), 3);
    assert_eq!(log[2].message, "Add the shop model");
}

#[test]
fn an_unrelated_repository_further_up_is_never_used() {
    let dir = tempfile::tempdir().unwrap();
    let home = git2::Repository::init(dir.path()).unwrap();
    let project = dir.path().join("projects/shop");
    let (mut history, _) = History::create(&project).unwrap();
    history.save(&model("package Shop;\n")).unwrap();
    history.commit("Create Shop").unwrap().unwrap();
    let own = git2::Repository::open(&project).unwrap();
    assert!(blob(&own, "HEAD", "model/Shop.sysml").is_some());
    assert!(home.head().is_err(), "nothing was committed further up");
}

#[test]
fn a_second_open_is_refused_until_the_first_is_closed() {
    let dir = tempfile::tempdir().unwrap();
    let (first, _) = History::create(dir.path()).unwrap();
    assert!(matches!(History::open(dir.path()), Err(Error::Locked)));
    drop(first);
    History::open(dir.path()).unwrap();
}

#[test]
fn an_edit_made_outside_the_app_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let (mut history, _) = History::create(dir.path()).unwrap();
    history.save(&model("package Shop;\n")).unwrap();
    let path = dir.path().join("model/Shop.sysml");
    fs::write(&path, "package Store;\n").unwrap();
    match history.save(&model("package Shop { part def A; }\n")) {
        Err(Error::ChangedOnDisk(paths)) => assert_eq!(paths, ["Shop.sysml"]),
        other => panic!("expected ChangedOnDisk, got {other:?}"),
    }
    assert_eq!(fs::read_to_string(&path).unwrap(), "package Store;\n");
    // Nor is the edit committed as if the app had saved it.
    assert!(matches!(
        history.commit("Checkpoint"),
        Err(Error::ChangedOnDisk(_))
    ));
    assert!(history.log().unwrap().is_empty());
    // After reading the folder again, saving works.
    history.load().unwrap();
    history.save(&model("package Shop;\n")).unwrap();
}

#[test]
fn switching_branch_switches_the_whole_repository() {
    let dir = tempfile::tempdir().unwrap();
    let (mut history, _) = History::create(dir.path()).unwrap();
    history.save(&model("package Shop;\n")).unwrap();
    history.commit("Create Shop").unwrap().unwrap();
    history.create_branch("idea").unwrap();

    // A model change that is not committed blocks the switch.
    history
        .save(&model("package Shop { part def A; }\n"))
        .unwrap();
    assert!(matches!(
        history.switch_branch("idea"),
        Err(Error::Uncommitted)
    ));
    history.commit("Add A").unwrap().unwrap();
    // Code that differs between the branches follows the branch.
    let code = dir.path().join("notes.txt");
    fs::write(&code, "main\n").unwrap();
    let repo = git2::Repository::open(dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("notes.txt")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let me = git2::Signature::now("Operator", "operator@localhost").unwrap();
    repo.commit(Some("HEAD"), &me, &me, "Add notes", &tree, &[&head])
        .unwrap();

    // A changed code file the switch would remove is not lost.
    fs::write(&code, "changed\n").unwrap();
    assert!(matches!(
        history.switch_branch("idea"),
        Err(Error::WouldOverwrite)
    ));
    assert_eq!(history.branch().unwrap(), "main");
    fs::write(&code, "main\n").unwrap();

    let idea = history.switch_branch("idea").unwrap();
    assert_eq!(idea, model("package Shop;\n"));
    assert!(!code.exists());
    assert_eq!(history.branch().unwrap(), "idea");
    assert_eq!(history.log().unwrap().len(), 1);
    let main = history.switch_branch("main").unwrap();
    assert_eq!(main, model("package Shop { part def A; }\n"));
    assert!(code.exists());
    assert_eq!(history.log().unwrap().len(), 2);
}
