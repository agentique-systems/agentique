//! Commits, the log, branches and reading any version, through libgit2.
use crate::folder::{is_model_file, model_files, read_model_files, text_of};
use crate::{Error, GITIGNORE, History, MODEL_FOLDER, ModelFiles, Result};
use git2::{
    BranchType, Commit, ErrorCode, FileMode, ObjectType, Oid, TreeWalkMode, TreeWalkResult,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// A saved point in a project's history: a git commit that changed the model
/// folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    /// The commit id.
    pub id: String,
    pub message: String,
    /// Seconds since the Unix epoch.
    pub time: i64,
}

impl Checkpoint {
    fn of(commit: &Commit) -> Self {
        Checkpoint {
            id: commit.id().to_string(),
            message: commit.message().unwrap_or_default().trim_end().to_owned(),
            time: commit.time().seconds(),
        }
    }
}

impl History {
    /// Commits the model folder as saved on disk. Everything else in the
    /// repository, staged or not, is left as it is. Returns `None` when the
    /// model folder is unchanged since the last commit. Refuses if a model
    /// file was edited outside the app since it was loaded or saved.
    pub fn commit(&mut self, message: &str) -> Result<Option<Checkpoint>> {
        self.check_unchanged()?;
        let mut files: BTreeSet<String> = read_model_files(&self.dir)?.into_keys().collect();
        if self.dir.join(GITIGNORE).is_file() {
            files.insert(GITIGNORE.to_owned());
        }
        let head = self.head_commit()?;
        let base = match &head {
            Some(commit) => commit.tree()?,
            None => {
                let empty = self.repo.treebuilder(None)?.write()?;
                self.repo.find_tree(empty)?
            }
        };
        // Stage the model files so that `git status` agrees with the commit,
        // and build the commit's tree from HEAD's, replacing the model files.
        let mut index = self.repo.index()?;
        // Start from the index on disk: git may have changed it since.
        index.read(true)?;
        let mut update = git2::build::TreeUpdateBuilder::new();
        for path in self.files_in(&base)?.keys() {
            if !files.contains(path) {
                let path = self.repo_path(path);
                index.remove_path(Path::new(&path))?;
                update.remove(path);
            }
        }
        for path in &files {
            let path = self.repo_path(path);
            index.add_path(Path::new(&path))?;
            let entry = index.get_path(Path::new(&path), 0).expect("just staged");
            update.upsert(path, entry.id, FileMode::Blob);
        }
        let tree = self
            .repo
            .find_tree(update.create_updated(&self.repo, &base)?)?;
        if tree.id() == base.id() {
            return Ok(None);
        }
        index.write()?;
        let signature = self
            .repo
            .signature()
            .or_else(|_| git2::Signature::now("Agentique", "agentique@localhost"))?;
        let parents: Vec<&Commit> = head.iter().collect();
        let id = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;
        Ok(Some(Checkpoint::of(&self.repo.find_commit(id)?)))
    }

    /// Commits reachable from HEAD that changed the model folder, newest
    /// first.
    pub fn log(&self) -> Result<Vec<Checkpoint>> {
        let mut out = Vec::new();
        if self.head_commit()?.is_none() {
            return Ok(out);
        }
        let mut walk = self.repo.revwalk()?;
        walk.push_head()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        for id in walk {
            let commit = self.repo.find_commit(id?)?;
            let folder = self.folder_tree(&commit)?;
            let changed = if commit.parent_count() == 0 {
                folder.is_some()
            } else {
                let mut changed = false;
                for parent in commit.parents() {
                    changed |= self.folder_tree(&parent)? != folder;
                }
                changed
            };
            if changed {
                out.push(Checkpoint::of(&commit));
            }
        }
        Ok(out)
    }

    /// The model folder at a commit (any git revision, e.g. a commit id or
    /// `refs/heads/main`).
    pub fn load_commit(&self, revision: &str) -> Result<ModelFiles> {
        let commit = self.repo.revparse_single(revision)?.peel_to_commit()?;
        model_files(&self.files_in(&commit.tree()?)?)
    }

    /// Whether the saved model files differ from the last commit.
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        self.recover()?;
        let committed = match self.head_commit()? {
            Some(commit) => self.files_in(&commit.tree()?)?,
            None => BTreeMap::new(),
        };
        Ok(read_model_files(&self.dir)? != committed)
    }

    /// The current branch.
    pub fn branch(&self) -> Result<String> {
        let head = self.repo.find_reference("HEAD")?;
        head.symbolic_target()?
            .and_then(|target| target.strip_prefix("refs/heads/"))
            .map(str::to_owned)
            .ok_or(Error::Detached)
    }

    /// The repository's local branches, sorted by name.
    pub fn branches(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for branch in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                names.push(name.to_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    /// Starts a branch at the last commit, without switching to it.
    pub fn create_branch(&self, name: &str) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head, false)?;
        Ok(())
    }

    /// Switches the whole repository to a branch and reads its model.
    /// Refuses when the model has changes that are not committed, and when
    /// the switch would overwrite other changed files.
    pub fn switch_branch(&mut self, name: &str) -> Result<ModelFiles> {
        if self.has_uncommitted_changes()? {
            return Err(Error::Uncommitted);
        }
        let reference = format!("refs/heads/{name}");
        // Refuse a model this version cannot read before touching anything.
        self.load_commit(&reference)?;
        {
            let target = self.repo.find_reference(&reference)?.peel_to_commit()?;
            self.repo
                .checkout_tree(
                    target.as_object(),
                    Some(git2::build::CheckoutBuilder::new().safe()),
                )
                .map_err(|e| match e.code() {
                    ErrorCode::Conflict => Error::WouldOverwrite,
                    _ => e.into(),
                })?;
        }
        self.repo.set_head(&reference)?;
        self.load()
    }

    fn head_commit(&self) -> Result<Option<Commit<'_>>> {
        match self.repo.head() {
            Ok(head) => Ok(Some(head.peel_to_commit()?)),
            Err(e) if matches!(e.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn repo_path(&self, path: &str) -> String {
        format!("{MODEL_FOLDER}/{path}")
    }

    fn folder_tree(&self, commit: &Commit) -> Result<Option<Oid>> {
        match commit.tree()?.get_path(Path::new(MODEL_FOLDER)) {
            Ok(entry) => Ok(Some(entry.id())),
            Err(e) if e.code() == ErrorCode::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Model files of the model folder inside a commit's tree.
    fn files_in(&self, root: &git2::Tree) -> Result<BTreeMap<String, String>> {
        let mut files = BTreeMap::new();
        let tree = match root.get_path(Path::new(MODEL_FOLDER)) {
            Ok(entry) => entry.to_object(&self.repo)?.peel_to_tree()?,
            Err(e) if e.code() == ErrorCode::NotFound => return Ok(files),
            Err(e) => return Err(e.into()),
        };
        let mut failure = None;
        tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
            let name = entry.name().unwrap_or_default();
            if name.starts_with('.') {
                return TreeWalkResult::Skip;
            }
            let path = format!("{dir}{name}");
            if entry.kind() == Some(ObjectType::Blob) && is_model_file(&path) {
                let text = self
                    .repo
                    .find_blob(entry.id())
                    .map_err(Error::from)
                    .and_then(|blob| text_of(&path, blob.content().to_vec()));
                match text {
                    Ok(text) => {
                        files.insert(path, text);
                    }
                    Err(e) => {
                        failure = Some(e);
                        return TreeWalkResult::Abort;
                    }
                }
            }
            TreeWalkResult::Ok
        })
        .or_else(|e| if failure.is_some() { Ok(()) } else { Err(e) })?;
        match failure {
            Some(e) => Err(e),
            None => Ok(files),
        }
    }
}
