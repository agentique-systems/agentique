//! Profile boundary and semantic-order regression for KERML11-1.
include!("common/result_fixture.rs");
use agq_kerml::BaselineProfile as P;

const PROFILES: [P; 8] = [
    P::PublishedKerMl10,
    P::OPERATIONAL_V1,
    P::OPERATIONAL_V2,
    P::OPERATIONAL_V3,
    P::OPERATIONAL_V4,
    P::OPERATIONAL_V5,
    P::OPERATIONAL_V6,
    P::OPERATIONAL_V7,
];

fn builder(profile: P) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::ASSOCIATION);
    f.create(2, c::FEATURE);
    f.value(2, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 1, 2, 102, c::END_FEATURE_MEMBERSHIP);
    f
}
fn options(profile: P) -> SemanticOptions {
    SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    }
}

#[test]
fn owned_cross_feature_eight_profile_matrix_uses_membership_order_and_metaclass_conformance() {
    for profile in PROFILES {
        for case in [
            "no-owner",
            "non-end",
            "binding-first",
            "feature-value-first",
            "multiplicity-first",
            "metadata-first",
            "feature-membership-first",
            "all-exclusions",
            "valid-first",
            "two-valid",
            "inherited",
            "authored-connector",
            "implied-connector",
            "contextual-chain",
            "correction-origin",
        ] {
            let mut f = builder(profile);
            if case == "no-owner" {
                f.owned.remove(&id(1));
            }
            if case == "non-end" {
                f.value(2, p::FEATURE_IS_END, Value::Boolean(false));
            }
            let exclusions: Vec<_> = match case {
                "binding-first" | "authored-connector" | "implied-connector" => {
                    vec![(c::BINDING_CONNECTOR, c::OWNING_MEMBERSHIP)]
                }
                "feature-value-first" => vec![(c::EXPRESSION, c::FEATURE_VALUE)],
                "multiplicity-first" => vec![(c::MULTIPLICITY_RANGE, c::OWNING_MEMBERSHIP)],
                "metadata-first" => vec![(c::METADATA_FEATURE, c::OWNING_MEMBERSHIP)],
                "feature-membership-first" => vec![(c::FEATURE, c::END_FEATURE_MEMBERSHIP)],
                "all-exclusions" => vec![
                    (c::MULTIPLICITY_RANGE, c::OWNING_MEMBERSHIP),
                    (c::METADATA_FEATURE, c::OWNING_MEMBERSHIP),
                    (c::FEATURE, c::FEATURE_MEMBERSHIP),
                    (c::EXPRESSION, c::FEATURE_VALUE),
                    (c::BINDING_CONNECTOR, c::OWNING_MEMBERSHIP),
                ],
                _ => vec![],
            };
            for (i, (class, membership)) in exclusions.iter().copied().enumerate() {
                let n = 40 + i as u128;
                f.create(n, class);
                member(&mut f, 2, n, 140 + i as u128, membership);
                if case == "implied-connector" {
                    // Relationship flag belongs to membership, not Feature.
                    f.value(140, p::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true));
                }
            }
            // The first semantic candidate has a deliberately larger identity.
            f.create(900, c::FEATURE);
            f.create(3, c::FEATURE);
            member(&mut f, 2, 900, 990, c::OWNING_MEMBERSHIP);
            member(&mut f, 2, 3, 103, c::OWNING_MEMBERSHIP);
            if case == "inherited" {
                f.create(8, c::FEATURE);
                f.create(7, c::FEATURE);
                member(&mut f, 8, 7, 107, c::OWNING_MEMBERSHIP);
                subset(&mut f, 2, 8, 108);
            }
            if case == "contextual-chain" {
                relation(
                    &mut f,
                    900,
                    3,
                    110,
                    c::FEATURE_CHAINING,
                    p::FEATURE_CHAINING_CHAINING_FEATURE,
                );
            }
            if case == "correction-origin" {
                // Selection does not use display names or provenance heuristics.
                f.changes.set(
                    id(900),
                    p::ELEMENT_DECLARED_NAME,
                    SlotValue::Scalar(Value::String("reviewed".into())),
                    DeclaredOrigin::ReviewedCorrection {
                        profile: profile.id().into(),
                        entry: "matrix".into(),
                        authority: BTreeSet::from(["KERML11-1".into()]),
                        library: LibraryId::from_u128(1),
                        source_key: "synthetic".into(),
                        output_key: "cross".into(),
                    },
                );
            }
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(&snapshot, options(profile), Default::default())
                    .unwrap(),
            );
            let answer = q.owned_cross_feature(id(2));
            assert_eq!(
                answer.completeness,
                Completeness::Complete,
                "{profile:?} {case}: {:?}",
                answer.diagnostics
            );
            let expected = if matches!(case, "no-owner" | "non-end") {
                None
            } else if !profile.corrects_owned_cross_feature() {
                exclusions
                    .iter()
                    .enumerate()
                    .find(|(_, (c, m))| *c == c::BINDING_CONNECTOR || *m == c::FEATURE_VALUE)
                    .map(|(i, _)| id(40 + i as u128))
                    .or(Some(id(900)))
            } else {
                Some(id(900))
            };
            assert_eq!(answer.value, expected, "{profile:?} {case}");
            if let Some(expected) = expected {
                assert!(answer.positive_dependencies.contains(&FactKey::Property {
                    element: id(2),
                    property: p::ELEMENT_OWNED_RELATIONSHIP
                }));
                assert!(q.is_owned_cross_feature(expected).value);
            }
            let sequence=q.memberships(id(2)).value.into_iter().map(|m|{
                let target=q.member(m).value.unwrap();
                serde_json::json!({"membership":m.to_string(),"membership_metaclass":snapshot.model().registry().class(snapshot.model().element(m).unwrap().metaclass()).unwrap().name,
                    "member":target.to_string(),"member_metaclass":snapshot.model().registry().class(snapshot.model().element(target).unwrap().metaclass()).unwrap().name})
            }).collect::<Vec<_>>();
            println!(
                "MATRIX {}",
                serde_json::json!({"profile":profile.id(),"case":case,"is_end":case!="non-end","owning_type":case!="no-owner","sequence":sequence,"selected":answer.value.map(|v|v.to_string())})
            );
        }
    }
}

#[test]
fn incomplete_owned_membership_population_never_selects_a_candidate() {
    let mut f = builder(P::OPERATIONAL_V7);
    for (n, m) in [(900, 990), (3, 103)] {
        f.create(n, c::FEATURE);
        member(&mut f, 2, n, m, c::OWNING_MEMBERSHIP);
    }
    let snapshot = f.finish();
    let context = SemanticContext::for_project_snapshot(
        &snapshot,
        options(P::OPERATIONAL_V7),
        Default::default(),
        BTreeSet::new(),
        BTreeSet::from([id(2)]),
    )
    .unwrap();
    let answer = KerMlQueries::new(context).owned_cross_feature(id(2));
    assert_eq!(answer.completeness, Completeness::Incomplete);
    assert_eq!(answer.value, None);
}

#[test]
fn immutable_readers_and_batch_partitions_preserve_complete_cross_answers() {
    let mut f = builder(P::OPERATIONAL_V7);
    for (n, m) in [(900, 990), (3, 103)] {
        f.create(n, c::FEATURE);
        member(&mut f, 2, n, m, c::OWNING_MEMBERSHIP);
    }
    let snapshot = f.finish();
    let context =
        SemanticContext::for_snapshot(&snapshot, options(P::OPERATIONAL_V7), Default::default())
            .unwrap();
    let q = KerMlQueries::new(context.fork());
    let expected = q.owned_cross_feature(id(2));
    std::thread::scope(|scope| {
        let shared = &q;
        let readers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(move || {
                    (0..64)
                        .map(|_| shared.owned_cross_feature(id(2)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for reader in readers {
            for actual in reader.join().unwrap() {
                assert_eq!(actual, expected);
            }
        }
    });
    for size in [1, 3, 8, 64] {
        for batch in [id(2); 64].chunks(size) {
            let partition = KerMlQueries::new(context.fork());
            assert_eq!(partition.context(), q.context());
            for &subject in batch {
                assert_eq!(partition.owned_cross_feature(subject), expected);
            }
        }
    }
}

#[test]
fn no_eligible_member_means_none_and_cross_derivation_does_not_manufacture_one() {
    let mut f = builder(P::OPERATIONAL_V7);
    f.create(3, c::EXPRESSION);
    member(&mut f, 2, 3, 103, c::FEATURE_VALUE);
    f.create(4, c::BINDING_CONNECTOR);
    member(&mut f, 2, 4, 104, c::OWNING_MEMBERSHIP);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(P::OPERATIONAL_V7), Default::default())
            .unwrap(),
    );
    for answer in [
        q.owned_cross_feature(id(2)),
        q.owned_cross_subsetting(id(2)),
        q.cross_feature(id(2)),
    ] {
        assert_eq!(answer.completeness, Completeness::Complete);
        assert_eq!(answer.value, None);
    }
}

#[test]
fn produced_value_infrastructure_does_not_become_an_owned_cross_feature() {
    let profile = P::OPERATIONAL_V7;
    let mut f = builder(profile);
    f.create(3, c::FEATURE_REFERENCE_EXPRESSION);
    f.create(4, c::FEATURE);
    f.create(5, c::FEATURE);
    member(&mut f, 2, 3, 103, c::FEATURE_VALUE);
    member(&mut f, 3, 4, 104, c::RETURN_PARAMETER_MEMBERSHIP);
    f.enumeration(4, p::FEATURE_DIRECTION, "out");
    member(&mut f, 1, 5, 105, c::FEATURE_MEMBERSHIP);
    relation(
        &mut f,
        3,
        5,
        106,
        c::MEMBERSHIP,
        p::MEMBERSHIP_MEMBER_ELEMENT,
    );
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(profile), Default::default()).unwrap(),
    );
    let result = q.derive_result_structure(&snapshot).unwrap();
    assert_eq!(
        result.production.completeness,
        Completeness::Complete,
        "{:?}",
        result.production.diagnostics
    );
    assert!(!result.contextual_results.is_empty());
    let derived = KerMlQueries::new(
        SemanticContext::for_overlay(&result.overlay, options(profile), Default::default())
            .unwrap(),
    );
    assert!(
        result
            .production
            .value
            .iter()
            .any(|&binding| derived.implied_binding_role(binding)
                == Some(ImpliedBindingRole::FeatureValue))
    );
    assert!(
        result
            .production
            .value
            .iter()
            .any(|&binding| derived.implied_binding_role(binding)
                == Some(ImpliedBindingRole::FeatureReferenceResult))
    );
    let answer = derived.owned_cross_feature(id(2));
    assert_eq!(answer.value, None);
    assert!(
        answer
            .fact_origins
            .values()
            .any(|o| matches!(o.as_ref(), Origin::Derived(_)))
    );
    let again = derived.owned_cross_feature(id(2));
    assert_eq!(answer, again);
    for (fact, origin) in &answer.fact_origins {
        assert!(Arc::ptr_eq(origin, &again.fact_origins[fact]));
        // Shared allocation changes no origin content or public debug encoding.
        assert_eq!(format!("{origin:?}"), format!("{:?}", origin.as_ref()));
        if let Origin::Derived(explanation) = origin.as_ref() {
            for dependency in &explanation.dependencies {
                let (Dependency::Declared(key) | Dependency::Derived(key)) = dependency;
                assert!(answer.positive_dependencies.contains(key));
                assert!(answer.fact_origins.contains_key(key));
            }
        }
    }
    let independent = KerMlQueries::new(
        SemanticContext::for_overlay(&result.overlay, options(profile), Default::default())
            .unwrap(),
    );
    assert_eq!(answer, independent.owned_cross_feature(id(2)));
}

#[test]
fn cross_subsetting_uses_second_chain_feature_and_keeps_operations_separate() {
    let mut f = builder(P::OPERATIONAL_V7);
    for n in [3, 4, 900] {
        f.create(n, c::FEATURE);
    }
    relation(
        &mut f,
        2,
        3,
        203,
        c::CROSS_SUBSETTING,
        p::CROSS_SUBSETTING_CROSSED_FEATURE,
    );
    for (r, n) in [(990, 900), (104, 4)] {
        relation(
            &mut f,
            3,
            n,
            r,
            c::FEATURE_CHAINING,
            p::FEATURE_CHAINING_CHAINING_FEATURE,
        );
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options(P::OPERATIONAL_V7), Default::default())
            .unwrap(),
    );
    assert_eq!(q.owned_cross_feature(id(2)).value, None);
    assert_eq!(q.owned_cross_subsetting(id(2)).value, Some(id(203)));
    assert_eq!(q.cross_feature(id(2)).value, Some(id(4)));
}
