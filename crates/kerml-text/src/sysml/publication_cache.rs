//! Export of already accepted Systems publications. Cache bytes confer no trust.
use super::*;
use serde_json::{Value, json};
use std::io::{self, Seek, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

/// I/O or identity failure while exporting an accepted Systems publication.
#[derive(Debug, thiserror::Error)]
pub enum SystemsPublicationCacheError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Graph(#[from] agq_kernel::archive::ArchiveError),
    #[error("accepted Systems publication export mismatch: {0}")]
    Mismatch(&'static str),
}

impl CanonicalSysmlSystemsLibrary {
    /// Reject stale anchors or interpretation identities against this exact
    /// accepted publication and the independently verified original sources.
    pub fn check_binding_manifest(
        &self,
        sources: &VerifiedLibrarySet,
        manifest: &Value,
    ) -> Result<(), SystemsPublicationCacheError> {
        if *manifest != self.binding_manifest(sources)? {
            return Err(SystemsPublicationCacheError::Mismatch("binding manifest"));
        }
        Ok(())
    }

    /// Accepted algorithmic anchors, including source and publication identities.
    /// This cannot be generated from an unpublished candidate.
    pub fn binding_manifest(
        &self,
        sources: &VerifiedLibrarySet,
    ) -> Result<Value, SystemsPublicationCacheError> {
        self.check_export_sources(sources)?;
        let bindings = self
            .bindings
            .targets()
            .iter()
            .map(|(role, element)| {
                let (path, expected) = role.specification();
                let metaclass = self
                    .overlay
                    .model()
                    .element(*element)
                    .ok_or(SystemsPublicationCacheError::Mismatch("binding element"))?
                    .metaclass();
                let source = self
                    .bindings
                    .declaration_sources()
                    .get(element)
                    .ok_or(SystemsPublicationCacheError::Mismatch("binding source"))?;
                let document = sources
                    .documents()
                    .find(|document| document.document() == source.document)
                    .ok_or(SystemsPublicationCacheError::Mismatch("binding document"))?;
                Ok(json!({
                    "role":format!("{role:?}"),
                    "element":element,
                    "library":self.bindings.identity().library,
                    "qualified_path":path,
                    "metaclass":metaclass,
                    "expected_metaclass":expected,
                    "visibility":"public",
                    "source":source,
                    "source_path":document.path(),
                    "source_sha256":document.sha256(),
                }))
            })
            .collect::<Result<Vec<_>, SystemsPublicationCacheError>>()?;
        Ok(json!({
            "format":"agq-sysml-accepted-bindings/1",
            "operational_profile":self.identity.dependencies.sysml_profile.id(),
            "rule_set":self.identity.dependencies.sysml_rule_set,
            "source_content_set":sources.content_set_id(),
            "systems_library":self.bindings.identity().library,
            "systems_kpar":self.bindings.identity().artifact_sha256,
            "systems_source_content_set":self.bindings.identity().source_content_set,
            "accepted_kerml_digest":self.accepted_kerml.semantic_digest(),
            "accepted_systems_digest":self.publication_digest(),
            "semantic_digest":self.semantic_digest(),
            "producer_registry_digest":self.producer_closure.registry_digest(),
            "producer_closure_digest":self.producer_closure.digest(),
            "bindings":bindings,
        }))
    }

    /// Stream local graph deltas and immutable closure evidence. The KerML graph
    /// remains an external, exact dependency. The returned receipt must be pinned
    /// independently before any future trusted restoration can be enabled.
    pub fn write_cache(
        &self,
        writer: impl Write + Seek,
        sources: &VerifiedLibrarySet,
    ) -> Result<Value, SystemsPublicationCacheError> {
        let bindings = self.binding_manifest(sources)?;
        let metadata = json!({
            "format":"agq-sysml-publication-facade/1",
            "source_content_set":sources.content_set_id(),
            "roots":self.roots,
            "source_map":self.source_map.iter().collect::<Vec<_>>(),
            "documents":self.documents.iter().map(|document|json!({
                "path":document.path,"document":document.document,
                "sha256":document.source_sha256,"profile":document.profile.id(),
                "parsed":document.parsed,"byte_exact":document.byte_exact,
                "recovery_count":document.recovery_count,
                "production_count":document.production_count,
                "construction_gap":document.construction_gap,
            })).collect::<Vec<_>>(),
            "mandatory_references":self.audit.mandatory_references,
            "complete_references":self.audit.complete_references,
            "checked":self.audit.checked.iter().map(|(family,count)|
                (format!("{family:?}"),*count)).collect::<BTreeMap<_,_>>(),
        });
        let closure = self.producer_closure.receipt_value();
        let mut archive = ZipWriter::new(writer);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .large_file(true);
        let mut entries = BTreeMap::new();
        for (name, value) in [("facade.json", &metadata), ("closure.json", &closure)] {
            archive.start_file(name, options)?;
            let mut digest = DigestWriter::new(&mut archive);
            serde_json::to_writer(&mut digest, value)?;
            entries.insert(name, digest.identity());
        }
        archive.start_file("kernel.jsonl", options)?;
        let mut digest = DigestWriter::new(&mut archive);
        agq_kernel::archive::write_dependent_overlay(&self.overlay, &mut digest)?;
        entries.insert("kernel.jsonl", digest.identity());
        archive.finish()?.flush()?;
        Ok(json!({
            "format":"agq-sysml-accepted-publication/1",
            "status":"accepted",
            "source_content_set":sources.content_set_id(),
            "binding_manifest_sha256":json_digest(&bindings)?,
            "entries":entries,
            "identity":{
                "publication_digest":self.publication_digest(),
                "semantic_digest":self.semantic_digest(),
                "dependency_contract_digest":self.identity.dependencies.context_identity_digest(),
                "accepted_kerml_digest":self.accepted_kerml.semantic_digest(),
                "systems_kpar":self.bindings.identity().artifact_sha256,
                "systems_source_content_set":self.bindings.identity().source_content_set,
                "operational_profile":self.identity.dependencies.sysml_profile.id(),
                "rule_set":self.identity.dependencies.sysml_rule_set,
                "grammar_compatibility_manifest":self.identity.dependencies.grammar_compatibility_manifest_digest,
                "semantic_correction_manifest":self.identity.dependencies.semantic_correction_manifest_digest,
                "combined_descriptor_graph":self.identity.dependencies.combined_descriptor_digest,
                "producer_registry_digest":self.producer_closure.registry_digest(),
                "producer_closure_digest":self.producer_closure.digest(),
                "producer_context_contract_digest":self.producer_closure.context_contract_digest(),
            },
        }))
    }

    fn check_export_sources(
        &self,
        sources: &VerifiedLibrarySet,
    ) -> Result<(), SystemsPublicationCacheError> {
        if sources.content_set_id() != self.identity.dependencies.kerml_source_content_set
            || !self.bindings.sources_verified()
            || sources
                .libraries()
                .get(&self.bindings.identity().library)
                .is_none_or(|library| {
                    library.archive_sha256() != self.bindings.identity().artifact_sha256
                })
        {
            return Err(SystemsPublicationCacheError::Mismatch(
                "verified source identity",
            ));
        }
        Ok(())
    }
}

struct DigestWriter<W> {
    inner: W,
    digest: Sha256,
    bytes: u64,
}
impl<W: Write> DigestWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            digest: Sha256::new(),
            bytes: 0,
        }
    }
    fn identity(self) -> Value {
        let digest: [u8; 32] = self.digest.finalize().into();
        json!({"sha256":digest,"bytes":self.bytes})
    }
}
impl<W: Write> Write for DigestWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.digest.update(&bytes[..written]);
        self.bytes += written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
fn json_digest(value: &Value) -> Result<[u8; 32], serde_json::Error> {
    let mut writer = DigestWriter::new(io::sink());
    serde_json::to_writer(&mut writer, value)?;
    Ok(writer.digest.finalize().into())
}
