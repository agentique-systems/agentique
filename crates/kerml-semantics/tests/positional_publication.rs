include!("common/result_fixture.rs");

#[test]
fn materialized_ordered_redefinitions_close_names_with_derived_provenance() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (owner, feature, name) in [(1, 11, Some("left")), (2, 12, Some("right")), (3, 13, None)] {
        f.create(owner, c::BEHAVIOR);
        f.create(feature, c::FEATURE);
        f.enumeration(feature, p::FEATURE_DIRECTION, "in");
        if let Some(name) = name {
            f.value(
                feature,
                p::ELEMENT_DECLARED_NAME,
                Value::String(name.into()),
            );
        }
        member(&mut f, owner, feature, 20 + owner, c::PARAMETER_MEMBERSHIP);
    }
    for (relationship, general) in [(31, 1), (32, 2)] {
        relation(
            &mut f,
            3,
            general,
            relationship,
            c::SPECIALIZATION,
            p::SPECIALIZATION_GENERAL,
        );
        f.value(
            relationship,
            p::SPECIALIZATION_SPECIFIC,
            Value::Reference(id(3)),
        );
    }
    let snapshot = f.finish();
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap(),
    );
    let before = q.effective_names(id(13));
    assert!(matches!(before.value, EffectiveNames::Ambiguous { .. }));
    let produced = q
        .plan_result_structure([id(13)])
        .materialize(&snapshot)
        .unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&produced.overlay, options, BTreeSet::new()).unwrap(),
    );
    let after = q.effective_names(id(13));
    assert_eq!(after.completeness, Completeness::Complete);
    let redefinitions = q.owned_relationships(id(13));
    let first = *redefinitions
        .value
        .iter()
        .find(|&&id| produced.overlay.model().element(id).unwrap().metaclass() == c::REDEFINITION)
        .unwrap();
    let target = produced
        .overlay
        .model()
        .navigation_slot(first, p::REDEFINITION_REDEFINED_FEATURE)
        .unwrap()
        .value()
        .values()
        .find_map(|v| {
            if let Value::Reference(id) = v {
                Some(*id)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(after.value, q.effective_names(target).value);
    assert!(matches!(
        produced.overlay.model().element(first).unwrap().origin(),
        Origin::Derived(_)
    ));
    assert!(
        after
            .fact_origins
            .values()
            .any(|origin| matches!(origin.as_ref(), Origin::Derived(_)))
    );
    let repeated = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    )
    .plan_result_structure([id(13), id(13)])
    .materialize(&snapshot)
    .unwrap();
    assert!(
        produced
            .overlay
            .model()
            .elements()
            .eq(repeated.overlay.model().elements())
    );
}

#[test]
fn positional_relationships_supply_typing_and_suppress_renamed_inherited_features() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (offset, class, membership) in [
        (0, c::BEHAVIOR, c::PARAMETER_MEMBERSHIP),
        (100, c::FUNCTION, c::RETURN_PARAMETER_MEMBERSHIP),
        (200, c::ASSOCIATION, c::END_FEATURE_MEMBERSHIP),
    ] {
        for owner in [1 + offset, 2 + offset] {
            f.create(owner, class);
        }
        f.create(9 + offset, c::CLASSIFIER);
        for (owner, feature, m) in [
            (1 + offset, 3 + offset, 4 + offset),
            (2 + offset, 5 + offset, 6 + offset),
        ] {
            f.create(feature, c::FEATURE);
            f.value(
                feature,
                p::ELEMENT_DECLARED_NAME,
                Value::String(
                    if owner == 1 + offset {
                        "original"
                    } else {
                        "renamed"
                    }
                    .into(),
                ),
            );
            if membership == c::END_FEATURE_MEMBERSHIP {
                f.value(feature, p::FEATURE_IS_END, Value::Boolean(true));
            } else {
                f.enumeration(
                    feature,
                    p::FEATURE_DIRECTION,
                    if membership == c::RETURN_PARAMETER_MEMBERSHIP {
                        "out"
                    } else {
                        "in"
                    },
                );
            }
            member(&mut f, owner, feature, m, membership);
        }
        relation(
            &mut f,
            2 + offset,
            1 + offset,
            7 + offset,
            c::SPECIALIZATION,
            p::SPECIALIZATION_GENERAL,
        );
        f.value(
            7 + offset,
            p::SPECIALIZATION_SPECIFIC,
            Value::Reference(id(2 + offset)),
        );
        relation(
            &mut f,
            3 + offset,
            9 + offset,
            8 + offset,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            8 + offset,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(3 + offset)),
        );
    }
    let snapshot = f.finish();
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap(),
    );
    let produced = q
        .plan_result_structure([id(5), id(105), id(205)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(
        produced.production.completeness,
        Completeness::Complete,
        "{:?}",
        produced.production.diagnostics
    );
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&produced.overlay, options, BTreeSet::new()).unwrap(),
    );
    for offset in [0, 100, 200] {
        let redefinitions: Vec<_> = produced
            .overlay
            .model()
            .elements()
            .filter(|r| {
                r.metaclass() == c::REDEFINITION
                    && q.owning_related_element(r.id()).value == Some(id(5 + offset))
            })
            .collect();
        assert_eq!(redefinitions.len(), 1);
        assert!(matches!(redefinitions[0].origin(), Origin::Derived(_)));
        assert_eq!(q.feature_types(id(5 + offset)).value, vec![id(9 + offset)]);
        assert_eq!(
            q.effective_features(id(2 + offset)).value,
            vec![id(5 + offset)]
        );
        assert!(
            q.lookup_member(id(2 + offset), "original", MemberAccess::All)
                .value
                .is_empty()
        );
        assert_eq!(
            q.lookup_member(id(2 + offset), "renamed", MemberAccess::All)
                .value[0]
                .element,
            id(5 + offset)
        );
        assert_eq!(
            snapshot.model().element(id(3 + offset)),
            produced.overlay.model().element(id(3 + offset))
        );
    }
}
