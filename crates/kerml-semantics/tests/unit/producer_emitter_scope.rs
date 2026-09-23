use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

const FAMILY: ProducerFamilyId = ProducerFamilyId::new("Fixture.ExactEmitter");
const RULE: RuleId = RuleId::from_u128(919100);

fn fixture() -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::FEATURE);
    f.create(3, c::CLASSIFIER);
    member(&mut f, 3, 1, 31, c::FEATURE_MEMBERSHIP);
    member(&mut f, 3, 2, 32, c::FEATURE_MEMBERSHIP);
    f.finish()
}

fn registry(scope: ProducerEffectScope) -> ProducerRegistry {
    let mut descriptor = ProducerDescriptor::new(
        FAMILY,
        [ProducerEffect::Typing, ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    descriptor.scope = scope;
    descriptor.scoped_fresh_ownership = true;
    ProducerRegistry::new([descriptor]).unwrap()
}

fn key(subject: u128, output: u128) -> DerivationKey {
    DerivationKey {
        subject: id(subject),
        rule: RULE,
        output: OutputKey::from_u128(output),
    }
}

fn typing_plan<'m>(
    q: &KerMlQueries<'m>,
    key_subject: u128,
    emitter: u128,
    typed: u128,
    owner: u128,
) -> ResultStructurePlan<'m> {
    let mut plan = q.plan_result_structure([]);
    let output = plan
        .add_derived_element(
            key(key_subject, 1),
            c::FEATURE_TYPING,
            BTreeMap::from([
                (
                    p::FEATURE_TYPING_TYPED_FEATURE,
                    SlotValue::Scalar(Value::Reference(id(typed))),
                ),
                (
                    p::FEATURE_TYPING_TYPE,
                    SlotValue::Scalar(Value::Reference(id(3))),
                ),
            ]),
            Some(id(owner)),
            &q.canonical_fact_evidence(FactKey::Element(id(key_subject))),
        )
        .unwrap();
    plan.attribute_producer_outputs(id(emitter), FAMILY, output);
    plan
}

#[test]
fn emitter_cannot_borrow_another_batch_subjects_existing_typing_scope() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let registry = registry(ProducerEffectScope::Subject);
    // A's fresh relationship is legitimately owned by A, but its semantic
    // typedFeature source is B. Supplying B in the same batch must not grant A
    // B's Subject-scoped effect permission.
    let plan = typing_plan(&q, 1, 1, 2, 1);
    let error = plan
        .validate_declared_effects(&[id(1), id(2)], &registry)
        .unwrap_err();
    assert!(
        matches!(error, PublicationOverlayError::ProducerEffectViolation(failure)
        if failure.operation == "relationship semantic effect" && failure.semantic_target == id(2))
    );
    assert!(
        typing_plan(&q, 1, 1, 1, 1)
            .validate_declared_effects(&[id(1), id(2)], &registry)
            .is_ok()
    );
}

#[test]
fn emitter_identity_is_independent_of_the_stable_derivation_key_subject() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    // The key names Classifier3, which is not a Feature and cannot be an
    // applicable subject of FAMILY. The actual emitter and target are Feature1.
    // Real result/chain/shared-domain producers likewise use semantic keys
    // distinct from the producer evaluation subject.
    let plan = typing_plan(&q, 3, 1, 1, 1);
    plan.validate_declared_effects(&[id(1)], &registry(ProducerEffectScope::Subject))
        .unwrap();
    let output = plan.materialize(&snapshot).unwrap();
    assert!(
        output
            .overlay
            .model()
            .element(key(3, 1).element_id())
            .is_some()
    );
}

#[test]
fn every_shared_output_emitter_survives_both_plan_merge_orders() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let registry = registry(ProducerEffectScope::Subject);
    for reverse in [false, true] {
        let mut valid = typing_plan(&q, 1, 1, 1, 1);
        let mut invalid = typing_plan(&q, 1, 2, 1, 1);
        let plan = if reverse {
            invalid.merge(valid).unwrap();
            invalid
        } else {
            valid.merge(invalid).unwrap();
            valid
        };
        assert!(
            plan.validate_declared_effects(&[id(1), id(2)], &registry)
                .is_err(),
            "an invalid second claimant was lost when reverse={reverse}"
        );
    }
}

fn shared_owner_plan<'m>(q: &KerMlQueries<'m>, emitter: u128) -> ResultStructurePlan<'m> {
    let mut plan = q.plan_result_structure([]);
    let evidence = q.canonical_fact_evidence(FactKey::Element(id(3)));
    let helper = plan
        .add_derived_element(key(3, 2), c::FEATURE, BTreeMap::new(), None, &evidence)
        .unwrap()
        .unwrap();
    let membership = plan
        .add_derived_element(
            key(3, 3),
            c::FEATURE_MEMBERSHIP,
            BTreeMap::from([(
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(helper)]),
            )]),
            Some(id(3)),
            &evidence,
        )
        .unwrap()
        .unwrap();
    plan.attribute_producer_outputs(id(emitter), FAMILY, [helper, membership]);
    plan
}

#[test]
fn legitimate_shared_owner_outputs_and_fresh_local_targets_keep_valid_attribution() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let owner_registry = registry(ProducerEffectScope::SubjectAndOwners);
    for reverse in [false, true] {
        let mut plan = shared_owner_plan(&q, if reverse { 2 } else { 1 });
        plan.merge(shared_owner_plan(&q, if reverse { 1 } else { 2 }))
            .unwrap();
        plan.validate_declared_effects(&[id(1), id(2)], &owner_registry)
            .unwrap();
        let output = plan.materialize(&snapshot).unwrap();
        let q = KerMlQueries::new(
            SemanticContext::for_overlay(&output.overlay, Default::default(), BTreeSet::new())
                .unwrap(),
        );
        assert_eq!(q.owning_type(key(3, 2).element_id()).value, Some(id(3)));
    }
    let mut plan = q.plan_result_structure([]);
    let evidence = q.canonical_fact_evidence(FactKey::Element(id(1)));
    let helper = plan
        .add_derived_element(key(1, 4), c::FEATURE, BTreeMap::new(), None, &evidence)
        .unwrap()
        .unwrap();
    let membership = plan
        .add_derived_element(
            key(1, 5),
            c::FEATURE_MEMBERSHIP,
            BTreeMap::from([(
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(helper)]),
            )]),
            Some(id(1)),
            &evidence,
        )
        .unwrap()
        .unwrap();
    let typing = plan
        .add_derived_element(
            key(1, 6),
            c::FEATURE_TYPING,
            BTreeMap::from([
                (
                    p::FEATURE_TYPING_TYPED_FEATURE,
                    SlotValue::Scalar(Value::Reference(helper)),
                ),
                (
                    p::FEATURE_TYPING_TYPE,
                    SlotValue::Scalar(Value::Reference(id(3))),
                ),
            ]),
            Some(helper),
            &evidence,
        )
        .unwrap()
        .unwrap();
    plan.attribute_producer_outputs(id(1), FAMILY, [helper, membership, typing]);
    plan.validate_declared_effects(&[id(1)], &registry(ProducerEffectScope::Subject))
        .unwrap();
    plan.materialize(&snapshot).unwrap();
}

#[test]
fn actual_shared_initial_value_chain_has_each_binding_emitter_after_merge() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V7;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::NAMESPACE);
    f.member(1, 102, 2, c::PACKAGE, "Base");
    f.member(2, 103, 3, c::FEATURE, "things");
    f.member(3, 104, 4, c::FEATURE, "that");
    f.member(1, 105, 5, c::PACKAGE, "Occurrences");
    f.member(5, 106, 6, c::CLASS, "Occurrence");
    f.member(6, 107, 7, c::FEATURE, "startShot");
    for (feature, expression, result, value) in [(10, 11, 12, 110), (20, 21, 22, 120)] {
        f.member(6, feature + 200, feature, c::FEATURE, "initial");
        f.create(expression, c::EXPRESSION);
        member(&mut f, feature, expression, value, c::FEATURE_VALUE);
        f.value(value, p::FEATURE_VALUE_IS_INITIAL, Value::Boolean(true));
        f.create(result, c::FEATURE);
        f.enumeration(result, p::FEATURE_DIRECTION, "out");
        member(
            &mut f,
            expression,
            result,
            value + 1,
            c::RETURN_PARAMETER_MEMBERSHIP,
        );
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(profile)),
    )
    .unwrap();
    for reverse in [false, true] {
        let mut plan = q.plan_result_structure([id(if reverse { 20 } else { 10 })]);
        plan.merge(q.plan_result_structure([id(if reverse { 10 } else { 20 })]))
            .unwrap();
        assert_eq!(plan.production.completeness, Completeness::Complete);
        plan.validate_declared_effects(&[id(10), id(20)], &registry)
            .unwrap();
        let result = plan.materialize(&snapshot).unwrap();
        let q = KerMlQueries::new(
            SemanticContext::for_overlay(
                &result.overlay,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap(),
        );
        let domains: Vec<_> = result
            .production
            .value
            .iter()
            .map(|binding| q.featuring_types(*binding).value)
            .collect();
        assert_eq!(domains.len(), 2);
        assert_eq!(domains[0], domains[1]);
        assert_eq!(q.chaining_features(domains[0][0]).value, [id(4), id(7)]);
    }
}
