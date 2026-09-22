include!("common/result_fixture.rs");

fn libraries() -> LibrarySetIdentity {
    LibrarySetIdentity {
        artifacts: StandardLibraryArtifact::ALL
            .into_iter()
            .map(|artifact| (artifact, LibraryId::from_u128(1)))
            .collect(),
        pins: BTreeSet::new(),
    }
}

#[test]
fn extension_rule_registry_only_authorizes_exact_provenance_ids() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let key = DerivationKey {
        rule: RuleId::from_u128(765),
        subject: id(1),
        output: OutputKey::from_u128(1),
    };
    let mut plan = q.plan_result_structure([]);
    let produced = plan
        .add_derived_element(
            key,
            c::CLASSIFIER,
            BTreeMap::new(),
            None,
            &q.canonical_fact_evidence(FactKey::Element(id(1))),
        )
        .unwrap()
        .unwrap();
    let result = plan.materialize(&snapshot).unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&result.overlay, Default::default(), BTreeSet::new()).unwrap(),
    );
    let rejected = |report: &PublicationCapabilityReport| {
        report
            .failures
            .values()
            .flatten()
            .any(|d| d.code == "KQ_PUBLICATION_RULE")
    };
    assert!(rejected(&q.audit_publication_capabilities([produced])));
    assert!(rejected(&q.audit_publication_capabilities_with_rules(
        [produced],
        [RuleId::from_u128(766)]
    )));
    let allowed = q.audit_publication_capabilities_with_rules([produced], [key.rule]);
    assert!(!rejected(&allowed));
    assert_eq!(
        allowed.context.derivation_phase,
        DerivationPhase::PartialDerivationOverlay
    );
}

#[test]
fn a_stage_limit_never_promotes_an_unexecuted_overlay() {
    let snapshot = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V8).unwrap(),
    ));
    let libraries = libraries();
    assert!(
        matches!(CanonicalPublicationBuilder::new(&snapshot, &[], &libraries).build(0, |_| panic!("no stages allowed")), Err(PublicationOverlayError::IncompleteProducers(stages)) if stages.is_empty())
    );
    assert!(matches!(
        CanonicalPublicationBuilder::new(&snapshot, &[], &libraries)
            .build(1, |_| panic!("missing bindings must fail first")),
        Err(PublicationOverlayError::Bindings(_))
    ));
}

#[test]
fn authored_facts_cannot_be_presented_as_standard_publication_inputs() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    let snapshot = f.finish();
    let libraries = libraries();
    assert!(
        matches!(CanonicalPublicationBuilder::new(&snapshot, &[], &libraries).build(1, |_| panic!("foreign source must fail first")), Err(PublicationOverlayError::ForeignDeclaredFact(FactKey::Element(element))) if element == id(1))
    );
}

#[test]
fn a_partial_overlay_context_remains_partial_after_scoped_capability_checks() {
    let snapshot = Fixture::new().finish();
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot)
        .build()
        .unwrap();
    let context =
        SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap();
    let q = KerMlQueries::new(context);
    let audit = q.audit_publication_capabilities([]);
    assert!(audit.failures.is_empty());
    assert_eq!(
        audit.context.derivation_phase,
        DerivationPhase::PartialDerivationOverlay
    );
    assert_eq!(
        q.context().derivation_phase,
        DerivationPhase::PartialDerivationOverlay
    );
}

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
