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
fn other_owned_features_are_read_only_when_they_can_suppress_an_inherited_result() {
    for local_result in [false, true] {
        let mut f = Fixture::new();
        f.create(1, c::FUNCTION);
        result(&mut f, 1, 101, 11, "inherited");
        f.create(2, c::EXPRESSION);
        general(&mut f, 2, 201, 1);
        if local_result {
            result(&mut f, 2, 102, 12, "replacement");
        }
        f.member(2, 103, 13, c::FEATURE, "other");
        f.create(203, c::REDEFINITION);
        f.value(
            203,
            p::REDEFINITION_REDEFINING_FEATURE,
            Value::Reference(id(13)),
        );
        f.value(
            203,
            p::REDEFINITION_REDEFINED_FEATURE,
            Value::Reference(id(11)),
        );
        f.own(13, 203);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        let answer = q.result_parameters(id(2));
        assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
        assert_eq!(
            answer.value,
            if local_result { vec![id(12)] } else { vec![] }
        );
        assert_eq!(
            answer
                .search_dependencies
                .contains(&SearchDependency::OwnedRelationships {
                    owner: id(2),
                    class: c::FEATURE_MEMBERSHIP,
                }),
            !local_result,
        );
    }
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

fn cycle_queries(snapshot: &Snapshot, pending: BTreeSet<ElementId>) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_project_snapshot(
            snapshot,
            Default::default(),
            BTreeSet::new(),
            pending.clone(),
            pending,
        )
        .unwrap()
        .with_semantic_extension_identity("fixture-result-cycles/1", [31; 32])
        .unwrap(),
    )
}

#[test]
fn empty_return_cycle_retains_complete_negative_population_evidence() {
    let mut f = Fixture::new();
    for owner in [1, 2, 3] {
        f.create(owner, c::FUNCTION);
    }
    general(&mut f, 1, 101, 2);
    general(&mut f, 2, 102, 1);
    general(&mut f, 1, 103, 3);
    let snapshot = f.finish();
    let answer = cycle_queries(&snapshot, BTreeSet::new()).result_parameters(id(1));
    assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
    assert!(answer.value.is_empty());
    for owner in [1, 2, 3] {
        assert!(
            answer
                .search_dependencies
                .contains(&SearchDependency::OwnedRelationships {
                    owner: id(owner),
                    class: c::RETURN_PARAMETER_MEMBERSHIP,
                })
        );
        assert!(
            answer
                .search_dependencies
                .contains(&SearchDependency::OwnedRelationships {
                    owner: id(owner),
                    class: c::SPECIALIZATION,
                })
        );
    }
    for member in [1, 2] {
        let conclusion = Conclusion {
            query: QueryKind::ResultPopulation,
            subject: id(member),
            value: id(member),
        };
        let proof = answer.explanations[&conclusion]
            .iter()
            .find(|proof| proof.rule == Rule::InheritedResultFixedPoint)
            .unwrap();
        for owner in [1, 2, 3] {
            assert!(proof.premises.contains(&Evidence::Search(
                SearchDependency::OwnedRelationships {
                    owner: id(owner),
                    class: c::RETURN_PARAMETER_MEMBERSHIP,
                }
            )));
            assert!(proof.premises.contains(&Evidence::Search(
                SearchDependency::OwnedRelationships {
                    owner: id(owner),
                    class: c::SPECIALIZATION,
                }
            )));
        }
    }
    let pending = cycle_queries(&snapshot, BTreeSet::from([id(3)])).result_parameters(id(1));
    assert_eq!(pending.completeness, Completeness::Incomplete);
    assert!(
        !pending
            .explanations
            .values()
            .flatten()
            .any(|proof| proof.rule == Rule::InheritedResultFixedPoint)
    );
    let historical = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    assert_eq!(
        historical.result_parameters(id(1)).completeness,
        Completeness::Incomplete
    );
}

#[test]
fn nonempty_return_cycles_preserve_owned_redefinition_and_inherited_identity() {
    for second_result in [false, true] {
        let mut f = Fixture::new();
        for owner in [1, 2, 3] {
            f.create(owner, c::FUNCTION);
        }
        general(&mut f, 1, 201, 2);
        general(&mut f, 2, 202, 1);
        general(&mut f, 3, 203, 2);
        result(&mut f, 1, 101, 11, "apple");
        if second_result {
            result(&mut f, 2, 102, 12, "pear");
        }
        let snapshot = f.finish();
        let q = cycle_queries(&snapshot, BTreeSet::new());
        for (owner, feature) in [
            (1, 11),
            (2, if second_result { 12 } else { 11 }),
            (3, if second_result { 12 } else { 11 }),
        ] {
            let answer = q.result_parameters(id(owner));
            assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
            assert_eq!(answer.value, vec![id(feature)]);
        }
        assert_eq!(q.owning_type(id(11)).value, Some(id(1)));
    }
}

#[test]
fn cyclic_return_candidates_apply_inherited_transitive_redefinition_before_ordering() {
    let mut f = Fixture::new();
    for owner in [1, 2, 3, 4, 5] {
        f.create(owner, c::TYPE);
    }
    general(&mut f, 1, 201, 2);
    general(&mut f, 2, 202, 1);
    general(&mut f, 1, 203, 3);
    general(&mut f, 2, 204, 5);
    for (owner, membership, feature) in [(3, 103, 31), (4, 104, 41), (5, 105, 51)] {
        result(&mut f, owner, membership, feature, "result");
    }
    for (specific, general, relationship) in [(51, 41, 251), (41, 31, 241)] {
        f.create(relationship, c::REDEFINITION);
        f.value(
            relationship,
            p::REDEFINITION_REDEFINING_FEATURE,
            Value::Reference(id(specific)),
        );
        f.value(
            relationship,
            p::REDEFINITION_REDEFINED_FEATURE,
            Value::Reference(id(general)),
        );
        f.own(specific, relationship);
    }
    let snapshot = f.finish();
    let q = cycle_queries(&snapshot, BTreeSet::new());
    for owner in [1, 2] {
        let answer = q.result_parameters(id(owner));
        assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
        assert_eq!(answer.value, vec![id(51)]);
        assert!(
            answer
                .positive_dependencies
                .contains(&FactKey::Element(id(241)))
        );
    }
    assert_eq!(q.owning_type(id(51)).value, Some(id(5)));
}

#[test]
fn cyclic_return_order_without_a_stable_semantic_vector_remains_incomplete() {
    let mut f = Fixture::new();
    for owner in [1, 2, 3, 4] {
        f.create(owner, c::TYPE);
    }
    general(&mut f, 1, 201, 2);
    general(&mut f, 2, 202, 1);
    general(&mut f, 1, 203, 3);
    general(&mut f, 2, 204, 4);
    result(&mut f, 3, 103, 31, "first");
    result(&mut f, 4, 104, 41, "second");
    let snapshot = f.finish();
    let answer = cycle_queries(&snapshot, BTreeSet::new()).result_parameters(id(1));
    assert_eq!(answer.completeness, Completeness::Incomplete);
    assert!(
        answer
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_RESULT_INHERITANCE_CYCLE")
    );
}
