use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use agq_kernel::derived::{DerivationBuilder, StructuralSearch};

fn edge(f: &mut Fixture, relationship: u128, specific: u128, general: u128) {
    f.create(relationship, c::SUBCLASSIFICATION);
    f.value(
        relationship,
        p::SUBCLASSIFICATION_SUBCLASSIFIER,
        Value::Reference(id(specific)),
    );
    f.value(
        relationship,
        p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        Value::Reference(id(general)),
    );
}

fn path_fixture() -> Fixture {
    let mut f = Fixture::new();
    for n in [1, 2, 3, 4, 20, 21] {
        f.create(n, c::CLASSIFIER);
    }
    edge(&mut f, 10, 1, 2);
    edge(&mut f, 11, 2, 3);
    f
}

#[test]
fn selected_derived_path_retains_its_proof_without_unrelated_edge_proof() {
    let mut f = path_fixture();
    edge(&mut f, 12, 1, 4);
    let snapshot = f.finish();
    let slots = |n| {
        snapshot
            .model()
            .element(id(n))
            .unwrap()
            .slots()
            .map(|(property, slot)| (property, slot.value().clone()))
            .collect::<Vec<_>>()
    };
    let selected_slots = slots(10);
    let unrelated_slots = slots(12);
    let mut remove = snapshot.change_set();
    remove.remove(id(10));
    remove.remove(id(12));
    let declared = snapshot.apply(&remove).unwrap();
    let mut builder = DerivationBuilder::new(declared);
    let mut outputs = vec![];
    for (number, slots, support) in [(30, selected_slots, 20), (31, unrelated_slots, 21)] {
        let key = DerivationKey {
            rule: RuleId::from_u128(93001),
            subject: id(1),
            output: OutputKey::from_u128(number),
        };
        builder.element(
            key,
            c::SUBCLASSIFICATION,
            slots,
            BTreeSet::from([Dependency::Declared(FactKey::Element(id(support)))]),
        );
        builder.searches(
            FactKey::Element(key.element_id()),
            BTreeSet::from([StructuralSearch::ProducerClosure {
                subject: id(support),
                requirement: SemanticClosureRequirement::EffectiveTyping
                    .contract_id()
                    .into(),
            }]),
        );
        outputs.push(key.element_id());
    }
    let overlay = builder.build().unwrap();
    let context =
        SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap();
    for q in [
        KerMlQueries::new(context.fork()),
        KerMlQueries::for_production(context),
    ] {
        let witness = q.canonical_specialization_witness(id(1), id(3)).unwrap();
        assert_eq!(witness.completeness, Completeness::Complete);
        assert!(
            witness
                .canonical_dependencies
                .contains(&Dependency::Derived(FactKey::Element(outputs[0])))
        );
        assert!(
            !witness
                .positive_dependencies
                .contains(&FactKey::Element(outputs[1]))
        );
        let reads = crate::producer_closure::producer_reads(&witness, overlay.model());
        assert!(
            reads.contains(&crate::producer_closure::ProducerRead::Requirement(
                id(20),
                SemanticClosureRequirement::EffectiveTyping
            ))
        );
        assert!(
            !reads.contains(&crate::producer_closure::ProducerRead::Requirement(
                id(21),
                SemanticClosureRequirement::EffectiveTyping
            ))
        );
    }
}

#[test]
fn changed_or_removed_selected_edge_reopens_retained_producer_evaluation() {
    use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
    const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.PositivePathReader");
    let snapshot = path_fixture().finish();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        READER,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let witness = KerMlQueries::for_production(context.fork())
        .canonical_specialization_witness(id(1), id(3))
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
    }
    table
        .record(&[(id(1), READER, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(1), READER, producer_reads(&witness, snapshot.model()))],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    let checkpoint = certificate.checkpoint(&context).unwrap();
    for remove in [false, true] {
        let mut changes = snapshot.change_set();
        if remove {
            changes.remove(id(10));
        } else {
            changes.set(
                id(10),
                p::SUBCLASSIFICATION_SUPERCLASSIFIER,
                SlotValue::Scalar(Value::Reference(id(4))),
                origin(),
            );
        }
        let changed = snapshot.apply(&changes).unwrap();
        let next = SemanticContext::for_snapshot(&changed, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        assert!(
            KerMlQueries::new(next.fork())
                .canonical_specialization_witness(id(1), id(3))
                .is_none()
        );
        let rebound = checkpoint.rebind(&next, &registry).unwrap();
        assert_eq!(
            rebound
                .certificate
                .evaluation(id(1), registry.index(READER).unwrap()),
            Some(ProducerEvaluationState::Pending)
        );
    }
}

#[test]
fn implied_filter_and_cycles_do_not_create_false_positive_paths() {
    let mut f = path_fixture();
    edge(&mut f, 12, 2, 1);
    f.value(10, p::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true));
    let snapshot = f.finish();
    for exclude_implied in [false, true] {
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(
                &snapshot,
                SemanticOptions {
                    exclude_implied,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap(),
        );
        assert_eq!(
            q.canonical_specialization_witness(id(1), id(3)).is_some(),
            !exclude_implied
        );
        assert!(q.canonical_specialization_witness(id(1), id(4)).is_none());
        assert!(q.canonical_specialization_witness(id(1), id(1)).is_some());
    }
}

#[test]
fn positive_path_respects_conjugation_state_and_pending_providers() {
    use agq_kernel::derived::{ComputationFailure, IncompleteReason};
    let snapshot = path_fixture().finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    // Normal declared nodes leave this derived scalar absent; the no-conjugator
    // search and scalar guard still belong to their positive path.
    let absent = q.canonical_specialization_witness(id(1), id(3)).unwrap();
    assert!(
        absent
            .search_dependencies
            .contains(&SearchDependency::PropertySet {
                element: id(1),
                property: p::TYPE_IS_CONJUGATED
            })
    );
    for state in [0, 1, 2] {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        let proof = agq_kernel::provenance::Explanation {
            rule: RuleId::from_u128(93002),
            dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
        };
        match state {
            0 => {
                builder.property(
                    id(1),
                    p::TYPE_IS_CONJUGATED,
                    SlotValue::Scalar(Value::Boolean(true)),
                    proof,
                );
            }
            1 => {
                builder
                    .failure(
                        id(1),
                        p::TYPE_IS_CONJUGATED,
                        ComputationFailure::Incomplete {
                            reason: IncompleteReason::MissingInput,
                            explanation: proof,
                            searches: BTreeSet::new(),
                        },
                    )
                    .unwrap();
            }
            _ => {
                builder
                    .failure(
                        id(1),
                        p::TYPE_IS_CONJUGATED,
                        ComputationFailure::Invalid {
                            diagnostic: "fixture unknown conjugation".into(),
                            explanation: proof,
                            searches: BTreeSet::new(),
                        },
                    )
                    .unwrap();
            }
        }
        let overlay = builder.build().unwrap();
        let q = KerMlQueries::new(
            SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap(),
        );
        assert!(
            q.canonical_specialization_witness(id(1), id(3)).is_none(),
            "conjugation state {state}"
        );
    }
    let pending = SemanticContext::for_project_snapshot(
        &snapshot,
        Default::default(),
        BTreeSet::new(),
        BTreeSet::from([id(1)]),
        BTreeSet::new(),
    )
    .unwrap();
    assert!(
        KerMlQueries::new(pending)
            .canonical_specialization_witness(id(1), id(3))
            .is_none()
    );
    let mut f = path_fixture();
    f.create(13, c::CONJUGATION);
    f.own(1, 13);
    let construction = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(&construction, Default::default(), BTreeSet::new())
            .unwrap(),
    );
    assert!(q.canonical_specialization_witness(id(1), id(3)).is_none());
}

#[test]
fn chain_only_reachability_is_left_to_ordinary_semantic_queries() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::FEATURE);
    f.create(3, c::FEATURE_CHAINING);
    f.own(1, 3);
    f.value(
        3,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
        Value::Reference(id(2)),
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    assert!(q.all_supertypes(id(1)).value.contains(&id(2)));
    assert!(q.canonical_specialization_witness(id(1), id(2)).is_none());
}
