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
    assert_eq!(compact.provider_reads, expanded.provider_reads);
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

#[test]
fn publication_distinguishes_declared_identity_from_persistent_population_searches() {
    use crate::read_dependencies::{query_publication_provider_keys, query_read_keys};
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 11, 2, c::FUNCTION, "Copper");
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let mut answer = q.effective_names(id(2));
    assert_eq!(answer.completeness, Completeness::Complete);
    assert!(query_publication_provider_keys(&answer, snapshot.model()).is_empty());
    // Revision invalidation still observes both changing the record and its name.
    assert!(!query_read_keys(&answer, snapshot.model()).is_empty());

    // Persistent Element searches can encode namespace populations. They must
    // not be confused with a native identity/metaclass check.
    answer.search_dependencies.insert(SearchDependency::Kernel(
        agq_kernel::derived::StructuralSearch::Element(id(2)),
    ));
    let providers = PublicationProviderReads::from_keys(query_publication_provider_keys(
        &answer,
        snapshot.model(),
    ));
    assert_eq!(providers.bounded_elements(), &[id(2)]);
}

#[test]
fn missing_identities_and_mutable_ownership_remain_publication_obligations() {
    use crate::read_dependencies::query_publication_provider_keys;
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 11, 2, c::FUNCTION, "Indigo");
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default()).unwrap(),
    );
    let mut answer = q.effective_names(id(2));
    answer.search_dependencies.extend([
        SearchDependency::Element(id(999)),
        SearchDependency::PropertySet {
            element: id(2),
            property: p::ELEMENT_OWNED_RELATIONSHIP,
        },
        SearchDependency::Incoming { target: id(2) },
    ]);
    let providers = PublicationProviderReads::from_keys(query_publication_provider_keys(
        &answer,
        snapshot.model(),
    ));
    assert_eq!(providers.bounded_elements(), &[id(2), id(999)]);
    assert!(!providers.reads_entire_model());
    answer.search_dependencies.insert(SearchDependency::Kernel(
        agq_kernel::derived::StructuralSearch::Model,
    ));
    assert!(
        PublicationProviderReads::from_keys(query_publication_provider_keys(
            &answer,
            snapshot.model(),
        ))
        .reads_entire_model()
    );
}
