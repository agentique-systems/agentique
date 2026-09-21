include!("common/result_fixture.rs");

#[test]
fn behavior_invocation_produces_owned_result_specialization_and_binding_across_stages() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::INVOCATION_EXPRESSION);
    f.create(2, c::BEHAVIOR);
    f.create(3, c::MEMBERSHIP);
    f.own(1, 3);
    f.value(3, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(2)));
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap(),
    );
    assert_eq!(q.instantiated_type(id(1)).value, Some(id(2)));
    let first = q
        .plan_result_structure([id(1)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(first.production.completeness, Completeness::Incomplete);
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&first.overlay, options.clone(), BTreeSet::new()).unwrap(),
    );
    let raw = q.structural_result(id(1)).value.unwrap();
    assert_eq!(q.owning_type(raw).value, Some(id(1)));
    let second = q
        .plan_result_structure([id(1), raw])
        .materialize_on_overlay(&first.overlay)
        .unwrap();
    assert_eq!(
        second.production.completeness,
        Completeness::Complete,
        "{:?}",
        second.production.diagnostics
    );
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&second.overlay, options, BTreeSet::new()).unwrap(),
    );
    assert_eq!(q.feature_types(id(1)).value, vec![id(2)]);
    assert_eq!(q.feature_types(raw).value, vec![id(2)]);
    let binding = *second.production.value.first().unwrap();
    assert_eq!(
        q.implied_binding_role(binding),
        Some(ImpliedBindingRole::Invocation)
    );
    assert_eq!(q.connector_endpoints(binding).value, vec![id(1), raw]);
    assert!(q.direct_features(id(1)).value.contains(&binding));
    let third = q
        .plan_result_structure([raw, id(1)])
        .materialize_on_overlay(&second.overlay)
        .unwrap();
    assert_eq!(
        format!("{:?}", second.overlay.model()),
        format!("{:?}", third.overlay.model())
    );
    assert!(snapshot.model().element(raw).is_none());
}
