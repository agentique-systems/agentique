//! Durable source identity, independent of semantic caches or accepted handles.
use super::*;
use agq_kernel::DeclaredIdentityCheckpoint;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Identity/shape carrier for one document. Bytes are supplied by the source store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentIdentityCheckpoint {
    /// Stable authored document identity, independent of its path label.
    pub document_id: DocumentId,
    /// Exact immutable source version parsed for this revision.
    pub source_revision_id: SourceRevisionId,
    /// User-visible label retained for source navigation, not semantic identity.
    pub path: String,
    /// Parser/lowering language selected for the authored bytes.
    pub language: SourceLanguage,
    /// SHA-256 of the separately stored exact UTF-8 source bytes.
    pub content_digest: [u8; 32],
    /// Checked production shapes and reconciled syntax identities.
    pub syntax_nodes: Vec<syntax::production::NodeIdentity>,
}

/// Versioned source identity format, not a serialized semantic model.
/// The authenticated publications and source blobs are external immutable inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIdentityCheckpoint {
    /// Representation version; currently one.
    pub format_version: u32,
    /// Stable source project identity.
    pub project_id: ProjectId,
    /// Canonical authored root identity.
    pub root: ElementId,
    /// Accepted KerML semantic publication digest.
    pub accepted_kerml: [u8; 32],
    /// Accepted SysML Systems publication digest.
    pub accepted_sysml: [u8; 32],
    /// Exact authored document population, excluding standard library sources.
    pub documents: Vec<DocumentIdentityCheckpoint>,
    /// Active and retired declared identity reservations; contains no graph values.
    pub identity_history: DeclaredIdentityCheckpoint,
    /// Source origins used to retire deleted syntax identities on subsequent edits.
    pub identity_sources: Vec<(FactKey, SourceOrigin)>,
}

/// A source checkpoint failed authentication or ordinary source reconstruction.
#[derive(Debug, thiserror::Error)]
pub enum SourceCheckpointError {
    #[error("source checkpoint does not match {0}")]
    Mismatch(&'static str),
    #[error(transparent)]
    Syntax(#[from] SourceError),
    #[error(transparent)]
    Kernel(#[from] agq_kernel::ModelError),
    #[error(transparent)]
    Build(#[from] LibraryLoadError),
    #[error(transparent)]
    Archive(Box<agq_kernel::archive::ArchiveError>),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl From<agq_kernel::archive::ArchiveError> for SourceCheckpointError {
    fn from(error: agq_kernel::archive::ArchiveError) -> Self {
        Self::Archive(Box::new(error))
    }
}

impl SourceCompilation {
    /// Export source identities and reservations without graphs or source bytes.
    pub fn identity_checkpoint(&self) -> SourceIdentityCheckpoint {
        SourceIdentityCheckpoint {
            format_version: 1,
            project_id: self.inputs.project,
            root: self.inputs.root,
            accepted_kerml: self
                .inputs
                .accepted_sysml()
                .accepted_kerml()
                .semantic_digest(),
            accepted_sysml: self.inputs.accepted_sysml().publication_digest(),
            documents: self
                .inputs
                .documents()
                .map(|(path, document)| DocumentIdentityCheckpoint {
                    document_id: document.id(),
                    source_revision_id: document.revision(),
                    path: path.into(),
                    language: document.language(),
                    content_digest: Sha256::digest(document.source().as_bytes()).into(),
                    syntax_nodes: document
                        .production_syntax()
                        .expect("workspace production arena")
                        .identity_checkpoint(),
                })
                .collect(),
            identity_history: self.history.identity_checkpoint(),
            identity_sources: self
                .identities
                .iter()
                .map(|(fact, origin)| (*fact, origin.clone()))
                .collect(),
        }
    }
}

impl SourceIdentityCheckpoint {
    /// Content identity of the deliberate identity representation and source digests.
    pub fn digest(&self) -> Result<[u8; 32], serde_json::Error> {
        Ok(Sha256::digest(serde_json::to_vec(self)?).into())
    }
    /// Reparse authenticated bytes, restore checked syntax/retirement identities,
    /// and perform full authored reconstruction. This creates a Working result.
    pub fn restore(
        &self,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        sources: &BTreeMap<DocumentId, String>,
    ) -> Result<SourceCompilation, SourceCheckpointError> {
        self.restore_maybe_cached(publication, sources, None)
    }
    /// Restore authenticated local effective facts, then rerun producer and audit
    /// validation. Any mismatch is an error; the caller may rebuild from source.
    pub fn restore_cached(
        &self,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        sources: &BTreeMap<DocumentId, String>,
        cache: &SourceSemanticCache,
    ) -> Result<SourceCompilation, SourceCheckpointError> {
        if cache.format_version != 1 || cache.source_identity_digest != self.digest()? {
            return Err(SourceCheckpointError::Mismatch(
                "semantic cache source identity or format",
            ));
        }
        let digest: [u8; 32] = Sha256::digest(&cache.kernel_frontier).into();
        if digest != cache.kernel_frontier_digest {
            return Err(SourceCheckpointError::Mismatch(
                "semantic cache archive checksum",
            ));
        }
        self.restore_maybe_cached(publication, sources, Some(cache))
    }
    fn restore_maybe_cached(
        &self,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        sources: &BTreeMap<DocumentId, String>,
        cache: Option<&SourceSemanticCache>,
    ) -> Result<SourceCompilation, SourceCheckpointError> {
        use SourceCheckpointError::Mismatch;
        if self.format_version != 1 {
            return Err(Mismatch("format version"));
        }
        if self.accepted_kerml != publication.accepted_kerml().semantic_digest()
            || self.accepted_sysml != publication.publication_digest()
        {
            return Err(Mismatch("accepted publication identities"));
        }
        if sources.len() != self.documents.len() {
            return Err(Mismatch("source population"));
        }
        let mut inputs = SourceInputs::with_accepted_sysml(publication)?;
        inputs.project = self.project_id;
        inputs.root = self.root;
        inputs.documents_reparsed = self.documents.len();
        let mut document_ids = BTreeSet::new();
        let mut syntax_ids = BTreeSet::new();
        for saved in &self.documents {
            if !document_ids.insert(saved.document_id) {
                return Err(Mismatch("duplicate document identity"));
            }
            if saved
                .syntax_nodes
                .iter()
                .any(|node| !syntax_ids.insert(node.id))
            {
                return Err(Mismatch("duplicate syntax identity"));
            }
            let source = sources
                .get(&saved.document_id)
                .ok_or(Mismatch("source blob"))?;
            if <[u8; 32]>::from(Sha256::digest(source.as_bytes())) != saved.content_digest {
                return Err(Mismatch("source content digest"));
            }
            let limits = syntax::production::Limits {
                source: inputs.limits,
                ..Default::default()
            };
            let parsed = match saved.language {
                SourceLanguage::KerMl => syntax::production::parse(
                    saved.document_id,
                    saved.source_revision_id,
                    source.as_str(),
                    limits,
                )?,
                SourceLanguage::SysMl => syntax::production::parse_sysml_with_profile(
                    inputs.dependency.syntax_profile(),
                    saved.document_id,
                    saved.source_revision_id,
                    source.as_str(),
                    limits,
                )?,
            }
            .restore_identities(&saved.syntax_nodes)?;
            let document = Arc::new(ProjectDocument {
                id: saved.document_id,
                revision: saved.source_revision_id,
                language: saved.language,
                source: source.as_str().into(),
                syntax: None,
                production: Some(parsed),
            });
            if inputs
                .documents
                .insert(saved.path.clone(), document)
                .is_some()
            {
                return Err(Mismatch("duplicate path label"));
            }
        }
        let history = DeclaredConstructionHistory::restore_identities(
            &inputs.dependency.mounted.project_snapshot(),
            &self.identity_history,
        )?;
        let ledger: LibrarySourceMap = self.identity_sources.iter().cloned().collect();
        if ledger.len() != self.identity_sources.len() {
            return Err(Mismatch("duplicate identity origin"));
        }
        Ok(Arc::new(inputs).compile_with_history(None, Some((history, ledger)), false, cache)?)
    }
}
