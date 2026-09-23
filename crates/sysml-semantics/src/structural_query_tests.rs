//! Canonical projection and revision regressions; positive certificate coverage
//! is also exercised by the real combined scheduler in producer_tests.
use super::*;
use agq_kerml_semantics::{SearchDependency, SemanticClosureRequirement};
use agq_kernel::provenance::FactKey;
use agq_sysml::properties as sp;

fn enumeration(f: &mut Fixture, subject: u128, property: PropertyId, name: &str) {
    let ValueKind::Enumeration(domain) = f
        .base
        .model()
        .registry()
        .property(property)
        .unwrap()
        .value_kind
    else {
        panic!("enum");
    };
    let literal = *f
        .base
        .model()
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, candidate)| candidate.as_str() == name)
        .unwrap()
        .0;
    f.value(subject, property, Value::Enumeration(literal));
}

fn has_closure_read<T>(answer: &SysmlQueryResult<T>, subject: ElementId) -> bool {
    answer.supporting_queries.iter().any(|proof| {
        proof.search_dependencies.iter().any(|read|
        matches!(read, SearchDependency::ProducerClosure { subject: id, .. } if *id == subject))
    })
}

#[test]
fn all_usage_families_preserve_inherited_ids_and_closed_empty_results_need_evidence() {
    let mut f = Fixture::new();
    f.create(1, sc::PART_DEFINITION, "Base");
    f.create(2, sc::PART_DEFINITION, "Specialized");
    f.relation(
        2,
        1,
        90,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    let families = [
        (UsageKind::Attribute, sc::ATTRIBUTE_USAGE),
        (UsageKind::Item, sc::ITEM_USAGE),
        (UsageKind::Part, sc::PART_USAGE),
        (UsageKind::Port, sc::PORT_USAGE),
        (UsageKind::Connection, sc::CONNECTION_USAGE),
        (UsageKind::Interface, sc::INTERFACE_USAGE),
        (UsageKind::Occurrence, sc::OCCURRENCE_USAGE),
        (UsageKind::Action, sc::ACTION_USAGE),
        (UsageKind::State, sc::STATE_USAGE),
        (UsageKind::Requirement, sc::REQUIREMENT_USAGE),
        (UsageKind::Constraint, sc::CONSTRAINT_USAGE),
        (UsageKind::Case, sc::CASE_USAGE),
        (UsageKind::VerificationCase, sc::VERIFICATION_CASE_USAGE),
    ];
    for (index, (_, class)) in families.iter().enumerate() {
        let subject = 10 + index as u128;
        f.create(subject, *class, &format!("member{index}"));
        f.member(1, subject, 100 + subject, kc::FEATURE_MEMBERSHIP);
        f.value(subject, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    }
    let snapshot = f.finish();
    let model = snapshot.model();
    let ids: BTreeSet<_> = model.elements().map(|record| record.id()).collect();
    let query = q(&snapshot);
    for (kind, class) in families {
        let direct = query.owned_usages_of_kind(id(1), kind);
        assert_eq!(
            direct.completeness(),
            Completeness::Complete,
            "{kind:?}: {direct:?}"
        );
        let inherited = query.effective_usages_of_kind(id(2), kind);
        assert_eq!(inherited.value(), direct.value(), "{kind:?}");
        assert_eq!(inherited.completeness(), Completeness::Incomplete);
        assert!(inherited.value().iter().all(|id| {
            model
                .registry()
                .is_subtype(model.element(*id).unwrap().metaclass(), class)
                .unwrap()
        }));
        assert!(has_closure_read(&inherited, id(1)));
        assert!(has_closure_read(&inherited, id(2)));
    }
    let empty = query.effective_ports(id(13));
    assert!(empty.value().is_empty());
    assert_eq!(empty.completeness(), Completeness::Incomplete);
    assert!(
        has_closure_read(&empty, id(13)),
        "empty populations must retain their closure search"
    );
    assert_eq!(ids, model.elements().map(|record| record.id()).collect());
}

#[test]
fn membership_roles_are_semantic_and_no_body_executes() {
    let mut f = Fixture::new();
    f.create(1, sc::STATE_DEFINITION, "State");
    for (subject, role) in [(11, "entry"), (12, "do"), (13, "exit")] {
        f.create(subject, sc::PERFORM_ACTION_USAGE, "action");
        f.member(1, subject, subject + 100, sc::STATE_SUBACTION_MEMBERSHIP);
        enumeration(
            &mut f,
            subject + 100,
            sp::STATE_SUBACTION_MEMBERSHIP_KIND,
            role,
        );
    }
    f.create(2, sc::TRANSITION_USAGE, "transition");
    for (subject, class, role) in [
        (21, sc::ACCEPT_ACTION_USAGE, "trigger"),
        (22, sc::CONSTRAINT_USAGE, "guard"),
        (23, sc::ACTION_USAGE, "effect"),
    ] {
        f.create(subject, class, "feature");
        f.member(2, subject, subject + 100, sc::TRANSITION_FEATURE_MEMBERSHIP);
        enumeration(
            &mut f,
            subject + 100,
            sp::TRANSITION_FEATURE_MEMBERSHIP_KIND,
            role,
        );
    }
    f.create(3, sc::CASE_DEFINITION, "case");
    for (subject, class, membership) in [
        (31, sc::REFERENCE_USAGE, sc::SUBJECT_MEMBERSHIP),
        (32, sc::PART_USAGE, sc::ACTOR_MEMBERSHIP),
        (33, sc::REQUIREMENT_USAGE, sc::OBJECTIVE_MEMBERSHIP),
    ] {
        f.create(subject, class, "role");
        f.member(3, subject, subject + 100, membership);
    }
    for (subject, role) in [(41, "in"), (42, "in")] {
        f.create(subject, sc::REFERENCE_USAGE, "parameter");
        enumeration(&mut f, subject, kp::FEATURE_DIRECTION, role);
        f.member(21, subject, subject + 100, kc::PARAMETER_MEMBERSHIP);
    }
    f.create(43, sc::REFERENCE_USAGE, "returnValue");
    enumeration(&mut f, 43, kp::FEATURE_DIRECTION, "out");
    f.member(3, 43, 143, kc::RETURN_PARAMETER_MEMBERSHIP);
    let snapshot = f.finish();
    let query = q(&snapshot);
    for (kind, expected) in [
        (StateSubactionKind::Entry, 11),
        (StateSubactionKind::Do, 12),
        (StateSubactionKind::Exit, 13),
    ] {
        let answer = query.state_actions(id(1), kind);
        assert_eq!(answer.value(), &[id(expected)]);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        assert!(has_closure_read(&answer, id(1)));
        assert!(answer.observations.contains_key(&FactKey::Property {
            element: id(expected + 100),
            property: sp::STATE_SUBACTION_MEMBERSHIP_KIND
        }));
    }
    for (kind, expected) in [
        (TransitionFeatureKind::Trigger, 21),
        (TransitionFeatureKind::Guard, 22),
        (TransitionFeatureKind::Effect, 23),
    ] {
        let answer = query.transition_features(id(2), kind);
        assert_eq!(answer.value(), &[id(expected)]);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
    }
    for (role, expected) in [
        (RequirementCaseRole::Subject, 31),
        (RequirementCaseRole::Actor, 32),
        (RequirementCaseRole::Objective, 33),
    ] {
        let answer = query.requirement_case_features(id(3), role);
        assert_eq!(answer.value(), &[id(expected)]);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
    }
    assert_eq!(
        query.effective_parameters(id(21)).value(),
        &[id(41), id(42)]
    );
    assert_eq!(
        query.accept_action_payload_parameter(id(21)).value(),
        &[id(41)]
    );
    assert_eq!(query.effective_return_parameters(id(3)).value(), &[id(43)]);
    assert_eq!(
        query
            .state_actions(id(3), StateSubactionKind::Entry)
            .completeness(),
        Completeness::Invalid
    );
}

#[test]
fn accept_payload_parameter_is_distinct_from_the_accepted_message_feature() {
    let mut f = Fixture::new();
    f.create(1, sc::ACTION_DEFINITION, "AcceptMessageAction");
    f.create(2, sc::ACCEPT_ACTION_USAGE, "accepter");
    f.relation(2, 1, 102, kc::FEATURE_TYPING, kp::FEATURE_TYPING_TYPE);
    f.create(10, sc::REFERENCE_USAGE, "payload");
    enumeration(&mut f, 10, kp::FEATURE_DIRECTION, "inout");
    f.member(1, 10, 110, kc::PARAMETER_MEMBERSHIP);
    f.create(11, sc::REFERENCE_USAGE, "acceptedMessage");
    f.member(1, 11, 111, kc::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let query = q(&snapshot);

    let payload = query.accept_action_payload_parameter(id(2));
    assert_eq!(payload.value(), &[id(10)]);
    assert_eq!(payload.completeness(), Completeness::Incomplete);
    let members = query.effective_usages(id(2));
    assert!(members.value().contains(&id(11)));
    assert_eq!(members.completeness(), Completeness::Incomplete);
    let message = query.kerml().lookup_path(
        id(2),
        &agq_kerml_semantics::QualifiedName {
            absolute: false,
            segments: vec!["acceptedMessage".into()],
        },
    );
    assert_eq!(message.completeness, Completeness::Complete);
    assert_eq!(message.value.len(), 1);
    assert_eq!(message.value[0].element, id(11));
    assert!(!payload.value().contains(&message.value[0].element));
    assert!(has_closure_read(&payload, id(2)));
    assert!(has_closure_read(&members, id(2)));
}

#[test]
fn composite_and_naming_queries_keep_old_revision_answers_and_searches() {
    let mut f = vertical();
    f.value(3, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    let first = f.finish();
    let first_query = q(&first);
    let inherited = first_query.effective_subparts(id(4));
    assert_eq!(inherited.value(), &[id(3)]);
    assert_eq!(inherited.completeness(), Completeness::Incomplete);
    assert!(inherited.observations.contains_key(&FactKey::Property {
        element: id(3),
        property: kp::FEATURE_IS_COMPOSITE
    }));
    let name = first_query.effective_qualified_name(id(3));
    assert_eq!(
        name.value(),
        first_query.current_qualified_name(id(3)).value()
    );
    assert_eq!(name.completeness(), Completeness::Incomplete);
    assert!(name.supporting_queries.iter().any(|proof| proof.search_dependencies.iter().any(|read|
        matches!(read, SearchDependency::ProducerClosure { subject, requirement: SemanticClosureRequirement::EffectiveNaming, .. } if *subject == id(10_000)))));
    let mut changes = first.change_set();
    changes.set(
        id(3),
        kp::FEATURE_IS_COMPOSITE,
        SlotValue::Scalar(Value::Boolean(false)),
        origin(),
    );
    changes.set(
        id(3),
        kp::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("renamed".into())),
        origin(),
    );
    let second = first.apply(&changes).unwrap();
    let second_query = q(&second);
    assert!(second_query.effective_subparts(id(4)).value().is_empty());
    assert_eq!(first_query.effective_subparts(id(4)).value(), &[id(3)]);
    assert_ne!(first_query.context(), second_query.context());
    assert_ne!(
        first_query.current_qualified_name(id(3)).value(),
        second_query.current_qualified_name(id(3)).value()
    );
    assert_eq!(
        first_query.current_qualified_name(id(3)).value(),
        name.value()
    );
}

#[test]
fn connector_and_interface_end_order_and_typed_domains_retain_closure_evidence() {
    let mut f = Fixture::new();
    f.create(1, sc::INTERFACE_DEFINITION, "Interface");
    f.create(2, sc::CONNECTION_USAGE, "connection");
    f.create(3, sc::PORT_DEFINITION, "Port");
    for subject in [22, 21] {
        f.create(subject, sc::PORT_USAGE, "end");
        f.value(subject, kp::FEATURE_IS_END, Value::Boolean(true));
        f.member(1, subject, subject + 100, kc::END_FEATURE_MEMBERSHIP);
        f.relation(
            subject,
            3,
            subject + 200,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
    }
    for (subject, endpoint) in [(32, 22), (31, 21)] {
        f.create(subject, sc::REFERENCE_USAGE, "end");
        f.value(subject, kp::FEATURE_IS_END, Value::Boolean(true));
        f.member(2, subject, subject + 100, kc::END_FEATURE_MEMBERSHIP);
        f.create(subject + 200, kc::REFERENCE_SUBSETTING, "reference");
        f.owned
            .entry(id(subject))
            .or_default()
            .push(Value::Reference(id(subject + 200)));
        f.value(
            subject + 200,
            kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
            Value::Reference(id(endpoint)),
        );
    }
    for (usage, usage_class, definition, definition_class) in [
        (40, sc::ATTRIBUTE_USAGE, 41, sc::ATTRIBUTE_DEFINITION),
        (50, sc::ITEM_USAGE, 51, sc::ITEM_DEFINITION),
        (60, sc::PART_USAGE, 61, sc::PART_DEFINITION),
    ] {
        f.create(usage, usage_class, "usage");
        f.create(definition, definition_class, "definition");
        f.relation(
            usage,
            definition,
            usage + 200,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
    }
    let snapshot = f.finish();
    let query = q(&snapshot);
    let interface_ends = query.effective_interface_ends(id(1));
    assert_eq!(interface_ends.value(), &[id(22), id(21)]);
    assert_eq!(interface_ends.completeness(), Completeness::Incomplete);
    let connection_ends = query.effective_connection_ends(id(2));
    assert_eq!(connection_ends.value(), &[id(32), id(31)]);
    assert_eq!(connection_ends.completeness(), Completeness::Incomplete);
    let related = query.effective_connection_related_features(id(2));
    assert_eq!(related.value(), &[id(22), id(21)]);
    assert_eq!(related.completeness(), Completeness::Incomplete);
    assert!(has_closure_read(&related, id(2)));
    for (answer, subject, definition) in [
        (query.effective_attribute_definitions(id(40)), 40, 41),
        (query.effective_item_definitions(id(50)), 50, 51),
        (query.effective_part_definitions(id(60)), 60, 61),
        (query.effective_port_definitions(id(21)), 21, 3),
    ] {
        assert_eq!(answer.value(), &[id(definition)]);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        assert!(has_closure_read(&answer, id(subject)));
    }
    assert_eq!(
        query.effective_interface_ends(id(2)).completeness(),
        Completeness::Invalid
    );
}

#[test]
fn nested_usage_populations_compose_typing_redefinition_and_composite_filters() {
    let mut f = Fixture::new();
    f.create(1, sc::PART_DEFINITION, "BaseAssembly");
    f.create(2, sc::PART_DEFINITION, "SpecializedAssembly");
    f.relation(
        2,
        1,
        102,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f.create(3, sc::PART_USAGE, "instance");
    f.relation(3, 2, 103, kc::FEATURE_TYPING, kp::FEATURE_TYPING_TYPE);
    for (subject, owner, class, composite) in [
        (10, 1, sc::PART_USAGE, true),
        (11, 1, sc::ITEM_USAGE, true),
        (12, 2, sc::PART_USAGE, true),
        (13, 3, sc::PART_USAGE, false),
        (14, 3, sc::ACTION_USAGE, true),
    ] {
        f.create(subject, class, &format!("child{subject}"));
        f.member(owner, subject, 200 + subject, kc::FEATURE_MEMBERSHIP);
        f.value(subject, kp::FEATURE_IS_COMPOSITE, Value::Boolean(composite));
    }
    f.relation(
        12,
        10,
        112,
        kc::REDEFINITION,
        kp::REDEFINITION_REDEFINED_FEATURE,
    );
    let snapshot = f.finish();
    let query = q(&snapshot);
    assert_eq!(query.nested_usages(id(3)).value(), &[id(13), id(14)]);
    for (answer, expected) in [
        (
            query.effective_nested_usages(id(3)),
            vec![id(11), id(12), id(13), id(14)],
        ),
        (query.effective_subitems(id(3)), vec![id(11), id(12)]),
        (query.effective_subparts(id(3)), vec![id(12)]),
        (query.effective_subactions(id(3)), vec![id(14)]),
    ] {
        assert_eq!(
            answer.value().iter().copied().collect::<BTreeSet<_>>(),
            expected.into_iter().collect()
        );
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        for subject in [1, 2, 3] {
            assert!(has_closure_read(&answer, id(subject)));
        }
        assert!(
            !answer.value().contains(&id(10)),
            "redefined original is suppressed without copying the inherited replacement"
        );
    }
    for answer in [
        query.effective_subsetted_features(id(12)),
        query.effective_redefined_features(id(12)),
    ] {
        assert_eq!(answer.value(), &[id(10)]);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        assert!(has_closure_read(&answer, id(12)));
    }
    assert_eq!(
        query.effective_nested_usages(id(1)).completeness(),
        Completeness::Invalid
    );
}

#[test]
fn connector_as_usage_supports_binding_and_succession_without_narrowing_to_connection_usage() {
    let mut f = Fixture::new();
    f.create(1, sc::BINDING_CONNECTOR_AS_USAGE, "binding");
    f.create(2, sc::SUCCESSION_AS_USAGE, "succession");
    for endpoint in [81, 82] {
        f.create(endpoint, sc::REFERENCE_USAGE, "endpoint");
    }
    for (owner, ends) in [(1, [(12, 82), (11, 81)]), (2, [(22, 82), (21, 81)])] {
        for (end, endpoint) in ends {
            f.create(end, sc::REFERENCE_USAGE, "end");
            f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
            f.member(owner, end, end + 100, kc::END_FEATURE_MEMBERSHIP);
            f.create(end + 200, kc::REFERENCE_SUBSETTING, "reference");
            f.owned
                .entry(id(end))
                .or_default()
                .push(Value::Reference(id(end + 200)));
            f.value(
                end + 200,
                kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
                Value::Reference(id(endpoint)),
            );
        }
    }
    let snapshot = f.finish();
    let query = q(&snapshot);
    for (owner, ends) in [(1, vec![id(12), id(11)]), (2, vec![id(22), id(21)])] {
        let actual_ends = query.effective_connection_ends(id(owner));
        assert_eq!(actual_ends.value(), &ends);
        assert_eq!(actual_ends.completeness(), Completeness::Incomplete);
        let related = query.effective_connection_related_features(id(owner));
        assert_eq!(related.value(), &[id(82), id(81)]);
        assert_eq!(related.completeness(), Completeness::Incomplete);
        assert!(related.rejected_targets.is_empty());
        assert!(has_closure_read(&related, id(owner)));
        assert_eq!(
            query.effective_interface_ends(id(owner)).completeness(),
            Completeness::Invalid
        );
    }
}

#[test]
fn verification_cases_inherit_membership_roles_and_redefine_subject_without_copies() {
    let mut f = Fixture::new();
    f.create(1, sc::CASE_DEFINITION, "BaseCase");
    f.create(2, sc::VERIFICATION_CASE_DEFINITION, "Verification");
    f.relation(
        2,
        1,
        102,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    for (subject, class, membership) in [
        (11, sc::REFERENCE_USAGE, sc::SUBJECT_MEMBERSHIP),
        (12, sc::PART_USAGE, sc::ACTOR_MEMBERSHIP),
        (13, sc::REQUIREMENT_USAGE, sc::OBJECTIVE_MEMBERSHIP),
        (14, sc::REFERENCE_USAGE, kc::RETURN_PARAMETER_MEMBERSHIP),
    ] {
        f.create(subject, class, &format!("role{subject}"));
        f.member(1, subject, 200 + subject, membership);
    }
    enumeration(&mut f, 14, kp::FEATURE_DIRECTION, "out");
    f.create(31, sc::REFERENCE_USAGE, "specificSubject");
    f.member(2, 31, 231, sc::SUBJECT_MEMBERSHIP);
    f.relation(
        31,
        11,
        131,
        kc::REDEFINITION,
        kp::REDEFINITION_REDEFINED_FEATURE,
    );
    f.create(4, sc::PART_DEFINITION, "CaseHost");
    for (subject, class, definition) in
        [(5, sc::CASE_USAGE, 1), (6, sc::VERIFICATION_CASE_USAGE, 2)]
    {
        f.create(subject, class, "case");
        f.member(4, subject, 200 + subject, kc::FEATURE_MEMBERSHIP);
        f.relation(
            subject,
            definition,
            100 + subject,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
    }
    let snapshot = f.finish();
    let query = q(&snapshot);
    for (role, expected) in [
        (RequirementCaseRole::Subject, 31),
        (RequirementCaseRole::Actor, 12),
        (RequirementCaseRole::Objective, 13),
    ] {
        let answer = query.requirement_case_features(id(2), role);
        assert_eq!(answer.value(), &[id(expected)]);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        assert!(has_closure_read(&answer, id(1)));
        assert!(has_closure_read(&answer, id(2)));
    }
    let returns = query.effective_return_parameters(id(2));
    assert_eq!(returns.value(), &[id(14)]);
    assert_eq!(returns.completeness(), Completeness::Incomplete);
    assert_eq!(
        query.owned_usages_of_kind(id(4), UsageKind::Case).value(),
        &[id(5), id(6)]
    );
    assert_eq!(
        query
            .effective_usages_of_kind(id(4), UsageKind::VerificationCase)
            .value(),
        &[id(6)]
    );
}
