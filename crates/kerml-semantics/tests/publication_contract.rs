include!("common/result_fixture.rs");

#[test]
fn partial_overlay_defers_completeness_assertion_without_changing_source_flags() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::FEATURE);
    subset(&mut f, 1, 2, 3);
    f.value(3, p::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true));
    let snapshot = f.finish();
    let before = snapshot.model().element(id(1)).unwrap().clone();
    let declared = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    assert!(
        declared
            .validate_local_structure(id(1))
            .diagnostics
            .iter()
            .any(|d| d.code == "validateElementIsImpliedIncluded")
    );
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let partial = KerMlQueries::new(
        SemanticContext::for_overlay(&overlay, Default::default(), Default::default()).unwrap(),
    );
    assert_ne!(partial.context(), declared.context());
    assert_eq!(
        partial.validation_deferred_by_phase(),
        ["validateElementIsImpliedIncluded"]
    );
    let local = partial.validate_local_structure(id(1));
    assert!(!local.value.contains(&"validateElementIsImpliedIncluded"));
    assert!(
        !local
            .diagnostics
            .iter()
            .any(|d| d.code == "validateElementIsImpliedIncluded")
    );
    assert_eq!(
        partial.validate_implied_inclusion(id(1)).completeness,
        Completeness::Invalid
    );
    assert_eq!(&before, snapshot.model().element(id(1)).unwrap());
    assert_eq!(&before, overlay.model().element(id(1)).unwrap());
}

#[test]
fn coverage_does_not_confuse_deferred_checks_or_extra_names_with_completion() {
    let mut coverage = ConstraintCoverage {
        inventory: BTreeSet::from(["one".into(), "two".into()]),
        checked: BTreeSet::from(["one".into()]),
        deferred_by_phase: BTreeSet::from(["two".into()]),
    };
    assert_eq!(coverage.status(), ValidationCoverage::Incomplete);
    coverage.checked.insert("two".into());
    assert_eq!(coverage.status(), ValidationCoverage::Incomplete);
    coverage.deferred_by_phase.clear();
    assert_eq!(coverage.status(), ValidationCoverage::Complete);
    coverage.checked.insert("unknown".into());
    assert_eq!(coverage.status(), ValidationCoverage::Incomplete);
}
