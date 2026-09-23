//! Explicit acceptance gate over original caches and compiled receipt authority.
//! Run this test alone; it never publishes or acquires standard libraries.
use agq_kerml_semantics::{
    PublicationCounters, SemanticClosureRequirement, TrustedPublicationError,
    TrustedPublicationReceipt,
};
use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries,
    sysml::{CanonicalSysmlSystemsLibrary, SystemsPublicationCacheError},
};
use agq_standard_libraries::VerifiedLibrarySet;
use agq_sysml_semantics::StandardSysmlRole;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

#[test]
#[ignore = "requires exact accepted KerML/Systems caches and activated compiled Systems receipt"]
fn accepted_systems_cache_roundtrip_and_tampering() {
    // Fail before the large KerML load when either input or authority is absent.
    let kerml_path = required_cache("AGENTIQUE_KERML_CACHE");
    let systems_path = required_cache("AGENTIQUE_SYSTEMS_CACHE");
    let trusted = TrustedPublicationReceipt::checked_in("sysml-systems-operational-v2")
        .expect("the requested gate requires the independently accepted compiled Systems receipt");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let scratch = Scratch::new(&root);
    let kerml = Arc::new(
        CanonicalKermlStandardLibraries::restore_cache(File::open(&kerml_path).unwrap(), &sources)
            .unwrap(),
    );
    let publication = restore(&systems_path, &sources, &kerml);
    assert_restored(&publication, &kerml);
    let identity = publication.identity().clone();
    let context = publication.context().clone();
    let bindings = publication.bindings().clone();
    let manifest = publication.binding_manifest(&sources).unwrap();
    assert_eq!(&manifest, trusted.binding_manifest());
    let certificate = publication.producer_closure().receipt_value();
    let selected_evidence = selected_reference_evidence(publication.overlay().model());
    assert!(
        publication
            .overlay()
            .model()
            .ordered_reference_contributions()
            .any(|((owner, _, _), _)| kerml.overlay().model().element(owner).is_none()),
        "accepted Systems retains local selected evidence"
    );
    let ids: Vec<_> = publication
        .overlay()
        .model()
        .elements()
        .map(|r| r.id())
        .collect();
    let roots = publication.roots().to_vec();
    let source_map = publication.source_map().clone();
    let roundtrip = scratch.path.join("roundtrip.zip");
    let mut output = File::create(&roundtrip).unwrap();
    let receipt = publication.write_cache(&mut output, &sources).unwrap();
    output.sync_all().unwrap();
    drop(output);
    assert_eq!(&receipt["identity"], trusted.identity());
    // Before any catalogue update, artifact issuance can only restore a
    // candidate by consuming an actual accepted facade as independent authority.
    let candidate = publication
        .verify_candidate_cache(
            File::open(&roundtrip).unwrap(),
            &sources,
            &receipt,
            &manifest,
        )
        .expect("live accepted facade authenticates exact candidate cache");
    assert_restored(&candidate, &kerml);
    assert_eq!(candidate.identity(), &identity);
    assert_eq!(candidate.context(), &context);
    assert_eq!(candidate.bindings(), &bindings);
    let mut changed_binding = manifest.clone();
    changed_binding["bindings"][0]["element"] =
        serde_json::json!(agq_kernel::ElementId::from_u128(0));
    assert!(
        candidate
            .verify_candidate_cache(
                File::open(&roundtrip).unwrap(),
                &sources,
                &receipt,
                &changed_binding,
            )
            .is_err(),
        "caller candidate binding cannot replace live accepted authority"
    );
    // Consuming verification keeps only one Systems graph and shared KerML.

    let restored = restore(&roundtrip, &sources, &kerml);
    assert_restored(&restored, &kerml);
    assert_eq!(restored.identity(), &identity);
    assert_eq!(restored.context(), &context);
    assert_eq!(restored.bindings(), &bindings);
    assert_eq!(restored.binding_manifest(&sources).unwrap(), manifest);
    assert_eq!(restored.producer_closure().receipt_value(), certificate);
    assert_eq!(
        selected_reference_evidence(restored.overlay().model()),
        selected_evidence
    );
    assert!(
        restored
            .overlay()
            .model()
            .elements()
            .map(|r| r.id())
            .eq(ids)
    );
    assert_eq!(restored.roots(), roots);
    assert_eq!(restored.source_map(), &source_map);
    let queries = restored.queries();
    assert!(std::ptr::eq(queries.model(), restored.overlay().model()));
    assert_eq!(queries.context(), &restored.context().kerml);
    drop(queries);
    // Check the separately authenticated manifest against actual accepted roles.
    for (field, changed) in [
        (
            "element",
            serde_json::json!(agq_kernel::ElementId::from_u128(0)),
        ),
        ("metaclass", serde_json::json!("changed-metaclass")),
        ("source_sha256", serde_json::json!("changed-source")),
    ] {
        let mut stale = manifest.clone();
        stale["bindings"][0][field] = changed;
        assert!(
            restored.check_binding_manifest(&sources, &stale).is_err(),
            "{field}"
        );
    }
    drop(restored);
    fs::remove_file(&roundtrip).unwrap();

    // Each case rewrites only a scratch file. Unchanged ZIP entries are copied
    // compressed, and altered entries stream through a bounded reader.
    let tampered = scratch.path.join("tampered.zip");
    for entry in ["facade.json", "closure.json", "kernel.jsonl"] {
        rewrite_archive(&systems_path, &tampered, Some(entry), false);
        let error = rejected(&tampered, &sources, &kerml, entry);
        // The altered bytes must fail receipt authentication before any JSON
        // interpretation can reject them merely as malformed syntax.
        assert!(
            matches!(
                error,
                SystemsPublicationCacheError::Trusted(TrustedPublicationError::Mismatch(
                    "archive entry digest"
                ))
            ),
            "{entry}: {error}"
        );
        fs::remove_file(&tampered).unwrap();
    }
    rewrite_archive(&systems_path, &tampered, None, true);
    rejected(&tampered, &sources, &kerml, "extra archive entry");
    fs::remove_file(&tampered).unwrap();

    fs::copy(&systems_path, &tampered).unwrap();
    duplicate_name(&tampered);
    rejected(&tampered, &sources, &kerml, "duplicate archive entry");
    fs::remove_file(&tampered).unwrap();

    fs::copy(&systems_path, &tampered).unwrap();
    let file = OpenOptions::new().write(true).open(&tampered).unwrap();
    let length = file.metadata().unwrap().len();
    file.set_len(length.checked_sub(32).expect("actual ZIP length"))
        .unwrap();
    drop(file);
    rejected(&tampered, &sources, &kerml, "truncated archive");
}

type SelectedEvidence = Vec<(
    (
        agq_kernel::ElementId,
        agq_kernel::PropertyId,
        agq_kernel::ElementId,
    ),
    usize,
    [u8; 32],
    [u8; 32],
)>;

// Retain content hashes, not a second publication's potentially large evidence
// payloads. Pool addresses accelerate repeated immutable values only; equality
// depends on exact proof/search contents and ordered-reference position.
fn selected_reference_evidence(model: &agq_kernel::ModelView) -> SelectedEvidence {
    let mut proofs = BTreeMap::new();
    let mut searches = BTreeMap::new();
    model
        .ordered_reference_contributions()
        .map(|(key, contribution)| {
            let proof = *proofs
                .entry(std::ptr::from_ref(contribution.explanation()) as usize)
                .or_insert_with(|| {
                    <[u8; 32]>::from(Sha256::digest(
                        serde_json::to_vec(contribution.explanation()).unwrap(),
                    ))
                });
            let search = *searches
                .entry(std::ptr::from_ref(contribution.searches()) as usize)
                .or_insert_with(|| {
                    <[u8; 32]>::from(Sha256::digest(
                        serde_json::to_vec(contribution.searches()).unwrap(),
                    ))
                });
            (key, contribution.position(), proof, search)
        })
        .collect()
}

fn required_cache(name: &str) -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} is required for this requested acceptance gate")),
    );
    let file = File::open(&path)
        .unwrap_or_else(|error| panic!("{name} must name a readable original cache: {error}"));
    assert!(
        file.metadata().unwrap().is_file(),
        "{name} must name a regular cache file"
    );
    path
}

fn restore(
    path: &Path,
    sources: &VerifiedLibrarySet,
    kerml: &Arc<CanonicalKermlStandardLibraries>,
) -> CanonicalSysmlSystemsLibrary {
    CanonicalSysmlSystemsLibrary::restore_cache(File::open(path).unwrap(), sources, kerml.clone())
        .unwrap_or_else(|error| panic!("exact accepted Systems restoration failed: {error}"))
}

fn assert_restored(
    publication: &CanonicalSysmlSystemsLibrary,
    kerml: &Arc<CanonicalKermlStandardLibraries>,
) {
    assert!(Arc::ptr_eq(publication.accepted_kerml(), kerml));
    let dependency = publication.declared().immutable_dependency().unwrap();
    assert!(std::ptr::eq(dependency.model(), kerml.overlay().model()));
    for record in kerml.overlay().model().elements() {
        assert!(std::ptr::eq(
            publication.overlay().model().element(record.id()).unwrap(),
            record
        ));
    }
    assert_eq!(publication.counters(), &PublicationCounters::default());
    assert_eq!(
        kerml.complete_overlay().counters(),
        &PublicationCounters::default()
    );
    assert!(kerml.complete_overlay().restored_from_receipt());
    assert!(publication.audit().findings.is_empty());
    assert_eq!(publication.audit().mandatory_references, 1327);
    assert_eq!(publication.audit().complete_references, 1327);
    assert_eq!(publication.documents().len(), 21);
    assert!(publication.bindings().sources_verified());
    assert!(
        publication
            .producer_closure()
            .compatible_context(&publication.context().kerml)
    );
    assert_eq!(
        publication.producer_closure().model_digest(),
        publication.semantic_digest()
    );
    assert_eq!(
        publication.bindings().targets().len(),
        StandardSysmlRole::ALL.len()
    );
    for role in StandardSysmlRole::ALL {
        assert!(publication.bindings().get(role).is_some(), "{role:?}");
    }
    for record in publication.overlay().model().elements() {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(
                publication
                    .producer_closure()
                    .is_closed(record.id(), requirement),
                "{:?}: {requirement:?}",
                record.id()
            );
        }
    }
}

fn rejected(
    path: &Path,
    sources: &VerifiedLibrarySet,
    kerml: &Arc<CanonicalKermlStandardLibraries>,
    case: &str,
) -> SystemsPublicationCacheError {
    let result = CanonicalSysmlSystemsLibrary::restore_cache(
        File::open(path).unwrap(),
        sources,
        kerml.clone(),
    );
    let error = result
        .err()
        .unwrap_or_else(|| panic!("compiled receipt accepted {case}"));
    assert_eq!(
        kerml.complete_overlay().counters(),
        &PublicationCounters::default()
    );
    error
}

fn rewrite_archive(source: &Path, output: &Path, changed: Option<&str>, extra: bool) {
    let mut input = ZipArchive::new(File::open(source).unwrap()).unwrap();
    let mut writer = ZipWriter::new(File::create(output).unwrap());
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(1))
        .large_file(true);
    let mut changed_found = false;
    for index in 0..input.len() {
        let entry = input.by_index(index).unwrap();
        if changed == Some(entry.name()) {
            changed_found = true;
            writer.start_file(entry.name(), options).unwrap();
            let mut reader = AlterFirstByte {
                inner: entry,
                altered: false,
            };
            io::copy(&mut reader, &mut writer).unwrap();
            assert!(reader.altered);
        } else {
            writer.raw_copy_file(entry).unwrap();
        }
    }
    assert_eq!(changed_found, changed.is_some());
    if extra {
        writer
            .start_file("untrusted-receipt.json", options)
            .unwrap();
        writer.write_all(br#"{"status":"accepted"}"#).unwrap();
    }
    writer.finish().unwrap().sync_all().unwrap();
}

struct AlterFirstByte<R> {
    inner: R,
    altered: bool,
}
impl<R: Read> Read for AlterFirstByte<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(bytes)?;
        if count != 0 && !self.altered {
            bytes[0] ^= 1;
            self.altered = true;
        }
        Ok(count)
    }
}

fn duplicate_name(path: &Path) {
    // ZIP writers reject duplicate names; edit only the two actual name fields.
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let entry = archive.by_name("kernel.jsonl").unwrap();
    let offsets = [entry.header_start() + 30, entry.central_header_start() + 46];
    drop(entry);
    drop(archive);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    for offset in offsets {
        file.seek(SeekFrom::Start(offset)).unwrap();
        let mut name = [0; 12];
        file.read_exact(&mut name).unwrap();
        assert_eq!(&name, b"kernel.jsonl");
        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(b"closure.json").unwrap();
    }
    file.sync_all().unwrap();
}

struct Scratch {
    path: PathBuf,
}
impl Scratch {
    fn new(root: &Path) -> Self {
        let parent = root.join("verification/generated");
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join(format!("accepted-systems-cache-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self { path }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        for name in ["roundtrip.zip", "tampered.zip"] {
            let _ = fs::remove_file(self.path.join(name));
        }
        let _ = fs::remove_dir(&self.path);
    }
}
