//! Public queries consume real scheduler-issued witnesses; tests cannot issue one.
include!("common/result_fixture.rs");

fn fixture(typed_owner: bool) -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.create(2, c::BEHAVIOR);
    f.create(3, c::STEP);
    f.value(3, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    member(&mut f, 1, 3, 13, c::FEATURE_MEMBERSHIP);
    if typed_owner {
        relation(&mut f, 1, 2, 12, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
        f.value(12, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    }
    f.finish()
}

fn overlay_context(overlay: &agq_kernel::derived::DerivedOverlay) -> SemanticContext<'_> {
    SemanticContext::for_overlay(overlay, Default::default(), Default::default()).unwrap()
}

fn close(snapshot: &Snapshot) -> PublicationClosure {
    let closure = close_result_structure(
        snapshot,
        Default::default(),
        |overlay| Ok(overlay_context(overlay)),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closure.converged, "{:?}", closure.stages);
    assert_eq!(
        closure.completeness,
        Completeness::Complete,
        "{:?}",
        closure.stages
    );
    closure
}

#[test]
fn negative_typing_becomes_complete_only_with_the_exact_scheduler_witness() {
    for typed_owner in [false, true] {
        let snapshot = fixture(typed_owner);
        let closure = close(&snapshot);
        let current = KerMlQueries::new(overlay_context(&closure.overlay));
        let rule = FormalConstraintId::StepOwnedPerformanceSpecialization;
        let open = current.formal_constraint_applies(rule, id(3));
        assert!(!open.value);
        assert_eq!(open.completeness, Completeness::Incomplete);
        let certificate = closure.certificate.as_ref().unwrap();
        let context = overlay_context(&closure.overlay)
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap()
            .with_producer_closure(certificate.clone())
            .unwrap();
        let effective = KerMlQueries::new(context);
        let answer = effective.formal_constraint_applies(rule, id(3));
        assert!(!answer.value);
        assert_eq!(
            answer.completeness,
            Completeness::Complete,
            "{:?}",
            answer.diagnostics
        );
        let witness = SearchDependency::ProducerClosure {
            subject: id(1),
            requirement: SemanticClosureRequirement::EffectiveTyping,
            certificate_digest: Some(certificate.digest()),
        };
        assert!(answer.search_dependencies.contains(&witness));
        assert!(answer.explanations.values().flatten().any(|proof| {
            proof.rule == Rule::ProducerClosure(SemanticClosureRequirement::EffectiveTyping)
                && proof.premises.contains(&Evidence::Search(witness.clone()))
        }));
        assert_eq!(effective.negative_queries_certified(), 1);
        assert_eq!(effective.fork().negative_queries_certified(), 0);
    }
}

#[test]
fn stale_graph_registry_or_interpretation_cannot_reuse_a_certificate() {
    let snapshot = fixture(true);
    let closure = close(&snapshot);
    let certificate = closure.certificate.as_ref().unwrap();
    let context = overlay_context(&closure.overlay)
        .with_producer_registry_digest(certificate.registry_digest())
        .unwrap()
        .with_producer_closure(certificate.clone())
        .unwrap();
    assert!(Arc::ptr_eq(
        context.producer_closure().unwrap(),
        certificate
    ));
    let changed_contract = context
        .fork()
        .with_semantic_extension_identity("fixture-extension/1", [9; 32])
        .unwrap();
    assert!(changed_contract.producer_closure().is_none());
    assert!(matches!(
        changed_contract.with_producer_closure(certificate.clone()),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let changed_registry = overlay_context(&closure.overlay)
        .with_producer_registry_digest([9; 32])
        .unwrap();
    assert!(matches!(
        changed_registry.with_producer_closure(certificate.clone()),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let mut changes = snapshot.change_set();
    changes.set(
        id(1),
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("changed".into())),
        origin(),
    );
    let changed = snapshot.apply(&changes).unwrap();
    let changed_graph =
        SemanticContext::for_snapshot(&changed, Default::default(), Default::default())
            .unwrap()
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap();
    assert!(matches!(
        changed_graph.with_producer_closure(certificate.clone()),
        Err(ContextError::ProducerClosureMismatch)
    ));
}
