mod common;
use agq_kernel::metamodel::*;
use agq_kernel::*;
use common::*;

#[test]
fn multiple_inheritance_diamond_preserves_both_branches_and_shared_properties_once() {
    let classes = [
        class(ELEMENT, "Root", &[]),
        class(TYPE, "Left", &[ELEMENT]),
        class(FEATURE, "Right", &[ELEMENT]),
        class(PART_DEF, "Diamond", &[TYPE, FEATURE]),
    ];
    let props = [
        property(
            NAME,
            "root",
            ELEMENT,
            ValueKind::String,
            Multiplicity::OPTIONAL,
        ),
        property(TAGS, "left", TYPE, ValueKind::String, Multiplicity::MANY),
        property(
            COUNT,
            "right",
            FEATURE,
            ValueKind::Integer,
            Multiplicity::ONE,
        ),
    ];
    let registry = MetamodelRegistry::new([model_descriptor()], classes, props).unwrap();
    for base in [ELEMENT, TYPE, FEATURE, PART_DEF] {
        assert!(registry.is_subtype(PART_DEF, base).unwrap());
    }
    assert!(!registry.is_subtype(TYPE, FEATURE).unwrap());
    assert_eq!(
        registry
            .effective_properties(PART_DEF)
            .unwrap()
            .map(|p| p.id)
            .collect::<Vec<_>>(),
        vec![NAME, COUNT, TAGS]
    );
    assert_eq!(registry.declared_properties(PART_DEF).unwrap().count(), 0);
    assert!(registry.is_legal(PART_DEF, COUNT).unwrap());
}

#[test]
fn cycles_and_unknown_parents_are_rejected() {
    for classes in [
        vec![class(TYPE, "Self", &[TYPE])],
        vec![class(TYPE, "A", &[FEATURE]), class(FEATURE, "B", &[TYPE])],
    ] {
        assert!(matches!(
            MetamodelRegistry::new([model_descriptor()], classes, []),
            Err(MetamodelError::InheritanceCycle(_))
        ));
    }
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], [class(TYPE, "A", &[FEATURE])], []),
        Err(MetamodelError::UnknownClass(FEATURE))
    ));
}

#[test]
fn incompatible_definitions_and_ambiguous_names_are_rejected() {
    let a = class(TYPE, "A", &[]);
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], [a.clone(), a], []),
        Err(MetamodelError::DuplicateClass(TYPE))
    ));
    let p = property(NAME, "same", TYPE, ValueKind::String, Multiplicity::ONE);
    let q = property(
        TAGS,
        "same",
        FEATURE,
        ValueKind::Boolean,
        Multiplicity::OPTIONAL,
    );
    let classes = [
        class(TYPE, "Left", &[]),
        class(FEATURE, "Right", &[]),
        class(PART_DEF, "Both", &[TYPE, FEATURE]),
    ];
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes, [p, q]),
        Err(MetamodelError::PropertyConflict {
            class: PART_DEF,
            ..
        })
    ));
}

#[test]
fn invalid_properties_and_duplicate_descriptors_are_rejected() {
    let (classes, properties) = descriptors();
    let mut invalid = properties.clone();
    invalid[0].multiplicity = Multiplicity {
        lower: 2,
        upper: Some(1),
    };
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes.clone(), invalid),
        Err(MetamodelError::InvalidMultiplicity(NAME))
    ));
    let mut invalid = properties.clone();
    invalid[0].composite = true;
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes.clone(), invalid),
        Err(MetamodelError::PrimitiveContainment(NAME))
    ));
    let mut invalid = properties.clone();
    invalid.push(properties[0].clone());
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes.clone(), invalid),
        Err(MetamodelError::DuplicateProperty(NAME))
    ));
    let mut invalid = properties;
    invalid[0].value_kind = ValueKind::Reference(MetaclassId::from_u128(999));
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes, invalid),
        Err(MetamodelError::UnknownClass(_))
    ));
}

#[test]
fn registry_versions_and_unknown_queries_are_explicit() {
    let mut second = model_descriptor();
    second.id = MetamodelId::from_u128(2);
    second.version.minor = 1;
    second.uri = "urn:agentique:test:kernel:1.1".into();
    let mut other = class(FEATURE, "Type", &[TYPE]);
    other.metamodel = second.id;
    let registry = MetamodelRegistry::new(
        [model_descriptor(), second.clone()],
        [class(TYPE, "Type", &[]), other],
        [property(
            NAME,
            "name",
            FEATURE,
            ValueKind::String,
            Multiplicity::OPTIONAL,
        )],
    )
    .unwrap();
    assert_eq!(registry.property_metamodel(NAME).unwrap(), &second);
    assert!(registry.is_subtype(FEATURE, TYPE).unwrap());
    let unknown = MetaclassId::from_u128(999);
    assert_eq!(
        registry.is_subtype(unknown, unknown),
        Err(MetamodelError::UnknownClass(unknown))
    );
    assert!(matches!(
        registry.effective_properties(unknown),
        Err(MetamodelError::UnknownClass(_))
    ));
    assert_eq!(
        registry.is_legal(TYPE, TAGS),
        Err(MetamodelError::UnknownProperty(TAGS))
    );
}

#[test]
fn registration_order_does_not_change_effective_properties() {
    let (mut classes, mut properties) = descriptors();
    let forward =
        MetamodelRegistry::new([model_descriptor()], classes.clone(), properties.clone()).unwrap();
    classes.reverse();
    properties.reverse();
    let reverse = MetamodelRegistry::new([model_descriptor()], classes, properties).unwrap();
    assert_eq!(
        forward
            .effective_properties(TYPING)
            .unwrap()
            .collect::<Vec<_>>(),
        reverse
            .effective_properties(TYPING)
            .unwrap()
            .collect::<Vec<_>>()
    );
}

#[test]
fn same_name_redefinition_and_diamond_join_require_explicit_identity_links() {
    let classes = vec![
        class(ELEMENT, "Base", &[]),
        class(TYPE, "Left", &[ELEMENT]),
        class(FEATURE, "Right", &[ELEMENT]),
        class(PART_DEF, "Join", &[TYPE, FEATURE]),
    ];
    let base = property(
        NAME,
        "value",
        ELEMENT,
        ValueKind::String,
        Multiplicity::OPTIONAL,
    );
    let mut left = property(
        TAGS,
        "value",
        TYPE,
        ValueKind::String,
        Multiplicity::OPTIONAL,
    );
    left.redefines.insert(NAME);
    let r = MetamodelRegistry::new(
        [model_descriptor()],
        classes.clone(),
        [base.clone(), left.clone()],
    )
    .unwrap();
    assert_eq!(
        r.property_named(PART_DEF, "value").unwrap().unwrap().id,
        TAGS
    );
    assert_eq!(r.effective_properties(PART_DEF).unwrap().count(), 1);
    let mut right = property(
        COUNT,
        "renamed",
        FEATURE,
        ValueKind::String,
        Multiplicity::OPTIONAL,
    );
    right.redefines.insert(NAME);
    assert!(matches!(
        MetamodelRegistry::new(
            [model_descriptor()],
            classes.clone(),
            [base.clone(), left.clone(), right.clone()]
        ),
        Err(MetamodelError::PropertyConflict { .. })
    ));
    let mut join = property(
        BAG,
        "joined",
        PART_DEF,
        ValueKind::String,
        Multiplicity::OPTIONAL,
    );
    join.redefines.extend([TAGS, COUNT]);
    let r =
        MetamodelRegistry::new([model_descriptor()], classes, [base, left, right, join]).unwrap();
    assert_eq!(r.resolve_property(PART_DEF, NAME).unwrap().unwrap().id, BAG);
    assert_eq!(r.effective_properties(PART_DEF).unwrap().count(), 1);
}

#[test]
fn invalid_redefinition_scope_domain_bounds_and_cycles_fail() {
    let classes = vec![class(ELEMENT, "Base", &[]), class(TYPE, "Sub", &[ELEMENT])];
    let base = property(NAME, "base", ELEMENT, ValueKind::String, Multiplicity::ONE);
    for (owner, kind, bounds) in [
        (ELEMENT, ValueKind::String, Multiplicity::ONE),
        (TYPE, ValueKind::Boolean, Multiplicity::ONE),
        (TYPE, ValueKind::String, Multiplicity::OPTIONAL),
    ] {
        let mut redef = property(TAGS, "replacement", owner, kind, bounds);
        redef.redefines.insert(NAME);
        assert!(matches!(
            MetamodelRegistry::new([model_descriptor()], classes.clone(), [base.clone(), redef]),
            Err(MetamodelError::InvalidRedefinition { .. })
        ));
    }
    let mut cycle = base;
    cycle.redefines.insert(NAME);
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes, [cycle]),
        Err(MetamodelError::PropertyCycle(_))
    ));
}

#[test]
fn package_names_do_not_replace_descriptor_identity() {
    let a = class(TYPE, "Same", &[]);
    let mut b = class(FEATURE, "Same", &[]);
    b.package = vec!["another".into()];
    let r = MetamodelRegistry::new([model_descriptor()], [a, b], []).unwrap();
    assert!(!r.is_subtype(TYPE, FEATURE).unwrap());
}

#[test]
fn locally_valid_subset_graphs_are_representable_not_uml_certified() {
    use std::collections::BTreeSet;
    for edges in [
        vec![(NAME, TAGS)],
        vec![(NAME, TAGS), (TAGS, NAME)],
        vec![(NAME, NAME)],
    ] {
        let mut a = property(NAME, "a", TYPE, ValueKind::String, Multiplicity::MANY);
        let mut b = property(TAGS, "b", TYPE, ValueKind::String, Multiplicity::MANY);
        for (source, target) in edges {
            if source == NAME {
                a.subsets.insert(target);
            } else {
                b.subsets.insert(target);
            }
        }
        let r = MetamodelRegistry::new(
            [model_descriptor()],
            [class(TYPE, "Context", &[])],
            [a.clone(), b.clone()],
        )
        .unwrap();
        assert_eq!(r.property(NAME).unwrap().subsets, a.subsets);
        assert_eq!(r.property(TAGS).unwrap().subsets, b.subsets);
        assert!(r.subset_closure(NAME).unwrap().len() <= 2);
        assert!(r.subset_contributors(NAME).unwrap().len() <= 2);
        // Reflexive metadata violates UML subsetted_property_names. It is still
        // coherent set inclusion and is never normalized out of the descriptor.
        if a.subsets == BTreeSet::from([NAME]) {
            assert_eq!(r.subset_closure(NAME).unwrap(), BTreeSet::from([NAME]));
        }
    }
}

#[test]
fn subset_local_context_domain_and_upper_bound_validation_remains_enabled() {
    let classes = vec![
        class(TYPE, "Context", &[]),
        class(FEATURE, "Unrelated", &[]),
    ];
    let base = property(NAME, "base", TYPE, ValueKind::String, Multiplicity::ONE);
    for (owner, kind, bounds) in [
        (FEATURE, ValueKind::String, Multiplicity::OPTIONAL),
        (TYPE, ValueKind::Boolean, Multiplicity::OPTIONAL),
        (TYPE, ValueKind::String, Multiplicity::MANY),
    ] {
        let mut sub = property(TAGS, "sub", owner, kind, bounds);
        sub.subsets.insert(NAME);
        assert!(matches!(
            MetamodelRegistry::new([model_descriptor()], classes.clone(), [base.clone(), sub]),
            Err(MetamodelError::InvalidPropertyMetadata(TAGS))
        ));
    }
    // Unlike redefinition, subsetting does not require the lower bound to narrow.
    let mut sub = property(TAGS, "sub", TYPE, ValueKind::String, Multiplicity::OPTIONAL);
    sub.subsets.insert(NAME);
    MetamodelRegistry::new(
        [model_descriptor()],
        classes.clone(),
        [base.clone(), sub.clone()],
    )
    .unwrap();
    sub.subsets.insert(BAG);
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes, [base, sub]),
        Err(MetamodelError::UnknownProperty(BAG))
    ));
}

#[test]
fn redefinition_cycles_and_composition_weakening_remain_rejected() {
    let classes = vec![class(ELEMENT, "Base", &[]), class(TYPE, "Sub", &[ELEMENT])];
    let mut a = property(
        NAME,
        "base",
        ELEMENT,
        ValueKind::Reference(ELEMENT),
        Multiplicity::MANY,
    );
    a.ordered = true;
    a.unique = true;
    a.composite = true;
    let mut b = property(
        TAGS,
        "replacement",
        TYPE,
        ValueKind::Reference(TYPE),
        Multiplicity::MANY,
    );
    b.ordered = true;
    b.unique = true;
    b.composite = true;
    b.redefines.insert(NAME);
    MetamodelRegistry::new(
        [model_descriptor()],
        classes.clone(),
        [a.clone(), b.clone()],
    )
    .unwrap();
    let mut invalid = b.clone();
    invalid.composite = false;
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes.clone(), [a.clone(), invalid]),
        Err(MetamodelError::InvalidRedefinition { .. })
    ));
    a.redefines.insert(TAGS);
    assert!(matches!(
        MetamodelRegistry::new([model_descriptor()], classes, [a, b]),
        Err(MetamodelError::PropertyCycle(_))
    ));
}

#[test]
fn redefining_collection_flags_are_preserved_and_effective_shape_is_validated() {
    use agq_kernel::{
        provenance::DeclaredOrigin,
        value::{SlotValue, Value},
    };
    use std::sync::Arc;
    let mut a = property(NAME, "base", ELEMENT, ValueKind::String, Multiplicity::MANY);
    a.ordered = true;
    a.unique = true;
    let mut b = property(
        TAGS,
        "replacement",
        TYPE,
        ValueKind::String,
        Multiplicity::MANY,
    );
    b.ordered = false;
    b.unique = false;
    b.redefines.insert(NAME);
    let registry = MetamodelRegistry::new(
        [model_descriptor()],
        [class(ELEMENT, "Base", &[]), class(TYPE, "Sub", &[ELEMENT])],
        [a, b.clone()],
    )
    .unwrap();
    assert_eq!(registry.resolve_property(TYPE, NAME).unwrap(), Some(&b));
    let base = Snapshot::new(Arc::new(registry));
    let origin = DeclaredOrigin::Authored { source: None };
    let values = vec![Value::String("same".into()), Value::String("same".into())];
    let mut changes = base.change_set();
    changes.create(ENGINE, TYPE, origin.clone());
    changes.set(ENGINE, TAGS, SlotValue::Bag(values.clone()), origin.clone());
    let model = base.apply(&changes).unwrap();
    assert_eq!(
        model
            .model()
            .element(ENGINE)
            .unwrap()
            .slot(TAGS)
            .unwrap()
            .value(),
        &SlotValue::Bag(values.clone())
    );
    let mut invalid = model.change_set();
    invalid.set(ENGINE, TAGS, SlotValue::Ordered(values), origin);
    assert!(model.apply(&invalid).is_err());
    assert!(base.model().element(ENGINE).is_none());
}

#[test]
fn future_union_metadata_frontier_is_finite_and_deduplicates_diamonds_and_cycles() {
    use std::collections::BTreeSet;
    let properties = [
        (NAME, vec![NAME]),
        (TAGS, vec![NAME, COUNT]),
        (COUNT, vec![NAME, TAGS]),
        (BAG, vec![TAGS, COUNT]),
    ]
    .map(|(id, targets)| {
        let mut p = property(
            id,
            &id.to_string(),
            TYPE,
            ValueKind::String,
            Multiplicity::MANY,
        );
        p.subsets.extend(targets);
        p.derived = id == NAME;
        p.derived_union = id == NAME;
        p
    });
    let r = MetamodelRegistry::new(
        [model_descriptor()],
        [class(TYPE, "Context", &[])],
        properties,
    )
    .unwrap();
    assert_eq!(
        r.subset_contributors(NAME).unwrap(),
        BTreeSet::from([NAME, TAGS, COUNT, BAG])
    );
    assert_eq!(
        r.subset_closure(BAG).unwrap(),
        BTreeSet::from([NAME, TAGS, COUNT])
    );
    assert_eq!(r.subset_contributors(BAG).unwrap(), BTreeSet::new());
    assert_eq!(
        r.subset_contributors(PropertyId::from_u128(999)),
        Err(MetamodelError::UnknownProperty(PropertyId::from_u128(999)))
    );
}
