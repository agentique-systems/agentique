use super::*;

#[test]
fn native_identity_reads_remain_distinct_from_kernel_population_reads() {
    let snapshot = agq_kernel::Snapshot::new(std::sync::Arc::new(agq_kerml::registry().unwrap()));
    let queries = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let absent = ElementId::from_u128(1);
    let mut answer = queries.result(());
    answer.search_dependencies.extend([
        SearchDependency::Element(absent),
        SearchDependency::Kernel(StructuralSearch::Element(absent)),
    ]);
    let searches = structural_searches(&answer);
    assert_eq!(
        searches,
        BTreeSet::from([
            StructuralSearch::ElementIdentity(absent),
            StructuralSearch::Element(absent),
        ])
    );
    let mut reread = queries.result(());
    reread
        .search_dependencies
        .extend(searches.into_iter().map(SearchDependency::Kernel));
    let ordinary = query_read_keys(&reread, snapshot.model());
    assert_eq!(ordinary, BTreeSet::from([InvalidationKey::Element(absent)]));
    // Missing existence can still be supplied by a producer. Persisting the
    // precision marker must not turn a negative read into a fixed fact.
    assert_eq!(
        query_publication_provider_keys(&reread, snapshot.model()),
        ordinary
    );
}

#[test]
fn source_role_precision_survives_persistent_search_transport_and_invalidation() {
    let snapshot = agq_kernel::Snapshot::new(std::sync::Arc::new(agq_kerml::registry().unwrap()));
    let queries = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let source = ElementId::from_u128(1);
    let class = agq_kerml::classes::FEATURE_TYPING;
    let property = agq_kerml::properties::FEATURE_TYPING_TYPED_FEATURE;
    let mut answer = queries.result(());
    answer
        .search_dependencies
        .insert(SearchDependency::SourceRelationships {
            source,
            class,
            property,
        });
    let searches = structural_searches(&answer);
    assert_eq!(
        searches,
        BTreeSet::from([StructuralSearch::SourceRelationships {
            source,
            class,
            property,
        }])
    );
    let keys = query_read_keys(&answer, snapshot.model());
    assert_eq!(keys, BTreeSet::from([InvalidationKey::Incoming(source)]));
    let persistent_keys: BTreeSet<_> = searches.iter().filter_map(structural_search_key).collect();
    assert_eq!(persistent_keys, keys);
    let compact = QueryInvalidationSet::from_keys(keys);
    assert!(compact.affected_by(&BTreeSet::from([source]), false));
    assert!(!compact.affected_by(&BTreeSet::from([ElementId::from_u128(99)]), false));
    assert!(compact.affected_by(&BTreeSet::new(), true));
}

#[test]
fn closure_witness_is_invalidated_by_any_graph_delta_but_is_not_a_graph_provider() {
    let snapshot = agq_kernel::Snapshot::new(std::sync::Arc::new(agq_kerml::registry().unwrap()));
    let queries = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let answer = queries.producer_closure(
        ElementId::from_u128(1),
        SemanticClosureRequirement::EffectiveTyping,
    );
    let keys = query_read_keys(&answer, snapshot.model());
    assert_eq!(keys, BTreeSet::from([InvalidationKey::Global]));
    let compact = QueryInvalidationSet::from_keys(keys);
    assert!(compact.affected_by(&BTreeSet::from([ElementId::from_u128(99)]), false));
    assert!(query_publication_provider_keys(&answer, snapshot.model()).is_empty());
    assert_eq!(
        structural_searches(&answer),
        BTreeSet::from([StructuralSearch::Model])
    );
    assert!(answer.canonical_dependencies.is_empty());
}

#[test]
fn compact_invalidation_matches_every_five_subject_read_and_change_population() {
    let ids: Vec<_> = (1..=5).map(ElementId::from_u128).collect();
    for reads_mask in 0..(1 << 11) {
        let mut keys = BTreeSet::new();
        for (index, &id) in ids.iter().enumerate() {
            if reads_mask & (1 << index) != 0 {
                keys.insert(InvalidationKey::Element(id));
            }
            if reads_mask & (1 << (index + 5)) != 0 {
                keys.insert(InvalidationKey::Incoming(id));
            }
        }
        if reads_mask & (1 << 10) != 0 {
            keys.insert(InvalidationKey::Global);
        }
        let reads = QueryReadSet {
            canonical_dependencies: BTreeSet::new(),
            search_dependencies: BTreeSet::new(),
            keys,
        };
        let compact = reads.clone().into_invalidation();
        assert_eq!(reads.reads_entire_model(), compact.reads_entire_model());
        assert_eq!(
            reads.bounded_elements().collect::<BTreeSet<_>>(),
            compact.bounded_elements().iter().copied().collect()
        );
        for changed_mask in 0..(1 << 6) {
            // Includes an identity outside every bounded read population.
            let changed = (0..6)
                .filter(|index| changed_mask & (1 << index) != 0)
                .map(|index| ElementId::from_u128(index + 1))
                .collect();
            for contract_changed in [false, true] {
                assert_eq!(
                    compact.affected_by(&changed, contract_changed),
                    reads.affected_by(&changed, contract_changed),
                    "reads {reads_mask}, changes {changed_mask}, contract {contract_changed}"
                );
            }
        }
    }
}

#[test]
fn retained_cache_uses_one_identity_per_element_even_with_many_property_reads() {
    let ids: Vec<_> = (1..=1024).map(ElementId::from_u128).collect();
    let reads = QueryReadSet {
        canonical_dependencies: ids
            .iter()
            .flat_map(|&element| {
                (1..=8).map(move |property| {
                    Dependency::Declared(FactKey::Property {
                        element,
                        property: agq_kernel::PropertyId::from_u128(property),
                    })
                })
            })
            .collect(),
        search_dependencies: ids
            .iter()
            .flat_map(|&element| {
                (1..=8).map(move |property| SearchDependency::PropertySet {
                    element,
                    property: agq_kernel::PropertyId::from_u128(property),
                })
            })
            .collect(),
        keys: ids
            .iter()
            .flat_map(|&id| [InvalidationKey::Element(id), InvalidationKey::Incoming(id)])
            .collect(),
    };
    let compact = reads.into_invalidation();
    assert_eq!(compact.bounded_elements(), ids);
    assert_eq!(std::mem::size_of_val(compact.bounded_elements()), 16 * 1024);
    assert!(!compact.reads_entire_model());
}
