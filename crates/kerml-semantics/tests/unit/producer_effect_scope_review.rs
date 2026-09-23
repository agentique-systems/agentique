use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.EffectScopeWriter");
const CREATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.EffectScopeCreator");
const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.EffectScopeReader");
const RULE: RuleId = RuleId::from_u128(940001);

fn fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (id, class) in [
        (1, c::FEATURE),
        (2, c::FEATURE_REFERENCE_EXPRESSION),
        (10, c::FEATURE),
        (12, c::FEATURE_REFERENCE_EXPRESSION),
        (20, c::CLASSIFIER),
    ] {
        f.create(id, class);
    }
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    f.finish()
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

fn writer(
    effects: impl IntoIterator<Item = ProducerEffect>,
    scope: ProducerEffectScope,
) -> ProducerDescriptor {
    let mut descriptor = ProducerDescriptor::new(
        WRITER,
        effects,
        ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
    );
    descriptor.scope = scope;
    descriptor.scoped_fresh_ownership = true;
    descriptor
}

#[test]
fn effect_scope_identity_rejects_unknown_effect_and_stale_certificate_transport() {
    let snapshot = fixture();
    let descriptor = writer([ProducerEffect::Typing], ProducerEffectScope::Subject);
    assert_eq!(
        descriptor.effect_scope(ProducerEffect::Typing),
        ProducerEffectScope::Subject
    );
    let old = ProducerRegistry::new([descriptor.clone()]).unwrap();
    let mut widened = descriptor.clone();
    widened
        .effect_scopes
        .insert(ProducerEffect::Typing, ProducerEffectScope::Model);
    let new = ProducerRegistry::new([widened]).unwrap();
    assert_ne!(old.digest(), new.digest());
    let before = context(&snapshot, &old);
    let after = context(&snapshot, &new);
    let certificate = ProducerClosureCertificate::initial(&before, &old).unwrap();
    assert!(
        after
            .fork()
            .with_producer_closure(Arc::new(certificate.clone()))
            .is_err()
    );
    assert!(certificate.rebind(&before, &after, &new).is_err());
    let mut invalid = descriptor;
    invalid
        .effect_scopes
        .insert(ProducerEffect::Featuring, ProducerEffectScope::Subject);
    assert!(
        ProducerRegistry::new([invalid]).is_err(),
        "an override cannot declare an otherwise absent effect"
    );
}

#[test]
fn effect_scope_masks_respect_narrowing_and_widening_independently() {
    let snapshot = fixture();
    for narrow in [false, true] {
        let mut descriptor = writer(
            [ProducerEffect::Typing, ProducerEffect::Membership],
            if narrow {
                ProducerEffectScope::Model
            } else {
                ProducerEffectScope::Subject
            },
        );
        descriptor.effect_scopes.insert(
            ProducerEffect::Typing,
            if narrow {
                ProducerEffectScope::Subject
            } else {
                ProducerEffectScope::Model
            },
        );
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let certificate =
            ProducerClosureCertificate::initial(&context(&snapshot, &registry), &registry).unwrap();
        assert!(!certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
        assert_eq!(
            certificate.is_closed(id(10), SemanticClosureRequirement::EffectiveTyping),
            narrow
        );
        assert!(
            !certificate.is_closed(id(10), SemanticClosureRequirement::EffectiveMembership),
            "non-typing masks retain the pre-existing conservative global block"
        );
    }
}

#[test]
fn future_effect_scopes_do_not_borrow_the_default_or_another_effects_scope() {
    let snapshot = fixture();
    for future_scope in [
        ProducerEffectScope::Subject,
        ProducerEffectScope::SubjectAndOwners,
        ProducerEffectScope::Model,
    ] {
        let mut creator = ProducerDescriptor::new(
            CREATOR,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
        );
        creator.scope = ProducerEffectScope::Subject;
        creator.fresh_effects.insert(ProducerEffect::Membership);
        creator.scoped_fresh_ownership = true;
        // No Behavior exists yet; a pending creator can introduce one. Its
        // effective Typing scope, not the default creation envelope, controls
        // whether it may alter existing ancestor or unrelated typing.
        let mut future = ProducerDescriptor::new(
            WRITER,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::BEHAVIOR]),
        );
        future.scope = ProducerEffectScope::Subject;
        future
            .effect_scopes
            .insert(ProducerEffect::Typing, future_scope);
        future.scoped_fresh_ownership = true;
        let reader = ProducerDescriptor::new(
            READER,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        let registry = ProducerRegistry::new([creator, future, reader]).unwrap();
        let context = context(&snapshot, &registry);
        let q = KerMlQueries::for_production(context.fork());
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            for descriptor in registry.descriptors() {
                if !descriptor
                    .applicability
                    .applies(snapshot.model(), record.metaclass())
                {
                    continue;
                }
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if descriptor.id == CREATOR {
                                Completeness::Incomplete
                            } else {
                                Completeness::Complete
                            },
                        )],
                        &registry,
                    )
                    .unwrap();
                if descriptor.id == READER {
                    let answer = q.owned_relationships_of_type(record.id(), c::FEATURE_TYPING);
                    table.record_reads(
                        &[(
                            record.id(),
                            READER,
                            producer_reads(&answer, snapshot.model()),
                        )],
                        &registry,
                    );
                }
            }
        }
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |_| None,
        );
        for target in [1, 10] {
            let blocked = future_scope == ProducerEffectScope::Model
                || future_scope == ProducerEffectScope::SubjectAndOwners && target == 1;
            assert_eq!(
                certificate.evaluation(id(target), registry.index(READER).unwrap()),
                Some(if blocked {
                    ProducerEvaluationState::Pending
                } else {
                    ProducerEvaluationState::EvaluatedComplete
                }),
                "target={target}, future_scope={future_scope:?}"
            );
            assert_eq!(
                certificate.is_closed(id(target), SemanticClosureRequirement::EffectiveTyping),
                !blocked
            );
        }
    }
}

fn key(subject: u128, output: u128) -> DerivationKey {
    DerivationKey {
        subject: id(subject),
        rule: RULE,
        output: OutputKey::from_u128(output),
    }
}

#[test]
fn reference_scalar_guard_survives_a_narrow_effect_and_protected_dependency() {
    let snapshot = fixture();
    for reference_scalar in [false, true] {
        let mut descriptor = writer(
            [ProducerEffect::Featuring],
            ProducerEffectScope::SubjectAndOwners,
        );
        descriptor
            .effect_scopes
            .insert(ProducerEffect::Featuring, ProducerEffectScope::Subject);
        if reference_scalar {
            descriptor
                .effects
                .insert(ProducerEffect::Scalar(p::FEATURE_FEATURE_TARGET));
            descriptor.effect_scopes.insert(
                ProducerEffect::Scalar(p::FEATURE_FEATURE_TARGET),
                ProducerEffectScope::Subject,
            );
        }
        let reader = ProducerDescriptor::new(
            READER,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        let registry = ProducerRegistry::new([descriptor, reader]).unwrap();
        let context = context(&snapshot, &registry);
        let q = KerMlQueries::for_production(context.fork());
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            for descriptor in registry.descriptors() {
                if !descriptor
                    .applicability
                    .applies(snapshot.model(), record.metaclass())
                {
                    continue;
                }
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if descriptor.id == WRITER && record.id() == id(2) {
                                Completeness::Incomplete
                            } else {
                                Completeness::Complete
                            },
                        )],
                        &registry,
                    )
                    .unwrap();
                if descriptor.id == READER && record.id() == id(1) {
                    // A local query consumer reads the protected dependency;
                    // producers on the dependency itself are inapplicable.
                    let mut read = q.result(());
                    q.incoming_source_relationships(
                        &mut read,
                        id(10),
                        c::FEATURE_TYPING,
                        p::FEATURE_TYPING_TYPED_FEATURE,
                    );
                    assert_eq!(read.completeness, Completeness::Complete);
                    table.record_reads(
                        &[(record.id(), READER, producer_reads(&read, snapshot.model()))],
                        &registry,
                    );
                }
            }
        }
        // Exercise the same protected-population callback used by authenticated
        // dependency mounting; this private unit test does not mint a public
        // publication or bypass the independently tested boundary constructor.
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |subject| (subject == id(10)).then_some(ClosureSource::AcceptedDependency),
        );
        assert_eq!(
            certificate.evaluation(id(1), registry.index(READER).unwrap()),
            Some(if reference_scalar {
                ProducerEvaluationState::Pending
            } else {
                ProducerEvaluationState::EvaluatedComplete
            })
        );
        assert_eq!(
            certificate.is_closed(id(10), SemanticClosureRequirement::EffectiveTyping),
            !reference_scalar,
            "external reference writes still protect against incoming facts on accepted subjects"
        );
    }
}

#[test]
fn effect_scope_audit_uses_specific_semantic_scalar_and_ownership_permissions() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: PROFILE,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    for target in [1, 2] {
        let mut descriptor = writer(
            [ProducerEffect::Featuring, ProducerEffect::Membership],
            ProducerEffectScope::SubjectAndOwners,
        );
        descriptor
            .effect_scopes
            .insert(ProducerEffect::Featuring, ProducerEffectScope::Subject);
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = q.plan_result_structure([]);
        let output = plan
            .add_derived_element(
                key(2, 1),
                c::TYPE_FEATURING,
                BTreeMap::from([
                    (
                        p::TYPE_FEATURING_FEATURE_OF_TYPE,
                        SlotValue::Scalar(Value::Reference(id(target))),
                    ),
                    (
                        p::TYPE_FEATURING_FEATURING_TYPE,
                        SlotValue::Scalar(Value::Reference(id(20))),
                    ),
                ]),
                Some(id(2)),
                &q.canonical_fact_evidence(FactKey::Element(id(2))),
            )
            .unwrap();
        plan.attribute_producer_outputs(id(2), WRITER, output);
        assert_eq!(
            plan.validate_declared_effects(&[id(2)], &registry).is_ok(),
            target == 2
        );

        let effect = ProducerEffect::Scalar(p::FEATURE_IS_END);
        let mut descriptor = writer([effect], ProducerEffectScope::SubjectAndOwners);
        descriptor
            .effect_scopes
            .insert(effect, ProducerEffectScope::Subject);
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = q.plan_result_structure([]);
        plan.add_derived_property(
            id(target),
            p::FEATURE_IS_END,
            SlotValue::Scalar(Value::Boolean(true)),
            RULE,
            &q.canonical_fact_evidence(FactKey::Element(id(2))),
        )
        .unwrap();
        assert_eq!(
            plan.validate_declared_effects(&[id(2)], &registry).is_ok(),
            target == 2,
            "scalar override target={target}"
        );
    }
    for own_subject in [false, true] {
        let mut descriptor = writer(
            [ProducerEffect::Ownership, ProducerEffect::Membership],
            ProducerEffectScope::Model,
        );
        descriptor
            .effect_scopes
            .insert(ProducerEffect::Ownership, ProducerEffectScope::Subject);
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = q.plan_result_structure([]);
        let output = plan
            .add_derived_element(
                key(12, 2),
                c::FEATURE_MEMBERSHIP,
                BTreeMap::from([(
                    p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                    SlotValue::Ordered(vec![Value::Reference(id(if own_subject {
                        12
                    } else {
                        10
                    }))]),
                )]),
                Some(id(1)),
                &q.canonical_fact_evidence(FactKey::Element(id(12))),
            )
            .unwrap();
        plan.attribute_producer_outputs(id(12), WRITER, output);
        assert_eq!(
            plan.validate_declared_effects(&[id(12)], &registry).is_ok(),
            own_subject,
            "existing child ownership must use Ownership override"
        );
    }
}
