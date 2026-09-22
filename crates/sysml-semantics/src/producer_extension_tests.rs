use super::*;
use crate::SystemsLibraryIdentity;
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, DerivationPhase, SemanticContext, SemanticOptions};
use agq_kernel::{
    Snapshot,
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use agq_sysml::{classes as sc, properties as sp};
use std::sync::Arc;

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}

fn fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    for (n, class, name) in [
        (1, kc::NAMESPACE, "root"),
        (2, kc::LIBRARY_PACKAGE, "Items"),
        (3, sc::ITEM_DEFINITION, "Item"),
        (4, sc::ITEM_DEFINITION, "AuthoredItem"),
        (5, sc::REFERENCE_USAGE, "unownedReference"),
        (10, kc::OWNING_MEMBERSHIP, ""),
        (11, kc::OWNING_MEMBERSHIP, ""),
    ] {
        changes.create(id(n), class, origin.clone());
        for property in base.model().registry().effective_properties(class).unwrap() {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match base
                .model()
                .registry()
                .storage_kind(property.value_kind)
                .unwrap()
            {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(name.into()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *base
                        .model()
                        .registry()
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                _ => panic!("fixture property domain"),
            };
            changes.set(id(n), property.id, SlotValue::Scalar(value), origin.clone());
        }
        changes.set(
            id(n),
            kp::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String(name.into())),
            origin.clone(),
        );
    }
    for (owner, membership, member) in [(1, 10, 2), (2, 11, 3)] {
        changes.set(
            id(owner),
            kp::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![Value::Reference(id(membership))]),
            origin.clone(),
        );
        changes.set(
            id(membership),
            kp::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(member))]),
            origin.clone(),
        );
    }
    base.apply(&changes).unwrap()
}

#[test]
fn reference_refinement_keeps_base_relationships_but_never_produces_stable_scalars() {
    let snapshot = fixture();
    let options = SemanticOptions {
        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
        ..Default::default()
    };
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), Default::default()).unwrap(),
    );
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned(
        SystemsLibraryIdentity::SOURCE_CONTENT_SET,
    ));
    let bootstrap = SysmlProducerExtension::for_reference_refinement(
        SysmlBaselineProfile::OperationalV1,
        bindings.clone(),
        vec![id(1)],
    );
    let full =
        SysmlProducerExtension::new(SysmlBaselineProfile::OperationalV1, bindings, vec![id(1)]);
    assert!(!bootstrap.has_stable_properties());
    assert!(full.has_stable_properties());
    for stratum in [
        ResultStructureStratum::Structural,
        ResultStructureStratum::StableProperties,
        ResultStructureStratum::ContextualBindings,
    ] {
        for (extension, scalar_expected) in [
            (&bootstrap, false),
            (&full, stratum != ResultStructureStratum::Structural),
        ] {
            let mut plan = q.plan_result_structure([]);
            for subject in [id(4), id(5)] {
                extension
                    .contribute(&q, subject, stratum, &mut plan)
                    .unwrap();
            }
            let result = plan.materialize(&snapshot).unwrap();
            assert_eq!(result.production.completeness, Completeness::Complete);
            let relationships: Vec<_> = result
                .overlay
                .model()
                .instances(kc::SUBCLASSIFICATION, true)
                .unwrap()
                .collect();
            assert_eq!(
                relationships.len(),
                1,
                "bootstrap must retain base specialization"
            );
            assert_eq!(
                result
                    .overlay
                    .model()
                    .navigation_slot(relationships[0].id(), kp::SUBCLASSIFICATION_SUPERCLASSIFIER)
                    .unwrap()
                    .value(),
                &SlotValue::Scalar(Value::Reference(id(3)))
            );
            let scalar = result
                .overlay
                .model()
                .navigation_slot(id(5), sp::USAGE_MAY_TIME_VARY);
            assert_eq!(scalar.is_some(), scalar_expected, "{stratum:?}");
            if let Some(scalar) = scalar {
                assert_eq!(scalar.value(), &SlotValue::Scalar(Value::Boolean(false)));
            }
            let context =
                SemanticContext::for_overlay(&result.overlay, options.clone(), Default::default())
                    .unwrap();
            assert_eq!(
                context.id().derivation_phase,
                DerivationPhase::PartialDerivationOverlay,
                "neither a mode nor a materialized plan confers publication acceptance"
            );
        }
    }
}
