use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.ArgumentPopulationWriter");
const CREATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.ArgumentHelperCreator");

fn fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::INDEX_EXPRESSION);
    f.create(2, c::FEATURE);
    f.create(3, c::FEATURE_REFERENCE_EXPRESSION);
    f.create(4, c::FEATURE);
    f.create(5, c::EXPRESSION);
    // Excluded return is first: ordering is the first eligible parameter.
    member(&mut f, 1, 4, 12, c::RETURN_PARAMETER_MEMBERSHIP);
    member(&mut f, 1, 2, 11, c::PARAMETER_MEMBERSHIP);
    member(&mut f, 2, 3, 13, c::FEATURE_VALUE);
    f.finish()
}

fn registry(classes: Option<BTreeSet<MetaclassId>>, future: bool) -> ProducerRegistry {
    let mut writer = ProducerDescriptor::new(
        WRITER,
        [ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.relationship_classes = classes;
    writer.scope = if future {
        ProducerEffectScope::SubjectAndOwningType
    } else {
        ProducerEffectScope::Subject
    };
    writer.scoped_fresh_ownership = true;
    let reader = ProducerDescriptor::new(
        ProducerFamily::IndexSelectResult.id(),
        [],
        ProducerApplicability::Subtypes(vec![c::INDEX_EXPRESSION]),
    );
    let mut descriptors = vec![writer, reader];
    if future {
        let mut creator = ProducerDescriptor::new(
            CREATOR,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        creator.scope = ProducerEffectScope::Subject;
        creator.fresh_effects.insert(ProducerEffect::Membership);
        creator.scoped_fresh_ownership = true;
        descriptors.push(creator);
    }
    ProducerRegistry::new(descriptors).unwrap()
}

fn context<'m>(
    snapshot: &'m Snapshot,
    registry: &ProducerRegistry,
    pending: BTreeSet<ElementId>,
) -> SemanticContext<'m> {
    SemanticContext::for_project_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: PROFILE,
            ..Default::default()
        },
        BTreeSet::new(),
        BTreeSet::new(),
        pending,
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap()
}

fn answer(q: &KerMlQueries<'_>) -> QueryResult<Option<ElementId>> {
    let mut answer = q.result(None);
    answer.value = q.argument_expression(&mut answer, id(1));
    answer
}

fn issue<T>(
    q: &KerMlQueries<'_>,
    registry: &ProducerRegistry,
    answer: &QueryResult<T>,
    pending_writer: ElementId,
    future: bool,
) -> ProducerClosureCertificate {
    let mut table = ProducerEvaluationTable::default();
    for record in q.model().elements() {
        table.pending(record.id(), q.model(), registry);
        for descriptor in registry.descriptors() {
            if descriptor
                .applicability
                .applies(q.model(), record.metaclass())
            {
                let pending = if future {
                    record.id() == pending_writer && descriptor.id == CREATOR
                } else {
                    record.id() == pending_writer && descriptor.id == WRITER
                };
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if pending {
                                Completeness::Incomplete
                            } else {
                                Completeness::Complete
                            },
                        )],
                        registry,
                    )
                    .unwrap();
            }
        }
    }
    table
        .record(
            &[(
                id(1),
                ProducerFamily::IndexSelectResult.id(),
                answer.completeness,
            )],
            registry,
        )
        .unwrap();
    table.record_reads(
        &[(
            id(1),
            ProducerFamily::IndexSelectResult.id(),
            producer_reads(answer, q.model()),
        )],
        registry,
    );
    ProducerClosureCertificate::issue(q.model(), q.context(), registry, &table, |_| None)
}

fn state(
    certificate: &ProducerClosureCertificate,
    registry: &ProducerRegistry,
) -> ProducerEvaluationState {
    certificate
        .evaluation(
            id(1),
            registry
                .index(ProducerFamily::IndexSelectResult.id())
                .unwrap(),
        )
        .unwrap()
}

#[test]
fn argument_expression_filters_only_ineligible_direct_and_future_relationships() {
    let snapshot = fixture();
    for future in [false, true] {
        for (owner, classes, expected) in [
            (
                1,
                Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])),
                ProducerEvaluationState::EvaluatedComplete,
            ),
            (
                1,
                Some(BTreeSet::from([c::RETURN_PARAMETER_MEMBERSHIP])),
                ProducerEvaluationState::EvaluatedComplete,
            ),
            (
                1,
                Some(BTreeSet::from([c::PARAMETER_MEMBERSHIP])),
                ProducerEvaluationState::Pending,
            ),
            (
                2,
                Some(BTreeSet::from([c::FEATURE_VALUE])),
                ProducerEvaluationState::Pending,
            ),
            (2, None, ProducerEvaluationState::Pending),
        ] {
            let registry = registry(classes, future);
            let q = KerMlQueries::for_production(context(&snapshot, &registry, BTreeSet::new()));
            let result = answer(&q);
            assert_eq!(result.value, Some(id(3)));
            assert_eq!(result.completeness, Completeness::Complete);
            assert_eq!(
                state(&issue(&q, &registry, &result, id(owner), future), &registry),
                expected,
                "future={future},owner={owner}; {:?}",
                producer_reads(&result, q.model())
            );
        }
    }
}

#[test]
fn argument_expression_keeps_pending_provider_and_explicit_broad_reads() {
    let snapshot = fixture();
    let registry = registry(Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])), false);
    for scope in [id(1), id(2)] {
        let q = KerMlQueries::new(context(&snapshot, &registry, BTreeSet::from([scope])));
        let result = answer(&q);
        assert_eq!(
            state(&issue(&q, &registry, &result, id(1), false), &registry),
            ProducerEvaluationState::Pending
        );
    }
    for production in [false, true] {
        let context = context(&snapshot, &registry, BTreeSet::new());
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        for reverse in [false, true] {
            let result = answer(&q);
            let broad = q.memberships(id(1));
            let combined = if reverse {
                let mut out = broad.map(|_| ());
                out.merge(result);
                out
            } else {
                let mut out = result.map(|_| ());
                out.merge(broad);
                out
            };
            assert_eq!(
                state(&issue(&q, &registry, &combined, id(1), false), &registry),
                ProducerEvaluationState::Pending
            );
        }
    }
}

#[test]
fn argument_expression_selected_endpoint_and_parameter_order_reopen_after_reconstruction() {
    let snapshot = fixture();
    let registry = registry(Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])), false);
    let context = context(&snapshot, &registry, BTreeSet::new());
    let q = KerMlQueries::new(context.fork());
    let certificate = issue(&q, &registry, &answer(&q), id(1), false);
    let checkpoint = certificate.checkpoint(&context).unwrap();
    for property in [
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        p::ELEMENT_OWNED_RELATIONSHIP,
    ] {
        let mut changes = snapshot.change_set();
        if property == p::RELATIONSHIP_OWNED_RELATED_ELEMENT {
            changes.set(
                id(13),
                property,
                SlotValue::Ordered(vec![Value::Reference(id(5))]),
                origin(),
            );
        } else {
            changes.set(
                id(1),
                property,
                SlotValue::Ordered(vec![Value::Reference(id(12))]),
                origin(),
            );
        }
        let changed = snapshot.apply(&changes).unwrap();
        let next = SemanticContext::for_snapshot(
            &changed,
            SemanticOptions {
                baseline_profile: PROFILE,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
        let rebound = checkpoint.rebind(&next, &registry).unwrap();
        assert_eq!(
            state(&rebound.certificate, &registry),
            ProducerEvaluationState::Pending
        );
        assert_eq!(
            answer(&KerMlQueries::new(next)).value,
            if property == p::RELATIONSHIP_OWNED_RELATED_ELEMENT {
                Some(id(5))
            } else {
                None
            }
        );
    }
}
