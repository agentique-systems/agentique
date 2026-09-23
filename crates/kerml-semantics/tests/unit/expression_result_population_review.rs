use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.ResultPopulationReader");
const CREATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.ResultPopulationCreator");
const SCALAR: ProducerFamilyId = ProducerFamilyId::new("Fixture.ResultPopulationScalar");
const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;

fn fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::FUNCTION);
    for (feature, carrier) in [
        (2, c::FEATURE_MEMBERSHIP),
        (3, c::END_FEATURE_MEMBERSHIP),
        (4, c::RETURN_PARAMETER_MEMBERSHIP),
    ] {
        f.create(feature, c::FEATURE);
        member(&mut f, 1, feature, feature + 10, carrier);
    }
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    f.value(3, p::FEATURE_IS_END, Value::Boolean(true));
    f.enumeration(4, p::FEATURE_DIRECTION, "out");
    f.finish()
}

fn registry(future: bool, unclassified: bool, direction_writer: bool) -> ProducerRegistry {
    let mut writer = ProducerFamily::ExpressionResult.descriptor(PROFILE);
    if unclassified {
        writer.feature_populations = None;
    }
    if future {
        writer.scope = ProducerEffectScope::SubjectAndOwningType;
    }
    let mut descriptors = vec![
        writer,
        ProducerDescriptor::new(
            READER,
            [],
            ProducerApplicability::Subtypes(vec![c::FUNCTION]),
        ),
    ];
    if future {
        let mut creator = ProducerDescriptor::new(
            CREATOR,
            [ProducerEffect::Membership],
            ProducerApplicability::Subtypes(vec![c::FUNCTION]),
        );
        creator.relationship_classes = Some(BTreeSet::from([c::OWNING_MEMBERSHIP]));
        creator.feature_populations = Some(BTreeSet::new());
        creator.scoped_fresh_ownership = true;
        descriptors.push(creator);
    }
    if direction_writer {
        descriptors.push(ProducerDescriptor::new(
            SCALAR,
            [ProducerEffect::Scalar(p::FEATURE_DIRECTION)],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        ));
    }
    ProducerRegistry::new(descriptors).unwrap()
}

fn context<'a>(snapshot: &'a Snapshot, registry: &ProducerRegistry) -> SemanticContext<'a> {
    SemanticContext::for_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: PROFILE,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap()
}

fn issue<T>(
    q: &KerMlQueries<'_>,
    registry: &ProducerRegistry,
    answer: &QueryResult<T>,
    future: bool,
) -> ProducerClosureCertificate {
    let mut table = ProducerEvaluationTable::default();
    for record in q.model().elements() {
        table.pending(record.id(), q.model(), registry);
        for descriptor in registry.descriptors() {
            if !descriptor
                .applicability
                .applies(q.model(), record.metaclass())
            {
                continue;
            }
            let state = if record.id() == id(1)
                && descriptor.id
                    == if future {
                        CREATOR
                    } else {
                        ProducerFamily::ExpressionResult.id()
                    } {
                Completeness::Incomplete
            } else if record.id() == id(1) && descriptor.id == READER {
                answer.completeness
            } else {
                Completeness::Complete
            };
            table
                .record(&[(record.id(), descriptor.id, state)], registry)
                .unwrap();
        }
    }
    table.record_reads(
        &[(id(1), READER, producer_reads(answer, q.model()))],
        registry,
    );
    ProducerClosureCertificate::issue(q.model(), q.context(), registry, &table, |_| None)
}

#[test]
fn result_binding_population_excludes_parameters_but_keeps_ends_unknown_and_direction_writers() {
    let snapshot = fixture();
    for future in [false, true] {
        for (unclassified, direction_writer) in [(false, false), (true, false), (false, true)] {
            let registry = registry(future, unclassified, direction_writer);
            let q = KerMlQueries::for_production(context(&snapshot, &registry));
            for (answer, expected) in [
                (
                    q.owned_parameter_features(id(1)),
                    if unclassified || direction_writer {
                        ProducerEvaluationState::Pending
                    } else {
                        ProducerEvaluationState::EvaluatedComplete
                    },
                ),
                (
                    q.owned_end_features(id(1)),
                    ProducerEvaluationState::Pending,
                ),
            ] {
                assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
                let certificate = issue(&q, &registry, &answer, future);
                assert_eq!(
                    certificate.evaluation(id(1), registry.index(READER).unwrap()),
                    Some(expected),
                    "future={future}; unknown={unclassified}; scalar={direction_writer}; {answer:?}"
                );
            }
        }
    }
}

#[test]
fn explicit_broad_membership_read_survives_result_population_merge_in_both_modes() {
    let snapshot = fixture();
    for future in [false, true] {
        let registry = registry(future, false, false);
        for production in [false, true] {
            let context = context(&snapshot, &registry);
            let q = if production {
                KerMlQueries::for_production(context)
            } else {
                KerMlQueries::new(context)
            };
            for reverse in [false, true] {
                let selected = q.owned_parameter_features(id(1));
                let broad = q.owned_relationships_of_type(id(1), c::MEMBERSHIP);
                let combined = if reverse {
                    let mut out = broad;
                    out.merge(selected);
                    out
                } else {
                    let mut out = selected;
                    out.merge(broad);
                    out
                };
                assert_eq!(
                    issue(&q, &registry, &combined, future)
                        .evaluation(id(1), registry.index(READER).unwrap()),
                    Some(ProducerEvaluationState::Pending)
                );
            }
        }
    }
}

#[test]
fn pending_namespace_can_still_supply_directed_parameters() {
    let snapshot = fixture();
    let registry = registry(false, false, false);
    let context = SemanticContext::for_project_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: PROFILE,
            ..Default::default()
        },
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::from([id(1)]),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let q = KerMlQueries::for_production(context);
    let answer = q.owned_parameter_features(id(1));
    assert_eq!(answer.value, vec![id(2)]);
    assert_ne!(
        issue(&q, &registry, &answer, false).evaluation(id(1), registry.index(READER).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
}

#[test]
fn result_population_promise_is_audited_for_new_direction_and_result_carriers() {
    let snapshot = fixture();
    let q = KerMlQueries::new(context(&snapshot, &registry(false, false, false)));
    for (carrier, direction, end, allowed) in [
        (c::FEATURE_MEMBERSHIP, false, false, true),
        (c::END_FEATURE_MEMBERSHIP, false, true, true),
        (c::FEATURE_MEMBERSHIP, true, false, false),
        (c::RETURN_PARAMETER_MEMBERSHIP, false, false, false),
    ] {
        let mut descriptor = ProducerFamily::ExpressionResult.descriptor(PROFILE);
        // Permit this class solely to independently test the Result-population
        // promise; the real descriptor already rejects this relationship class.
        descriptor
            .relationship_classes
            .as_mut()
            .unwrap()
            .insert(c::RETURN_PARAMETER_MEMBERSHIP);
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let key = |output| DerivationKey {
            subject: id(1),
            rule: RuleId::from_u128(981_220),
            output: OutputKey::from_u128(output),
        };
        let mut plan = q.plan_result_structure([]);
        let mut slots =
            BTreeMap::from([(p::FEATURE_IS_END, SlotValue::Scalar(Value::Boolean(end)))]);
        if direction {
            slots.insert(
                p::FEATURE_DIRECTION,
                snapshot
                    .model()
                    .navigation_slot(id(2), p::FEATURE_DIRECTION)
                    .unwrap()
                    .value()
                    .clone(),
            );
        }
        let evidence = q.canonical_fact_evidence(FactKey::Element(id(1)));
        plan.add_derived_element(key(1), c::FEATURE, slots, None, &evidence)
            .unwrap();
        plan.add_derived_element(
            key(2),
            carrier,
            BTreeMap::from([(
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(key(1).element_id())]),
            )]),
            Some(id(1)),
            &evidence,
        )
        .unwrap();
        let outputs = plan.planned_elements().collect::<Vec<_>>();
        plan.attribute_producer_outputs(id(1), ProducerFamily::ExpressionResult.id(), outputs);
        let audit = plan.validate_declared_effects(&[id(1)], &registry);
        assert_eq!(audit.is_ok(), allowed, "{audit:?}");
        if !allowed {
            assert!(
                matches!(audit,Err(PublicationOverlayError::ProducerEffectViolation(failure)) if failure.operation=="relationship semantic effect")
            );
        }
    }
}

#[test]
fn changed_result_population_contract_rejects_old_certificate_identity() {
    let snapshot = fixture();
    let old = registry(false, true, false);
    let new = registry(false, false, false);
    assert_ne!(old.digest(), new.digest());
    let old_context = context(&snapshot, &old);
    let certificate = ProducerClosureCertificate::initial(&old_context, &old).unwrap();
    assert!(
        context(&snapshot, &new)
            .with_producer_closure(Arc::new(certificate))
            .is_err()
    );
}
