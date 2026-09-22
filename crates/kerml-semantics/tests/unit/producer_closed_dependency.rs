use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn registry() -> ProducerRegistry {
    ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(agq_kerml::BaselineProfile::default())),
    )
    .unwrap()
}

fn closed_package() -> Arc<ProducerClosedDependency> {
    let mut fixture = Fixture::new();
    fixture.create(1, c::PACKAGE);
    let snapshot = fixture.finish();
    let result = close_result_structure(
        &snapshot,
        Default::default(),
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .map_err(PublicationOverlayError::Context)?
                .with_available_roots(BTreeMap::from([(id(1), BTreeSet::from([id(1)]))]))
                .map_err(PublicationOverlayError::Context)
        },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(result.converged);
    assert_eq!(result.completeness, Completeness::Complete);
    let overlay = Arc::new(result.overlay);
    let context = SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_available_roots(BTreeMap::from([(id(1), BTreeSet::from([id(1)]))]))
        .unwrap()
        .with_producer_registry_digest(registry().digest())
        .unwrap()
        .with_producer_closure(result.certificate.unwrap())
        .unwrap();
    ProducerClosedDependency::new(overlay.clone(), &context, &registry()).unwrap()
}

#[test]
fn producer_closed_dependency_is_shared_without_claiming_publication_acceptance() {
    let dependency = closed_package();
    let snapshot = dependency.project_snapshot();
    assert!(Arc::ptr_eq(
        snapshot.immutable_dependency().unwrap(),
        dependency.overlay()
    ));
    let mut fixture = Fixture {
        changes: snapshot.change_set(),
        base: snapshot,
        owned: BTreeMap::new(),
    };
    fixture.create(10, c::PACKAGE);
    let snapshot = fixture.finish();
    let context = dependency
        .project_context(&snapshot, &[id(10)], BTreeSet::new(), BTreeSet::new())
        .unwrap();
    assert_eq!(
        context.id().available_roots[&id(1)],
        BTreeSet::from([id(1)])
    );
    assert_eq!(
        context.id().available_roots[&id(10)],
        BTreeSet::from([id(1), id(10)])
    );
    let certificate = ProducerClosureCertificate::initial(&context, &registry()).unwrap();
    for requirement in SemanticClosureRequirement::ALL {
        assert_eq!(
            certificate.closure_source(id(1), requirement),
            Some(ClosureSource::LocalProducerClosure)
        );
    }
    assert!(context.id().publication_dependency_digest.is_none());
    let reread = dependency.project_snapshot();
    assert!(Arc::ptr_eq(
        reread.immutable_dependency().unwrap(),
        snapshot.immutable_dependency().unwrap()
    ));
}

#[test]
fn producer_closed_dependency_rejects_unproven_graphs_wrong_registry_and_rebuilt_mounts() {
    let dependency = closed_package();
    let overlay = dependency.overlay();
    let context = SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_available_roots(BTreeMap::from([(id(1), BTreeSet::from([id(1)]))]))
        .unwrap()
        .with_producer_registry_digest(registry().digest())
        .unwrap();
    assert!(matches!(
        ProducerClosedDependency::new(overlay.clone(), &context, &registry()),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let context = context
        .with_producer_closure(dependency.certificate().clone())
        .unwrap();
    let wrong = ProducerRegistry::new([]).unwrap();
    assert!(matches!(
        ProducerClosedDependency::new(overlay.clone(), &context, &wrong),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let rebuilt = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(overlay.declared().clone())
            .build()
            .unwrap(),
    );
    let snapshot = Snapshot::with_immutable_dependency(rebuilt.clone());
    assert!(matches!(
        dependency.project_context(&snapshot, &[], BTreeSet::new(), BTreeSet::new()),
        Err(ContextError::PublicationDependencyMismatch)
    ));
    assert!(matches!(
        ProducerClosedDependency::new(rebuilt, &context, &registry()),
        Err(ContextError::ProducerClosureMismatch)
    ));
}

#[test]
fn digest_marker_alone_cannot_authenticate_a_dependency() {
    let dependency = closed_package();
    let snapshot = dependency.project_snapshot();
    let mut context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry().digest())
        .unwrap();
    context.id.publication_dependency_digest = Some([7; 32]);
    assert_eq!(context.dependency_closure_source(id(1)), None);
}

#[test]
fn nested_mount_preserves_the_exact_inner_accepted_population() {
    let inner = closed_package();
    let base = inner.project_snapshot();
    let mut fixture = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    fixture.create(10, c::PACKAGE);
    let snapshot = fixture.finish();
    // Exercise the private authority marker independently of real corpus acceptance.
    // Production installs it only after the accepted facade checks exact Arc identity.
    fn context_for<'m>(
        inner: &Arc<ProducerClosedDependency>,
        overlay: &'m agq_kernel::derived::DerivedOverlay,
    ) -> Result<SemanticContext<'m>, ContextError> {
        let mut context = inner.project_overlay_context(overlay, &[id(10)])?;
        context.accepted_dependency = Some(inner.overlay().clone());
        context.id.publication_dependency_digest = Some([7; 32]);
        Ok(context)
    }
    let closed = close_result_structure(
        &snapshot,
        Default::default(),
        |overlay| context_for(&inner, overlay).map_err(PublicationOverlayError::Context),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(closed.completeness, Completeness::Complete);
    let overlay = Arc::new(closed.overlay);
    let context = context_for(&inner, &overlay)
        .unwrap()
        .with_producer_closure(closed.certificate.unwrap())
        .unwrap();
    let outer = ProducerClosedDependency::new(overlay.clone(), &context, &registry()).unwrap();
    let project = outer.project_snapshot();
    let context = outer
        .project_context(&project, &[], BTreeSet::new(), BTreeSet::new())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry()).unwrap();
    for requirement in SemanticClosureRequirement::ALL {
        assert_eq!(
            certificate.closure_source(id(1), requirement),
            Some(ClosureSource::AcceptedDependency)
        );
        assert_eq!(
            certificate.closure_source(id(10), requirement),
            Some(ClosureSource::LocalProducerClosure)
        );
    }
}
