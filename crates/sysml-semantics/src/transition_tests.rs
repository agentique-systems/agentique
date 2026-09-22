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
fn transition_payload_specialization_materializes_ordered_chain_and_is_idempotent() {
    let snapshot = fixture(true, true).finish();
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
    for (subject, target, relation) in [(2, 4, 302), (6, 8, 306), (9, 4, 309)] {
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
}
