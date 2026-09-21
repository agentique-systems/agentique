use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn variable_features_share_one_snapshot_domain_with_stable_batch_evidence() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    let mut targets = BTreeMap::new();
    for (index, role) in StandardRole::ALL.into_iter().enumerate() {
        let element = 1000 + index as u128;
        f.create(element, role.specification().1);
        targets.insert(
            role,
            BoundStandardElement {
                element: id(element),
                library: LibraryId::from_u128(1),
            },
        );
    }
    let snapshots = targets[&StandardRole::OccurrenceSnapshots].element;
    let occurrence = targets[&StandardRole::Occurrence].element;
    member(
        &mut f,
        occurrence.as_u128(),
        snapshots.as_u128(),
        500,
        c::FEATURE_MEMBERSHIP,
    );
    relation(
        &mut f,
        snapshots.as_u128(),
        occurrence.as_u128(),
        501,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        501,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(snapshots),
    );
    f.create(1, c::CLASS);
    relation(
        &mut f,
        1,
        occurrence.as_u128(),
        502,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(502, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
    for n in [10, 11] {
        f.create(n, c::FEATURE);
        f.value(n, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
        member(&mut f, 1, n, n + 100, c::FEATURE_MEMBERSHIP);
    }
    let snapshot = f.finish();
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    // Algorithm fixture; exact-path, per-artifact binding validation has separate
    // integration coverage. No constructor bypass is exposed to production callers.
    let bindings = Arc::new(StandardKermlBindings {
        targets,
        library_set: LibrarySetIdentity {
            artifacts: BTreeMap::from([(
                StandardLibraryArtifact::Semantic,
                LibraryId::from_u128(1),
            )]),
            pins: BTreeSet::new(),
        },
    });
    let mut context =
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap();
    context.id.standard_bindings = Some(bindings.clone());
    let q = KerMlQueries::new(context);
    assert_eq!(
        q.featuring_types(id(10)).completeness,
        Completeness::Incomplete
    );
    let all = q
        .plan_result_structure([id(10), id(11)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(
        all.production.completeness,
        Completeness::Complete,
        "{:?}",
        all.production.diagnostics
    );
    let mut plan = q.plan_result_structure([id(11)]);
    plan.merge(q.plan_result_structure([id(10)])).unwrap();
    let batched = plan.materialize(&snapshot).unwrap();
    assert_eq!(
        format!("{:?}", all.overlay.model()),
        format!("{:?}", batched.overlay.model())
    );
    let mut context = SemanticContext::for_overlay(&all.overlay, options, BTreeSet::new()).unwrap();
    context.id.standard_bindings = Some(bindings);
    let q = KerMlQueries::new(context);
    let domains = q.featuring_types(id(10));
    assert_eq!(
        domains.completeness,
        Completeness::Complete,
        "{:?}",
        domains.diagnostics
    );
    assert_eq!(domains.value, q.featuring_types(id(11)).value);
    assert_eq!(domains.value.len(), 1);
    let domain = domains.value[0];
    assert_ne!(domain, snapshots);
    assert_eq!(q.owning_type(domain).value, Some(id(1)));
    assert!(q.is_featuring_type(id(10), domain).value);
    assert!(q.redefined_features(domain).value.contains(&snapshots));
    assert_eq!(q.feature_types(domain).value, vec![occurrence]);
    let again = q
        .plan_result_structure([id(11), id(10)])
        .materialize_on_overlay(&all.overlay)
        .unwrap();
    assert_eq!(
        format!("{:?}", all.overlay.model()),
        format!("{:?}", again.overlay.model())
    );
    assert!(snapshot.model().element(domain).is_none());
}
