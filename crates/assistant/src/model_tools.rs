//! The Assistant's model tools on a working copy's own model (a task's or an
//! objective's cycle's worktree, C-50, C-53): the same tools and System
//! State operations as in the Conversation, on the worktree's project, so a
//! proposed model change stays proposed until it is integrated. A locked
//! element is refused unless the objective names it.
//!
//! [`locked_changes`] is §4.15's integration check on a worktree's commit:
//! the elements it changes that were locked then or are locked now, and
//! whether it changed the locks. [`model_at`] and [`compare_commits`] read a
//! project's model at commits as plain data by identity, for the
//! Orchestrator's traceability (C-55).

use crate::conversation::ToolResult;
use crate::tools::{Prepared, cap, prepare};
use crate::turn::ToolCall;
use agq_language::{ElementId, Tree};
use agq_system_state::{Actor, ApplyError, Project};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A working copy's model, opened when a tool first needs it.
pub struct WorkingModel {
    folder: PathBuf,
    project: Option<Project>,
    /// Locked elements (qualified names) changes may touch: those an
    /// objective names. Empty for a task (a worker cannot confirm).
    confirmed: Vec<String>,
}

impl WorkingModel {
    pub fn new(folder: impl Into<PathBuf>) -> WorkingModel {
        WorkingModel {
            folder: folder.into(),
            project: None,
            confirmed: Vec::new(),
        }
    }

    /// Lets changes touch the locked elements an objective names.
    pub fn confirming(mut self, names: Vec<String>) -> WorkingModel {
        self.confirmed = names;
        self
    }

    fn project(&mut self) -> Result<&mut Project, String> {
        if self.project.is_none() {
            let project = Project::open(&self.folder)
                .map_err(|e| format!("The working copy's model cannot be opened: {e}"))?;
            self.project = Some(project);
        }
        Ok(self.project.as_mut().expect("opened above"))
    }

    /// Carries out a model tool (read_model, find_elements, get_problems,
    /// apply_changes, inspect_behaviour, list_scenarios) on this model.
    pub fn execute(&mut self, call: &ToolCall) -> ToolResult {
        let confirmed = self.confirmed.clone();
        let project = match self.project() {
            Ok(project) => project,
            Err(error) => return ToolResult::error(error),
        };
        let library = agq_library::Library::default();
        match prepare(project.state(), &library, &call.name, &call.input) {
            Prepared::Answer(text) => ToolResult::answer(cap(text)),
            Prepared::Change(mut change) => {
                change.actor = Actor::Assistant;
                change.confirmed = confirmed
                    .iter()
                    .filter_map(|name| project.state().tree().find(name))
                    .collect();
                match project.apply(change) {
                    Ok(event) => ToolResult::applied(project.state(), &event),
                    Err(ApplyError::Rejection(rejection)) => {
                        ToolResult::rejected(project.state(), &rejection)
                    }
                    Err(error) => ToolResult::error(format!("Not applied: {error}")),
                }
            }
            Prepared::Invalid(message) => ToolResult::error(message),
            _ => ToolResult::error(format!("`{}` is not available here", call.name)),
        }
    }

    /// Closes the model (its files are saved with each change; this
    /// releases its lock), returning it if a tool read or changed it.
    pub fn close(&mut self) -> Option<Tree> {
        let project = self.project.take()?;
        Some(project.state().tree().clone())
    }
}

/// Whether `id` or one of its owners is in `locks` (a lock covers the part
/// and what it owns, R-11).
fn covered(tree: &Tree, id: ElementId, locks: &BTreeSet<ElementId>) -> bool {
    let mut next = Some(id);
    while let Some(id) = next {
        if locks.contains(&id) {
            return true;
        }
        next = tree.get(id).and_then(|element| element.owner());
    }
    false
}

/// The locked parts whose linked code a change touches (§4.15: a link's
/// path covering a changed file): each element locked when `base` was
/// committed or as the change leaves the model, by name, unless `allowed`
/// names it or an element that owns it. `links` are the links files to read
/// (the change's and the base's, so removing a link hides nothing).
pub fn locked_code(
    folder: &Path,
    base: &str,
    files: &[String],
    links: &[String],
    allowed: &[String],
) -> Result<Vec<String>, String> {
    let project = Project::open(folder).map_err(|e| e.to_string())?;
    let before = project.tree_at(base).map_err(|e| e.to_string())?;
    let locks_before = project.locks_at(base).map_err(|e| e.to_string())?;
    let after = project.state().tree();
    let locks_now = project.state().locks().clone();
    let allowed_ids: BTreeSet<ElementId> = allowed
        .iter()
        .filter_map(|name| after.find(name).or_else(|| before.find(name)))
        .collect();
    let mut found = Vec::new();
    for text in links {
        // Links that cannot be read hide nothing: the gate fails.
        let parsed = agq_implementation::links::Links::parse(text)
            .map_err(|e| format!("the implementation links cannot be read: {e}"))?;
        for link in &parsed.links {
            if !files.iter().any(|f| agq_execution::within(f, &link.path)) {
                continue;
            }
            let id = link.element();
            let now = after.contains(id) && covered(after, id, &locks_now);
            let then = before.contains(id) && covered(&before, id, &locks_before);
            let named = (after.contains(id) && covered(after, id, &allowed_ids))
                || (before.contains(id) && covered(&before, id, &allowed_ids));
            if (now || then) && !named {
                let name = if after.contains(id) {
                    after.qualified_name(id)
                } else {
                    before.qualified_name(id)
                };
                if !found.contains(&name) {
                    found.push(name);
                }
            }
        }
    }
    found.sort();
    Ok(found)
}

/// What a worktree's commit changes in locked parts of the model, compared
/// with `base` (§4.15): each changed element (by qualified name) covered by
/// a lock then or now, and "the locks" if the commit changed them. Names in
/// `allowed` (an objective's locked elements) and what they own are not
/// listed. The model is read from `folder` as committed.
pub fn locked_changes(
    folder: &Path,
    base: &str,
    allowed: &[String],
) -> Result<Vec<String>, String> {
    let project = Project::open(folder).map_err(|e| e.to_string())?;
    let before = project.tree_at(base).map_err(|e| e.to_string())?;
    let locks_before = project.locks_at(base).map_err(|e| e.to_string())?;
    let after = project.state().tree();
    let locks_now = project.state().locks().clone();
    let mut found = Vec::new();
    if locks_before != locks_now {
        found.push("the locks".to_string());
    }
    let allowed_ids: Vec<ElementId> = allowed
        .iter()
        .filter_map(|name| after.find(name).or_else(|| before.find(name)))
        .collect();
    let named = |tree: &Tree, id: ElementId| {
        let set: BTreeSet<ElementId> = allowed_ids.iter().copied().collect();
        covered(tree, id, &set)
    };
    let comparison = agq_system_state::compare(&before, after);
    let mut check = |tree: &Tree, id: ElementId| {
        let locked = covered(tree, id, &locks_before) || covered(tree, id, &locks_now);
        if locked && !named(tree, id) {
            let name = tree.qualified_name(id);
            if !found.contains(&name) {
                found.push(name);
            }
        }
    };
    for id in comparison.updated.iter().chain(&comparison.created) {
        check(after, *id);
    }
    for id in &comparison.deleted {
        check(&before, *id);
    }
    Ok(found)
}

/// One element of a project's model at a commit, as plain data by identity
/// (C-55): what checks outside the Assistant read of a model without the
/// language core's types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Described {
    pub id: u64,
    /// Its qualified name; an unnamed element is described, as
    /// `(dependency from A to B)`.
    pub name: String,
    /// Its kind's keyword: `package`, `part def`, `requirement def`,
    /// `requirement`, `dependency`, `doc`, …
    pub kind: String,
    /// The elements that own it, nearest first.
    pub owners: Vec<u64>,
    /// It, or an element that owns it, is locked.
    pub locked: bool,
}

/// A project's model at two commits and what differs between them by
/// identity (`agq_system_state::compare`): `created` and `deleted` list
/// every element, those inside a created or deleted one too.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compared {
    pub before: BTreeMap<u64, Described>,
    pub after: BTreeMap<u64, Described>,
    pub created: Vec<u64>,
    pub updated: Vec<u64>,
    pub deleted: Vec<u64>,
}

fn described(tree: &Tree, locks: &BTreeSet<ElementId>) -> BTreeMap<u64, Described> {
    tree.walk()
        .into_iter()
        .map(|id| {
            let mut owners = Vec::new();
            let mut next = tree.get(id).and_then(|e| e.owner());
            while let Some(owner) = next {
                owners.push(owner.raw());
                next = tree.get(owner).and_then(|e| e.owner());
            }
            let element = Described {
                id: id.raw(),
                name: tree.qualified_name(id),
                kind: tree[id].kind.keyword().to_string(),
                owners,
                locked: covered(tree, id, locks),
            };
            (id.raw(), element)
        })
        .collect()
}

/// The model of the repository checked out in `folder`, as committed in
/// `commit`, by identity. The project in `folder` is opened to read it, and
/// closed again.
pub fn model_at(folder: &Path, commit: &str) -> Result<BTreeMap<u64, Described>, String> {
    let project = Project::open(folder).map_err(|e| e.to_string())?;
    let tree = project.tree_at(commit).map_err(|e| e.to_string())?;
    let locks = project.locks_at(commit).map_err(|e| e.to_string())?;
    Ok(described(&tree, &locks))
}

/// The model of the repository checked out in `folder`, as committed in
/// `from` and in `to`, compared by identity.
pub fn compare_commits(folder: &Path, from: &str, to: &str) -> Result<Compared, String> {
    let project = Project::open(folder).map_err(|e| e.to_string())?;
    let before = project.tree_at(from).map_err(|e| e.to_string())?;
    let locks_before = project.locks_at(from).map_err(|e| e.to_string())?;
    let after = project.tree_at(to).map_err(|e| e.to_string())?;
    let locks_after = project.locks_at(to).map_err(|e| e.to_string())?;
    let comparison = agq_system_state::compare(&before, &after);
    let raw = |ids: &[ElementId]| ids.iter().map(|id| id.raw()).collect();
    Ok(Compared {
        before: described(&before, &locks_before),
        after: described(&after, &locks_after),
        created: raw(&comparison.created),
        updated: raw(&comparison.updated),
        deleted: raw(&comparison.deleted),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_system_state::{Change, Operation};

    #[test]
    fn changing_the_code_of_a_locked_part_is_found_through_the_links() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::create_dir_all(folder.join("model")).unwrap();
        std::fs::create_dir_all(folder.join("src")).unwrap();
        std::fs::write(
            folder.join("model/Shop.sysml"),
            "package Shop {\n    part def Store;\n    part def Cart;\n}\n",
        )
        .unwrap();
        std::fs::write(folder.join("src/store.rs"), "fn store() {}\n").unwrap();
        std::fs::write(folder.join("src/cart.rs"), "fn cart() {}\n").unwrap();
        let mut project = Project::open(folder).unwrap();
        let store = project.state().tree().find("Shop::Store").unwrap();
        let cart = project.state().tree().find("Shop::Cart").unwrap();
        project
            .apply(Change::new(
                Actor::Operator,
                "Lock Store",
                vec![Operation::Lock { element: store }],
            ))
            .unwrap();
        let links = format!(
            "{{\"format\": 1, \"repository\": \".\", \"links\": [{{\"element\": {}, \"name\": \"Shop::Store\", \"kind\": \"crate\", \"path\": \"src/store.rs\"}}, {{\"element\": {}, \"name\": \"Shop::Cart\", \"kind\": \"crate\", \"path\": \"src/cart.rs\"}}]}}",
            store.raw(),
            cart.raw()
        );
        drop(project);
        let base = agq_execution::git::init_and_commit(folder, "Start").unwrap();
        let files = |f: &[&str]| f.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let found = locked_code(
            folder,
            &base,
            &files(&["src/store.rs"]),
            std::slice::from_ref(&links),
            &[],
        )
        .unwrap();
        assert_eq!(found, vec!["Shop::Store".to_string()]);
        assert!(
            locked_code(
                folder,
                &base,
                &files(&["src/cart.rs"]),
                std::slice::from_ref(&links),
                &[]
            )
            .unwrap()
            .is_empty()
        );
        let named = locked_code(
            folder,
            &base,
            &files(&["src/store.rs"]),
            &[links],
            &["Shop::Store".into()],
        )
        .unwrap();
        assert!(named.is_empty(), "the objective names it");
        assert!(
            locked_code(
                folder,
                &base,
                &files(&["src/store.rs"]),
                &["not json".into()],
                &[]
            )
            .is_err()
        );
    }

    /// C-55: a model at commits as plain data by identity, and what differs
    /// between two commits, with owners and locks.
    #[test]
    fn a_model_is_read_and_compared_at_commits_by_identity() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::create_dir_all(folder.join("model")).unwrap();
        std::fs::write(
            folder.join("model/Shop.sysml"),
            "package Shop {\n    part def Store;\n    requirement def Fast;\n}\n",
        )
        .unwrap();
        let mut project = Project::open(folder).unwrap();
        let store = project.state().tree().find("Shop::Store").unwrap();
        project
            .apply(Change::new(
                Actor::Operator,
                "Lock Store",
                vec![Operation::Lock { element: store }],
            ))
            .unwrap();
        drop(project);
        let base = agq_execution::git::init_and_commit(folder, "Start").unwrap();
        let model = model_at(folder, &base).unwrap();
        let fast = model.values().find(|e| e.name == "Shop::Fast").unwrap();
        assert_eq!(fast.kind, "requirement def");
        let shop = model.values().find(|e| e.name == "Shop").unwrap();
        assert_eq!(fast.owners, vec![shop.id]);
        assert!(model[&store.raw()].locked && !fast.locked);
        // An attribute added to Store: created inside it, Store updated.
        std::fs::write(
            folder.join("model/Shop.sysml"),
            "package Shop {\n    part def Store {\n        attribute open;\n    }\n    requirement def Fast;\n}\n",
        )
        .unwrap();
        // Opened, the new element gets its identity.
        drop(Project::open(folder).unwrap());
        let commit = agq_execution::git::init_and_commit(folder, "Open").unwrap();
        let compared = compare_commits(folder, &base, &commit).unwrap();
        let names = |ids: &[u64]| -> Vec<String> {
            ids.iter()
                .map(|id| compared.after[id].name.clone())
                .collect()
        };
        assert_eq!(names(&compared.created), vec!["Shop::Store::open"]);
        assert!(names(&compared.updated).contains(&"Shop::Store".to_string()));
        assert!(compared.deleted.is_empty());
        assert_eq!(compared.after[&compared.created[0]].owners[0], store.raw());
        assert!(compared.after[&compared.created[0]].locked, "inside a lock");
    }

    /// Reading the model as committed, comparing commits and checking a
    /// change against a base only read: they take no editing lock and write
    /// nothing, so they work while the project is open for editing (the
    /// Studio, an agent's session) and never make an open fail.
    #[test]
    fn reading_versions_takes_no_editing_lock_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::create_dir_all(folder.join("model")).unwrap();
        std::fs::write(
            folder.join("model/Shop.sysml"),
            "package Shop {
    part def Store;
}
",
        )
        .unwrap();
        drop(Project::open(folder).unwrap());
        let base = agq_execution::git::init_and_commit(folder, "Start").unwrap();
        let editing = Project::open(folder).unwrap();
        let before = std::fs::read_to_string(folder.join("model/agentique.json")).unwrap();
        assert!(
            model_at(folder, &base)
                .unwrap()
                .values()
                .any(|e| e.name == "Shop::Store")
        );
        assert!(
            compare_commits(folder, &base, &base)
                .unwrap()
                .created
                .is_empty()
        );
        assert_eq!(
            locked_changes(folder, &base, &[]).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            locked_code(folder, &base, &[], &[], &[]).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            std::fs::read_to_string(folder.join("model/agentique.json")).unwrap(),
            before,
            "nothing written"
        );
        drop(editing);
    }
}
