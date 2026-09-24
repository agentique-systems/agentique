//! Source-backed revision checkpoints; repository receipts remain an outer concern.
use super::*;
use agq_kerml_text::{SourceCheckpointError, SourceIdentityCheckpoint};
use serde::{Deserialize, Serialize};

/// Explicit versioned revision representation without graphs or source bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRevisionCheckpoint {
    pub format_version: u32,
    pub project_revision_id: ProjectRevisionId,
    pub parent_revision_id: Option<ProjectRevisionId>,
    pub source: SourceIdentityCheckpoint,
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
    #[error(transparent)]
    Source(#[from] SourceCheckpointError),
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
