include!("common/result_fixture.rs");

#[test]
fn association_source_and_targets_follow_end_order_and_preserve_repeated_types() {
    let mut f = Fixture::new();
    f.create(1, c::ASSOCIATION);
    f.create(20, c::CLASSIFIER);
    f.create(21, c::CLASSIFIER);
    for (end, ty, membership, typing) in [
        (13, 20, 30, 40),
        (12, 21, 31, 41),
        (11, 20, 32, 42),
        (10, 21, 33, 43),
    ] {
        f.create(end, c::FEATURE);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, end, membership, c::END_FEATURE_MEMBERSHIP);
        relation(
            &mut f,
            end,
            ty,
            typing,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            typing,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(end)),
        );
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let answer = q.association_structure(id(1));
    assert_eq!(answer.completeness, Completeness::Complete);
    assert_eq!(answer.value.ends, vec![id(13), id(12), id(11), id(10)]);
    assert_eq!(
        answer.value.related_types,
        vec![id(20), id(21), id(20), id(21)]
    );
    assert_eq!(answer.value.source_type, Some(id(20)));
    assert_eq!(answer.value.target_types, vec![id(21), id(20)]);
}

#[test]
fn connector_projection_keeps_both_occurrences_of_a_self_binding() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    binding(&mut f, 1, 20, c::FEATURE_MEMBERSHIP, 2, 2);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let answer = q.connector_structure(id(20));
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{:?}",
        answer.diagnostics
    );
    assert_eq!(answer.value.related_features, vec![id(2), id(2)]);
    assert_eq!(answer.value.source_feature, Some(id(2)));
    assert_eq!(answer.value.target_features, vec![id(2)]);
    assert_eq!(answer.value.default_featuring_type, Some(id(1)));
    assert!(answer.positive_dependencies.contains(&FactKey::Property {
        element: id(24),
        property: p::REFERENCE_SUBSETTING_REFERENCED_FEATURE
    }));
}
