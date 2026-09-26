use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn compact_proof_taint_survives_without_changing_receiver_expansion_mode() {
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    let snapshot = fixture.finish();
    let key = DerivationKey {
        rule: RuleId::from_u128(44103),
        subject: id(1),
        output: OutputKey::from_u128(1),
    };
    let root = FactKey::Element(key.element_id());
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.element(
        key,
        c::TYPE,
        snapshot
            .model()
            .element(id(1))
            .unwrap()
            .slots()
            .map(|(property, slot)| (property, slot.value().clone())),
        BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
    );
    let search = StructuralSearch::Incoming(id(99));
    builder.searches_shared(root, Arc::new(BTreeSet::from([search.clone()])));
    let overlay = builder.build().unwrap();
    let context =
        SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap();
    let ordinary = KerMlQueries::new(context.fork());
    let producer = KerMlQueries::for_production(context);
    let full = ordinary.canonical_fact_evidence(FactKey::Element(id(1)));
    let compact = producer.canonical_fact_evidence(root);
    assert!(!full.producer_evidence);
    assert!(!full.contains_compact_evidence);
    assert!(compact.producer_evidence);
    assert!(compact.contains_compact_evidence);
    assert!(
        compact
            .shared_search_dependencies
            .iter()
            .any(|s| *s == search)
    );
    let expected_dependencies: BTreeSet<_> = full
        .canonical_dependencies
        .union(&compact.canonical_dependencies)
        .cloned()
        .collect();
    for public_boundary in [false, true] {
        for compact_receiver in [false, true] {
            let (mut actual, other) = if compact_receiver {
                (compact.clone(), full.clone())
            } else {
                (full.clone(), compact.clone())
            };
            if public_boundary {
                actual.merge_evidence(other).unwrap();
            } else {
                actual.merge(other);
            }
            assert!(
                actual.contains_compact_evidence,
                "compact inputs cannot become full proof"
            );
            assert_eq!(actual.producer_evidence, compact_receiver);
            if !compact_receiver {
                assert!(actual.shared_search_dependencies.iter().next().is_none());
                assert!(
                    actual
                        .search_dependencies
                        .contains(&SearchDependency::Kernel(search.clone()))
                );
            }
            assert_eq!(actual.canonical_dependencies, expected_dependencies);
            let mut mapped = actual.map(|_| 42);
            mapped.expand_search_dependencies();
            mapped.merge_evidence(full.clone()).unwrap();
            assert!(
                mapped.contains_compact_evidence,
                "later full inputs cannot erase compact proof history"
            );
            assert_eq!(mapped.producer_evidence, compact_receiver);
            assert_eq!(mapped.value, 42);
            assert!(
                mapped
                    .search_dependencies
                    .contains(&SearchDependency::Kernel(search.clone()))
            );
            assert_eq!(mapped.canonical_dependencies, expected_dependencies);
            let mut relayed = full.clone();
            relayed.merge_evidence(mapped.clone()).unwrap();
            assert!(relayed.contains_compact_evidence);
            assert!(!relayed.producer_evidence);
            assert!(relayed.shared_search_dependencies.iter().next().is_none());
            // An ordinary receiver continues producing full public explanations
            // even though its earlier compact input cannot qualify audit reuse.
            mapped.prove(QueryKind::Owner, id(1), id(1), Rule::ElementOwner, []);
            assert_eq!(
                mapped.explanations.contains_key(&Conclusion {
                    query: QueryKind::Owner,
                    subject: id(1),
                    value: id(1),
                }),
                !compact_receiver
            );
        }
    }
    let mut entirely_full = full.clone();
    entirely_full.merge_evidence(full.clone()).unwrap();
    assert_eq!(
        entirely_full, full,
        "ordinary proof composition remains unchanged"
    );
}

#[test]
fn rejected_foreign_compact_merge_does_not_taint_or_mutate_receiver() {
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    let snapshot = fixture.finish();
    let ordinary = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let producer = KerMlQueries::for_production(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                exclude_implied: true,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    let mut result = ordinary.canonical_fact_evidence(FactKey::Element(id(1)));
    let before = result.clone();
    assert!(
        result
            .merge_evidence(producer.canonical_fact_evidence(FactKey::Element(id(1))))
            .is_err()
    );
    assert_eq!(result, before);
    assert!(!result.producer_evidence);
    assert!(!result.contains_compact_evidence);
}
