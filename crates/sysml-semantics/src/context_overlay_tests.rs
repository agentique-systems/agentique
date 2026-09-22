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
