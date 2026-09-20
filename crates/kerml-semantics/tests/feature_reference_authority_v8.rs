//! KERML11-8 is independent of the five authorized KERML11-145 corrections.
//! This fully specified, arbitrary-name graph is a witness, not a correction.
include!("common/namespace_fixture.rs");

fn member(f: &mut Fixture, owner: u128, member: u128, rel: u128, class: MetaclassId) {
    f.create(rel, class);
    f.own(owner, rel);
    f.changes.set(
        id(rel),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(member))]),
        origin(),
    );
}

fn relation(
    f: &mut Fixture,
    specific: u128,
    general: u128,
    rel: u128,
    class: MetaclassId,
    property: PropertyId,
) {
    f.create(rel, class);
    f.own(specific, rel);
    f.value(rel, property, Value::Reference(id(general)));
}

fn subset(f: &mut Fixture, specific: u128, general: u128, rel: u128) {
    relation(
        f,
        specific,
        general,
        rel,
        c::SUBSETTING,
        p::SUBSETTING_SUBSETTED_FEATURE,
    );
    f.value(
        rel,
        p::SUBSETTING_SUBSETTING_FEATURE,
        Value::Reference(id(specific)),
    );
}

fn type_featuring(f: &mut Fixture, feature: u128, ty: u128, rel: u128) {
    relation(
        f,
        feature,
        ty,
        rel,
        c::TYPE_FEATURING,
        p::TYPE_FEATURING_FEATURING_TYPE,
    );
    f.value(
        rel,
        p::TYPE_FEATURING_FEATURE_OF_TYPE,
        Value::Reference(id(feature)),
    );
}

fn redefine(f: &mut Fixture, specific: u128, general: u128, rel: u128) {
    relation(
        f,
        specific,
        general,
        rel,
        c::REDEFINITION,
        p::REDEFINITION_REDEFINED_FEATURE,
    );
    f.value(
        rel,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(specific)),
    );
}

fn binding(f: &mut Fixture, owner: u128, n: u128, membership: MetaclassId, a: u128, b: u128) {
    f.create(n, c::BINDING_CONNECTOR);
    member(f, owner, n, n + 1, membership);
    for (offset, endpoint) in [(2, a), (5, b)] {
        let end = n + offset;
        f.create(end, c::FEATURE);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(f, n, end, end + 1, c::END_FEATURE_MEMBERSHIP);
        relation(
            f,
            end,
            endpoint,
            end + 2,
            c::REFERENCE_SUBSETTING,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        );
    }
}

#[test]
fn contextual_outer_binding_does_not_repair_the_separate_raw_reference_binding() {
    use agq_kerml::BaselineProfile as P;
    let mut identities = BTreeSet::new();
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
            (1, c::FUNCTION, "orchard"),
            (2, c::FEATURE_REFERENCE_EXPRESSION, "acorn"),
            (3, c::FEATURE, "sprout"),
            (4, c::FEATURE, "branch"),
            (5, c::FEATURE, "fruit"),
            (6, c::FEATURE, "context"),
            (7, c::FEATURE, "root"),
            (8, c::FEATURE, "leaf"),
            (10, c::FUNCTION, "growth"),
            (11, c::FEATURE, "produce"),
            (12, c::EXPRESSION, "germination"),
            (20, c::BINDING_CONNECTOR, "graft"),
            (21, c::FEATURE, "stock"),
            (22, c::FEATURE, "scion"),
        ] {
            f.create(n, class);
            f.value(n, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
        }
        member(&mut f, 1, 2, 101, c::RESULT_EXPRESSION_MEMBERSHIP);
        member(&mut f, 2, 3, 102, c::RETURN_PARAMETER_MEMBERSHIP);
        member(&mut f, 1, 4, 103, c::FEATURE_MEMBERSHIP);
        member(&mut f, 1, 5, 104, c::RETURN_PARAMETER_MEMBERSHIP);
        member(&mut f, 1, 6, 105, c::OWNING_MEMBERSHIP);
        member(&mut f, 1, 7, 106, c::FEATURE_MEMBERSHIP);
        member(&mut f, 7, 8, 107, c::FEATURE_MEMBERSHIP);
        f.enumeration(3, p::FEATURE_DIRECTION, "out");
        f.enumeration(5, p::FEATURE_DIRECTION, "out");
        f.enumeration(11, p::FEATURE_DIRECTION, "out");
        member(&mut f, 10, 11, 130, c::RETURN_PARAMETER_MEMBERSHIP);
        for (end, rel) in [(21, 131), (22, 132)] {
            member(&mut f, 20, end, rel, c::END_FEATURE_MEMBERSHIP);
            f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        }
        relation(
            &mut f,
            1,
            10,
            133,
            c::SUBCLASSIFICATION,
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
        relation(
            &mut f,
            12,
            10,
            134,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            133,
            p::SUBCLASSIFICATION_SUBCLASSIFIER,
            Value::Reference(id(1)),
        );
        f.value(
            134,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(12)),
        );
        subset(&mut f, 2, 12, 135);
        redefine(&mut f, 3, 11, 136);
        redefine(&mut f, 5, 11, 137);
        relation(
            &mut f,
            2,
            4,
            108,
            c::MEMBERSHIP,
            p::MEMBERSHIP_MEMBER_ELEMENT,
        );
        // Complete the required referent subsetting, before assessing domain.
        subset(&mut f, 3, 4, 109);
        // Normative chain order is [source, target] and [expression, result].
        for (owner, rel, target) in [(4, 110, 7), (4, 111, 8), (6, 112, 2), (6, 113, 3)] {
            relation(
                &mut f,
                owner,
                target,
                rel,
                c::FEATURE_CHAINING,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
            );
        }
        for (feature, ty, rel) in [(2, 1, 120), (3, 2, 121), (4, 1, 122), (5, 1, 123)] {
            type_featuring(&mut f, feature, ty, rel);
        }
        // The inner connector already uses plain membership, as in the pilot.
        binding(&mut f, 2, 200, c::OWNING_MEMBERSHIP, 4, 3);
        type_featuring(&mut f, 200, 1, 220);
        // Independently build the authorized outer correction as a proof
        // object. This is not claiming a v6 profile or production implication.
        binding(&mut f, 1, 300, c::FEATURE_MEMBERSHIP, 5, 6);
        type_featuring(&mut f, 300, 1, 320);
        // Add the positional and base-specialization facts too. These cannot
        // turn the outer classifier into its nested expression or remove the
        // nested result's mandatory owning domain.
        for connector in [200, 300] {
            subset(&mut f, connector, 20, connector + 30);
            redefine(&mut f, connector + 2, 21, connector + 31);
            redefine(&mut f, connector + 5, 22, connector + 32);
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
        identities.insert(q.context().baseline_profile_id);
        for (feature, owner) in [(3, Some(2)), (4, Some(1)), (200, None), (300, Some(1))] {
            let answer = q.owning_type(id(feature));
            assert_eq!(answer.completeness, Completeness::Complete);
            assert_eq!(answer.value, owner.map(id));
        }
        for (specific, excluded) in [(1, 2), (2, 1)] {
            let answer = q.all_specializations(id(specific));
            assert_eq!(answer.completeness, Completeness::Complete);
            assert!(!answer.value.contains(&id(excluded)));
        }
        let subset = q.subsetted_features(id(3));
        assert_eq!(subset.completeness, Completeness::Complete);
        assert!(subset.value.contains(&id(4)));
        let chain = q.supertypes(id(6));
        assert_eq!(chain.completeness, Completeness::Complete);
        assert!(chain.value.contains(&id(3)));
        assert_ne!(id(6), id(3));
        // Type::isCompatibleWith is specializes for the Function classifier.
        // Feature's common-redefinition alternative cannot apply to E: E owns
        // a result Feature. Thus no candidate in {F,E} features both endpoints.
        let expression_features = q.direct_features(id(2));
        assert_eq!(expression_features.completeness, Completeness::Complete);
        assert!(expression_features.value.contains(&id(3)));
        for (specific, general) in [(3, 11), (202, 21), (205, 22)] {
            let answer = q.redefined_features(id(specific));
            assert_eq!(answer.completeness, Completeness::Complete);
            assert!(answer.value.contains(&id(general)));
        }
        println!(
            "{}: complete ownership/subsetting/chain facts; corrected outer result domain F; inner raw result domain E; neither F specializes E nor E specializes F",
            profile.id()
        );
    }
    assert_eq!(identities.len(), 6);
}
