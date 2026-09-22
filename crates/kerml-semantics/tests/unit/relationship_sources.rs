use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn source_roles_include_registered_redefinitions_and_cache_only_descriptors() {
    use agq_kernel::metamodel::{MetamodelRegistry, PropertyOwner};
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let custom_class = MetaclassId::from_u128(0xfeed01);
    let custom_source = PropertyId::from_u128(0xfeed02);
    let mut descriptors = agq_kerml::descriptors_for_profile(profile).unwrap();
    let mut class = descriptors
        .classes
        .iter()
        .find(|c| c.id == c::FEATURE_TYPING)
        .unwrap()
        .clone();
    class.id = custom_class;
    class.name = "FixtureTyping".into();
    class.direct_supertypes = BTreeSet::from([c::FEATURE_TYPING]);
    class.is_abstract = false;
    descriptors.classes.push(class);
    let mut property = descriptors
        .properties
        .iter()
        .find(|p| p.id == p::FEATURE_TYPING_TYPED_FEATURE)
        .unwrap()
        .clone();
    property.id = custom_source;
    property.name = "fixtureSpecific".into();
    property.owner = PropertyOwner::Class(custom_class);
    property.redefines = BTreeSet::from([p::FEATURE_TYPING_TYPED_FEATURE]);
    property.association = None;
    property.opposite_ends.clear();
    descriptors.properties.push(property);
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::CLASSIFIER);
    for n in 0..1024 {
        let feature = 100 + n * 2;
        f.create(feature, c::FEATURE);
        f.create(feature + 1, custom_class);
        f.value(feature + 1, p::FEATURE_TYPING_TYPE, Value::Reference(id(1)));
        f.value(feature + 1, custom_source, Value::Reference(id(feature)));
    }
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                exclude_implied: true,
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    assert_eq!(q.direct_specializations(id(100)).value, vec![id(1)]);
    assert_eq!(q.direct_feature_types(id(100)).value, vec![id(1)]);
    let roles = q.source_roles(c::SPECIALIZATION, p::SPECIALIZATION_SPECIFIC);
    assert!(roles[&custom_source].contains(&custom_class));
    assert!(Arc::ptr_eq(
        &roles,
        &q.source_roles(c::SPECIALIZATION, p::SPECIALIZATION_SPECIFIC)
    ));
    // No reference in the 1024-element general population is examined: none
    // belongs to a possible source role bucket. This is a work bound, not timing.
    assert_eq!(snapshot.model().incoming(id(1)).count(), 1024);
    assert_eq!(
        roles
            .keys()
            .map(|property| snapshot
                .model()
                .incoming_for_property(id(1), *property)
                .count())
            .sum::<usize>(),
        0
    );
    let absent = q.direct_specializations(id(1));
    assert!(absent.value.is_empty());
    assert_eq!(absent.completeness, Completeness::Complete);
    assert!(
        absent
            .search_dependencies
            .contains(&SearchDependency::SourceRelationships {
                source: id(1),
                class: c::SPECIALIZATION,
                property: p::SPECIALIZATION_SPECIFIC,
            })
    );
    assert_eq!(q.source_role_cache.lock().unwrap().len(), 2);
    assert!(q.fork().source_role_cache.lock().unwrap().is_empty());
}
