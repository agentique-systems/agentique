//! Frontend-equivalent expression result shape from Items::Item::isSolid.
use super::*;
#[path = "producer_index_usage_tests.rs"]
mod index_usage_tests;
#[path = "producer_inline_function_tests.rs"]
mod inline_function_tests;
#[path = "producer_other_participants_tests.rs"]
mod other_participants_tests;
#[path = "producer_verification_case_tests.rs"]
mod verification_case_tests;
#[path = "producer_views_satisfaction_tests.rs"]
mod views_satisfaction_tests;
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

/// Opt-in, bounded diagnostic for exact retained proof roots from a causal trace.
fn trace_helper_origins(overlay: &agq_kernel::derived::DerivedOverlay) {
    use agq_kernel::provenance::{FactKey, Origin};
    let Ok(selected) = std::env::var("AGQ_EXPRESSION_HELPER_ORIGINS") else {
        return;
    };
    let model = overlay.model();
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9))
            .chain(sysml_producer_descriptors()),
    )
    .unwrap();
    for encoded in selected.split(',').take(4) {
        let subject = ElementId::from_u128(
            u128::from_str_radix(&encoded.trim().replace('-', ""), 16).unwrap(),
        );
        let Some(record) = model.element(subject) else {
            eprintln!("helper-origin {subject}: absent");
            continue;
        };
        eprintln!(
            "helper-origin {subject}: class={} ({})",
            model.registry().class(record.metaclass()).unwrap().name,
            record.metaclass()
        );
        if let Origin::Derived(proof) = record.origin() {
            let families: Vec<_> = registry
                .descriptors()
                .iter()
                .filter(|descriptor| {
                    descriptor
                        .derivation_rules
                        .as_ref()
                        .is_some_and(|rules| rules.contains(&proof.rule))
                })
                .map(|descriptor| descriptor.id.name())
                .collect();
            eprintln!(
                "helper-proof rule={} families={families:?} dependencies={} (first 64)",
                proof.rule,
                proof.dependencies.len()
            );
            for dependency in proof.dependencies.iter().take(64) {
                eprintln!("helper-dependency {dependency:?}");
            }
        }
        for (property, slot) in record.slots().take(12) {
            eprintln!(
                "helper-slot {}: {:?}",
                model.registry().property(property).unwrap().name,
                slot.value()
            );
        }
        for reference in model.incoming(subject).filter(|reference| {
            [
                kp::ELEMENT_OWNED_RELATIONSHIP,
                kp::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ]
            .contains(&reference.property)
        }) {
            eprintln!("helper-owner {reference:?}");
        }
        for search in model
            .computation_searches_for(FactKey::Element(subject))
            .filter(|search| {
                matches!(
                    search,
                    agq_kernel::derived::StructuralSearch::OwnedRelationships { owner, .. }
                    if (73_000..=73_019).contains(&owner.as_u128())
                )
            })
            .take(32)
        {
            if let agq_kernel::derived::StructuralSearch::OwnedRelationships { owner, class } =
                search
            {
                eprintln!(
                    "helper-owned-search {owner} class={} ({class})",
                    model.registry().class(*class).unwrap().name
                );
            }
        }
    }
}

/// Real scheduler closure of synthetic anchors, never a publication receipt.
fn expression_dependency() -> (Arc<ProducerClosedDependency>, Vec<ElementId>) {
    expression_dependency_with_signature("isEmpty", &["seq"])
}

fn expression_dependency_with_signature(
    function: &str,
    parameters: &[&str],
) -> (Arc<ProducerClosedDependency>, Vec<ElementId>) {
    let (dependency, _, roots) = closed_kernel_anchor_fixture(true, false, false);
    expression_dependency_from_kernel(dependency, roots, function, parameters)
}

fn expression_dependency_from_kernel(
    dependency: Arc<ProducerClosedDependency>,
    roots: Vec<ElementId>,
    function: &str,
    parameters: &[&str],
) -> (Arc<ProducerClosedDependency>, Vec<ElementId>) {
    expression_dependency_from_kernel_with(dependency, roots, function, parameters, |_, _, _| {})
}

fn expression_dependency_from_kernel_with(
    dependency: Arc<ProducerClosedDependency>,
    mut roots: Vec<ElementId>,
    function: &str,
    parameters: &[&str],
    customize: impl FnOnce(&mut Fixture, &BTreeMap<StandardSysmlRole, u128>, ElementId),
) -> (Arc<ProducerClosedDependency>, Vec<ElementId>) {
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
    // Closed Function signatures corresponding to isEmpty/contains.
    // Its evaluation body is irrelevant to structural invocation semantics.
    f.create(71_000, kc::FUNCTION, function);
    f.member(1, 71_000, 171_000, kc::OWNING_MEMBERSHIP);
    f.relation(
        71_000,
        occurrence.as_u128(),
        271_001,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    for (index, (name, direction, member)) in parameters
        .iter()
        .map(|&name| (name, "in", kc::PARAMETER_MEMBERSHIP))
        .chain([("result", "out", kc::RETURN_PARAMETER_MEMBERSHIP)])
        .enumerate()
    {
        let feature = 71_001 + index as u128;
        f.create(feature, kc::FEATURE, name);
        set_enum(&mut f, feature, kp::FEATURE_DIRECTION, direction);
        f.member(71_000, feature, feature + 100_000, member);
    }
    customize(&mut f, &roles, occurrence);
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
    assert!(
        certificate.is_fully_closed(closed.overlay.model()),
        "result class {result_class:?}: the full graph must be producer-closed"
    );
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

fn invocation_chain_value(nested: bool) {
    // Items::Item::boundingShapes::faces has contains(inter.intersectionsOf, ...).
    // Preserve the nested expression/argument/result ownership shape, using a
    // direct second argument rather than adding the unrelated union invocation.
    let (dependency, roots) = expression_dependency_with_signature("contains", &["col", "values"]);
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
    f.create(73_000, sc::ITEM_DEFINITION, "Container");
    f.relation(
        73_000,
        occurrence.as_u128(),
        273_000,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    for (owner, feature, class, name) in [
        (73_000, 73_001, sc::ITEM_USAGE, "inter"),
        (73_001, 73_002, sc::ITEM_USAGE, "intersectionsOf"),
        (73_000, 73_003, sc::ITEM_USAGE, "face"),
        (73_000, 73_004, sc::ATTRIBUTE_USAGE, "isContained"),
    ] {
        f.create(feature, class, name);
        f.member(owner, feature, feature + 100_000, kc::FEATURE_MEMBERSHIP);
    }
    if nested {
        f.create(73_019, sc::ITEM_USAGE, "otherFeature");
        f.member(73_002, 73_019, 173_019, kc::FEATURE_MEMBERSHIP);
    }
    for (owner, expression, class) in [
        (73_004, 73_005, kc::INVOCATION_EXPRESSION),
        (73_006, 73_007, kc::FEATURE_CHAIN_EXPRESSION),
        (
            if nested { 73_017 } else { 73_008 },
            73_009,
            kc::FEATURE_REFERENCE_EXPRESSION,
        ),
        (73_012, 73_013, kc::FEATURE_REFERENCE_EXPRESSION),
    ] {
        f.create(expression, class, "");
        f.member(owner, expression, expression + 100_000, kc::FEATURE_VALUE);
    }
    if nested {
        // contains(inter.intersectionsOf.otherFeature, face): the outer chain's
        // first input is valued by a second chain, which has its own input,
        // direct referent membership and frontend ReferenceUsage result.
        f.create(73_016, kc::FEATURE_CHAIN_EXPRESSION, "");
        f.value(
            73_016,
            kp::FEATURE_CHAIN_EXPRESSION_OPERATOR,
            Value::String(".".into()),
        );
        f.member(73_008, 73_016, 173_016, kc::FEATURE_VALUE);
        f.create(73_017, kc::FEATURE, "");
        set_enum(&mut f, 73_017, kp::FEATURE_DIRECTION, "in");
        f.member(73_016, 73_017, 173_017, kc::PARAMETER_MEMBERSHIP);
        f.create(73_018, sc::REFERENCE_USAGE, "");
        set_enum(&mut f, 73_018, kp::FEATURE_DIRECTION, "out");
        f.member(73_016, 73_018, 173_018, kc::RETURN_PARAMETER_MEMBERSHIP);
        reference(&mut f, 73_016, 73_002, 173_024);
    }
    f.value(
        73_007,
        kp::FEATURE_CHAIN_EXPRESSION_OPERATOR,
        Value::String(".".into()),
    );
    for (owner, parameter, direction, member) in [
        (73_005, 73_006, "in", kc::PARAMETER_MEMBERSHIP),
        (73_007, 73_008, "in", kc::PARAMETER_MEMBERSHIP),
        (73_009, 73_010, "out", kc::RETURN_PARAMETER_MEMBERSHIP),
        (73_007, 73_011, "out", kc::RETURN_PARAMETER_MEMBERSHIP),
        (73_005, 73_012, "in", kc::PARAMETER_MEMBERSHIP),
        (73_013, 73_014, "out", kc::RETURN_PARAMETER_MEMBERSHIP),
        (73_005, 73_015, "out", kc::RETURN_PARAMETER_MEMBERSHIP),
    ] {
        let class = if direction == "out" {
            sc::REFERENCE_USAGE
        } else {
            kc::FEATURE
        };
        f.create(parameter, class, "");
        set_enum(&mut f, parameter, kp::FEATURE_DIRECTION, direction);
        f.member(owner, parameter, parameter + 100_000, member);
    }
    for (expression, target, membership) in [
        (73_005, 71_000, 173_020),
        (73_007, if nested { 73_019 } else { 73_002 }, 173_021),
        (73_009, 73_001, 173_022),
        (73_013, 73_003, 173_023),
    ] {
        reference(&mut f, expression, target, membership);
    }
    for subject in 73_005..=if nested { 73_018 } else { 73_015 } {
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
    if closed.completeness != Completeness::Complete {
        trace_helper_origins(&closed.overlay);
        let registry = ProducerRegistry::new(
            ProducerFamily::ALL
                .into_iter()
                .map(|family| family.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9))
                .chain(sysml_producer_descriptors()),
        )
        .unwrap();
        let certificate = closed.certificate.as_ref().unwrap();
        for record in closed.overlay.model().elements() {
            for (index, descriptor) in registry.descriptors().iter().enumerate() {
                if certificate.evaluation(record.id(), index)
                    == Some(agq_kerml_semantics::ProducerEvaluationState::EvaluatedIncomplete)
                {
                    eprintln!("incomplete {} / {}", record.id(), descriptor.id.name());
                }
            }
        }
    }
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "feature-chain expression (nested={nested}): {:?}",
        closed.stages.last()
    );
    let certificate = closed.certificate.unwrap();
    assert!(certificate.is_fully_closed(closed.overlay.model()));
    for subject in 73_001..=if nested { 73_019 } else { 73_015 } {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(
                certificate.is_closed(id(subject), requirement),
                "{subject}: {requirement:?}"
            );
        }
    }
}

#[test]
fn invocation_chain_value_with_frontend_reference_usage_results_closes() {
    invocation_chain_value(false);
}

#[test]
fn invocation_nested_chain_with_frontend_reference_usage_results_closes() {
    invocation_chain_value(true);
}
