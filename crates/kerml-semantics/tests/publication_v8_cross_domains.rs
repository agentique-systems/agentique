include!("common/result_fixture.rs");
use agq_kerml::BaselineProfile as P;

fn fixture(profile: P, end_count: usize) -> Snapshot {
    construction(profile, end_count).finish()
}

fn construction(profile: P, end_count: usize) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::ASSOCIATION);
    f.create(9, c::FEATURE);
    f.value(
        9,
        p::ELEMENT_DECLARED_NAME,
        Value::String("arbitraryReturn".into()),
    );
    for i in 0..end_count as u128 {
        let end = 10 + i;
        let ty = 20 + i;
        f.create(end, c::FEATURE);
        f.value(
            end,
            p::ELEMENT_DECLARED_NAME,
            Value::String(format!("arbitraryEnd{i}")),
        );
        f.create(ty, c::CLASSIFIER);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, end, 30 + i, c::END_FEATURE_MEMBERSHIP);
        relation(
            &mut f,
            end,
            ty,
            40 + i,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            40 + i,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(end)),
        );
    }
    member(&mut f, 10, 9, 50, c::OWNING_MEMBERSHIP);
    f
}
fn context(snapshot: &Snapshot, profile: P) -> SemanticContext<'_> {
    SemanticContext::for_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap()
}

#[test]
fn profile_boundary_preserves_selector_and_excludes_owning_end_only_in_v8() {
    for profile in [P::PublishedKerMl10, P::OPERATIONAL_V7, P::OPERATIONAL_V8] {
        let snapshot = fixture(profile, 2);
        let q = KerMlQueries::new(context(&snapshot, profile));
        assert_eq!(q.owned_cross_feature(id(10)).value, Some(id(9)));
        let domain = q.owned_cross_feature_domain(id(9));
        assert_eq!(
            domain.completeness,
            Completeness::Complete,
            "{:?}",
            domain.diagnostics
        );
        let domain = domain.value.unwrap();
        assert_eq!(domain.owning_end, id(10));
        assert_eq!(domain.owning_type, id(1));
        assert_eq!(
            domain.factors.iter().map(|f| f.end).collect::<Vec<_>>(),
            if profile.corrects_owned_cross_domain() {
                vec![id(11)]
            } else {
                vec![id(10), id(11)]
            }
        );
        if profile.corrects_owned_cross_domain() {
            assert_eq!(q.featuring_types(id(9)).value, vec![id(21)]);
            assert!(q.context().owned_cross_domain_manifest_digest.is_some());
        }
    }
}

#[test]
fn nary_domain_materializes_nested_cartesian_features_without_owning_end_factor() {
    let profile = P::OPERATIONAL_V8;
    let snapshot = fixture(profile, 4);
    let q = KerMlQueries::new(context(&snapshot, profile));
    assert_eq!(
        q.featuring_types(id(9)).completeness,
        Completeness::Incomplete
    );
    let derived = q
        .plan_result_structure([id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(
        derived.production.completeness,
        Completeness::Complete,
        "{:?}",
        derived.production.diagnostics
    );
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(
            &derived.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let domains = q.featuring_types(id(9));
    assert_eq!(
        domains.completeness,
        Completeness::Complete,
        "{:?}",
        domains.diagnostics
    );
    assert_eq!(domains.value.len(), 1);
    let outer = domains.value[0];
    assert_eq!(q.owner(outer).value, Some(id(9)));
    assert_eq!(q.feature_types(outer).value, vec![id(23)]);
    let inner = q.featuring_types(outer).value[0];
    assert_eq!(q.owner(inner).value, Some(outer));
    assert_eq!(q.feature_types(inner).value, vec![id(22)]);
    assert_eq!(q.featuring_types(inner).value, vec![id(21)]);
    assert_eq!(q.feature_types(id(9)).value, vec![id(20)]);
    assert_eq!(q.owned_cross_feature(id(10)).value, Some(id(9)));
    assert!(snapshot.model().element(outer).is_none());
    assert!(derived.overlay.explain(FactKey::Element(outer)).is_some());
}

#[test]
fn binary_domain_production_keeps_opposite_types_and_provenance() {
    let profile = P::OPERATIONAL_V8;
    let snapshot = fixture(profile, 2);
    let q = KerMlQueries::new(context(&snapshot, profile));
    let derived = q
        .plan_result_structure([id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(derived.production.completeness, Completeness::Complete);
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(
            &derived.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let domains = q.featuring_types(id(9));
    assert_eq!(domains.completeness, Completeness::Complete);
    assert_eq!(domains.value, vec![id(21)]);
    assert!(
        domains
            .fact_origins
            .values()
            .any(|o| matches!(o.as_ref(), Origin::Derived(_)))
    );
}

#[test]
fn authored_required_featuring_and_typing_keep_their_relationship_identities() {
    let profile = P::OPERATIONAL_V8;
    let mut f = construction(profile, 2);
    type_featuring(&mut f, 9, 21, 80);
    relation(&mut f, 9, 20, 81, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(81, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(9)));
    let snapshot = f.finish();
    let q = KerMlQueries::new(context(&snapshot, profile));
    let plan = q.plan_result_structure([id(9)]);
    assert_eq!(plan.production.completeness, Completeness::Complete);
    assert_eq!(plan.planned_elements().count(), 0);
    let derived = plan.materialize(&snapshot).unwrap();
    assert_eq!(
        derived.overlay.model().element(id(80)),
        snapshot.model().element(id(80))
    );
    assert_eq!(
        derived.overlay.model().element(id(81)),
        snapshot.model().element(id(81))
    );
}

#[test]
fn known_unary_end_population_is_determinate_without_inventing_an_opposite_end() {
    let profile = P::OPERATIONAL_V8;
    let snapshot = fixture(profile, 1);
    let q = KerMlQueries::new(context(&snapshot, profile));
    let domain = q.owned_cross_feature_domain(id(9));
    assert_eq!(domain.completeness, Completeness::Complete);
    assert!(domain.value.unwrap().factors.is_empty());
    let featuring = q.featuring_types(id(9));
    assert_eq!(featuring.completeness, Completeness::Complete);
    assert!(featuring.value.is_empty());
    let result = q
        .plan_result_structure([id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(result.production.completeness, Completeness::Complete);
    assert!(
        !result
            .overlay
            .model()
            .elements()
            .any(|e| e.metaclass() == c::TYPE_FEATURING)
    );
    // The separate conformance authority record continues to carry KERML11-2.
    // This fixture proves structural determinacy, not valid cross cardinality.
}

#[test]
fn missing_inherited_crossing_cannot_be_reported_as_complete_domain_production() {
    let profile = P::OPERATIONAL_V8;
    let mut f = construction(profile, 2);
    f.create(100, c::ASSOCIATION);
    f.create(110, c::FEATURE);
    f.value(110, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 100, 110, 130, c::END_FEATURE_MEMBERSHIP);
    f.create(111, c::FEATURE);
    f.value(111, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 100, 111, 131, c::END_FEATURE_MEMBERSHIP);
    f.create(109, c::FEATURE);
    member(&mut f, 110, 109, 150, c::OWNING_MEMBERSHIP);
    redefine(&mut f, 10, 110, 160);
    let snapshot = f.finish();
    let q = KerMlQueries::new(context(&snapshot, profile));
    assert_eq!(q.owned_cross_feature(id(110)).value, Some(id(109)));
    assert_eq!(q.cross_feature(id(110)).value, None);
    let domain = q.owned_cross_feature_domain(id(9));
    assert_eq!(domain.completeness, Completeness::Incomplete);
    assert!(
        domain
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_INHERITED_CROSS_STRUCTURE")
    );
    let derived = q.plan_result_structure([id(9)]);
    assert_eq!(derived.production.completeness, Completeness::Incomplete);
    assert_eq!(derived.planned_elements().count(), 0);
}

#[test]
fn every_nary_factor_uses_all_effective_types_as_an_intersection() {
    let profile = P::OPERATIONAL_V8;
    let mut f = construction(profile, 3);
    f.create(90, c::CLASSIFIER);
    relation(
        &mut f,
        11,
        90,
        91,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        91,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(11)),
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(context(&snapshot, profile));
    let derived = q
        .plan_result_structure([id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(derived.production.completeness, Completeness::Complete);
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(
            &derived.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let product = q.featuring_types(id(9)).value[0];
    assert_eq!(q.feature_types(product).value, vec![id(22)]);
    let intersection = q.featuring_types(product).value[0];
    let relationships = q.owned_relationships(intersection).value;
    let factors: BTreeSet<_> = relationships
        .into_iter()
        .filter_map(|r| {
            let record = derived.overlay.model().element(r).unwrap();
            (record.metaclass() == c::INTERSECTING).then(|| {
                let Value::Reference(target) = record
                    .slot(p::INTERSECTING_INTERSECTING_TYPE)
                    .unwrap()
                    .value()
                    .values()
                    .next()
                    .unwrap()
                else {
                    panic!("intersection endpoint")
                };
                *target
            })
        })
        .collect();
    assert_eq!(factors, BTreeSet::from([id(21), id(90)]));
}

#[test]
fn nary_domain_preserves_existing_inherited_cross_and_cartesian_specializations() {
    let profile = P::OPERATIONAL_V8;
    let mut f = construction(profile, 3);
    f.create(100, c::ASSOCIATION);
    for n in [109, 110, 111, 112, 120, 195] {
        f.create(n, c::FEATURE);
    }
    for n in [122, 123] {
        f.create(n, c::CLASSIFIER);
    }
    for (end, ty, m, typing) in [
        (110, 20, 130, 140),
        (111, 123, 131, 141),
        (112, 122, 132, 142),
    ] {
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 100, end, m, c::END_FEATURE_MEMBERSHIP);
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
    member(&mut f, 110, 109, 150, c::OWNING_MEMBERSHIP);
    member(&mut f, 109, 120, 151, c::OWNING_MEMBERSHIP);
    type_featuring(&mut f, 109, 120, 152);
    type_featuring(&mut f, 120, 123, 153);
    relation(
        &mut f,
        120,
        122,
        154,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        154,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(120)),
    );
    for (specific, general, r) in [(21, 123, 161), (22, 122, 162)] {
        relation(
            &mut f,
            specific,
            general,
            r,
            c::SUBCLASSIFICATION,
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
        f.value(
            r,
            p::SUBCLASSIFICATION_SUBCLASSIFIER,
            Value::Reference(id(specific)),
        );
    }
    redefine(&mut f, 10, 110, 160);
    relation(
        &mut f,
        110,
        195,
        180,
        c::CROSS_SUBSETTING,
        p::CROSS_SUBSETTING_CROSSED_FEATURE,
    );
    for (target, r) in [(111, 181), (109, 182)] {
        relation(
            &mut f,
            195,
            target,
            r,
            c::FEATURE_CHAINING,
            p::FEATURE_CHAINING_CHAINING_FEATURE,
        );
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(context(&snapshot, profile));
    assert_eq!(
        q.owned_cross_feature_domain(id(9))
            .value
            .unwrap()
            .inherited_cross_features,
        vec![id(109)]
    );
    let derived = q
        .plan_result_structure([id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(
        derived.production.completeness,
        Completeness::Complete,
        "{:?}",
        derived.production.diagnostics
    );
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(
            &derived.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    assert!(q.supertypes(id(9)).value.contains(&id(109)));
    let product = q.featuring_types(id(9)).value[0];
    assert!(q.supertypes(product).value.contains(&id(120)));
    assert_eq!(q.feature_types(product).value, vec![id(22)]);
}

#[test]
fn equal_factor_types_do_not_collapse_distinct_nary_end_positions() {
    let profile = P::OPERATIONAL_V8;
    let mut f = construction(profile, 4);
    for typing in [42, 43] {
        f.value(typing, p::FEATURE_TYPING_TYPE, Value::Reference(id(21)));
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(context(&snapshot, profile));
    let domain = q.owned_cross_feature_domain(id(9)).value.unwrap();
    assert_eq!(domain.factors.len(), 3);
    assert!(
        domain
            .factors
            .iter()
            .all(|factor| factor.types == vec![id(21)])
    );
    let derived = q
        .plan_result_structure([id(9)])
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(derived.production.completeness, Completeness::Complete);
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(
            &derived.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let outer = q.featuring_types(id(9)).value[0];
    let inner = q.featuring_types(outer).value[0];
    assert_ne!(outer, inner);
    assert_ne!(inner, id(21));
    assert_eq!(q.feature_types(outer).value, vec![id(21)]);
    assert_eq!(q.feature_types(inner).value, vec![id(21)]);
    assert_eq!(q.featuring_types(inner).value, vec![id(21)]);
}
