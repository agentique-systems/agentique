use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn variable_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
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
    (snapshot, bindings)
}
fn expression_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
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
    let package = targets[&StandardRole::ControlFunctions].element;
    let standard_target = targets[&StandardRole::FeatureChainSourceTarget].element;
    f.create(500, c::FUNCTION);
    f.value(500, p::ELEMENT_DECLARED_NAME, Value::String(".".into()));
    member(&mut f, package.as_u128(), 500, 501, c::OWNING_MEMBERSHIP);
    f.create(502, c::FEATURE);
    f.enumeration(502, p::FEATURE_DIRECTION, "in");
    member(&mut f, 500, 502, 503, c::PARAMETER_MEMBERSHIP);
    member(
        &mut f,
        502,
        standard_target.as_u128(),
        504,
        c::FEATURE_MEMBERSHIP,
    );
    f.create(900, c::FEATURE);
    f.create(901, c::CLASSIFIER);
    relation(
        &mut f,
        900,
        901,
        902,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        902,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(900)),
    );
    f.create(1, c::FEATURE_CHAIN_EXPRESSION);
    f.value(
        1,
        p::FEATURE_CHAIN_EXPRESSION_OPERATOR,
        Value::String(".".into()),
    );
    f.create(2, c::FEATURE);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    member(&mut f, 1, 2, 3, c::PARAMETER_MEMBERSHIP);
    f.create(4, c::MEMBERSHIP);
    f.own(1, 4);
    f.value(4, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(900)));
    let snapshot = f.finish();
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
    (snapshot, bindings)
}
fn crossing_fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V8).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::ASSOCIATION);
    for (end, ty, membership, typing) in [(10, 20, 30, 40), (11, 21, 31, 41)] {
        f.create(end, c::FEATURE);
        f.create(ty, c::CLASSIFIER);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, end, membership, c::END_FEATURE_MEMBERSHIP);
        relation(
            &mut f,
            end,
            ty,
            typing,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            typing,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(end)),
        );
    }
    f.create(9, c::FEATURE);
    member(&mut f, 10, 9, 50, c::OWNING_MEMBERSHIP);
    f.finish()
}
fn close(
    snapshot: &Snapshot,
    bindings: Option<&Arc<StandardKermlBindings>>,
    options: PublicationClosureOptions,
) -> PublicationClosure {
    close_result_structure(
        snapshot,
        options,
        |overlay| {
            let mut context = SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .map_err(PublicationOverlayError::Context)?;
            context.id.standard_bindings = bindings.cloned();
            Ok(context)
        },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap()
}
fn compare(
    expected: &PublicationClosure,
    actual: &PublicationClosure,
    bindings: Option<&Arc<StandardKermlBindings>>,
) {
    assert!(actual.converged, "{:?}", actual.stages);
    assert_eq!(expected.completeness, actual.completeness);
    assert!(
        expected
            .overlay
            .model()
            .elements()
            .eq(actual.overlay.model().elements()),
        "canonical records and ownership differ"
    );
    assert!(
        expected
            .overlay
            .model()
            .association_occurrences()
            .eq(actual.overlay.model().association_occurrences()),
        "occurrences differ"
    );
    assert!(
        expected.overlay.facts().eq(actual.overlay.facts()),
        "derived explanations differ"
    );
    let query = |overlay| {
        let mut context = SemanticContext::for_overlay(
            overlay,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap();
        context.id.standard_bindings = bindings.cloned();
        KerMlQueries::new(context)
    };
    let expected_q = query(&expected.overlay);
    let actual_q = query(&actual.overlay);
    assert_eq!(
        expected_q.context(),
        actual_q.context(),
        "semantic digest/search metadata differ"
    );
    let population: Vec<_> = actual.overlay.model().elements().map(|r| r.id()).collect();
    let a = expected_q.audit_publication_capabilities(population.iter().copied());
    let b = actual_q.audit_publication_capabilities(population.iter().copied());
    assert_eq!(a.checked_items, b.checked_items);
    assert_eq!(a.failures, b.failures);
    for subject in population {
        if actual_q.is(subject, c::FEATURE) {
            assert_eq!(
                expected_q.feature_types(subject),
                actual_q.feature_types(subject)
            );
            assert_eq!(
                expected_q.featuring_types(subject),
                actual_q.featuring_types(subject)
            );
            assert_eq!(
                expected_q.cross_feature(subject),
                actual_q.cross_feature(subject)
            );
        }
    }
}
fn permutations(
    snapshot: &Snapshot,
    bindings: Option<&Arc<StandardKermlBindings>>,
    subjects: Option<BTreeSet<ElementId>>,
) {
    let options = PublicationClosureOptions {
        initial_subjects: subjects,
        ..Default::default()
    };
    let expected = close(
        snapshot,
        bindings,
        PublicationClosureOptions {
            strategy: PublicationClosureStrategy::ReferenceFullScan,
            ..options.clone()
        },
    );
    assert!(expected.converged, "{:?}", expected.stages);
    assert_eq!(
        expected.completeness,
        Completeness::Complete,
        "{:?}",
        expected.stages.last()
    );
    for order in [
        PublicationWorklistOrder::Fifo,
        PublicationWorklistOrder::Lifo,
        PublicationWorklistOrder::ReversedInitial,
        PublicationWorklistOrder::Partitioned,
    ] {
        for batch_size in [1, 7, 32] {
            let actual = close(
                snapshot,
                bindings,
                PublicationClosureOptions {
                    order,
                    batch_size,
                    ..options.clone()
                },
            );
            compare(&expected, &actual, bindings);
        }
    }
}
#[test]
fn worklist_matches_fullscan_for_crossing_occurrences_and_negative_searches() {
    permutations(&crossing_fixture(), None, None);
}
#[test]
fn worklist_matches_fullscan_for_shared_variable_domains() {
    let (snapshot, bindings) = variable_fixture();
    permutations(
        &snapshot,
        Some(&bindings),
        Some(BTreeSet::from([id(10), id(11)])),
    );
}
#[test]
fn worklist_matches_fullscan_for_multiple_expression_rounds() {
    let (snapshot, bindings) = expression_fixture();
    permutations(
        &snapshot,
        Some(&bindings),
        Some(BTreeSet::from([id(1), id(2)])),
    );
}
#[test]
fn resource_limit_does_not_claim_closure() {
    let (snapshot, bindings) = expression_fixture();
    let result = close(
        &snapshot,
        Some(&bindings),
        PublicationClosureOptions {
            max_rounds: 1,
            initial_subjects: Some(BTreeSet::from([id(1), id(2)])),
            ..Default::default()
        },
    );
    assert!(!result.converged);
    assert_eq!(result.completeness, Completeness::Incomplete);
}
