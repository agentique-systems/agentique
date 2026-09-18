use agq_kerml::{ViewError, classes as c, properties as p, views::*};
use agq_kernel::provenance::Dependency;
use agq_kernel::{
    derived::{DerivationBuilder, DerivationError},
    metamodel::*,
    provenance::*,
    value::*,
    *,
};
use std::{collections::BTreeSet, sync::Arc};

const BASE: ElementId = ElementId::from_u128(1);
const SPECIFIC: ElementId = ElementId::from_u128(2);
const F1: ElementId = ElementId::from_u128(3);
const F2: ElementId = ElementId::from_u128(4);
const MEMBER: ElementId = ElementId::from_u128(5);
const SPECIALIZES: ElementId = ElementId::from_u128(6);
const TYPING: ElementId = ElementId::from_u128(7);
const SUBSETS: ElementId = ElementId::from_u128(8);
const REDEFINES: ElementId = ElementId::from_u128(9);
const OWNS: ElementId = ElementId::from_u128(10);

fn authored() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
fn reference(id: ElementId) -> SlotValue {
    SlotValue::Scalar(Value::Reference(id))
}
fn text(value: &str) -> SlotValue {
    SlotValue::Scalar(Value::String(value.into()))
}
fn evidence() -> Explanation {
    // Explicit fixture assertions, NOT an implementation of normative derivation rules.
    Explanation {
        rule: RuleId::from_u128(1),
        dependencies: BTreeSet::from([
            Dependency::Declared(FactKey::Element(F1)),
            Dependency::Declared(FactKey::Element(F2)),
        ]),
    }
}

fn fixture() -> Snapshot {
    let registry = Arc::new(agq_kerml::registry().unwrap());
    let empty = Snapshot::new(registry.clone());
    let mut change = empty.change_set();
    for (id, class) in [
        (BASE, c::TYPE),
        (SPECIFIC, c::TYPE),
        (F1, c::FEATURE),
        (F2, c::FEATURE),
        (MEMBER, c::MEMBERSHIP),
        (SPECIALIZES, c::SPECIALIZATION),
        (TYPING, c::FEATURE_TYPING),
        (SUBSETS, c::SUBSETTING),
        (REDEFINES, c::REDEFINITION),
        (OWNS, c::FEATURE_MEMBERSHIP),
    ] {
        change.create(id, class, authored());
        for property in registry
            .effective_properties(class)
            .unwrap()
            .filter(|p| !p.derived && p.multiplicity.lower > 0)
        {
            let value = match registry.storage_kind(property.value_kind).unwrap() {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(format!("fixture-{id}")),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *registry
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .keys()
                        .next()
                        .unwrap(),
                ),
                ValueKind::Reference(_) => continue, // Endpoints authored explicitly below.
                other => panic!("unexpected required domain {other:?}"),
            };
            change.set(id, property.id, SlotValue::Scalar(value), authored());
        }
    }
    for (id, property, target) in [
        (MEMBER, p::MEMBERSHIP_MEMBER_ELEMENT, F1),
        (SPECIALIZES, p::SPECIALIZATION_SPECIFIC, SPECIFIC),
        (SPECIALIZES, p::SPECIALIZATION_GENERAL, BASE),
        (TYPING, p::FEATURE_TYPING_TYPED_FEATURE, F1),
        (TYPING, p::FEATURE_TYPING_TYPE, BASE),
        (SUBSETS, p::SUBSETTING_SUBSETTING_FEATURE, F2),
        (SUBSETS, p::SUBSETTING_SUBSETTED_FEATURE, F1),
        (REDEFINES, p::REDEFINITION_REDEFINING_FEATURE, F2),
        (REDEFINES, p::REDEFINITION_REDEFINED_FEATURE, F1),
    ] {
        change.set(id, property, reference(target), authored());
    }
    change.set(F1, p::ELEMENT_DECLARED_NAME, text("speed"), authored());
    change.set(
        F1,
        p::ELEMENT_ALIAS_IDS,
        SlotValue::Ordered(vec![Value::String("z".into()), Value::String("a".into())]),
        authored(),
    );
    let snapshot = empty.apply(&change).unwrap();
    assert!(empty.model().is_empty());
    snapshot
}

#[test]
fn relationships_are_first_class_records_and_views_share_canonical_data() {
    let snapshot = fixture();
    let model = snapshot.model();
    assert_eq!(model.len(), 10);
    assert_eq!(model.instances(c::RELATIONSHIP, true).unwrap().count(), 6);
    let feature = Feature::try_new(F1, model).unwrap();
    assert_eq!(feature.id(), F1);
    assert!(std::ptr::eq(feature.model(), model));
    assert!(std::ptr::eq(feature.record(), model.element(F1).unwrap()));
    let SlotValue::Scalar(Value::String(generic)) = feature
        .record()
        .slot(p::ELEMENT_DECLARED_NAME)
        .unwrap()
        .value()
    else {
        panic!()
    };
    let typed = feature.declared_name().unwrap().unwrap();
    assert_eq!(typed, "speed");
    assert_eq!(typed.as_ptr(), generic.as_ptr());
    assert_eq!(
        std::mem::size_of::<Feature<'_>>(),
        std::mem::size_of::<(ElementId, &ModelView)>()
    );

    let typing = FeatureTyping::try_new(TYPING, model).unwrap();
    assert_eq!(typing.typed_feature().unwrap(), F1);
    assert_eq!(typing.r#type().unwrap(), BASE);
    assert_eq!(typing.as_specialization().unwrap().specific().unwrap(), F1);
    assert_eq!(typing.as_specialization().unwrap().general().unwrap(), BASE);
    let relationship = typing.as_relationship().unwrap();
    assert_eq!(relationship.id(), TYPING);
    let source = relationship.source().unwrap().unwrap();
    assert_eq!(source.iter().collect::<Vec<_>>(), vec![F1]);
    assert_eq!(source.shape(), SlotShape::Scalar); // Normative narrowing, not a second array.
    assert_eq!(source.descriptor().id, p::FEATURE_TYPING_TYPED_FEATURE);
    assert!(std::ptr::eq(
        source.raw(),
        typing
            .record()
            .slot(p::FEATURE_TYPING_TYPED_FEATURE)
            .unwrap()
            .value()
    ));
    assert!(typing.record().slot(p::SPECIALIZATION_SPECIFIC).is_none());
    assert!(typing.record().slot(p::RELATIONSHIP_SOURCE).is_none());
    assert_eq!(model.outgoing(TYPING).count(), 2);
    assert_eq!(model.incoming(F1).filter(|r| r.source == TYPING).count(), 1);

    assert_eq!(
        Membership::try_new(MEMBER, model)
            .unwrap()
            .member_element()
            .unwrap(),
        F1
    );
    let specialization = Specialization::try_new(SPECIALIZES, model).unwrap();
    assert_eq!(
        (
            specialization.specific().unwrap(),
            specialization.general().unwrap()
        ),
        (SPECIFIC, BASE)
    );
    let subsetting = Subsetting::try_new(SUBSETS, model).unwrap();
    assert_eq!(
        (
            subsetting.subsetting_feature().unwrap(),
            subsetting.subsetted_feature().unwrap()
        ),
        (F2, F1)
    );
    let redefinition = Redefinition::try_new(REDEFINES, model).unwrap();
    assert_eq!(redefinition.redefining_feature().unwrap(), F2);
    assert_eq!(redefinition.redefined_feature().unwrap(), F1);
    assert_eq!(
        redefinition
            .as_subsetting()
            .unwrap()
            .subsetted_feature()
            .unwrap(),
        F1
    );
    assert_eq!(
        redefinition.as_specialization().unwrap().general().unwrap(),
        F1
    );
}

#[test]
fn checked_upcasts_downcasts_unknowns_and_cross_snapshot_identity() {
    let before = fixture();
    let feature = Feature::try_new(F1, before.model()).unwrap();
    let base = feature.as_type().unwrap();
    assert_eq!(base.as_namespace().unwrap().as_element().unwrap().id(), F1);
    assert_eq!(base.cast::<Feature>().unwrap().id(), F1);
    assert!(matches!(
        Feature::try_new(BASE, before.model()),
        Err(ViewError::WrongClass { .. })
    ));
    assert!(matches!(
        base.cast::<Membership>(),
        Err(ViewError::WrongClass { .. })
    ));
    assert!(matches!(
        Feature::try_new(ElementId::new(), before.model()),
        Err(ViewError::UnknownElement(_))
    ));
    let mut change = before.change_set();
    change.set(F1, p::ELEMENT_DECLARED_NAME, text("velocity"), authored());
    change.set(
        TYPING,
        p::FEATURE_TYPING_TYPE,
        reference(SPECIFIC),
        authored(),
    );
    let after = before.apply(&change).unwrap();
    let updated = Feature::try_new(F1, after.model()).unwrap();
    assert_eq!(updated.id(), feature.id());
    assert_eq!(updated.declared_name().unwrap(), Some("velocity"));
    assert_eq!(feature.declared_name().unwrap(), Some("speed"));
    assert_eq!(
        FeatureTyping::try_new(TYPING, before.model())
            .unwrap()
            .r#type()
            .unwrap(),
        BASE
    );
    assert_eq!(
        FeatureTyping::try_new(TYPING, after.model())
            .unwrap()
            .r#type()
            .unwrap(),
        SPECIFIC
    );
    assert_eq!(before.model().outgoing(TYPING).count(), 2);
    assert_eq!(after.model().outgoing(TYPING).count(), 2);
}

#[test]
fn optional_ordered_unordered_nonunique_and_uncomputed_values_stay_distinct() {
    let snapshot = fixture();
    let feature = Feature::try_new(F1, snapshot.model()).unwrap();
    assert_eq!(feature.declared_short_name().unwrap(), None);
    assert_eq!(feature.direction().unwrap(), None);
    assert!(!feature.is_abstract().unwrap());
    let aliases = feature.alias_ids().unwrap().unwrap();
    assert_eq!(aliases.shape(), SlotShape::Ordered);
    assert!(aliases.descriptor().ordered && aliases.descriptor().unique);
    assert_eq!(aliases.iter().collect::<Vec<_>>(), vec!["z", "a"]);
    assert!(
        Feature::try_new(F2, snapshot.model())
            .unwrap()
            .alias_ids()
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        feature.r#type(),
        Err(ViewError::NotComputed {
            property: p::FEATURE_TYPE,
            ..
        })
    ));
    assert!(feature.owned_relationship().unwrap().is_none());
    let mut overlay = DerivationBuilder::new(snapshot.clone());
    overlay.property(F1, p::FEATURE_TYPE, SlotValue::Ordered(vec![]), evidence());
    overlay.property(
        F2,
        p::FEATURE_OWNED_SUBSETTING,
        SlotValue::Set(BTreeSet::from([
            Value::Reference(REDEFINES),
            Value::Reference(SUBSETS),
        ])),
        evidence(),
    );
    // relatedElement is ordered and nonunique: duplicate occurrences are retained.
    overlay.property(
        TYPING,
        p::RELATIONSHIP_RELATED_ELEMENT,
        SlotValue::Ordered(vec![
            Value::Reference(F1),
            Value::Reference(BASE),
            Value::Reference(F1),
        ]),
        evidence(),
    );
    overlay.property(
        OWNS,
        p::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE,
        reference(F2),
        evidence(),
    );
    let overlay = overlay.build().unwrap();
    assert!(
        Feature::try_new(F1, overlay.model())
            .unwrap()
            .r#type()
            .unwrap()
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        feature.r#type(),
        Err(ViewError::NotComputed { .. })
    ));
    let unordered = Feature::try_new(F2, overlay.model())
        .unwrap()
        .owned_subsetting()
        .unwrap()
        .unwrap();
    assert_eq!(unordered.shape(), SlotShape::Set);
    assert!(!unordered.descriptor().ordered);
    assert_eq!(
        unordered.iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([REDEFINES, SUBSETS])
    );
    let related = Relationship::try_new(TYPING, overlay.model())
        .unwrap()
        .related_element()
        .unwrap()
        .unwrap();
    assert!(!related.descriptor().unique);
    assert_eq!(related.iter().collect::<Vec<_>>(), vec![F1, BASE, F1]);
    let membership = FeatureMembership::try_new(OWNS, overlay.model()).unwrap();
    assert_eq!(membership.owned_member_feature().unwrap(), F2);
    assert_eq!(
        membership
            .as_membership()
            .unwrap()
            .member_element()
            .unwrap(),
        F2
    );
    assert_eq!(
        membership
            .as_relationship()
            .unwrap()
            .target()
            .unwrap()
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![F2]
    );
}

#[test]
fn malformed_writes_and_replaced_slots_cannot_be_hidden_by_typed_access() {
    let before = fixture();
    for (property, value) in [
        (
            p::ELEMENT_ALIAS_IDS,
            SlotValue::Scalar(Value::String("bad".into())),
        ),
        (
            p::ELEMENT_ALIAS_IDS,
            SlotValue::Ordered(vec![Value::Boolean(false)]),
        ),
        (
            p::ELEMENT_ALIAS_IDS,
            SlotValue::Ordered(vec![Value::String("dup".into()); 2]),
        ),
    ] {
        let mut change = before.change_set();
        change.set(F1, property, value, authored());
        assert!(before.apply(&change).is_err());
    }
    let mut change = before.change_set();
    change.clear(TYPING, p::FEATURE_TYPING_TYPED_FEATURE);
    assert!(matches!(
        before.apply(&change),
        Err(ModelError::Multiplicity { .. })
    ));
    let mut change = before.change_set();
    change.set(
        TYPING,
        p::SPECIALIZATION_SPECIFIC,
        reference(F1),
        authored(),
    );
    assert!(matches!(
        before.apply(&change),
        Err(ModelError::IllegalProperty { .. })
    ));
    let mut change = before.change_set();
    change.set(
        TYPING,
        p::FEATURE_TYPING_TYPED_FEATURE,
        reference(BASE),
        authored(),
    );
    assert!(matches!(
        before.apply(&change),
        Err(ModelError::ReferenceType { .. })
    ));
    let mut overlay = DerivationBuilder::new(before.clone());
    overlay.property(
        F1,
        p::FEATURE_TYPE,
        SlotValue::Set(BTreeSet::new()),
        evidence(),
    );
    assert!(matches!(
        overlay.build(),
        Err(DerivationError::Model(ModelError::Shape { .. }))
    ));
    assert_eq!(
        Feature::try_new(F1, before.model())
            .unwrap()
            .alias_ids()
            .unwrap()
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec!["z", "a"]
    );
}

#[test]
fn property_reads_use_identity_even_when_display_names_change() {
    let mut descriptors = agq_kerml::descriptors();
    for c in &mut descriptors.classes {
        c.name = format!("Renamed{}", c.name);
    }
    for p in &mut descriptors.properties {
        p.name = format!("renamed{}", p.name);
    }
    let registry = Arc::new(MetamodelRegistry::from_descriptors(descriptors).unwrap());
    let before = Snapshot::new(registry);
    let mut change = before.change_set();
    change.create(BASE, c::NAMESPACE, authored());
    change.set(
        BASE,
        p::ELEMENT_ELEMENT_ID,
        text("separate-normative-string"),
        authored(),
    );
    change.set(
        BASE,
        p::ELEMENT_IS_IMPLIED_INCLUDED,
        SlotValue::Scalar(Value::Boolean(false)),
        authored(),
    );
    change.set(BASE, p::ELEMENT_DECLARED_NAME, text("works"), authored());
    let after = before.apply(&change).unwrap();
    let view = Namespace::try_new(BASE, after.model()).unwrap();
    assert_eq!(view.declared_name().unwrap(), Some("works"));
    assert_eq!(view.element_id().unwrap(), "separate-normative-string");
    assert_eq!(view.id(), BASE);
}

#[test]
fn casts_use_the_live_registry_and_incompatible_domains_are_explicit() {
    let mut descriptors = agq_kerml::descriptors();
    let extension = MetaclassId::new();
    descriptors.classes.push(MetaclassDescriptor {
        id: extension,
        name: "Extension".into(),
        package: vec!["Test".into()],
        metamodel: agq_kerml::metamodel::KERML,
        direct_supertypes: BTreeSet::from([c::NAMESPACE]),
        is_abstract: false,
    });
    // A generic registry can be structurally valid but incompatible with a typed domain.
    descriptors
        .properties
        .iter_mut()
        .find(|p| p.id == p::ELEMENT_DECLARED_NAME)
        .unwrap()
        .value_kind = ValueKind::Integer;
    let before = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    let mut change = before.change_set();
    change.create(BASE, extension, authored());
    change.set(BASE, p::ELEMENT_ELEMENT_ID, text("extension"), authored());
    change.set(
        BASE,
        p::ELEMENT_IS_IMPLIED_INCLUDED,
        SlotValue::Scalar(Value::Boolean(false)),
        authored(),
    );
    change.set(
        BASE,
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::Integer(42.into())),
        authored(),
    );
    let after = before.apply(&change).unwrap();
    let view = Namespace::try_new(BASE, after.model()).unwrap();
    assert_eq!(view.record().metaclass(), extension);
    assert_eq!(view.as_element().unwrap().id(), BASE);
    assert!(matches!(
        view.declared_name(),
        Err(ViewError::IncompatibleProperty { .. })
    ));
}
