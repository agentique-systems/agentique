use super::construction::{LoweringCache, SourceInput, construct_on_cached};
use super::*;
use agq_kernel::{DocumentId, GeneratorId, SourceRevisionId, provenance::DeclaredOrigin};
use std::sync::Arc;

#[test]
fn unchanged_document_lowering_is_reused_with_exact_full_reconstruction() {
    let registry =
        Arc::new(agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap());
    let base = Snapshot::new(registry);
    let root = (
        ElementId::new(),
        DeclaredOrigin::Generated {
            generator: GeneratorId::new(),
        },
    );
    let parse = |source: &str| {
        production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV3,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            Default::default(),
        )
        .unwrap()
    };
    let stable = parse("package Stable { part def Unchanged; }");
    let editable = parse(
        "package Edited { part def Part { attribute count : ScalarValues::Integer = 1; part child : Part[1]; } }",
    );
    let mut cache = LoweringCache::default();
    let build = |editable: &production::Document, cache: Option<&mut LoweringCache>| {
        let inputs = [
            SourceInput {
                syntax: &stable,
                library: None,
                sysml: true,
            },
            SourceInput {
                syntax: editable,
                library: None,
                sysml: true,
            },
        ];
        construct_on_cached(
            &inputs,
            &BTreeMap::new(),
            agq_kerml::BaselineProfile::OPERATIONAL,
            base.clone(),
            Some(root.clone()),
            cache,
        )
        .unwrap()
    };
    let initial = build(&editable, Some(&mut cache));
    assert_eq!(cache.documents_lowered, 2);
    assert!(initial.candidate().model().elements().count() > 1);
    for (old, new) in [
        ("= 1", "= 2"),
        ("Integer", "Real"),
        ("[1]", "[2]"),
        ("child", "renamed"),
        ("part child", "part another : Part; part child"),
    ] {
        let offset = editable.source().find(old).unwrap();
        let edited = editable
            .edit(
                &agq_kerml_syntax::TextEdit {
                    range: agq_kernel::provenance::ByteRange::new(
                        offset as u64,
                        (offset + old.len()) as u64,
                    )
                    .unwrap(),
                    replacement: new.into(),
                },
                Default::default(),
            )
            .unwrap();
        let mut child_cache = cache.next_revision();
        let incremental = build(&edited, Some(&mut child_cache));
        let full = build(&edited, None);
        assert!(
            incremental
                .candidate()
                .model()
                .elements()
                .eq(full.candidate().model().elements()),
            "{old} -> {new}"
        );
        assert!(
            incremental
                .candidate()
                .model()
                .association_occurrences()
                .eq(full.candidate().model().association_occurrences())
        );
        assert_eq!(incremental.source_map(), full.source_map());
        assert_eq!(incremental.references(), full.references());
        assert_eq!(child_cache.documents_lowered, 1);
        assert_eq!(child_cache.documents_reused, 1);
    }
}
