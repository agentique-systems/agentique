use crate as agq_kerml_semantics;
include!("../common/namespace_fixture.rs");

#[test]
fn capability_dependencies_preserve_external_providers_and_negative_reads() {
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 11, 2, c::CLASS, "External");
    f.member(1, 12, 3, c::FEATURE, "use");
    f.create(4, c::FEATURE_TYPING);
    f.value(4, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(3)));
    f.value(4, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    f.own(3, 4);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let compact = q.audit_publication_capabilities([id(3)]);
    let expanded = q.audit_expanded_capabilities([id(3)]);
    assert_eq!(compact.checked_items, expanded.checked_items);
    assert_eq!(compact.failures, expanded.failures);
    assert_eq!(compact.read_dependencies, expanded.read_dependencies);
    for changed in [id(2), id(3), id(4)] {
        assert!(
            compact
                .read_dependencies
                .affected_by(&BTreeSet::from([changed]), false)
        );
    }
    assert!(
        !compact
            .read_dependencies
            .affected_by(&BTreeSet::from([id(99999)]), false)
    );
}
