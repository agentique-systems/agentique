use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn end(fixture: &mut Fixture, owner: u128, feature: u128) {
    fixture.create(feature, c::FEATURE);
    fixture.value(feature, p::FEATURE_IS_END, Value::Boolean(true));
    fixture.value(
        feature,
        p::ELEMENT_DECLARED_NAME,
        Value::String(format!("end{feature}")),
    );
    member(
        fixture,
        owner,
        feature,
        feature + 1000,
        c::END_FEATURE_MEMBERSHIP,
    );
}

fn cycle(reverse_creation: bool, a: &[u128], b: &[u128], external: &[u128]) -> Snapshot {
    let mut fixture = Fixture::new();
    for owner in if reverse_creation {
        [3, 2, 1]
    } else {
        [1, 2, 3]
    } {
        fixture.create(owner, c::CONNECTOR);
    }
    for (owner, features) in [(1, a), (2, b), (3, external)] {
        for (position, &feature) in features.iter().enumerate() {
            end(&mut fixture, owner, feature);
            fixture.value(
                feature,
                p::ELEMENT_DECLARED_NAME,
                Value::String(format!("position{position}")),
            );
        }
    }
    for (specific, general, relationship) in if reverse_creation {
        [(1, 3, 103), (2, 1, 102), (1, 2, 101)]
    } else {
        [(1, 2, 101), (2, 1, 102), (1, 3, 103)]
    } {
        subset(&mut fixture, specific, general, relationship);
    }
    fixture.finish()
}

fn queries(snapshot: &Snapshot, composed: bool) -> KerMlQueries<'_> {
    let context =
        SemanticContext::for_snapshot(snapshot, Default::default(), BTreeSet::new()).unwrap();
    let context = if composed {
        context
            .with_semantic_extension_identity("test-ordered-positions/1", [17; 32])
            .unwrap()
    } else {
        context
    };
    KerMlQueries::new(context)
}

#[test]
fn composed_cycle_with_fully_owned_positions_keeps_exact_order_and_evidence() {
    for reverse in [false, true] {
        let snapshot = cycle(reverse, &[12, 11], &[21, 22], &[32, 31]);
        let q = queries(&snapshot, true);
        for (owner, expected) in [(1, vec![id(12), id(11)]), (2, vec![id(21), id(22)])] {
            let answer = q.structural_end_features(id(owner));
            assert_eq!(
                answer.completeness,
                Completeness::Complete,
                "{:?}",
                answer.diagnostics
            );
            assert_eq!(answer.value, expected);
            for feature in [11, 12, 21, 22, 31, 32] {
                assert!(answer.positive_dependencies.contains(&FactKey::Property {
                    element: id(feature),
                    property: p::FEATURE_IS_END,
                }));
            }
            for namespace in [1, 2, 3] {
                assert!(answer.search_dependencies.contains(
                    &SearchDependency::OwnedRelationships {
                        owner: id(namespace),
                        class: c::SPECIALIZATION,
                    }
                ));
                assert!(answer.search_dependencies.contains(
                    &SearchDependency::StructuralFeaturePopulation {
                        owner: id(namespace),
                        kind: FeaturePopulationKind::End,
                    }
                ));
            }
        }
        // Cyclic specializations imply reciprocal positional redefinitions;
        // the local position proof does not select or copy inherited Features.
        for (feature, target) in [(12, 21), (21, 12), (11, 22), (22, 11)] {
            let redefined = q.redefined_features(id(feature));
            assert_eq!(
                redefined.completeness,
                Completeness::Complete,
                "{:?}",
                redefined.diagnostics
            );
            assert!(redefined.value.contains(&id(target)));
        }
        for (owner, feature) in [(1, 12), (2, 21)] {
            let lookup = q.lookup_member(id(owner), "position0", MemberAccess::All);
            assert_eq!(
                lookup.completeness,
                Completeness::Complete,
                "{:?}",
                lookup.diagnostics
            );
            assert_eq!(lookup.value.len(), 1);
            assert_eq!(lookup.value[0].element, id(feature));
        }
    }
}

#[test]
fn sealed_kerml_only_cycle_interpretation_stays_incomplete() {
    let snapshot = cycle(false, &[12, 11], &[21, 22], &[]);
    let q = queries(&snapshot, false);
    let answer = q.structural_end_features(id(1));
    assert_eq!(answer.completeness, Completeness::Incomplete);
    assert!(answer.diagnostics.iter().any(|d| d.code == "KQ_END_CYCLE"));
    assert!(q.context().semantic_extensions.is_empty());
}

#[test]
fn empty_parameter_cycle_is_complete_only_with_empty_external_positions() {
    let snapshot = cycle(false, &[12, 11], &[21, 22], &[31]);
    let answer = queries(&snapshot, true).structural_parameter_features(id(1));
    assert_eq!(answer.completeness, Completeness::Complete);
    assert!(answer.value.is_empty());
    let answer = queries(&snapshot, false).structural_parameter_features(id(1));
    assert_eq!(answer.completeness, Completeness::Incomplete);

    let mut changes = snapshot.change_set();
    let ValueKind::Enumeration(domain) = snapshot
        .model()
        .registry()
        .property(p::FEATURE_DIRECTION)
        .unwrap()
        .value_kind
    else {
        unreachable!()
    };
    let input = snapshot
        .model()
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, name)| name.as_str() == "in")
        .unwrap()
        .0;
    changes.set(
        id(31),
        p::FEATURE_DIRECTION,
        SlotValue::Scalar(Value::Enumeration(*input)),
        origin(),
    );
    let directed = snapshot.apply(&changes).unwrap();
    let answer = queries(&directed, true).structural_parameter_features(id(1));
    assert_eq!(answer.completeness, Completeness::Incomplete);
}

#[test]
fn uncovered_cyclic_positions_do_not_guess_an_inherited_order() {
    for (a, b, external) in [
        (vec![12, 11], vec![21], vec![]),
        (vec![12, 11], vec![21, 22], vec![31, 32, 33]),
        (vec![], vec![], vec![31]),
    ] {
        let snapshot = cycle(false, &a, &b, &external);
        let answer = queries(&snapshot, true).structural_end_features(id(1));
        assert_eq!(answer.completeness, Completeness::Incomplete);
        assert!(answer.diagnostics.iter().any(|d| d.code == "KQ_END_CYCLE"));
    }
}

#[test]
fn resolved_component_feeds_acyclic_descendants_without_copying_positions() {
    let mut fixture = Fixture::new();
    for owner in [1, 2, 3] {
        fixture.create(owner, c::CONNECTOR);
    }
    for (owner, feature) in [(1, 12), (1, 11), (2, 22), (2, 21)] {
        end(&mut fixture, owner, feature);
    }
    subset(&mut fixture, 1, 2, 101);
    subset(&mut fixture, 2, 1, 102);
    subset(&mut fixture, 3, 1, 103);
    let snapshot = fixture.finish();
    let answer = queries(&snapshot, true).structural_end_features(id(3));
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{:?}",
        answer.diagnostics
    );
    assert_eq!(answer.value, vec![id(12), id(11)]);
}
