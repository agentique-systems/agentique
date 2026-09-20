//! KERML11-145: raw result identity and owned-feature domain cannot be conflated.
//! This reproduces authority choices; it does not apply an operational correction.
include!("common/namespace_fixture.rs");

fn membership(f: &mut Fixture, owner: u128, member: u128, relationship: u128, class: MetaclassId) {
    f.create(relationship, class);
    f.own(owner, relationship);
    f.changes.set(
        id(relationship),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(member))]),
        origin(),
    );
}
fn specialize(f: &mut Fixture, specific: u128, general: u128, relationship: u128) {
    f.create(relationship, c::SPECIALIZATION);
    f.own(specific, relationship);
    f.value(
        relationship,
        p::SPECIALIZATION_SPECIFIC,
        Value::Reference(id(specific)),
    );
    f.value(
        relationship,
        p::SPECIALIZATION_GENERAL,
        Value::Reference(id(general)),
    );
}

#[test]
fn raw_result_binding_requires_a_separate_authority_choice_in_all_six_profiles() {
    use agq_kerml::BaselineProfile as P;
    let mut profiles = BTreeSet::new();
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
    ] {
        let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
        let changes = base.change_set();
        let mut f = Fixture {
            base,
            changes,
            owned: BTreeMap::new(),
        };
        for (n, class, name) in [
            (1, c::FUNCTION, "tamarind"),
            (2, c::FUNCTION, "mango"),
            (3, c::FEATURE_REFERENCE_EXPRESSION, "papaya"),
            (4, c::FEATURE, "yield"),
            (5, c::FEATURE, "nestedYield"),
            (6, c::BINDING_CONNECTOR, "binding"),
            (7, c::FEATURE, "chain"),
        ] {
            f.create(n, class);
            f.value(n, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
        }
        membership(&mut f, 1, 4, 101, c::RETURN_PARAMETER_MEMBERSHIP);
        membership(&mut f, 2, 3, 102, c::RESULT_EXPRESSION_MEMBERSHIP);
        membership(&mut f, 3, 5, 103, c::RETURN_PARAMETER_MEMBERSHIP);
        membership(&mut f, 2, 6, 104, c::FEATURE_MEMBERSHIP);
        f.enumeration(4, p::FEATURE_DIRECTION, "out");
        f.enumeration(5, p::FEATURE_DIRECTION, "out");
        specialize(&mut f, 2, 1, 201);
        specialize(&mut f, 3, 1, 202);
        for (n, target) in [(301, 3), (302, 5)] {
            f.create(n, c::FEATURE_CHAINING);
            f.own(7, n);
            f.value(
                n,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                Value::Reference(id(target)),
            );
        }
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
        profiles.insert(q.context().baseline_profile_id);
        let owner = q.owning_type(id(6));
        let nested = q.owning_type(id(5));
        let supers = q.all_specializations(id(2));
        for state in [owner.completeness, nested.completeness, supers.completeness] {
            assert_eq!(state, Completeness::Complete);
        }
        assert_eq!(owner.value, Some(id(2)));
        assert_eq!(nested.value, Some(id(3)));
        // Type::isCompatibleWith on the Function is specializes(otherType).
        assert!(!supers.value.contains(&id(3)));
        // Both defaults are explicit facts in the canonical fixture: no variable
        // or chained-result clause can change isFeaturedWithin(function).
        for property in [p::FEATURE_IS_VARIABLE, p::FEATURE_IS_END] {
            assert_eq!(
                snapshot
                    .model()
                    .navigation_slot(id(5), property)
                    .unwrap()
                    .value(),
                &SlotValue::Scalar(Value::Boolean(false))
            );
        }
        // Chaining repairs the domain but cannot satisfy identity-based
        // relatedFeature->includes(rawResult) in the frozen formal requirement.
        assert_ne!(id(7), id(5));
        assert!(q.supertypes(id(7)).value.contains(&id(5)));
        assert_eq!(q.result_parameters(id(2)).value, vec![id(4)]);
        assert_eq!(q.result_parameters(id(3)).value, vec![id(5)]);
        println!(
            "{}: required function domain differs from nested result domain; chain identity distinct",
            profile.id()
        );
    }
    assert_eq!(profiles.len(), 6);
}
