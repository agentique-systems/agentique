//! Application boundary for durable source-backed modeling.
//!
//! Resolve a branch once, then query its immutable handle. Edits construct a local
//! candidate before persistence; no acknowledged head precedes durable commit.
#![forbid(unsafe_code)]

mod cache_codec;
mod part_insertion;
mod part_rename;
mod profile;
mod source_identity;
pub use part_rename::RenamePart;
mod query;
pub use agq_kerml_text::{CompilationControl, CompilationStage};
use agq_kerml_text::{ProjectChange, sysml::CanonicalSysmlSystemsLibrary};
pub use agq_modeling_repository as repository;
use agq_modeling_repository::*;
use agq_modeling_workspace::{
    ProjectRevisionCheckpoint, ProjectWorkspace, ValidatedProjectRevision, WorkingProjectRevision,
};
pub use query::*;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

/// Resolve exactly one immutable revision at request start.
#[derive(Clone, Copy, Debug)]
pub enum RevisionSelector {
    /// Explicit immutable revision.
    Revision(ProjectRevisionId),
    /// Resolve the branch head once.
    Branch(BranchId),
}

/// Errors retain durability conflicts, reconstruction failures and validation findings.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// Unpublished work was interrupted; no candidate or durable commit exists.
    #[error(transparent)]
    Cancelled(#[from] agq_kerml_semantics::Cancelled),
    /// Repository failure, including explicit CAS conflict and unknown acknowledgement.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Source preparation failed before persistence.
    #[error(transparent)]
    Workspace(Box<agq_modeling_workspace::WorkspaceError>),
    /// Actual Phase1V1 acceptance failed.
    #[error(transparent)]
    Validation(#[from] agq_modeling_workspace::ValidationFailure),
    /// Invalid stored identity, publication, receipt or unavailable semantic query.
    #[error("modeling service: {0}")]
    Invalid(String),
    /// Representation encoding failed.
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}
impl ServiceError {
    /// Distinguish operator cancellation from failed semantics or authentication.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled(_))
    }
}
impl From<agq_modeling_workspace::WorkspaceError> for ServiceError {
    fn from(error: agq_modeling_workspace::WorkspaceError) -> Self {
        Self::Workspace(Box::new(error))
    }
}

/// One request's immutable source, semantic and persisted validation binding.
#[derive(Clone)]
pub struct BoundRevision {
    manifest: Arc<RevisionManifest>,
    working: Arc<WorkingProjectRevision>,
    validated: Option<ValidatedProjectRevision>,
    load_path: RevisionLoadPath,
}
/// Observed reconstruction path, separate from semantic or validation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionLoadPath {
    /// Reconstructed from exact durable source and identity checkpoints.
    DurableSource,
    /// Restored from a persisted cache after authenticating all semantic inputs.
    AuthenticatedSemanticCache,
    /// Reused an already authenticated immutable revision in this service.
    ImmutableMemory,
}
impl BoundRevision {
    /// Report the actual path for cache diagnostics and performance measurement.
    pub fn load_path(&self) -> RevisionLoadPath {
        self.load_path
    }
    /// Exact immutable handle for this request; branch movement cannot replace it.
    pub fn revision(&self) -> &Arc<WorkingProjectRevision> {
        &self.working
    }
    /// Durable metadata authenticated when the request was resolved.
    pub fn manifest(&self) -> &RevisionManifest {
        &self.manifest
    }
    /// Present only for a stored Validated receipt that passed restoration checks.
    pub fn validated(&self) -> Option<&ValidatedProjectRevision> {
        self.validated.as_ref()
    }
    /// Execute the actual workspace contract; does not mutate persisted status.
    pub fn validate(&self) -> Result<ValidatedProjectRevision, ServiceError> {
        Ok(self.working.validate()?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CacheKey {
    revision: ProjectRevisionId,
    publications: PublicationBinding,
    context: ContentDigest,
}
struct RevisionCache {
    capacity: usize,
    entries: BTreeMap<CacheKey, BoundRevision>,
    order: VecDeque<CacheKey>,
}
impl RevisionCache {
    fn insert(&mut self, revision: BoundRevision) {
        if self.capacity == 0 {
            return;
        }
        let Ok(context) = context_digest(&revision.working) else {
            return;
        };
        let key = CacheKey {
            revision: revision.working.revision(),
            publications: revision.manifest.accepted_publications.clone(),
            context,
        };
        self.order.retain(|entry| entry != &key);
        self.order.push_back(key.clone());
        self.entries.insert(key, revision);
        while self.entries.len() > self.capacity {
            if let Some(key) = self.order.pop_front() {
                self.entries.remove(&key);
            }
        }
    }
}

/// Shared immutable standards, durable repository and evictable request cache.
pub struct ModelingService {
    repository: Arc<dyn ModelingRepository>,
    publication: Arc<CanonicalSysmlSystemsLibrary>,
    cache: Mutex<RevisionCache>,
}
impl ModelingService {
    /// Bind an authenticated publication catalogue entry; never rebuild standards.
    pub fn new(
        repository: Arc<dyn ModelingRepository>,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        cache_capacity: usize,
    ) -> Self {
        Self {
            repository,
            publication,
            cache: Mutex::new(RevisionCache {
                capacity: cache_capacity,
                entries: BTreeMap::new(),
                order: VecDeque::new(),
            }),
        }
    }
    /// Transport-neutral repository access for project/history projections.
    pub fn repository(&self) -> &Arc<dyn ModelingRepository> {
        &self.repository
    }
    /// Open another repository over the same authenticated immutable runtime.
    /// Revision and candidate caches are independent of the previous repository.
    pub fn for_repository(&self, repository: Arc<dyn ModelingRepository>) -> Self {
        Self::new(repository, self.publication.clone(), 8)
    }
    /// Cache eviction is unrelated to durability and does not affect retained readers.
    pub fn evict_all(&self) {
        let mut cache = self.cache.lock().expect("revision cache");
        cache.entries.clear();
        cache.order.clear();
    }
    /// Resolve a branch once and restore only that exact immutable revision.
    pub fn resolve(
        &self,
        project: ProjectId,
        selector: RevisionSelector,
    ) -> Result<BoundRevision, ServiceError> {
        let mut profile = profile::ReadProfile::new("resolve_revision");
        let revision = match selector {
            RevisionSelector::Revision(id) => id,
            RevisionSelector::Branch(id) => self.repository.get_branch(project, id)?.head,
        };
        let manifest = self.repository.load_revision(project, revision)?;
        manifest.verify()?;
        if manifest.project_id != project
            || manifest.revision_id != revision
            || manifest.accepted_publications != publication_binding(&self.publication)
        {
            return Err(ServiceError::Invalid(
                "revision/publication identity mismatch".into(),
            ));
        }
        {
            let cache = self.cache.lock().expect("revision cache");
            if let Some(bound) = cache
                .entries
                .values()
                .find(|bound| bound.manifest.as_ref() == &manifest)
            {
                // Each cached entry was authenticated against its immutable semantic context.
                let mut bound = bound.clone();
                bound.load_path = RevisionLoadPath::ImmutableMemory;
                profile.phase("immutable_memory_lookup");
                return Ok(bound);
            }
        }
        profile.phase("repository_manifest_authentication");
        let checkpoint_bytes = self.repository.read_blob(manifest.checkpoint_digest)?;
        if ContentDigest::of(&checkpoint_bytes) != manifest.checkpoint_digest {
            return Err(ServiceError::Invalid(
                "identity checkpoint checksum mismatch".into(),
            ));
        }
        let checkpoint: ProjectRevisionCheckpoint = serde_json::from_slice(&checkpoint_bytes)?;
        let mut sources = BTreeMap::new();
        for document in &manifest.documents {
            let bytes = self.repository.read_blob(document.content_digest)?;
            sources.insert(
                document.document_id,
                String::from_utf8(bytes)
                    .map_err(|_| ServiceError::Invalid("non-UTF8 source blob".into()))?,
            );
        }
        profile.phase("source_and_identity_load");
        let (working, load_path) =
            match self.restore_semantic_cache(&manifest, &checkpoint, &sources) {
                Some(working) => (working, RevisionLoadPath::AuthenticatedSemanticCache),
                None => (
                    checkpoint
                        .restore(self.publication.clone(), &sources)
                        .map_err(|error| ServiceError::Invalid(error.to_string()))?,
                    RevisionLoadPath::DurableSource,
                ),
            };
        profile.phase(match load_path {
            RevisionLoadPath::DurableSource => "source_fallback",
            _ => "semantic_cache_restoration",
        });
        if std::env::var_os("AGENTIQUE_STARTUP_PROFILE").is_some_and(|value| value == "1") {
            use std::io::Write;
            let _ = writeln!(
                std::io::stderr().lock(),
                "{}",
                serde_json::json!({
                    "format": "agentique-restored-compilation-profile/1",
                    "revision": revision,
                    "load_path": format!("{load_path:?}"),
                    "timings": working.compilation_timings(),
                    "work": working.compilation_work(),
                })
            );
        }
        verify_restored_documents(&manifest, &working)?;
        let validated = match &manifest.validation {
            ValidationState::Working => None,
            ValidationState::Validated(receipt) => {
                if receipt.semantic_digest != graph_digest(&working)?
                    || receipt.semantic_context != context_digest(&working)?
                    || working.producer_closure().is_none_or(|certificate| {
                        ContentDigest(certificate.digest()) != receipt.closure_digest
                    })
                {
                    return Err(ServiceError::Invalid(
                        "validation receipt does not match restored semantics".into(),
                    ));
                }
                Some(working.validate()?)
            }
        };
        profile.phase("revision_receipt_and_validation_authentication");
        let bound = BoundRevision {
            manifest: Arc::new(manifest),
            working,
            validated,
            load_path,
        };
        self.cache
            .lock()
            .expect("revision cache")
            .insert(bound.clone());
        profile.phase("immutable_revision_cache_insert");
        Ok(bound)
    }
    fn restore_semantic_cache(
        &self,
        manifest: &RevisionManifest,
        checkpoint: &ProjectRevisionCheckpoint,
        sources: &BTreeMap<DocumentId, String>,
    ) -> Option<Arc<WorkingProjectRevision>> {
        let mut profile = profile::ReadProfile::new("restore_semantic_cache");
        // Cache failure never grants validation or removes the durable source path.
        let reference = manifest.semantic_cache.as_ref()?;
        if reference.source_binding != manifest.source_binding().ok()? {
            return None;
        }
        if let ValidationState::Validated(receipt) = &manifest.validation
            && (reference.semantic_context != receipt.semantic_context
                || reference.closure_digest != receipt.closure_digest)
        {
            return None;
        }
        let bytes = self.repository.read_blob(reference.content_digest).ok()?;
        if ContentDigest::of(&bytes) != reference.content_digest {
            return None;
        }
        profile.phase("cache_load_and_blob_authentication");
        let cache = cache_codec::decode(&reference.format, &bytes)?;
        profile.phase("cache_decode_and_frontier_digest");
        if let ValidationState::Validated(receipt) = &manifest.validation
            && receipt.semantic_digest != ContentDigest(cache.source.model_digest)
        {
            return None;
        }
        if ContentDigest(cache.source.context_contract_digest) != reference.semantic_context
            || ContentDigest(cache.source.semantic_closure_digest) != reference.closure_digest
        {
            return None;
        }
        let working = checkpoint
            .restore_cached(self.publication.clone(), sources, &cache)
            .ok()?;
        profile.phase("source_bound_graph_closure_and_audit_restore");
        if !working.compilation_work().semantic_cache_used
            || context_digest(&working).ok()? != reference.semantic_context
            || working
                .producer_closure()
                .map(|closure| ContentDigest(closure.digest()))
                != Some(reference.closure_digest)
            || graph_digest(&working).ok()? != ContentDigest(cache.source.model_digest)
        {
            return None;
        }
        profile.phase("cache_semantic_identity_authentication");
        Some(working)
    }
    /// Create a project with a real parentless empty Working revision and main head.
    /// Retain a prepared project instead when lost acknowledgement must be retried.
    pub fn create_project(
        &self,
        name: &str,
        description: Option<String>,
    ) -> Result<Project, ServiceError> {
        self.commit_project(&self.prepare_project(name, description)?)
    }
    /// Prepare a new project without storing it or acknowledging any branch head.
    pub fn prepare_project(
        &self,
        name: &str,
        description: Option<String>,
    ) -> Result<PreparedProject, ServiceError> {
        let workspace = ProjectWorkspace::open(self.publication.clone())?;
        self.prepare_project_from_revision(name, description, workspace.head().clone(), false)
    }
    /// Import an initial workspace revision without inventing a new semantic identity.
    ///
    /// Parentless initial revisions only. Use source changes after create_project for
    /// normal authored projects so every stored parent resolves.
    pub fn create_from_revision(
        &self,
        name: &str,
        description: Option<String>,
        initial: Arc<WorkingProjectRevision>,
        validate: bool,
    ) -> Result<Project, ServiceError> {
        self.commit_project(&self.prepare_project_from_revision(
            name,
            description,
            initial,
            validate,
        )?)
    }
    /// Prepare an initial workspace import with a stable operation identity.
    pub fn prepare_project_from_revision(
        &self,
        name: &str,
        description: Option<String>,
        initial: Arc<WorkingProjectRevision>,
        validate: bool,
    ) -> Result<PreparedProject, ServiceError> {
        if name.trim().is_empty() || initial.parent().is_some() {
            return Err(ServiceError::Invalid(
                "project requires a name and parentless initial revision".into(),
            ));
        }
        if publication_binding(initial.accepted_sysml()) != publication_binding(&self.publication) {
            return Err(ServiceError::Invalid(
                "initial revision uses foreign publications".into(),
            ));
        }
        let validated = if validate {
            Some(initial.validate()?)
        } else {
            None
        };
        let metadata = metadata(description);
        let project = Project {
            id: initial.project(),
            name: name.into(),
            default_branch: BranchId::new(),
            metadata: metadata.clone(),
        };
        let branch = Branch {
            id: project.default_branch,
            project_id: project.id,
            name: "main".into(),
            head: initial.revision(),
            metadata,
        };
        let candidate = prepare_candidate(&initial, validate)?;
        Ok(PreparedProject {
            request: CreateProject {
                operation_id: OperationId::new(),
                project,
                branch,
                initial: candidate,
            },
            working: initial,
            validated,
        })
    }
    /// Durably create exactly the prepared project; replay after lost acknowledgement.
    pub fn commit_project(&self, prepared: &PreparedProject) -> Result<Project, ServiceError> {
        self.repository.create_project(&prepared.request)?;
        self.cache_committed(
            prepared.request.initial.manifest.clone(),
            prepared.working.clone(),
            prepared.validated.clone(),
        );
        Ok(prepared.request.project.clone())
    }
    /// Create an independent head at an existing same-project revision.
    pub fn create_branch(
        &self,
        project: ProjectId,
        name: &str,
        at: ProjectRevisionId,
    ) -> Result<Branch, ServiceError> {
        if name.trim().is_empty() {
            return Err(ServiceError::Invalid("empty branch name".into()));
        }
        self.repository.load_revision(project, at)?;
        let branch = Branch {
            id: BranchId::new(),
            project_id: project,
            name: name.into(),
            head: at,
            metadata: metadata(None),
        };
        self.repository.create_branch(&branch)?;
        Ok(branch)
    }
    /// Construct an inspectable candidate without changing durable or acknowledged heads.
    pub fn prepare_changes(
        &self,
        command: ApplyDocumentChanges,
    ) -> Result<PreparedChanges, ServiceError> {
        let branch = self
            .repository
            .get_branch(command.project, command.branch)?;
        if branch.head != command.expected_head {
            return Err(RepositoryError::Conflict {
                expected: command.expected_head,
                actual: branch.head,
            }
            .into());
        }
        let base = self.resolve(
            command.project,
            RevisionSelector::Revision(command.expected_head),
        )?;
        let workspace = ProjectWorkspace::from_revision(base.working.clone());
        let working = workspace.prepare(command.expected_head, command.changes)?;
        let candidate = prepare_candidate(&working, command.validate)?;
        let validated = if command.validate {
            Some(working.validate()?)
        } else {
            None
        };
        Ok(PreparedChanges {
            request: CommitRevision {
                operation_id: command.operation_id,
                project_id: command.project,
                branch_id: command.branch,
                expected_head: command.expected_head,
                candidate,
            },
            working,
            validated,
        })
    }
    /// Commit exactly the prepared candidate. Retry this same value after lost acknowledgement.
    pub fn commit_prepared(
        &self,
        prepared: &PreparedChanges,
    ) -> Result<CommitReceipt, ServiceError> {
        let mut profile = profile::ReadProfile::new("commit_prepared");
        let receipt = self.repository.commit_revision(&prepared.request)?;
        profile.phase("durable_repository_commit");
        self.cache_committed(
            prepared.request.candidate.manifest.clone(),
            prepared.working.clone(),
            prepared.validated.clone(),
        );
        profile.phase("committed_revision_memory_binding");
        Ok(receipt)
    }
    /// Ordinary source-backed edit flow, preserving all CAS and validation failures.
    pub fn apply_document_changes(
        &self,
        command: ApplyDocumentChanges,
    ) -> Result<CommitReceipt, ServiceError> {
        self.commit_prepared(&self.prepare_changes(command)?)
    }
    fn cache_committed(
        &self,
        manifest: RevisionManifest,
        working: Arc<WorkingProjectRevision>,
        validated: Option<ValidatedProjectRevision>,
    ) {
        self.cache
            .lock()
            .expect("revision cache")
            .insert(BoundRevision {
                manifest: Arc::new(manifest),
                working,
                validated,
                load_path: RevisionLoadPath::ImmutableMemory,
            })
    }
    /// Identity-driven diff of two explicitly bound revisions.
    pub fn diff(
        &self,
        project: ProjectId,
        from: ProjectRevisionId,
        to: ProjectRevisionId,
    ) -> Result<RevisionDiff, ServiceError> {
        let before = self.resolve(project, RevisionSelector::Revision(from))?;
        let after = self.resolve(project, RevisionSelector::Revision(to))?;
        revision_diff(&before, &after)
    }
    /// Offline structural checks plus authentic restoration of every retained revision.
    pub fn check_integrity(&self) -> Result<IntegrityReport, ServiceError> {
        let mut report = self.repository.check_integrity()?;
        // An offline audit must inspect durable inputs even when this service
        // already retains a valid immutable handle for the same revision.
        let checker = Self::new(self.repository.clone(), self.publication.clone(), 0);
        let mut seen = std::collections::BTreeSet::new();
        let projects = match self.repository.list_projects() {
            Ok(projects) => projects,
            Err(error) => {
                report.errors.push(format!("project enumeration: {error}"));
                return Ok(report);
            }
        };
        for project in projects {
            let revisions = match self.repository.list_revisions(project.id) {
                Ok(revisions) => revisions,
                Err(error) => {
                    report
                        .errors
                        .push(format!("{} revision enumeration: {error}", project.id));
                    continue;
                }
            };
            for manifest in revisions {
                if !seen.insert(manifest.revision_id) {
                    continue;
                }
                match checker.resolve(project.id, RevisionSelector::Revision(manifest.revision_id))
                {
                    Ok(bound)
                        if manifest.semantic_cache.is_some()
                            && bound.load_path() == RevisionLoadPath::DurableSource =>
                    {
                        report.discardable_caches.push(manifest.revision_id);
                    }
                    Ok(_) => {}
                    Err(error) => report
                        .errors
                        .push(format!("{}: {error}", manifest.revision_id)),
                }
            }
        }
        report.discardable_caches.sort();
        report.discardable_caches.dedup();
        Ok(report)
    }
}

/// Source command; arbitrary semantic graph mutation is not exposed.
pub struct ApplyDocumentChanges {
    /// Idempotency key retained with the prepared candidate.
    pub operation_id: OperationId,
    /// Project identity.
    pub project: ProjectId,
    /// Named head identity.
    pub branch: BranchId,
    /// Explicit base expected by the caller.
    pub expected_head: ProjectRevisionId,
    /// Source changes evaluated by ProjectWorkspace.
    pub changes: Vec<ProjectChange>,
    /// Require Phase1V1 before persistence; otherwise persist Working.
    pub validate: bool,
}
/// Initial project candidate retained across durability or acknowledgement failures.
pub struct PreparedProject {
    request: CreateProject,
    working: Arc<WorkingProjectRevision>,
    validated: Option<ValidatedProjectRevision>,
}
impl PreparedProject {
    /// Stable project identity and metadata allocated during preparation.
    pub fn project(&self) -> &Project {
        &self.request.project
    }
    /// Exact replayable initial persistence request.
    pub fn request(&self) -> &CreateProject {
        &self.request
    }
    /// Immutable candidate before or after durable creation.
    pub fn revision(&self) -> &Arc<WorkingProjectRevision> {
        &self.working
    }
}
/// Inspectable candidate survives CAS conflicts and durability errors.
pub struct PreparedChanges {
    request: CommitRevision,
    working: Arc<WorkingProjectRevision>,
    validated: Option<ValidatedProjectRevision>,
}
impl PreparedChanges {
    /// Query an uncommitted candidate without granting durability or branch authority.
    pub fn bound_revision(&self) -> BoundRevision {
        BoundRevision {
            manifest: Arc::new(self.request.candidate.manifest.clone()),
            working: self.working.clone(),
            validated: self.validated.clone(),
            load_path: RevisionLoadPath::ImmutableMemory,
        }
    }
    /// Validate this exact candidate; source and project revision identity are unchanged.
    pub fn validate(&self) -> Result<Self, ServiceError> {
        if self.validated.is_some() {
            // Preserve the exact durable request after validation, including
            // metadata and cache bytes. Rebuilding it would change the payload
            // under the same operation ID and break lost-acknowledgement retry.
            return Ok(Self {
                request: self.request.clone(),
                working: self.working.clone(),
                validated: self.validated.clone(),
            });
        }
        let mut candidate = prepare_candidate(&self.working, true)?;
        // Source-command reconciliation evidence survives Working -> Validated.
        candidate.manifest.metadata = self.request.candidate.manifest.metadata.clone();
        let mut request = self.request.clone();
        request.candidate = candidate;
        Ok(Self {
            request,
            working: self.working.clone(),
            validated: Some(self.working.validate()?),
        })
    }
    /// Exact immutable semantic candidate.
    pub fn revision(&self) -> &Arc<WorkingProjectRevision> {
        &self.working
    }
    /// Exact replayable durable request.
    pub fn request(&self) -> &CommitRevision {
        &self.request
    }
}

fn metadata(description: Option<String>) -> ResourceMetadata {
    ResourceMetadata {
        created: chrono::Utc::now().to_rfc3339(),
        description,
        ..Default::default()
    }
}
/// Accepted catalogue identities used by every revision in this service.
pub fn publication_binding(publication: &CanonicalSysmlSystemsLibrary) -> PublicationBinding {
    PublicationBinding {
        kerml: ContentDigest(publication.accepted_kerml().semantic_digest()),
        sysml: ContentDigest(publication.publication_digest()),
    }
}
fn graph_digest(revision: &WorkingProjectRevision) -> Result<ContentDigest, ServiceError> {
    Ok(ContentDigest(
        revision
            .kerml_queries()
            .map_err(|e| ServiceError::Invalid(format!("{e:?}")))?
            .context()
            .model_digest,
    ))
}
fn context_digest(revision: &WorkingProjectRevision) -> Result<ContentDigest, ServiceError> {
    Ok(ContentDigest(
        revision
            .kerml_queries()
            .map_err(|e| ServiceError::Invalid(format!("{e:?}")))?
            .context()
            .closure_contract_digest(),
    ))
}
/// Export exact durable sources and identities, plus a disposable validated graph cache.
/// Accepted standard graphs remain external authenticated dependencies.
pub fn prepare_candidate(
    revision: &Arc<WorkingProjectRevision>,
    validate: bool,
) -> Result<CandidateRevision, ServiceError> {
    let mut profile = profile::ReadProfile::new("prepare_durable_candidate");
    let validated = if validate {
        Some(revision.validate()?)
    } else {
        None
    };
    profile.phase("existing_semantic_state_validation");
    let checkpoint = serde_json::to_vec(&revision.checkpoint())?;
    let checkpoint_digest = ContentDigest::of(&checkpoint);
    let mut blobs = BTreeMap::from([(checkpoint_digest, checkpoint)]);
    let documents = revision
        .documents()
        .map(|(path, document)| {
            let bytes = document.source().as_bytes().to_vec();
            let content_digest = ContentDigest::of(&bytes);
            blobs.insert(content_digest, bytes);
            DocumentManifest {
                document_id: document.id(),
                source_revision_id: document.revision(),
                language: document.language(),
                path: path.into(),
                content_digest,
            }
        })
        .collect();
    let mut manifest = RevisionManifest {
        format_version: RevisionManifest::FORMAT_VERSION,
        revision_id: revision.revision(),
        parent_revision_id: revision.parent(),
        project_id: revision.project(),
        metadata: metadata(None),
        documents,
        accepted_publications: publication_binding(revision.accepted_sysml()),
        checkpoint_digest,
        validation: ValidationState::Working,
        semantic_cache: None,
    };
    profile.phase("authoritative_sources_and_identity_export");
    if validate {
        manifest.validation = ValidationState::Validated(ValidationReceipt {
            acceptance_contract: agq_modeling_workspace::PlatformAcceptanceContract::Phase1V1
                .id()
                .into(),
            source_binding: manifest.source_binding()?,
            semantic_digest: graph_digest(revision)?,
            semantic_context: context_digest(revision)?,
            closure_digest: ContentDigest(
                revision
                    .producer_closure()
                    .expect("validated closure")
                    .digest(),
            ),
        });
    }
    profile.phase("validation_receipt_identity");
    // A cache is optional: unsupported archive boundaries must not prevent
    // committing otherwise validated, reconstructible durable source.
    if let Some(validated) = validated {
        let cache = validated.semantic_cache();
        profile.phase("semantic_cache_frontier_export");
        if let Ok(cache) = cache
            && let Some(bytes) = cache_codec::encode(&cache)
        {
            let content_digest = ContentDigest::of(&bytes);
            manifest.semantic_cache = Some(SemanticCacheReference {
                format: cache_codec::FORMAT.into(),
                content_digest,
                source_binding: manifest.source_binding()?,
                semantic_context: ContentDigest(cache.source.context_contract_digest),
                closure_digest: ContentDigest(cache.source.semantic_closure_digest),
            });
            blobs.insert(content_digest, bytes);
        }
        profile.phase("semantic_cache_encode_and_digest");
    }
    let candidate = CandidateRevision { manifest, blobs };
    candidate.verify()?;
    profile.phase("durable_candidate_integrity_check");
    Ok(candidate)
}
fn verify_restored_documents(
    manifest: &RevisionManifest,
    revision: &WorkingProjectRevision,
) -> Result<(), ServiceError> {
    if revision.project() != manifest.project_id
        || revision.revision() != manifest.revision_id
        || revision.parent() != manifest.parent_revision_id
        || revision.documents().count() != manifest.documents.len()
    {
        return Err(ServiceError::Invalid(
            "checkpoint/manifest identity mismatch".into(),
        ));
    }
    for expected in &manifest.documents {
        let actual = revision
            .document_at(&expected.path)
            .ok_or_else(|| ServiceError::Invalid("missing checkpoint document".into()))?;
        if actual.id() != expected.document_id
            || actual.revision() != expected.source_revision_id
            || actual.language() != expected.language
            || ContentDigest::of(actual.source().as_bytes()) != expected.content_digest
        {
            return Err(ServiceError::Invalid(
                "checkpoint/source binding mismatch".into(),
            ));
        }
    }
    Ok(())
}
