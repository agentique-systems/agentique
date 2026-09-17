use agq_kernel::{metamodel::*, provenance::DeclaredOrigin, value::*, *};
use std::{collections::BTreeSet, sync::Arc};

fn class(key: &str) -> MetaclassId {
    agq_kerml::CLASS_IDS
        .iter()
        .find(|(id, _)| *id == key)
        .unwrap()
        .1
}
fn property(key: &str) -> PropertyId {
    agq_kerml::PROPERTY_IDS
        .iter()
        .find(|(id, _)| *id == key)
        .unwrap()
        .1
}

#[test]
fn normative_redefinitions_resolve_inherited_names_and_replaced_slots() {
    let r = agq_kerml::registry().unwrap();
    let owning = class("Root-Namespaces-OwningMembership");
    let base = property("Root-Namespaces-Membership-memberName");
    let replacement = property("Root-Namespaces-OwningMembership-ownedMemberName");
    assert_eq!(
        r.resolve_property(owning, base).unwrap().unwrap().id,
        replacement
    );
    assert_eq!(
        r.property_named(owning, "ownedMemberName")
            .unwrap()
            .unwrap()
            .id,
        replacement
    );
    assert!(r.property_named(owning, "memberName").unwrap().is_none());
    assert!(!r.is_legal(owning, base).unwrap());
    assert!(r.is_legal(owning, replacement).unwrap());
    // Transitive narrowing: Relationship.target -> Membership.memberElement ->
    // OwningMembership.ownedMemberElement -> FeatureMembership.ownedMemberFeature.
    let feature_membership = class("Core-Types-FeatureMembership");
    let resolved = r
        .resolve_property(
            feature_membership,
            property("Root-Elements-Relationship-target"),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        resolved.id,
        property("Core-Types-FeatureMembership-ownedMemberFeature")
    );
    assert_eq!(
        resolved.value_kind,
        ValueKind::Reference(class("Core-Features-Feature"))
    );
    assert!(resolved.derived);
}

#[test]
fn normative_subsets_opposites_and_unions_remain_distinct_metadata() {
    let r = agq_kerml::registry().unwrap();
    let p = r
        .property(property("Root-Namespaces-Namespace-ownedMembership"))
        .unwrap();
    let union = property("Root-Namespaces-Namespace-membership");
    assert!(p.subsets.contains(&union));
    assert!(p.redefines.is_empty());
    assert!(r.property(union).unwrap().derived_union);
    assert!(!p.derived_union);
    let set = agq_kerml::descriptors();
    assert_eq!(set.properties.iter().filter(|p| p.derived_union).count(), 3);
    for p in &set.properties {
        for other in &p.opposite_ends {
            let q = r.property(*other).unwrap();
            assert!(q.opposite_ends.contains(&p.id));
            assert_eq!(q.association, p.association);
        }
    }
    let inverse = property("Root-Elements-A_relatedElement_relationship-relationship");
    let p = r.property(inverse).unwrap();
    assert!(matches!(p.owner, PropertyOwner::Association(_)));
    assert!(p.derived_union && !p.unique);
    assert!(!r.is_legal(class("Root-Elements-Element"), inverse).unwrap());
    assert!(!r.is_navigable(inverse).unwrap());
    assert!(
        r.is_navigable(property("Root-Elements-Relationship-relatedElement"))
            .unwrap()
    );
}

#[test]
fn both_nonderived_ownership_ends_are_preserved_without_inventing_composition() {
    let r = agq_kerml::registry().unwrap();
    for (left, right) in [
        (
            "Root-Elements-Element-ownedRelationship",
            "Root-Elements-Relationship-owningRelatedElement",
        ),
        (
            "Root-Elements-Element-owningRelationship",
            "Root-Elements-Relationship-ownedRelatedElement",
        ),
    ] {
        let p = r.property(property(left)).unwrap();
        let q = r.property(property(right)).unwrap();
        assert!(!p.derived && !q.derived);
        assert!(!p.composite && !q.composite);
        assert_eq!(p.opposite_ends, BTreeSet::from([q.id]));
        assert_eq!(q.opposite_ends, BTreeSet::from([p.id]));
    }
    assert!(
        agq_kerml::descriptors()
            .properties
            .iter()
            .all(|p| !p.composite)
    );
}

#[test]
fn association_storage_and_derived_writes_fail_atomically() {
    let r = Arc::new(agq_kerml::registry().unwrap());
    let before = Snapshot::new(r);
    let id = ElementId::new();
    for (key, derived) in [
        ("Root-Elements-Element-owningRelationship", false),
        ("Root-Elements-Element-ownedElement", true),
        ("Root-Namespaces-Namespace-membership", true),
    ] {
        let mut change = before.change_set();
        change.create(
            id,
            class("Root-Namespaces-Namespace"),
            DeclaredOrigin::Authored { source: None },
        );
        change.set(
            id,
            property(key),
            SlotValue::Ordered(vec![]),
            DeclaredOrigin::Authored { source: None },
        );
        let error = before.apply(&change).unwrap_err();
        if derived {
            assert!(matches!(error, ModelError::DerivedWrite { .. }));
        } else {
            assert!(matches!(
                error,
                ModelError::UnsupportedAssociationStorage(_)
            ));
        }
        assert!(before.model().is_empty());
    }
}

#[test]
fn corrupt_normative_metadata_is_rejected() {
    let pid = property("Core-Types-FeatureMembership-ownedMemberFeature");
    let mut input = agq_kerml::descriptors();
    input
        .properties
        .iter_mut()
        .find(|p| p.id == pid)
        .unwrap()
        .value_kind = ValueKind::String;
    assert!(MetamodelRegistry::from_descriptors(input).is_err());
    let mut input = agq_kerml::descriptors();
    input
        .properties
        .iter_mut()
        .find(|p| p.id == pid)
        .unwrap()
        .opposite_ends
        .clear();
    assert!(matches!(
        MetamodelRegistry::from_descriptors(input),
        Err(MetamodelError::InvalidAssociation(_))
    ));
    let mut input = agq_kerml::descriptors();
    input
        .properties
        .iter_mut()
        .find(|p| p.derived_union)
        .unwrap()
        .derived = false;
    assert!(matches!(
        MetamodelRegistry::from_descriptors(input),
        Err(MetamodelError::InvalidPropertyMetadata(_))
    ));
    let mut input = agq_kerml::descriptors();
    input
        .properties
        .iter_mut()
        .find(|p| p.id == pid)
        .unwrap()
        .redefines
        .insert(PropertyId::new());
    assert!(matches!(
        MetamodelRegistry::from_descriptors(input),
        Err(MetamodelError::UnknownProperty(_))
    ));
}

#[test]
fn normative_enum_literals_are_checked_against_the_property_domain() {
    let registry = Arc::new(agq_kerml::registry().unwrap());
    let feature = class("Core-Features-Feature");
    let direction = property("Core-Features-Feature-direction");
    let set = agq_kerml::descriptors();
    let direction_literal = *set
        .enumerations
        .iter()
        .find(|e| e.name == "FeatureDirectionKind")
        .unwrap()
        .literals
        .keys()
        .next()
        .unwrap();
    let visibility_literal = *set
        .enumerations
        .iter()
        .find(|e| e.name == "VisibilityKind")
        .unwrap()
        .literals
        .keys()
        .next()
        .unwrap();
    for (literal, valid) in [(direction_literal, true), (visibility_literal, false)] {
        let before = Snapshot::new(registry.clone());
        let id = ElementId::new();
        let origin = DeclaredOrigin::Authored { source: None };
        let mut change = before.change_set();
        change.create(id, feature, origin.clone());
        for p in registry
            .effective_properties(feature)
            .unwrap()
            .filter(|p| !p.derived && p.multiplicity.lower > 0)
        {
            let value = match p.value_kind {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String("structural-test".into()),
                _ => panic!("unexpected required primitive"),
            };
            change.set(id, p.id, SlotValue::Scalar(value), origin.clone());
        }
        change.set(
            id,
            direction,
            SlotValue::Scalar(Value::Enumeration(literal)),
            origin,
        );
        let result = before.apply(&change);
        if valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(matches!(result, Err(ModelError::ValueKind { .. })));
        }
    }
}
