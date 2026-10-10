//! History: a project's model folder on disk and its history in git
//! (ROADMAP §4.5, R-6; part `History` in `model/Agentique.sysml`).
//!
//! A project folder holds the model folder [`MODEL_FOLDER`]: SysML text files
//! plus one identity and lock file ([`IDENTITY_FILE`]). The project folder is
//! a git repository: usually the project's code repository. A project folder
//! that is not the working folder of a repository becomes one, even inside
//! another repository, so an unrelated repository further up (such as one
//! for the home folder) is never used.
//!
//! History never reads SysML. The System State hands it printed documents and
//! identity entries as strings; History saves them, commits them at
//! checkpoints, and reads back any version.
//!
//! - **Saving** ([`History::save`]) writes every changed file to a temporary
//!   file, makes a small save journal durable, and only then moves the files
//!   into place. A crash leaves the old or the new state, never a mix; the
//!   next open finishes or discards the interrupted save.
//! - **One writer.** [`History::open`] locks the model folder until the
//!   `History` is dropped or the process ends. A second open fails with
//!   [`Error::Locked`].
//! - **Commits** contain only the model folder. Other work in a code
//!   repository, staged or not, is left alone.
//! - **Branches** are the repository's branches. Switching refuses while the
//!   model has changes that are not committed.
#![forbid(unsafe_code)]

#[cfg(test)]
mod crash_tests;
mod folder;
mod identity;
mod repo;

pub use identity::{FORMAT, IDENTITY_FILE, Identities};
pub use repo::{Checkpoint, Revisions};

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The model folder inside a project folder.
pub const MODEL_FOLDER: &str = "model";
/// The implementation links and the harness binding of a project (C-50,
/// ROADMAP §4.5 item 7): optional, saved and committed with the model files.
/// History stores its text and never reads it.
pub const LINKS_FILE: &str = "links.json";
/// Held locked while a project is open; never committed.
const LOCK_FILE: &str = "agentique.lock";
/// Keeps files that exist only while saving or while the project is open out
/// of git. Committed with the model.
pub(crate) const GITIGNORE: &str = ".gitignore";
const GITIGNORE_TEXT: &str = "\
# Written by Agentique while saving or while the project is open.
*.tmp
agentique.pending
agentique.lock
";

/// The files of one version of a model folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelFiles {
    /// Relative path (with `/`, ending in `.sysml`) to SysML text.
    pub documents: BTreeMap<String, String>,
    pub identities: Identities,
    /// The text of [`LINKS_FILE`], when the project has one; `None` removes it.
    pub links: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read or write the project files: {0}")]
    Io(#[from] std::io::Error),
    #[error("git: {}", .0.message())]
    Git(#[from] git2::Error),
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
    #[error("{path} has format {found}; this version of Agentique reads format {FORMAT} only")]
    UnknownFormat { path: String, found: u64 },
    #[error(
        "{path} contains git conflict markers: a git merge was not finished. \
         Abort it (git merge --abort) and open the project again; \
         Agentique merges models element by element, not as text"
    )]
    ConflictMarkers { path: String },
    #[error(
        "model files were changed outside Agentique while the project was open ({}); \
         open the project again to load them",
        .0.join(", ")
    )]
    ChangedOnDisk(Vec<String>),
    #[error("the model has changes since the last checkpoint; make a checkpoint first")]
    Uncommitted,
    #[error(
        "switching branch would overwrite changed files outside the model folder; \
         commit or stash them first"
    )]
    WouldOverwrite,
    #[error("the project is already open in another Agentique window")]
    Locked,
    #[error("the repository is not on a branch (detached HEAD); switch to a branch with git")]
    Detached,
    #[error("not an Agentique project: there is no model folder at {}", .0.display())]
    NoModelFolder(PathBuf),
    #[error("{} already holds a model", .0.display())]
    Exists(PathBuf),
    #[error("the project folder is not the working folder of its git repository")]
    NotWorkingFolder,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An open model folder and the git repository that holds its history.
pub struct History {
    /// The repository whose working folder is the project folder.
    repo: git2::Repository,
    /// The model folder.
    dir: PathBuf,
    /// Model files as last loaded or saved. Save refuses to overwrite
    /// anything that differs from this, so edits made outside the app are
    /// never lost silently.
    seen: BTreeMap<String, String>,
    /// Held open and locked for as long as the folder is open.
    _lock: fs::File,
}

impl History {
    /// Opens the model folder of an existing project and reads it, first
    /// finishing or discarding a save that was interrupted.
    pub fn open(project: impl AsRef<Path>) -> Result<(Self, ModelFiles)> {
        let project = project.as_ref();
        let folder = project.join(MODEL_FOLDER);
        if !folder.is_dir() {
            return Err(Error::NoModelFolder(folder));
        }
        let dir = folder.canonicalize()?;
        let lock = lock(&dir)?;
        // Only a repository whose working folder is the project folder;
        // opening never searches the folders above.
        let repo = match git2::Repository::open(project) {
            Ok(repo) => repo,
            Err(e) if e.code() == git2::ErrorCode::NotFound => git2::Repository::init_opts(
                project,
                git2::RepositoryInitOptions::new().initial_head("main"),
            )?,
            Err(e) => return Err(e.into()),
        };
        if repo.workdir().map(Path::canonicalize).transpose()?
            != dir.parent().map(Path::to_path_buf)
        {
            return Err(Error::NotWorkingFolder);
        }
        if !dir.join(GITIGNORE).exists() {
            fs::write(dir.join(GITIGNORE), GITIGNORE_TEXT)?;
        }
        let mut history = Self {
            repo,
            dir,
            seen: BTreeMap::new(),
            _lock: lock,
        };
        let files = history.load()?;
        Ok((history, files))
    }

    /// Creates the model folder of a new project and opens it. Refuses a
    /// folder that already holds model files.
    pub fn create(project: impl AsRef<Path>) -> Result<(Self, ModelFiles)> {
        let folder = project.as_ref().join(MODEL_FOLDER);
        fs::create_dir_all(&folder)?;
        if !folder::read_model_files(&folder)?.is_empty() {
            return Err(Error::Exists(folder));
        }
        Self::open(project)
    }

    /// The model folder on disk.
    pub fn folder(&self) -> &Path {
        &self.dir
    }
}

/// How long a lock that looks held is waited for before the folder is
/// taken to be open elsewhere: on Linux a process another thread of this
/// process starts holds a copy of every open file, the lock file among
/// them, until it has started (close-on-exec closes it then), so a lock
/// released a moment ago can still look held.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_millis(500);

/// Locks the model folder for this process. The operating system releases
/// the lock when the file is closed, also when the process dies. A lock
/// still held after [`LOCK_WAIT`] is another holder's: [`Error::Locked`].
fn lock(dir: &Path) -> Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(LOCK_FILE))?;
    let deadline = std::time::Instant::now() + LOCK_WAIT;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(fs::TryLockError::WouldBlock) => return Err(Error::Locked),
            Err(fs::TryLockError::Error(e)) => return Err(e.into()),
        }
    }
}
