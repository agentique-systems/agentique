include!("common/result_fixture.rs");

#[test]
fn initial_value_context_is_the_exact_shared_chain_and_is_batch_invariant() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V7;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::NAMESPACE);
    f.member(1, 102, 2, c::PACKAGE, "Base");
    f.member(2, 103, 3, c::FEATURE, "things");
    f.member(3, 104, 4, c::FEATURE, "that");
    f.member(1, 105, 5, c::PACKAGE, "Occurrences");
    f.member(5, 106, 6, c::CLASS, "Occurrence");
    f.member(6, 107, 7, c::FEATURE, "startShot");
    for (feature, expression, raw, value) in [(10, 11, 12, 110), (20, 21, 22, 120)] {
        f.member(6, feature + 200, feature, c::FEATURE, "initial");
        f.create(expression, c::EXPRESSION);
        member(&mut f, feature, expression, value, c::FEATURE_VALUE);
        f.value(value, p::FEATURE_VALUE_IS_INITIAL, Value::Boolean(true));
        f.create(raw, c::FEATURE);
        f.enumeration(raw, p::FEATURE_DIRECTION, "out");
        member(
            &mut f,
            expression,
            raw,
            value + 1,
            c::RETURN_PARAMETER_MEMBERSHIP,
        );
    }
    let snapshot = f.finish();
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), Default::default()).unwrap(),
    );
    let all = q.plan_result_structure([id(10), id(20)]);
    assert_eq!(
        all.production.completeness,
        Completeness::Complete,
        "{:?}",
        all.production.diagnostics
    );
    let planned: BTreeSet<_> = all.planned_elements().collect();
    let partitioned: BTreeSet<_> = [id(20), id(10)]
        .into_iter()
        .flat_map(|subject| {
            q.plan_result_structure([subject])
                .planned_elements()
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(planned, partitioned);
    let reversed = q
        .plan_result_structure([id(20), id(10)])
        .materialize(&snapshot)
        .unwrap();
    let result = all.materialize(&snapshot).unwrap();
    let mut merged = q.plan_result_structure([id(20)]);
    merged.merge(q.plan_result_structure([id(10)])).unwrap();
    let merged = merged.materialize(&snapshot).unwrap();
    assert_eq!(result.production, reversed.production);
    assert_eq!(result.production, merged.production);
    assert_eq!(result.contextual_results, reversed.contextual_results);
    assert_eq!(result.contextual_results, merged.contextual_results);
    assert_eq!(
        format!("{:?}", merged.overlay.model()),
        format!("{:?}", result.overlay.model())
    );
    assert_eq!(
        format!("{:?}", result.overlay.model()),
        format!("{:?}", reversed.overlay.model())
    );
    assert_eq!(q.feature_with_value(id(110)).value, Some(id(10)));
    assert_eq!(q.structural_result(id(11)).value, Some(id(12)));
    let derived = KerMlQueries::new(
        SemanticContext::for_overlay(&result.overlay, options, Default::default()).unwrap(),
    );
    assert_eq!(result.production.value.len(), 2);
    let contexts: Vec<_> = result
        .production
        .value
        .iter()
        .map(|&binding| {
            assert_eq!(
                derived.implied_binding_role(binding),
                Some(ImpliedBindingRole::FeatureValue)
            );
            let domains = derived.featuring_types(binding);
            assert_eq!(domains.completeness, Completeness::Complete);
            assert_eq!(domains.value.len(), 1);
            domains.value[0]
        })
        .collect();
    assert_eq!(contexts[0], contexts[1]);
    assert_eq!(
        derived.chaining_features(contexts[0]).value,
        vec![id(4), id(7)]
    );
    assert_eq!(derived.feature_target(contexts[0]).value, Some(id(7)));
}
