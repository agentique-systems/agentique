//! Worktrees, patches and integration for implementation tasks, through an
//! embedded git (no git install needed). A task works in its own worktree on
//! its own branch, made from a base commit; the Operator's working tree and
//! uncommitted work are never touched until integration, which checks the
//! base again and refuses rather than overwrite.

use crate::Refusal;
use git2::{
    ApplyLocation, BranchType, Diff, DiffFormat, DiffOptions, Oid, Repository, Signature, Status,
    StatusOptions, WorktreeAddOptions,
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

/// Why integration was refused; nothing changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conflict {
    /// The repository's head moved since the task started.
    BaseMoved { base: String, head: String },
    /// Files the patch changes have uncommitted changes in the working tree.
    UncommittedWork(Vec<String>),
    /// Nothing to integrate.
    Empty,
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
        }
    }
}

/// Applies the worktree's patch to the repository's working tree and index
/// and commits it with `message`, if the head is still `base` and none of
/// the patched files has uncommitted changes. Returns the new commit.
pub fn integrate(
    repository: &Path,
    worktree: &Path,
    base: &str,
    message: &str,
) -> Result<Result<String, Conflict>, Refusal> {
    let current = head(repository)?;
    if current.commit != base {
        return Ok(Err(Conflict::BaseMoved {
            base: base.to_string(),
            head: current.commit,
        }));
    }
    let patch = patch(worktree, base)?;
    if patch.is_empty() {
        return Ok(Err(Conflict::Empty));
    }
    let dirty = changed_files(repository)?;
    let touched: Vec<String> = patch
        .files
        .iter()
        .map(|f| f.path.clone())
        .filter(|p| dirty.contains(p))
        .collect();
    if !touched.is_empty() {
        return Ok(Err(Conflict::UncommittedWork(touched)));
    }
    let source = open(worktree)?;
    let diff = diff_since(&source, base)?;
    let mut buffer = Vec::new();
    diff.print(DiffFormat::Patch, |_, _, line| {
        if matches!(line.origin(), '+' | '-' | ' ') {
            buffer.push(line.origin() as u8);
        }
        buffer.extend_from_slice(line.content());
        true
    })
    .map_err(io)?;
    let repo = open(repository)?;
    let portable = Diff::from_buffer(&buffer).map_err(io)?;
    repo.apply(&portable, ApplyLocation::Both, None)
        .map_err(io)?;
    let mut index = repo.index().map_err(io)?;
    let tree_id = index.write_tree().map_err(io)?;
    let tree = repo.find_tree(tree_id).map_err(io)?;
    let parent = repo.head().map_err(io)?.peel_to_commit().map_err(io)?;
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
    Ok(Ok(commit.to_string()))
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
