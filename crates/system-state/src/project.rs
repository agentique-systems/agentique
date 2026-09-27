//! A project on disk: the System State saved as SysML text in git
//! (REALIGNMENT §3.5, R-6), through History (`agq-history`).
//!
//! A project folder holds `model/`, with one `.sysml` file per document and
//! `agentique.json`, which gives every element its identity (the number of
//! its [`ElementId`]) by a *locator*: the element's kind and its path of names
//! from the top, such as `part def Shop::Store`. An unnamed element is `#n`,
//! its position among its owner's unnamed members
//! (`connection Shop::System::#1`); a second member with the same name is
//! `name#2`. Locators are written again at every save, so renames and moves
//! made in the app keep identities.
//!
//! Every change is saved before [`Project::apply`] returns. A [`Checkpoint`]
//! is a git commit of the model folder.
//!
//! When the text was edited by hand, an element whose locator has no entry
//! gets a new identity on open and is listed in [`Project::unmatched`].
//! Elements are matched only by their exact locator, never by similarity.
use crate::{Change, ChangeEvent, Rejection, SystemState};
use agq_history::{CommitInfo, History, Identities, Snapshot};
use agq_language::{
    Element, ElementId, ElementKind, Parent, QualifiedName, Source, Tree, parse, print,
};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};

/// Ids from 2^48 up belong to the built-in library ([`ElementId::from_raw`]).
const LIBRARY_IDS: u64 = 1 << 48;

/// An open project: its folder, its git history and its live System State.
/// Only one `Project` (in any process) can have a folder open at a time.
pub struct Project {
    folder: PathBuf,
    history: History,
    state: SystemState,
    unmatched: Vec<String>,
}

/// A saved point in the project's history: a git commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    /// The commit id.
    pub id: String,
    pub message: String,
    /// Seconds since the Unix epoch.
    pub time: i64,
}

/// Why a project operation failed. The System State is unchanged.
#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    Git(String),
    /// A project file cannot be read: bad JSON, an unknown format, an
    /// invalid element id, or conflict markers from an unfinished git merge.
    Format(String),
    /// Another Agentique window has the project open.
    Locked,
    /// The model has changes since the last checkpoint.
    UncommittedChanges,
    /// There is nothing to checkpoint.
    NoChanges,
    /// Model files were changed outside Agentique while the project was open.
    ChangedOnDisk(Vec<String>),
    /// No model folder at this path.
    NotAProject(PathBuf),
    /// A model folder with model files is already at this path.
    AlreadyExists(PathBuf),
    /// A project name that cannot name the model file.
    InvalidName(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::Io(e) => write!(f, "could not read or write the project files: {e}"),
            ProjectError::Git(message) | ProjectError::Format(message) => f.write_str(message),
            ProjectError::Locked => {
                f.write_str("the project is already open in another Agentique window")
            }
            ProjectError::UncommittedChanges => f.write_str(
                "the model has changes since the last checkpoint; make a checkpoint first",
            ),
            ProjectError::NoChanges => {
                f.write_str("there are no changes since the last checkpoint")
            }
            ProjectError::ChangedOnDisk(paths) => write!(
                f,
                "model files were changed outside Agentique while the project was open ({}); \
                 open the project again to load them",
                paths.join(", ")
            ),
            ProjectError::NotAProject(path) => write!(
                f,
                "not an Agentique project: there is no model folder at {}",
                path.display()
            ),
            ProjectError::AlreadyExists(path) => {
                write!(f, "{} already holds a model", path.display())
            }
            ProjectError::InvalidName(name) => write!(
                f,
                "\"{name}\" cannot be a project name: it names the model file, so it must be \
                 a file name without / \\ : * ? \" < > | and without a leading or trailing \
                 dot or space"
            ),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<agq_history::Error> for ProjectError {
    fn from(error: agq_history::Error) -> Self {
        use agq_history::Error as E;
        match error {
            E::Io(e) => ProjectError::Io(e),
            E::Locked => ProjectError::Locked,
            E::Uncommitted => ProjectError::UncommittedChanges,
            E::ChangedOnDisk(paths) => ProjectError::ChangedOnDisk(paths),
            E::NoModelFolder(path) => ProjectError::NotAProject(path),
            E::Exists(path) => ProjectError::AlreadyExists(path),
            error @ (E::Invalid { .. } | E::UnknownFormat { .. } | E::ConflictMarkers { .. }) => {
                ProjectError::Format(error.to_string())
            }
            error => ProjectError::Git(error.to_string()),
        }
    }
}

/// Why [`Project::apply`] failed. The model is as it was.
#[derive(Debug)]
pub enum ApplyError {
    /// The change was rejected (R-18).
    Rejection(Rejection),
    /// The change could not be saved, so it was undone; redo tries again.
    Project(ProjectError),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::Rejection(rejection) => rejection.fmt(f),
            ApplyError::Project(error) => write!(f, "the change was not saved: {error}"),
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<Rejection> for ApplyError {
    fn from(rejection: Rejection) -> Self {
        ApplyError::Rejection(rejection)
    }
}

impl From<ProjectError> for ApplyError {
    fn from(error: ProjectError) -> Self {
        ApplyError::Project(error)
    }
}

impl Project {
    /// Creates a new project folder with `model/<Name>.sysml` holding
    /// `package <Name>;`, makes the folder a git repository unless it is
    /// inside one, and makes a first checkpoint.
    pub fn create(folder: &Path, name: &str) -> Result<Project, ProjectError> {
        check_project_name(name)?;
        let (history, _) = History::create(folder)?;
        let mut tree = Tree::new();
        let document = tree.add_document(&format!("{name}.sysml"));
        tree.add(
            Parent::Document(document),
            Element::named(ElementKind::Package, name),
        )
        .expect("the document was just added");
        let mut project = Project {
            folder: folder.to_path_buf(),
            history,
            state: SystemState::new(tree, BTreeSet::new()),
            unmatched: Vec::new(),
        };
        project.save()?;
        project.checkpoint(&format!("Create {name}"))?;
        Ok(project)
    }

    /// Opens a project: finishes or discards an interrupted save, parses the
    /// model files, gives elements their identities from `agentique.json`
    /// and restores locks. Elements without an identity entry get new ids,
    /// are listed in [`unmatched`](Self::unmatched), and their ids are saved
    /// at once.
    pub fn open(folder: &Path) -> Result<Project, ProjectError> {
        let (history, snapshot) = History::open(folder)?;
        let model = read(&snapshot, highest_on_branches(&history))?;
        let mut project = Project {
            folder: folder.to_path_buf(),
            history,
            state: SystemState::new(model.tree, model.locks),
            unmatched: model.unmatched,
        };
        project.keep_identities(snapshot)?;
        Ok(project)
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn state(&self) -> &SystemState {
        &self.state
    }

    /// Locators of the elements that had no identity entry when the model
    /// was last read (on open or branch switch), in document order.
    pub fn unmatched(&self) -> &[String] {
        &self.unmatched
    }

    /// Applies a change and saves the model before returning.
    pub fn apply(&mut self, change: Change) -> Result<ChangeEvent, ApplyError> {
        let event = self.state.apply(change)?;
        if let Err(error) = self.save() {
            self.state.undo();
            return Err(error.into());
        }
        Ok(event)
    }

    /// Reverts the most recent change and saves.
    pub fn undo(&mut self) -> Result<Option<ChangeEvent>, ProjectError> {
        let Some(event) = self.state.undo() else {
            return Ok(None);
        };
        if let Err(error) = self.save() {
            self.state.redo();
            return Err(error);
        }
        Ok(Some(event))
    }

    /// Reapplies the most recently undone change and saves.
    pub fn redo(&mut self) -> Result<Option<ChangeEvent>, ProjectError> {
        let Some(event) = self.state.redo() else {
            return Ok(None);
        };
        if let Err(error) = self.save() {
            self.state.undo();
            return Err(error);
        }
        Ok(Some(event))
    }

    /// Commits the saved model. Fails with [`ProjectError::NoChanges`] when
    /// nothing changed since the last checkpoint.
    pub fn checkpoint(&mut self, message: &str) -> Result<Checkpoint, ProjectError> {
        let commit = self.history.commit(message)?;
        commit.map(checkpoint).ok_or(ProjectError::NoChanges)
    }

    /// The checkpoints of the current branch, newest first.
    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>, ProjectError> {
        Ok(self.history.log()?.into_iter().map(checkpoint).collect())
    }

    /// The model at a checkpoint, with identities, for the "what changed"
    /// view: compare it with [`crate::compare`].
    pub fn tree_at(&self, checkpoint: &str) -> Result<Tree, ProjectError> {
        let snapshot = self.history.load_commit(checkpoint)?;
        // An element without an identity entry gets an id that no element of
        // the current model has, so it cannot pass for one of them.
        let highest = self
            .state
            .tree()
            .walk()
            .into_iter()
            .map(ElementId::raw)
            .max();
        Ok(read(&snapshot, highest.unwrap_or(0))?.tree)
    }

    pub fn branches(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self.history.branches()?)
    }

    pub fn current_branch(&self) -> Result<String, ProjectError> {
        self.history.branch()?.ok_or_else(|| {
            ProjectError::Git("the repository is not on a branch (detached HEAD)".into())
        })
    }

    /// Starts a branch at the last checkpoint, without switching to it.
    pub fn create_branch(&mut self, name: &str) -> Result<(), ProjectError> {
        Ok(self.history.create_branch(name)?)
    }

    /// Switches the repository to a branch and loads its model (a `Loaded`
    /// change event; undo and redo start again). Refuses with
    /// [`ProjectError::UncommittedChanges`] if the model has changes since
    /// the last checkpoint.
    pub fn switch_branch(&mut self, name: &str) -> Result<ChangeEvent, ProjectError> {
        let previous = self.history.branch()?;
        let snapshot = self.history.switch_branch(name)?;
        let model = match read(&snapshot, highest_on_branches(&self.history)) {
            Ok(model) => model,
            Err(error) => {
                if let Some(previous) = previous {
                    self.history.switch_branch(&previous)?;
                }
                return Err(error);
            }
        };
        self.unmatched = model.unmatched;
        let event = self
            .state
            .load(model.tree, model.locks, &format!("Switch to branch {name}"));
        self.keep_identities(snapshot)?;
        Ok(event)
    }

    /// Whether the model changed since the last checkpoint.
    pub fn has_uncommitted_changes(&self) -> Result<bool, ProjectError> {
        Ok(self.history.has_uncommitted_changes()?)
    }

    /// Saves the model: every document printed, and its identities and locks.
    fn save(&mut self) -> Result<(), ProjectError> {
        let tree = self.state.tree();
        let snapshot = Snapshot {
            documents: print(tree).into_iter().map(|s| (s.path, s.text)).collect(),
            identities: identities(tree, self.state.locks()),
        };
        Ok(self.history.save(&snapshot)?)
    }

    /// After reading `snapshot`: saves the identity file if it no longer
    /// matches (new ids, locks of elements that are gone), leaving the text
    /// as it was written.
    fn keep_identities(&mut self, snapshot: Snapshot) -> Result<(), ProjectError> {
        let identities = identities(self.state.tree(), self.state.locks());
        if identities != snapshot.identities {
            self.history.save(&Snapshot {
                documents: snapshot.documents,
                identities,
            })?;
        }
        Ok(())
    }
}

fn checkpoint(commit: CommitInfo) -> Checkpoint {
    Checkpoint {
        id: commit.id.to_string(),
        message: commit.message,
        time: commit.time,
    }
}

/// A model read from text, with identities attached.
struct Model {
    tree: Tree,
    locks: BTreeSet<ElementId>,
    unmatched: Vec<String>,
}

/// Parses a snapshot and gives each element the id stored for its locator.
/// New ids are above `floor` and above every stored id.
fn read(snapshot: &Snapshot, floor: u64) -> Result<Model, ProjectError> {
    let sources: Vec<Source> = snapshot
        .documents
        .iter()
        .map(|(path, text)| Source::new(path.as_str(), text.as_str()))
        .collect();
    let mut tree = parse(&sources);
    // Locator to stored id; `None` when two entries claim the same locator.
    let mut stored: HashMap<&str, Option<ElementId>> = HashMap::new();
    let mut highest = floor;
    for (id, locator) in &snapshot.identities.elements {
        let id = element_id(id)?;
        highest = highest.max(id.raw());
        stored
            .entry(locator.as_str())
            .and_modify(|entry| *entry = None)
            .or_insert(Some(id));
    }
    let mut ids = HashMap::new();
    let mut unmatched = Vec::new();
    for (element, locator) in locators(&tree) {
        match stored.get(locator.as_str()) {
            Some(Some(id)) => {
                ids.insert(element, *id);
            }
            _ => unmatched.push(locator),
        }
    }
    // Stored ids that no element takes stay retired: `rekey` hands out new
    // ids above every id in the map, also one whose key is not in the tree
    // (parsing never assigns 0).
    ids.insert(ElementId::from_raw(0), ElementId::from_raw(highest));
    tree.rekey(&ids).map_err(|_| {
        ProjectError::Format("agentique.json: an element id is reserved or used twice".into())
    })?;
    let locks = snapshot
        .identities
        .locks
        .iter()
        .map(|id| element_id(id))
        .collect::<Result<_, _>>()?;
    Ok(Model {
        tree,
        locks,
        unmatched,
    })
}

/// The highest element id at the tip of any branch. New elements get ids
/// above it, so that an element created on one branch never takes the id of
/// a different element on another. Unreadable branches are skipped.
fn highest_on_branches(history: &History) -> u64 {
    let branches = history.branches().unwrap_or_default();
    let tips = branches
        .iter()
        .filter_map(|branch| history.load_commit(&format!("refs/heads/{branch}")).ok());
    tips.flat_map(|tip| tip.identities.elements.into_keys())
        .filter_map(|id| element_id(&id).ok())
        .map(ElementId::raw)
        .max()
        .unwrap_or(0)
}

fn element_id(text: &str) -> Result<ElementId, ProjectError> {
    match text.parse::<u64>() {
        Ok(raw) if raw > 0 && raw < LIBRARY_IDS => Ok(ElementId::from_raw(raw)),
        _ => Err(ProjectError::Format(format!(
            "agentique.json: \"{text}\" is not an element id"
        ))),
    }
}

/// The identity file's entries for a model.
fn identities(tree: &Tree, locks: &BTreeSet<ElementId>) -> Identities {
    Identities {
        elements: locators(tree)
            .into_iter()
            .map(|(id, locator)| (id.raw().to_string(), locator))
            .collect(),
        locks: locks.iter().map(|id| id.raw().to_string()).collect(),
    }
}

/// Every element's locator, in document order. Unique within a tree: each
/// path segment is unique among its siblings (the top-level elements of all
/// documents are siblings).
fn locators(tree: &Tree) -> Vec<(ElementId, String)> {
    let mut out = Vec::with_capacity(tree.len());
    let roots: Vec<ElementId> = tree.roots().collect();
    add_locators(tree, &roots, "", &mut out);
    out
}

fn add_locators(
    tree: &Tree,
    members: &[ElementId],
    owner: &str,
    out: &mut Vec<(ElementId, String)>,
) {
    let mut named: HashMap<&str, usize> = HashMap::new();
    let mut unnamed = 0;
    for &id in members {
        let element = &tree[id];
        let segment = match &element.name {
            Some(name) => {
                let count = named.entry(name).or_default();
                *count += 1;
                // Quoted when needed, so `a#2` never clashes with a name.
                let name = QualifiedName::new([name.as_str()]).to_string();
                match *count {
                    1 => name,
                    n => format!("{name}#{n}"),
                }
            }
            None => {
                unnamed += 1;
                format!("#{unnamed}")
            }
        };
        let path = if owner.is_empty() {
            segment
        } else {
            format!("{owner}::{segment}")
        };
        out.push((id, format!("{} {path}", element.kind.keyword())));
        add_locators(tree, element.children(), &path, out);
    }
}

/// The name becomes the model file's name and the top package's name.
fn check_project_name(name: &str) -> Result<(), ProjectError> {
    let bad = name.is_empty()
        || name != name.trim()
        || name.starts_with('.')
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c));
    if bad {
        Err(ProjectError::InvalidName(name.to_string()))
    } else {
        Ok(())
    }
}
