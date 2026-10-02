//! Worktrees, patches and integration for implementation tasks, through an
//! embedded git (no git install needed). A task works in its own worktree on
//! its own branch, made from a base commit; the Operator's working tree and
//! uncommitted work are never touched until integration, which checks the
//! base again and refuses rather than overwrite.

use crate::Refusal;
use git2::{
    BranchType, Diff, DiffFormat, DiffOptions, Oid, Repository, Signature, Status, StatusOptions,
    WorktreeAddOptions,
};
use std::path::{Path, PathBuf};

fn open(path: &Path) -> Result<Repository, Refusal> {
    Repository::open(path).map_err(|e| Refusal::Io(format!("{}: {}", path.display(), e.message())))
}

fn io(error: git2::Error) -> Refusal {
    Refusal::Io(format!("git: {}", error.message()))
}

/// The current commit of a repository, and its branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    pub commit: String,
    pub branch: Option<String>,
}

pub fn head(repository: &Path) -> Result<Head, Refusal> {
    let repo = open(repository)?;
    let head = repo.head().map_err(io)?;
    let commit = head.peel_to_commit().map_err(io)?.id().to_string();
    let branch = head
        .shorthand()
        .ok()
        .map(str::to_string)
        .filter(|_| head.is_branch());
    Ok(Head { commit, branch })
}

/// Files with changes not committed (tracked and untracked, not ignored),
/// as `/` paths.
pub fn changed_files(repository: &Path) -> Result<Vec<String>, Refusal> {
    let repo = open(repository)?;
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let statuses = repo.statuses(Some(&mut options)).map_err(io)?;
    Ok(statuses
        .iter()
        .filter(|s| s.status() != Status::CURRENT && !s.status().contains(Status::IGNORED))
        .filter_map(|s| s.path().ok().map(str::to_string))
        .collect())
}

/// A digest of the working tree's state: the head commit plus the content
/// of every changed file, so a result can say which code it ran.
pub fn tree_digest(repository: &Path) -> Result<String, Refusal> {
    use std::fmt::Write;
    let head = head(repository)?;
    let mut text = head.commit.clone();
    for file in changed_files(repository)? {
        let content = std::fs::read(repository.join(&file)).unwrap_or_default();
        let _ = write!(text, "\n{file}\n{}", content.len());
        text.push_str(&String::from_utf8_lossy(&content));
    }
    Ok(format!("{:016x}", fnv(&text)))
}

fn fnv(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// A task's worktree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    /// The commit it was made from.
    pub base: String,
}

/// Makes a worktree for task `name` at `path`, on a new branch
/// `agentique/<name>` from the repository's head.
pub fn create_worktree(repository: &Path, name: &str, path: &Path) -> Result<Worktree, Refusal> {
    let repo = open(repository)?;
    let base = repo.head().map_err(io)?.peel_to_commit().map_err(io)?;
    let branch_name = format!("agentique/{name}");
    let branch = match repo.find_branch(&branch_name, BranchType::Local) {
        Ok(branch) => branch,
        Err(_) => repo.branch(&branch_name, &base, false).map_err(io)?,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Refusal::Io(e.to_string()))?;
    }
    let reference = branch.into_reference();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    repo.worktree(name, path, Some(&options)).map_err(io)?;
    Ok(Worktree {
        path: path.to_path_buf(),
        branch: branch_name,
        base: base.id().to_string(),
    })
}

/// Makes a worktree at `path` holding exactly `commit` (detached: no branch
/// is left behind), for building or trying that commit.
pub fn checkout_worktree(
    repository: &Path,
    name: &str,
    path: &Path,
    commit: &str,
) -> Result<(), Refusal> {
    let repo = open(repository)?;
    let target = repo
        .find_commit(Oid::from_str(commit).map_err(io)?)
        .map_err(io)?;
    // git2 adds a worktree on a branch; the branch is removed again once the
    // worktree is detached at the commit.
    let branch_name = format!("agentique/{name}");
    let branch = repo.branch(&branch_name, &target, true).map_err(io)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Refusal::Io(e.to_string()))?;
    }
    let reference = branch.into_reference();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    if let Err(error) = repo.worktree(name, path, Some(&options)) {
        drop(reference);
        if let Ok(mut branch) = repo.find_branch(&branch_name, BranchType::Local) {
            let _ = branch.delete();
        }
        return Err(io(error));
    }
    let worktree = open(path)?;
    worktree.set_head_detached(target.id()).map_err(io)?;
    drop(reference);
    if let Ok(mut branch) = repo.find_branch(&branch_name, BranchType::Local) {
        let _ = branch.delete();
    }
    Ok(())
}

/// The tree a commit holds, as an id.
pub fn tree_of(repository: &Path, commit: &str) -> Result<String, Refusal> {
    let repo = open(repository)?;
    let commit = repo
        .find_commit(Oid::from_str(commit).map_err(io)?)
        .map_err(io)?;
    Ok(commit.tree_id().to_string())
}

/// Removes a task's worktree (its branch is kept for history).
pub fn remove_worktree(repository: &Path, name: &str) -> Result<(), Refusal> {
    let repo = open(repository)?;
    if let Ok(worktree) = repo.find_worktree(name) {
        if let Some(path) = worktree.path().to_str().map(PathBuf::from) {
            let _ = std::fs::remove_dir_all(path);
        }
        let mut options = git2::WorktreePruneOptions::new();
        options.valid(true).working_tree(true);
        worktree.prune(Some(&mut options)).map_err(io)?;
    }
    Ok(())
}

/// One file of a patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    /// `added`, `modified`, `deleted` or `renamed`.
    pub status: String,
    pub added: usize,
    pub removed: usize,
    /// The unified diff of the file.
    pub diff: String,
}

/// What a worktree changed since `base`: every file, with its diff.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    pub files: Vec<FileChange>,
}

impl Patch {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn text(&self) -> String {
        self.files.iter().map(|f| f.diff.as_str()).collect()
    }
}

fn diff_since<'r>(repo: &'r Repository, base: &str) -> Result<Diff<'r>, Refusal> {
    let oid = Oid::from_str(base).map_err(io)?;
    let tree = repo.find_commit(oid).map_err(io)?.tree().map_err(io)?;
    let mut options = DiffOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut options))
        .map_err(io)
}

/// The patch a worktree holds since `base`.
pub fn patch(worktree: &Path, base: &str) -> Result<Patch, Refusal> {
    let repo = open(worktree)?;
    let diff = diff_since(&repo, base)?;
    patch_from(&diff)
}

fn patch_from(diff: &Diff<'_>) -> Result<Patch, Refusal> {
    let mut files: Vec<FileChange> = Vec::new();
    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if files.last().is_none_or(|f| f.path != path) {
            let status = match delta.status() {
                git2::Delta::Added | git2::Delta::Untracked => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Renamed => "renamed",
                _ => "modified",
            };
            files.push(FileChange {
                path: path.clone(),
                status: status.to_string(),
                added: 0,
                removed: 0,
                diff: String::new(),
            });
        }
        let file = files.last_mut().expect("pushed");
        let origin = line.origin();
        match origin {
            '+' => file.added += 1,
            '-' => file.removed += 1,
            _ => {}
        }
        if matches!(origin, '+' | '-' | ' ') {
            file.diff.push(origin);
        }
        file.diff.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(io)?;
    Ok(Patch { files })
}

/// Commits everything a task's worktree holds (changed, added and deleted
/// files that are not ignored) on its own branch: the **task commit** that
/// verification, review, builds and integration all refer to. Returns the
/// worktree's head unchanged when there is nothing new to commit.
pub fn commit_worktree(worktree: &Path, message: &str) -> Result<String, Refusal> {
    let repo = open(worktree)?;
    let mut index = repo.index().map_err(io)?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(io)?;
    index.update_all(["*"].iter(), None).map_err(io)?;
    index.write().map_err(io)?;
    let tree_id = index.write_tree().map_err(io)?;
    let parent = repo.head().map_err(io)?.peel_to_commit().map_err(io)?;
    if parent.tree_id() == tree_id {
        return Ok(parent.id().to_string());
    }
    let tree = repo.find_tree(tree_id).map_err(io)?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("Agentique", "agentique@localhost"))
        .map_err(io)?;
    let commit = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent],
        )
        .map_err(io)?;
    Ok(commit.to_string())
}

/// What `commit` changes since `base`, file by file.
pub fn patch_of(repository: &Path, base: &str, commit: &str) -> Result<Patch, Refusal> {
    let repo = open(repository)?;
    let tree = |id: &str| -> Result<git2::Tree<'_>, Refusal> {
        repo.find_commit(Oid::from_str(id).map_err(io)?)
            .map_err(io)?
            .tree()
            .map_err(io)
    };
    let diff = repo
        .diff_tree_to_tree(Some(&tree(base)?), Some(&tree(commit)?), None)
        .map_err(io)?;
    patch_from(&diff)
}

/// The files `commit` changes since `base`: new and old paths, `/`-separated.
fn changed_paths(repo: &Repository, base: Oid, commit: Oid) -> Result<Vec<String>, Refusal> {
    let tree = |id: Oid| repo.find_commit(id).and_then(|c| c.tree()).map_err(io);
    let diff = repo
        .diff_tree_to_tree(Some(&tree(base)?), Some(&tree(commit)?), None)
        .map_err(io)?;
    let mut paths = Vec::new();
    for delta in diff.deltas() {
        for file in [delta.old_file(), delta.new_file()] {
            if let Some(path) = file.path() {
                let path = path.to_string_lossy().replace('\\', "/");
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    Ok(paths)
}

/// Why integration was refused; nothing changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conflict {
    /// The repository's head moved since the task started.
    BaseMoved { base: String, head: String },
    /// Files the patch changes have uncommitted changes in the working tree.
    UncommittedWork(Vec<String>),
    /// Nothing to integrate.
    Empty,
    /// The patch changes paths no task may change.
    Protected(Vec<String>),
    /// The repository is not on a branch, or the task commit does not
    /// descend from the base.
    NotABranch(String),
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Conflict::BaseMoved { base, head } => write!(
                f,
                "the repository moved on since the task started (from {} to {}); review the task again before integrating",
                &base[..base.len().min(8)],
                &head[..head.len().min(8)]
            ),
            Conflict::UncommittedWork(files) => write!(
                f,
                "your working tree has changes that are not committed in {}; commit or put them aside first",
                files.join(", ")
            ),
            Conflict::Empty => f.write_str("the task changed nothing"),
            Conflict::Protected(paths) => write!(
                f,
                "the patch changes protected paths ({}); no task may change them",
                paths.join(", ")
            ),
            Conflict::NotABranch(why) => f.write_str(why),
        }
    }
}

/// Commits the worktree (the task commit) and integrates exactly that
/// commit: [`commit_worktree`] then [`integrate_commit`].
pub fn integrate(
    repository: &Path,
    worktree: &Path,
    base: &str,
    message: &str,
) -> Result<Result<String, Conflict>, Refusal> {
    let commit = commit_worktree(worktree, message)?;
    integrate_commit(repository, base, &commit, &[])
}

/// Brings exactly `commit`, a descendant of `base`, into the repository's
/// current branch: the branch must still be at `base`; only the files the
/// commit changes are written (working tree and index), each of which must
/// still hold the base's version; nothing else, staged or not, is touched;
/// paths inside `protected` are refused. Then the branch moves to `commit`
/// itself, so the integrated commit is the reviewed one.
///
/// Repeating it after an interruption finishes what was left: files that
/// already hold the commit's version count as written, and a branch already
/// at `commit` means it was done.
pub fn integrate_commit(
    repository: &Path,
    base: &str,
    commit: &str,
    protected: &[String],
) -> Result<Result<String, Conflict>, Refusal> {
    let repo = open(repository)?;
    let head_ref = repo.head().map_err(io)?;
    if !head_ref.is_branch() {
        return Ok(Err(Conflict::NotABranch(
            "the repository is not on a branch; check out a branch first".into(),
        )));
    }
    let branch = head_ref.name().map_err(io)?.to_string();
    let current = head_ref.peel_to_commit().map_err(io)?.id();
    let commit_id = Oid::from_str(commit).map_err(io)?;
    let base_id = Oid::from_str(base).map_err(io)?;
    if current == commit_id {
        return Ok(Ok(commit.to_string()));
    }
    if current != base_id {
        return Ok(Err(Conflict::BaseMoved {
            base: base.to_string(),
            head: current.to_string(),
        }));
    }
    if !repo.graph_descendant_of(commit_id, base_id).map_err(io)? {
        return Ok(Err(Conflict::NotABranch(
            "the task commit does not follow the base it started from".into(),
        )));
    }
    let paths = changed_paths(&repo, base_id, commit_id)?;
    if paths.is_empty() {
        return Ok(Err(Conflict::Empty));
    }
    let refused: Vec<String> = paths
        .iter()
        .filter(|path| protected.iter().any(|p| crate::within(path, p)))
        .cloned()
        .collect();
    if !refused.is_empty() {
        return Ok(Err(Conflict::Protected(refused)));
    }
    let target = repo.find_commit(commit_id).map_err(io)?;
    let target_tree = target.tree().map_err(io)?;
    // Each file must hold the base's version, or already the commit's.
    let mut dirty = Vec::new();
    for path in &paths {
        let at_base = match repo.status_file(Path::new(path)) {
            Ok(status) => status.is_empty(),
            // Nowhere: not in the base, the index or the working tree.
            Err(e) if e.code() == git2::ErrorCode::NotFound => true,
            Err(e) => return Err(io(e)),
        };
        if at_base || holds(&repo, &target_tree, path)? {
            continue;
        }
        dirty.push(path.clone());
    }
    if !dirty.is_empty() {
        return Ok(Err(Conflict::UncommittedWork(dirty)));
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    // Exactly these paths: never as patterns that also match the Operator's
    // other files (`app/[id]` would match `app/i`).
    checkout.force().disable_pathspec_match(true);
    for path in &paths {
        checkout.path(path);
    }
    repo.checkout_tree(target.as_object(), Some(&mut checkout))
        .map_err(io)?;
    repo.reference_matching(
        &branch,
        commit_id,
        true,
        base_id,
        "Agentique: integrate a reviewed task commit",
    )
    .map_err(io)?;
    Ok(Ok(commit.to_string()))
}

/// Whether the working tree and index hold `tree`'s version of `path`.
fn holds(repo: &Repository, tree: &git2::Tree<'_>, path: &str) -> Result<bool, Refusal> {
    let mut options = DiffOptions::new();
    options
        .pathspec(path)
        .disable_pathspec_match(true)
        .include_untracked(true);
    let diff = repo
        .diff_tree_to_workdir_with_index(Some(tree), Some(&mut options))
        .map_err(io)?;
    Ok(diff.deltas().len() == 0)
}

/// Initialises a repository with everything in `folder` committed (for new
/// implementation repositories and tests).
pub fn init_and_commit(folder: &Path, message: &str) -> Result<String, Refusal> {
    let repo = match Repository::open(folder) {
        Ok(repo) => repo,
        Err(_) => Repository::init_opts(
            folder,
            git2::RepositoryInitOptions::new().initial_head("main"),
        )
        .map_err(io)?,
    };
    let mut index = repo.index().map_err(io)?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(io)?;
    index.write().map_err(io)?;
    let tree = repo
        .find_tree(index.write_tree().map_err(io)?)
        .map_err(io)?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("Agentique", "agentique@localhost"))
        .map_err(io)?;
    let parents = match repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
        Some(parent) => vec![parent],
        None => Vec::new(),
    };
    let parents: Vec<&git2::Commit> = parents.iter().collect();
    let commit = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .map_err(io)?;
    Ok(commit.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n").unwrap();
        std::fs::write(repo.join("README.md"), "demo\n").unwrap();
        init_and_commit(&repo, "Start").unwrap();
        (dir, repo)
    }

    #[test]
    fn a_task_works_in_its_worktree_and_integrates_its_patch() {
        let (dir, repo) = repository();
        let base = head(&repo).unwrap().commit;
        let worktree =
            create_worktree(&repo, "task-1", &dir.path().join("worktrees/task-1")).unwrap();
        assert_eq!(worktree.base, base);
        std::fs::write(
            worktree.path.join("src/lib.rs"),
            "pub fn one() -> u32 { 1 }\npub fn two() -> u32 { 2 }\n",
        )
        .unwrap();
        std::fs::write(worktree.path.join("src/extra.rs"), "// new\n").unwrap();
        // The Operator's working tree is untouched.
        assert!(
            !std::fs::read_to_string(repo.join("src/lib.rs"))
                .unwrap()
                .contains("two")
        );
        let patch = patch(&worktree.path, &base).unwrap();
        let paths: Vec<&str> = patch.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["src/extra.rs", "src/lib.rs"]);
        assert_eq!(patch.files[1].added, 1);
        assert!(patch.files[1].diff.contains("+pub fn two()"));
        // Uncommitted work elsewhere does not block; in a patched file it does.
        std::fs::write(repo.join("README.md"), "demo, edited\n").unwrap();
        let commit = integrate(&repo, &worktree.path, &base, "Task 1")
            .unwrap()
            .unwrap();
        assert_eq!(head(&repo).unwrap().commit, commit);
        assert!(
            std::fs::read_to_string(repo.join("src/lib.rs"))
                .unwrap()
                .contains("two")
        );
        assert!(repo.join("src/extra.rs").exists());
        assert_eq!(
            std::fs::read_to_string(repo.join("README.md")).unwrap(),
            "demo, edited\n"
        );
        remove_worktree(&repo, "task-1").unwrap();
    }

    /// Regression (ROADMAP §5.6 item 3): integration committed the whole
    /// index, so a file the Operator had staged went into the task's commit.
    #[test]
    fn integration_leaves_the_operators_staged_work_alone() {
        let (dir, repo) = repository();
        let base = head(&repo).unwrap().commit;
        let worktree =
            create_worktree(&repo, "task-3", &dir.path().join("worktrees/task-3")).unwrap();
        std::fs::write(
            worktree.path.join("src/lib.rs"),
            "pub fn one() -> u32 { 3 }
",
        )
        .unwrap();
        // The Operator stages a change of their own.
        std::fs::write(
            repo.join("README.md"),
            "demo, staged
",
        )
        .unwrap();
        let operator = Repository::open(&repo).unwrap();
        let mut index = operator.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let commit = integrate(&repo, &worktree.path, &base, "Task 3")
            .unwrap()
            .unwrap();
        let committed = operator
            .find_commit(Oid::from_str(&commit).unwrap())
            .unwrap()
            .tree()
            .unwrap();
        let readme = committed.get_path(Path::new("README.md")).unwrap();
        let blob = operator.find_blob(readme.id()).unwrap();
        assert_eq!(
            blob.content(),
            b"demo
",
            "the staged README went into the task's commit"
        );
        // Still staged, still the Operator's.
        assert_eq!(
            std::fs::read_to_string(repo.join("README.md")).unwrap(),
            "demo, staged
"
        );
        assert!(
            changed_files(&repo)
                .unwrap()
                .contains(&"README.md".to_string())
        );
    }

    /// The task commit is what is integrated: added, changed and deleted
    /// files arrive, protected paths are refused, a repeat does nothing, and
    /// an integration interrupted after writing the files is finished by
    /// running it again (the journal's recovery).
    #[test]
    fn exactly_the_task_commit_is_integrated_and_a_repeat_finishes_or_does_nothing() {
        let (dir, repo) = repository();
        std::fs::write(repo.join("old.txt"), "going\n").unwrap();
        init_and_commit(&repo, "With a file to delete").unwrap();
        let base = head(&repo).unwrap().commit;
        let worktree =
            create_worktree(&repo, "task-4", &dir.path().join("worktrees/task-4")).unwrap();
        std::fs::write(
            worktree.path.join("src/lib.rs"),
            "pub fn one() -> u32 { 4 }\n",
        )
        .unwrap();
        std::fs::write(worktree.path.join("src/new.rs"), "// new\n").unwrap();
        std::fs::remove_file(worktree.path.join("old.txt")).unwrap();
        let commit = commit_worktree(&worktree.path, "Task 4").unwrap();
        assert_ne!(commit, base);
        // Committing again with nothing new returns the same commit.
        assert_eq!(commit_worktree(&worktree.path, "Task 4").unwrap(), commit);
        let patch = patch_of(&repo, &base, &commit).unwrap();
        let paths: Vec<&str> = patch.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["old.txt", "src/lib.rs", "src/new.rs"]);
        // A protected path in the patch is refused, whatever its case.
        let refused = integrate_commit(&repo, &base, &commit, &["SRC/new.rs".into()])
            .unwrap()
            .unwrap_err();
        assert_eq!(refused, Conflict::Protected(vec!["src/new.rs".into()]));
        assert_eq!(head(&repo).unwrap().commit, base);
        // An interruption after the files were written, before the branch
        // moved: the files hold the commit's version.
        let operator = Repository::open(&repo).unwrap();
        let target = operator
            .find_commit(Oid::from_str(&commit).unwrap())
            .unwrap();
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force().path("src/lib.rs");
        operator
            .checkout_tree(target.as_object(), Some(&mut checkout))
            .unwrap();
        let integrated = integrate_commit(&repo, &base, &commit, &[])
            .unwrap()
            .unwrap();
        assert_eq!(integrated, commit);
        assert_eq!(head(&repo).unwrap().commit, commit);
        assert!(!repo.join("old.txt").exists());
        assert!(repo.join("src/new.rs").exists());
        assert!(
            changed_files(&repo).unwrap().is_empty(),
            "{:?}",
            changed_files(&repo)
        );
        // Again: already done, nothing happens.
        assert_eq!(
            integrate_commit(&repo, &base, &commit, &[]).unwrap(),
            Ok(commit.clone())
        );
    }

    /// A path with pattern characters is written as exactly that path: the
    /// Operator's files it would match as a pattern are left alone.
    #[test]
    fn integration_writes_exactly_the_tasks_paths_never_as_patterns() {
        let (dir, repo) = repository();
        std::fs::create_dir_all(repo.join("app/i")).unwrap();
        std::fs::write(repo.join("app/i/page.tsx"), "base\n").unwrap();
        init_and_commit(&repo, "An app").unwrap();
        let worktree = create_worktree(&repo, "task-p", &dir.path().join("wt")).unwrap();
        std::fs::create_dir_all(worktree.path.join("app/[id]")).unwrap();
        std::fs::write(worktree.path.join("app/[id]/page.tsx"), "new\n").unwrap();
        let commit = commit_worktree(&worktree.path, "A dynamic page").unwrap();
        // The Operator's own edit, uncommitted.
        std::fs::write(repo.join("app/i/page.tsx"), "the Operator's\n").unwrap();
        integrate_commit(&repo, &worktree.base, &commit, &[])
            .unwrap()
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(repo.join("app/i/page.tsx")).unwrap(),
            "the Operator's\n"
        );
        assert!(repo.join("app/[id]/page.tsx").is_file());
    }

    /// A worktree that cannot be made leaves no branch behind.
    #[test]
    fn a_worktree_that_cannot_be_made_leaves_no_branch() {
        let (dir, repo) = repository();
        let first = head(&repo).unwrap().commit;
        let path = dir.path().join("try/a");
        checkout_worktree(&repo, "try-a", &path, &first).unwrap();
        assert!(checkout_worktree(&repo, "try-a", &dir.path().join("try/b"), &first).is_err());
        let branches = Repository::open(&repo)
            .unwrap()
            .branches(Some(BranchType::Local))
            .unwrap()
            .count();
        assert_eq!(branches, 1, "only the repository's own branch");
    }

    /// A build's worktree holds exactly its commit, and leaves no branch.
    #[test]
    fn a_worktree_at_a_commit_holds_exactly_that_commit() {
        let (dir, repo) = repository();
        let first = head(&repo).unwrap().commit;
        std::fs::write(
            repo.join("src/lib.rs"),
            "pub fn one() -> u32 { 2 }
",
        )
        .unwrap();
        init_and_commit(&repo, "Second").unwrap();
        let path = dir.path().join("builds/src/b1");
        checkout_worktree(&repo, "build-b1", &path, &first).unwrap();
        // (With `core.autocrlf`, git writes Windows line endings on checkout.)
        assert_eq!(
            std::fs::read_to_string(path.join("src/lib.rs"))
                .unwrap()
                .replace("\r\n", "\n"),
            "pub fn one() -> u32 { 1 }\n"
        );
        assert_eq!(head(&path).unwrap().commit, first);
        assert!(head(&path).unwrap().branch.is_none(), "detached");
        let branches = Repository::open(&repo)
            .unwrap()
            .branches(Some(BranchType::Local))
            .unwrap()
            .count();
        assert_eq!(branches, 1, "no branch is left behind");
        assert_eq!(tree_of(&repo, &first).unwrap().len(), 40);
        remove_worktree(&repo, "build-b1").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn integration_refuses_a_moved_base_and_uncommitted_work_in_patched_files() {
        let (dir, repo) = repository();
        let base = head(&repo).unwrap().commit;
        let worktree =
            create_worktree(&repo, "task-2", &dir.path().join("worktrees/task-2")).unwrap();
        std::fs::write(
            worktree.path.join("src/lib.rs"),
            "pub fn one() -> u32 { 11 }\n",
        )
        .unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn one() -> u32 { 100 }\n").unwrap();
        let refused = integrate(&repo, &worktree.path, &base, "Task 2")
            .unwrap()
            .unwrap_err();
        assert_eq!(
            refused,
            Conflict::UncommittedWork(vec!["src/lib.rs".into()])
        );
        assert_eq!(
            std::fs::read_to_string(repo.join("src/lib.rs")).unwrap(),
            "pub fn one() -> u32 { 100 }\n"
        );
        init_and_commit(&repo, "The Operator's own change").unwrap();
        let refused = integrate(&repo, &worktree.path, &base, "Task 2")
            .unwrap()
            .unwrap_err();
        assert!(matches!(refused, Conflict::BaseMoved { .. }));
        assert!(refused.to_string().contains("moved on"));
    }
}
