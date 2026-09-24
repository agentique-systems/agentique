//! Held test support: every semantic test restores exact accepted publications.
//! No fallback constructs standards or substitutes the strict SourceProject API.
use agq_kerml_semantics::{
    Completeness, KerMlQueries, QualifiedName, Resolution, SemanticContextId,
    TrustedPublicationReceipt,
};
use agq_kerml_syntax::{TextEdit, production::Production};
use agq_kerml_text::{
    ProjectChange, SourceLanguage, library::CanonicalKermlStandardLibraries,
    sysml::CanonicalSysmlSystemsLibrary,
};
use agq_kernel::{DocumentId, ElementId, SourceRevisionId, SyntaxNodeId, provenance::ByteRange};
use agq_modeling_workspace::{ProjectWorkspace, WorkingProjectRevision};
use agq_standard_libraries::VerifiedLibrarySet;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt::{self, Write},
    fs::File,
    path::Path,
    sync::{Arc, OnceLock},
};

#[path = "../../../../verification/fixtures/modeling-workspace-phase1/executable_inputs.rs"]
pub mod inputs;

pub fn accepted() -> Arc<CanonicalSysmlSystemsLibrary> {
    static ACCEPTED: OnceLock<Arc<CanonicalSysmlSystemsLibrary>> = OnceLock::new();
    ACCEPTED
        .get_or_init(|| {
            let kerml_path = std::env::var_os("AGENTIQUE_KERML_CACHE")
                .expect("requested workspace acceptance requires AGENTIQUE_KERML_CACHE");
            let systems_path = std::env::var_os("AGENTIQUE_SYSTEMS_CACHE")
                .expect("requested workspace acceptance requires AGENTIQUE_SYSTEMS_CACHE");
            // Both inputs and compiled authority must exist before the large
            // KerML restore. A requested but unavailable gate fails immediately.
            let kerml_file = File::open(kerml_path).unwrap();
            let systems_file = File::open(systems_path).unwrap();
            TrustedPublicationReceipt::checked_in("sysml-systems-operational-v3")
                .expect("requested workspace acceptance requires an accepted Systems receipt");
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
            let kerml = Arc::new(
                CanonicalKermlStandardLibraries::restore_cache(kerml_file, &sources).unwrap(),
            );
            Arc::new(
                CanonicalSysmlSystemsLibrary::restore_cache(systems_file, &sources, kerml).unwrap(),
            )
        })
        .clone()
}

pub fn open() -> ProjectWorkspace {
    ProjectWorkspace::open(accepted()).unwrap()
}

pub fn add(path: &str, language: SourceLanguage, source: &str) -> ProjectChange {
    ProjectChange::Add {
        path: path.into(),
        language,
        source: source.into(),
    }
}

pub fn seed(workspace: &mut ProjectWorkspace) -> Arc<WorkingProjectRevision> {
    workspace
        .apply(
            workspace.head().revision(),
            [
                add("Contracts.kerml", SourceLanguage::KerMl, inputs::CONTRACTS),
                add(
                    "Repository.sysml",
                    SourceLanguage::SysMl,
                    inputs::REPOSITORY,
                ),
                add("Workspace.sysml", SourceLanguage::SysMl, inputs::WORKSPACE),
            ],
        )
        .unwrap()
}

pub fn edit(
    revision: &WorkingProjectRevision,
    path: &str,
    start: usize,
    end: usize,
    replacement: &str,
) -> ProjectChange {
    ProjectChange::Edit {
        document: revision.document_at(path).unwrap().id(),
        edit: TextEdit {
            range: ByteRange::new(start as u64, end as u64).unwrap(),
            replacement: replacement.into(),
        },
    }
}

pub fn replace_by_edit(
    revision: &WorkingProjectRevision,
    path: &str,
    source: &str,
) -> ProjectChange {
    edit(
        revision,
        path,
        0,
        revision.document_at(path).unwrap().source().len(),
        source,
    )
}

pub fn path(q: &KerMlQueries<'_>, root: ElementId, segments: &[&str]) -> ElementId {
    let answer = q.lookup_path(
        root,
        &QualifiedName {
            absolute: false,
            segments: segments.iter().map(|s| (*s).to_owned()).collect(),
        },
    );
    assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
    let mut ids: Vec<_> = answer.value.iter().map(|m| m.element).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 1, "{segments:?}: {answer:?}");
    ids[0]
}

pub fn element(revision: &WorkingProjectRevision, segments: &[&str]) -> ElementId {
    path(
        &revision.kerml_queries().unwrap(),
        revision.root(),
        segments,
    )
}

pub fn assert_valid(revision: &Arc<WorkingProjectRevision>) {
    let validated = revision
        .validate()
        .expect("platform supported-slice acceptance");
    assert!(Arc::ptr_eq(validated.working(), revision));
    assert!(revision.strict_snapshot().is_some());
    let status = revision.producer_status().expect("local producer status");
    assert!(status.converged, "{status:?}");
    assert_eq!(status.completeness, Completeness::Complete, "{status:?}");
    assert!(
        revision
            .producer_closure()
            .expect("validated revision needs a closure certificate")
            .is_fully_closed(revision.semantic_model().unwrap()),
        "convergence or a partial certificate alone cannot validate a revision"
    );
    for reference in revision.references() {
        assert_eq!(
            reference.resolution.completeness,
            Completeness::Complete,
            "{reference:?}"
        );
        assert!(
            matches!(reference.resolution.value, Resolution::Resolved(_)),
            "{reference:?}"
        );
    }
    let kerml = revision.kerml_queries().unwrap();
    let sysml = revision.sysml_queries().unwrap();
    assert!(std::ptr::eq(kerml.model(), sysml.model()));
    assert!(std::ptr::eq(
        revision.semantic_model().unwrap(),
        kerml.model()
    ));
    assert_shared(revision);
}

pub fn assert_shared(revision: &WorkingProjectRevision) {
    let accepted = accepted();
    assert!(Arc::ptr_eq(revision.accepted_sysml(), &accepted));
    assert!(Arc::ptr_eq(
        revision.accepted_kerml(),
        accepted.accepted_kerml()
    ));
    // Test-only storage observations must come from actual retained tables,
    // including Working/empty mounts. Facade/record Arc identity cannot detect
    // cloning an accepted graph's maps, indexes or proof/search tables.
    let expected_tables = agq_modeling_workspace::testing::publication_storage(&accepted);
    let storage = agq_modeling_workspace::testing::dependency_storage(revision);
    assert!(!expected_tables.is_empty());
    assert_eq!(storage.base_tables, expected_tables);
    assert!(storage.copied_dependency_entries.is_zero(), "{storage:?}");
    if let Some(model) = revision.semantic_model() {
        for (standard, standard_model) in [
            (accepted.roots()[0], accepted.overlay().model()),
            (
                accepted.accepted_kerml().roots()[0],
                accepted.accepted_kerml().overlay().model(),
            ),
        ] {
            assert!(std::ptr::eq(
                model.element(standard).unwrap(),
                standard_model.element(standard).unwrap()
            ));
        }
    }
    // Test-only scheduler observer, specified in README. Counting evaluations
    // alone cannot establish that accepted standard subjects were never replayed.
    let observed: BTreeSet<_> =
        agq_modeling_workspace::testing::producer_subjects(revision).collect();
    if revision
        .producer_status()
        .is_some_and(|status| status.counters.subjects_evaluated > 0)
    {
        assert!(
            !observed.is_empty(),
            "the scheduler evaluated subjects but its verification observer recorded none"
        );
    }
    for subject in observed {
        assert!(
            accepted.overlay().model().element(subject).is_none(),
            "replayed accepted standard subject {subject}"
        );
    }
}

pub fn assert_unresolved(revision: &Arc<WorkingProjectRevision>, removed: Option<ElementId>) {
    assert!(revision.validate().is_err());
    assert!(!revision.diagnostics().is_empty());
    assert!(revision.references().iter().any(|reference| {
        reference.resolution.completeness != Completeness::Complete
            || !matches!(reference.resolution.value, Resolution::Resolved(_))
    }));
    if let Some(removed) = removed {
        assert!(
            revision
                .references()
                .iter()
                .all(|reference| reference.resolution.value != Resolution::Resolved(removed))
        );
        assert!(
            revision
                .semantic_model()
                .is_none_or(|model| model.element(removed).is_none())
        );
    }
    assert_shared(revision);
}

#[derive(Debug, PartialEq, Eq)]
pub struct DocumentSignature {
    path: String,
    id: DocumentId,
    revision: SourceRevisionId,
    language: SourceLanguage,
    source: String,
    syntax_nodes: Vec<(SyntaxNodeId, ByteRange)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ImmutableSignature {
    documents: Vec<DocumentSignature>,
    diagnostics: String,
    references: DebugSignature,
    semantic_context: Option<SemanticContextId>,
    closure_digest: Option<[u8; 32]>,
}

#[derive(Debug, PartialEq, Eq)]
struct DebugSignature {
    bytes: usize,
    sha256: [u8; 32],
}

fn debug_signature<T: fmt::Debug + ?Sized>(value: &T) -> DebugSignature {
    struct Stream {
        bytes: usize,
        hash: Sha256,
    }
    impl Write for Stream {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            self.bytes += value.len();
            self.hash.update(value.as_bytes());
            Ok(())
        }
    }
    let mut stream = Stream {
        bytes: 0,
        hash: Sha256::new(),
    };
    // Exactly the former non-pretty Debug byte sequence, including every field
    // of each reference and its query evidence. Stream it instead of retaining
    // one large proof string per revision and four more during parallel reads.
    write!(&mut stream, "{value:?}").expect("infallible digest writer");
    DebugSignature {
        bytes: stream.bytes,
        sha256: stream.hash.finalize().into(),
    }
}

#[test]
fn immutable_reference_signature_matches_exact_debug_bytes_and_evidence() {
    let document = agq_kerml_text::Document::new(
        "namespace Signature { type Base; type Child specializes Base; }",
    )
    .unwrap();
    let references = document.current().references();
    assert_eq!(references.len(), 1);
    let exact = format!("{references:?}");
    let baseline = debug_signature(references);
    assert_eq!(baseline.bytes, exact.len());
    assert_eq!(
        baseline.sha256,
        <[u8; 32]>::from(Sha256::digest(exact.as_bytes()))
    );

    let mut changed = references.to_vec();
    changed[0].resolution.search_dependencies.insert(
        agq_kerml_semantics::SearchDependency::ValidationRule("signature-evidence-change"),
    );
    assert_eq!(changed[0].resolution.value, references[0].resolution.value);
    assert_ne!(debug_signature(changed.as_slice()), baseline);

    // Byte counts and the digest follow UTF-8 bytes, not character counts.
    changed[0].name.segments.push("référence".into());
    let exact = format!("{changed:?}");
    let unicode = debug_signature(changed.as_slice());
    assert_eq!(unicode.bytes, exact.len());
    assert_eq!(
        unicode.sha256,
        <[u8; 32]>::from(Sha256::digest(exact.as_bytes()))
    );
}

pub fn immutable_signature(revision: &WorkingProjectRevision) -> ImmutableSignature {
    ImmutableSignature {
        documents: revision
            .documents()
            .map(|(path, doc)| DocumentSignature {
                path: path.to_owned(),
                id: doc.id(),
                revision: doc.revision(),
                language: doc.language(),
                source: doc.source().to_owned(),
                syntax_nodes: doc
                    .production_syntax()
                    .into_iter()
                    .flat_map(|syntax| syntax.nodes().map(|node| (node.id(), node.range())))
                    .collect(),
            })
            .collect(),
        diagnostics: format!("{:?}", revision.diagnostics()),
        references: debug_signature(revision.references()),
        semantic_context: revision.kerml_queries().ok().map(|q| q.context().clone()),
        closure_digest: revision
            .producer_closure()
            .map(|certificate| certificate.digest()),
    }
}

pub fn syntax_id(
    revision: &WorkingProjectRevision,
    path: &str,
    kind: Production,
    text: &str,
) -> SyntaxNodeId {
    let matches: Vec<_> = revision
        .document_at(path)
        .unwrap()
        .production_syntax()
        .unwrap()
        .nodes()
        .filter(|node| node.kind() == kind && node.text() == text)
        .map(|node| node.id())
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "fixture needs one syntax node for {text:?}"
    );
    matches[0]
}

pub fn group_projection(revision: &WorkingProjectRevision, group: usize) -> String {
    let q = match revision.sysml_queries() {
        Ok(q) => q,
        Err(unavailable) => return format!("unavailable: {unavailable:?}"),
    };
    let owner = q.kerml().lookup_path(
        revision.root(),
        &QualifiedName {
            absolute: false,
            segments: vec![format!("Workbench{group:03}"), "Worker".into()],
        },
    );
    let ids: BTreeSet<_> = owner.value.iter().map(|member| member.element).collect();
    let children: Vec<_> = ids
        .iter()
        .map(|&owner| {
            let answer = q.effective_usages(owner);
            (owner, answer.completeness(), answer.value().clone())
        })
        .collect();
    format!("{:?} {children:?}", owner.completeness)
}
