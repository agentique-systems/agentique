//! Reusable effective facts bound to durable source identity, never source truth.
use super::*;
use agq_kerml_semantics::SemanticContextId;
use agq_kernel::derived::DerivedOverlay;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Versioned local effective-graph cache. Accepted standards remain an external
/// authenticated dependency. Source, context and closure identities are checked;
/// every restored graph also passes ordinary producer and effective-query audits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceSemanticCache {
    /// Representation version; currently one.
    pub format_version: u32,
    /// Exact source identity checkpoint, including source-content digests.
    pub source_identity_digest: [u8; 32],
    /// Canonical effective graph and provenance identity.
    pub model_digest: [u8; 32],
    /// Semantic context independent of a fresh kernel revision label.
    pub context_contract_digest: [u8; 32],
    /// Exact combined KerML and SysML producer registry.
    pub producer_registry_digest: [u8; 32],
    /// Closure certificate semantic identity, reproduced on restore.
    pub semantic_closure_digest: [u8; 32],
    /// Accepted language profiles, descriptors, grammar and rule dependencies.
    pub sysml_dependency_digest: [u8; 32],
    /// SHA-256 of the local kernel frontier bytes.
    pub kernel_frontier_digest: [u8; 32],
    /// Versioned kernel cache archive, excluding immutable standard dependencies.
    pub kernel_frontier: Vec<u8>,
}

impl SourceCompilation {
    /// Export a strict, completely closed source graph as disposable cache data.
    /// This operation neither serializes source bytes nor creates a Validated handle.
    pub fn semantic_cache(&self) -> Result<SourceSemanticCache, SourceCheckpointError> {
        let Some(certificate) = self.producer_closure() else {
            return Err(SourceCheckpointError::Mismatch(
                "missing closure certificate",
            ));
        };
        let SourceFrontier::Strict(model) = &self.frontier else {
            return Err(SourceCheckpointError::Mismatch("non-strict source graph"));
        };
        if !self.diagnostics.is_empty()
            || !certificate.is_fully_closed(model.semantic_model())
            || self
                .effective_audit
                .as_ref()
                .is_none_or(|audit| !audit.report.findings.is_empty())
        {
            return Err(SourceCheckpointError::Mismatch(
                "unaccepted authored cache candidate",
            ));
        }
        let queries = self
            .kerml_queries()
            .map_err(|_| SourceCheckpointError::Mismatch("source context"))?;
        let kernel_frontier = crate::sysml::source::write_source_frontier(model)?;
        Ok(SourceSemanticCache {
            format_version: 1,
            source_identity_digest: self.identity_checkpoint().digest()?,
            model_digest: queries.context().model_digest,
            context_contract_digest: queries.context().closure_contract_digest(),
            producer_registry_digest: certificate.registry_digest(),
            semantic_closure_digest: certificate.digest(),
            sysml_dependency_digest: self
                .inputs
                .accepted_sysml()
                .identity()
                .dependencies
                .context_identity_digest(),
            kernel_frontier_digest: Sha256::digest(&kernel_frontier).into(),
            kernel_frontier,
        })
    }
}

impl SourceSemanticCache {
    pub(crate) fn restore_frontier(
        &self,
        declared: Snapshot,
        dependency: &AcceptedSourceDependency,
        root: ElementId,
    ) -> Result<DerivedOverlay, LibraryLoadError> {
        let mismatch = |field| {
            LibraryLoadError::Interpretation(format!("authored semantic cache mismatch: {field}"))
        };
        if self.format_version != 1
            || self.sysml_dependency_digest
                != dependency
                    .publication
                    .identity()
                    .dependencies
                    .context_identity_digest()
            || self.producer_registry_digest != dependency.registry().digest()
        {
            return Err(mismatch(
                "format or language/descriptor/producer dependencies",
            ));
        }
        // The existing kernel reader compares exact declarations, source origins,
        // ordered values and retired reservations against this source-derived input.
        // Decoded standard records cannot replace the protected dependency.
        let overlay = agq_kernel::archive::read_publication_frontier_on(
            std::io::Cursor::new(&self.kernel_frontier),
            declared,
        )
        .map_err(|error| LibraryLoadError::Interpretation(error.to_string()))?;
        let context = dependency
            .mounted
            .project_overlay_context(&overlay, &[root])
            .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?;
        if context.id().model_digest != self.model_digest
            || context.id().closure_contract_digest() != self.context_contract_digest
        {
            return Err(mismatch("canonical graph/provenance or semantic context"));
        }
        drop(context);
        Ok(overlay)
    }
    pub(crate) fn verify_closed(
        &self,
        context: &SemanticContextId,
        certificate: Option<&ProducerClosureCertificate>,
    ) -> Result<(), LibraryLoadError> {
        if context.model_digest != self.model_digest
            || context.closure_contract_digest() != self.context_contract_digest
            || certificate.is_none_or(|certificate| {
                certificate.registry_digest() != self.producer_registry_digest
                    || certificate.digest() != self.semantic_closure_digest
            })
        {
            return Err(LibraryLoadError::Interpretation(
                "authored semantic cache did not reproduce its graph and closure".into(),
            ));
        }
        Ok(())
    }
}
