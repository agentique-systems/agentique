use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, ProducerRead, producer_reads};
use agq_kernel::{derived::DerivationBuilder, provenance::Explanation as KernelExplanation};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.ReferentPrefixReader");

fn fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::FEATURE_REFERENCE_EXPRESSION);
    f.create(3, c::FEATURE);
    member(&mut f, 1, 3, 2, c::PARAMETER_MEMBERSHIP);
    f.create(12, c::CLASSIFIER);
    relation(
        &mut f,
        1,
        12,
        6,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(6, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
    for (membership, target) in [(4, 10), (7, 11)] {
        f.create(target, c::FEATURE);
        f.create(membership, c::MEMBERSHIP);
        f.value(
            membership,
            p::MEMBERSHIP_MEMBER_ELEMENT,
            Value::Reference(id(target)),
        );
        f.own(1, membership);
    }
    f.finish()
}

fn options() -> SemanticOptions {
    SemanticOptions {
        baseline_profile: PROFILE,
        ..Default::default()
    }
}

#[test]
fn prefix_witness_and_current_population_reads_survive_both_merge_orders() {
    let snapshot = fixture();
    for production in [false, true] {
        let context = SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap();
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        let referent = q.reference_referent(id(1));
        assert_eq!(referent.value, Some(id(10)));
        assert_eq!(referent.completeness, Completeness::Complete);
        let reads = producer_reads(&referent, snapshot.model());
        assert!(reads.contains(&ProducerRead::DeclaredProperty(
            id(1),
            p::ELEMENT_OWNED_RELATIONSHIP
        )));
        assert!(!reads.contains(&ProducerRead::Structural(id(1))));
        let current = q.memberships(id(1));
        let current_fact = q.canonical_fact_evidence(FactKey::Property {
            element: id(1),
            property: p::ELEMENT_OWNED_RELATIONSHIP,
        });
        for prefix_first in [false, true] {
            let mut mixed = q.canonical_fact_evidence(FactKey::Element(id(1)));
            if prefix_first {
                mixed.merge(referent.clone());
                mixed.merge(current.clone());
                mixed.merge(current_fact.clone());
            } else {
                mixed.merge(current_fact.clone());
                mixed.merge(current.clone());
                mixed.merge(referent.clone());
            }
            let reads = producer_reads(&mixed, snapshot.model());
            assert!(reads.contains(&ProducerRead::DeclaredProperty(
                id(1),
                p::ELEMENT_OWNED_RELATIONSHIP
            )));
            assert!(reads.contains(&ProducerRead::Structural(id(1))));
            assert!(reads.contains(&ProducerRead::Property(
                id(1),
                p::ELEMENT_OWNED_RELATIONSHIP
            )));
        }
    }
}

#[test]
fn prefix_does_not_skip_unknown_first_endpoint_or_pending_source_population() {
    let snapshot = fixture();
    let mut edit = snapshot.change_set();
    edit.clear(id(4), p::MEMBERSHIP_MEMBER_ELEMENT);
    let construction = snapshot.preview(&edit).unwrap();
    let context =
        SemanticContext::for_construction(&construction, options(), BTreeSet::new()).unwrap();
    let result = KerMlQueries::new(context).reference_referent(id(1));
    assert_eq!(
        result.value, None,
        "later established member11 cannot replace first unknown member4"
    );
    assert_eq!(result.completeness, Completeness::Incomplete);
    let context = SemanticContext::for_project_snapshot(
        &snapshot,
        options(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::from([id(1)]),
    )
    .unwrap();
    let result = KerMlQueries::for_production(context).reference_referent(id(1));
    assert_eq!(result.value, Some(id(10)));
    assert_eq!(result.completeness, Completeness::Incomplete);
    assert!(producer_reads(&result, snapshot.model()).contains(&ProducerRead::Structural(id(1))));
}

#[test]
fn prefix_checkpoint_reopens_prepend_remove_reorder_and_endpoint_retarget() {
    let snapshot = fixture();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        READER,
        [],
        ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let result = KerMlQueries::for_production(context.fork()).reference_referent(id(1));
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(1), snapshot.model(), &registry);
    table
        .record(&[(id(1), READER, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(1), READER, producer_reads(&result, snapshot.model()))],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert_eq!(
        certificate.evaluation(id(1), registry.index(READER).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
    for case in ["prepend", "remove", "reorder", "retarget"] {
        let mut edit = snapshot.change_set();
        match case {
            "prepend" => {
                edit.create(id(8), c::MEMBERSHIP, origin());
                for (property, slot) in snapshot.model().element(id(7)).unwrap().slots() {
                    edit.set(id(8), property, slot.value().clone(), origin());
                }
                edit.set(
                    id(1),
                    p::ELEMENT_OWNED_RELATIONSHIP,
                    SlotValue::Ordered([8, 2, 6, 4, 7].map(|n| Value::Reference(id(n))).to_vec()),
                    origin(),
                );
            }
            "remove" => {
                edit.set(
                    id(1),
                    p::ELEMENT_OWNED_RELATIONSHIP,
                    SlotValue::Ordered([2, 6, 7].map(|n| Value::Reference(id(n))).to_vec()),
                    origin(),
                );
                edit.remove(id(4));
            }
            "reorder" => {
                edit.set(
                    id(1),
                    p::ELEMENT_OWNED_RELATIONSHIP,
                    SlotValue::Ordered([2, 6, 7, 4].map(|n| Value::Reference(id(n))).to_vec()),
                    origin(),
                );
            }
            "retarget" => {
                edit.set(
                    id(4),
                    p::MEMBERSHIP_MEMBER_ELEMENT,
                    SlotValue::Scalar(Value::Reference(id(11))),
                    origin(),
                );
            }
            _ => unreachable!(),
        }
        let changed = snapshot.apply(&edit).unwrap();
        let next = SemanticContext::for_snapshot(&changed, options(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        assert_eq!(
            KerMlQueries::new(next.fork())
                .reference_referent(id(1))
                .value,
            Some(id(11)),
            "{case}"
        );
        let rebound = certificate.rebind(&context, &next, &registry).unwrap();
        assert_eq!(
            rebound
                .certificate
                .evaluation(id(1), registry.index(READER).unwrap()),
            Some(ProducerEvaluationState::Pending),
            "{case}"
        );
    }
}

#[test]
fn declared_first_survives_append_but_derived_only_first_keeps_current_proof() {
    for original_candidate in [false, true] {
        let base = fixture();
        let slots: BTreeMap<_, _> = base
            .model()
            .element(id(7))
            .unwrap()
            .slots()
            .map(|(property, slot)| (property, slot.value().clone()))
            .collect();
        let snapshot = if original_candidate {
            base
        } else {
            let mut edit = base.change_set();
            edit.set(
                id(1),
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered([2, 6].map(|n| Value::Reference(id(n))).to_vec()),
                origin(),
            );
            edit.remove(id(4));
            edit.remove(id(7));
            base.apply(&edit).unwrap()
        };
        let key = DerivationKey {
            subject: id(1),
            rule: RuleId::from_u128(900),
            output: OutputKey::from_u128(901),
        };
        let mut builder = DerivationBuilder::new(snapshot);
        builder.element(
            key,
            c::MEMBERSHIP,
            slots,
            BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
        );
        builder.extend_ordered_references(
            id(1),
            p::ELEMENT_OWNED_RELATIONSHIP,
            vec![key.element_id()],
            KernelExplanation {
                rule: key.rule,
                dependencies: BTreeSet::from([Dependency::Derived(FactKey::Element(
                    key.element_id(),
                ))]),
            },
        );
        builder.searches(
            FactKey::Element(key.element_id()),
            BTreeSet::from([agq_kernel::derived::StructuralSearch::Incoming(id(11))]),
        );
        let overlay = builder.build().unwrap();
        for archive in [false, true] {
            let restored;
            let overlay = if archive {
                let mut bytes = vec![];
                agq_kernel::archive::write_overlay(&overlay, &mut bytes).unwrap();
                restored = agq_kernel::archive::read_overlay(
                    std::io::Cursor::new(bytes),
                    Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()),
                )
                .unwrap();
                &restored
            } else {
                &overlay
            };
            for production in [false, true] {
                let context =
                    SemanticContext::for_overlay(overlay, options(), BTreeSet::new()).unwrap();
                let q = if production {
                    KerMlQueries::for_production(context)
                } else {
                    KerMlQueries::new(context)
                };
                let answer = q.reference_referent(id(1));
                assert_eq!(
                    answer.value,
                    Some(id(if original_candidate { 10 } else { 11 }))
                );
                assert_eq!(answer.completeness, Completeness::Complete);
                let reads = producer_reads(&answer, overlay.model());
                assert_eq!(
                    reads.contains(&ProducerRead::Structural(id(1))),
                    !original_candidate
                );
                assert_eq!(reads.contains(&ProducerRead::Inverse), !original_candidate);
                if !original_candidate {
                    assert!(
                        answer
                            .canonical_dependencies
                            .contains(&Dependency::Derived(FactKey::Element(key.element_id())))
                    );
                }
            }
        }
    }
}
