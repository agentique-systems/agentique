use super::construction::{LoweringCache, SourceInput, construct_on_cached};
use super::*;
use agq_kernel::{DocumentId, GeneratorId, SourceRevisionId, provenance::DeclaredOrigin};
use std::sync::Arc;

#[test]
fn full_construction_rejects_existing_identity_instead_of_reusing_it() {
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap(),
    ));
    let root = (
        ElementId::new(),
        DeclaredOrigin::Generated {
            generator: GeneratorId::new(),
        },
    );
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV3,
        DocumentId::new(),
        SourceRevisionId::new(),
        "package P { part def A; }",
        Default::default(),
    )
    .unwrap();
    let construct = |base: Snapshot, cache: Option<&mut LoweringCache>| {
        construct_on_cached(
            &[SourceInput {
                syntax: &syntax,
                library: None,
                sysml: true,
            }],
            &BTreeMap::new(),
            agq_kerml::BaselineProfile::OPERATIONAL,
            base,
            Some(root.clone()),
            cache,
        )
    };
    let initial = construct(base, None)
        .unwrap()
        .candidate()
        .clone()
        .revalidate_declared()
        .unwrap();
    let records: Vec<_> = initial.model().elements().cloned().collect();
    let mut disabled_cache = LoweringCache::default();
    disabled_cache.allow_reuse = false;
    disabled_cache.reconstruction_base = Some(initial.clone());
    for cache in [None, Some(&mut disabled_cache)] {
        let failure = construct(initial.clone(), cache).unwrap_err();
        assert!(
            matches!(failure, LibraryLoadError::Kernel(agq_kernel::ModelError::ReusedIdentity(id)) if id == root.0)
        );
        assert!(
            initial.model().elements().eq(records.iter()),
            "failed full construction leaves the base unchanged"
        );
    }
}

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

#[test]
fn declared_reconstruction_frontier_matches_full_and_shares_unchanged_records() {
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
    let stable = parse("package Stable { part def Unit; }");
    let editable = parse("package Edited { part def Assembly { part child : Stable::Unit; } }");
    let build = |editable: &production::Document,
                 resolved: &BTreeMap<_, _>,
                 cache: Option<&mut LoweringCache>| {
        construct_on_cached(
            &[
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
            ],
            resolved,
            agq_kerml::BaselineProfile::OPERATIONAL,
            base.clone(),
            Some(root.clone()),
            cache,
        )
        .unwrap()
    };
    let mut cache = LoweringCache::default();
    let unresolved = build(&editable, &BTreeMap::new(), Some(&mut cache));
    let unit = unresolved
        .candidate()
        .model()
        .elements()
        .find(|record| {
            record
                .slot(agq_kerml::properties::ELEMENT_DECLARED_NAME)
                .is_some_and(|slot| {
                    slot.value().values().any(
                        |value| matches!(value, agq_kernel::value::Value::String(name) if name == "Unit"),
                    )
                })
        })
        .unwrap()
        .id();
    let endpoints = unresolved
        .references()
        .iter()
        .map(|reference| ((reference.relationship, reference.property), unit))
        .collect();
    let initial = build(&editable, &endpoints, Some(&mut cache));
    let previous = initial.candidate().clone().revalidate_declared().unwrap();
    let original_records: Vec<_> = previous.model().elements().cloned().collect();
    for (name, old, new) in [
        ("rename", "child", "renamed"),
        ("add-part", "part child", "part other; part child"),
        ("remove-part", "part child : Stable::Unit;", ""),
        ("remove-typing", " : Stable::Unit", ""),
        ("unresolved-reference", "Stable::Unit", "Missing::Unit"),
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
        child_cache.reconstruction_base = Some(previous.clone());
        let started = std::time::Instant::now();
        // Refinement begins without inherited endpoint guesses. The transaction
        // must clear the prior resolved reference before any new lookup occurs.
        let incremental = build(&edited, &BTreeMap::new(), Some(&mut child_cache));
        let incremental_us = started.elapsed().as_micros();
        let mut full_cache = cache.next_revision();
        full_cache.allow_reuse = false;
        let started = std::time::Instant::now();
        let full = build(&edited, &BTreeMap::new(), Some(&mut full_cache));
        let full_us = started.elapsed().as_micros();
        assert!(
            incremental
                .candidate()
                .model()
                .elements()
                .eq(full.candidate().model().elements()),
            "canonical records/provenance: {name}"
        );
        assert!(
            incremental
                .candidate()
                .model()
                .association_occurrences()
                .eq(full.candidate().model().association_occurrences()),
            "canonical occurrences: {name}"
        );
        assert_eq!(
            incremental.candidate().obligations(),
            full.candidate().obligations(),
            "obligations: {name}"
        );
        assert_eq!(incremental.source_map(), full.source_map());
        assert_eq!(incremental.references(), full.references());
        assert!(child_cache.records_reused > 0, "unchanged records: {name}");
        assert!(
            std::ptr::eq(
                previous.model().element(unit).unwrap(),
                incremental.candidate().model().element(unit).unwrap()
            ),
            "unchanged declared record allocation: {name}"
        );
        assert!(
            previous.model().elements().eq(original_records.iter()),
            "prior revision immutable"
        );
        eprintln!(
            "{name}: incremental_records={} full_records={} retained_records={} incremental_us={incremental_us} full_us={full_us}",
            child_cache.records_rebuilt, full_cache.records_rebuilt, child_cache.records_reused
        );
    }
}

#[test]
fn hundred_document_declared_reconstruction_frontier_is_bounded() {
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap(),
    ));
    let root = (
        ElementId::new(),
        DeclaredOrigin::Generated {
            generator: GeneratorId::new(),
        },
    );
    let mut documents: Vec<_> = (0..100)
        .map(|index| {
            production::parse_sysml_with_profile(
                production::SysmlSyntaxProfile::OperationalV3,
                DocumentId::new(),
                SourceRevisionId::new(),
                format!("package Doc{index} {{ part def Unit{index}; }}"),
                Default::default(),
            )
            .unwrap()
        })
        .collect();
    let build = |documents: &[production::Document], cache: &mut LoweringCache| {
        let inputs: Vec<_> = documents
            .iter()
            .map(|syntax| SourceInput {
                syntax,
                library: None,
                sysml: true,
            })
            .collect();
        construct_on_cached(
            &inputs,
            &BTreeMap::new(),
            agq_kerml::BaselineProfile::OPERATIONAL,
            base.clone(),
            Some(root.clone()),
            Some(cache),
        )
        .unwrap()
    };
    let mut cache = LoweringCache::default();
    let initial = build(&documents, &mut cache);
    let previous = initial.candidate().clone().revalidate_declared().unwrap();
    let start = documents[50].source().find("Unit50").unwrap();
    documents[50] = documents[50]
        .edit(
            &agq_kerml_syntax::TextEdit {
                range: agq_kernel::provenance::ByteRange::new(start as u64, (start + 6) as u64)
                    .unwrap(),
                replacement: "RenamedUnit".into(),
            },
            Default::default(),
        )
        .unwrap();
    let mut incremental_cache = cache.next_revision();
    incremental_cache.reconstruction_base = Some(previous);
    let started = std::time::Instant::now();
    let incremental = build(&documents, &mut incremental_cache);
    let incremental_us = started.elapsed().as_micros();
    let mut full_cache = cache.next_revision();
    full_cache.allow_reuse = false;
    let started = std::time::Instant::now();
    let full = build(&documents, &mut full_cache);
    let full_us = started.elapsed().as_micros();
    assert!(
        incremental
            .candidate()
            .model()
            .elements()
            .eq(full.candidate().model().elements())
    );
    assert_eq!(
        incremental.candidate().obligations(),
        full.candidate().obligations()
    );
    assert_eq!(incremental.source_map(), full.source_map());
    assert_eq!(incremental.references(), full.references());
    assert_eq!(incremental_cache.documents_lowered, 1);
    assert_eq!(full_cache.documents_lowered, 100);
    // Reconciled syntax can retire/recreate the edited document's declarations;
    // count deletions too. The independent 99 documents still require no writes.
    assert!(incremental_cache.records_rebuilt * 40 < full_cache.records_rebuilt);
    eprintln!(
        "100-documents: incremental_records={} full_records={} retained_records={} incremental_us={incremental_us} full_us={full_us}; declared construction only, no accepted publication or semantic closure",
        incremental_cache.records_rebuilt,
        full_cache.records_rebuilt,
        incremental_cache.records_reused
    );
}
