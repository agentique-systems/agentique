//! Reproduces the independent KERML11-205 target disagreement; no correction is applied.
include!("common/namespace_fixture.rs");

#[test]
fn published_subobject_target_and_prose_target_have_different_denotations() {
    let mut identities = BTreeSet::new();
    for profile in [
        agq_kerml::BaselineProfile::PublishedKerMl10,
        agq_kerml::BaselineProfile::OPERATIONAL_V1,
        agq_kerml::BaselineProfile::OPERATIONAL_V2,
        agq_kerml::BaselineProfile::OPERATIONAL_V3,
        agq_kerml::BaselineProfile::OPERATIONAL_V4,
    ] {
        let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
        let changes = base.change_set();
        let mut f = Fixture {
            base,
            changes,
            owned: BTreeMap::new(),
        };
        f.create(1, c::NAMESPACE);
        f.member(1, 101, 2, c::PACKAGE, "Objects");
        f.member(2, 102, 3, c::STRUCTURE, "Object");
        f.member(3, 103, 4, c::FEATURE, "subobjects");
        f.member(1, 104, 5, c::PACKAGE, "Occurrences");
        f.member(5, 105, 6, c::CLASS, "Occurrence");
        f.member(6, 106, 7, c::FEATURE, "suboccurrences");
        // Arbitrary authored names. The antecedent is true independently of
        // either disputed library target and without any inference mechanism.
        f.member(1, 108, 8, c::STRUCTURE, "tamarind");
        f.member(8, 109, 9, c::FEATURE, "mango");
        f.value(9, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.create(10, c::FEATURE_TYPING);
        f.value(10, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(9)));
        f.value(10, p::FEATURE_TYPING_TYPE, Value::Reference(id(8)));
        f.own(9, 10);
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
        assert_eq!(q.owning_type(id(9)).value, Some(id(8)));
        let typing = q.direct_feature_types(id(9));
        assert_eq!(typing.completeness, Completeness::Complete);
        assert_eq!(typing.value, vec![id(8)]);
        let lookup = |path: &str| {
            q.lookup_path(
                id(9),
                &QualifiedName {
                    absolute: true,
                    segments: path.split("::").map(str::to_owned).collect(),
                },
            )
        };
        let literal = lookup("Occurrence::Occurrence::suboccurrences");
        let prose = lookup("Objects::Object::subobjects");
        let spelling_only = lookup("Occurrences::Occurrence::suboccurrences");
        for answer in [&literal, &prose, &spelling_only] {
            assert_eq!(
                answer.completeness,
                Completeness::Complete,
                "{:?}",
                answer.diagnostics
            );
        }
        assert!(literal.value.is_empty());
        assert_eq!(prose.value[0].element, id(4));
        assert_eq!(spelling_only.value[0].element, id(7));
        assert_ne!(prose.value[0].element, spelling_only.value[0].element);
        println!(
            "{}: antecedent=true; literal=unresolved-complete; prose={}; spelling-only={}; targets-distinct=true",
            profile.id(),
            id(4),
            id(7)
        );
    }
    assert_eq!(identities.len(), 5);
}
