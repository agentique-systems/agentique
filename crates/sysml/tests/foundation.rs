use agq_kernel::{metamodel::*, provenance::DeclaredOrigin, value::*, *};
use std::{collections::BTreeSet, sync::Arc};

#[test]
fn complete_graph_preserves_every_identity_and_shuffled_registration() {
    let set = agq_sysml::descriptors();
    assert_eq!(
        (
            set.classes.len(),
            set.properties.len(),
            set.associations.len(),
            set.enumerations.len()
        ),
        (175, 715, 319, 7)
    );
    assert_eq!(agq_sysml::own_descriptors().classes.len(), 93);
    assert_eq!(agq_kerml::descriptors().classes.len(), 82);
    assert_eq!(set.primitives.len(), 4);
    let original = agq_sysml::registry().unwrap();
    let inherited: Vec<_> = set
        .associations
        .iter()
        .filter(|a| !a.direct_supertypes.is_empty())
        .collect();
    assert_eq!(inherited.len(), 1);
    assert_eq!(inherited[0].name, "A_participantFeature_Interaction");
    let parent = *inherited[0].direct_supertypes.first().unwrap();
    assert_eq!(
        original.association(parent).unwrap().name,
        "A_participantFeature_Association"
    );
    assert_eq!(
        original
            .association_ancestry(inherited[0].id)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        original
            .inherited_association_ends(inherited[0].id)
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        original
            .effective_association_ends(inherited[0].id)
            .unwrap()
            .len(),
        3
    );
    let expected = format!("{original:?}");
    for salt in 0..12_u128 {
        let mut shuffled = set.clone();
        shuffled
            .classes
            .sort_by_key(|v| v.id.as_u128().wrapping_mul(3 + salt * 2));
        shuffled
            .properties
            .sort_by_key(|v| v.id.as_u128().wrapping_mul(7 + salt * 2));
        shuffled
            .associations
            .sort_by_key(|v| v.id.as_u128().wrapping_mul(11 + salt * 2));
        shuffled.enumerations.reverse();
        shuffled.primitives.reverse();
        shuffled.models.reverse();
        shuffled.reviews.reverse();
        let registry = MetamodelRegistry::from_descriptors(shuffled).unwrap();
        assert_eq!(format!("{registry:?}"), expected);
        for c in &set.classes {
            assert_eq!(registry.class(c.id).unwrap(), c);
            let props: Vec<_> = registry.effective_properties(c.id).unwrap().collect();
            for p in props {
                assert_eq!(registry.resolve_property(c.id, p.id).unwrap(), Some(p));
            }
        }
    }
}

#[test]
fn exact_defined_flow_metadata_registers_but_never_replaces_a_class_slot() {
    let registry = agq_sysml::registry().unwrap();
    let property = PropertyId::from_u128(0x2abb22848e2551acb4861792cc60e1b1);
    let base = PropertyId::from_u128(0x2e4efe582d095275991e104649d59bf3);
    let p = registry.property(property).unwrap();
    let PropertyOwner::Association(owner) = p.owner else {
        panic!("association-owned source")
    };
    assert!(p.derived);
    assert!(!registry.is_navigable(property).unwrap());
    assert_eq!(
        registry.declared_redefinitions(property).unwrap(),
        &BTreeSet::from([base])
    );
    let source = registry.source(DescriptorId::Property(property)).unwrap();
    assert_eq!(
        source.external_id,
        "Systems-Flows-A_flowDefinition_definedFlow-definedFlow"
    );
    assert_eq!(
        source.sha256,
        "caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f"
    );
    for c in &agq_sysml::descriptors().classes {
        assert!(!registry.is_legal(c.id, property).unwrap());
        assert!(
            registry
                .effective_redefinitions_for_class(property, c.id)
                .unwrap()
                .is_empty()
        );
    }
    assert!(
        registry
            .resolve_property(agq_sysml::classes::DEFINITION, base)
            .unwrap()
            .is_some()
    );
    assert!(
        registry
            .interpret_association_redefinition(property, base, owner)
            .is_err()
    );
    let report = registry.validate_conformance();
    let d = report
        .diagnostics
        .iter()
        .find(|d| {
            d.subject == DescriptorId::Property(property)
                && d.rule == MetamodelRule::RedefinitionContext
        })
        .unwrap();
    assert!(matches!(
        d.disposition,
        DiagnosticDisposition::ReviewedBaselineAnomaly { .. }
    ));
    assert!(report.require_conformance().is_err());
    // The disposition cannot propagate to another source content identity.
    let mut changed = agq_sysml::descriptors();
    changed
        .sources
        .get_mut(&DescriptorId::Property(property))
        .unwrap()
        .sha256 = "0".repeat(64);
    let changed = MetamodelRegistry::from_descriptors(changed)
        .unwrap()
        .validate_conformance();
    assert!(
        changed
            .diagnostics
            .iter()
            .any(|d| d.subject == DescriptorId::Property(property)
                && d.disposition == DiagnosticDisposition::Unreviewed)
    );
}

#[test]
fn sysml_views_borrow_one_kernel_record_and_upcast_across_language_boundary() {
    let registry = Arc::new(agq_sysml::registry().unwrap());
    let base = Snapshot::new(registry.clone());
    let id = ElementId::from_u128(1);
    let origin = DeclaredOrigin::Authored { source: None };
    let mut edits = base.change_set();
    let class = agq_sysml::classes::PART_DEFINITION;
    edits.create(id, class, origin.clone());
    for p in registry
        .effective_properties(class)
        .unwrap()
        .filter(|p| !p.derived && p.multiplicity.lower > 0)
    {
        let value = match registry.storage_kind(p.value_kind).unwrap() {
            ValueKind::Boolean => Value::Boolean(false),
            ValueKind::String => Value::String("part".into()),
            kind => panic!("required domain {kind:?}"),
        };
        edits.set(id, p.id, SlotValue::Scalar(value), origin.clone());
    }
    let snapshot = base.apply(&edits).unwrap();
    let part = agq_sysml::views::PartDefinition::try_new(id, snapshot.model()).unwrap();
    let element: agq_kerml::views::Element<'_> = part.as_element().unwrap();
    assert_eq!(part.id(), element.id());
    assert!(std::ptr::eq(part.record(), element.record()));
    assert_eq!(
        std::mem::size_of_val(&part),
        std::mem::size_of::<(ElementId, &ModelView)>()
    );
}
