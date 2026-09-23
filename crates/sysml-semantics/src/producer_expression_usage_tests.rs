//! Frontend-equivalent expression result shape from Items::Item::isSolid.
use super::*;
use agq_kerml_semantics::{
    ProducerClosedDependency, ProducerFamily, ProducerRegistry, PublicationOverlayError,
    SemanticClosureRequirement, close_result_structure_with_extension,
};

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

fn without_membership_names(snapshot: Snapshot) -> Snapshot {
    let mut changes = snapshot.change_set();
    for record in snapshot.model().elements() {
        if !snapshot.is_dependency_element(record.id())
            && snapshot
                .model()
                .registry()
                .is_subtype(record.metaclass(), kc::MEMBERSHIP)
                .unwrap()
        {
            changes.clear(record.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    snapshot.apply(&changes).unwrap()
}

/// Real scheduler closure of synthetic anchors, never a publication receipt.
fn expression_dependency() -> (Arc<ProducerClosedDependency>, Vec<ElementId>) {
    let (dependency, _, mut roots) = closed_kernel_anchor_fixture(true, false, false);
    let occurrence = dependency
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::Occurrence);
    let (source, roles) = corpus_anchor_fixture_complete(true);
    let source = source.finish();
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
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
    f.relation(
        roles[&StandardSysmlRole::Item],
        occurrence.as_u128(),
        271_000,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    // Closed Function signature corresponding to SequenceFunctions::isEmpty.
    // Its evaluation body is irrelevant to structural invocation semantics.
    f.create(71_000, kc::FUNCTION, "isEmpty");
    f.member(1, 71_000, 171_000, kc::OWNING_MEMBERSHIP);
    f.relation(
        71_000,
        occurrence.as_u128(),
        271_001,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    for (feature, name, direction, member) in [
        (71_001, "seq", "in", kc::PARAMETER_MEMBERSHIP),
        (71_002, "result", "out", kc::RETURN_PARAMETER_MEMBERSHIP),
    ] {
        f.create(feature, kc::FEATURE, name);
        set_enum(&mut f, feature, kp::FEATURE_DIRECTION, direction);
        f.member(71_000, feature, feature + 100_000, member);
    }
    let snapshot = without_membership_names(f.finish());
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots.clone(),
    );
    let closed = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| context(overlay, &dependency, &roots),
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "dependency closure {:?}",
        closed.stages.last()
    );
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9))
            .chain(sysml_producer_descriptors()),
    )
    .unwrap();
    let overlay = Arc::new(closed.overlay);
    let context = context(&overlay, &dependency, &roots)
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
        .with_producer_closure(closed.certificate.unwrap())
        .unwrap();
    let dependency = ProducerClosedDependency::new(overlay.clone(), &context, &registry).unwrap();
    drop(context);
    (dependency, roots)
}

fn reference(f: &mut Fixture, owner: u128, target: u128, membership: u128) {
    f.create(membership, kc::MEMBERSHIP, "");
    f.value(
        membership,
        kp::MEMBERSHIP_MEMBER_ELEMENT,
        Value::Reference(id(target)),
    );
    f.owned
        .entry(id(owner))
        .or_default()
        .push(Value::Reference(id(membership)));
}

fn invocation_value(result_class: MetaclassId) {
    let (dependency, roots) = expression_dependency();
    let occurrence = dependency
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::Occurrence);
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(72_000, sc::ITEM_DEFINITION, "Container");
    f.relation(
        72_000,
        occurrence.as_u128(),
        272_000,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f.create(72_001, sc::ITEM_USAGE, "voids");
    f.member(72_000, 72_001, 172_001, kc::FEATURE_MEMBERSHIP);
    f.create(72_002, sc::ATTRIBUTE_USAGE, "isSolid");
    f.member(72_000, 72_002, 172_002, kc::FEATURE_MEMBERSHIP);
    f.create(72_003, kc::INVOCATION_EXPRESSION, "");
    f.member(72_002, 72_003, 172_003, kc::FEATURE_VALUE);
    reference(&mut f, 72_003, 71_000, 172_004);
    // Argument/ArgumentValue contains the FeatureReferenceExpression `voids`.
    f.create(72_004, kc::FEATURE, "");
    set_enum(&mut f, 72_004, kp::FEATURE_DIRECTION, "in");
    f.member(72_003, 72_004, 172_005, kc::PARAMETER_MEMBERSHIP);
    f.create(72_005, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(72_004, 72_005, 172_006, kc::FEATURE_VALUE);
    reference(&mut f, 72_005, 72_001, 172_007);
    // SysML EmptyResultMember/EmptyFeature canonically lowers to ReferenceUsage.
    // The plain Feature variant preserves the former synthetic fixture shape.
    for (owner, feature, membership) in [(72_003, 72_006, 172_008), (72_005, 72_007, 172_009)] {
        f.create(feature, result_class, "");
        set_enum(&mut f, feature, kp::FEATURE_DIRECTION, "out");
        f.member(owner, feature, membership, kc::RETURN_PARAMETER_MEMBERSHIP);
    }
    for subject in 72_003..=72_007 {
        f.changes.clear(id(subject), kp::ELEMENT_DECLARED_NAME);
    }
    let snapshot = without_membership_names(f.finish());
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots.clone(),
    );
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
        "result class {result_class:?}: {:?}",
        closed.stages.last()
    );
    let certificate = closed.certificate.unwrap();
    for subject in 72_001..=72_007 {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(
                certificate.is_closed(id(subject), requirement),
                "{subject}: {requirement:?}"
            );
        }
    }
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}

#[test]
fn invocation_value_with_plain_feature_results_control() {
    invocation_value(kc::FEATURE);
}

#[test]
fn invocation_value_with_frontend_reference_usage_results_closes() {
    invocation_value(sc::REFERENCE_USAGE);
}
