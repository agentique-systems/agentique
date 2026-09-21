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
