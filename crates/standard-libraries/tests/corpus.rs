use agq_kerml_syntax::production::{self, Limits};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use std::{collections::BTreeSet, path::Path};

#[test]
fn all_pinned_kerml_documents_parse_losslessly_and_repeatably() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let mut documents = 0;
    let mut discrepancies = 0;
    for source in sources
        .documents()
        .filter(|d| d.language() == LibraryLanguage::KerMl)
    {
        let first = production::parse(
            source.document(),
            source.revision(),
            source.source(),
            Limits::default(),
        )
        .unwrap();
        let second = production::parse(
            source.document(),
            source.revision(),
            source.source(),
            Limits::default(),
        )
        .unwrap();
        assert!(
            first.is_complete(),
            "{}: {:?}",
            source.path(),
            first.diagnostics()
        );
        assert!(first.recovery().is_empty());
        assert_eq!(
            first
                .tokens()
                .iter()
                .map(|t| first.token_text(t))
                .collect::<String>(),
            source.source()
        );
        assert_eq!(
            first
                .nodes()
                .map(|n| (n.id(), n.kind(), n.range()))
                .collect::<Vec<_>>(),
            second
                .nodes()
                .map(|n| (n.id(), n.kind(), n.range()))
                .collect::<Vec<_>>()
        );
        let ids: BTreeSet<_> = first.nodes().map(|n| n.id()).collect();
        assert_eq!(ids.len(), first.nodes().count());
        assert_eq!(first.discrepancies(), second.discrepancies());
        discrepancies += first.discrepancies().len();
        documents += 1;
    }
    assert_eq!(documents, 36);
    assert_eq!(discrepancies, 42);
}
