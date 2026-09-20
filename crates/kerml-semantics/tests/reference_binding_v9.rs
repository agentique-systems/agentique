//! Independent arbitrary-name raw reference and contextual result graphs.
include!("common/result_fixture.rs");

fn reference_builder(profile: agq_kerml::BaselineProfile) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (n, class, name) in [
        (1, c::FUNCTION, "grove"),
        (2, c::FEATURE_REFERENCE_EXPRESSION, "bud"),
        (3, c::FEATURE, "sap"),
        (4, c::FEATURE, "leaf"),
        (5, c::FEATURE, "fruit"),
    ] {
        f.create(n, class);
        f.value(n, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
    }
    member(&mut f, 1, 2, 101, c::RESULT_EXPRESSION_MEMBERSHIP);
    member(&mut f, 2, 3, 102, c::RETURN_PARAMETER_MEMBERSHIP);
    member(&mut f, 1, 4, 103, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 5, 104, c::RETURN_PARAMETER_MEMBERSHIP);
    f.enumeration(3, p::FEATURE_DIRECTION, "out");
    f.enumeration(5, p::FEATURE_DIRECTION, "out");
    relation(
        &mut f,
        2,
        4,
        105,
        c::MEMBERSHIP,
        p::MEMBERSHIP_MEMBER_ELEMENT,
    );
    type_featuring(&mut f, 2, 1, 106);
    f
}
fn reference_fixture(profile: agq_kerml::BaselineProfile) -> Snapshot {
    reference_builder(profile).finish()
}

fn queries(snapshot: &Snapshot, profile: agq_kerml::BaselineProfile) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(
            snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    )
}

#[test]
fn seven_profiles_separate_inner_raw_identity_from_outer_contextual_identity() {
    use agq_kerml::BaselineProfile as P;
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
    ] {
        let snapshot = reference_fixture(profile);
        let q = queries(&snapshot, profile);
        let expanded = q.derive_result_structure(&snapshot).unwrap();
        let other_profile = if profile == P::OPERATIONAL_V6 {
            P::OPERATIONAL_V5
        } else {
            P::OPERATIONAL_V6
        };
        assert!(matches!(
            SemanticContext::for_overlay(
                &expanded.overlay,
                SemanticOptions {
                    baseline_profile: other_profile,
                    ..Default::default()
                },
                Default::default()
            ),
            Err(ContextError::CorrectionProfileMismatch(_))
        ));
        assert_eq!(
            expanded.production.completeness,
            Completeness::Complete,
            "{:?}",
            expanded.production.diagnostics
        );
        let view = KerMlQueries::new(
            SemanticContext::for_overlay(
                &expanded.overlay,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        let mut roles = BTreeMap::new();
        for &connector in &expanded.production.value {
            roles.insert(view.implied_binding_role(connector).unwrap(), connector);
        }
        let inner =
            view.validate_connector_featuring(roles[&ImpliedBindingRole::FeatureReferenceResult]);
        assert_eq!(inner.value.endpoints, vec![id(4), id(3)]);
        assert_eq!(
            inner.value.valid,
            profile == P::OPERATIONAL_V6,
            "{profile:?}: {inner:?}"
        );
        if profile == P::OPERATIONAL_V6 {
            assert_eq!(inner.completeness, Completeness::Complete, "{inner:?}");
            assert_eq!(inner.value.domains, vec![id(1)]);
            assert_eq!(inner.value.ordinary_valid, vec![true, false]);
            assert_eq!(inner.value.exception_applied, vec![false, true]);
            let outer =
                view.validate_connector_featuring(roles[&ImpliedBindingRole::FunctionResult]);
            assert_eq!(outer.completeness, Completeness::Complete, "{outer:?}");
            assert!(outer.value.valid);
            assert_eq!(outer.value.exception_applied, vec![false, false]);
            let chain = &expanded.contextual_results[0];
            assert_ne!(chain.feature, chain.raw_result);
            assert_eq!(
                view.chaining_features(chain.feature).value,
                vec![id(2), id(3)]
            );
            assert_eq!(view.featuring_types(chain.feature).value, vec![id(1)]);
            assert!(view.all_supertypes(chain.feature).value.contains(&id(3)));
        }
        // Neither graph production nor an overlay changes any declared bytes.
        assert_eq!(
            snapshot.model().len(),
            expanded.overlay.declared().model().len()
        );
        for record in snapshot.model().elements() {
            assert_eq!(
                expanded.overlay.declared().model().element(record.id()),
                Some(record)
            );
        }
    }
}

#[test]
fn reference_role_adversarial_domains_and_ownership() {
    use agq_kerml::BaselineProfile as P;
    let mut fingerprints = BTreeSet::new();
    for case in [
        "outer",
        "inherited",
        "wrong-result-owner",
        "variable",
        "shared",
        "no-common",
        "missing-result",
        "missing-referent",
        "nested",
        "inside-chain",
    ] {
        let mut f = reference_builder(P::OPERATIONAL_V6);
        match case {
            "inherited" => {
                f.create(9, c::FUNCTION);
                f.owned
                    .get_mut(&id(1))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(103)));
                f.own(9, 103);
                relation(
                    &mut f,
                    1,
                    9,
                    109,
                    c::SUBCLASSIFICATION,
                    p::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
                f.value(
                    109,
                    p::SUBCLASSIFICATION_SUBCLASSIFIER,
                    Value::Reference(id(1)),
                );
            }
            "wrong-result-owner" => {
                f.create(9, c::EXPRESSION);
                f.owned
                    .get_mut(&id(2))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(102)));
                f.own(9, 102);
                subset(&mut f, 2, 9, 109);
            }
            "variable" => {
                f.value(3, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
                f.create(9, c::FEATURE);
                member(&mut f, 2, 9, 109, c::FEATURE_MEMBERSHIP);
                type_featuring(&mut f, 3, 9, 110);
            }
            "shared" => {
                f.owned
                    .get_mut(&id(1))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(103)));
                f.own(2, 103);
            }
            "no-common" => {
                f.create(9, c::FUNCTION);
                f.owned
                    .get_mut(&id(1))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(103)));
                f.own(9, 103);
            }
            "missing-result" => {
                f.owned
                    .get_mut(&id(2))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(102)));
            }
            "missing-referent" => {
                f.owned
                    .get_mut(&id(2))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(105)));
            }
            "nested" | "inside-chain" => {
                f.create(
                    9,
                    if case == "nested" {
                        c::FEATURE_REFERENCE_EXPRESSION
                    } else {
                        c::FEATURE_CHAIN_EXPRESSION
                    },
                );
                f.create(10, c::FEATURE);
                f.enumeration(10, p::FEATURE_DIRECTION, "out");
                member(&mut f, 9, 10, 109, c::RETURN_PARAMETER_MEMBERSHIP);
                member(&mut f, 1, 9, 110, c::FEATURE_MEMBERSHIP);
                relation(
                    &mut f,
                    9,
                    4,
                    111,
                    c::MEMBERSHIP,
                    p::MEMBERSHIP_MEMBER_ELEMENT,
                );
                f.owned
                    .get_mut(&id(1))
                    .unwrap()
                    .retain(|v| *v != Value::Reference(id(101)));
                f.own(9, 101);
                f.value(
                    106,
                    p::TYPE_FEATURING_FEATURING_TYPE,
                    Value::Reference(id(9)),
                );
                if case == "nested" {
                    redefine(&mut f, 3, 10, 112);
                }
            }
            _ => {}
        }
        let snapshot = f.finish();
        let q = queries(&snapshot, P::OPERATIONAL_V6);
        let expanded = q.derive_result_structure(&snapshot).unwrap();
        let view = KerMlQueries::new(
            SemanticContext::for_overlay(
                &expanded.overlay,
                SemanticOptions {
                    baseline_profile: P::OPERATIONAL_V6,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        let inner = expanded.production.value.iter().copied().find(|b| {
            view.implied_binding_role(*b) == Some(ImpliedBindingRole::FeatureReferenceResult)
                && view.owner(*b).value == Some(id(2))
        });
        if ["no-common", "missing-result", "missing-referent"].contains(&case) {
            assert!(inner.is_none(), "{case}");
            assert_ne!(
                expanded.production.completeness,
                Completeness::Complete,
                "{case}"
            );
            continue;
        }
        let check = view.validate_connector_featuring(
            inner.unwrap_or_else(|| panic!("{case}: {:?}", expanded.production.diagnostics)),
        );
        assert_eq!(
            check.value.valid,
            case != "wrong-result-owner",
            "{case}: {check:?}"
        );
        if case == "shared" {
            assert_eq!(check.value.exception_applied, vec![false, false]);
        }
        if case == "outer" {
            let repeated = q.derive_result_structure(&snapshot).unwrap();
            assert_eq!(expanded.production.value, repeated.production.value);
            assert_eq!(expanded.contextual_results, repeated.contextual_results);
        }
        fingerprints.insert(format!("{case}:{:?}", check.value));
    }
    assert_eq!(fingerprints.len(), 7);
}

#[test]
fn authored_and_unclassified_implied_bindings_keep_strict_domains_and_binary_shape() {
    use agq_kerml::BaselineProfile as P;
    for implied in [false, true] {
        for extra in [false, true] {
            let mut f = reference_builder(P::OPERATIONAL_V6);
            binding(&mut f, 2, 200, c::OWNING_MEMBERSHIP, 4, 3);
            type_featuring(&mut f, 200, 1, 220);
            f.value(200, p::RELATIONSHIP_IS_IMPLIED, Value::Boolean(implied));
            if extra {
                f.create(230, c::FEATURE);
                f.value(230, p::FEATURE_IS_END, Value::Boolean(true));
                member(&mut f, 200, 230, 231, c::END_FEATURE_MEMBERSHIP);
                relation(
                    &mut f,
                    230,
                    4,
                    232,
                    c::REFERENCE_SUBSETTING,
                    p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
                );
            }
            let snapshot = f.finish();
            let q = queries(&snapshot, P::OPERATIONAL_V6);
            let result = q.validate_connector_featuring(id(200));
            assert!(!result.value.valid);
            assert_eq!(result.value.role, None);
            assert!(result.value.exception_applied.iter().all(|v| !*v));
            assert_eq!(result.value.endpoints.len(), if extra { 3 } else { 2 });
        }
    }
}

#[test]
fn incomplete_referent_never_becomes_a_complete_endpoint() {
    use agq_kerml::BaselineProfile as P;
    let mut f = reference_builder(P::OPERATIONAL_V6);
    f.changes.clear(id(105), p::MEMBERSHIP_MEMBER_ELEMENT);
    let construction = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(
            &construction,
            SemanticOptions {
                baseline_profile: P::OPERATIONAL_V6,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let answer = q.reference_referent(id(2));
    assert_eq!(answer.value, None);
    assert_eq!(answer.completeness, Completeness::Incomplete);
}

#[test]
fn ambiguous_source_reference_is_not_a_canonical_referent() {
    use agq_kerml::BaselineProfile as P;
    let mut f = reference_builder(P::OPERATIONAL_V6);
    f.member(1, 150, 50, c::FEATURE, "leaf");
    f.changes.clear(id(105), p::MEMBERSHIP_MEMBER_ELEMENT);
    let candidate = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(
            &candidate,
            SemanticOptions {
                baseline_profile: P::OPERATIONAL_V6,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let resolution = q.resolve_reference(
        id(2),
        &QualifiedName {
            absolute: false,
            segments: vec!["leaf".into()],
        },
        c::FEATURE,
    );
    assert!(
        matches!(resolution.value, Resolution::Ambiguous(_)),
        "{resolution:?}"
    );
    assert_eq!(
        q.reference_referent(id(2)).completeness,
        Completeness::Incomplete
    );
    assert_eq!(q.reference_referent(id(2)).value, None);
}

#[test]
fn incomparable_connector_contexts_remain_incomplete_regardless_of_ids() {
    use agq_kerml::BaselineProfile as P;
    for second in [9, 999_999] {
        let mut f = reference_builder(P::OPERATIONAL_V6);
        f.create(second, c::FUNCTION);
        f.create(10, c::FUNCTION);
        f.owned
            .get_mut(&id(1))
            .unwrap()
            .retain(|v| *v != Value::Reference(id(103)));
        f.own(10, 103);
        for (specific, relationship) in [(1, 150), (second, 151)] {
            relation(
                &mut f,
                specific,
                10,
                relationship,
                c::SUBCLASSIFICATION,
                p::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            f.value(
                relationship,
                p::SUBCLASSIFICATION_SUBCLASSIFIER,
                Value::Reference(id(specific)),
            );
        }
        type_featuring(&mut f, 2, second, 152);
        let snapshot = f.finish();
        let q = queries(&snapshot, P::OPERATIONAL_V6);
        let context = q.reference_binding_context(id(2), id(4), id(3));
        assert_eq!(context.value, None, "{context:?}");
        assert_eq!(context.completeness, Completeness::Incomplete);
        assert!(
            context
                .diagnostics
                .iter()
                .any(|d| d.code == "KQ_AMBIGUOUS_CONNECTOR_CONTEXT")
        );
    }
}

#[test]
fn role_provenance_extra_endpoint_and_endpoint_asymmetry_are_enforced() {
    use agq_kerml::BaselineProfile as P;
    use agq_kernel::derived::DerivationBuilder;
    for (role, extra, reverse) in [
        (ImpliedBindingRole::FeatureReferenceResult, false, false),
        (ImpliedBindingRole::Invocation, false, false),
        (ImpliedBindingRole::FeatureReferenceResult, true, false),
        (ImpliedBindingRole::FeatureReferenceResult, false, true),
    ] {
        let mut f = reference_builder(P::OPERATIONAL_V6);
        f.create(900, c::BINDING_CONNECTOR); // Template supplies ordinary stored defaults only.
        let mut ends = vec![];
        for (n, endpoint) in
            [(200, 4), (203, 3), (206, 4)]
                .into_iter()
                .take(if extra { 3 } else { 2 })
        {
            f.create(n, c::FEATURE);
            f.value(n, p::FEATURE_IS_END, Value::Boolean(true));
            f.create(n + 1, c::END_FEATURE_MEMBERSHIP);
            f.changes.set(
                id(n + 1),
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(id(n))]),
                origin(),
            );
            relation(
                &mut f,
                n,
                endpoint,
                n + 2,
                c::REFERENCE_SUBSETTING,
                p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
            );
            ends.push(Value::Reference(id(n + 1)));
        }
        if reverse {
            ends.reverse();
        }
        let snapshot = f.finish();
        let key = |n| DerivationKey {
            rule: role.rule_id(P::OPERATIONAL_V6),
            subject: id(2),
            output: OutputKey::from_u128(n),
        };
        let connector = key(1).element_id();
        let membership = key(2).element_id();
        let featuring = key(3).element_id();
        let mut connector_slots: BTreeMap<_, _> = snapshot
            .model()
            .element(id(900))
            .unwrap()
            .slots()
            .map(|(p, s)| (p, s.value().clone()))
            .collect();
        connector_slots.insert(
            p::ELEMENT_ELEMENT_ID,
            SlotValue::Scalar(Value::String(connector.to_string())),
        );
        connector_slots.insert(
            p::RELATIONSHIP_IS_IMPLIED,
            SlotValue::Scalar(Value::Boolean(true)),
        );
        ends.push(Value::Reference(featuring));
        connector_slots.insert(p::ELEMENT_OWNED_RELATIONSHIP, SlotValue::Ordered(ends));
        let mut membership_slots: BTreeMap<_, _> = snapshot
            .model()
            .element(id(101))
            .unwrap()
            .slots()
            .filter(|(property, _)| {
                [
                    p::ELEMENT_ELEMENT_ID,
                    p::ELEMENT_IS_IMPLIED_INCLUDED,
                    p::RELATIONSHIP_IS_IMPLIED,
                    p::MEMBERSHIP_VISIBILITY,
                ]
                .contains(property)
            })
            .map(|(p, s)| (p, s.value().clone()))
            .collect();
        membership_slots.insert(
            p::ELEMENT_ELEMENT_ID,
            SlotValue::Scalar(Value::String(membership.to_string())),
        );
        membership_slots.insert(
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(connector)]),
        );
        let mut b = DerivationBuilder::new(snapshot);
        b.element(
            key(1),
            c::BINDING_CONNECTOR,
            connector_slots,
            BTreeSet::new(),
        );
        b.element(
            key(2),
            c::OWNING_MEMBERSHIP,
            membership_slots,
            BTreeSet::new(),
        );
        b.element(
            key(3),
            c::TYPE_FEATURING,
            [
                (
                    p::ELEMENT_IS_IMPLIED_INCLUDED,
                    SlotValue::Scalar(Value::Boolean(true)),
                ),
                (
                    p::ELEMENT_ELEMENT_ID,
                    SlotValue::Scalar(Value::String(featuring.to_string())),
                ),
                (
                    p::RELATIONSHIP_IS_IMPLIED,
                    SlotValue::Scalar(Value::Boolean(true)),
                ),
                (
                    p::TYPE_FEATURING_FEATURE_OF_TYPE,
                    SlotValue::Scalar(Value::Reference(connector)),
                ),
                (
                    p::TYPE_FEATURING_FEATURING_TYPE,
                    SlotValue::Scalar(Value::Reference(id(1))),
                ),
            ],
            BTreeSet::new(),
        );
        b.extend_ordered_references(
            id(2),
            p::ELEMENT_OWNED_RELATIONSHIP,
            vec![membership],
            agq_kernel::provenance::Explanation {
                rule: role.rule_id(P::OPERATIONAL_V6),
                dependencies: BTreeSet::new(),
            },
        );
        let overlay = b.build().unwrap();
        let q = KerMlQueries::new(
            SemanticContext::for_overlay(
                &overlay,
                SemanticOptions {
                    baseline_profile: P::OPERATIONAL_V6,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        let check = q.validate_connector_featuring(connector);
        let qualifies = role == ImpliedBindingRole::FeatureReferenceResult && !extra && !reverse;
        assert_eq!(check.value.role, Some(role));
        assert_eq!(
            check.value.valid, qualifies,
            "{role:?} extra={extra} reverse={reverse}: {check:?}"
        );
        assert_eq!(
            check.value.exception_applied.iter().filter(|v| **v).count(),
            usize::from(qualifies)
        );
        assert!(
            check
                .search_dependencies
                .contains(&SearchDependency::ImpliedBindingRole(Some(role)))
        );
    }
}
