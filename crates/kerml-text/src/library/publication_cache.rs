//! Persistence of an accepted facade; cached labels never establish acceptance.
use super::*;
use agq_kerml_semantics::{AcceptedPublicationReceipt, PublicationRestoreError};
use agq_kernel::provenance::{FactKey, SourceOrigin};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, BufReader, Read, Seek, Write};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const METADATA: &str = "facade.json";
const GRAPH: &str = "kernel.jsonl";

/// I/O or identity rejection while persisting an accepted standard-library graph.
#[derive(Debug, thiserror::Error)]
pub enum PublicationCacheError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Restoration(Box<PublicationRestoreError>),
    #[error(transparent)]
    Source(#[from] LibraryLoadError),
    #[error("kernel graph archive: {0}")]
    Graph(#[from] agq_kernel::archive::ArchiveError),
    #[error("accepted publication cache mismatch: {0}")]
    Mismatch(&'static str),
}
impl From<PublicationRestoreError> for PublicationCacheError {
    fn from(value: PublicationRestoreError) -> Self {
        Self::Restoration(Box::new(value))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacadeMetadata {
    format: String,
    source_content_set: String,
    roots: Vec<ElementId>,
    mandatory_references: usize,
    // JSON object keys cannot represent FactKey; canonical BTreeMap order is
    // retained in this sequence and duplicate entries are rejected on import.
    source_map: Vec<(FactKey, SourceOrigin)>,
}
impl FacadeMetadata {
    fn publication(publication: &CanonicalKermlStandardLibraries) -> Self {
        Self {
            format: "agq-kerml-publication-facade/1".into(),
            source_content_set: publication.source_content_set.clone(),
            roots: publication.roots.clone(),
            mandatory_references: publication.mandatory_references,
            source_map: publication
                .source_map
                .iter()
                .map(|(key, source)| (*key, source.clone()))
                .collect(),
        }
    }
    fn validate(&self, sources: &VerifiedLibrarySet) -> Result<(), PublicationCacheError> {
        if self.format != "agq-kerml-publication-facade/1"
            || self.source_content_set != sources.content_set_id()
        {
            return Err(PublicationCacheError::Mismatch(
                "source content set or facade format",
            ));
        }
        let documents: std::collections::BTreeMap<_, _> = sources
            .documents()
            .map(|document| (document.document(), document))
            .collect();
        for (_, source) in &self.source_map {
            let document = documents
                .get(&source.document)
                .ok_or(PublicationCacheError::Mismatch("source document"))?;
            if document.revision() != source.revision
                || document
                    .source()
                    .get(source.range.start() as usize..source.range.end() as usize)
                    .is_none()
            {
                return Err(PublicationCacheError::Mismatch(
                    "source revision or byte range",
                ));
            }
        }
        if self
            .source_map
            .windows(2)
            .any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(PublicationCacheError::Mismatch(
                "source map order or duplicate fact",
            ));
        }
        Ok(())
    }
}

impl CanonicalKermlStandardLibraries {
    /// Stream a compressed cache from this accepted publication. Returns a
    /// separate receipt to check in alongside its accepted binding manifest.
    /// Creating files does not activate restoration: both trusted files are
    /// compiled into the next build and checked before accepting a cache.
    pub fn write_cache(
        &self,
        writer: impl Write + Seek,
        sources: &VerifiedLibrarySet,
    ) -> Result<Value, PublicationCacheError> {
        let manifest = self.binding_manifest(sources)?;
        let metadata = FacadeMetadata::publication(self);
        metadata.validate(sources)?;
        let metadata = serde_json::to_value(metadata)?;
        let graph_receipt = self.write_cache_parts(writer, &metadata)?;
        let (metadata_digest, metadata_bytes) = json_identity(&metadata)?;
        Ok(json!({
            "format":"agq-kerml-accepted-publication/1",
            "status":"accepted",
            "source_content_set":self.source_content_set(),
            "binding_manifest_sha256":json_identity(&manifest)?.0,
            "facade_metadata_sha256":metadata_digest,
            "facade_metadata_bytes":metadata_bytes,
            "complete_overlay":graph_receipt,
        }))
    }

    fn write_cache_parts(
        &self,
        writer: impl Write + Seek,
        metadata: &Value,
    ) -> Result<Value, PublicationCacheError> {
        let mut archive = ZipWriter::new(writer);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .large_file(true);
        archive.start_file(METADATA, options)?;
        serde_json::to_writer(&mut archive, metadata)?;
        archive.start_file(GRAPH, options)?;
        let receipt = self.complete.write_accepted_archive(&mut archive)?;
        archive.finish()?.flush()?;
        Ok(receipt)
    }

    /// Restore only the exact publication pinned by the checked-in accepted
    /// receipt and binding manifest. No producer closure or corpus reparsing runs.
    pub fn restore_cache(
        reader: impl Read + Seek,
        sources: &VerifiedLibrarySet,
    ) -> Result<Self, PublicationCacheError> {
        let receipt = AcceptedPublicationReceipt::checked_in()?;
        if receipt.source_content_set() != sources.content_set_id() {
            return Err(PublicationCacheError::Mismatch(
                "verified source content set",
            ));
        }
        let mut archive = ZipArchive::new(reader)?;
        if archive.len() != 2 {
            return Err(PublicationCacheError::Mismatch("archive entries"));
        }
        let metadata_bytes = receipt.facade_metadata_bytes()?;
        let metadata_digest = entry_digest(&mut archive, METADATA, metadata_bytes)?;
        receipt.verify_facade_digest(metadata_digest)?;
        let metadata: Value =
            serde_json::from_reader(archive.by_name(METADATA)?.take(metadata_bytes))?;
        receipt.verify_facade_metadata(&metadata)?;
        let metadata: FacadeMetadata = serde_json::from_value(metadata)?;
        metadata.validate(sources)?;
        // Verify compressed input before allocating records. The semantics layer
        // also re-encodes and hashes the decoded graph before issuing Complete.
        receipt.verify_graph_digest(entry_digest(&mut archive, GRAPH, receipt.graph_bytes()?)?)?;
        let registry =
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).map_err(|error| {
                LibraryLoadError::Interpretation(format!("Cache registry: {error:?}"))
            })?;
        let overlay = agq_kernel::archive::read_overlay(
            BufReader::new(archive.by_name(GRAPH)?),
            Arc::new(registry),
        )?;
        let libraries = verified_libraries(sources)?;
        let complete = CompletePublicationOverlay::restore_accepted(
            overlay,
            &metadata.roots,
            &libraries,
            &receipt,
        )?;
        let publication = Self::from_restored_parts(complete, metadata);
        publication.check_binding_manifest(sources, receipt.binding_manifest())?;
        Ok(publication)
    }

    fn from_restored_parts(complete: CompletePublicationOverlay, metadata: FacadeMetadata) -> Self {
        Self {
            snapshot: complete.overlay().declared().clone(),
            shared_overlay: Arc::new(complete.overlay().clone()),
            complete,
            source_map: metadata.source_map.into_iter().collect(),
            roots: metadata.roots,
            mandatory_references: metadata.mandatory_references,
            source_content_set: metadata.source_content_set,
        }
    }
}

fn verified_libraries(
    sources: &VerifiedLibrarySet,
) -> Result<LibrarySetIdentity, PublicationCacheError> {
    use agq_kerml_semantics::{LibraryPin, StandardLibraryArtifact};
    let mut artifacts = std::collections::BTreeMap::new();
    let mut pins = std::collections::BTreeSet::new();
    for artifact in StandardLibraryArtifact::ALL {
        let library = sources
            .libraries()
            .values()
            .find(|library| library.resource() == artifact.resource())
            .ok_or(PublicationCacheError::Mismatch("standard library artifact"))?;
        let mut sha256 = [0; 32];
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&library.archive_sha256()[index * 2..index * 2 + 2], 16)
                .expect("verified digest");
        }
        artifacts.insert(artifact, library.id());
        pins.insert(LibraryPin {
            name: library.resource().into(),
            sha256,
        });
    }
    Ok(LibrarySetIdentity { artifacts, pins })
}

fn json_identity(value: &Value) -> Result<([u8; 32], u64), serde_json::Error> {
    let mut writer = DigestWriter {
        digest: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, value)?;
    Ok((writer.digest.finalize().into(), writer.bytes))
}
struct DigestWriter {
    digest: Sha256,
    bytes: u64,
}
impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.digest.update(bytes);
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn entry_digest<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    expected_bytes: u64,
) -> Result<[u8; 32], PublicationCacheError> {
    let entry = archive.by_name(name)?;
    if entry.size() != expected_bytes {
        return Err(PublicationCacheError::Mismatch("archive entry byte count"));
    }
    bounded_digest(entry, expected_bytes)
}
fn bounded_digest(
    reader: impl Read,
    expected_bytes: u64,
) -> Result<[u8; 32], PublicationCacheError> {
    let limit = expected_bytes
        .checked_add(1)
        .ok_or(PublicationCacheError::Mismatch(
            "archive entry size overflow",
        ))?;
    let mut bounded = reader.take(limit);
    let mut writer = DigestWriter {
        digest: Sha256::new(),
        bytes: 0,
    };
    io::copy(&mut bounded, &mut writer)?;
    if writer.bytes != expected_bytes {
        return Err(PublicationCacheError::Mismatch(
            "actual archive entry byte count",
        ));
    }
    Ok(writer.digest.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_kernel::{DocumentId, SourceRevisionId, SyntaxNodeId, provenance::ByteRange};
    use std::io::Cursor;

    #[test]
    fn compressed_facade_roundtrip_preserves_sources_roots_references_and_authored_readers() {
        let publication = super::super::tests::synthetic_publication();
        let mut metadata = FacadeMetadata::publication(&publication);
        metadata.mandatory_references = 17;
        metadata.source_map.push((
            FactKey::Element(publication.roots()[0]),
            SourceOrigin {
                document: DocumentId::from_u128(1),
                revision: SourceRevisionId::from_u128(2),
                range: ByteRange::new(3, 7).unwrap(),
                syntax_node: Some(SyntaxNodeId::from_u128(4)),
            },
        ));
        let metadata_json = serde_json::to_value(&metadata).unwrap();
        let mut bytes = Cursor::new(vec![]);
        let graph_receipt = publication
            .write_cache_parts(&mut bytes, &metadata_json)
            .unwrap();
        bytes.set_position(0);
        let mut archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(archive.len(), 2);
        let actual: Value = serde_json::from_reader(archive.by_name(METADATA).unwrap()).unwrap();
        assert_eq!(actual, metadata_json);
        let metadata: FacadeMetadata = serde_json::from_value(actual).unwrap();
        let overlay = agq_kernel::archive::read_overlay(
            BufReader::new(archive.by_name(GRAPH).unwrap()),
            Arc::new(agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap()),
        )
        .unwrap();
        // This private synthetic fixture cannot impersonate the checked-in
        // library receipt. Re-seal it through the actual tiny-fixture builder.
        // The semantics unit tests separately exercise trusted receipt minting.
        let complete = CanonicalPublicationBuilder::new(
            overlay.declared(),
            &metadata.roots,
            publication.library_set(),
        )
        .build(16, |_| {})
        .unwrap();
        assert_eq!(complete.context(), publication.context());
        let restored = Arc::new(CanonicalKermlStandardLibraries::from_restored_parts(
            complete, metadata,
        ));
        assert_eq!(restored.mandatory_reference_count(), 17);
        assert_eq!(restored.roots(), publication.roots());
        assert_eq!(restored.source_map().len(), 1);
        let source = &restored.source_map()[&FactKey::Element(publication.roots()[0])];
        assert_eq!((source.range.start(), source.range.end()), (3, 7));
        assert_eq!(source.syntax_node, Some(SyntaxNodeId::from_u128(4)));
        let first = crate::SourceProject::with_standard_libraries(restored.clone()).unwrap();
        let second = crate::SourceProject::with_standard_libraries(restored.clone()).unwrap();
        assert!(Arc::ptr_eq(
            first.standard_libraries().unwrap(),
            second.standard_libraries().unwrap()
        ));
        let mut graph = vec![];
        assert_eq!(
            restored
                .complete_overlay()
                .write_accepted_archive(&mut graph)
                .unwrap(),
            graph_receipt
        );
    }

    #[test]
    fn caller_cache_cannot_replace_the_checked_in_acceptance_receipt() {
        let sources = VerifiedLibrarySet::load_from_directory(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        )
        .unwrap();
        let publication = super::super::tests::synthetic_publication();
        let metadata = serde_json::to_value(FacadeMetadata::publication(&publication)).unwrap();
        let mut cache = Cursor::new(vec![]);
        publication
            .write_cache_parts(&mut cache, &metadata)
            .unwrap();
        cache.set_position(0);
        assert!(CanonicalKermlStandardLibraries::restore_cache(cache, &sources).is_err());
    }

    #[test]
    fn bounded_stream_hash_rejects_short_and_excess_input_without_reading_past_limit() {
        let bytes = b"0123456789";
        let mut cursor = Cursor::new(bytes);
        assert!(bounded_digest(&mut cursor, 3).is_err());
        assert_eq!(cursor.position(), 4);
        assert!(bounded_digest(Cursor::new(bytes), 11).is_err());
        assert_eq!(
            bounded_digest(Cursor::new(bytes), 10).unwrap(),
            <[u8; 32]>::from(Sha256::digest(bytes))
        );
        assert!(bounded_digest(Cursor::new(bytes), u64::MAX).is_err());
    }
}
