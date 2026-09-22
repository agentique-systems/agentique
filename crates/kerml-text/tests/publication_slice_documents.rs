#[path = "../examples/support/publication_documents.rs"]
mod publication_documents;
use agq_standard_libraries::VerifiedLibrarySet;
use publication_documents::*;
use std::{collections::BTreeSet, path::Path};

#[test]
fn every_slice_seed_identifies_one_exact_pinned_kpar_document() {
    let sources = VerifiedLibrarySet::load_from_directory(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    )
    .unwrap();
    validate_all_documents(&sources).unwrap();
    for (slice, expected) in SLICES.into_iter().zip([3, 2, 4, 2, 4]) {
        let names = slice_documents(slice).unwrap();
        let selected = select(&sources, names).unwrap();
        assert_eq!(selected.len(), expected, "slice {slice}");
        let paths = resolve_paths(sources.documents().map(|d| d.path()), names).unwrap();
        assert_eq!(paths.len(), expected);
        for name in names {
            assert_eq!(
                paths
                    .iter()
                    .filter(|path| path.rsplit('/').next() == Some(*name))
                    .count(),
                1,
                "{slice}: {name}"
            );
        }
    }
    for slice in ["D", "E"] {
        for name in slice_documents(slice).unwrap() {
            let full = format!("Kernel Semantic Library/{name}");
            assert_eq!(
                select(&sources, &[name]).unwrap(),
                select(&sources, &[&full]).unwrap()
            );
        }
    }
    assert!(slice_documents("unknown").is_none());
}
#[test]
fn basename_matching_excludes_suffix_collisions_and_full_paths_disambiguate() {
    let paths = [
        "Kernel Semantic Library/Performances.kerml",
        "Kernel Semantic Library/ControlPerformances.kerml",
        "Kernel Semantic Library/StatePerformances.kerml",
        "Kernel Semantic Library/TransitionPerformances.kerml",
    ];
    assert_eq!(
        resolve_paths(paths, &["Performances.kerml"]).unwrap(),
        BTreeSet::from([paths[0]])
    );
    assert!(resolve_paths(paths, &["Semantic Library/Performances.kerml"]).is_err());
    assert!(resolve_paths(paths, &["Missing.kerml"]).is_err());
    let duplicate = [paths[0], "Other/Performances.kerml"];
    assert!(resolve_paths(duplicate, &["Performances.kerml"]).is_err());
    assert_eq!(
        resolve_paths(duplicate, &[paths[0]]).unwrap(),
        BTreeSet::from([paths[0]])
    );
}
