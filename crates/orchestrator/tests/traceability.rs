//! Traceability on real repositories with a model (C-55): a proposal's
//! names resolved in the base commit's model by identity; what a commit
//! changed against what was named, through the model and the code linked
//! to it; the cumulative change since the approved baseline (the tag on
//! `origin`, never a local one an agent could move) or, without one, since
//! the objective's start; and the purpose requirement, changed by identity,
//! failing its gate even when the objective names it. Needs git.

use agq_assistant::model_tools::{self, locked_changes};
use agq_orchestrator::gates;
use agq_orchestrator::record::Proposal;
use agq_orchestrator::traceability::{self, APPROVED_BASELINE};
use std::path::{Path, PathBuf};

fn git(folder: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(folder)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn repository(dir: &Path) -> PathBuf {
    let repository = dir.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    git(&repository, &["init", "-q", "-b", "main"]);
    git(&repository, &["config", "user.name", "Agentique test"]);
    git(
        &repository,
        &["config", "user.email", "test@example.invalid"],
    );
    repository
}

fn write(repository: &Path, path: &str, text: &str) {
    let path = repository.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Commits everything, the model's new elements given their identities
/// first (opening the project does), and returns the commit.
fn commit(repository: &Path, message: &str) -> String {
    let _ = model_tools::model_at(repository, "HEAD");
    git(repository, &["add", "-A"]);
    git(repository, &["commit", "-q", "-m", message]);
    git(repository, &["rev-parse", "HEAD"])
}

fn id_of(repository: &Path, commit: &str, name: &str) -> u64 {
    model_tools::model_at(repository, commit)
        .unwrap()
        .values()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("{name}"))
        .id
}

const SHOP: &str =
    "package Shop {\n    part def Store;\n    part def Cart;\n    requirement def Fast;\n}\n";

/// C-55: a change to a part the proposal does not name (its linked code)
/// is listed as changed but not named; a named requirement nothing touched
/// as named but not changed; what the named part owns is named with it.
#[test]
fn an_undeclared_change_is_listed_as_changed_but_not_named() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository(dir.path());
    write(&repository, "README.md", "A shop.\n");
    let empty = commit(&repository, "Empty");
    write(&repository, "model/Shop.sysml", SHOP);
    write(&repository, "src/store.rs", "fn store() {}\n");
    write(&repository, "src/cart.rs", "fn cart() {}\n");
    let first = commit(&repository, "Start");
    let links = format!(
        "{{\"format\": 1, \"repository\": \".\", \"links\": [{{\"element\": {}, \"name\": \"Shop::Store\", \"kind\": \"crate\", \"path\": \"src/store.rs\"}}, {{\"element\": {}, \"name\": \"Shop::Cart\", \"kind\": \"crate\", \"path\": \"src/cart.rs\"}}]}}",
        id_of(&repository, &first, "Shop::Store"),
        id_of(&repository, &first, "Shop::Cart")
    );
    write(&repository, "model/links.json", &links);
    let base = commit(&repository, "Links");
    // The proposal, resolved in the base commit's model.
    let model = traceability::base_model(&repository, &repository, &base)
        .unwrap()
        .expect("a model");
    // A commit without a model folder (by its tree) resolves nothing.
    assert!(!traceability::has_model(&repository, &empty).unwrap());
    assert!(traceability::has_model(&repository, &base).unwrap());
    assert!(
        traceability::base_model(&repository, &repository, &empty)
            .unwrap()
            .is_none()
    );
    let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let resolved = traceability::resolve(
        &names(&["Shop::Fast"]),
        &names(&["Shop::Store", "Shop::Fast"]),
        &model,
    )
    .unwrap();
    assert_eq!(
        resolved.serves[0].element,
        id_of(&repository, &base, "Shop::Fast")
    );
    let refused = traceability::resolve(&names(&["Shop::Store"]), &[], &model).unwrap_err();
    assert!(refused.contains("not a requirement"), "{refused}");
    let proposal = Proposal {
        parts: names(&["Shop::Store", "Shop::Fast"]),
        resolved: Some(resolved),
        ..Proposal::default()
    };
    // The commit: an attribute inside Store (named), and Cart's code (not).
    write(
        &repository,
        "model/Shop.sysml",
        &SHOP.replace(
            "part def Store;",
            "part def Store {\n        attribute open;\n    }",
        ),
    );
    write(&repository, "src/cart.rs", "fn cart() { checkout(); }\n");
    let change = commit(&repository, "Change");
    let patch = agq_execution::git::patch_of(&repository, &base, &change).unwrap();
    let traced = traceability::traced_in(
        &repository,
        Some(&repository),
        &base,
        &change,
        &patch,
        &proposal,
    );
    assert_eq!(
        traced.not_named,
        vec!["Shop::Cart (part def, its linked code changed)".to_string()],
        "{traced:?}"
    );
    assert_eq!(
        traced.not_changed,
        vec!["Shop::Fast (requirement def)".to_string()]
    );
    assert!(traced.skipped.is_none());
    assert_eq!(
        (traced.base.as_str(), traced.commit.as_str()),
        (base.as_str(), change.as_str())
    );
    assert!(
        traced
            .text()
            .contains("changed but not named in `parts`:\n  - Shop::Cart")
    );
    // Named as well, Cart's change is the proposal's.
    let both = Proposal {
        parts: names(&["Shop::Store", "Shop::Cart"]),
        ..Proposal::default()
    };
    let traced = traceability::traced_in(
        &repository,
        Some(&repository),
        &base,
        &change,
        &patch,
        &both,
    );
    assert!(
        traced.not_named.is_empty() && traced.not_changed.is_empty(),
        "{traced:?}"
    );
}

/// C-55: with the tag `approved-baseline` on `origin` two commits before the
/// cycle's base, the cumulative change counts what was added since the
/// tag, not only since the base, with the tests weakened over that range; a
/// local tag an agent moved is not read; without the tag on `origin`, it
/// compares with the objective's start and says so.
#[test]
fn the_cumulative_change_counts_since_the_approved_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository(dir.path());
    let origin = dir.path().join("origin.git");
    git(dir.path(), &["init", "-q", "--bare", "origin.git"]);
    git(
        &repository,
        &["remote", "add", "origin", &origin.display().to_string()],
    );
    write(
        &repository,
        "model/Shop.sysml",
        "package Shop {\n    part def Store;\n    requirement def Fast;\n}\n",
    );
    write(
        &repository,
        "tests/a.rs",
        "#[test]\nfn a() {\n    assert!(store_opens());\n}\n",
    );
    let approved = commit(&repository, "Approved");
    git(&repository, &["tag", APPROVED_BASELINE]);
    git(&repository, &["push", "-q", "origin", APPROVED_BASELINE]);
    write(
        &repository,
        "model/Shop.sysml",
        "package Shop {\n    part def Store;\n    part def Cart;\n    requirement def Fast;\n}\n",
    );
    commit(&repository, "Cart");
    write(
        &repository,
        "model/Shop.sysml",
        "package Shop {\n    part def Store;\n    part def Cart;\n    requirement def Fast;\n    dependency from Cart to Store;\n}\n",
    );
    write(&repository, "tests/a.rs", "#[test]\nfn a() {\n}\n");
    let start = commit(&repository, "Depend");
    write(
        &repository,
        "model/Shop.sysml",
        "package Shop {\n    part def Store;\n    part def Cart;\n    part def Basket;\n    requirement def Fast;\n    dependency from Cart to Store;\n}\n",
    );
    let reviewed = commit(&repository, "Basket");
    // The URL recorded when the objective was created.
    let recorded = origin.display().to_string();
    // An agent moved the local tag, and pointed `origin` elsewhere: neither
    // is what is read.
    git(&repository, &["tag", "-f", APPROVED_BASELINE, &reviewed]);
    let decoy = dir.path().join("decoy.git");
    git(dir.path(), &["init", "-q", "--bare", "decoy.git"]);
    git(
        &repository,
        &["remote", "set-url", "origin", &decoy.display().to_string()],
    );
    assert_eq!(
        traceability::approved_baseline(&repository, &recorded).unwrap(),
        Some(approved.clone())
    );
    let cumulative = traceability::cumulative_in(
        &repository,
        Some(&repository),
        Some(&recorded),
        &start,
        &reviewed,
    );
    assert!(cumulative.approved);
    assert_eq!(cumulative.since, approved);
    let group = |what: &str| {
        cumulative
            .groups
            .iter()
            .find(|g| g.what == what)
            .unwrap_or_else(|| panic!("{what}"))
            .clone()
    };
    assert_eq!(
        group("top-level part defs").added,
        vec!["Shop::Cart".to_string(), "Shop::Basket".to_string()]
    );
    assert_eq!(group("dependencies").added.len(), 1);
    assert!(group("requirements").added.is_empty());
    assert_eq!(cumulative.created, 3);
    assert!(
        cumulative
            .test_changes
            .iter()
            .any(|c| c.contains("tests/a.rs: removes or changes `assert!(store_opens());`")),
        "{:?}",
        cumulative.test_changes
    );
    let text = cumulative.text();
    assert!(
        text.contains(&format!(
            "since the approved baseline at {}",
            &approved[..8]
        )),
        "{text}"
    );
    assert!(
        cumulative.line().contains("top-level part defs +2"),
        "{}",
        cumulative.line()
    );
    // Without the tag on origin: the objective's start, said so.
    git(
        &repository,
        &[
            "push",
            "-q",
            &recorded,
            &format!(":refs/tags/{APPROVED_BASELINE}"),
        ],
    );
    assert_eq!(
        traceability::approved_baseline(&repository, &recorded).unwrap(),
        None
    );
    let fallback = traceability::cumulative_in(
        &repository,
        Some(&repository),
        Some(&recorded),
        &start,
        &reviewed,
    );
    assert!(!fallback.approved);
    assert_eq!(fallback.since, start);
    assert_eq!(
        fallback
            .groups
            .iter()
            .find(|g| g.what == "top-level part defs")
            .unwrap()
            .added,
        vec!["Shop::Basket".to_string()]
    );
    assert!(fallback.test_changes.is_empty());
    assert!(
        fallback
            .text()
            .starts_with("No approved baseline recorded; compared with the objective's start"),
        "{}",
        fallback.text()
    );
    // A remote that cannot be read: the start, saying why, not that no
    // baseline was recorded.
    let gone = dir.path().join("gone.git").display().to_string();
    let unread = traceability::cumulative_in(
        &repository,
        Some(&repository),
        Some(&gone),
        &start,
        &reviewed,
    );
    assert!(!unread.approved && unread.since == start);
    assert!(unread.unread.is_some());
    assert!(
        unread
            .text()
            .starts_with("The approved baseline could not be read"),
        "{}",
        unread.text()
    );
    // No remote recorded: the same, saying so.
    let none = traceability::cumulative_in(&repository, Some(&repository), None, &start, &reviewed);
    assert!(
        none.unread
            .as_deref()
            .unwrap_or_default()
            .contains("no remote")
    );
    // A baseline the local repository does not have: said so, never as
    // "not an ancestor".
    let elsewhere = dir.path().join("elsewhere");
    git(dir.path(), &["init", "-q", "-b", "main", "elsewhere"]);
    git(&elsewhere, &["config", "user.name", "Agentique test"]);
    git(
        &elsewhere,
        &["config", "user.email", "test@example.invalid"],
    );
    write(&elsewhere, "README.md", "elsewhere\n");
    git(&elsewhere, &["add", "-A"]);
    git(&elsewhere, &["commit", "-q", "-m", "Elsewhere"]);
    git(&elsewhere, &["tag", APPROVED_BASELINE]);
    git(
        &elsewhere,
        &[
            "push",
            "-q",
            &recorded,
            &format!("refs/tags/{APPROVED_BASELINE}"),
        ],
    );
    let missing = traceability::cumulative_in(
        &repository,
        Some(&repository),
        Some(&recorded),
        &start,
        &reviewed,
    );
    assert!(missing.approved);
    assert!(
        missing
            .notes
            .iter()
            .any(|n| n.contains("is not in the local repository")),
        "{:?}",
        missing.notes
    );
    assert!(!missing.notes.iter().any(|n| n.contains("not an ancestor")));
}

/// C-55: a change to the model's purpose requirement, found by identity
/// (its doc, inside it), fails the purpose gate even when the objective
/// names it and the locked-elements gate lets it through; a change beside
/// it does not.
#[test]
fn a_change_to_the_purpose_requirement_fails_even_when_named() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository(dir.path());
    let shop = "package Shop {\n    requirement def Purpose {\n        doc /* Shops sell. */\n    }\n    requirement purpose : Purpose;\n    part def Store;\n}\n";
    write(&repository, "model/Shop.sysml", shop);
    commit(&repository, "Start");
    // The Operator locks it.
    let identities = repository.join("model").join("agentique.json");
    let mut file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&identities).unwrap()).unwrap();
    let key = file["elements"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, locator)| *locator == "requirement def Shop::Purpose")
        .map(|(key, _)| key.clone())
        .unwrap();
    file["locks"] = serde_json::json!([key]);
    std::fs::write(&identities, serde_json::to_string_pretty(&file).unwrap()).unwrap();
    let base = commit(&repository, "Lock the purpose");
    write(
        &repository,
        "model/Shop.sysml",
        &shop.replace("Shops sell.", "Shops sell, and lend."),
    );
    let change = commit(&repository, "Redefine the purpose");
    let patch = agq_execution::git::patch_of(&repository, &base, &change).unwrap();
    let named = ["Shop::Purpose".to_string()];
    assert!(
        locked_changes(&repository, &base, &named)
            .unwrap()
            .is_empty(),
        "named, the locked-elements gate lets it through"
    );
    let changed = traceability::purpose_in(Some(&repository), &base, &change, &[]);
    let check = changed.as_ref().unwrap();
    assert!(check.governed);
    assert!(
        check.changes.iter().any(|c| c.contains("Shop::Purpose")),
        "{changed:?}"
    );
    let gate = gates::purpose(&patch, &changed);
    assert!(!gate.passed(), "{gate:?}");
    assert!(
        gate.detail.contains("purpose requirement"),
        "{}",
        gate.detail
    );
    // A change beside it passes.
    write(
        &repository,
        "model/Shop.sysml",
        &shop
            .replace("Shops sell.", "Shops sell, and lend.")
            .replace("part def Store;", "part def Store;\n    part def Cart;"),
    );
    let beside = commit(&repository, "Cart");
    let patch = agq_execution::git::patch_of(&repository, &change, &beside).unwrap();
    let changed = traceability::purpose_in(Some(&repository), &change, &beside, &[]);
    assert!(changed.as_ref().unwrap().changes.is_empty());
    assert!(gates::purpose(&patch, &changed).passed());
}

/// C-55: a base whose model cannot be read is an error that stops the
/// cycle, never a proposal resolved against nothing; only a base without a
/// model folder resolves nothing.
#[test]
fn a_model_that_cannot_be_read_is_an_error_not_a_skip() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository(dir.path());
    write(&repository, "model/Shop.sysml", SHOP);
    let readable = commit(&repository, "Start");
    write(
        &repository,
        "model/Shop.sysml",
        "<<<<<<< ours\npackage Shop;\n=======\npackage Store;\n>>>>>>> theirs\n",
    );
    git(&repository, &["add", "-A"]);
    git(&repository, &["commit", "-q", "-m", "A conflict"]);
    let broken = git(&repository, &["rev-parse", "HEAD"]);
    // Read through a clean checkout of the readable commit.
    let reader = dir.path().join("reader");
    git(
        &repository,
        &[
            "worktree",
            "add",
            "-q",
            &reader.display().to_string(),
            &readable,
        ],
    );
    assert!(
        traceability::base_model(&repository, &reader, &readable)
            .unwrap()
            .is_some()
    );
    let error = traceability::base_model(&repository, &reader, &broken).unwrap_err();
    assert!(error.contains("could not be read"), "{error}");
}

/// C-55: a project whose model declares no root purpose requirement (a
/// user's) is not governed: ROADMAP.md and its model change freely.
#[test]
fn a_project_without_a_purpose_is_not_governed() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository(dir.path());
    write(&repository, "model/Shop.sysml", SHOP);
    write(&repository, "ROADMAP.md", "Plans.\n");
    let base = commit(&repository, "Start");
    write(&repository, "ROADMAP.md", "Other plans.\n");
    write(
        &repository,
        "model/Shop.sysml",
        &SHOP.replace(
            "requirement def Fast;",
            "requirement def Fast;\n    requirement def Purpose;",
        ),
    );
    let change = commit(&repository, "Plans and a purpose");
    let patch = agq_execution::git::patch_of(&repository, &base, &change).unwrap();
    let check = traceability::purpose_in(Some(&repository), &base, &change, &[]);
    assert!(!check.as_ref().unwrap().governed);
    let gate = gates::purpose(&patch, &check);
    assert!(gate.passed(), "{gate:?}");
}

/// The W13.7 repair: the projects of a commit are the folders that hold a
/// model's `.sysml` files themselves, by the commit's tree (not a
/// checkout's files), and a project other than the repository's own
/// `model` has its model read by name, so a plan's names resolve in it.
#[test]
fn a_commits_projects_and_a_projects_model_are_read() {
    let dir = tempfile::tempdir().unwrap();
    let repository = repository(dir.path());
    write(&repository, "model/Shop.sysml", SHOP);
    write(
        &repository,
        "models/garden/Garden.sysml",
        "package Garden {\n    part def Bed;\n    requirement def Watered;\n}\n",
    );
    write(&repository, "models/notes/README.md", "No model here.\n");
    write(&repository, "Top.sysml", "package Top;\n");
    let first = commit(&repository, "Start");
    // A later file is not in the first commit's projects.
    write(&repository, "models/later/Later.sysml", "package Later;\n");
    assert_eq!(
        traceability::projects(&repository, &first).unwrap(),
        vec!["model".to_string(), "models/garden".to_string()]
    );
    let scratch = dir.path().join("scratch");
    let garden = traceability::project_model(&repository, "models/garden", &scratch).unwrap();
    let names: Vec<&str> = garden.values().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Garden::Watered"), "{names:?}");
    assert!(!scratch.exists(), "the scratch folder is removed");
    let unknown = traceability::unknown(
        &garden,
        &["Garden::Bed".to_string(), "Garden::Pond".to_string()],
    );
    assert_eq!(unknown, vec!["`Garden::Pond`".to_string()]);
    let own = traceability::project_model(&repository, "model", &scratch).unwrap();
    assert!(own.values().any(|e| e.name == "Shop::Fast"));
    assert!(traceability::project_model(&repository, "models/notes", &scratch).is_err());
}
