//! Test-only repository faults; the wrapped repository remains the durable store.
//!
//! The explicit overrides deliberately violate read contracts to exercise service
//! authentication. They never rewrite database rows or become persistence truth.
#![allow(dead_code)] // Individual accepted-cache fixtures use different fault subsets.

use agq_modeling_repository::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

/// Substitute one selected blob read without modifying durable content.
#[derive(Clone, Debug)]
pub enum BlobOverride {
    /// Return exact test bytes, including intentionally malformed cache bytes.
    Bytes(Vec<u8>),
    /// Behave as if the selected content address is unavailable.
    Missing,
    /// Return an explicit storage error with the supplied test explanation.
    Failure(String),
}

#[derive(Default)]
struct Faults {
    manifest: Option<(ProjectId, ProjectRevisionId, RevisionManifest)>,
    blobs: BTreeMap<ContentDigest, BlobOverride>,
    integrity_report: Option<IntegrityReport>,
    fail_projects: bool,
    fail_revisions: bool,
    lose_create_acknowledgement: bool,
    fail_commit_before_durability: bool,
}

/// Forwarding decorator with independently resettable deterministic fault points.
pub struct RepositoryProbe {
    inner: Arc<dyn ModelingRepository>,
    faults: Mutex<Faults>,
}

impl RepositoryProbe {
    /// Start with no faults enabled and preserve the wrapped repository identity.
    pub fn new(inner: Arc<dyn ModelingRepository>) -> Self {
        Self {
            inner,
            faults: Mutex::new(Faults::default()),
        }
    }

    /// Override one existing revision's returned manifest in every read operation.
    ///
    /// Lookup identity is separate from the replacement's fields, allowing tests
    /// to corrupt a returned identity as well as caches or validation receipts.
    pub fn override_manifest(
        &self,
        project: ProjectId,
        revision: ProjectRevisionId,
        manifest: RevisionManifest,
    ) {
        self.faults.lock().unwrap().manifest = Some((project, revision, manifest));
    }

    /// Restore ordinary manifest reads without changing other configured faults.
    pub fn clear_manifest_override(&self) {
        self.faults.lock().unwrap().manifest = None;
    }

    /// Override exactly one content address until cleared or replaced.
    pub fn override_blob(&self, digest: ContentDigest, value: BlobOverride) {
        self.faults.lock().unwrap().blobs.insert(digest, value);
    }

    /// Restore all ordinary blob reads without changing manifest or write faults.
    pub fn clear_blob_overrides(&self) {
        self.faults.lock().unwrap().blobs.clear();
    }

    /// Substitute structural findings independently of enumeration failures.
    /// Passing None restores the wrapped adapter's normal integrity report.
    pub fn override_integrity_report(&self, report: Option<IntegrityReport>) {
        self.faults.lock().unwrap().integrity_report = report;
    }

    /// Independently fail project enumeration and revision enumeration.
    pub fn fail_enumeration(&self, projects: bool, revisions: bool) {
        let mut faults = self.faults.lock().unwrap();
        faults.fail_projects = projects;
        faults.fail_revisions = revisions;
    }

    /// The next successful durable create returns OutcomeUnknown to its caller.
    /// A failed underlying create does not consume this one-shot fault.
    pub fn lose_next_create_acknowledgement(&self) {
        self.faults.lock().unwrap().lose_create_acknowledgement = true;
    }

    /// Reject the next commit before invoking any method on the durable adapter.
    pub fn fail_next_commit_before_durability(&self) {
        self.faults.lock().unwrap().fail_commit_before_durability = true;
    }

    fn project_manifest(&self, value: RevisionManifest) -> RevisionManifest {
        match &self.faults.lock().unwrap().manifest {
            Some((project, revision, replacement))
                if *project == value.project_id && *revision == value.revision_id =>
            {
                replacement.clone()
            }
            _ => value,
        }
    }
}

impl ModelingRepository for RepositoryProbe {
    fn repository_id(&self) -> RepositoryId {
        self.inner.repository_id()
    }

    fn create_project(&self, request: &CreateProject) -> Result<CommitReceipt, RepositoryError> {
        let receipt = self.inner.create_project(request)?;
        let lose_acknowledgement =
            std::mem::take(&mut self.faults.lock().unwrap().lose_create_acknowledgement);
        if lose_acknowledgement {
            Err(RepositoryError::OutcomeUnknown(request.operation_id))
        } else {
            Ok(receipt)
        }
    }

    fn get_project(&self, project: ProjectId) -> Result<Project, RepositoryError> {
        self.inner.get_project(project)
    }

    fn list_projects(&self) -> Result<Vec<Project>, RepositoryError> {
        if self.faults.lock().unwrap().fail_projects {
            return Err(RepositoryError::Storage(
                "injected project enumeration failure".into(),
            ));
        }
        self.inner.list_projects()
    }

    fn create_branch(&self, branch: &Branch) -> Result<(), RepositoryError> {
        self.inner.create_branch(branch)
    }

    fn get_branch(&self, project: ProjectId, branch: BranchId) -> Result<Branch, RepositoryError> {
        self.inner.get_branch(project, branch)
    }

    fn list_branches(&self, project: ProjectId) -> Result<Vec<Branch>, RepositoryError> {
        self.inner.list_branches(project)
    }

    fn delete_branch(&self, project: ProjectId, branch: BranchId) -> Result<(), RepositoryError> {
        self.inner.delete_branch(project, branch)
    }

    fn load_revision(
        &self,
        project: ProjectId,
        revision: ProjectRevisionId,
    ) -> Result<RevisionManifest, RepositoryError> {
        self.inner
            .load_revision(project, revision)
            .map(|value| self.project_manifest(value))
    }

    fn list_revisions(&self, project: ProjectId) -> Result<Vec<RevisionManifest>, RepositoryError> {
        if self.faults.lock().unwrap().fail_revisions {
            return Err(RepositoryError::Storage(
                "injected revision enumeration failure".into(),
            ));
        }
        self.inner.list_revisions(project).map(|values| {
            values
                .into_iter()
                .map(|value| self.project_manifest(value))
                .collect()
        })
    }

    fn read_blob(&self, digest: ContentDigest) -> Result<Vec<u8>, RepositoryError> {
        let replacement = self.faults.lock().unwrap().blobs.get(&digest).cloned();
        match replacement {
            Some(BlobOverride::Bytes(bytes)) => Ok(bytes),
            Some(BlobOverride::Missing) => Err(RepositoryError::NotFound(format!(
                "injected missing blob {digest}"
            ))),
            Some(BlobOverride::Failure(message)) => Err(RepositoryError::Storage(message)),
            None => self.inner.read_blob(digest),
        }
    }

    fn list_revision_history(
        &self,
        project: ProjectId,
        start: ProjectRevisionId,
    ) -> Result<Vec<RevisionManifest>, RepositoryError> {
        self.inner
            .list_revision_history(project, start)
            .map(|values| {
                values
                    .into_iter()
                    .map(|value| self.project_manifest(value))
                    .collect()
            })
    }

    fn commit_revision(&self, request: &CommitRevision) -> Result<CommitReceipt, RepositoryError> {
        let fail_before_durability =
            std::mem::take(&mut self.faults.lock().unwrap().fail_commit_before_durability);
        if fail_before_durability {
            return Err(RepositoryError::Storage(
                "injected commit failure before durability".into(),
            ));
        }
        self.inner.commit_revision(request)
    }

    fn check_integrity(&self) -> Result<IntegrityReport, RepositoryError> {
        let replacement = self.faults.lock().unwrap().integrity_report.clone();
        match replacement {
            Some(report) => Ok(report),
            None => self.inner.check_integrity(),
        }
    }
}
