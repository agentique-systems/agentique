//! A selected append witness must coexist with an independently requested slot proof.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn selected_append_and_whole_slot_reads_keep_broad_evidence_in_both_merge_orders() {
    let mut f = Fixture::new();
    for subject in 1..=4 {
        f.create(subject, c::FEATURE);
    }
    let snapshot = f.finish();
    let queries = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut plan = queries.plan_result_structure([]);
    let key = |output| DerivationKey {
        rule: RuleId::from_u128(7100),
        subject: id(1),
        output: OutputKey::from_u128(output),
    };
    plan.add_derived_element(
        key(1),
        c::SUBSETTING,
        BTreeMap::from([
            (
                p::SUBSETTING_SUBSETTING_FEATURE,
                SlotValue::Scalar(Value::Reference(id(1))),
            ),
            (
                p::SUBSETTING_SUBSETTED_FEATURE,
                SlotValue::Scalar(Value::Reference(id(2))),
            ),
        ]),
        Some(id(1)),
        &queries.canonical_fact_evidence(FactKey::Element(id(3))),
    )
    .unwrap();
    plan.add_derived_element(
        key(2),
        c::FEATURE_CHAINING,
        BTreeMap::from([(
            p::FEATURE_CHAINING_CHAINING_FEATURE,
            SlotValue::Scalar(Value::Reference(id(2))),
        )]),
        Some(id(1)),
        &queries.canonical_fact_evidence(FactKey::Element(id(4))),
    )
    .unwrap();
    let overlay = plan.materialize(&snapshot).unwrap().overlay;
    let context =
        SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap();
    let whole_slot = FactKey::Property {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    };
    for (producer_mode, queries) in [
        (false, KerMlQueries::new(context.fork())),
        (true, KerMlQueries::for_production(context.fork())),
    ] {
        let selected = queries
            .owned_relationships_of_type(id(1), c::SUBSETTING)
            .map(|_| ());
        // Compact producer evidence retains immediate DAG edges and searches,
        // rather than expanding declared leaves into positive_dependencies.
        assert!(
            selected
                .canonical_dependencies
                .contains(&Dependency::Derived(FactKey::Element(key(1).element_id())))
        );
        assert!(
            !selected
                .canonical_dependencies
                .contains(&Dependency::Derived(FactKey::Element(key(2).element_id())))
        );
        assert!(
            !selected
                .canonical_dependencies
                .contains(&Dependency::Derived(whole_slot))
        );
        if !producer_mode {
            assert!(
                selected
                    .positive_dependencies
                    .contains(&FactKey::Element(id(3)))
            );
            assert!(
                !selected
                    .positive_dependencies
                    .contains(&FactKey::Element(id(4)))
            );
        }
        let broad = queries.canonical_fact_evidence(whole_slot);
        assert!(
            broad
                .canonical_dependencies
                .contains(&Dependency::Derived(whole_slot))
        );
        if !producer_mode {
            assert!(
                broad
                    .positive_dependencies
                    .contains(&FactKey::Element(id(4)))
            );
        }
        for (mut combined, other) in [(selected.clone(), broad.clone()), (broad, selected)] {
            combined.merge_evidence(other).unwrap();
            assert!(
                combined
                    .canonical_dependencies
                    .contains(&Dependency::Derived(whole_slot))
            );
            if !producer_mode {
                assert!(
                    combined
                        .positive_dependencies
                        .contains(&FactKey::Element(id(4)))
                );
            }
            assert!(
                crate::producer_closure::producer_reads(&combined, overlay.model()).contains(
                    &crate::producer_closure::ProducerRead::Property(
                        id(1),
                        p::ELEMENT_OWNED_RELATIONSHIP
                    ),
                )
            );
            assert!(
                crate::read_dependencies::structural_searches(&combined).contains(
                    &agq_kernel::derived::StructuralSearch::Property {
                        element: id(1),
                        property: p::ELEMENT_OWNED_RELATIONSHIP,
                    },
                )
            );
        }
    }
}
