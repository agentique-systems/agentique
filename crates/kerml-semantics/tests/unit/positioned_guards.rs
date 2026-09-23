//! Exclusion guards must survive reconstruction and unknown scalar state.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
use agq_kernel::derived::{
    ComputationFailure, DerivationBuilder, IncompleteReason, PropertyState, StructuralSearch,
};

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.PositionedReader");

fn carrier_fixture(class: MetaclassId) -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::BEHAVIOR);
    f.create(2, c::FEATURE);
    f.create(4, c::CLASSIFIER);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    f.value(2, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 1, 2, 3, class);
    if class == c::MEMBERSHIP {
        f.value(3, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(2)));
    } else if class == c::FEATURE_TYPING {
        f.value(3, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(2)));
        f.value(3, p::FEATURE_TYPING_TYPE, Value::Reference(id(4)));
    }
    f.finish()
}

#[test]
fn positioned_guard_class_reconstruction_reopens_filtered_memberships() {
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        READER,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::BEHAVIOR]),
    )])
    .unwrap();
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    for original_class in [
        c::FEATURE_TYPING,
        c::MEMBERSHIP,
        c::RETURN_PARAMETER_MEMBERSHIP,
    ] {
        let original = carrier_fixture(original_class);
        let changed = carrier_fixture(c::FEATURE_MEMBERSHIP);
        assert_eq!(
            original.model().element(id(1)),
            changed.model().element(id(1)),
            "only the carrier changed; owner-only invalidation is insufficient"
        );
        let context = SemanticContext::for_snapshot(&original, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let next = SemanticContext::for_snapshot(&changed, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        for production in [false, true] {
            let q = if production {
                KerMlQueries::for_production(context.fork())
            } else {
                KerMlQueries::new(context.fork())
            };
            let mut populations = vec![q.owned_parameter_features(id(1))];
            if original_class != c::RETURN_PARAMETER_MEMBERSHIP {
                populations.push(q.owned_end_features(id(1)));
            }
            for answer in populations {
                assert_eq!(answer.completeness, Completeness::Complete);
                assert!(answer.value.is_empty());
                assert!(
                    answer
                        .search_dependencies
                        .contains(&SearchDependency::Kernel(
                            StructuralSearch::ElementIdentity(id(3))
                        ))
                );
                let mut table = ProducerEvaluationTable::default();
                for record in original.model().elements() {
                    table.pending(record.id(), original.model(), &registry);
                }
                table
                    .record(&[(id(1), READER, Completeness::Complete)], &registry)
                    .unwrap();
                table.record_reads(
                    &[(id(1), READER, producer_reads(&answer, original.model()))],
                    &registry,
                );
                let certificate = ProducerClosureCertificate::issue(
                    original.model(),
                    context.id(),
                    &registry,
                    &table,
                    |_| None,
                );
                assert!(certificate.is_closed(id(1), requirement));
                let checkpoint = certificate.checkpoint(&context).unwrap();
                let rebound = checkpoint.rebind(&next, &registry).unwrap();
                assert!(rebound.affected_subjects.contains(&id(3)));
                assert_eq!(
                    rebound.reopened_evaluations, 1,
                    "{original_class:?}; production={production}"
                );
                assert!(!rebound.certificate.is_closed(id(1), requirement));
            }
        }
        let q = KerMlQueries::new(next);
        assert_eq!(q.owned_parameter_features(id(1)).value, [id(2)]);
        assert_eq!(q.owned_end_features(id(1)).value, [id(2)]);
    }
}

fn derived_flag_fixture(original: PropertyId) -> (Snapshot, PropertyId) {
    use agq_kernel::metamodel::{MetamodelRegistry, PropertyOwner};
    let custom_class = MetaclassId::from_u128(0xfea201);
    let alias = PropertyId::from_u128(0xfea202);
    let mut descriptors = agq_kerml::descriptors();
    let mut class = descriptors
        .classes
        .iter()
        .find(|class| class.id == c::FEATURE)
        .unwrap()
        .clone();
    class.id = custom_class;
    class.name = "FixtureComputedDirectionOrEnd".into();
    class.is_abstract = false;
    class.direct_supertypes = BTreeSet::from([c::FEATURE]);
    descriptors.classes.push(class);
    let mut property = descriptors
        .properties
        .iter()
        .find(|property| property.id == original)
        .unwrap()
        .clone();
    property.id = alias;
    property.name = "fixtureComputedFlag".into();
    property.owner = PropertyOwner::Class(custom_class);
    property.derived = true;
    property.redefines = BTreeSet::from([original]);
    property.association = None;
    property.opposite_ends.clear();
    descriptors.properties.push(property);
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::BEHAVIOR);
    f.create(2, custom_class);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    (f.finish(), alias)
}

#[test]
fn positioned_guard_computed_flags_preserve_unknown_and_failed_states() {
    for original in [p::FEATURE_DIRECTION, p::FEATURE_IS_END] {
        let (snapshot, alias) = derived_flag_fixture(original);
        assert!(matches!(
            snapshot.model().property_state(id(2), original).unwrap(),
            PropertyState::NotComputed
        ));
        for (state, expected) in [
            (0, Completeness::Incomplete),
            (1, Completeness::Incomplete),
            (2, Completeness::Invalid),
        ] {
            let mut builder = DerivationBuilder::new(snapshot.clone());
            let proof = agq_kernel::provenance::Explanation {
                rule: RuleId::from_u128(0xfea203),
                dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(2)))]),
            };
            let searches = BTreeSet::from([StructuralSearch::ElementIdentity(id(1))]);
            if state == 1 {
                builder
                    .failure(
                        id(2),
                        alias,
                        ComputationFailure::Incomplete {
                            reason: IncompleteReason::MissingInput,
                            explanation: proof,
                            searches,
                        },
                    )
                    .unwrap();
            } else if state == 2 {
                builder
                    .failure(
                        id(2),
                        alias,
                        ComputationFailure::Invalid {
                            diagnostic: "fixture invalid direction/end computation".into(),
                            explanation: proof,
                            searches,
                        },
                    )
                    .unwrap();
            }
            let overlay = builder.build().unwrap();
            for production in [false, true] {
                let context =
                    SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
                        .unwrap();
                let q = if production {
                    KerMlQueries::for_production(context)
                } else {
                    KerMlQueries::new(context)
                };
                let answer = if original == p::FEATURE_DIRECTION {
                    q.owned_parameter_features(id(1))
                } else {
                    q.owned_end_features(id(1))
                };
                assert!(answer.value.is_empty());
                assert_eq!(
                    answer.completeness, expected,
                    "original={original}; state={state}; production={production}: {answer:?}"
                );
                assert!(!answer.diagnostics.is_empty());
                assert!(
                    answer
                        .search_dependencies
                        .contains(&SearchDependency::PropertySet {
                            element: id(2),
                            property: alias
                        })
                );
                if state != 0 {
                    assert!(answer.canonical_dependencies.contains(&Dependency::Derived(
                        FactKey::Property {
                            element: id(2),
                            property: alias
                        }
                    )));
                }
            }
        }
    }
}

#[test]
fn positioned_guard_known_absent_direction_and_false_end_remain_complete() {
    let mut f = Fixture::new();
    f.create(1, c::BEHAVIOR);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    assert!(matches!(
        snapshot
            .model()
            .property_state(id(2), p::FEATURE_DIRECTION)
            .unwrap(),
        PropertyState::Absent
    ));
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    for answer in [
        q.owned_parameter_features(id(1)),
        q.owned_end_features(id(1)),
    ] {
        assert!(answer.value.is_empty());
        assert_eq!(answer.completeness, Completeness::Complete);
    }
}
