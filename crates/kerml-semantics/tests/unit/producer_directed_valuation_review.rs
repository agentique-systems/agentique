//! Directed valuation is a no-output proof; contextual binding is independent.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
use agq_kerml::BaselineProfile;
use agq_kernel::derived::{ComputationFailure, DerivationBuilder, IncompleteReason};
use agq_kernel::provenance::Explanation;

fn value_fixture(profile: BaselineProfile, direction: Option<&str>, result: bool) -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    populate(&mut f, c::FEATURE, direction, result);
    f.finish()
}

fn populate(f: &mut Fixture, class: MetaclassId, direction: Option<&str>, result: bool) {
    f.create(10, c::CLASSIFIER);
    f.create(1, class);
    member(f, 10, 1, 11, c::FEATURE_MEMBERSHIP);
    if let Some(direction) = direction {
        f.enumeration(1, p::FEATURE_DIRECTION, direction);
    }
    f.create(2, c::EXPRESSION);
    member(f, 1, 2, 4, c::FEATURE_VALUE);
    f.create(3, c::FEATURE);
    if result {
        member(f, 2, 3, 5, c::RETURN_PARAMETER_MEMBERSHIP);
    }
}

fn evaluation(plan: &ResultStructurePlan<'_>, family: ProducerFamily) -> Completeness {
    plan.producer_evaluations
        .iter()
        .find(|(s, f, _)| *s == id(1) && *f == family.id())
        .unwrap()
        .2
}

fn reads<'a>(plan: &'a ResultStructurePlan<'_>, family: ProducerFamily) -> &'a [ProducerRead] {
    &plan
        .producer_reads
        .iter()
        .find(|(s, f, _)| *s == id(1) && *f == family.id())
        .unwrap()
        .2
}

fn result_read(reads: &[ProducerRead]) -> bool {
    reads.contains(&ProducerRead::Owned(id(2), c::RETURN_PARAMETER_MEMBERSHIP))
}

#[test]
fn known_directions_separate_valuation_from_required_result_and_binding_context() {
    for profile in [
        BaselineProfile::PublishedKerMl10,
        BaselineProfile::OPERATIONAL_V9,
    ] {
        for direction in ["in", "out", "inout"] {
            for result in [false, true] {
                let snapshot = value_fixture(profile, Some(direction), result);
                for production in [false, true] {
                    let context = SemanticContext::for_snapshot(
                        &snapshot,
                        SemanticOptions {
                            baseline_profile: profile,
                            ..Default::default()
                        },
                        BTreeSet::new(),
                    )
                    .unwrap();
                    let q = if production {
                        KerMlQueries::for_production(context)
                    } else {
                        KerMlQueries::new(context)
                    };
                    let plan = q.plan_result_structure([id(1)]);
                    assert_eq!(
                        evaluation(&plan, ProducerFamily::FeatureValuation),
                        Completeness::Complete
                    );
                    assert_eq!(
                        evaluation(&plan, ProducerFamily::FeatureValue),
                        if result {
                            Completeness::Complete
                        } else {
                            Completeness::Incomplete
                        }
                    );
                    let valuation = reads(&plan, ProducerFamily::FeatureValuation);
                    assert!(
                        valuation.contains(&ProducerRead::Property(id(1), p::FEATURE_DIRECTION))
                    );
                    assert!(!result_read(valuation));
                    assert!(!valuation.contains(&ProducerRead::Owned(id(1), c::SPECIALIZATION)));
                    let binding = reads(&plan, ProducerFamily::FeatureValue);
                    assert!(result_read(binding));
                    if result {
                        assert!(
                            binding.contains(&ProducerRead::Owned(id(1), c::TYPE_FEATURING)),
                            "{binding:?}"
                        );
                        let output = plan.materialize(&snapshot).unwrap();
                        let q = KerMlQueries::new(
                            SemanticContext::for_overlay(
                                &output.overlay,
                                SemanticOptions {
                                    baseline_profile: profile,
                                    ..Default::default()
                                },
                                BTreeSet::new(),
                            )
                            .unwrap(),
                        );
                        assert!(
                            q.owned_relationships_of_type(id(1), c::SUBSETTING)
                                .value
                                .is_empty()
                        );
                        assert_eq!(
                            output.production.value.len(),
                            1,
                            "the real nondefault binding is still emitted"
                        );
                    }
                }
            }
        }
    }
}

fn alias_fixture() -> (Snapshot, PropertyId) {
    use agq_kernel::metamodel::{MetamodelRegistry, PropertyOwner};
    let class_id = MetaclassId::from_u128(0xd1ec701);
    let alias = PropertyId::from_u128(0xd1ec702);
    let mut descriptors = agq_kerml::descriptors();
    let mut class = descriptors
        .classes
        .iter()
        .find(|c| c.id == c::FEATURE)
        .unwrap()
        .clone();
    class.id = class_id;
    class.name = "ComputedValuationDirection".into();
    class.is_abstract = false;
    class.direct_supertypes = BTreeSet::from([c::FEATURE]);
    descriptors.classes.push(class);
    let mut property = descriptors
        .properties
        .iter()
        .find(|p| p.id == p::FEATURE_DIRECTION)
        .unwrap()
        .clone();
    property.id = alias;
    property.name = "computedValuationDirection".into();
    property.owner = PropertyOwner::Class(class_id);
    property.derived = true;
    property.redefines = BTreeSet::from([p::FEATURE_DIRECTION]);
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
    populate(&mut f, class_id, None, true);
    (f.finish(), alias)
}

#[test]
fn uncomputed_and_failed_direction_cannot_certify_valuation_nonapplicability() {
    let (snapshot, alias) = alias_fixture();
    for state in 0..3 {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        if state > 0 {
            let explanation = Explanation {
                rule: RuleId::from_u128(0xd1ec703),
                dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
            };
            let searches = BTreeSet::new();
            let failure = if state == 1 {
                ComputationFailure::Incomplete {
                    reason: IncompleteReason::MissingInput,
                    explanation,
                    searches,
                }
            } else {
                ComputationFailure::Invalid {
                    diagnostic: "invalid direction computation".into(),
                    explanation,
                    searches,
                }
            };
            builder.failure(id(1), alias, failure).unwrap();
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
            let plan = q.plan_result_structure([id(1)]);
            let expected = if state == 2 {
                Completeness::Invalid
            } else {
                Completeness::Incomplete
            };
            for family in [
                ProducerFamily::FeatureValuation,
                ProducerFamily::FeatureValue,
            ] {
                assert_eq!(
                    evaluation(&plan, family),
                    expected,
                    "state={state}, production={production}"
                );
                assert!(reads(&plan, family).contains(&ProducerRead::Property(id(1), alias)));
            }
            assert!(plan.planned_elements().next().is_none());
        }
    }
}

const RESULT_WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.DirectedValueResultWriter");
const VALUE_WRITER: ProducerFamilyId =
    ProducerFamilyId::new("Fixture.DirectedValuePopulationWriter");

fn certificate(
    context: &SemanticContext<'_>,
    registry: &ProducerRegistry,
    plan: &ResultStructurePlan<'_>,
    pending: Option<(ElementId, ProducerFamilyId)>,
) -> ProducerClosureCertificate {
    let mut table = ProducerEvaluationTable::default();
    for record in context.model().elements() {
        table.pending(record.id(), context.model(), registry);
    }
    let mut evaluations: Vec<_> = plan
        .producer_evaluations
        .iter()
        .copied()
        .filter(|(_, f, _)| registry.index(*f).is_some())
        .collect();
    if let Some((subject, family)) = pending {
        evaluations.push((subject, family, Completeness::Incomplete));
    }
    table.record(&evaluations, registry).unwrap();
    table.record_reads(&plan.producer_reads, registry);
    ProducerClosureCertificate::issue(context.model(), context.id(), registry, &table, |_| None)
}

#[test]
fn real_valuation_and_binding_keep_pending_result_and_value_writers() {
    for direction in [None, Some("in")] {
        let snapshot = value_fixture(Default::default(), direction, true);
        for (writer_id, class, relation, writer_subject) in [
            (
                RESULT_WRITER,
                c::EXPRESSION,
                c::RETURN_PARAMETER_MEMBERSHIP,
                id(2),
            ),
            (VALUE_WRITER, c::FEATURE, c::FEATURE_VALUE, id(1)),
        ] {
            let mut writer = ProducerDescriptor::new(
                writer_id,
                [ProducerEffect::Membership],
                ProducerApplicability::Subtypes(vec![class]),
            );
            writer.relationship_classes = Some(BTreeSet::from([relation]));
            writer.scoped_fresh_ownership = true;
            let registry = ProducerRegistry::new([
                ProducerFamily::FeatureValuation.descriptor(Default::default()),
                ProducerFamily::FeatureValue.descriptor(Default::default()),
                writer,
            ])
            .unwrap();
            let context =
                SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
                    .unwrap()
                    .with_producer_registry_digest(registry.digest())
                    .unwrap();
            let q = KerMlQueries::new(context.fork());
            let plan = q.plan_result_structure(snapshot.model().elements().map(|r| r.id()));
            assert_eq!(
                evaluation(&plan, ProducerFamily::FeatureValuation),
                Completeness::Complete
            );
            assert_eq!(
                result_read(reads(&plan, ProducerFamily::FeatureValuation)),
                direction.is_none()
            );
            let certificate = certificate(
                &context,
                &registry,
                &plan,
                Some((writer_subject, writer_id)),
            );
            let valuation_state = certificate.evaluation(
                id(1),
                registry
                    .index(ProducerFamily::FeatureValuation.id())
                    .unwrap(),
            );
            assert_eq!(
                valuation_state,
                Some(if direction.is_some() && writer_id == RESULT_WRITER {
                    ProducerEvaluationState::EvaluatedComplete
                } else {
                    ProducerEvaluationState::Pending
                }),
                "direction={direction:?}; writer={writer_id:?}"
            );
            assert_eq!(
                certificate.evaluation(
                    id(1),
                    registry.index(ProducerFamily::FeatureValue.id()).unwrap()
                ),
                Some(ProducerEvaluationState::Pending)
            );
        }
    }
}

#[test]
fn removing_direction_reopens_and_enables_real_valuation_without_changing_historical_outputs() {
    for profile in [
        BaselineProfile::PublishedKerMl10,
        BaselineProfile::OPERATIONAL_V9,
    ] {
        let registry = ProducerRegistry::new([
            ProducerFamily::FeatureValuation.descriptor(profile),
            ProducerFamily::FeatureValue.descriptor(profile),
        ])
        .unwrap();
        let options = SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        };
        let snapshot = value_fixture(profile, Some("in"), true);
        let context = SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let q = KerMlQueries::new(context.fork());
        let plan = q.plan_result_structure(snapshot.model().elements().map(|r| r.id()));
        let certificate = certificate(&context, &registry, &plan, None);
        assert_eq!(
            certificate.evaluation(
                id(1),
                registry
                    .index(ProducerFamily::FeatureValuation.id())
                    .unwrap()
            ),
            Some(ProducerEvaluationState::EvaluatedComplete)
        );
        let next = value_fixture(profile, None, true);
        let next_context = SemanticContext::for_snapshot(&next, options.clone(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let rebound = certificate
            .checkpoint(&context)
            .unwrap()
            .rebind(&next_context, &registry)
            .unwrap();
        assert_eq!(
            rebound.certificate.evaluation(
                id(1),
                registry
                    .index(ProducerFamily::FeatureValuation.id())
                    .unwrap()
            ),
            Some(ProducerEvaluationState::Pending)
        );
        let q = KerMlQueries::new(next_context);
        let plan = q.plan_result_structure([id(1)]);
        assert!(result_read(reads(&plan, ProducerFamily::FeatureValuation)));
        let output = plan.materialize(&next).unwrap();
        let q = KerMlQueries::new(
            SemanticContext::for_overlay(&output.overlay, options, BTreeSet::new()).unwrap(),
        );
        let subsettings = q.owned_relationships_of_type(id(1), c::SUBSETTING);
        assert_eq!(subsettings.value.len(), 1);
        let expected_target = if profile == BaselineProfile::PublishedKerMl10 {
            id(3)
        } else {
            let contextual = output
                .contextual_results
                .iter()
                .find(|result| result.rule == ResultDomainRule::FeatureValuation)
                .unwrap();
            assert_eq!(contextual.expression, id(2));
            assert_eq!(contextual.raw_result, id(3));
            contextual.feature
        };
        assert_eq!(
            output
                .overlay
                .model()
                .navigation_slot(subsettings.value[0], p::SUBSETTING_SUBSETTED_FEATURE)
                .unwrap()
                .value()
                .values()
                .cloned()
                .collect::<Vec<_>>(),
            [Value::Reference(expected_target)]
        );
        assert_eq!(output.production.value.len(), 1);
        let missing = value_fixture(profile, None, false);
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(
                &missing,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap(),
        );
        assert_eq!(
            evaluation(
                &q.plan_result_structure([id(1)]),
                ProducerFamily::FeatureValuation
            ),
            Completeness::Incomplete
        );
    }
}
