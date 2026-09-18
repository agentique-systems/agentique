use agq_kernel::provenance::{ByteRange, DeclaredOrigin};
use agq_standard_libraries::{
    LibraryDiagnostic, LibraryElementRole, LibraryError, LibraryLanguage, VerifiedLibrarySet,
};
use std::{collections::BTreeMap, path::Path};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn exact_pinned_closure_terminates_cycles_and_is_repeatable() {
    let mut reads = BTreeMap::new();
    let first = VerifiedLibrarySet::load_with(|path| {
        *reads.entry(path.to_string()).or_insert(0) += 1;
        std::fs::read(root().join(path)).ok()
    })
    .unwrap();
    let second = VerifiedLibrarySet::load_from_directory(root()).unwrap();
    assert_eq!(first, second);
    assert_eq!(reads.len(), 4);
    assert!(reads.values().all(|&n| n == 1));
    assert_eq!(first.documents().count(), 57);
    assert_eq!(
        first
            .documents()
            .filter(|d| d.language() == LibraryLanguage::KerMl)
            .count(),
        36
    );
    assert_eq!(
        first
            .documents()
            .filter(|d| d.language() == LibraryLanguage::SysMl)
            .count(),
        21
    );
    for library in first.libraries.values() {
        assert_eq!(library.dependencies.len(), library.project.usage.len());
        for dep in &library.dependencies {
            assert!(first.libraries.contains_key(dep));
        }
        for doc in &library.documents {
            assert_eq!(doc.library(), library.id);
            assert_eq!(
                doc.origin(),
                DeclaredOrigin::StandardLibrary {
                    library: library.id
                }
            );
            let range = ByteRange::new(0, 0).unwrap();
            let other = second
                .documents()
                .find(|d| d.document() == doc.document())
                .unwrap();
            assert_eq!(
                doc.element_id(range, LibraryElementRole::Declaration)
                    .unwrap(),
                other
                    .element_id(range, LibraryElementRole::Declaration)
                    .unwrap()
            );
            assert_ne!(
                doc.element_id(range, LibraryElementRole::Declaration)
                    .unwrap(),
                doc.element_id(range, LibraryElementRole::OwningMembership)
                    .unwrap()
            );
        }
    }
    let semantic = first
        .libraries
        .values()
        .find(|l| l.resource.ends_with("/Semantic-Library.kpar"))
        .unwrap();
    let data = first
        .libraries
        .values()
        .find(|l| l.resource.ends_with("/Data-Type-Library.kpar"))
        .unwrap();
    assert!(semantic.dependencies.contains(&data.id));
    assert!(data.dependencies.contains(&semantic.id));
}

#[test]
fn changed_archive_and_missing_dependency_fail_without_partial_publication() {
    let wrong = VerifiedLibrarySet::load_with(|path| {
        let mut bytes = std::fs::read(root().join(path)).ok()?;
        if path.ends_with("Data-Type-Library.kpar") {
            bytes[0] ^= 1;
        }
        Some(bytes)
    });
    assert!(
        matches!(wrong,Err(LibraryError::ContentMismatch(path)) if path.ends_with("Data-Type-Library.kpar"))
    );
    let missing = VerifiedLibrarySet::load_with(|path| {
        if path.ends_with("Function-Library.kpar") {
            None
        } else {
            std::fs::read(root().join(path)).ok()
        }
    });
    assert!(
        matches!(missing,Err(LibraryError::MissingArchive(path)) if path.ends_with("Function-Library.kpar"))
    );
}

#[test]
fn published_metadata_error_and_junk_are_retained_without_rewriting_sources() {
    let set = VerifiedLibrarySet::load_from_directory(root()).unwrap();
    assert!(set.diagnostics.iter().any(|d| matches!(d, LibraryDiagnostic::MissingIndexTarget{name,target,..} if name=="AnalysisCases" && target.ends_with("/AnalysisCase.sysml"))));
    assert!(set.diagnostics.iter().any(|d| matches!(d, LibraryDiagnostic::UnindexedDocument{path,..} if path.ends_with("/AnalysisCases.sysml"))));
    assert_eq!(
        set.diagnostics
            .iter()
            .filter(|d| matches!(d, LibraryDiagnostic::RetainedOtherEntry { .. }))
            .count(),
        2
    );
    let systems = set
        .libraries
        .values()
        .find(|l| l.resource.ends_with("/Systems-Library.kpar"))
        .unwrap();
    assert_eq!(
        systems.metadata.index["AnalysisCases"],
        "AnalysisCase.sysml"
    );
    let doc = systems
        .documents
        .iter()
        .find(|d| d.path().ends_with("/AnalysisCases.sysml"))
        .unwrap();
    assert!(
        doc.source()
            .contains("standard library package AnalysisCases")
    );
    assert!(
        doc.element_id(
            ByteRange::new(0, u64::MAX).unwrap(),
            LibraryElementRole::Declaration
        )
        .is_err()
    );
}

#[test]
fn private_library_identity_encoding_has_an_independent_uuid_v5_golden() {
    let set = VerifiedLibrarySet::load_from_directory(root()).unwrap();
    // Independently calculated using Python uuid.uuid5 over the documented tuple.
    let expected = [
        (
            "/Semantic-Library.kpar",
            "ea196f53-7c8f-5648-ac40-fd76b770a376",
        ),
        (
            "/Data-Type-Library.kpar",
            "19e20130-43ab-5fe6-9f75-f86caec6d743",
        ),
        (
            "/Function-Library.kpar",
            "0ae35d47-050d-504c-a7a6-c48073400003",
        ),
        (
            "/Systems-Library.kpar",
            "6c418b74-7704-5af0-bce2-28b92196cb15",
        ),
    ];
    for (suffix, id) in expected {
        assert_eq!(
            set.libraries
                .values()
                .find(|l| l.resource.ends_with(suffix))
                .unwrap()
                .id
                .to_string(),
            id
        );
    }
    let base = set
        .documents()
        .find(|d| d.path() == "Kernel Semantic Library/Base.kerml")
        .unwrap();
    assert_eq!(
        base.document().to_string(),
        "e2cc3fe6-8ab4-546a-9f75-b3cc47a5208c"
    );
    assert_eq!(
        base.revision().to_string(),
        "a725e616-d38d-5d44-ad34-e020203ff4c2"
    );
    assert_eq!(
        base.element_id(
            ByteRange::new(0, 0).unwrap(),
            LibraryElementRole::Declaration
        )
        .unwrap()
        .to_string(),
        "dbdebbbf-0001-5ca0-9ff2-a3b601e3ef44"
    );
}
