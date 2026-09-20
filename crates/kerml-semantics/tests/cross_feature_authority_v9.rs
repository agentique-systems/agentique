//! KLCV9-F-001 is reproduced from complete arbitrary-name canonical facts.
//! The literal published selector is deliberately not repaired in this witness.
include!("common/result_fixture.rs");

fn typed(f: &mut Fixture, feature: u128, classifier: u128, relationship: u128) {
    relation(
        f,
        feature,
        classifier,
        relationship,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        relationship,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(feature)),
    );
}

fn select_cross(
    q: &KerMlQueries<'_>,
    model: &ModelView,
    end: ElementId,
    exclude_value: bool,
) -> ElementId {
    let memberships = q.memberships(end);
    assert_eq!(memberships.completeness, Completeness::Complete);
    let is = |id, class| {
        model
            .registry()
            .is_subtype(model.element(id).unwrap().metaclass(), class)
            .unwrap()
    };
    memberships
        .value
        .into_iter()
        .filter(|m| is(*m, c::OWNING_MEMBERSHIP))
        .filter_map(|m| {
            let member = q.member(m);
            assert_eq!(member.completeness, Completeness::Complete);
            member.value.map(|value| (m, value))
        })
        .find(|(m, v)| {
            if !is(*v, c::FEATURE)
                || is(*v, c::MULTIPLICITY)
                || is(*v, c::METADATA_FEATURE)
                || is(*v, c::FEATURE_VALUE)
            {
                return false;
            }
            !is(*m, c::FEATURE_MEMBERSHIP) && (!exclude_value || !is(*m, c::FEATURE_VALUE))
        })
        .unwrap()
        .1
}

#[test]
fn end_value_member_cannot_have_both_exact_domains_in_any_profile() {
    use agq_kerml::BaselineProfile as P;
    let mut profiles = BTreeSet::new();
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
    ] {
        profiles.insert(profile.id());
        let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
        let mut f = Fixture {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
        };
        for (n, cls) in [
            (1, c::FEATURE),
            (2, c::FEATURE),
            (3, c::FEATURE),
            (4, c::CLASS),
            (5, c::ASSOCIATION),
            (6, c::EXPRESSION),
            (7, c::FEATURE),
            (8, c::FEATURE),
        ] {
            f.create(n, cls);
        }
        f.value(1, p::ELEMENT_DECLARED_NAME, Value::String("arch".into()));
        for (n, rel) in [(2, 102), (3, 103)] {
            f.value(n, p::FEATURE_IS_END, Value::Boolean(true));
            member(&mut f, 1, n, rel, c::END_FEATURE_MEMBERSHIP);
            typed(&mut f, n, 4, n + 110);
        }
        typed(&mut f, 1, 5, 111);
        member(&mut f, 2, 6, 106, c::FEATURE_VALUE);
        member(&mut f, 6, 7, 107, c::RETURN_PARAMETER_MEMBERSHIP);
        f.enumeration(7, p::FEATURE_DIRECTION, "out");
        // Complete graph of the independent ordinary FeatureValue obligation.
        // Owning it by FeatureMembership would require a second, different domain.
        binding(&mut f, 2, 200, c::OWNING_MEMBERSHIP, 2, 8);
        f.changes.set(
            id(207),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(8))]),
            origin(),
        );
        relation(
            &mut f,
            8,
            6,
            108,
            c::FEATURE_CHAINING,
            p::FEATURE_CHAINING_CHAINING_FEATURE,
        );
        relation(
            &mut f,
            8,
            7,
            109,
            c::FEATURE_CHAINING,
            p::FEATURE_CHAINING_CHAINING_FEATURE,
        );
        type_featuring(&mut f, 200, 1, 220);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(
                &snapshot,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        let binding = q.validate_connector_featuring(id(200));
        assert_eq!(
            binding.completeness,
            Completeness::Complete,
            "{:?}",
            binding.diagnostics
        );
        assert!(binding.value.valid);
        assert_eq!(select_cross(&q, snapshot.model(), id(2), false), id(6));
        assert_eq!(select_cross(&q, snapshot.model(), id(2), true), id(200));
        let expected = q.direct_feature_types(id(3));
        assert_eq!(expected.completeness, Completeness::Complete);
        assert_eq!(expected.value, vec![id(4)]);
        for selected in [id(6), id(200)] {
            let actual = q.featuring_types(selected);
            assert_eq!(actual.completeness, Completeness::Complete);
            assert_eq!(actual.value, vec![id(1)]);
            assert_ne!(actual.value, expected.value, "{profile:?}");
        }
        // The literal OCL's excluding(self) retains both ends. Its Cartesian
        // branch fails too: the domain's type must occur in asCartesianProduct,
        // while both end types are the distinct classifier 4.
        assert_eq!(q.direct_feature_types(id(1)).value, vec![id(5)]);
        assert!(!expected.value.contains(&id(5)));
    }
    assert_eq!(profiles.len(), 7);
}
