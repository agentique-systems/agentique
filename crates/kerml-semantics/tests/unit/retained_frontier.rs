use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use agq_kernel::derived::{DerivationBuilder, DerivedOverlay, StructuralSearch};

fn context<'a>(
    overlay: &'a DerivedOverlay,
    registry: &ProducerRegistry,
) -> Result<SemanticContext<'a>, ContextError> {
    SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())?
        .with_producer_registry_digest(registry.digest())
}
fn old_context<'a>(
    overlay: &'a DerivedOverlay,
    registry: &ProducerRegistry,
) -> SemanticContext<'a> {
    let context = context(overlay, registry).unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, registry).unwrap();
    assert!(certificate.is_fully_closed(overlay.model()));
    context
        .with_producer_closure(Arc::new(certificate))
        .unwrap()
}
fn fixture(search: StructuralSearch) -> (Snapshot, DerivedOverlay, ElementId) {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::CLASSIFIER);
    let snapshot = f.finish();
    let key = DerivationKey {
        rule: RuleId::from_u128(999001),
        subject: id(2),
        output: OutputKey::from_u128(1),
    };
    let slots = snapshot
        .model()
        .element(id(2))
        .unwrap()
        .slots()
        .map(|(property, slot)| (property, slot.value().clone()));
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.element(key, c::CLASSIFIER, slots, BTreeSet::new());
    builder.searches(FactKey::Element(key.element_id()), BTreeSet::from([search]));
    (snapshot, builder.build().unwrap(), key.element_id())
}
fn changed(snapshot: &Snapshot, apply: impl FnOnce(&mut Fixture)) -> Snapshot {
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    apply(&mut f);
    f.finish()
}

#[test]
fn unrelated_addition_retains_exact_output_and_empty_incoming_search_retracts_on_new_match() {
    let (snapshot, overlay, output) = fixture(StructuralSearch::Incoming(id(1)));
    let registry = ProducerRegistry::new([]).unwrap();
    let old = old_context(&overlay, &registry);
    let unrelated = changed(&snapshot, |f| f.create(3, c::CLASSIFIER));
    let retained = super::verification_retain_semantic_frontier(
        &overlay,
        &old,
        unrelated,
        &registry,
        |overlay| context(overlay, &registry),
    )
    .unwrap();
    assert!(retained.retained_facts > 0);
    assert_eq!(
        retained.overlay.model().element(output),
        overlay.model().element(output)
    );
    let related = changed(&snapshot, |f| {
        f.create(4, c::SUBCLASSIFICATION);
        f.value(4, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
        f.value(4, p::SPECIALIZATION_GENERAL, Value::Reference(id(2)));
    });
    let retracted = super::verification_retain_semantic_frontier(
        &overlay,
        &old,
        related.clone(),
        &registry,
        |overlay| context(overlay, &registry),
    )
    .unwrap();
    assert_eq!(retracted.retained_facts, 0);
    assert!(retracted.overlay.model().element(output).is_none());
    let cold = DerivationBuilder::new(related).build().unwrap();
    assert!(
        retracted
            .overlay
            .model()
            .elements()
            .eq(cold.model().elements())
    );
    assert!(retracted.overlay.facts().eq(cold.facts()));
}

#[test]
fn rename_matching_a_negative_record_search_retracts_existing_output() {
    let (snapshot, overlay, output) = fixture(StructuralSearch::Element(id(1)));
    let registry = ProducerRegistry::new([]).unwrap();
    let old = old_context(&overlay, &registry);
    let next = changed(&snapshot, |f| {
        f.value(
            1,
            p::ELEMENT_DECLARED_NAME,
            Value::String("new-name".into()),
        )
    });
    let retracted =
        super::verification_retain_semantic_frontier(&overlay, &old, next, &registry, |overlay| {
            context(overlay, &registry)
        })
        .unwrap();
    assert!(retracted.overlay.model().element(output).is_none());
}

#[test]
fn newly_applicable_potential_writer_retracts_even_before_it_changes_the_read_population() {
    let (snapshot, overlay, output) = fixture(StructuralSearch::OwnedRelationships {
        owner: id(1),
        class: c::MEMBERSHIP,
    });
    let mut writer = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.FutureWriter"),
        [ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let old = old_context(&overlay, &registry);
    let next = changed(&snapshot, |f| f.create(3, c::FEATURE));
    assert_eq!(snapshot.model().element(id(1)), next.model().element(id(1)));
    let retracted =
        super::verification_retain_semantic_frontier(&overlay, &old, next, &registry, |overlay| {
            context(overlay, &registry)
        })
        .unwrap();
    assert!(retracted.overlay.model().element(output).is_none());
    assert!(
        !retracted
            .certificate
            .is_closed(id(1), SemanticClosureRequirement::EffectiveMembership)
    );
}

#[test]
fn unknown_search_contract_and_model_wide_change_fail_closed() {
    for search in [
        StructuralSearch::Model,
        StructuralSearch::OwnedMemberProjection {
            owner: id(1),
            contract: "unknown-contract".into(),
        },
    ] {
        let (snapshot, overlay, output) = fixture(search);
        let registry = ProducerRegistry::new([]).unwrap();
        let old = old_context(&overlay, &registry);
        let next = changed(&snapshot, |f| f.create(3, c::CLASSIFIER));
        let retracted = super::verification_retain_semantic_frontier(
            &overlay,
            &old,
            next,
            &registry,
            |overlay| context(overlay, &registry),
        )
        .unwrap();
        assert!(retracted.overlay.model().element(output).is_none());
    }
}
