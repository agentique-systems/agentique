//! Selected ownership entries retain their append/adoption proofs independently.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, ProducerRead, producer_reads};
use agq_kernel::derived::{DerivationBuilder, DerivedOverlay, StructuralSearch};
use agq_kernel::provenance::Explanation;

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.SelectedContributionReader");

fn property(element: u128) -> FactKey {
    FactKey::Property {
        element: id(element),
        property: p::ELEMENT_DECLARED_NAME,
    }
}

fn fixture(dependency: Option<Arc<DerivedOverlay>>, changed_guard: bool) -> Snapshot {
    let base = dependency.map_or_else(
        || Snapshot::new(Arc::new(agq_kerml::registry().unwrap())),
        Snapshot::with_immutable_dependency,
    );
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for n in [1, 2] {
        f.create(n, c::CLASSIFIER);
    }
    for n in [6, 20, 21, 22] {
        f.create(n, c::FEATURE);
    }
    for n in [21, 22] {
        f.value(n, p::ELEMENT_DECLARED_NAME, Value::String(format!("guard{n}")));
    }
    f.value(
        20,
        p::ELEMENT_DECLARED_NAME,
        Value::String(if changed_guard { "after" } else { "before" }.into()),
    );
    for n in [3, 4] {
        f.create(n, c::SUBCLASSIFICATION);
        f.value(
            n,
            p::SUBCLASSIFICATION_SUBCLASSIFIER,
            Value::Reference(id(1)),
        );
        f.value(
            n,
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
            Value::Reference(id(2)),
        );
    }
    // 3 is deliberately detached: its declared existence does not prove adoption.
    f.own(1, 4);
    f.create(5, c::FEATURE_MEMBERSHIP);
    f.changes.set(
        id(5),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(6))]),
        origin(),
    );
    f.finish()
}

fn selected_key() -> DerivationKey {
    DerivationKey {
        rule: RuleId::from_u128(94001),
        subject: id(1),
        output: OutputKey::from_u128(94002),
    }
}

fn append_fixture(snapshot: Snapshot, adopt_declared: bool) -> (DerivedOverlay, ElementId) {
    let mut builder = DerivationBuilder::new(snapshot.clone());
    let selected = if adopt_declared {
        id(3)
    } else {
        let slots: Vec<_> = snapshot
            .model()
            .element(id(3))
            .unwrap()
            .slots()
            .map(|(property, slot)| (property, slot.value().clone()))
            .collect();
        builder.element(
            selected_key(),
            c::SUBCLASSIFICATION,
            slots,
            BTreeSet::from([Dependency::Declared(property(22))]),
        );
        selected_key().element_id()
    };
    let owner_slot = FactKey::Property {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    };
    builder.extend_ordered_references(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![selected],
        Explanation {
            rule: RuleId::from_u128(94003),
            dependencies: BTreeSet::from([Dependency::Declared(property(20))]),
        },
    );
    builder.searches(
        owner_slot,
        BTreeSet::from([StructuralSearch::Property {
            element: id(20),
            property: p::ELEMENT_DECLARED_NAME,
        }]),
    );
    let mut next = DerivationBuilder::from_overlay(builder.build().unwrap());
    // A later snapshot-member adoption observes a distinct, unrelated guard.
    next.extend_ordered_references(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![id(5)],
        Explanation {
            rule: RuleId::from_u128(94004),
            dependencies: BTreeSet::from([Dependency::Declared(property(21))]),
        },
    );
    next.searches(
        owner_slot,
        BTreeSet::from([StructuralSearch::ProducerClosure {
            subject: id(21),
            requirement: SemanticClosureRequirement::EffectiveTyping
                .contract_id()
                .into(),
        }]),
    );
    (next.build().unwrap(), selected)
}

fn context<'m>(overlay: &'m DerivedOverlay, registry: &ProducerRegistry) -> SemanticContext<'m> {
    SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
}

fn registry() -> ProducerRegistry {
    ProducerRegistry::new([ProducerDescriptor::new(
        READER,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
    )])
    .unwrap()
}

fn certificate(
    overlay: &DerivedOverlay,
    context: &SemanticContext<'_>,
    registry: &ProducerRegistry,
    answer: &QueryResult<Vec<ElementId>>,
) -> ProducerClosureCertificate {
    let mut table = ProducerEvaluationTable::default();
    for record in overlay.model().elements() {
        table.pending(record.id(), overlay.model(), registry);
    }
    table
        .record(&[(id(1), READER, Completeness::Complete)], registry)
        .unwrap();
    table.record_reads(
        &[(id(1), READER, producer_reads(answer, overlay.model()))],
        registry,
    );
    ProducerClosureCertificate::issue(overlay.model(), context.id(), registry, &table, |_| None)
}

#[test]
fn selected_derived_and_declared_adopted_relationships_keep_only_their_append_support() {
    for adopt_declared in [false, true] {
        let (overlay, selected) = append_fixture(fixture(None, false), adopt_declared);
        let registry = registry();
        for production in [false, true] {
            let context = context(&overlay, &registry);
            let q = if production {
                KerMlQueries::for_production(context)
            } else {
                KerMlQueries::new(context)
            };
            let answer = q.owned_relationships_of_type(id(1), c::SPECIALIZATION);
            assert_eq!(answer.completeness, Completeness::Complete);
            assert_eq!(answer.value, [id(4), selected]);
            assert!(
                answer.positive_dependencies.contains(&property(20)),
                "adoption proof is independent of the selected element's own origin"
            );
            if !adopt_declared {
                assert!(
                    answer.positive_dependencies.contains(&property(22)),
                    "selected derived element still needs its creation proof"
                );
            }
            assert!(
                !answer.positive_dependencies.contains(&property(21)),
                "unselected snapshot append must not enter the specialization proof"
            );
            assert!(
                answer
                    .canonical_dependencies
                    .contains(&Dependency::Declared(FactKey::Property {
                        element: id(1),
                        property: p::ELEMENT_OWNED_RELATIONSHIP,
                    })),
                "mixed original and derived selection retains the original entry"
            );
            let reads = producer_reads(&answer, overlay.model());
            assert!(reads.contains(&ProducerRead::Property(id(20), p::ELEMENT_DECLARED_NAME)));
            assert!(!reads.contains(&ProducerRead::Requirement(
                id(21),
                SemanticClosureRequirement::EffectiveTyping
            )));
            assert!(
                producer_reads(&q.owned_relationships(id(1)), overlay.model()).contains(
                    &ProducerRead::Requirement(id(21), SemanticClosureRequirement::EffectiveTyping)
                )
            );
        }
    }
}

#[test]
fn changed_selected_adoption_guard_reopens_checkpoint_with_identical_membership_values() {
    let registry = registry();
    for adopt_declared in [false, true] {
        let (before, _) = append_fixture(fixture(None, false), adopt_declared);
        let (after, _) = append_fixture(fixture(None, true), adopt_declared);
        assert_eq!(before.model().element(id(1)), after.model().element(id(1)));
        let old_context = context(&before, &registry);
        let next_context = context(&after, &registry);
        let answer = KerMlQueries::for_production(old_context.fork())
            .owned_relationships_of_type(id(1), c::SPECIALIZATION);
        let certificate = certificate(&before, &old_context, &registry, &answer);
        assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
        let rebound = certificate
            .checkpoint(&old_context)
            .unwrap()
            .rebind(&next_context, &registry)
            .unwrap();
        assert_eq!(
            rebound
                .certificate
                .evaluation(id(1), registry.index(READER).unwrap()),
            Some(ProducerEvaluationState::Pending)
        );
    }
}

#[test]
fn dependent_archive_without_contribution_metadata_falls_back_and_reopens_precision() {
    use agq_kernel::archive::{read_dependent_overlay, write_dependent_overlay};
    use std::io::Cursor;
    let mut dependency = Fixture::new();
    dependency.create(99, c::PACKAGE);
    let dependency = Arc::new(DerivationBuilder::new(dependency.finish()).build().unwrap());
    let (overlay, selected) = append_fixture(fixture(Some(dependency.clone()), false), true);
    let mut bytes = vec![];
    write_dependent_overlay(&overlay, &mut bytes).unwrap();
    let restored = read_dependent_overlay(
        Cursor::new(&bytes),
        Arc::new(agq_kerml::registry().unwrap()),
        dependency.clone(),
    )
    .unwrap();
    assert!(Arc::ptr_eq(
        restored.declared().immutable_dependency().unwrap(),
        &dependency
    ));
    assert!(
        overlay
            .model()
            .ordered_reference_contribution(id(1), p::ELEMENT_OWNED_RELATIONSHIP, selected)
            .is_some()
    );
    assert!(
        restored
            .model()
            .ordered_reference_contribution(id(1), p::ELEMENT_OWNED_RELATIONSHIP, selected)
            .is_none()
    );
    let registry = registry();
    let old_context = context(&overlay, &registry);
    let next_context = context(&restored, &registry);
    let answer = KerMlQueries::for_production(old_context.fork())
        .owned_relationships_of_type(id(1), c::SPECIALIZATION);
    let fallback = KerMlQueries::for_production(next_context.fork())
        .owned_relationships_of_type(id(1), c::SPECIALIZATION);
    assert_eq!(answer.value, fallback.value);
    assert!(fallback.positive_dependencies.contains(&property(20)));
    assert!(fallback.positive_dependencies.contains(&property(21)));
    assert!(
        producer_reads(&fallback, restored.model()).contains(&ProducerRead::Requirement(
            id(21),
            SemanticClosureRequirement::EffectiveTyping
        ))
    );
    let certificate = certificate(&overlay, &old_context, &registry, &answer);
    let rebound = certificate
        .checkpoint(&old_context)
        .unwrap()
        .rebind(&next_context, &registry)
        .unwrap();
    assert_eq!(
        rebound
            .certificate
            .evaluation(id(1), registry.index(READER).unwrap()),
        Some(ProducerEvaluationState::Pending)
    );
}
