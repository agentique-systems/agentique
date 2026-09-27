//! History: a project's model folder on disk and its history in git
//! (REALIGNMENT §3.5, R-6; part `History` in `models/agentique/Agentique.sysml`).
//!
//! A project folder holds the model folder [`MODEL_FOLDER`]: SysML text files
//! plus one identity and lock file ([`IDENTITY_FILE`]). The project folder is
//! usually the project's code repository; a project folder outside any git
//! repository becomes one.
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

mod folder;
mod identity;
mod repo;

pub use identity::{FORMAT, IDENTITY_FILE, Identities};
pub use repo::{CommitId, CommitInfo};

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The model folder inside a project folder.
pub const MODEL_FOLDER: &str = "model";
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

/// One version of a model folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Relative path (with `/`, ending in `.sysml`) to SysML text.
    pub documents: BTreeMap<String, String>,
    pub identities: Identities,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
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
    #[error("model files were changed outside Agentique: {}", .0.join(", "))]
    ChangedOnDisk(Vec<String>),
    #[error("the model has changes that are not committed")]
    Uncommitted,
    #[error(
        "switching branch would overwrite changed files outside the model folder; \
         commit or stash them first"
    )]
    WouldOverwrite,
    #[error("the project is already open in another Agentique window")]
    Locked,
    #[error("there is no model folder at {}", .0.display())]
    NoModelFolder(PathBuf),
    #[error("{} already holds a model", .0.display())]
    Exists(PathBuf),
    #[error("the repository has no working folder")]
    Bare,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An open model folder and the git repository that holds its history.
pub struct History {
    repo: git2::Repository,
    /// The model folder relative to the repository's working folder, with `/`.
    prefix: String,
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
    pub fn open(project: impl AsRef<Path>) -> Result<(Self, Snapshot)> {
        let project = project.as_ref();
        let folder = project.join(MODEL_FOLDER);
        if !folder.is_dir() {
            return Err(Error::NoModelFolder(folder));
        }
        let dir = folder.canonicalize()?;
        let lock = lock(&dir)?;
        let repo = match git2::Repository::discover(&dir) {
            Ok(repo) => repo,
            Err(e) if e.code() == git2::ErrorCode::NotFound => git2::Repository::init_opts(
                project,
                git2::RepositoryInitOptions::new().initial_head("main"),
            )?,
            Err(e) => return Err(e.into()),
        };
        let workdir = repo.workdir().ok_or(Error::Bare)?.canonicalize()?;
        let relative = dir.strip_prefix(&workdir).map_err(|_| Error::Invalid {
            path: dir.display().to_string(),
            message: "not inside the repository's working folder".into(),
        })?;
        let prefix = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if !dir.join(GITIGNORE).exists() {
            fs::write(dir.join(GITIGNORE), GITIGNORE_TEXT)?;
        }
        let mut history = Self {
            repo,
            prefix,
            dir,
            seen: BTreeMap::new(),
            _lock: lock,
        };
        let snapshot = history.load()?;
        Ok((history, snapshot))
    }

    /// Creates the model folder of a new project and opens it. Refuses a
    /// folder that already holds model files.
    pub fn create(project: impl AsRef<Path>) -> Result<(Self, Snapshot)> {
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

/// Locks the model folder for this process. The operating system releases
/// the lock when the file is closed, also when the process dies.
fn lock(dir: &Path) -> Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(LOCK_FILE))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(fs::TryLockError::WouldBlock) => Err(Error::Locked),
        Err(fs::TryLockError::Error(e)) => Err(e.into()),
    }
}
