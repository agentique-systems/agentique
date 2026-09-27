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
//! made in the app keep identities. `agentique.json` also holds the next id,
//! so an id is never handed out twice.
//!
//! Every change is saved before [`Project::apply`] returns. Only documents
//! whose content changed are written; the others keep their text as written.
//! A [`Checkpoint`] is a git commit of the model folder.
//!
//! When the text was edited by hand, an element whose locator has no entry
//! gets a new identity on open. Such elements, and entries that no longer
//! match an element, are listed in [`Project::unmatched`]. Elements are
//! matched only by their exact locator, never by similarity; an unnamed
//! element is matched by its position, so inserting one by hand before
//! another shifts the positions and both are reported.
use crate::{Change, ChangeEvent, Rejection, SystemState};
pub use agq_history::{Checkpoint, Error as HistoryError};
use agq_history::{History, Identities, ModelFiles};
use agq_language::{
    Element, ElementId, ElementKind, Parent, QualifiedName, Source, Tree, parse, print,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    /// Each document's text as last read or saved.
    saved: BTreeMap<String, String>,
    /// Each document as printed from the model when it was last read or
    /// saved. A document whose print is unchanged keeps its saved text.
    printed: BTreeMap<String, String>,
}

/// Why a project operation failed. The System State is unchanged.
#[derive(Debug)]
pub enum ProjectError {
    /// Reading or saving the model folder, or git, failed; the History error
    /// says what happened and what to do.
    History(HistoryError),
    /// `agentique.json` holds something that is not an element id.
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
        // Cases that come from History use its messages.
        match self {
            ProjectError::History(error) => error.fmt(f),
            ProjectError::Locked => HistoryError::Locked.fmt(f),
            ProjectError::UncommittedChanges => HistoryError::Uncommitted.fmt(f),
            ProjectError::ChangedOnDisk(paths) => HistoryError::ChangedOnDisk(paths.clone()).fmt(f),
            ProjectError::NotAProject(path) => HistoryError::NoModelFolder(path.clone()).fmt(f),
            ProjectError::AlreadyExists(path) => HistoryError::Exists(path.clone()).fmt(f),
            ProjectError::Format(message) => f.write_str(message),
            ProjectError::NoChanges => {
                f.write_str("there are no changes since the last checkpoint")
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
            ProjectError::History(error) => Some(error),
            _ => None,
        }
    }
}

impl From<HistoryError> for ProjectError {
    fn from(error: HistoryError) -> Self {
        match error {
            HistoryError::Locked => ProjectError::Locked,
            HistoryError::Uncommitted => ProjectError::UncommittedChanges,
            HistoryError::ChangedOnDisk(paths) => ProjectError::ChangedOnDisk(paths),
            HistoryError::NoModelFolder(path) => ProjectError::NotAProject(path),
            HistoryError::Exists(path) => ProjectError::AlreadyExists(path),
            error => ProjectError::History(error),
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
    /// `package <Name>;`, makes the folder a git repository unless it is the
    /// working folder of one, and makes a first checkpoint.
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
            saved: BTreeMap::new(),
            printed: BTreeMap::new(),
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
        let (history, files) = History::open(folder)?;
        let model = read(&files, next_on_branches(&history))?;
        let mut project = Project {
            folder: folder.to_path_buf(),
            history,
            state: SystemState::new(model.tree, model.locks),
            unmatched: model.unmatched,
            saved: BTreeMap::new(),
            printed: BTreeMap::new(),
        };
        project.adopt(files)?;
        Ok(project)
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn state(&self) -> &SystemState {
        &self.state
    }

    /// What did not match when the model was last read (on open or branch
    /// switch): locators of elements that got a new id, in document order,
    /// then locators of identity entries that no element took.
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
    /// nothing changed since the last checkpoint, and with
    /// [`ProjectError::ChangedOnDisk`] when a model file was edited outside
    /// the app.
    pub fn checkpoint(&mut self, message: &str) -> Result<Checkpoint, ProjectError> {
        self.history.commit(message)?.ok_or(ProjectError::NoChanges)
    }

    /// The checkpoints of the current branch, newest first.
    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>, ProjectError> {
        Ok(self.history.log()?)
    }

    /// The model at a checkpoint, with identities, for the "what changed"
    /// view: compare it with [`crate::compare`].
    pub fn tree_at(&self, checkpoint: &str) -> Result<Tree, ProjectError> {
        let files = self.history.load_commit(checkpoint)?;
        // An element without an identity entry gets an id that no element of
        // the current model has, so it cannot pass for one of them.
        Ok(read(&files, self.state.tree().next_id().raw())?.tree)
    }

    pub fn branches(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self.history.branches()?)
    }

    pub fn current_branch(&self) -> Result<String, ProjectError> {
        Ok(self.history.branch()?)
    }

    /// Starts a branch at the last checkpoint, without switching to it.
    pub fn create_branch(&mut self, name: &str) -> Result<(), ProjectError> {
        Ok(self.history.create_branch(name)?)
    }

    /// Switches the repository to a branch and loads its model (a `Loaded`
    /// change event; undo and redo start again). Refuses with
    /// [`ProjectError::UncommittedChanges`] if the model has changes since
    /// the last checkpoint, and refuses a branch whose model cannot be read,
    /// before anything is switched.
    pub fn switch_branch(&mut self, name: &str) -> Result<ChangeEvent, ProjectError> {
        if self.history.has_uncommitted_changes()? {
            return Err(ProjectError::UncommittedChanges);
        }
        let target = self.history.load_commit(&format!("refs/heads/{name}"))?;
        let model = read(&target, next_on_branches(&self.history))?;
        let files = self.history.switch_branch(name)?;
        self.unmatched = model.unmatched;
        let event = self
            .state
            .load(model.tree, model.locks, &format!("Switch to branch {name}"));
        self.adopt(files)?;
        Ok(event)
    }

    /// Whether the model changed since the last checkpoint.
    pub fn has_uncommitted_changes(&self) -> Result<bool, ProjectError> {
        Ok(self.history.has_uncommitted_changes()?)
    }

    /// Saves the model: the documents whose printed form changed, and the
    /// identities and locks.
    fn save(&mut self) -> Result<(), ProjectError> {
        let tree = self.state.tree();
        let printed = print_documents(tree);
        let documents = printed
            .iter()
            .map(|(path, text)| {
                let unchanged = self.printed.get(path) == Some(text);
                let text = match self.saved.get(path) {
                    Some(saved) if unchanged => saved,
                    _ => text,
                };
                (path.clone(), text.clone())
            })
            .collect();
        let files = ModelFiles {
            documents,
            identities: identities(tree, self.state.locks()),
        };
        self.history.save(&files)?;
        self.saved = files.documents;
        self.printed = printed;
        Ok(())
    }

    /// Takes `files`, just read, as the saved state of the model. Saves the
    /// identity file at once if elements got new ids or locks were dropped,
    /// leaving the text as it was written.
    fn adopt(&mut self, files: ModelFiles) -> Result<(), ProjectError> {
        let identities = identities(self.state.tree(), self.state.locks());
        self.printed = print_documents(self.state.tree());
        self.saved = files.documents;
        if (&identities.elements, &identities.locks)
            != (&files.identities.elements, &files.identities.locks)
        {
            self.history.save(&ModelFiles {
                documents: self.saved.clone(),
                identities,
            })?;
        }
        Ok(())
    }
}

fn print_documents(tree: &Tree) -> BTreeMap<String, String> {
    print(tree).into_iter().map(|s| (s.path, s.text)).collect()
}

/// A model read from text, with identities attached.
struct Model {
    tree: Tree,
    locks: BTreeSet<ElementId>,
    unmatched: Vec<String>,
}

/// Parses model files and gives each element the id stored for its locator.
/// New ids start at `next`, or higher if the files say so.
fn read(files: &ModelFiles, next: u64) -> Result<Model, ProjectError> {
    let sources: Vec<Source> = files
        .documents
        .iter()
        .map(|(path, text)| Source::new(path.as_str(), text.as_str()))
        .collect();
    let mut tree = parse(&sources);
    // Locator to stored id; `None` when two entries claim the same locator.
    let mut stored: HashMap<&str, Option<ElementId>> = HashMap::new();
    for (id, locator) in &files.identities.elements {
        let id = element_id(id)?;
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
    let taken: BTreeSet<ElementId> = ids.values().copied().collect();
    for (id, locator) in &files.identities.elements {
        if !taken.contains(&element_id(id)?) {
            unmatched.push(locator.clone());
        }
    }
    tree.reserve_ids(ElementId::from_raw(next.max(next_id(&files.identities)?)));
    tree.rekey(&ids).map_err(|_| {
        ProjectError::Format("agentique.json: an element id is reserved or used twice".into())
    })?;
    let locks = files
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

/// The next id an identity file allows: above its `next` and every id in it.
fn next_id(identities: &Identities) -> Result<u64, ProjectError> {
    let mut next = identities.next;
    for id in identities.elements.keys() {
        next = next.max(element_id(id)?.raw() + 1);
    }
    if next >= LIBRARY_IDS {
        return Err(ProjectError::Format(format!(
            "agentique.json: the next id {next} is reserved for the library"
        )));
    }
    Ok(next)
}

/// The highest next id at the tip of any branch, so that an element created
/// on one branch never takes the id of a different element on another.
/// Unreadable branches are skipped. Reads each branch's model folder: fine
/// for a few branches, to be replaced by reading only `agentique.json`.
fn next_on_branches(history: &History) -> u64 {
    let branches = history.branches().unwrap_or_default();
    branches
        .iter()
        .filter_map(|branch| history.load_commit(&format!("refs/heads/{branch}")).ok())
        .filter_map(|tip| next_id(&tip.identities).ok())
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
        next: tree.next_id().raw(),
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
