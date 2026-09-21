include!("common/result_fixture.rs");
use agq_kerml::BaselineProfile as P;

fn options() -> SemanticOptions {
    SemanticOptions {
        baseline_profile: P::OPERATIONAL_V8,
        ..Default::default()
    }
}
fn fixture() -> Fixture {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(P::OPERATIONAL_V8).unwrap(),
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
    f
}

#[test]
fn binary_crossing_is_a_canonical_occurrence_with_transitive_derived_query_evidence() {
    let snapshot = fixture().finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    assert_eq!(q.cross_feature(id(10)).value, None);
    let result = q
        .plan_result_structure([id(10), id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(
        result.production.completeness,
        Completeness::Complete,
        "{:?}",
        result.production.diagnostics
    );
    let model = result.overlay.model();
    let expanded = KerMlQueries::new(
        SemanticContext::for_overlay(&result.overlay, options(), BTreeSet::new()).unwrap(),
    );
    let relationship = expanded.owned_cross_subsetting(id(10)).value.unwrap();
    let view = agq_kerml::views::CrossSubsetting::try_new(relationship, model).unwrap();
    assert_eq!(view.crossing_feature().unwrap(), id(10));
    assert_eq!(
        agq_kerml::views::Feature::try_new(id(10), model)
            .unwrap()
            .owned_cross_subsetting()
            .unwrap(),
        Some(relationship)
    );
    assert_eq!(
        agq_kerml::views::Specialization::try_new(relationship, model)
            .unwrap()
            .specific()
            .unwrap(),
        id(10)
    );
    assert_eq!(
        expanded.owning_related_element(relationship).value,
        Some(id(10))
    );
    let answer = expanded.cross_feature(id(10));
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{:?}",
        answer.diagnostics
    );
    assert_eq!(answer.value, Some(id(9)));
    let nav = model
        .navigation_slot(relationship, p::CROSS_SUBSETTING_CROSSED_FEATURE)
        .unwrap();
    let Origin::AssociationOccurrences(links) = nav.origin() else {
        panic!("canonical occurrence projection")
    };
    assert_eq!(links.len(), 1);
    let link_id = *links.first().unwrap();
    let link = model.association_occurrence(link_id).unwrap();
    assert_eq!(
        link.association(),
        model
            .registry()
            .property(p::CROSS_SUBSETTING_CROSSED_FEATURE)
            .unwrap()
            .association
            .unwrap()
    );
    assert!(matches!(link.origin(), Origin::Derived(_)));
    assert!(
        result
            .overlay
            .explain(FactKey::AssociationOccurrence(link_id))
            .is_some()
    );
    assert!(matches!(
        answer.fact_origins[&FactKey::AssociationOccurrence(link_id)].as_ref(),
        Origin::Derived(_)
    ));
    assert!(
        answer
            .positive_dependencies
            .contains(&FactKey::Element(id(11)))
    );
    assert!(
        answer
            .search_dependencies
            .contains(&SearchDependency::Kernel(
                agq_kernel::derived::StructuralSearch::Model
            ))
    );
    assert!(answer.positive_dependencies.contains(&FactKey::Property {
        element: id(10),
        property: p::ELEMENT_OWNED_RELATIONSHIP
    }));
    let Value::Reference(chain) = nav.value().values().next().unwrap() else {
        panic!("chain")
    };
    assert_eq!(
        agq_kerml::views::Specialization::try_new(relationship, model)
            .unwrap()
            .general()
            .unwrap(),
        *chain
    );
    assert!(matches!(
        model
            .navigation_slot(relationship, p::CROSS_SUBSETTING_CROSSING_FEATURE)
            .unwrap()
            .origin(),
        Origin::AssociationOccurrences(_)
    ));
    assert!(
        model
            .element(relationship)
            .unwrap()
            .slot(p::CROSS_SUBSETTING_CROSSING_FEATURE)
            .is_none()
    );
    assert_eq!(
        expanded.chaining_features(*chain).value,
        vec![id(11), id(9)]
    );
    assert_eq!(expanded.owner(*chain).value, Some(id(10)));
    assert_eq!(
        expanded.direct_specializations(id(10)).value,
        vec![*chain, id(20)]
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    );
    assert_eq!(expanded.feature_types(id(9)).value, vec![id(20)]);
    assert_eq!(expanded.featuring_types(id(9)).value, vec![id(21)]);
    assert!(
        model
            .element(relationship)
            .unwrap()
            .slot(p::CROSS_SUBSETTING_CROSSED_FEATURE)
            .is_none()
    );
    assert!(
        model
            .incoming(*chain)
            .any(|r| r.carrier == ReferenceCarrier::AssociationOccurrence(link_id))
    );
    assert_eq!(snapshot.model().association_occurrences().count(), 0);
    assert!(snapshot.model().element(relationship).is_none());
    assert_eq!(model.derived_navigation_results().count(), 0);
    // Profile identity is part of the rule and is checked even on occurrence provenance.
    assert!(
        SemanticContext::for_overlay(
            &result.overlay,
            SemanticOptions {
                baseline_profile: P::OPERATIONAL_V7,
                ..Default::default()
            },
            BTreeSet::new()
        )
        .is_err()
    );
}

#[test]
fn crossing_element_and_occurrence_identities_are_stable_across_batches_and_rebuilds() {
    let snapshot = fixture().finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    let a = q
        .plan_result_structure([id(10), id(9)])
        .materialize(&snapshot)
        .unwrap();
    let mut batch = q.plan_result_structure([id(9)]);
    batch.merge(q.plan_result_structure([id(10)])).unwrap();
    let b = batch.materialize(&snapshot).unwrap();
    assert_eq!(
        format!("{:?}", a.overlay.model()),
        format!("{:?}", b.overlay.model())
    );
}

#[test]
fn conflicting_authored_crossing_is_reported_and_never_overwritten() {
    let mut f = fixture();
    f.create(60, c::FEATURE);
    f.create(61, c::FEATURE);
    relation(
        &mut f,
        60,
        11,
        62,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    relation(
        &mut f,
        60,
        61,
        63,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    relation(
        &mut f,
        10,
        60,
        64,
        c::CROSS_SUBSETTING,
        p::CROSS_SUBSETTING_CROSSED_FEATURE,
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    let result = q
        .plan_result_structure([id(10)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(result.production.completeness, Completeness::Invalid);
    assert!(
        result
            .production
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_CROSSING_CONFLICT")
    );
    assert_eq!(
        result.overlay.model().element(id(64)),
        snapshot.model().element(id(64))
    );
    assert_eq!(result.overlay.model().association_occurrences().count(), 1);
}

#[test]
fn later_domain_stage_retains_crossing_identity_and_can_be_repeated() {
    let snapshot = fixture().finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    let first = q
        .plan_result_structure([id(10)])
        .materialize(&snapshot)
        .unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&first.overlay, options(), BTreeSet::new()).unwrap(),
    );
    let second = q
        .plan_result_structure([id(9), id(10)])
        .materialize_on_overlay(&first.overlay)
        .unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&second.overlay, options(), BTreeSet::new()).unwrap(),
    );
    let third = q
        .plan_result_structure([id(9), id(10)])
        .materialize_on_overlay(&second.overlay)
        .unwrap();
    assert_eq!(
        format!("{:?}", second.overlay.model()),
        format!("{:?}", third.overlay.model())
    );
    assert_eq!(second.production.completeness, Completeness::Complete);
    assert_eq!(
        first
            .overlay
            .model()
            .association_occurrences()
            .collect::<Vec<_>>(),
        second
            .overlay
            .model()
            .association_occurrences()
            .collect::<Vec<_>>()
    );
}

#[test]
fn nary_crossing_preserves_all_other_ends_in_the_ordered_product() {
    let mut f = fixture();
    for (end, ty, membership, typing) in [(12, 22, 32, 42), (13, 23, 33, 43)] {
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
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    let result = q
        .plan_result_structure([id(10), id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(
        result.production.completeness,
        Completeness::Complete,
        "{:?}",
        result.production.diagnostics
    );
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&result.overlay, options(), BTreeSet::new()).unwrap(),
    );
    assert_eq!(q.cross_feature(id(10)).value, Some(id(9)));
    let relationship = q.owned_cross_subsetting(id(10)).value.unwrap();
    let model = result.overlay.model();
    let Value::Reference(chain) = model
        .navigation_slot(relationship, p::CROSS_SUBSETTING_CROSSED_FEATURE)
        .unwrap()
        .value()
        .values()
        .next()
        .unwrap()
    else {
        panic!("chain")
    };
    let sequence = q.chaining_features(*chain);
    assert_eq!(sequence.value.len(), 2);
    assert_eq!(sequence.value[1], id(9));
    let outer = sequence.value[0];
    assert_eq!(q.direct_feature_types(outer).value, vec![id(13)]);
    let inner = q.featuring_types(outer).value[0];
    assert_eq!(q.owner(inner).value, Some(outer));
    assert_eq!(q.direct_feature_types(inner).value, vec![id(12)]);
    assert_eq!(q.featuring_types(inner).value, vec![id(11)]);
    let domains = q.featuring_types(id(9));
    assert_eq!(
        domains.completeness,
        Completeness::Complete,
        "{:?}",
        domains.diagnostics
    );
    assert_eq!(domains.value.len(), 1);
    assert_eq!(q.feature_types(domains.value[0]).value, vec![id(23)]);
}

#[test]
fn redefined_end_crossing_is_available_to_the_dependent_cross_domain_stage() {
    let mut f = fixture();
    f.create(100, c::ASSOCIATION);
    for (end, ty, membership, typing) in [(110, 20, 130, 140), (111, 21, 131, 141)] {
        f.create(end, c::FEATURE);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 100, end, membership, c::END_FEATURE_MEMBERSHIP);
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
    f.create(109, c::FEATURE);
    member(&mut f, 110, 109, 150, c::OWNING_MEMBERSHIP);
    redefine(&mut f, 10, 110, 160);
    let snapshot = f.finish();
    let subjects = [id(10), id(110), id(9), id(109)];
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap(),
    );
    let first = q
        .plan_result_structure(subjects)
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(first.production.completeness, Completeness::Incomplete);
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&first.overlay, options(), BTreeSet::new()).unwrap(),
    );
    let second = q
        .plan_result_structure(subjects)
        .materialize_on_overlay(&first.overlay)
        .unwrap();
    assert_eq!(
        second.production.completeness,
        Completeness::Complete,
        "{:?}",
        second.production.diagnostics
    );
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&second.overlay, options(), BTreeSet::new()).unwrap(),
    );
    assert_eq!(q.cross_feature(id(110)).value, Some(id(109)));
    assert!(q.subsetted_features(id(9)).value.contains(&id(109)));
    assert_eq!(
        q.featuring_types(id(9)).completeness,
        Completeness::Complete
    );
}
