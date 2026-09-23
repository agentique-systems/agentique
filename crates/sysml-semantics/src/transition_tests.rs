use super::*;
use agq_kerml_semantics::{EffectiveNames, MemberAccess};
use agq_sysml::properties as sp;

fn enumeration(f: &mut Fixture, n: u128, property: PropertyId, value: &str) {
    let ValueKind::Enumeration(domain) = f
        .base
        .model()
        .registry()
        .property(property)
        .unwrap()
        .value_kind
    else {
        panic!("enumeration property")
    };
    let literal = *f
        .base
        .model()
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, name)| name.as_str() == value)
        .unwrap()
        .0;
    f.value(n, property, Value::Enumeration(literal));
}

fn fixture(trigger: bool, payload: bool) -> Fixture {
    let mut f = Fixture::new();
    f.create(100, sc::TRANSITION_USAGE, "transition");
    for (n, membership) in [(101, 201), (102, 202)] {
        f.create(n, sc::REFERENCE_USAGE, "");
        f.changes.clear(id(n), kp::ELEMENT_DECLARED_NAME);
        enumeration(&mut f, n, kp::FEATURE_DIRECTION, "in");
        f.member(100, n, membership, kc::PARAMETER_MEMBERSHIP);
        f.changes.clear(id(membership), kp::ELEMENT_DECLARED_NAME);
    }
    if trigger {
        f.create(110, sc::ACCEPT_ACTION_USAGE, "trigger");
        f.member(100, 110, 210, sc::TRANSITION_FEATURE_MEMBERSHIP);
        enumeration(
            &mut f,
            210,
            sp::TRANSITION_FEATURE_MEMBERSHIP_KIND,
            "trigger",
        );
        f.changes.clear(id(210), kp::ELEMENT_DECLARED_NAME);
        if payload {
            f.create(111, sc::REFERENCE_USAGE, "messageContent");
            enumeration(&mut f, 111, kp::FEATURE_DIRECTION, "in");
            f.member(110, 111, 211, kc::PARAMETER_MEMBERSHIP);
            f.changes.clear(id(211), kp::ELEMENT_DECLARED_NAME);
        }
    }
    f
}

#[test]
fn second_transition_input_uses_trigger_payload_canonical_name_in_shared_lookup() {
    let snapshot = fixture(true, true).finish();
    let queries = q(&snapshot);
    let names = queries.kerml().effective_names(id(102));
    assert_eq!(names.completeness, Completeness::Complete, "{names:?}");
    assert_eq!(
        names.value,
        EffectiveNames::Determinate(BTreeSet::from(["messageContent".into()]))
    );
    let lookup = queries
        .kerml()
        .lookup_member(id(100), "messageContent", MemberAccess::All);
    assert_eq!(lookup.completeness, Completeness::Complete, "{lookup:?}");
    assert_eq!(
        lookup.value.iter().map(|m| m.element).collect::<Vec<_>>(),
        [id(102)]
    );
    let first = current_sysml_naming_source(queries.kerml(), id(101));
    assert_eq!(first.value, None);
    assert!(!first.search_dependencies.is_empty());
    assert!(
        snapshot
            .model()
            .navigation_slot(id(102), kp::ELEMENT_DECLARED_NAME)
            .is_none()
    );
}

#[test]
fn transition_naming_preserves_explicit_names_and_distinguishes_null_from_pending() {
    let mut f = fixture(true, true);
    f.value(
        102,
        kp::ELEMENT_DECLARED_NAME,
        Value::String("localName".into()),
    );
    let snapshot = f.finish();
    assert_eq!(
        q(&snapshot).kerml().effective_names(id(102)).value,
        EffectiveNames::Determinate(BTreeSet::from(["localName".into()]))
    );
    let snapshot = fixture(false, false).finish();
    let queries = q(&snapshot);
    let missing_trigger = current_sysml_naming_source(queries.kerml(), id(102));
    assert_eq!(missing_trigger.completeness, Completeness::Complete);
    assert_eq!(missing_trigger.value, Some(None));
    let snapshot = fixture(true, false).finish();
    let queries = q(&snapshot);
    let pending = current_sysml_naming_source(queries.kerml(), id(102));
    assert_eq!(pending.completeness, Completeness::Incomplete);
    assert_eq!(pending.value, Some(None));
    let plan = crate::transition::plan_transition_payload(
        queries.kerml(),
        SysmlBaselineProfile::OperationalV1,
        id(100),
    );
    assert_eq!(plan.evidence.completeness, Completeness::Incomplete);
    assert!(plan.elements.is_empty());
}

#[test]
fn transition_input_parameters_exclude_inherited_parameter_identities() {
    let mut f = fixture(true, true);
    f.create(300, sc::TRANSITION_USAGE, "specializedTransition");
    f.relation(
        300,
        100,
        400,
        kc::SUBSETTING,
        kp::SUBSETTING_SUBSETTED_FEATURE,
    );
    f.create(310, sc::ACCEPT_ACTION_USAGE, "ownTrigger");
    f.member(300, 310, 410, sc::TRANSITION_FEATURE_MEMBERSHIP);
    enumeration(
        &mut f,
        410,
        sp::TRANSITION_FEATURE_MEMBERSHIP_KIND,
        "trigger",
    );
    f.create(311, sc::REFERENCE_USAGE, "ownPayload");
    enumeration(&mut f, 311, kp::FEATURE_DIRECTION, "in");
    f.member(310, 311, 411, kc::PARAMETER_MEMBERSHIP);
    let snapshot = f.finish();
    let queries = q(&snapshot);
    assert_eq!(
        queries.kerml().structural_parameter_features(id(300)).value,
        [id(101), id(102)]
    );
    assert!(
        queries
            .kerml()
            .owned_parameter_features(id(300))
            .value
            .is_empty()
    );
    let result = crate::transition::plan_transition_payload(
        queries.kerml(),
        SysmlBaselineProfile::OPERATIONAL_V2,
        id(300),
    );
    assert_eq!(result.evidence.completeness, Completeness::Incomplete);
    assert!(
        result.elements.is_empty(),
        "inherited inputs remain unchanged"
    );
    assert!(
        result
            .evidence
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "SQ_TRANSITION_PAYLOAD_PENDING" })
    );
}

#[test]
fn pending_extra_parameter_ancestors_do_not_suppress_known_payload_structure() {
    let snapshot = fixture(true, true).finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::from([id(102)]));
    let queries = KerMlQueries::new(context.kerml);
    assert_eq!(
        queries.all_supertypes(id(102)).completeness,
        Completeness::Incomplete
    );
    let result = crate::transition::plan_transition_payload(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V2,
        id(100),
    );
    assert_eq!(result.evidence.completeness, Completeness::Complete);
    assert_eq!(result.elements.len(), 4);
}

#[test]
fn pending_transition_payload_does_not_retype_trigger_or_its_parameters() {
    use agq_kerml_semantics::{
        ProducerClosureCertificate, ProducerFamilyId, ProducerRegistry, SemanticClosureRequirement,
    };
    let snapshot = fixture(true, true).finish();
    let queries = q(&snapshot);
    let plan = crate::transition::plan_transition_payload(
        queries.kerml(),
        SysmlBaselineProfile::OPERATIONAL_V2,
        id(100),
    );
    assert_eq!(plan.evidence.completeness, Completeness::Complete);
    let existing_targets: Vec<_> = plan
        .elements
        .iter()
        .filter_map(|element| element.owner)
        .collect();
    assert_eq!(
        existing_targets,
        [id(102)],
        "only the second direct input is modified"
    );
    let descriptor = sysml_producer_descriptors()
        .into_iter()
        .find(|descriptor| {
            descriptor.id == ProducerFamilyId::new("checkTransitionUsagePayloadSpecialization")
        })
        .unwrap();
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    let typing = SemanticClosureRequirement::EffectiveTyping;
    assert!(
        !certificate.is_closed(id(102), typing),
        "the actual target remains open"
    );
    assert!(
        certificate.is_closed(id(110), typing),
        "the trigger is not a transition input"
    );
    assert!(
        certificate.is_closed(id(111), typing),
        "nested inputs belong to the trigger"
    );
}

#[test]
fn transition_payload_specialization_materializes_ordered_chain_and_is_idempotent() {
    let mut fixture = fixture(true, true);
    fixture.create(9000, kc::FEATURE, "unrelatedAncestor");
    fixture.create(9001, kc::FEATURE, "unrelatedPremise");
    let snapshot = fixture.finish();
    let queries = q(&snapshot);
    let result = crate::transition::plan_transition_payload(
        queries.kerml(),
        SysmlBaselineProfile::OperationalV1,
        id(100),
    );
    assert_eq!(
        result.evidence.completeness,
        Completeness::Complete,
        "{:?}",
        result.evidence
    );
    assert_eq!(result.elements.len(), 4);
    assert_eq!(
        result.elements,
        crate::transition::plan_transition_payload(
            queries.kerml(),
            SysmlBaselineProfile::OperationalV1,
            id(100)
        )
        .elements
    );
    let chain = result
        .elements
        .iter()
        .find(|element| element.metaclass == kc::FEATURE)
        .unwrap()
        .key
        .element_id();
    let mut plan = queries.kerml().plan_result_structure([]);
    for element in result.elements {
        plan.add_derived_element(
            element.key,
            element.metaclass,
            element.slots,
            element.owner,
            &result.evidence,
        )
        .unwrap();
    }
    // A second, independent ancestor is a discovery candidate, not part of the
    // positive proof of the existing trigger/payload chain.
    plan.add_derived_element(
        agq_kernel::DerivationKey {
            rule: agq_kernel::RuleId::from_u128(9002),
            subject: id(102),
            output: agq_kernel::OutputKey::from_u128(9003),
        },
        kc::SUBSETTING,
        BTreeMap::from([
            (
                kp::SUBSETTING_SUBSETTING_FEATURE,
                SlotValue::Scalar(Value::Reference(id(102))),
            ),
            (
                kp::SUBSETTING_SUBSETTED_FEATURE,
                SlotValue::Scalar(Value::Reference(id(9000))),
            ),
        ]),
        Some(id(102)),
        &queries
            .kerml()
            .canonical_fact_evidence(agq_kernel::provenance::FactKey::Element(id(9001))),
    )
    .unwrap();
    let derived = plan.materialize(&snapshot).unwrap();
    assert_eq!(derived.production.completeness, Completeness::Complete);
    let derived_queries = KerMlQueries::new(
        SemanticContext::for_overlay(
            &derived.overlay,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    assert_eq!(
        derived_queries.chaining_features(chain).value,
        [id(110), id(111)]
    );
    assert!(
        derived_queries
            .all_supertypes(id(102))
            .value
            .contains(&chain)
    );
    assert!(
        derived_queries
            .all_supertypes(id(102))
            .positive_dependencies
            .contains(&agq_kernel::provenance::FactKey::Element(id(9001)))
    );
    let repeated = crate::transition::plan_transition_payload(
        &derived_queries,
        SysmlBaselineProfile::OperationalV1,
        id(100),
    );
    assert_eq!(
        repeated.evidence.completeness,
        Completeness::Complete,
        "{:?}",
        repeated.evidence
    );
    assert!(repeated.elements.is_empty());
    assert!(
        !repeated
            .evidence
            .positive_dependencies
            .contains(&agq_kernel::provenance::FactKey::Element(id(9001)))
    );
}

#[test]
fn perform_constraint_and_variant_naming_follow_their_distinct_published_targets() {
    let mut f = Fixture::new();
    for (n, class, name) in [
        (1, sc::ACTION_DEFINITION, "owner"),
        (2, sc::PERFORM_ACTION_USAGE, ""),
        (3, sc::ACTION_USAGE, "behavior"),
        (4, kc::FEATURE, "chain"),
        (5, sc::REQUIREMENT_DEFINITION, "requirement"),
        (6, sc::CONSTRAINT_USAGE, ""),
        (7, sc::CONSTRAINT_USAGE, "condition"),
        (8, kc::FEATURE, "constraintChain"),
        (9, sc::ACTION_USAGE, ""),
        (10, sc::ACTION_USAGE, "variation"),
        (11, sc::PERFORM_ACTION_USAGE, ""),
        (12, sc::PERFORM_ACTION_USAGE, ""),
        (13, kc::FEATURE, "nonOccurrence"),
    ] {
        f.create(n, class, name);
        if name.is_empty() {
            f.changes.clear(id(n), kp::ELEMENT_DECLARED_NAME);
        }
    }
    f.member(1, 2, 102, kc::FEATURE_MEMBERSHIP);
    f.member(5, 6, 106, sc::REQUIREMENT_CONSTRAINT_MEMBERSHIP);
    f.member(10, 9, 109, sc::VARIANT_MEMBERSHIP);
    f.member(1, 11, 111, kc::FEATURE_MEMBERSHIP);
    for (chain, target, relationship) in [(4, 3, 204), (8, 7, 208)] {
        f.create(relationship, kc::FEATURE_CHAINING, "");
        f.owned
            .entry(id(chain))
            .or_default()
            .push(Value::Reference(id(relationship)));
        f.value(
            relationship,
            kp::FEATURE_CHAINING_CHAINING_FEATURE,
            Value::Reference(id(target)),
        );
    }
    for (subject, target, relation) in [(2, 4, 302), (6, 8, 306), (9, 4, 309), (12, 13, 312)] {
        f.create(relation, kc::REFERENCE_SUBSETTING, "");
        f.owned
            .entry(id(subject))
            .or_default()
            .push(Value::Reference(id(relation)));
        f.value(
            relation,
            kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
            Value::Reference(id(target)),
        );
    }
    let snapshot = f.finish();
    let queries = q(&snapshot);
    for (subject, target, name) in [(2, 3, "behavior"), (6, 7, "condition"), (9, 4, "chain")] {
        let source = current_sysml_naming_source(queries.kerml(), id(subject));
        assert_eq!(source.completeness, Completeness::Complete, "{source:?}");
        assert_eq!(source.value, Some(Some(id(target))));
        assert_eq!(
            queries.kerml().effective_names(id(subject)).value,
            EffectiveNames::Determinate(BTreeSet::from([name.into()]))
        );
    }
    assert_eq!(
        current_sysml_naming_source(queries.kerml(), id(11)).value,
        None
    );
    let non_occurrence = current_sysml_naming_source(queries.kerml(), id(12));
    assert_eq!(non_occurrence.completeness, Completeness::Complete);
    assert_eq!(non_occurrence.value, Some(None));
}
