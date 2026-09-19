include!("common/namespace_fixture.rs");

fn result(f: &mut Fixture, owner: u128, membership: u128, feature: u128, name: &str) {
    f.create(feature, c::FEATURE);
    f.value(
        feature,
        p::ELEMENT_DECLARED_NAME,
        Value::String(name.into()),
    );
    f.enumeration(feature, p::FEATURE_DIRECTION, "out");
    f.create(membership, c::RETURN_PARAMETER_MEMBERSHIP);
    f.changes.set(
        id(membership),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(feature))]),
        origin(),
    );
    f.own(owner, membership);
}
fn general(f: &mut Fixture, specific: u128, relationship: u128, parent: u128) {
    f.create(relationship, c::SPECIALIZATION);
    f.value(
        relationship,
        p::SPECIALIZATION_SPECIFIC,
        Value::Reference(id(specific)),
    );
    f.value(
        relationship,
        p::SPECIALIZATION_GENERAL,
        Value::Reference(id(parent)),
    );
    f.own(specific, relationship);
}

#[test]
fn parameter_projection_uses_owned_directed_features_of_every_general_type() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.member(1, 101, 11, c::FEATURE, "mango");
    f.enumeration(11, p::FEATURE_DIRECTION, "in");
    f.create(2, c::STEP);
    f.member(2, 102, 12, c::FEATURE, "tamarind");
    f.enumeration(12, p::FEATURE_DIRECTION, "in");
    general(&mut f, 2, 202, 1);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let answer = q.redefined_features(id(12));
    assert_eq!(answer.completeness, Completeness::Complete);
    assert_eq!(answer.value, vec![id(11)]);
}

#[test]
fn local_result_redefines_canonical_inherited_result_without_copying() {
    let mut f = Fixture::new();
    f.create(1, c::FUNCTION);
    result(&mut f, 1, 101, 11, "fruit");
    f.create(2, c::FUNCTION);
    general(&mut f, 2, 202, 1);
    f.create(3, c::EXPRESSION);
    general(&mut f, 3, 203, 2);
    result(&mut f, 3, 103, 13, "yield");
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let inherited = q.result_parameters(id(2));
    assert_eq!(inherited.completeness, Completeness::Complete);
    assert_eq!(inherited.value, vec![id(11)]);
    let redefined = q.redefined_features(id(13));
    assert_eq!(redefined.completeness, Completeness::Complete);
    assert_eq!(redefined.value, vec![id(11)]);
    let effective = q.effective_features(id(3));
    assert_eq!(effective.completeness, Completeness::Complete);
    assert_eq!(effective.value, vec![id(13)]);
    assert_eq!(q.owning_type(id(11)).value, Some(id(1)));
    assert_eq!(
        q.validate_local_structure(id(3)).completeness,
        Completeness::Complete
    );
    assert!(
        inherited
            .positive_dependencies
            .contains(&FactKey::Property {
                element: id(202),
                property: p::SPECIALIZATION_GENERAL
            })
    );
}

#[test]
fn multiple_inheritance_preserves_distinct_results_until_local_redefinition() {
    for reverse in [false, true] {
        for local in [false, true] {
            let mut f = Fixture::new();
            for n in [1, 2, 3] {
                f.create(n, c::FUNCTION);
            }
            result(&mut f, 1, 101, 11, "one");
            result(&mut f, 2, 102, 12, "two");
            let parents = if reverse { [2, 1] } else { [1, 2] };
            for parent in parents {
                general(&mut f, 3, 200 + parent, parent);
            }
            if local {
                result(&mut f, 3, 103, 13, "both");
            }
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                    .unwrap(),
            );
            let answer = q.result_parameters(id(3));
            assert_eq!(answer.completeness, Completeness::Complete);
            assert_eq!(
                answer.value,
                if local {
                    vec![id(13)]
                } else {
                    parents.map(|p| id(p + 10)).to_vec()
                }
            );
            assert_eq!(
                q.validate_local_structure(id(3)).completeness,
                if local {
                    Completeness::Complete
                } else {
                    Completeness::Invalid
                }
            );
            if local {
                assert_eq!(q.redefined_features(id(13)).value, vec![id(11), id(12)]);
            }
        }
    }
}

#[test]
fn ordered_specializations_follow_ownership_not_creation_or_id_order() {
    let mut f = Fixture::new();
    for n in [1, 2, 3] {
        f.create(n, c::FUNCTION);
    }
    general(&mut f, 3, 201, 2);
    general(&mut f, 3, 200, 1);
    // A non-owned relationship is outside Type::ownedSpecialization.
    f.create(199, c::SPECIALIZATION);
    f.value(199, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(3)));
    f.value(199, p::SPECIALIZATION_GENERAL, Value::Reference(id(3)));
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    assert_eq!(
        q.owned_specialization_targets(id(3)).value,
        vec![id(2), id(1)]
    );
}

#[test]
fn implied_naming_requires_agreement_and_retains_typed_ambiguity() {
    for reverse in [false, true] {
        for same_name in [false, true] {
            let mut f = Fixture::new();
            for n in [1, 2, 3] {
                f.create(n, c::FUNCTION);
            }
            result(&mut f, 1, 101, 11, "mango");
            result(
                &mut f,
                2,
                102,
                12,
                if same_name { "mango" } else { "tamarind" },
            );
            result(&mut f, 3, 103, 13, "unused");
            f.changes.clear(id(13), p::ELEMENT_DECLARED_NAME);
            for parent in if reverse { [2, 1] } else { [1, 2] } {
                general(&mut f, 3, 200 + parent, parent);
            }
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                    .unwrap(),
            );
            let names = q.effective_names(id(13));
            if same_name {
                assert_eq!(names.completeness, Completeness::Complete);
                assert_eq!(
                    names.value,
                    EffectiveNames::Determinate(BTreeSet::from(["mango".into()]))
                );
            } else {
                assert_eq!(names.completeness, Completeness::Incomplete);
                let EffectiveNames::Ambiguous { alternatives, .. } = names.value else {
                    panic!("ambiguity must remain typed")
                };
                assert_eq!(
                    alternatives,
                    BTreeSet::from([
                        BTreeSet::from(["mango".into()]),
                        BTreeSet::from(["tamarind".into()])
                    ])
                );
            }
            assert!(
                !names
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "KQ_IMPLIED_NAMING_ORDER")
            );
            for owner in [1, 2, 3] {
                assert!(names.positive_dependencies.contains(&FactKey::Property {
                    element: id(owner),
                    property: p::ELEMENT_OWNED_RELATIONSHIP
                }));
            }
        }
    }
}

#[test]
fn feature_chain_terminal_type_supplies_positional_end_population() {
    let mut f = Fixture::new();
    f.create(1, c::TYPE);
    for (m, e, name) in [(180, 80, "first"), (140, 40, "second")] {
        f.member(1, m, e, c::FEATURE, name);
        f.value(e, p::FEATURE_IS_END, Value::Boolean(true));
    }
    f.create(2, c::FEATURE);
    f.create(3, c::FEATURE);
    f.create(4, c::FEATURE);
    // A FeatureChaining target must itself be a Feature. It inherits the two
    // ends from its typing, through the terminal feature of the chain.
    general(&mut f, 4, 204, 1);
    for (r, target) in [(201, 2), (202, 4)] {
        f.create(r, c::FEATURE_CHAINING);
        f.value(
            r,
            p::FEATURE_CHAINING_CHAINING_FEATURE,
            Value::Reference(id(target)),
        );
        f.own(3, r);
    }
    f.create(5, c::CONNECTOR);
    general(&mut f, 5, 205, 3);
    for (m, e) in [(106, 6), (107, 7)] {
        f.member(5, m, e, c::FEATURE, "replacement");
        f.value(e, p::FEATURE_IS_END, Value::Boolean(true));
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    for (source, target) in [(6, 80), (7, 40)] {
        let answer = q.redefined_features(id(source));
        assert_eq!(answer.completeness, Completeness::Complete);
        assert_eq!(answer.value, vec![id(target)]);
        assert!(answer.positive_dependencies.contains(&FactKey::Property {
            element: id(202),
            property: p::FEATURE_CHAINING_CHAINING_FEATURE
        }));
    }
}
