//! Each authorized rule is exercised through its own antecedent and canonical graph.
include!("common/result_fixture.rs");

fn result(f: &mut Fixture, expression: u128, raw: u128, relationship: u128) {
    f.create(raw, c::FEATURE);
    f.enumeration(raw, p::FEATURE_DIRECTION, "out");
    member(
        f,
        expression,
        raw,
        relationship,
        c::RETURN_PARAMETER_MEMBERSHIP,
    );
}
fn fixture(
    profile: agq_kerml::BaselineProfile,
    array: bool,
    default: bool,
    directed: bool,
    explicit: bool,
) -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::NAMESPACE);
    f.member(1, 100, 2, c::CLASS, "habitat");
    f.member(1, 170, 70, c::PACKAGE, "Collections");
    f.member(70, 171, 71, c::DATA_TYPE, "Array");
    f.member(2, 110, 10, c::FEATURE, "foliage");
    f.create(20, c::EXPRESSION);
    member(&mut f, 10, 20, 120, c::FEATURE_VALUE);
    f.value(120, p::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(default));
    f.value(120, p::FEATURE_VALUE_IS_INITIAL, Value::Boolean(false));
    result(&mut f, 20, 21, 121);
    if directed {
        f.enumeration(10, p::FEATURE_DIRECTION, "in");
    }
    if explicit {
        f.member(2, 122, 22, c::FEATURE, "alternate");
        subset(&mut f, 10, 22, 123);
    }
    for (outer, outer_raw, inner, inner_raw, is_function) in
        [(30, 31, 32, 33, false), (40, 41, 42, 43, true)]
    {
        f.create(
            outer,
            if is_function {
                c::FUNCTION
            } else {
                c::EXPRESSION
            },
        );
        member(
            &mut f,
            2,
            outer,
            outer + 200,
            if is_function {
                c::OWNING_MEMBERSHIP
            } else {
                c::FEATURE_MEMBERSHIP
            },
        );
        result(&mut f, outer, outer_raw, outer + 201);
        f.create(inner, c::EXPRESSION);
        member(
            &mut f,
            outer,
            inner,
            outer + 202,
            c::RESULT_EXPRESSION_MEMBERSHIP,
        );
        type_featuring(&mut f, inner, outer, outer + 203);
        result(&mut f, inner, inner_raw, outer + 204);
    }
    for (expr, raw, parameter, argument, arg_raw, class) in [
        (50, 51, 52, 53, 55, c::INDEX_EXPRESSION),
        (60, 61, 62, 63, 65, c::SELECT_EXPRESSION),
    ] {
        f.create(expr, class);
        member(&mut f, 2, expr, expr + 300, c::FEATURE_MEMBERSHIP);
        result(&mut f, expr, raw, expr + 301);
        f.create(parameter, c::FEATURE);
        f.enumeration(parameter, p::FEATURE_DIRECTION, "in");
        member(&mut f, expr, parameter, expr + 302, c::PARAMETER_MEMBERSHIP);
        f.create(argument, c::EXPRESSION);
        member(&mut f, parameter, argument, expr + 303, c::FEATURE_VALUE);
        f.value(
            expr + 303,
            p::FEATURE_VALUE_IS_DEFAULT,
            Value::Boolean(false),
        );
        f.value(
            expr + 303,
            p::FEATURE_VALUE_IS_INITIAL,
            Value::Boolean(false),
        );
        result(&mut f, argument, arg_raw, expr + 304);
        if array {
            relation(
                &mut f,
                arg_raw,
                71,
                expr + 305,
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPE,
            );
            f.value(
                expr + 305,
                p::FEATURE_TYPING_TYPED_FEATURE,
                Value::Reference(id(arg_raw)),
            );
        }
    }
    f.finish()
}

#[test]
fn five_rules_and_seven_profiles_have_independent_graphs_and_domains() {
    use agq_kerml::BaselineProfile as P;
    let mut profiles = BTreeSet::new();
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
    ] {
        let snapshot = fixture(profile, false, false, false, false);
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(
                &snapshot,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        profiles.insert(q.context().baseline_profile_id);
        let expanded = q.derive_result_structure(&snapshot).unwrap();
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
        for (subject, raw, rule) in [
            (10, 21, ResultDomainRule::FeatureValuation),
            (51, 55, ResultDomainRule::IndexResult),
            (61, 65, ResultDomainRule::SelectResult),
        ] {
            let targets = view.subsetted_features(id(subject));
            assert_eq!(targets.completeness, Completeness::Complete);
            let target = if profile == P::OPERATIONAL_V6 {
                expanded
                    .contextual_results
                    .iter()
                    .find(|r| r.rule == rule)
                    .unwrap()
                    .feature
            } else {
                id(raw)
            };
            assert!(
                targets.value.contains(&target),
                "{profile:?} {rule:?}: {targets:?}"
            );
            let access = view.can_access(id(subject), target);
            assert_eq!(
                access.value,
                profile == P::OPERATIONAL_V6,
                "{rule:?}: {access:?}"
            );
        }
        for role in [
            ImpliedBindingRole::ExpressionResult,
            ImpliedBindingRole::FunctionResult,
        ] {
            let binding = *expanded
                .production
                .value
                .iter()
                .find(|b| view.implied_binding_role(**b) == Some(role))
                .unwrap();
            let check = view.validate_connector_featuring(binding);
            assert_eq!(
                check.value.valid,
                profile == P::OPERATIONAL_V6,
                "{role:?}: {check:?}"
            );
            assert!(check.value.exception_applied.iter().all(|v| !*v));
        }
        // FeatureValue's contextual binding is independently prescribed in all profiles.
        let binding = *expanded
            .production
            .value
            .iter()
            .find(|b| {
                view.implied_binding_role(**b) == Some(ImpliedBindingRole::FeatureValue)
                    && view.owner(**b).value == Some(id(10))
            })
            .unwrap();
        assert!(view.validate_connector_featuring(binding).value.valid);
        if profile == P::OPERATIONAL_V6 {
            for rule in ResultDomainRule::ALL {
                assert!(expanded.contextual_results.iter().any(|r| r.rule == rule));
            }
        }
    }
    assert_eq!(profiles.len(), 7);
}

#[test]
fn valuation_antecedent_and_array_guard_are_not_copied_from_other_rules() {
    use agq_kerml::BaselineProfile as P;
    for (array, default, directed, explicit) in [
        (true, false, false, false),
        (false, true, false, false),
        (false, false, true, false),
        (false, false, false, true),
    ] {
        let snapshot = fixture(P::OPERATIONAL_V6, array, default, directed, explicit);
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(
                &snapshot,
                SemanticOptions {
                    baseline_profile: P::OPERATIONAL_V6,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        let expanded = q.derive_result_structure(&snapshot).unwrap();
        assert_eq!(
            expanded.production.completeness,
            Completeness::Complete,
            "{:?}",
            expanded.production.diagnostics
        );
        assert_eq!(
            expanded
                .contextual_results
                .iter()
                .any(|r| r.rule == ResultDomainRule::IndexResult),
            !array
        );
        assert!(
            expanded
                .contextual_results
                .iter()
                .any(|r| r.rule == ResultDomainRule::SelectResult)
        );
        assert_eq!(
            expanded
                .contextual_results
                .iter()
                .any(|r| r.rule == ResultDomainRule::FeatureValuation),
            !directed && !explicit
        );
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
        assert_eq!(
            expanded
                .production
                .value
                .iter()
                .any(
                    |b| view.implied_binding_role(*b) == Some(ImpliedBindingRole::FeatureValue)
                        && view.owner(*b).value == Some(id(10))
                ),
            !default
        );
    }
}
