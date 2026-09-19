include!("common/namespace_fixture.rs");

#[test]
fn singleton_implied_result_redefinition_supplies_the_inherited_result_name() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 102, 2, c::FUNCTION, "General");
    f.member(1, 103, 3, c::FUNCTION, "Specific");
    for (owner, membership, feature) in [(2, 104, 4), (3, 105, 5)] {
        f.create(feature, c::FEATURE);
        f.enumeration(feature, p::FEATURE_DIRECTION, "out");
        f.create(membership, c::RETURN_PARAMETER_MEMBERSHIP);
        f.own(owner, membership);
        f.changes.set(
            id(membership),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(feature))]),
            origin(),
        );
    }
    f.value(4, p::ELEMENT_DECLARED_NAME, Value::String("result".into()));
    specialize(&mut f, 200, 3, 2);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let lookup = q.lookup_member(id(3), "result", MemberAccess::All);
    assert_eq!(
        lookup.value.iter().map(|m| m.element).collect::<Vec<_>>(),
        vec![id(5)]
    );
    assert_eq!(lookup.completeness, Completeness::Complete);
    assert!(
        lookup
            .explanations
            .values()
            .flatten()
            .any(|proof| proof.rule == Rule::ResultRedefinition)
    );
}

fn specialize(f: &mut Fixture, relationship: u128, specific: u128, general: u128) {
    f.create(relationship, c::SPECIALIZATION);
    f.value(
        relationship,
        p::SPECIALIZATION_SPECIFIC,
        Value::Reference(id(specific)),
    );
    f.value(
        relationship,
        p::SPECIALIZATION_GENERAL,
        Value::Reference(id(general)),
    );
    f.own(specific, relationship);
}

#[test]
fn adversarial_redefinition_matrix() {
    // Each row asserts candidates, completeness, ambiguity, profile, positive
    // evidence, negative dependencies and selected operational path.
    let cases = [
        "lexical",
        "inherited",
        "second-general",
        "diamond",
        "two-generals",
        "lexical-and-general",
        "private",
        "protected",
        "import",
        "alias",
        "root-qualified",
        "relative-qualified",
        "self",
        "unrelated",
        "zero",
        "multiple",
        "nested-levels",
        "long-graph",
        "cyclic-imports",
        "prefix-shadow",
        "renamed-suppression",
    ];
    for case in cases {
        for reverse in [false, true] {
            let profile = agq_kerml::BaselineProfile::OPERATIONAL;
            let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
            let changes = base.change_set();
            let mut f = Fixture {
                base,
                changes,
                owned: BTreeMap::new(),
            };
            f.create(100, c::NAMESPACE);
            f.member(100, 101, 1, c::CLASS, "Cobalt");
            f.member(100, 102, 3, c::CLASS, "Quartz");
            f.member(1, 105, 5, c::FEATURE, "interval");
            f.member(5, 106, 6, c::FEATURE, "signal");
            f.create(202, c::REDEFINITION);
            f.value(
                202,
                p::REDEFINITION_REDEFINING_FEATURE,
                Value::Reference(id(6)),
            );
            f.own(6, 202);
            specialize(&mut f, 200, 5, 3);
            let mut name = QualifiedName {
                absolute: false,
                segments: vec!["signal".into()],
            };
            let (expected, lexical): (Vec<u128>, bool) = match case {
                "lexical" | "root-qualified" | "relative-qualified" | "lexical-and-general" => {
                    f.member(1, 103, 2, c::FEATURE, "signal");
                    if case == "lexical-and-general" {
                        f.member(3, 104, 4, c::FEATURE, "signal");
                        (vec![4], false)
                    } else {
                        if case.ends_with("qualified") {
                            name.segments = vec!["Cobalt".into(), "signal".into()];
                            name.absolute = case == "root-qualified";
                        }
                        (vec![2], !name.absolute)
                    }
                }
                "inherited" | "private" | "protected" => {
                    f.member(3, 104, 4, c::FEATURE, "signal");
                    if case != "inherited" {
                        f.visibility(104, case);
                    }
                    if case == "private" {
                        (vec![], true)
                    } else {
                        (vec![4], false)
                    }
                }
                "second-general" | "two-generals" | "diamond" | "renamed-suppression" => {
                    f.member(100, 107, 7, c::CLASS, "Jasper");
                    specialize(&mut f, 201, 5, 7);
                    if case == "diamond" {
                        f.member(100, 109, 9, c::CLASS, "Base");
                        f.member(9, 110, 10, c::FEATURE, "signal");
                        specialize(&mut f, 203, 3, 9);
                        specialize(&mut f, 204, 7, 9);
                        (vec![10], false)
                    } else {
                        f.member(7, 108, 8, c::FEATURE, "signal");
                        if case == "two-generals" {
                            f.member(3, 104, 4, c::FEATURE, "signal");
                            (vec![4, 8], false)
                        } else if case == "renamed-suppression" {
                            f.member(3, 104, 4, c::FEATURE, "signal");
                            f.value(8, p::ELEMENT_DECLARED_NAME, Value::String("renamed".into()));
                            f.create(205, c::REDEFINITION);
                            f.value(
                                205,
                                p::REDEFINITION_REDEFINING_FEATURE,
                                Value::Reference(id(8)),
                            );
                            f.value(
                                205,
                                p::REDEFINITION_REDEFINED_FEATURE,
                                Value::Reference(id(4)),
                            );
                            f.own(8, 205);
                            (vec![4], false)
                        } else {
                            (vec![8], false)
                        }
                    }
                }
                "import" | "cyclic-imports" | "alias" => {
                    f.member(100, 130, 30, c::NAMESPACE, "Imported");
                    f.member(30, 104, 4, c::FEATURE, "signal");
                    if case == "alias" {
                        f.create(109, c::MEMBERSHIP);
                        f.value(109, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(4)));
                        f.value(
                            109,
                            p::MEMBERSHIP_MEMBER_NAME,
                            Value::String("alias".into()),
                        );
                        f.own(1, 109);
                        name.segments = vec!["alias".into()];
                    } else {
                        f.import(1, 210, 30, false);
                        if case == "cyclic-imports" {
                            f.import(30, 211, 1, false);
                        }
                    }
                    (vec![4], true)
                }
                "unrelated" => {
                    f.member(3, 140, 40, c::FEATURE, "other");
                    f.member(40, 108, 8, c::FEATURE, "signal");
                    // An unrelated local peer in the specific type is also not inherited.
                    f.member(5, 109, 9, c::FEATURE, "signal");
                    (vec![], true)
                }
                "multiple" => {
                    f.member(1, 103, 2, c::FEATURE, "signal");
                    f.member(1, 108, 8, c::FEATURE, "signal");
                    (vec![2, 8], true)
                }
                "nested-levels" => {
                    f.member(1, 103, 2, c::FEATURE, "signal");
                    f.member(5, 107, 7, c::FEATURE, "deep");
                    f.owned
                        .get_mut(&id(5))
                        .unwrap()
                        .retain(|v| *v != Value::Reference(id(106)));
                    f.own(7, 106);
                    (vec![2], true)
                }
                "long-graph" => {
                    for n in 1000..1080 {
                        f.member(100, n + 1000, n, c::CLASS, &format!("G{n}"));
                        specialize(&mut f, n + 2000, if n == 1000 { 3 } else { n - 1 }, n);
                    }
                    f.member(1079, 104, 4, c::FEATURE, "signal");
                    (vec![4], false)
                }
                "prefix-shadow" => {
                    f.member(3, 107, 7, c::FEATURE, "prefix");
                    f.member(1, 108, 8, c::FEATURE, "prefix");
                    f.member(8, 109, 9, c::FEATURE, "signal");
                    name.segments = vec!["prefix".into(), "signal".into()];
                    (vec![], false)
                }
                "self" | "zero" => (vec![], true),
                _ => unreachable!(),
            };
            if reverse {
                for values in f.owned.values_mut() {
                    values.reverse();
                }
            }
            let candidate = f.construction();
            let q = KerMlQueries::new(
                SemanticContext::for_construction(
                    &candidate,
                    SemanticOptions {
                        baseline_profile: profile,
                        ..Default::default()
                    },
                    Default::default(),
                )
                .unwrap(),
            );
            let result =
                q.lookup_relationship_target(id(202), p::REDEFINITION_REDEFINED_FEATURE, &name);
            let actual: BTreeSet<_> = result.value.iter().map(|m| m.element).collect();
            assert_eq!(
                actual,
                expected.iter().copied().map(id).collect(),
                "{case} reverse={reverse}: {:?}",
                result.diagnostics
            );
            assert_eq!(
                result.completeness,
                Completeness::Complete,
                "{case}: {:?}",
                result.diagnostics
            );
            assert_eq!(
                result.value.len() > 1,
                expected.len() > 1,
                "{case}: ambiguity"
            );
            assert_eq!(result.context.baseline_profile_id, profile.id());
            assert!(
                result
                    .positive_dependencies
                    .contains(&FactKey::Element(id(202)))
            );
            assert!(
                result
                    .search_dependencies
                    .iter()
                    .any(|s| matches!(s, SearchDependency::NamespaceMembers { .. }))
            );
            assert_eq!(
                result.search_dependencies.iter().any(|s| matches!(
                    s,
                    SearchDependency::RedefinitionScope {
                        path: RedefinitionRulePath::LexicalContaining,
                        ..
                    }
                )),
                lexical,
                "{case}"
            );
            for target in &actual {
                assert!(
                    result.explanations[&Conclusion {
                        query: QueryKind::ResolveReference,
                        subject: id(202),
                        value: *target
                    }]
                        .iter()
                        .any(|e| e.rule == Rule::OperationalRedefinitionTargetV1)
                );
            }
            println!(
                "MATRIX {}",
                serde_json::json!({
                    "case":case,"reverse":reverse,"profile":profile.id(),
                    "expected_candidates":expected.iter().copied().map(id).map(|i|i.to_string()).collect::<Vec<_>>(),
                    "candidates":actual.iter().map(|i|i.to_string()).collect::<Vec<_>>(),
                    "completeness":format!("{:?}",result.completeness),"ambiguous":result.value.len()>1,
                    "lexical_expected":lexical,
                    "positive_dependencies":result.positive_dependencies.iter().map(|v|format!("{v:?}")).collect::<Vec<_>>(),
                    "search_dependencies":result.search_dependencies.iter().map(|v|format!("{v:?}")).collect::<Vec<_>>(),
                    "rule":"AGQ-KERML10-002 / agentique-kerml10-redefinition-target/1"
                })
            );
        }
    }
}

#[test]
fn ordinary_lookup_still_includes_declared_feature() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 2, 3, c::FEATURE, "signal");
    let s = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&s, Default::default(), Default::default()).unwrap(),
    );
    assert_eq!(
        q.lookup_member(id(1), "signal", MemberAccess::All).value[0].element,
        id(3)
    );
}

#[test]
fn effective_imported_memberships_keep_identity_and_do_not_become_owned_features() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 102, 3, c::TYPE, "General");
    f.member(1, 105, 5, c::TYPE, "Specific");
    f.member(1, 130, 30, c::TYPE, "Imported");
    f.member(30, 104, 4, c::FEATURE, "signal");
    f.import(3, 210, 30, false);
    f.import(30, 211, 3, false);
    specialize(&mut f, 200, 5, 3);
    let s = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&s, Default::default(), Default::default()).unwrap(),
    );
    assert!(q.effective_features(id(3)).value.is_empty());
    let result = q.effective_features(id(5));
    assert_eq!(result.value, vec![id(4)]);
    assert_eq!(result.completeness, Completeness::Complete);
    assert!(
        result
            .search_dependencies
            .contains(&SearchDependency::ImportSet { namespace: id(3) })
    );
    assert_eq!(q.owner(id(4)).value, Some(id(30)));
}

#[test]
fn conjugation_and_feature_chain_supply_original_member_identities() {
    for conjugation in [false, true] {
        let mut f = Fixture::new();
        f.create(1, c::NAMESPACE);
        f.member(1, 103, 3, c::FEATURE, "original");
        f.member(3, 104, 4, c::FEATURE, "signal");
        f.member(1, 105, 5, c::FEATURE, "specific");
        if conjugation {
            f.create(200, c::CONJUGATION);
            f.value(200, p::CONJUGATION_CONJUGATED_TYPE, Value::Reference(id(5)));
            f.value(200, p::CONJUGATION_ORIGINAL_TYPE, Value::Reference(id(3)));
        } else {
            f.create(200, c::FEATURE_CHAINING);
            f.value(
                200,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                Value::Reference(id(3)),
            );
        }
        f.own(5, 200);
        let s = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&s, Default::default(), Default::default()).unwrap(),
        );
        let result = q.effective_features(id(5));
        assert_eq!(result.value, vec![id(4)]);
        assert_eq!(result.completeness, Completeness::Complete);
        assert_eq!(q.owner(id(4)).value, Some(id(3)));
    }
}

#[test]
fn positional_end_redefinition_uses_inherited_end_order_and_keeps_the_other_end() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 102, 2, c::TYPE, "Base");
    f.member(1, 103, 3, c::TYPE, "Intermediate");
    f.member(1, 104, 4, c::TYPE, "Specific");
    // Source order intentionally differs from canonical identity order.
    for (membership, target, name) in [(190, 90, "first"), (140, 40, "second"), (180, 80, "third")]
    {
        f.member(2, membership, target, c::FEATURE, name);
        f.value(target, p::FEATURE_IS_END, Value::Boolean(true));
    }
    for (membership, target, name) in [(106, 6, "replacementFirst"), (107, 7, "replacementSecond")]
    {
        f.member(4, membership, target, c::FEATURE, name);
        f.value(target, p::FEATURE_IS_END, Value::Boolean(true));
    }
    specialize(&mut f, 200, 3, 2);
    specialize(&mut f, 201, 4, 3);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    for (source, target) in [(6, 90), (7, 40)] {
        let result = q.redefined_features(id(source));
        assert_eq!(result.value, vec![id(target)]);
        assert_eq!(result.completeness, Completeness::Complete);
        assert!(
            result.explanations[&Conclusion {
                query: QueryKind::RedefinedFeatures,
                subject: id(source),
                value: id(target)
            }]
                .iter()
                .any(|proof| proof.rule == Rule::EndRedefinition)
        );
    }
    let result = q.effective_features(id(4));
    assert_eq!(result.value, vec![id(6), id(7), id(80)]);
    assert_eq!(result.completeness, Completeness::Complete);
    assert_eq!(q.owner(id(80)).value, Some(id(2)));
    assert_eq!(
        snapshot
            .model()
            .instances(c::REDEFINITION, true)
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn ordinary_inheritance_retains_the_published_membership_based_suppression() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 103, 3, c::TYPE, "General");
    f.member(1, 105, 5, c::TYPE, "Specific");
    f.member(3, 104, 4, c::FEATURE, "signal");
    f.create(110, c::MEMBERSHIP);
    f.value(110, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(4)));
    f.value(
        110,
        p::MEMBERSHIP_MEMBER_NAME,
        Value::String("alias".into()),
    );
    f.own(3, 110);
    specialize(&mut f, 200, 5, 3);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    // removeRedefinedFeatures compares Membership identities, and its closure
    // includes the memberElement itself. The semantic erratum does not change it.
    assert_eq!(q.effective_features(id(3)).value, vec![id(4)]);
    let result = q.effective_features(id(5));
    assert!(result.value.is_empty());
    assert_eq!(result.completeness, Completeness::Complete);
    assert!(
        q.namespace_members(id(5), MemberAccess::All)
            .value
            .is_empty()
    );
}

#[test]
fn unnamed_feature_uses_only_the_first_owned_redefinition_for_its_name() {
    for reverse in [false, true] {
        let mut f = Fixture::new();
        f.create(1, c::NAMESPACE);
        f.member(1, 103, 3, c::TYPE, "General");
        f.member(1, 105, 5, c::TYPE, "Specific");
        f.member(3, 104, 4, c::FEATURE, "alpha");
        f.member(3, 108, 8, c::FEATURE, "beta");
        f.member(5, 106, 6, c::FEATURE, "temporary");
        f.changes.clear(id(6), p::ELEMENT_DECLARED_NAME);
        for (relationship, target) in if reverse {
            [(201, 8), (200, 4)]
        } else {
            [(200, 4), (201, 8)]
        } {
            f.create(relationship, c::REDEFINITION);
            f.value(
                relationship,
                p::REDEFINITION_REDEFINING_FEATURE,
                Value::Reference(id(6)),
            );
            f.value(
                relationship,
                p::REDEFINITION_REDEFINED_FEATURE,
                Value::Reference(id(target)),
            );
            f.own(6, relationship);
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        let (first, second) = if reverse {
            ("beta", "alpha")
        } else {
            ("alpha", "beta")
        };
        let result = q.lookup_member(id(5), first, MemberAccess::All);
        assert_eq!(
            result.value.iter().map(|m| m.element).collect::<Vec<_>>(),
            vec![id(6)]
        );
        assert_eq!(result.completeness, Completeness::Complete);
        assert!(
            q.lookup_member(id(5), second, MemberAccess::All)
                .value
                .is_empty()
        );
    }
}

#[test]
fn incomplete_general_search_and_wrong_kind_prefix_never_trigger_lexical_redefinition_search() {
    for incomplete in [false, true] {
        let profile = agq_kerml::BaselineProfile::OPERATIONAL_V2;
        let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
        let changes = base.change_set();
        let mut f = Fixture {
            base,
            changes,
            owned: BTreeMap::new(),
        };
        f.create(1, c::NAMESPACE);
        f.member(1, 102, 2, c::CLASS, "Outer");
        f.member(2, 104, 4, c::FEATURE, "signal");
        f.member(2, 105, 5, c::FEATURE, "specific");
        f.member(5, 106, 6, c::FEATURE, "replacement");
        f.member(1, 103, 3, c::CLASS, "General");
        f.create(200, c::SPECIALIZATION);
        f.value(200, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(5)));
        f.own(5, 200);
        if !incomplete {
            f.value(200, p::SPECIALIZATION_GENERAL, Value::Reference(id(3)));
            f.member(3, 108, 8, c::CLASS, "signal");
        }
        f.create(201, c::REDEFINITION);
        f.value(
            201,
            p::REDEFINITION_REDEFINING_FEATURE,
            Value::Reference(id(6)),
        );
        f.own(6, 201);
        let candidate = f.construction();
        let q = KerMlQueries::new(
            SemanticContext::for_construction(
                &candidate,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        let result = q.lookup_relationship_target(
            id(201),
            p::REDEFINITION_REDEFINED_FEATURE,
            &QualifiedName {
                absolute: false,
                segments: vec!["signal".into()],
            },
        );
        assert_eq!(
            result.completeness,
            if incomplete {
                Completeness::Incomplete
            } else {
                Completeness::Invalid
            }
        );
        assert!(result.value.iter().all(|m| m.element != id(4)));
        assert!(!result.search_dependencies.iter().any(|s| matches!(
            s,
            SearchDependency::RedefinitionScope {
                path: RedefinitionRulePath::LexicalContaining,
                ..
            }
        )));
    }
}

#[test]
fn cross_subsetting_derives_the_source_from_ownership_and_does_not_make_the_target_a_source() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 103, 3, c::FEATURE, "crossing");
    f.create(200, c::CROSS_SUBSETTING);
    f.own(3, 200);
    f.create(4, c::FEATURE);
    f.value(
        200,
        p::CROSS_SUBSETTING_CROSSED_FEATURE,
        Value::Reference(id(4)),
    );
    f.changes.set(
        id(200),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(4))]),
        origin(),
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    for result in [q.subsetted_features(id(3)), q.direct_specializations(id(3))] {
        assert_eq!(result.value, vec![id(4)]);
        assert_eq!(result.completeness, Completeness::Complete);
        assert!(result.positive_dependencies.contains(&FactKey::Property {
            element: id(3),
            property: p::ELEMENT_OWNED_RELATIONSHIP
        }));
    }
    for result in [q.subsetted_features(id(4)), q.direct_specializations(id(4))] {
        assert!(result.value.is_empty());
        assert_eq!(result.completeness, Completeness::Complete);
    }
}
