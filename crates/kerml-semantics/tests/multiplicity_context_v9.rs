include!("common/result_fixture.rs");
use agq_kerml::BaselineProfile as P;

fn fixture(profile: P) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (n, class) in [
        (1, c::ASSOCIATION),
        (2, c::FEATURE_REFERENCE_EXPRESSION),
        (3, c::FEATURE),
        (4, c::FEATURE),
        (10, c::FEATURE),
        (20, c::MULTIPLICITY_RANGE),
    ] {
        f.create(n, class);
        f.value(
            n,
            p::ELEMENT_DECLARED_NAME,
            Value::String(format!("arbitrary_{n}")),
        );
    }
    member(&mut f, 1, 4, 104, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 10, 110, c::END_FEATURE_MEMBERSHIP);
    member(&mut f, 20, 2, 122, c::OWNING_MEMBERSHIP);
    member(&mut f, 2, 3, 123, c::RETURN_PARAMETER_MEMBERSHIP);
    f.enumeration(3, p::FEATURE_DIRECTION, "out");
    relation(
        &mut f,
        2,
        4,
        124,
        c::MEMBERSHIP,
        p::MEMBERSHIP_MEMBER_ELEMENT,
    );
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
fn ordinary_bound_context_changes_only_in_v9_for_every_historical_profile() {
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
        P::OPERATIONAL_V7,
        P::OPERATIONAL_V8,
        P::OPERATIONAL_V9,
    ] {
        let mut f = fixture(profile);
        member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
        let snapshot = f.finish();
        let q = queries(&snapshot, profile);
        let expected = if profile == P::OPERATIONAL_V9 {
            vec![id(1)]
        } else {
            vec![]
        };
        assert_eq!(q.owning_type(id(20)).value, None);
        for subject in [id(20), id(2)] {
            let domain = q.featuring_types(subject);
            assert_eq!(
                domain.completeness,
                Completeness::Complete,
                "{profile:?}: {:?}",
                domain.diagnostics
            );
            assert_eq!(domain.value, expected, "{profile:?}: {subject:?}");
        }
        assert_eq!(q.multiplicity_featuring_context(id(20)).value, expected);
        let binding = q.reference_binding_context(id(2), id(4), id(3));
        if profile == P::OPERATIONAL_V9 {
            assert_eq!(
                binding.completeness,
                Completeness::Complete,
                "{:?}",
                binding.diagnostics
            );
            assert_eq!(binding.value, Some(id(1)));
        } else {
            assert_eq!(binding.value, None);
        }
    }
}

#[test]
fn owned_cross_bounds_use_the_owning_end_domain_not_the_cross_domain() {
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
        P::OPERATIONAL_V7,
        P::OPERATIONAL_V8,
        P::OPERATIONAL_V9,
    ] {
        let mut f = fixture(profile);
        f.value(10, p::FEATURE_IS_END, Value::Boolean(true));
        f.create(11, c::FEATURE);
        f.create(30, c::FEATURE);
        f.create(40, c::CLASSIFIER);
        f.value(30, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, 30, 130, c::END_FEATURE_MEMBERSHIP);
        relation(
            &mut f,
            30,
            40,
            140,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            140,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(30)),
        );
        member(&mut f, 10, 11, 111, c::OWNING_MEMBERSHIP);
        member(&mut f, 11, 20, 120, c::OWNING_MEMBERSHIP);
        let snapshot = f.finish();
        let q = queries(&snapshot, profile);
        assert!(q.is_owned_cross_feature(id(11)).value);
        if profile.corrects_owned_cross_domain() {
            assert_eq!(q.featuring_types(id(11)).value, vec![id(40)]);
        }
        for subject in [id(20), id(2)] {
            let domain = q.featuring_types(subject);
            assert_eq!(
                domain.completeness,
                Completeness::Complete,
                "{:?}",
                domain.diagnostics
            );
            assert_eq!(
                domain.value,
                if profile == P::OPERATIONAL_V9 {
                    vec![id(1)]
                } else {
                    vec![]
                }
            );
        }
        if profile == P::OPERATIONAL_V9 {
            let binding = q.reference_binding_context(id(2), id(4), id(3));
            assert_eq!(
                binding.completeness,
                Completeness::Complete,
                "{:?}",
                binding.diagnostics
            );
            assert_eq!(binding.value, Some(id(1)));
            let context = q.multiplicity_featuring_context(id(20));
            assert!(context.search_dependencies.iter().any(|d| matches!(
                d,
                SearchDependency::ValidationRule("agentique-kerml10-cross-multiplicity-context/1")
            )));
        }
    }
}

#[test]
fn classifier_package_and_missing_owners_have_no_lexical_fallback() {
    for owner_class in [Some(c::CLASSIFIER), Some(c::PACKAGE), None] {
        let mut f = fixture(P::OPERATIONAL_V9);
        if let Some(class) = owner_class {
            f.create(50, class);
            member(&mut f, 10, 50, 150, c::OWNING_MEMBERSHIP);
            member(&mut f, 50, 20, 120, c::OWNING_MEMBERSHIP);
        }
        let snapshot = f.finish();
        let q = queries(&snapshot, P::OPERATIONAL_V9);
        for subject in [id(20), id(2)] {
            let domain = q.featuring_types(subject);
            assert_eq!(domain.completeness, Completeness::Complete);
            assert!(domain.value.is_empty());
        }
    }
}

#[test]
fn incomplete_owner_evidence_propagates_into_multiplicity_and_bound() {
    let mut f = fixture(P::OPERATIONAL_V9);
    member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
    f.changes.clear(id(10), p::FEATURE_IS_VARIABLE);
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
    assert_eq!(
        q.multiplicity_featuring_context(id(20)).completeness,
        Completeness::Incomplete
    );
    assert_eq!(
        q.featuring_types(id(2)).completeness,
        Completeness::Incomplete
    );
    assert_eq!(
        q.reference_binding_context(id(2), id(4), id(3))
            .completeness,
        Completeness::Incomplete
    );
}

#[test]
fn explicitly_unresolved_owning_namespace_retains_failure_and_search_evidence() {
    use agq_kernel::derived::{
        ComputationFailure, DerivationBuilder, IncompleteReason, StructuralSearch,
    };
    for (subject, property) in [
        (20, p::ELEMENT_OWNING_NAMESPACE),
        (120, p::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE),
    ] {
        let mut f = fixture(P::OPERATIONAL_V9);
        if subject == 120 {
            // A known owning Membership whose namespace is still unresolved.
            f.create(120, c::OWNING_MEMBERSHIP);
            f.changes.set(
                id(120),
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(id(20))]),
                origin(),
            );
        }
        let snapshot = f.finish();
        for invalid in [false, true] {
            let explanation = agq_kernel::provenance::Explanation {
                rule: RuleId::from_u128(900),
                dependencies: BTreeSet::new(),
            };
            let searches = BTreeSet::from([StructuralSearch::Incoming(id(901))]);
            let failure = if invalid {
                ComputationFailure::Invalid {
                    diagnostic: "invalid owning namespace".into(),
                    explanation,
                    searches,
                }
            } else {
                ComputationFailure::Incomplete {
                    reason: IncompleteReason::MissingInput,
                    explanation,
                    searches,
                }
            };
            let mut builder = DerivationBuilder::new(snapshot.clone());
            builder.failure(id(subject), property, failure).unwrap();
            let overlay = builder.build().unwrap();
            let q = KerMlQueries::new(
                SemanticContext::for_overlay(
                    &overlay,
                    SemanticOptions {
                        baseline_profile: P::OPERATIONAL_V9,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .unwrap(),
            );
            for result in [
                q.multiplicity_featuring_context(id(20)),
                q.featuring_types(id(2)),
            ] {
                assert_eq!(
                    result.completeness,
                    if invalid {
                        Completeness::Invalid
                    } else {
                        Completeness::Incomplete
                    }
                );
                assert!(result.value.is_empty());
                assert!(
                    result
                        .search_dependencies
                        .contains(&SearchDependency::Kernel(StructuralSearch::Incoming(id(
                            901
                        ))))
                );
            }
        }
    }
}

#[test]
fn incomparable_domains_do_not_select_a_context_by_element_id() {
    let mut f = fixture(P::OPERATIONAL_V9);
    member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
    f.create(90, c::CLASSIFIER);
    type_featuring(&mut f, 10, 90, 190);
    // Both incomparable domains specialize the referent's featuring domain.
    f.create(70, c::CLASSIFIER);
    f.owned
        .get_mut(&id(1))
        .unwrap()
        .retain(|v| *v != Value::Reference(id(104)));
    f.own(70, 104);
    relation(
        &mut f,
        1,
        70,
        170,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    relation(
        &mut f,
        90,
        70,
        179,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(170, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
    f.value(179, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(90)));
    let snapshot = f.finish();
    let q = queries(&snapshot, P::OPERATIONAL_V9);
    assert_eq!(
        q.multiplicity_featuring_context(id(20)).value,
        vec![id(1), id(90)]
    );
    let binding = q.reference_binding_context(id(2), id(4), id(3));
    assert_eq!(binding.value, None);
    assert_eq!(binding.completeness, Completeness::Incomplete);
    assert!(
        binding
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_AMBIGUOUS_CONNECTOR_CONTEXT")
    );
}

#[test]
fn nested_reference_expression_reaches_the_structural_bound_context() {
    let mut f = fixture(P::OPERATIONAL_V9);
    member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
    f.create(60, c::FEATURE_REFERENCE_EXPRESSION);
    f.create(61, c::FEATURE);
    member(&mut f, 2, 60, 160, c::PARAMETER_MEMBERSHIP);
    member(&mut f, 60, 61, 161, c::RETURN_PARAMETER_MEMBERSHIP);
    f.enumeration(61, p::FEATURE_DIRECTION, "out");
    relation(
        &mut f,
        60,
        4,
        164,
        c::MEMBERSHIP,
        p::MEMBERSHIP_MEMBER_ELEMENT,
    );
    let snapshot = f.finish();
    let q = queries(&snapshot, P::OPERATIONAL_V9);
    assert_eq!(q.featuring_types(id(60)).value, vec![id(2)]);
    let context = q.reference_binding_context(id(60), id(4), id(61));
    assert_eq!(
        context.completeness,
        Completeness::Complete,
        "{:?}",
        context.diagnostics
    );
    assert_eq!(context.value, Some(id(1)));
}

#[test]
fn conflicting_explicit_multiplicity_domain_is_invalid() {
    let mut f = fixture(P::OPERATIONAL_V9);
    member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
    f.create(90, c::CLASSIFIER);
    type_featuring(&mut f, 20, 90, 190);
    let snapshot = f.finish();
    let q = queries(&snapshot, P::OPERATIONAL_V9);
    for result in [
        q.multiplicity_featuring_context(id(20)),
        q.featuring_types(id(2)),
    ] {
        assert_eq!(result.completeness, Completeness::Invalid);
        assert_eq!(result.value, vec![id(1)]);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "KQ_MULTIPLICITY_FEATURING_CONFLICT")
        );
    }
}

#[test]
fn v9_symbolic_bound_closure_agrees_between_worklist_and_independent_full_scan() {
    let profile = P::OPERATIONAL_V9;
    let mut f = fixture(profile);
    member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
    let snapshot = f.finish();
    let run = |strategy, order, batch_size| {
        close_result_structure(
            &snapshot,
            PublicationClosureOptions {
                strategy,
                order,
                batch_size,
                ..Default::default()
            },
            |overlay| {
                SemanticContext::for_overlay(
                    overlay,
                    SemanticOptions {
                        baseline_profile: profile,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .map_err(PublicationOverlayError::Context)
            },
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap()
    };
    let expected = run(
        PublicationClosureStrategy::ReferenceFullScan,
        PublicationWorklistOrder::Fifo,
        32,
    );
    assert!(expected.converged);
    assert_eq!(
        expected.completeness,
        Completeness::Complete,
        "{:?}",
        expected.stages.last()
    );
    for (order, batch_size) in [
        (PublicationWorklistOrder::Fifo, 1),
        (PublicationWorklistOrder::Lifo, 7),
        (PublicationWorklistOrder::Partitioned, 32),
    ] {
        let actual = run(PublicationClosureStrategy::Worklist, order, batch_size);
        assert!(actual.converged);
        assert_eq!(
            actual.completeness,
            Completeness::Complete,
            "{:?}",
            actual.stages.last()
        );
        assert!(
            actual
                .overlay
                .model()
                .elements()
                .eq(expected.overlay.model().elements())
        );
        assert!(actual.overlay.facts().eq(expected.overlay.facts()));
        assert!(
            actual
                .overlay
                .model()
                .association_occurrences()
                .eq(expected.overlay.model().association_occurrences())
        );
    }
}
