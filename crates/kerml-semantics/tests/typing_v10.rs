include!("common/result_fixture.rs");

#[test]
fn typing_closure_excludes_cross_subsetting_and_removes_redundant_types() {
    let mut f = Fixture::new();
    for n in [1, 2, 3, 4] {
        f.create(n, c::FEATURE);
    }
    for n in [10, 11, 12] {
        f.create(n, c::CLASSIFIER);
    }
    for (feature, ty, r) in [(1, 10, 100), (2, 11, 101), (4, 12, 102)] {
        relation(
            &mut f,
            feature,
            ty,
            r,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            r,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(feature)),
        );
    }
    relation(
        &mut f,
        11,
        10,
        103,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(103, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(11)));
    subset(&mut f, 1, 2, 104);
    subset(&mut f, 2, 3, 105);
    subset(&mut f, 3, 1, 106);
    relation(
        &mut f,
        1,
        4,
        107,
        c::CROSS_SUBSETTING,
        p::CROSS_SUBSETTING_CROSSED_FEATURE,
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let answer = q.feature_types(id(1));
    assert_eq!(answer.value, vec![id(11)]);
    assert_eq!(answer.completeness, Completeness::Complete);
}

#[test]
fn chain_terminal_contributes_typing_and_conjugation_replaces_it() {
    for conjugated in [false, true] {
        let mut f = Fixture::new();
        for n in [1, 2, 3, 4] {
            f.create(n, c::FEATURE);
        }
        for n in [10, 11] {
            f.create(n, c::CLASSIFIER);
        }
        for (feature, ty, r) in [(2, 10, 100), (4, 11, 101)] {
            relation(
                &mut f,
                feature,
                ty,
                r,
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPE,
            );
            f.value(
                r,
                p::FEATURE_TYPING_TYPED_FEATURE,
                Value::Reference(id(feature)),
            );
        }
        for (r, target) in [(109, 3), (108, 2)] {
            relation(
                &mut f,
                1,
                target,
                r,
                c::FEATURE_CHAINING,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
            );
        }
        if conjugated {
            relation(
                &mut f,
                1,
                4,
                110,
                c::CONJUGATION,
                p::CONJUGATION_ORIGINAL_TYPE,
            );
            f.value(110, p::CONJUGATION_CONJUGATED_TYPE, Value::Reference(id(1)));
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        assert_eq!(
            q.feature_types(id(1)).value,
            vec![id(if conjugated { 11 } else { 10 })]
        );
    }
}

#[test]
fn missing_subsetting_target_remains_incomplete_with_positive_type_evidence() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(10, c::CLASSIFIER);
    relation(
        &mut f,
        1,
        10,
        100,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        100,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(1)),
    );
    f.create(101, c::SUBSETTING);
    f.own(1, 101);
    f.value(
        101,
        p::SUBSETTING_SUBSETTING_FEATURE,
        Value::Reference(id(1)),
    );
    let candidate = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(&candidate, Default::default(), Default::default())
            .unwrap(),
    );
    let answer = q.feature_types(id(1));
    assert_eq!(answer.value, vec![id(10)]);
    assert_eq!(answer.completeness, Completeness::Incomplete);
}
