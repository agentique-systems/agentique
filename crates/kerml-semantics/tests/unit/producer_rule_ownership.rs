//! An output cannot borrow a different producer family's effect permissions.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn subjects() -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::FEATURE);
    f.create(2, c::FEATURE);
    f.create(3, c::CLASSIFIER);
    f.finish()
}

fn audit_subsetting(
    snapshot: &Snapshot,
    rule: RuleId,
    registry: &ProducerRegistry,
) -> Result<(), PublicationOverlayError> {
    audit_subsetting_for_subjects(snapshot, rule, registry, &[id(1)])
}

fn audit_subsetting_for_subjects(
    snapshot: &Snapshot,
    rule: RuleId,
    registry: &ProducerRegistry,
    scheduled: &[ElementId],
) -> Result<(), PublicationOverlayError> {
    let queries = KerMlQueries::new(
        SemanticContext::for_snapshot(
            snapshot,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    let mut plan = queries.plan_result_structure([]);
    let evidence = queries.canonical_fact_evidence(FactKey::Element(id(1)));
    plan.add_derived_element(
        DerivationKey {
            rule,
            subject: id(1),
            output: OutputKey::from_u128(0xfeed01),
        },
        c::SUBSETTING,
        BTreeMap::from([
            (
                p::SUBSETTING_SUBSETTING_FEATURE,
                SlotValue::Scalar(Value::Reference(id(1))),
            ),
            (
                p::SUBSETTING_SUBSETTED_FEATURE,
                SlotValue::Scalar(Value::Reference(id(2))),
            ),
        ]),
        Some(id(1)),
        &evidence,
    )
    .unwrap();
    plan.validate_declared_effects(scheduled, registry)
}

#[test]
fn binding_rule_cannot_borrow_registered_valuation_subsetting_permission() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    let valuation = ProducerFamily::FeatureValuation.descriptor(profile);
    let binding = ProducerFamily::FeatureValue.descriptor(profile);
    let valuation_rule = *valuation
        .derivation_rules
        .as_ref()
        .unwrap()
        .first()
        .unwrap();
    let registry = ProducerRegistry::new([valuation, binding]).unwrap();
    let snapshot = subjects();
    assert!(audit_subsetting(&snapshot, valuation_rule, &registry).is_ok());
    assert!(matches!(
        audit_subsetting(
            &snapshot,
            ImpliedBindingRole::FeatureValue.rule_id(profile),
            &registry,
        ),
        Err(PublicationOverlayError::ProducerEffectViolation(_))
    ));
}

#[test]
fn inapplicable_rule_owner_cannot_fall_back_to_unclaimed_effect_permissions() {
    let rule = RuleId::from_u128(0xfeed02);
    let snapshot = subjects();
    for applicability in [
        ProducerApplicability::Never,
        ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
    ] {
        let mut owner = ProducerDescriptor::new(
            ProducerFamilyId::new("Fixture.RuleOwner"),
            [ProducerEffect::Subsetting],
            applicability,
        );
        owner.derivation_rules = Some(BTreeSet::from([rule]));
        let mut fallback = ProducerDescriptor::new(
            ProducerFamilyId::new("Fixture.UnclaimedFallback"),
            [ProducerEffect::Subsetting],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        fallback.scope = ProducerEffectScope::Model;
        let fallback_only = ProducerRegistry::new([fallback.clone()]).unwrap();
        assert!(audit_subsetting(&snapshot, rule, &fallback_only).is_ok());
        let registry = ProducerRegistry::new([owner, fallback]).unwrap();
        assert!(matches!(
            audit_subsetting(&snapshot, rule, &registry),
            Err(PublicationOverlayError::ProducerEffectViolation(_))
        ));
    }
}

#[test]
fn rule_claim_checks_key_subject_even_when_another_scheduled_subject_applies() {
    let rule = RuleId::from_u128(0xfeed06);
    let mut owner = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.OtherSubject"),
        [ProducerEffect::Subsetting],
        ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
    );
    owner.scope = ProducerEffectScope::Model;
    let snapshot = subjects();
    let ordinary = ProducerRegistry::new([owner.clone()]).unwrap();
    assert!(audit_subsetting_for_subjects(&snapshot, rule, &ordinary, &[id(1), id(3)]).is_ok());
    owner.derivation_rules = Some(BTreeSet::from([rule]));
    let claimed = ProducerRegistry::new([owner]).unwrap();
    assert!(matches!(
        audit_subsetting_for_subjects(&snapshot, rule, &claimed, &[id(1), id(3)]),
        Err(PublicationOverlayError::ProducerEffectViolation(_))
    ));
}

#[test]
fn duplicate_derivation_rule_owners_are_rejected_independently_of_input_order() {
    let rule = RuleId::from_u128(0xfeed03);
    let mut first = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.A"),
        [ProducerEffect::Subsetting],
        ProducerApplicability::Any,
    );
    first.derivation_rules = Some(BTreeSet::from([rule]));
    let mut second = first.clone();
    second.id = ProducerFamilyId::new("Fixture.B");
    for descriptors in [
        [first.clone(), second.clone()],
        [second.clone(), first.clone()],
    ] {
        assert_eq!(ProducerRegistry::new(descriptors).err(), Some(second.id));
    }
}

#[test]
fn rule_ownership_changes_registry_identity_without_changing_effects() {
    let mut descriptor = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.Identity"),
        [ProducerEffect::Subsetting],
        ProducerApplicability::Any,
    );
    let ordinary = ProducerRegistry::new([descriptor.clone()]).unwrap();
    descriptor.derivation_rules = Some(BTreeSet::from([RuleId::from_u128(0xfeed04)]));
    let owned = ProducerRegistry::new([descriptor.clone()]).unwrap();
    assert_ne!(ordinary.digest(), owned.digest());
    descriptor.derivation_rules = Some(BTreeSet::from([RuleId::from_u128(0xfeed05)]));
    let changed = ProducerRegistry::new([descriptor]).unwrap();
    assert_ne!(owned.digest(), changed.digest());
}
