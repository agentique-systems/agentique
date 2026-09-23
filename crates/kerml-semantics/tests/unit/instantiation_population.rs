use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.InstantiationMembershipWriter");

fn fixture(target: bool) -> Snapshot {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::INVOCATION_EXPRESSION);
    f.create(2, c::FUNCTION);
    f.create(6, c::FUNCTION);
    f.create(4, c::FEATURE);
    f.enumeration(4, p::FEATURE_DIRECTION, "out");
    member(&mut f, 1, 4, 5, c::RETURN_PARAMETER_MEMBERSHIP);
    if target {
        f.create(3, c::MEMBERSHIP);
        f.own(1, 3);
        f.value(3, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(2)));
    }
    f.finish()
}

fn registry(classes: Option<BTreeSet<MetaclassId>>) -> ProducerRegistry {
    let mut writer = ProducerDescriptor::new(
        WRITER,
        [ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::INVOCATION_EXPRESSION]),
    );
    writer.scope = ProducerEffectScope::Subject;
    writer.relationship_classes = classes;
    ProducerRegistry::new([
        writer,
        ProducerFamily::Invocation.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9),
    ])
    .unwrap()
}

fn context<'m>(snapshot: &'m Snapshot, registry: &ProducerRegistry) -> SemanticContext<'m> {
    SemanticContext::for_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
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
) -> ProducerClosureCertificate {
    let mut table = ProducerEvaluationTable::default();
    for record in q.model().elements() {
        table.pending(record.id(), q.model(), registry);
    }
    table
        .record(
            &[(id(1), ProducerFamily::Invocation.id(), answer.completeness)],
            registry,
        )
        .unwrap();
    table.record_reads(
        &[(
            id(1),
            ProducerFamily::Invocation.id(),
            producer_reads(answer, q.model()),
        )],
        registry,
    );
    ProducerClosureCertificate::issue(q.model(), q.context(), registry, &table, |_| None)
}

#[test]
fn instantiated_type_does_not_depend_on_excluded_feature_membership_writers() {
    let snapshot = fixture(true);
    for production in [false, true] {
        let registry = registry(Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])));
        let context = context(&snapshot, &registry);
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        let answer = q.instantiated_type(id(1));
        assert_eq!(answer.value, Some(id(2)));
        assert_eq!(answer.completeness, Completeness::Complete);
        let certificate = issue(&q, &registry, &answer);
        assert_eq!(
            certificate.evaluation(
                id(1),
                registry.index(ProducerFamily::Invocation.id()).unwrap()
            ),
            Some(ProducerEvaluationState::EvaluatedComplete),
            "production={production}; reads={:?}",
            producer_reads(&answer, q.model())
        );
    }
}

#[test]
fn instantiated_type_retains_qualifying_unknown_and_explicit_broad_membership_writes() {
    let snapshot = fixture(true);
    for classes in [
        None,
        Some(BTreeSet::from([c::MEMBERSHIP])),
        Some(BTreeSet::from([c::OWNING_MEMBERSHIP])),
    ] {
        let registry = registry(classes);
        let q = KerMlQueries::new(context(&snapshot, &registry));
        let answer = q.instantiated_type(id(1));
        let certificate = issue(&q, &registry, &answer);
        assert_eq!(
            certificate.evaluation(
                id(1),
                registry.index(ProducerFamily::Invocation.id()).unwrap()
            ),
            Some(ProducerEvaluationState::Pending)
        );
    }
    let registry = registry(Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])));
    let q = KerMlQueries::new(context(&snapshot, &registry));
    for reverse in [false, true] {
        let answer = q.instantiated_type(id(1));
        let broad = q.memberships(id(1));
        let combined = if reverse {
            let mut out = broad.map(|_| ());
            out.merge(answer);
            out
        } else {
            let mut out = answer.map(|_| ());
            out.merge(broad);
            out
        };
        let certificate = issue(&q, &registry, &combined);
        assert_eq!(
            certificate.evaluation(
                id(1),
                registry.index(ProducerFamily::Invocation.id()).unwrap()
            ),
            Some(ProducerEvaluationState::Pending)
        );
    }
}

#[test]
fn instantiated_type_selected_target_reconstruction_reopens_the_reader() {
    let snapshot = fixture(true);
    let registry = registry(Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])));
    let context = context(&snapshot, &registry);
    let q = KerMlQueries::new(context.fork());
    let answer = q.instantiated_type(id(1));
    let certificate = issue(&q, &registry, &answer);
    let checkpoint = certificate.checkpoint(&context).unwrap();
    let mut changes = snapshot.change_set();
    changes.set(
        id(3),
        p::MEMBERSHIP_MEMBER_ELEMENT,
        SlotValue::Scalar(Value::Reference(id(6))),
        origin(),
    );
    let changed = snapshot.apply(&changes).unwrap();
    assert_eq!(
        snapshot.model().element(id(1)),
        changed.model().element(id(1))
    );
    let next = SemanticContext::for_snapshot(
        &changed,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let rebound = checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(
        rebound.certificate.evaluation(
            id(1),
            registry.index(ProducerFamily::Invocation.id()).unwrap()
        ),
        Some(ProducerEvaluationState::Pending)
    );
    assert_eq!(
        KerMlQueries::new(next).instantiated_type(id(1)).value,
        Some(id(6))
    );
}

#[test]
fn instantiated_type_accepts_derived_eligible_memberships_in_canonical_order() {
    let snapshot = fixture(false);
    let registry = registry(Some(BTreeSet::from([c::FEATURE_MEMBERSHIP])));
    let q = KerMlQueries::new(context(&snapshot, &registry));
    let mut plan = q.plan_result_structure([]);
    let evidence = q.canonical_fact_evidence(FactKey::Element(id(2)));
    for (output, target) in [(90001, 2), (90002, 6)] {
        plan.add_derived_element(
            DerivationKey {
                rule: RuleId::from_u128(90000),
                subject: id(1),
                output: OutputKey::from_u128(output),
            },
            c::MEMBERSHIP,
            BTreeMap::from([(
                p::MEMBERSHIP_MEMBER_ELEMENT,
                SlotValue::Scalar(Value::Reference(id(target))),
            )]),
            Some(id(1)),
            &evidence,
        )
        .unwrap();
    }
    let result = plan.materialize(&snapshot).unwrap();
    let next = SemanticContext::for_overlay(
        &result.overlay,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap();
    let q = KerMlQueries::new(next);
    let eligible = q.owned_relationships_excluding(id(1), c::MEMBERSHIP, [c::FEATURE_MEMBERSHIP]);
    assert_eq!(eligible.value.len(), 2);
    let first = q.member(eligible.value[0]);
    let answer = q.instantiated_type(id(1));
    assert_eq!(answer.completeness, Completeness::Complete);
    assert_eq!(answer.value, first.value);
    assert!(matches!(answer.value, Some(value) if value == id(2) || value == id(6)));
}
