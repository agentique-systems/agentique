include!("common/result_fixture.rs");
use agq_kerml::BaselineProfile as P;

fn fixture(profile: P, abstract_flow: bool) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::FLOW);
    f.value(1, p::TYPE_IS_ABSTRACT, Value::Boolean(abstract_flow));
    for (end, membership) in [(3, 13), (2, 12)] {
        f.create(end, c::FEATURE);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, end, membership, c::END_FEATURE_MEMBERSHIP);
    }
    f
}

fn queries(snapshot: &Snapshot, profile: P) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(
            snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    )
}

#[test]
fn absent_related_features_are_structurally_complete_without_changing_strict_contracts() {
    for profile in [P::PublishedKerMl10, P::OPERATIONAL_V8, P::OPERATIONAL_V9] {
        for abstract_flow in [true, false] {
            let snapshot = fixture(profile, abstract_flow).finish();
            let q = queries(&snapshot, profile);
            let related = q.connector_related_features(id(1));
            assert_eq!(related.completeness, Completeness::Complete);
            assert!(related.value.is_empty());
            let structure = q.connector_related_structure(id(1));
            assert_eq!(
                structure.completeness,
                Completeness::Complete,
                "{:?}",
                structure.diagnostics
            );
            assert_eq!(structure.value.ends, vec![id(3), id(2)]);
            assert!(structure.value.related_features.is_empty());
            assert_eq!(structure.value.source_feature, None);
            assert_eq!(structure.value.default_featuring_type, None);
            // Concrete cardinality is a separate validateConnectorRelatedFeatures
            // obligation. Complete structure alone makes no conformance claim.
            assert_eq!(
                q.connector_endpoints(id(1)).completeness,
                Completeness::Incomplete
            );
            assert_eq!(
                q.connector_structure(id(1)).completeness,
                Completeness::Incomplete
            );
            assert_ne!(
                q.validate_connector_featuring(id(1)).completeness,
                Completeness::Complete
            );
        }
    }
}

#[test]
fn related_features_filter_missing_references_in_semantic_end_order() {
    let mut f = fixture(P::OPERATIONAL_V9, true);
    f.create(90, c::FEATURE);
    relation(
        &mut f,
        2,
        90,
        50,
        c::REFERENCE_SUBSETTING,
        p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
    );
    let snapshot = f.finish();
    let q = queries(&snapshot, P::OPERATIONAL_V9);
    let related = q.connector_related_features(id(1));
    assert_eq!(
        related.completeness,
        Completeness::Complete,
        "{:?}",
        related.diagnostics
    );
    assert_eq!(related.value, vec![id(90)]);
    assert_eq!(
        q.connector_endpoints(id(1)).completeness,
        Completeness::Incomplete
    );
    assert!(
        related
            .search_dependencies
            .contains(&SearchDependency::PropertySet {
                element: id(3),
                property: p::ELEMENT_OWNED_RELATIONSHIP,
            })
    );
}

#[test]
fn ordered_nonunique_related_features_preserve_repeated_end_targets() {
    let mut f = fixture(P::OPERATIONAL_V9, true);
    f.create(90, c::FEATURE);
    for (end, reference) in [(3, 51), (2, 50)] {
        relation(
            &mut f,
            end,
            90,
            reference,
            c::REFERENCE_SUBSETTING,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        );
    }
    let snapshot = f.finish();
    let descriptor = snapshot
        .model()
        .registry()
        .property(p::CONNECTOR_RELATED_FEATURE)
        .unwrap();
    assert!(descriptor.ordered);
    assert!(!descriptor.unique);
    assert_eq!(descriptor.multiplicity.lower, 0);
    assert_eq!(descriptor.multiplicity.upper, None);
    let q = queries(&snapshot, P::OPERATIONAL_V9);
    let related = q.connector_related_features(id(1));
    assert_eq!(related.completeness, Completeness::Complete);
    assert_eq!(related.value, vec![id(90), id(90)]);
    assert_eq!(related.value, q.connector_endpoints(id(1)).value);
}

#[test]
fn pending_end_population_and_present_missing_endpoint_remain_incomplete() {
    let snapshot = fixture(P::OPERATIONAL_V9, true).finish();
    for specialization in [true, false] {
        let context = SemanticContext::for_project_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: P::OPERATIONAL_V9,
                ..Default::default()
            },
            BTreeSet::new(),
            if specialization {
                BTreeSet::from([id(2)])
            } else {
                BTreeSet::new()
            },
            if specialization {
                BTreeSet::new()
            } else {
                BTreeSet::from([id(2)])
            },
        )
        .unwrap();
        let related = KerMlQueries::new(context).connector_related_features(id(1));
        assert_eq!(related.completeness, Completeness::Incomplete);
        assert!(
            related
                .diagnostics
                .iter()
                .any(|d| d.code == "KQ_CONNECTOR_REFERENCE_POPULATION")
        );
    }
    let mut f = fixture(P::OPERATIONAL_V9, true);
    f.create(50, c::REFERENCE_SUBSETTING);
    f.own(2, 50);
    let candidate = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(
            &candidate,
            SemanticOptions {
                baseline_profile: P::OPERATIONAL_V9,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    let related = q.connector_related_features(id(1));
    assert_eq!(
        related.completeness,
        Completeness::Incomplete,
        "{:?}",
        related.diagnostics
    );
    assert!(
        related
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_CONNECTOR_ENDPOINT")
    );
}

#[test]
fn multiple_reference_subsettings_never_select_by_identity_order() {
    let mut f = fixture(P::OPERATIONAL_V9, true);
    for (reference, target) in [(51, 90), (50, 91)] {
        f.create(target, c::FEATURE);
        relation(
            &mut f,
            2,
            target,
            reference,
            c::REFERENCE_SUBSETTING,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        );
    }
    let snapshot = f.finish();
    let related = queries(&snapshot, P::OPERATIONAL_V9).connector_related_features(id(1));
    assert_eq!(related.completeness, Completeness::Incomplete);
    assert!(related.value.is_empty());
    assert!(
        related
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_CONNECTOR_ENDPOINT")
    );
}

#[test]
fn inherited_abstract_flow_ends_keep_original_kernel_identity() {
    let mut f = fixture(P::OPERATIONAL_V9, true);
    f.create(5, c::FLOW);
    f.value(5, p::TYPE_IS_ABSTRACT, Value::Boolean(true));
    relation(
        &mut f,
        5,
        1,
        60,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(60, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(5)));
    let snapshot = f.finish();
    let result = queries(&snapshot, P::OPERATIONAL_V9).connector_related_structure(id(5));
    assert_eq!(
        result.completeness,
        Completeness::Complete,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.value.ends, vec![id(3), id(2)]);
    assert!(result.value.related_features.is_empty());
}
