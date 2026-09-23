use super::*;
use agq_kerml_semantics::DerivationPhase;
use agq_kernel::derived::DerivationBuilder;

fn bindings() -> StandardSysmlBindings {
    StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned(
        SystemsLibraryIdentity::SOURCE_CONTENT_SET,
    ))
}

fn current_context(overlay: &DerivedOverlay) -> SemanticContext<'_> {
    SemanticContext::for_overlay(
        overlay,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
}

#[test]
fn producer_overlay_bindings_attach_only_to_the_exact_producer_graph() {
    use agq_kerml_semantics::{KerMlQueries, ProducerClosureCertificate, ProducerRegistry};
    use agq_kernel::{
        provenance::DeclaredOrigin,
        value::{SlotValue, Value},
    };

    let declared = crate::tests::library_fixture(agq_sysml::classes::PART_DEFINITION, false);
    let overlay = DerivationBuilder::new(declared.clone()).build().unwrap();
    let raw = current_context(&overlay);
    let producer = producer_context(raw.fork()).unwrap();
    assert_ne!(raw.id().model_digest, producer.id().model_digest);
    let validate = |context| {
        StandardSysmlBindings::validate(
            overlay.model(),
            &KerMlQueries::new(context),
            SystemsLibraryIdentity::pinned([7; 32]),
            &[ElementId::from_u128(1)],
            [StandardSysmlRole::Part],
        )
        .unwrap()
    };
    let bindings = validate(producer.fork());
    assert_eq!(
        bindings.get(StandardSysmlRole::Part),
        Some(ElementId::from_u128(3))
    );
    let trusted = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OPERATIONAL_V2,
    )
    .unwrap();
    // The exact private composition used by for_producer_overlay, without
    // forging an accepted library or loading its large graph in a unit test.
    let context =
        SysmlSemanticContext::attach(overlay.model(), producer, trusted.clone(), bindings.clone())
            .unwrap();
    assert!(matches!(
        SysmlSemanticContext::attach(
            overlay.model(),
            raw.fork(),
            trusted.clone(),
            bindings.clone()
        ),
        Err(SysmlContextError::IdentityMismatch(
            "standard SysML bindings graph"
        ))
    ));
    let registry = ProducerRegistry::new(
        agq_kerml_semantics::ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(BaselineProfile::OPERATIONAL_V9))
            .chain(crate::sysml_producer_descriptors()),
    )
    .unwrap();
    let certificate =
        Arc::new(ProducerClosureCertificate::initial(context.kerml_context(), &registry).unwrap());
    let attached = context
        .fork()
        .with_producer_closure(certificate.clone())
        .unwrap();
    assert!(
        attached
            .bindings
            .valid_for_context(attached.kerml_context())
    );
    assert_eq!(
        attached.id().kerml.model_digest,
        context.id().kerml.model_digest
    );
    assert_eq!(
        attached.id().kerml.producer_registry_digest,
        Some(registry.digest())
    );

    let raw_bindings = validate(raw.fork());
    let raw_context =
        SysmlSemanticContext::attach(overlay.model(), raw.fork(), trusted.clone(), raw_bindings)
            .unwrap();
    assert!(matches!(
        raw_context.with_producer_closure(certificate.clone()),
        Err(SysmlContextError::IdentityMismatch(
            "standard SysML bindings graph"
        ))
    ));

    let weaker_registry = ProducerRegistry::new([]).unwrap();
    let weaker_context = SysmlSemanticContext::attach(
        overlay.model(),
        raw,
        trusted.clone(),
        validate(current_context(&overlay)),
    )
    .unwrap()
    .kerml
    .with_producer_registry_digest(weaker_registry.digest())
    .unwrap();
    let weaker_certificate =
        Arc::new(ProducerClosureCertificate::initial(&weaker_context, &weaker_registry).unwrap());
    assert!(matches!(
        context.with_producer_closure(weaker_certificate),
        Err(SysmlContextError::KerMl(
            ContextError::ProducerClosureMismatch
        ))
    ));

    let mut changes = declared.change_set();
    changes.set(
        ElementId::from_u128(1),
        agq_kerml::properties::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("changed root".into())),
        DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        },
    );
    let changed = DerivationBuilder::new(declared.apply(&changes).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        SysmlSemanticContext::attach(
            changed.model(),
            producer_context(current_context(&changed)).unwrap(),
            trusted,
            bindings
        ),
        Err(SysmlContextError::IdentityMismatch(
            "standard SysML bindings graph"
        ))
    ));
}

#[test]
fn sysml_facade_rejects_weaker_registry_on_the_same_graph_and_interpretation() {
    use agq_kerml_semantics::{
        Completeness, PublicationOverlayError, close_result_structure_with_extension,
    };
    let declared = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let profile = SysmlBaselineProfile::OPERATIONAL_V2;
    let bindings = bindings();
    let trusted = SysmlDependencyContract::checked_in_for_profile(&bindings, profile).unwrap();
    fn context_for<'m>(
        overlay: &'m DerivedOverlay,
        trusted: &SysmlDependencyContract,
        bindings: &StandardSysmlBindings,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        SysmlSemanticContext::attach(
            overlay.model(),
            current_context(overlay),
            trusted.clone(),
            bindings.clone(),
        )
        .map(|context| context.kerml)
        .map_err(|error| match error {
            SysmlContextError::KerMl(error) => PublicationOverlayError::Context(error),
            _ => panic!("fixture context: {error}"),
        })
    }
    // Both are real scheduler runs over the exact same combined descriptor
    // graph and SysML naming/profile contract. Only the registry differs.
    let weaker = close_result_structure_with_extension(
        &declared,
        Default::default(),
        |overlay| context_for(overlay, &trusted, &bindings),
        &(),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    let combined = close_result_structure_with_extension(
        &declared,
        Default::default(),
        |overlay| context_for(overlay, &trusted, &bindings),
        &crate::SysmlProducerExtension::new(profile, bindings.clone(), vec![]),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(weaker.completeness, Completeness::Complete);
    assert_eq!(combined.completeness, Completeness::Complete);
    let weaker_certificate = weaker.certificate.unwrap();
    let combined_certificate = combined.certificate.unwrap();
    assert_eq!(
        weaker_certificate.model_digest(),
        combined_certificate.model_digest()
    );
    assert_ne!(
        weaker_certificate.registry_digest(),
        combined_certificate.registry_digest()
    );
    let context = SysmlSemanticContext::attach(
        combined.overlay.model(),
        current_context(&combined.overlay),
        trusted,
        bindings,
    )
    .unwrap();
    assert!(matches!(
        context.fork().with_producer_closure(weaker_certificate),
        Err(SysmlContextError::KerMl(
            ContextError::ProducerClosureMismatch
        ))
    ));
    let accepted = context
        .with_producer_closure(combined_certificate.clone())
        .unwrap();
    assert_eq!(
        accepted.id().kerml.producer_registry_digest,
        Some(combined_certificate.registry_digest())
    );
    assert_eq!(
        accepted.id().kerml.producer_closure_digest,
        Some(combined_certificate.digest())
    );
    assert!(Arc::ptr_eq(
        accepted.kerml.producer_closure().unwrap(),
        &combined_certificate
    ));
}

#[test]
fn overlay_attachment_preserves_current_graph_phase_and_explicit_profile() {
    let declared = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let overlay = DerivationBuilder::new(declared).build().unwrap();
    for profile in [
        SysmlBaselineProfile::PUBLISHED,
        SysmlBaselineProfile::OPERATIONAL_V1,
    ] {
        let bindings = bindings();
        let trusted = SysmlDependencyContract::checked_in_for_profile(&bindings, profile).unwrap();
        let kerml = current_context(&overlay);
        let before = kerml.id().clone();
        // Exercise the shared attachment step without forging an accepted
        // publication fixture or loading the full accepted library cache.
        let context =
            SysmlSemanticContext::attach(overlay.model(), kerml, trusted.clone(), bindings)
                .unwrap();
        assert!(std::ptr::eq(context.model(), overlay.model()));
        let fork = context.fork();
        assert!(std::ptr::eq(context.model(), fork.model()));
        assert_eq!(context.id(), fork.id());
        assert_eq!(context.bindings.targets(), fork.bindings.targets());
        assert_eq!(context.id().dependencies, trusted);
        assert_eq!(context.id().dependencies.sysml_profile, profile);
        assert_eq!(
            context.id().kerml.derivation_phase,
            DerivationPhase::PartialDerivationOverlay
        );
        assert_eq!(context.id().kerml.model_digest, before.model_digest);
        assert_eq!(
            context.id().kerml.publication_dependency_digest,
            before.publication_dependency_digest
        );
        assert_eq!(
            context.id().kerml.semantic_extensions[crate::SYSML_SEMANTIC_CONTEXT_DOMAIN],
            trusted.context_identity_digest()
        );
    }
}

#[test]
fn overlay_checks_reject_an_unaccepted_graph_and_a_forged_expected_dependency() {
    let overlay = DerivationBuilder::new(Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    )))
    .build()
    .unwrap();
    let context = current_context(&overlay);
    let trusted = SysmlDependencyContract::checked_in_for_profile(
        &bindings(),
        SysmlBaselineProfile::OPERATIONAL_V1,
    )
    .unwrap();
    assert_eq!(
        validate_accepted(context.id(), &trusted),
        Err(SysmlContextError::IdentityMismatch(
            "accepted KerML publication digest"
        ))
    );
    let mut forged = trusted.clone();
    forged.kerml_publication_digest = context.id().model_digest;
    assert_eq!(
        validate_contract(&forged, &trusted),
        Err(SysmlContextError::IdentityMismatch(
            "KerML publication digest"
        ))
    );
}

#[test]
fn overlay_attachment_requires_the_combined_descriptor_graph() {
    let overlay = DerivationBuilder::new(Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    )))
    .build()
    .unwrap();
    let bindings = bindings();
    let trusted = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OPERATIONAL_V1,
    )
    .unwrap();
    assert!(matches!(
        SysmlSemanticContext::attach(
            overlay.model(),
            current_context(&overlay),
            trusted,
            bindings,
        ),
        Err(SysmlContextError::IdentityMismatch(
            "combined descriptor graph"
        ))
    ));
}
