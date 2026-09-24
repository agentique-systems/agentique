//! Source-backed revision checkpoints; repository receipts remain an outer concern.
use super::*;
use agq_kerml_text::{SourceCheckpointError, SourceIdentityCheckpoint, SourceSemanticCache};
use serde::{Deserialize, Serialize};

/// Explicit versioned revision representation without graphs or source bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRevisionCheckpoint {
    pub format_version: u32,
    pub project_revision_id: ProjectRevisionId,
    pub parent_revision_id: Option<ProjectRevisionId>,
    pub source: SourceIdentityCheckpoint,
}

/// Disposable authored effective-graph cache bound to an exact project revision.
/// Source checkpoints and authenticated publications remain required to restore it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSemanticCache {
    pub format_version: u32,
    pub project_revision_id: ProjectRevisionId,
    pub source: SourceSemanticCache,
}

/// Stable graph/provenance and semantic-contract identities, independent of the
/// fresh kernel revision allocated during a source rebuild.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFingerprint {
    pub model_digest: [u8; 32],
    pub context_contract_digest: [u8; 32],
    pub closure_digest: Option<[u8; 32]>,
    pub accepted_kerml: [u8; 32],
    pub accepted_sysml: [u8; 32],
}

/// Checkpoint authentication or ordinary source reconstruction failed.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("unsupported project checkpoint version {0}")]
    Version(u32),
    #[error("semantic cache revision or format does not match the source checkpoint")]
    CacheIdentity,
    #[error(transparent)]
    Source(#[from] SourceCheckpointError),
}

impl ValidatedProjectRevision {
    /// Export local semantic cache bytes only after actual platform validation.
    /// Restoring these bytes still returns Working and repeats semantic audits.
    pub fn semantic_cache(&self) -> Result<ProjectSemanticCache, CheckpointError> {
        Ok(ProjectSemanticCache {
            format_version: 1,
            project_revision_id: self.revision(),
            source: self.working.compilation.semantic_cache()?,
        })
    }
}

impl ProjectRevision {
    /// Export deliberate source identity data; no accepted semantic cache is implied.
    pub fn checkpoint(&self) -> ProjectRevisionCheckpoint {
        ProjectRevisionCheckpoint {
            format_version: 1,
            project_revision_id: self.revision,
            parent_revision_id: self.parent,
            source: self.compilation.identity_checkpoint(),
        }
    }
    /// Fingerprint the canonical graph, provenance and actual semantic contract.
    pub fn semantic_fingerprint(&self) -> Result<SemanticFingerprint, QueryUnavailable> {
        let queries = self.kerml_queries()?;
        Ok(SemanticFingerprint {
            model_digest: queries.context().model_digest,
            context_contract_digest: queries.context().closure_contract_digest(),
            closure_digest: self
                .producer_closure()
                .map(|certificate| certificate.digest()),
            accepted_kerml: self.accepted_kerml().semantic_digest(),
            accepted_sysml: self.accepted_sysml().publication_digest(),
        })
    }
}

impl ProjectRevisionCheckpoint {
    /// Authenticate a cache against exact source, accepted language dependencies
    /// and closure identities, then rerun producer and effective-query audits.
    /// Callers may discard any failed cache and use ordinary source restoration.
    pub fn restore_cached(
        &self,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        sources: &BTreeMap<DocumentId, String>,
        cache: &ProjectSemanticCache,
    ) -> Result<Arc<WorkingProjectRevision>, CheckpointError> {
        if self.format_version != 1 {
            return Err(CheckpointError::Version(self.format_version));
        }
        if cache.format_version != 1 || cache.project_revision_id != self.project_revision_id {
            return Err(CheckpointError::CacheIdentity);
        }
        Ok(Arc::new(WorkingProjectRevision {
            revision: ProjectRevision {
                revision: self.project_revision_id,
                parent: self.parent_revision_id,
                compilation: self
                    .source
                    .restore_cached(publication, sources, &cache.source)?,
            },
        }))
    }

    /// Full source reconstruction with preserved semantic and source identities.
    /// A stored validation claim does not create a Validated handle here.
    pub fn restore(
        &self,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        sources: &BTreeMap<DocumentId, String>,
    ) -> Result<Arc<WorkingProjectRevision>, CheckpointError> {
        if self.format_version != 1 {
            return Err(CheckpointError::Version(self.format_version));
        }
        Ok(Arc::new(WorkingProjectRevision {
            revision: ProjectRevision {
                revision: self.project_revision_id,
                parent: self.parent_revision_id,
                compilation: self.source.restore(publication, sources)?,
            },
        }))
    }
}
