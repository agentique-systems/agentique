//! TEMPORARY stand-in for `agq_system_state::Project` (git-backed save,
//! identities, locks, checkpoints and branches), built on the History branch.
//! It has the same interface, so integrating it means deleting this file and
//! importing `agq_system_state::project::{ApplyError, Checkpoint, Project}`.
//! Until then it keeps the model as SysML text in `<folder>/model/*.sysml`,
//! saved after every change; checkpoints live in memory only, and locks and
//! element identities are not saved.
use agq_language::{Source, Tree, parse, print};
use agq_system_state::{Change, ChangeEvent, Rejection, SystemState};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
pub struct ProjectError(pub String);
impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl From<std::io::Error> for ProjectError {
    fn from(error: std::io::Error) -> Self {
        ProjectError(error.to_string())
    }
}

#[derive(Debug)]
pub enum ApplyError {
    Rejection(Rejection),
    Project(ProjectError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub id: String,
    pub message: String,
    /// Seconds since the Unix epoch.
    pub time: i64,
}

pub struct Project {
    folder: PathBuf,
    state: SystemState,
    unmatched: Vec<String>,
    checkpoints: Vec<(Checkpoint, Tree)>,
}

impl Project {
    pub fn create(folder: &Path, name: &str) -> Result<Project, ProjectError> {
        let model = folder.join("model");
        if model.exists() {
            return Err(ProjectError(format!(
                "{} already holds a project",
                folder.display()
            )));
        }
        std::fs::create_dir_all(&model)?;
        let file = format!("{}.sysml", file_name(name));
        std::fs::write(
            model.join(&file),
            format!("package {} {{\n}}\n", quoted(name)),
        )?;
        Self::open(folder)
    }

    pub fn open(folder: &Path) -> Result<Project, ProjectError> {
        let model = folder.join("model");
        let mut sources = Vec::new();
        let entries = std::fs::read_dir(&model).map_err(|error| {
            ProjectError(format!("{} is not a project: {error}", folder.display()))
        })?;
        for entry in entries {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "sysml") {
                let text = std::fs::read_to_string(&path)?;
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                sources.push(Source::new(format!("model/{name}"), text));
            }
        }
        sources.sort_by(|a, b| a.path.cmp(&b.path));
        let tree = parse(&sources);
        Ok(Project {
            folder: folder.to_path_buf(),
            state: SystemState::new(tree, BTreeSet::new()),
            unmatched: Vec::new(),
            checkpoints: Vec::new(),
        })
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn state(&self) -> &SystemState {
        &self.state
    }

    pub fn unmatched(&self) -> &[String] {
        &self.unmatched
    }

    pub fn apply(&mut self, change: Change) -> Result<ChangeEvent, ApplyError> {
        let event = self.state.apply(change).map_err(ApplyError::Rejection)?;
        self.save().map_err(ApplyError::Project)?;
        Ok(event)
    }

    pub fn undo(&mut self) -> Result<Option<ChangeEvent>, ProjectError> {
        let event = self.state.undo();
        self.save()?;
        Ok(event)
    }

    pub fn redo(&mut self) -> Result<Option<ChangeEvent>, ProjectError> {
        let event = self.state.redo();
        self.save()?;
        Ok(event)
    }

    pub fn checkpoint(&mut self, message: &str) -> Result<Checkpoint, ProjectError> {
        let checkpoint = Checkpoint {
            id: format!("{:08}", self.checkpoints.len() + 1),
            message: message.to_string(),
            time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        };
        self.checkpoints
            .push((checkpoint.clone(), self.state.tree().clone()));
        Ok(checkpoint)
    }

    /// Newest first.
    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>, ProjectError> {
        Ok(self
            .checkpoints
            .iter()
            .rev()
            .map(|(c, _)| c.clone())
            .collect())
    }

    pub fn tree_at(&self, id: &str) -> Result<Tree, ProjectError> {
        self.checkpoints
            .iter()
            .find(|(c, _)| c.id == id)
            .map(|(_, tree)| tree.clone())
            .ok_or_else(|| ProjectError(format!("no checkpoint {id}")))
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool, ProjectError> {
        Ok(self
            .checkpoints
            .last()
            .is_none_or(|(_, tree)| tree != self.state.tree()))
    }

    fn save(&self) -> Result<(), ProjectError> {
        for source in print(self.state.tree()) {
            let path = self.folder.join(&source.path);
            let temporary = path.with_extension("sysml.next");
            std::fs::write(&temporary, source.text)?;
            std::fs::rename(&temporary, &path)?;
        }
        Ok(())
    }
}

fn file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "Model".into()
    } else {
        cleaned
    }
}

fn quoted(name: &str) -> String {
    let plain = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if plain {
        name.to_string()
    } else {
        format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}
