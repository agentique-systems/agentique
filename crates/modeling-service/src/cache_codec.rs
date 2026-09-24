//! Bounded, disposable cache encoding. This is not a repository truth format.
use agq_kerml_text::SourceSemanticCache;
use agq_modeling_repository::ContentDigest;
use agq_modeling_workspace::ProjectSemanticCache;
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub(crate) const FORMAT: &str = "agq-project-semantic-cache/2";
const LEGACY_FORMAT: &str = "agq-project-semantic-cache/1";
const METADATA: &str = "metadata.json";
const FRONTIER: &str = "kernel-frontier.jsonl";
const MAX_METADATA: u64 = 64 * 1024;
const MAX_CACHE: u64 = 512 * 1024 * 1024;

pub(crate) fn encode(cache: &ProjectSemanticCache) -> Option<Vec<u8>> {
    if cache.source.kernel_frontier.len() as u64 > MAX_CACHE || !valid(cache) {
        return None;
    }
    // Copy only small identity metadata; never clone the frontier or turn its
    // bytes into a decimal JSON array. Fixed entry order/timestamps are reproducible.
    let source = &cache.source;
    let metadata = ProjectSemanticCache {
        format_version: cache.format_version,
        project_revision_id: cache.project_revision_id,
        source: SourceSemanticCache {
            format_version: source.format_version,
            source_identity_digest: source.source_identity_digest,
            model_digest: source.model_digest,
            context_contract_digest: source.context_contract_digest,
            producer_registry_digest: source.producer_registry_digest,
            semantic_closure_digest: source.semantic_closure_digest,
            sysml_dependency_digest: source.sysml_dependency_digest,
            kernel_frontier_digest: source.kernel_frontier_digest,
            kernel_frontier: Vec::new(),
        },
    };
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(1))
        .last_modified_time(zip::DateTime::default());
    archive.start_file(METADATA, options).ok()?;
    serde_json::to_writer(&mut archive, &metadata).ok()?;
    archive.start_file(FRONTIER, options).ok()?;
    archive.write_all(&source.kernel_frontier).ok()?;
    let bytes = archive.finish().ok()?.into_inner();
    (bytes.len() as u64 <= MAX_CACHE).then_some(bytes)
}

pub(crate) fn decode(format: &str, bytes: &[u8]) -> Option<ProjectSemanticCache> {
    if bytes.len() as u64 > MAX_CACHE {
        return None;
    }
    let cache = match format {
        LEGACY_FORMAT => serde_json::from_slice(bytes).ok()?,
        FORMAT => {
            let mut archive = ZipArchive::new(Cursor::new(bytes)).ok()?;
            if archive.len() != 2 {
                return None;
            }
            let metadata = read_entry(&mut archive, 0, METADATA, MAX_METADATA)?;
            let mut cache: ProjectSemanticCache = serde_json::from_slice(&metadata).ok()?;
            if !cache.source.kernel_frontier.is_empty() {
                return None;
            }
            cache.source.kernel_frontier = read_entry(&mut archive, 1, FRONTIER, MAX_CACHE)?;
            cache
        }
        _ => return None,
    };
    valid(&cache).then_some(cache)
}

fn read_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    index: usize,
    name: &str,
    limit: u64,
) -> Option<Vec<u8>> {
    let entry = archive.by_index(index).ok()?;
    if entry.name() != name || entry.size() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    entry.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

fn valid(cache: &ProjectSemanticCache) -> bool {
    cache.format_version == 1
        && cache.source.format_version == 1
        && ContentDigest::of(&cache.source.kernel_frontier).0 == cache.source.kernel_frontier_digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_modeling_workspace::ProjectRevisionId;

    fn fixture() -> ProjectSemanticCache {
        let frontier = b"{\"fact\":\"exact source-bound frontier\"}\n".repeat(200);
        ProjectSemanticCache {
            format_version: 1,
            project_revision_id: ProjectRevisionId::from_u128(42),
            source: SourceSemanticCache {
                format_version: 1,
                source_identity_digest: [1; 32],
                model_digest: [2; 32],
                context_contract_digest: [3; 32],
                producer_registry_digest: [4; 32],
                semantic_closure_digest: [5; 32],
                sysml_dependency_digest: [6; 32],
                kernel_frontier_digest: ContentDigest::of(&frontier).0,
                kernel_frontier: frontier,
            },
        }
    }

    #[test]
    fn compact_cache_roundtrip_is_exact_and_deterministic() {
        let cache = fixture();
        let encoded = encode(&cache).unwrap();
        assert_eq!(encoded, encode(&cache).unwrap());
        assert!(encoded.len() < cache.source.kernel_frontier.len());
        assert_eq!(
            serde_json::to_vec(&decode(FORMAT, &encoded).unwrap()).unwrap(),
            serde_json::to_vec(&cache).unwrap()
        );
    }

    #[test]
    fn legacy_cache_remains_readable_and_invalid_caches_are_discarded() {
        let mut cache = fixture();
        let legacy = serde_json::to_vec(&cache).unwrap();
        assert_eq!(
            serde_json::to_vec(&decode(LEGACY_FORMAT, &legacy).unwrap()).unwrap(),
            legacy
        );
        assert!(decode("future", &legacy).is_none());
        assert!(decode(FORMAT, &legacy).is_none());
        let encoded = encode(&cache).unwrap();
        assert!(decode(FORMAT, &encoded[..encoded.len() / 2]).is_none());
        cache.source.kernel_frontier[0] ^= 1;
        assert!(encode(&cache).is_none());
        assert!(decode(LEGACY_FORMAT, &serde_json::to_vec(&cache).unwrap()).is_none());
        cache = fixture();
        cache.source.format_version = 2;
        assert!(encode(&cache).is_none());
        assert!(decode(LEGACY_FORMAT, &serde_json::to_vec(&cache).unwrap()).is_none());
    }

    #[test]
    fn archive_entry_bounds_names_and_frontier_digest_are_checked() {
        for (name, metadata, frontier) in [
            (METADATA, vec![b' '; MAX_METADATA as usize + 1], vec![]),
            ("wrong.json", b"{}".to_vec(), vec![]),
            (METADATA, b"not JSON".to_vec(), vec![]),
            (METADATA, serde_json::to_vec(&fixture()).unwrap(), vec![]),
            (
                METADATA,
                {
                    let mut cache = fixture();
                    cache.source.kernel_frontier.clear();
                    serde_json::to_vec(&cache).unwrap()
                },
                b"tampered".to_vec(),
            ),
        ] {
            let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(&metadata).unwrap();
            zip.start_file(FRONTIER, SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&frontier).unwrap();
            let bytes = zip.finish().unwrap().into_inner();
            assert!(decode(FORMAT, &bytes).is_none());
        }
    }

    #[test]
    #[ignore = "offline codec measurement of an explicitly supplied legacy cache; no semantic acceptance claim"]
    fn characterize_legacy_cache_encoding() {
        let path = std::env::var_os("AGENTIQUE_LEGACY_CACHE_FILE").expect("legacy cache file");
        let bytes = std::fs::read(path).unwrap();
        let legacy_bytes = bytes.len();
        let start = std::time::Instant::now();
        let mut original = decode(LEGACY_FORMAT, &bytes).unwrap();
        let legacy_decode_ms = start.elapsed().as_millis();
        drop(bytes);
        let start = std::time::Instant::now();
        let encoded = encode(&original).unwrap();
        let encode_ms = start.elapsed().as_millis();
        let start = std::time::Instant::now();
        let mut restored = decode(FORMAT, &encoded).unwrap();
        let decode_ms = start.elapsed().as_millis();
        let original_frontier = std::mem::take(&mut original.source.kernel_frontier);
        let restored_frontier = std::mem::take(&mut restored.source.kernel_frontier);
        assert_eq!(original_frontier, restored_frontier);
        assert_eq!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&restored).unwrap()
        );
        eprintln!(
            "codec_legacy_bytes={legacy_bytes} raw_frontier_bytes={} v2_bytes={} legacy_decode_ms={legacy_decode_ms} v2_encode_ms={encode_ms} v2_decode_ms={decode_ms} v2_digest={}",
            original_frontier.len(),
            encoded.len(),
            ContentDigest::of(&encoded).hex()
        );
    }
}
