//! Held test support: every semantic test restores exact accepted publications.
//! No fallback constructs standards or substitutes the strict SourceProject API.
use agq_kerml_semantics::{Completeness, KerMlQueries, QualifiedName, Resolution};
use agq_kerml_syntax::TextEdit;
use agq_kerml_text::{
    ProjectChange, SourceLanguage, library::CanonicalKermlStandardLibraries,
    sysml::CanonicalSysmlSystemsLibrary,
};
use agq_kernel::{ElementId, provenance::ByteRange};
use agq_modeling_workspace::{ProjectWorkspace, WorkingProjectRevision};
use agq_standard_libraries::VerifiedLibrarySet;
use std::{
    collections::BTreeSet,
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
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
            let kerml_path = std::env::var_os("AGENTIQUE_KERML_CACHE")
                .expect("requested workspace acceptance requires AGENTIQUE_KERML_CACHE");
            let systems_path = std::env::var_os("AGENTIQUE_SYSTEMS_CACHE")
                .expect("requested workspace acceptance requires AGENTIQUE_SYSTEMS_CACHE");
            let kerml = Arc::new(
                CanonicalKermlStandardLibraries::restore_cache(
                    File::open(kerml_path).unwrap(),
                    &sources,
                )
                .unwrap(),
            );
            Arc::new(
                CanonicalSysmlSystemsLibrary::restore_cache(
                    File::open(systems_path).unwrap(),
                    &sources,
                    kerml,
                )
                .unwrap(),
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
    assert!(revision.producer_closure().is_some());
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
    for subject in agq_modeling_workspace::testing::producer_subjects(revision) {
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

pub fn immutable_signature(
    revision: &WorkingProjectRevision,
) -> (
    Vec<(String, String, agq_kernel::SourceRevisionId)>,
    String,
    Option<agq_kerml_semantics::SemanticContextId>,
) {
    (
        revision
            .documents()
            .map(|(path, doc)| (path.to_owned(), doc.source().to_owned(), doc.revision()))
            .collect(),
        format!("{:?}", revision.diagnostics()),
        revision.kerml_queries().ok().map(|q| q.context().clone()),
    )
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
