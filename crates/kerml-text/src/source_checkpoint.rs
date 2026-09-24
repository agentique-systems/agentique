//! Durable source identity, independent of semantic caches or accepted handles.
use super::*;
use agq_kernel::DeclaredIdentityCheckpoint;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Identity/shape carrier for one document. Bytes are supplied by the source store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentIdentityCheckpoint {
    pub document_id: DocumentId,
    pub source_revision_id: SourceRevisionId,
    pub path: String,
    pub language: SourceLanguage,
    pub content_digest: [u8; 32],
    pub syntax_nodes: Vec<syntax::production::NodeIdentity>,
}

/// Versioned source identity format, not a serialized semantic model.
/// The authenticated publications and source blobs are external immutable inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIdentityCheckpoint {
    pub format_version: u32,
    pub project_id: ProjectId,
    pub root: ElementId,
    pub accepted_kerml: [u8; 32],
    pub accepted_sysml: [u8; 32],
    pub documents: Vec<DocumentIdentityCheckpoint>,
    pub identity_history: DeclaredIdentityCheckpoint,
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
    /// Reparse authenticated bytes, restore checked syntax/retirement identities,
    /// and perform full authored reconstruction. This creates a Working result.
    pub fn restore(
        &self,
        publication: Arc<CanonicalSysmlSystemsLibrary>,
        sources: &BTreeMap<DocumentId, String>,
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
        Ok(Arc::new(inputs).compile_with_history(None, Some((history, ledger)))?)
    }
}
