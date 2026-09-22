use crate as agq_kerml_semantics;
include!("../common/namespace_fixture.rs");

#[test]
fn repeated_naming_preserves_complete_and_incomplete_answers_and_reads() {
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 11, 2, c::FEATURE, "declared");
    f.create(3, c::FEATURE);
    f.create(4, c::REDEFINITION);
    f.value(
        4,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(3)),
    );
    f.own(3, 4);
    let candidate = f.construction();
    for compact in [false, true] {
        let context =
            SemanticContext::for_construction(&candidate, Default::default(), Default::default())
                .unwrap();
        let q = if compact {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        for element in [id(2), id(3)] {
            let uncached = q.compute_effective_names(element);
            for _ in 0..16 {
                assert_eq!(q.effective_names(element), uncached);
            }
        }
        assert_eq!(
            q.effective_names(id(2)).completeness,
            Completeness::Complete
        );
        assert_eq!(
            q.effective_names(id(3)).completeness,
            Completeness::Incomplete
        );
        assert_eq!(q.effective_names_cache.lock().unwrap().len(), 2);
        assert!(q.fork().effective_names_cache.lock().unwrap().is_empty());
    }
}

#[test]
fn naming_memo_cannot_leak_across_immutable_revisions() {
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 11, 2, c::FEATURE, "before");
    let before = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&before, Default::default(), Default::default()).unwrap(),
    );
    let original = q.effective_names(id(2));
    let mut changes = before.change_set();
    changes.set(
        id(2),
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("after".into())),
        origin(),
    );
    let after = before.apply(&changes).unwrap();
    let next = KerMlQueries::new(
        SemanticContext::for_snapshot(&after, Default::default(), Default::default()).unwrap(),
    );
    assert_ne!(original.value, next.effective_names(id(2)).value);
    assert_eq!(q.effective_names(id(2)), original);
}

#[test]
fn composed_naming_keeps_fallback_reads_and_identity_through_forks() {
    struct Naming;
    impl SemanticNamingExtension for Naming {
        fn naming_source(
            &self,
            q: &KerMlQueries<'_>,
            subject: ElementId,
        ) -> QueryResult<Option<Option<ElementId>>> {
            q.owned_relationships(subject).map(|_| {
                if subject == id(3) {
                    Some(Some(id(2)))
                } else if subject == id(6) {
                    Some(None)
                } else if subject == id(7) {
                    Some(Some(id(7)))
                } else {
                    None
                }
            })
        }
    }
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 11, 2, c::FEATURE, "language-name");
    f.member(1, 12, 5, c::FEATURE, "kerml-name");
    for subject in [3, 6, 7, 8] {
        f.create(subject, c::FEATURE);
        let relationship = subject + 20;
        f.create(relationship, c::REDEFINITION);
        f.value(
            relationship,
            p::REDEFINITION_REDEFINING_FEATURE,
            Value::Reference(id(subject)),
        );
        f.value(
            relationship,
            p::REDEFINITION_REDEFINED_FEATURE,
            Value::Reference(id(5)),
        );
        f.own(subject, relationship);
    }
    let snapshot = f.finish();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
        .unwrap()
        .with_semantic_extension_identity("test-language", [8; 32])
        .unwrap();
    let without_callback = context.fork();
    let context = context
        .with_naming_extension("test-language", [8; 32], Arc::new(Naming))
        .unwrap();
    assert_ne!(context.id, without_callback.id);
    let q = KerMlQueries::new(context);
    for query in [&q, &q.fork()] {
        assert_eq!(
            query.effective_names(id(3)).value,
            EffectiveNames::Determinate(BTreeSet::from(["language-name".into()]))
        );
        assert_eq!(
            query.effective_names(id(6)).value,
            EffectiveNames::Determinate(BTreeSet::new())
        );
        assert_eq!(
            query.effective_names(id(7)).completeness,
            Completeness::Incomplete
        );
        let fallback = query.effective_names(id(8));
        assert_eq!(
            fallback.value,
            EffectiveNames::Determinate(BTreeSet::from(["kerml-name".into()]))
        );
        assert!(
            fallback
                .search_dependencies
                .contains(&SearchDependency::PropertySet {
                    element: id(8),
                    property: p::ELEMENT_OWNED_RELATIONSHIP
                })
        );
    }
    assert_eq!(
        KerMlQueries::new(without_callback)
            .effective_names(id(3))
            .value,
        EffectiveNames::Determinate(BTreeSet::from(["kerml-name".into()]))
    );
    assert!(
        q.context
            .fork()
            .with_naming_extension("test-language", [9; 32], Arc::new(Naming))
            .is_err()
    );
}
