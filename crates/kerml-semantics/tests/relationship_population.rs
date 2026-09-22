include!("common/result_fixture.rs");

fn options() -> SemanticOptions {
    SemanticOptions {
        exclude_implied: true,
        ..Default::default()
    }
}

#[test]
fn incoming_general_fanout_is_a_negative_search_not_positive_evidence() {
    for count in [1, 128, 1024] {
        let mut f = Fixture::new();
        f.create(1, c::CLASSIFIER);
        f.create(2, c::FEATURE);
        for n in 0..count {
            let feature = 100 + n * 4;
            f.create(feature, c::FEATURE);
            relation(
                &mut f,
                feature,
                1,
                feature + 1,
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPE,
            );
            f.value(
                feature + 1,
                p::FEATURE_TYPING_TYPED_FEATURE,
                Value::Reference(id(feature)),
            );
            subset(&mut f, feature, 2, feature + 2);
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
        );
        for answer in [q.direct_specializations(id(1)), q.typing_features(id(2))] {
            assert_eq!(
                answer.completeness,
                Completeness::Complete,
                "{:?}",
                answer.diagnostics
            );
            assert!(answer.value.is_empty());
            assert!(
                answer.canonical_dependencies.len() < 16,
                "count={count}, dependencies={}",
                answer.canonical_dependencies.len()
            );
            assert!(answer.search_dependencies.len() < 16);
            assert!(
                answer
                    .search_dependencies
                    .iter()
                    .any(|d| matches!(d, SearchDependency::SourceRelationships { .. }))
            );
        }
        // FeatureTyping::typedFeature redefines Specialization::specific;
        // filtering must use the effective property, not the inherited raw ID.
        assert_eq!(q.direct_feature_types(id(100)).value, vec![id(1)]);
        assert_eq!(q.direct_specializations(id(100)).value, vec![id(1), id(2)]);
        assert_eq!(q.typing_features(id(100)).value, vec![id(2)]);
    }
}

#[test]
fn retargeting_a_specific_endpoint_invalidates_its_negative_population() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    f.create(3, c::FEATURE);
    f.create(4, c::FEATURE_TYPING);
    f.value(4, p::FEATURE_TYPING_TYPE, Value::Reference(id(1)));
    f.value(4, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(2)));
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    let before = q.direct_feature_types(id(3));
    assert!(before.value.is_empty());
    assert!(
        before
            .search_dependencies
            .contains(&SearchDependency::SourceRelationships {
                source: id(3),
                class: c::FEATURE_TYPING,
                property: p::FEATURE_TYPING_TYPED_FEATURE,
            })
    );
    let mut changes = snapshot.change_set();
    changes.set(
        id(4),
        p::FEATURE_TYPING_TYPED_FEATURE,
        SlotValue::Scalar(Value::Reference(id(3))),
        origin(),
    );
    let updated = snapshot.apply(&changes).unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&updated, options(), BTreeSet::new()).unwrap(),
    );
    assert!(q.direct_feature_types(id(2)).value.is_empty());
    assert_eq!(q.direct_feature_types(id(3)).value, vec![id(1)]);
}

#[test]
fn missing_specific_at_general_endpoint_does_not_poison_outgoing_query() {
    for owned in [false, true] {
        let mut f = Fixture::new();
        f.create(1, c::CLASSIFIER);
        f.create(2, c::FEATURE);
        f.create(3, c::FEATURE_TYPING);
        if owned {
            f.own(2, 3);
        }
        f.value(3, p::FEATURE_TYPING_TYPE, Value::Reference(id(1)));
        let construction = f.construction();
        let q = KerMlQueries::new(
            SemanticContext::for_construction(&construction, options(), BTreeSet::new()).unwrap(),
        );
        let unrelated = q.direct_specializations(id(1));
        assert_eq!(
            unrelated.completeness,
            Completeness::Complete,
            "{:?}",
            unrelated.diagnostics
        );
        assert!(unrelated.value.is_empty());
        if owned {
            // Ownership makes this a relevant incomplete relationship even
            // before its required source is computed.
            assert_ne!(
                q.direct_feature_types(id(2)).completeness,
                Completeness::Complete
            );
        }
    }
}

#[test]
fn source_matching_relationship_with_missing_general_remains_incomplete() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::FEATURE_TYPING);
    f.value(2, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    let construction = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(&construction, options(), BTreeSet::new()).unwrap(),
    );
    let answer = q.direct_feature_types(id(1));
    assert_ne!(answer.completeness, Completeness::Complete);
    assert!(answer.value.is_empty());
}
