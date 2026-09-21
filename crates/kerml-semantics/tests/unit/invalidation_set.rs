use super::*;

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
