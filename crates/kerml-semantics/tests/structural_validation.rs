include!("common/namespace_fixture.rs");

#[test]
fn duplicate_feature_names_are_invalid_but_distinct_kinds_remain_distinguishable() {
    for comparable in [false, true] {
        let mut f = Fixture::new();
        f.create(1, c::NAMESPACE);
        f.member(1, 103, 3, c::FEATURE, "shared");
        f.member(
            1,
            104,
            4,
            if comparable { c::FEATURE } else { c::CLASS },
            "shared",
        );
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        let result = q.validate_namespace_distinguishability(id(1));
        assert_eq!(
            result.completeness,
            if comparable {
                Completeness::Invalid
            } else {
                Completeness::Complete
            }
        );
        assert_eq!(result.value, vec!["validateNamespaceDistinguishibility"]);
        assert!(
            result
                .positive_dependencies
                .contains(&FactKey::Element(id(3)))
        );
        assert!(
            result
                .positive_dependencies
                .contains(&FactKey::Element(id(4)))
        );
        // Constraint success does not select a winner for an ambiguous name.
        assert_eq!(
            q.lookup_member(id(1), "shared", MemberAccess::All)
                .value
                .len(),
            2
        );
    }
}

#[test]
fn structural_end_checks_reject_directions_and_nonconstant_variable_ends() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 103, 3, c::FEATURE, "invalidEnd");
    f.value(3, p::FEATURE_IS_END, Value::Boolean(true));
    f.value(3, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
    f.enumeration(3, p::FEATURE_DIRECTION, "in");
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let result = q.validate_local_structure(id(3));
    assert_eq!(result.completeness, Completeness::Invalid);
    let codes: BTreeSet<_> = result.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        BTreeSet::from([
            "validateFeatureEndNoDirection",
            "validateFeatureEndIsConstant"
        ])
    );
    for property in [
        p::FEATURE_IS_END,
        p::FEATURE_IS_VARIABLE,
        p::FEATURE_IS_CONSTANT,
        p::FEATURE_DIRECTION,
    ] {
        assert!(result.positive_dependencies.contains(&FactKey::Property {
            element: id(3),
            property
        }));
    }
}

#[test]
fn one_step_feature_chain_is_a_structural_failure_even_with_a_resolved_target() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 103, 3, c::FEATURE, "chain");
    f.member(1, 104, 4, c::FEATURE, "target");
    f.create(200, c::FEATURE_CHAINING);
    f.value(
        200,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
        Value::Reference(id(4)),
    );
    f.own(3, 200);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let result = q.validate_local_structure(id(3));
    assert_eq!(result.completeness, Completeness::Invalid);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        vec!["validateFeatureChainingFeatureNotOne"]
    );
}
