include!("common/result_fixture.rs");

#[test]
fn structural_bounds_use_owned_expression_order_without_evaluation() {
    for count in 0..=3 {
        let mut f = Fixture::new();
        f.create(1, c::MULTIPLICITY_RANGE);
        for (index, n) in [90, 40, 10].into_iter().take(count).enumerate() {
            f.create(n, c::EXPRESSION);
            member(&mut f, 1, n, 100 + index as u128, c::OWNING_MEMBERSHIP);
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        let bounds = q.multiplicity_bounds(id(1));
        assert_eq!(bounds.completeness, Completeness::Complete);
        let expected = match count {
            0 => vec![],
            1 => vec![id(90)],
            _ => vec![id(90), id(40)],
        };
        assert_eq!(bounds.value.bound, expected);
        assert_eq!(bounds.value.lower, (count >= 2).then_some(id(90)));
        assert_eq!(bounds.value.upper, expected.last().copied());
        // The KERML11-4 validation conflict does not change canonical ownership/domain.
        assert_eq!(q.owning_type(id(1)).value, None);
        assert!(q.featuring_types(id(1)).value.is_empty());
    }
}

/// Shape of Transfers::Transfer::instant[instantNum], without library names or
/// source identities. The retained KERML11-4 decision in
/// verification/kerml-publication-convergence/authority-blockers.json fixes an
/// empty multiplicity domain when its membership is merely OwningMembership.
/// The v6 reference-binding correction requires a featuring context; it does
/// not authorize replacing that empty domain with the lexical owning Type.
#[test]
fn bound_reference_to_outer_feature_retains_the_unresolved_context_obligation() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (element, class) in [
        (1, c::CLASS),
        (2, c::FEATURE_REFERENCE_EXPRESSION),
        (3, c::FEATURE),
        (4, c::FEATURE),
        (10, c::FEATURE),
        (20, c::MULTIPLICITY_RANGE),
    ] {
        f.create(element, class);
    }
    member(&mut f, 1, 4, 103, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 10, 110, c::FEATURE_MEMBERSHIP);
    member(&mut f, 10, 20, 120, c::OWNING_MEMBERSHIP);
    member(&mut f, 20, 2, 122, c::OWNING_MEMBERSHIP);
    member(&mut f, 2, 3, 102, c::RETURN_PARAMETER_MEMBERSHIP);
    f.enumeration(3, p::FEATURE_DIRECTION, "out");
    relation(
        &mut f,
        2,
        4,
        105,
        c::MEMBERSHIP,
        p::MEMBERSHIP_MEMBER_ELEMENT,
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    assert_eq!(q.owning_type(id(20)).value, None);
    assert_eq!(q.owner(id(20)).value, Some(id(10)));
    assert_eq!(q.featuring_types(id(4)).value, vec![id(1)]);
    assert_eq!(q.featuring_types(id(10)).value, vec![id(1)]);
    for element in [id(20), id(2)] {
        let domain = q.featuring_types(element);
        assert_eq!(domain.completeness, Completeness::Complete);
        assert!(domain.value.is_empty());
    }
    let bounds = q.multiplicity_bounds(id(20));
    assert_eq!(bounds.completeness, Completeness::Complete);
    assert_eq!(bounds.value.bound, vec![id(2)]);
    let reference = q.reference_binding_context(id(2), id(4), id(3));
    assert_eq!(reference.value, None);
    assert_eq!(reference.completeness, Completeness::Incomplete);
    assert!(
        reference
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_REFERENCE_CONTEXT")
    );

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
    for (order, batch_size) in [
        (PublicationWorklistOrder::Fifo, 1),
        (PublicationWorklistOrder::Lifo, 7),
        (PublicationWorklistOrder::Partitioned, 32),
    ] {
        let actual = run(PublicationClosureStrategy::Worklist, order, batch_size);
        assert!(actual.converged);
        assert_eq!(actual.completeness, Completeness::Incomplete);
        assert_eq!(actual.completeness, expected.completeness);
        assert!(
            actual
                .stages
                .last()
                .unwrap()
                .diagnostics
                .iter()
                .any(|d| { d.subject == id(2) && d.code == "KQ_REFERENCE_CONTEXT" })
        );
        assert_eq!(
            actual.stages.last().unwrap().diagnostics,
            expected.stages.last().unwrap().diagnostics
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
        assert!(
            actual
                .overlay
                .model()
                .computation_searches()
                .eq(expected.overlay.model().computation_searches())
        );
        let q = overlay_queries(&actual.overlay, profile);
        assert_eq!(
            q.context(),
            overlay_queries(&expected.overlay, profile).context()
        );
        let final_context = q.reference_binding_context(id(2), id(4), id(3));
        assert_eq!(final_context.value, None);
        assert_eq!(final_context.completeness, Completeness::Incomplete);
        assert!(
            final_context
                .diagnostics
                .iter()
                .any(|d| { d.subject == id(2) && d.code == "KQ_REFERENCE_CONTEXT" })
        );
        assert_eq!(
            final_context,
            overlay_queries(&expected.overlay, profile).reference_binding_context(
                id(2),
                id(4),
                id(3)
            )
        );
    }
}

fn overlay_queries(
    overlay: &agq_kernel::derived::DerivedOverlay,
    profile: agq_kerml::BaselineProfile,
) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_overlay(
            overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    )
}
