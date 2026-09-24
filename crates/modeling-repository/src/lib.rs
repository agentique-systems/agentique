//! Storage-neutral immutable project history. Graphs remain workspace-owned.
//!
//! Adapters atomically persist a complete candidate and compare-and-set its head.
//! A manifest is reconstruction input, never an accepted semantic model.
#![forbid(unsafe_code)]

pub use agq_kerml_text::{ProjectId, SourceLanguage};
pub use agq_kernel::{DocumentId, SourceRevisionId};
pub use agq_modeling_workspace::ProjectRevisionId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! identity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(uuid::Uuid);
        impl $name {
            /// Allocate an identity independently of names and other identity domains.
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4())
            }
            /// Restore an externally allocated UUID.
            pub const fn from_uuid(id: uuid::Uuid) -> Self {
                Self(id)
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }
    };
}
identity!(
    RepositoryId,
    "Identity of one repository, independent of database path."
);
identity!(
    BranchId,
    "Identity of one named project head, never a revision identity."
);
identity!(
    OperationId,
    "Idempotency identity binding one exact durable operation."
);

/// SHA-256 identity of exact bytes; deserialization rejects malformed digests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentDigest(
    /// Exact SHA-256 bytes, encoded as canonical lowercase hexadecimal by serde.
    pub [u8; 32],
);
impl From<ContentDigest> for String {
    fn from(value: ContentDigest) -> Self {
        value.hex()
    }
}
impl TryFrom<String> for ContentDigest {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("expected canonical lowercase SHA-256".into());
        }
        let mut digest = [0; 32];
        for (index, byte) in digest.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|e| e.to_string())?;
        }
        Ok(Self(digest))
    }
}
impl ContentDigest {
    /// Hash bytes without normalization.
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }
    /// Lowercase hexadecimal representation for SQL and protocol projections.
    pub fn hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}
impl std::fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hex())
    }
}

/// Exact authenticated publication identities, not specification version labels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PublicationBinding {
    /// Accepted KerML publication digest.
    pub kerml: ContentDigest,
    /// Accepted Systems publication digest.
    pub sysml: ContentDigest,
}

/// One revision's source binding; the path is a label only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentManifest {
    /// Stable document identity.
    pub document_id: DocumentId,
    /// Project-local path label.
    pub path: String,
    /// Explicit source dialect.
    pub language: SourceLanguage,
    /// Exact source revision identity.
    pub source_revision_id: SourceRevisionId,
    /// Exact source bytes in the content-addressed store.
    pub content_digest: ContentDigest,
}

/// Evidence binding checked by the application on semantic restoration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReceipt {
    /// Versioned workspace acceptance contract, never whole-language conformance.
    pub acceptance_contract: String,
    /// Digest of the exact source/identity binding.
    pub source_binding: ContentDigest,
    /// Canonical semantic comparison digest.
    pub semantic_digest: ContentDigest,
    /// Exact effective interpretation context digest.
    pub semantic_context: ContentDigest,
    /// Exact checked producer closure certificate semantic digest.
    pub closure_digest: ContentDigest,
}

/// Persisted status; only workspace validation can create a validated handle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationState {
    /// Incomplete or unchecked authored semantics are retained as Working.
    Working,
    /// Receipt must authenticate against rebuilt semantics and checked validation.
    Validated(ValidationReceipt),
}

/// Optional, discardable artifact. No decoder may trust a partial identity match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCacheReference {
    /// Versioned cache format.
    pub format: String,
    /// Payload address.
    pub content_digest: ContentDigest,
    /// Exact source and identity binding.
    pub source_binding: ContentDigest,
    /// Profiles, descriptors, registry, rules, publications and interpretation context.
    pub semantic_context: ContentDigest,
    /// Producer closure semantic certificate identity.
    pub closure_digest: ContentDigest,
}

/// Deliberate repository format v1. Sources and identity checkpoint are mandatory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionManifest {
    /// Refuse unknown versions instead of guessing semantics.
    pub format_version: u32,
    /// Immutable history identity.
    pub revision_id: ProjectRevisionId,
    /// One same-project parent, or initial revision.
    pub parent_revision_id: Option<ProjectRevisionId>,
    /// Owning project identity.
    pub project_id: ProjectId,
    /// Protocol-neutral immutable revision metadata.
    pub metadata: ResourceMetadata,
    /// Exact authored inputs.
    pub documents: Vec<DocumentManifest>,
    /// Accepted standards remain external authenticated dependencies.
    pub accepted_publications: PublicationBinding,
    /// Versioned frontend reconciliation/identity checkpoint, separate from sources.
    pub checkpoint_digest: ContentDigest,
    /// Persisted Working/Validated distinction.
    pub validation: ValidationState,
    /// Discardable semantic acceleration artifact.
    pub semantic_cache: Option<SemanticCacheReference>,
}
impl RevisionManifest {
    /// Current durable format.
    pub const FORMAT_VERSION: u32 = 1;
    /// Hash the immutable manifest's deterministic JSON representation.
    pub fn digest(&self) -> Result<ContentDigest, RepositoryError> {
        Ok(ContentDigest::of(&serde_json::to_vec(self)?))
    }
    /// Bind sources, accepted standards and identity metadata independently of cache/status.
    pub fn source_binding(&self) -> Result<ContentDigest, RepositoryError> {
        Ok(ContentDigest::of(&serde_json::to_vec(&(
            self.format_version,
            self.revision_id,
            self.parent_revision_id,
            self.project_id,
            &self.documents,
            &self.accepted_publications,
            self.checkpoint_digest,
        ))?))
    }
    /// Storage-level checks only; semantic receipt verification belongs to the service.
    pub fn verify(&self) -> Result<(), RepositoryError> {
        if self.format_version != Self::FORMAT_VERSION {
            return Err(RepositoryError::UnsupportedFormat(self.format_version));
        }
        self.metadata.verify()?;
        if self.parent_revision_id == Some(self.revision_id) {
            return Err(RepositoryError::Integrity(
                "revision is its own parent".into(),
            ));
        }
        let mut documents = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut sources = BTreeSet::new();
        for document in &self.documents {
            if document.path.is_empty()
                || !documents.insert(document.document_id)
                || !paths.insert(&document.path)
                || !sources.insert(document.source_revision_id)
            {
                return Err(RepositoryError::Integrity(
                    "duplicate or empty document binding".into(),
                ));
            }
        }
        if let ValidationState::Validated(receipt) = &self.validation
            && (receipt.acceptance_contract
                != agq_modeling_workspace::PlatformAcceptanceContract::Phase1V1.id()
                || receipt.source_binding != self.source_binding()?)
        {
            return Err(RepositoryError::Integrity(
                "validation receipt source/contract mismatch".into(),
            ));
        }
        Ok(())
    }
    /// Mandatory blob addresses; caches are deliberately excluded.
    pub fn required_blobs(&self) -> BTreeSet<ContentDigest> {
        self.documents
            .iter()
            .map(|d| d.content_digest)
            .chain(std::iter::once(self.checkpoint_digest))
            .collect()
    }
}

/// Fully prepared immutable revision, with exact mandatory bytes for durable commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRevision {
    /// Versioned reconstruction manifest.
    pub manifest: RevisionManifest,
    /// Exact mandatory and optional cache bytes, deduplicated by content.
    pub blobs: BTreeMap<ContentDigest, Vec<u8>>,
}
impl CandidateRevision {
    /// Reject incomplete candidates and content-address mismatches before a transaction.
    pub fn verify(&self) -> Result<(), RepositoryError> {
        self.manifest.verify()?;
        for digest in self.manifest.required_blobs() {
            if !self.blobs.contains_key(&digest) {
                return Err(RepositoryError::Integrity(format!(
                    "missing candidate blob {digest}"
                )));
            }
        }
        for (digest, bytes) in &self.blobs {
            if *digest != ContentDigest::of(bytes) {
                return Err(RepositoryError::Integrity(format!(
                    "blob checksum mismatch {digest}"
                )));
            }
        }
        for document in &self.manifest.documents {
            std::str::from_utf8(&self.blobs[&document.content_digest])
                .map_err(|_| RepositoryError::Integrity("source is not UTF-8".into()))?;
        }
        Ok(())
    }
}

/// Repository project metadata, separate from semantic root elements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Stable identity matching its workspace project.
    pub id: ProjectId,
    /// Human-readable name.
    pub name: String,
    /// Initial/default branch identity.
    pub default_branch: BranchId,
    /// Creation and optional descriptive metadata.
    pub metadata: ResourceMetadata,
}
impl Project {
    /// Check the required display name and descriptive metadata before storage or use.
    pub fn verify(&self) -> Result<(), RepositoryError> {
        if self.name.trim().is_empty() {
            return Err(RepositoryError::Integrity("empty project name".into()));
        }
        self.metadata.verify()
    }
}
/// One named immutable-revision reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    /// Stable branch identity.
    pub id: BranchId,
    /// Owning project.
    pub project_id: ProjectId,
    /// Project-local branch name.
    pub name: String,
    /// Existing committed revision.
    pub head: ProjectRevisionId,
    /// Creation and optional descriptive metadata.
    pub metadata: ResourceMetadata,
}
impl Branch {
    /// Check the required display name and descriptive metadata before storage or use.
    pub fn verify(&self) -> Result<(), RepositoryError> {
        if self.name.trim().is_empty() {
            return Err(RepositoryError::Integrity("empty branch name".into()));
        }
        self.metadata.verify()
    }
}
/// Descriptive metadata projected by protocol adapters, never semantic graph facts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceMetadata {
    /// RFC 3339 creation timestamp.
    pub created: String,
    /// Optional display name (project/branch use their explicit name field).
    pub name: Option<String>,
    /// Optional descriptive text.
    pub description: Option<String>,
    /// Alternative human-readable identifiers.
    pub alias: Vec<String>,
}
impl ResourceMetadata {
    /// Require a complete RFC 3339 creation timestamp, preserving its exact spelling.
    pub fn verify(&self) -> Result<(), RepositoryError> {
        chrono::DateTime::parse_from_rfc3339(&self.created)
            .map_err(|_| RepositoryError::Integrity("creation timestamp is not RFC 3339".into()))?;
        Ok(())
    }
}
/// Initial project publication is atomic with its first revision and main branch.
#[derive(Clone, Debug)]
pub struct CreateProject {
    /// Idempotency key.
    pub operation_id: OperationId,
    /// Project metadata.
    pub project: Project,
    /// Initial head, whose identity matches the default branch.
    pub branch: Branch,
    /// Initial parentless candidate.
    pub initial: CandidateRevision,
}
/// Complete source-backed candidate plus explicit optimistic concurrency guard.
#[derive(Clone, Debug)]
pub struct CommitRevision {
    /// Stable idempotency key; reuse is valid only for this exact request.
    pub operation_id: OperationId,
    /// Owning project.
    pub project_id: ProjectId,
    /// Head to move atomically with revision registration.
    pub branch_id: BranchId,
    /// Expected current branch head; mismatches are explicit conflicts.
    pub expected_head: ProjectRevisionId,
    /// Candidate's parent must equal expected_head.
    pub candidate: CandidateRevision,
}
/// Acknowledgement issued only after a durable transaction (including idempotent retry).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitReceipt {
    /// Exact operation identity.
    pub operation_id: OperationId,
    /// Branch affected by the operation.
    pub branch_id: BranchId,
    /// Durably registered revision, not necessarily the branch's later current head.
    pub revision_id: ProjectRevisionId,
    /// Whether this is a replay of an already committed operation.
    pub replayed: bool,
}

/// Storage-level integrity results. Semantic authentication is a service operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityReport {
    /// Number of immutable revisions inspected.
    pub revisions_checked: usize,
    /// Number of unique blob addresses inspected.
    pub blobs_checked: usize,
    /// Fatal structural or checksum failures.
    pub errors: Vec<String>,
    /// Unusable optional caches do not invalidate durable source.
    pub discardable_caches: Vec<ProjectRevisionId>,
}
impl IntegrityReport {
    /// True only when all mandatory storage checks passed.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Neutral failures preserve optimistic conflicts and corruption distinctly.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// Named project, branch or revision is absent.
    #[error("not found: {0}")]
    NotFound(String),
    /// Stable identity or project-local branch name already exists.
    #[error("already exists: {0}")]
    AlreadyExists(String),
    /// Explicit head race; no retry or overwrite was performed.
    #[error("head conflict: expected {expected:?}, actual {actual:?}")]
    Conflict {
        /// Head the caller used to construct its candidate.
        expected: ProjectRevisionId,
        /// Head observed atomically by the repository.
        actual: ProjectRevisionId,
    },
    /// Operation identity was reused for different content.
    #[error("operation identity collision: {0}")]
    OperationCollision(OperationId),
    /// Transaction committed but acknowledgement was lost; retry the exact operation.
    #[error("durable operation outcome unknown; retry {0}")]
    OutcomeUnknown(OperationId),
    /// Mandatory durable data failed authentication.
    #[error("repository integrity: {0}")]
    Integrity(String),
    /// Future formats are refused.
    #[error("unsupported repository format: {0}")]
    UnsupportedFormat(u32),
    /// Adapter I/O/transaction failure; callers must not acknowledge a new head.
    #[error("storage failure: {0}")]
    Storage(String),
    /// Deterministic representation could not be encoded or decoded.
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}

/// Durable repository contract. Implementations must serialize CAS transactions.
///
/// Every returned manifest is immutable. All branches and parents stay within a
/// project. Deletion of a branch must never cascade into revision or blob deletion.
pub trait ModelingRepository: Send + Sync {
    /// Identity survives repository reopen.
    fn repository_id(&self) -> RepositoryId;
    /// Atomically register project, initial revision and initial branch.
    fn create_project(&self, request: &CreateProject) -> Result<CommitReceipt, RepositoryError>;
    /// Get project metadata.
    fn get_project(&self, project: ProjectId) -> Result<Project, RepositoryError>;
    /// List project metadata in stable identity order.
    fn list_projects(&self) -> Result<Vec<Project>, RepositoryError>;
    /// Create a named head at an existing revision without copying its graph.
    fn create_branch(&self, branch: &Branch) -> Result<(), RepositoryError>;
    /// Read branch metadata and its current head atomically.
    fn get_branch(&self, project: ProjectId, branch: BranchId) -> Result<Branch, RepositoryError>;
    /// List heads in stable identity order.
    fn list_branches(&self, project: ProjectId) -> Result<Vec<Branch>, RepositoryError>;
    /// Delete a non-default named head, retaining every revision and source blob.
    fn delete_branch(&self, project: ProjectId, branch: BranchId) -> Result<(), RepositoryError>;
    /// Read immutable reconstruction metadata.
    fn load_revision(
        &self,
        project: ProjectId,
        revision: ProjectRevisionId,
    ) -> Result<RevisionManifest, RepositoryError>;
    /// All retained revisions, including histories whose last branch was deleted.
    fn list_revisions(&self, project: ProjectId) -> Result<Vec<RevisionManifest>, RepositoryError>;
    /// Read exact bytes with mandatory content-address authentication.
    fn read_blob(&self, digest: ContentDigest) -> Result<Vec<u8>, RepositoryError>;
    /// Parent-first traversal from start, newest first; no implicit branch reread.
    fn list_revision_history(
        &self,
        project: ProjectId,
        start: ProjectRevisionId,
    ) -> Result<Vec<RevisionManifest>, RepositoryError>;
    /// One durable transaction registers candidate and conditionally moves the head.
    fn commit_revision(&self, request: &CommitRevision) -> Result<CommitReceipt, RepositoryError>;
    /// Check mandatory data independently of application graph queries.
    fn check_integrity(&self) -> Result<IntegrityReport, RepositoryError>;
}
