use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn fresh_subject_attachment_contract_is_audited_through_intermediate_carriers() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let family = ProducerFamilyId::new("Fixture.FreshAttachment");
    let rule = RuleId::from_u128(99901);
    for (scoped, owner) in [
        (true, Some(id(1))),
        (true, Some(id(2))),
        (true, None),
        (false, Some(id(2))),
    ] {
        let mut descriptor = crate::ProducerDescriptor::new(
            family,
            [ProducerEffect::ResultStructure],
            crate::ProducerApplicability::Any,
        );
        descriptor.scope = ProducerEffectScope::Subject;
        descriptor.scoped_fresh_ownership = scoped;
        descriptor.derivation_rules = Some(BTreeSet::from([rule]));
        let registry = crate::ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = q.plan_result_structure([]);
        let key = DerivationKey {
            rule,
            subject: id(1),
            output: OutputKey::from_u128(1),
        };
        let feature = plan.graph.create(key, c::FEATURE, &BTreeSet::new());
        let carrier = plan.graph.create(
            DerivationKey {
                output: OutputKey::from_u128(2),
                ..key
            },
            c::RELATIONSHIP,
            &BTreeSet::new(),
        );
        plan.graph.set_value(
            carrier,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(feature)]),
        );
        if let Some(owner) = owner {
            plan.graph.own(owner, carrier);
        }
        plan.attribute_producer_outputs(id(1), family, [feature, carrier]);
        let result = plan.validate_declared_effects(&[id(1), id(2)], &registry);
        if scoped && owner == Some(id(2)) {
            let Err(PublicationOverlayError::ProducerEffectViolation(failure)) = result else {
                panic!("outside-scope fresh attachment was accepted");
            };
            assert_eq!(failure.operation, "fresh subject ownership");
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }
}

#[test]
fn scalar_only_contract_cannot_create_detached_subjects() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let family = ProducerFamilyId::new("Fixture.ScalarOnly");
    for fresh_capability in [false, true] {
        let mut descriptor = ProducerDescriptor::new(
            family,
            [ProducerEffect::Scalar(p::FEATURE_IS_VARIABLE)],
            ProducerApplicability::Any,
        );
        descriptor.scoped_fresh_ownership = true;
        if fresh_capability {
            descriptor
                .fresh_effects
                .insert(ProducerEffect::ResultStructure);
        }
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = q.plan_result_structure([]);
        let key = DerivationKey {
            rule: RuleId::from_u128(99902),
            subject: id(1),
            output: OutputKey::from_u128(1),
        };
        let feature = plan.graph.create(key, c::FEATURE, &BTreeSet::new());
        plan.attribute_producer_outputs(id(1), family, [feature]);
        let result = plan.validate_declared_effects(&[id(1)], &registry);
        if fresh_capability {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let Err(PublicationOverlayError::ProducerEffectViolation(failure)) = result else {
                panic!("scalar-only creation accepted");
            };
            assert_eq!(failure.operation, "fresh subject creation");
        }
    }
}

#[test]
fn existing_detached_carrier_adoption_requires_ownership_capability() {
    for already_owned in [false, true] {
        let mut f = Fixture::new();
        f.create(1, c::FEATURE);
        f.create(2, c::CLASSIFIER);
        f.create(3, c::RELATIONSHIP);
        f.create(4, c::FEATURE);
        f.changes.set(
            id(3),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(4))]),
            origin(),
        );
        if already_owned {
            f.own(2, 3);
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
        );
        let family = ProducerFamilyId::new("Fixture.Adopter");
        for ownership in [false, true] {
            let mut descriptor = ProducerDescriptor::new(
                family,
                [ProducerEffect::ResultStructure],
                ProducerApplicability::Any,
            );
            descriptor.scope = ProducerEffectScope::Model;
            descriptor.scoped_fresh_ownership = true;
            if ownership {
                descriptor.effects.insert(ProducerEffect::Ownership);
            }
            let registry = ProducerRegistry::new([descriptor]).unwrap();
            let mut plan = q.plan_result_structure([]);
            plan.graph.own(id(2), id(3));
            plan.attribute_producer_outputs(id(1), family, [id(3)]);
            let result = plan.validate_declared_effects(&[id(1)], &registry);
            if already_owned || ownership {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let Err(PublicationOverlayError::ProducerEffectViolation(failure)) = result else {
                    panic!("unclaimed adoption accepted");
                };
                assert_eq!(failure.operation, "existing relationship adoption");
            }
        }
    }
}
