include!("common/result_fixture.rs");

#[test]
fn positional_parameters_redefine_inherited_chain_target_members_even_when_renamed() {
    let mut f = Fixture::new();
    f.create(1, c::BEHAVIOR);
    f.create(2, c::STEP);
    f.create(3, c::FEATURE);
    f.create(4, c::FEATURE);
    f.create(5, c::STEP);
    f.create(6, c::STEP);
    f.create(7, c::FEATURE);
    f.value(3, p::ELEMENT_DECLARED_NAME, Value::String("input".into()));
    f.value(
        7,
        p::ELEMENT_DECLARED_NAME,
        Value::String("renamedInput".into()),
    );
    for n in [3, 7] {
        f.enumeration(n, p::FEATURE_DIRECTION, "in");
    }
    member(&mut f, 1, 3, 103, c::PARAMETER_MEMBERSHIP);
    member(&mut f, 2, 7, 107, c::PARAMETER_MEMBERSHIP);
    relation(&mut f, 6, 1, 106, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(
        106,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(6)),
    );
    relation(
        &mut f,
        5,
        4,
        104,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    relation(
        &mut f,
        5,
        6,
        105,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    subset(&mut f, 2, 5, 102);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    assert_eq!(q.structural_parameter_features(id(5)).value, vec![id(3)]);
    let redefined = q.redefined_features(id(7));
    assert_eq!(
        redefined.completeness,
        Completeness::Complete,
        "{:?}",
        redefined.diagnostics
    );
    assert_eq!(redefined.value, vec![id(3)]);
    let members = q.namespace_members(id(2), MemberAccess::All);
    assert_eq!(
        members.completeness,
        Completeness::Complete,
        "{:?}",
        members.diagnostics
    );
    assert_eq!(
        members.value,
        vec![MemberMatch {
            membership: id(107),
            element: id(7)
        }]
    );
    assert!(
        q.lookup_member(id(2), "input", MemberAccess::All)
            .value
            .is_empty()
    );
    assert_eq!(
        q.lookup_member(id(2), "renamedInput", MemberAccess::All)
            .value[0]
            .element,
        id(7)
    );
}

#[test]
fn current_graph_owner_typing_cannot_disprove_an_antecedent_without_producer_closure() {
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.create(2, c::BEHAVIOR);
    f.create(3, c::STEP);
    f.create(4, c::STRUCTURE);
    f.value(3, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    member(&mut f, 1, 3, 13, c::FEATURE_MEMBERSHIP);
    relation(&mut f, 1, 2, 12, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(12, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    relation(&mut f, 3, 4, 34, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(34, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(3)));
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let answer = q.formal_constraint_applies(
        FormalConstraintId::StepOwnedPerformanceSpecialization,
        id(3),
    );
    assert!(!answer.value);
    assert_eq!(
        answer.completeness,
        Completeness::Incomplete,
        "{:?}",
        answer.diagnostics
    );
    assert!(
        answer
            .positive_dependencies
            .contains(&FactKey::Element(id(2)))
    );
    assert!(
        answer
            .search_dependencies
            .contains(&SearchDependency::ProducerClosure {
                subject: id(1),
                requirement: SemanticClosureRequirement::EffectiveTyping,
                certificate_digest: None,
            })
    );
    assert!(
        q.formal_constraint_applies(FormalConstraintId::StepSubperformanceSpecialization, id(3))
            .value
    );
}

#[test]
fn positive_owner_typing_does_not_wait_for_negative_closure() {
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.create(2, c::STRUCTURE);
    f.create(3, c::STEP);
    f.value(3, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    member(&mut f, 1, 3, 13, c::FEATURE_MEMBERSHIP);
    relation(&mut f, 1, 2, 12, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(12, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let answer = q.formal_constraint_applies(
        FormalConstraintId::StepOwnedPerformanceSpecialization,
        id(3),
    );
    assert!(answer.value);
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{:?}",
        answer.diagnostics
    );
    assert!(
        !answer
            .search_dependencies
            .iter()
            .any(|search| matches!(search, SearchDependency::ProducerClosure { .. }))
    );
}

#[test]
fn unfinished_type_producers_do_not_establish_negative_owner_typing() {
    for case in 0..4 {
        let mut f = Fixture::new();
        f.create(1, c::STEP);
        f.create(2, c::BEHAVIOR);
        f.create(3, c::STEP);
        f.value(3, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        member(&mut f, 1, 3, 13, c::FEATURE_MEMBERSHIP);
        relation(&mut f, 1, 2, 12, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
        f.value(12, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
        match case {
            0 => f.value(1, p::FEATURE_IS_COMPOSITE, Value::Boolean(true)),
            1 => f.value(1, p::FEATURE_IS_END, Value::Boolean(true)),
            2 => f.enumeration(1, p::FEATURE_DIRECTION, "in"),
            _ => {
                f.create(7, c::EXPRESSION);
                member(&mut f, 1, 7, 17, c::FEATURE_VALUE);
            }
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        assert_eq!(q.feature_types(id(1)).completeness, Completeness::Complete);
        let answer = q.formal_constraint_applies(
            FormalConstraintId::StepOwnedPerformanceSpecialization,
            id(3),
        );
        assert_eq!(answer.completeness, Completeness::Incomplete, "case {case}");
        assert!(
            answer
                .diagnostics
                .iter()
                .any(|d| d.code == "KQ_FORMAL_ANTECEDENT_TYPE_CLOSURE")
        );
    }
}
