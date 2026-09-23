//! Bounded canonical shape of Actions::AcceptAction's `accept ... via receiver`.
use super::*;
use agq_kerml_semantics::{
    ProducerClosedDependency, PublicationOverlayError, SemanticClosureRequirement,
    close_result_structure_with_extension,
};

#[test]
fn trigger_receiver_node_parameter_closes_with_nested_feature_value() {
    let (dependency, _, mut roots) = closed_kernel_anchor_fixture(true, false, false);
    let standard = dependency.context().standard_bindings.as_ref().unwrap();
    let occurrence = standard.get(StandardRole::Occurrence);
    let anything = standard.get(StandardRole::Anything);
    let (source, roles) = corpus_anchor_fixture_complete(true);
    let source = source.finish();
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        },
    };
    for record in source.model().elements() {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        f.changes
            .create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            f.changes
                .set(record.id(), property, slot.value().clone(), origin.clone());
            if property == kp::ELEMENT_OWNED_RELATIONSHIP {
                let SlotValue::Ordered(values) = slot.value() else {
                    unreachable!()
                };
                f.owned.insert(record.id(), values.clone());
            }
        }
    }
    roots.push(id(1));
    // The actual Action is an Occurrence. AcceptMessageAction contributes the
    // inherited inout payload / in receiver shape from AcceptPerformance.
    f.relation(
        roles[&StandardSysmlRole::Action],
        occurrence.as_u128(),
        260_000,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f.relation(
        roles[&StandardSysmlRole::Actions],
        roles[&StandardSysmlRole::Action],
        260_004,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    f.create(60_000, sc::ACTION_DEFINITION, "AcceptMessageAction");
    f.relation(
        60_000,
        roles[&StandardSysmlRole::Action],
        260_001,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f.create(60_001, kc::FEATURE, "payload");
    set_enum(&mut f, 60_001, kp::FEATURE_DIRECTION, "inout");
    f.member(60_000, 60_001, 160_001, kc::PARAMETER_MEMBERSHIP);
    f.create(60_002, kc::FEATURE, "receiver");
    set_enum(&mut f, 60_002, kp::FEATURE_DIRECTION, "in");
    f.member(60_000, 60_002, 160_002, kc::PARAMETER_MEMBERSHIP);
    f.relation(
        60_002,
        occurrence.as_u128(),
        260_002,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    f.relation(
        roles[&StandardSysmlRole::TransitionAccepter],
        60_000,
        260_003,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );

    f.create(60_010, sc::ACTION_DEFINITION, "AcceptAction");
    f.relation(
        60_010,
        60_000,
        260_010,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f.member(1, 60_010, 160_010, kc::OWNING_MEMBERSHIP);
    f.create(60_011, sc::STATE_USAGE, "aState");
    f.value(60_011, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(60_010, 60_011, 160_011, kc::FEATURE_MEMBERSHIP);
    f.create(60_012, sc::TRANSITION_USAGE, "aTransition");
    f.value(60_012, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(60_011, 60_012, 160_012, kc::FEATURE_MEMBERSHIP);
    for parameter in [60_018, 60_019] {
        f.create(parameter, sc::REFERENCE_USAGE, "");
        f.changes.clear(id(parameter), kp::ELEMENT_DECLARED_NAME);
        set_enum(&mut f, parameter, kp::FEATURE_DIRECTION, "in");
        f.member(
            60_012,
            parameter,
            parameter + 100_000,
            kc::PARAMETER_MEMBERSHIP,
        );
    }
    f.create(60_013, sc::ACCEPT_ACTION_USAGE, "");
    f.value(60_013, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(60_012, 60_013, 160_013, sc::TRANSITION_FEATURE_MEMBERSHIP);
    set_enum(
        &mut f,
        160_013,
        agq_sysml::properties::TRANSITION_FEATURE_MEMBERSHIP_KIND,
        "trigger",
    );
    // PayloadParameter and NodeParameter are ReferenceUsage under
    // ParameterMembership. Their grammar assigns no stored direction.
    f.create(60_014, sc::REFERENCE_USAGE, "apayload");
    f.member(60_013, 60_014, 160_014, kc::PARAMETER_MEMBERSHIP);
    f.relation(
        60_014,
        anything.as_u128(),
        260_014,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    f.create(60_015, sc::REFERENCE_USAGE, "");
    f.member(60_013, 60_015, 160_015, kc::PARAMETER_MEMBERSHIP);
    f.create(60_016, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(60_015, 60_016, 160_016, kc::FEATURE_VALUE);
    f.value(160_016, kp::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(false));
    f.value(160_016, kp::FEATURE_VALUE_IS_INITIAL, Value::Boolean(false));
    f.create(60_017, kc::FEATURE, "");
    set_enum(&mut f, 60_017, kp::FEATURE_DIRECTION, "out");
    f.member(60_016, 60_017, 160_017, kc::RETURN_PARAMETER_MEMBERSHIP);
    f.create(160_020, kc::MEMBERSHIP, "");
    f.value(
        160_020,
        kp::MEMBERSHIP_MEMBER_ELEMENT,
        Value::Reference(id(60_002)),
    );
    f.owned
        .entry(id(60_016))
        .or_default()
        .push(Value::Reference(id(160_020)));
    let snapshot = f.finish();
    let mut names = snapshot.change_set();
    for record in snapshot.model().elements() {
        if snapshot.is_dependency_element(record.id()) {
            continue;
        }
        if snapshot
            .model()
            .registry()
            .is_subtype(record.metaclass(), kc::MEMBERSHIP)
            .unwrap()
            || [id(60_013), id(60_015), id(60_016), id(60_017)].contains(&record.id())
        {
            names.clear(record.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    let snapshot = snapshot.apply(&names).unwrap();
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots.clone(),
    );
    fn context<'m>(
        overlay: &'m agq_kernel::derived::DerivedOverlay,
        dependency: &Arc<ProducerClosedDependency>,
        roots: &[ElementId],
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        let context = dependency
            .project_overlay_context(overlay, roots)
            .map_err(PublicationOverlayError::Context)?;
        Ok(crate::context::fixture_overlay_context(
            overlay,
            context,
            SysmlBaselineProfile::OPERATIONAL_V2,
        )
        .unwrap()
        .kerml)
    }
    let closed = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| context(overlay, &dependency, &roots),
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closed.converged);
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages.last()
    );
    let certificate = closed.certificate.unwrap();
    for subject in [60_013, 60_014, 60_015, 60_016] {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(
                certificate.is_closed(id(subject), requirement),
                "{subject}: {requirement:?}"
            );
        }
    }
    let queries = KerMlQueries::new(
        context(&closed.overlay, &dependency, &roots)
            .unwrap()
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap()
            .with_producer_closure(certificate)
            .unwrap(),
    );
    assert_eq!(queries.owning_type(id(60_015)).value, Some(id(60_013)));
    let types = queries.feature_types(id(60_014));
    assert_eq!(types.completeness, Completeness::Complete, "{types:?}");
    assert!(types.value.contains(&anything));
    assert!(closed.overlay.model().elements().any(|record| {
        queries.implied_binding_role(record.id())
            == Some(agq_kerml_semantics::ImpliedBindingRole::FeatureValue)
    }));
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
