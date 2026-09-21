mod common;
use agq_kernel::{derived::*, metamodel::*, numeric::*, provenance::*, value::*, *};
use common::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

fn hash<T: Hash>(v: &T) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

#[test]
fn numeric_values_are_exact_canonical_and_totally_ordered() {
    let huge = "987654321098765432109876543210987654321098765432109876543210";
    for value in [huge.to_owned(), format!("-{huge}")] {
        let n: Integer = value.parse().unwrap();
        assert_eq!(n.to_string(), value);
    }
    for zero in ["0", "-0", "+000"] {
        assert_eq!(zero.parse::<Integer>().unwrap(), 0.into());
    }
    for (a, b) in [
        ("0.1", "1e-1"),
        ("-00.01000", "-1E-2"),
        ("-0e999999999999999999999999", "0"),
        ("12300", "123e2"),
    ] {
        let a: ExactDecimal = a.parse().unwrap();
        let b: ExactDecimal = b.parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
        assert_eq!(hash(&a), hash(&b));
        assert_eq!(a, a.to_string().parse().unwrap());
    }
    assert_ne!(
        "9007199254740993.1".parse::<ExactDecimal>().unwrap(),
        "9007199254740993.2".parse().unwrap()
    );
    let ordered = [
        "-1e99999999999999999999",
        "-100",
        "-0.1",
        "0",
        "0.00001",
        "0.1",
        "1",
        "1.0000000000000001",
        "1e99999999999999999999",
    ];
    let values: Vec<ExactDecimal> = ordered.iter().map(|s| s.parse().unwrap()).collect();
    for window in values.windows(2) {
        assert!(window[0] < window[1]);
    }
    for invalid in [
        "", " ", " 1", "1 ", "1_0", "1.2.3", "NaN", "Infinity", "1e", "1e+", "--1", ".", "e1",
    ] {
        assert!(invalid.parse::<ExactDecimal>().is_err(), "{invalid}");
    }
    for invalid in ["", "-", "+", "1.0", "1e2", " 1", "1_2"] {
        assert!(invalid.parse::<Integer>().is_err());
    }
    // Many spellings exercise canonical equality/hash independently of serialization.
    for n in -250..250_i64 {
        let a: ExactDecimal = format!("{n}00e-2").parse().unwrap();
        let b: ExactDecimal = n.to_string().parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));
    }
}

fn aid(n: u128) -> AssociationId {
    AssociationId::from_u128(n)
}
fn end(n: u128, opposite: bool) -> PropertyId {
    PropertyId::from_u128(100 + 2 * n + u128::from(opposite))
}
fn associations(parents: &[&[u128]]) -> DescriptorSet {
    let mut set = DescriptorSet {
        models: vec![model_descriptor()],
        classes: vec![class(ELEMENT, "Element", &[])],
        ..Default::default()
    };
    for (i, parents) in parents.iter().enumerate() {
        let n = i as u128 + 1;
        set.associations.push(AssociationDescriptor {
            id: aid(n),
            name: format!("A{n}"),
            package: vec![],
            metamodel: MM,
            member_ends: vec![end(n, false), end(n, true)],
            navigable_owned_ends: BTreeSet::from([end(n, false)]),
            direct_supertypes: parents.iter().copied().map(aid).collect(),
            is_abstract: false,
        });
        for opposite in [false, true] {
            let mut p = property(
                end(n, opposite),
                &format!("end{n}{opposite}"),
                ELEMENT,
                ValueKind::Reference(ELEMENT),
                Multiplicity::MANY,
            );
            p.owner = PropertyOwner::Association(aid(n));
            p.association = Some(aid(n));
            p.opposite_ends = BTreeSet::from([end(n, !opposite)]);
            if !opposite {
                p.redefines = parents.iter().map(|&parent| end(parent, false)).collect();
            }
            set.properties.push(p);
        }
    }
    set
}

#[test]
fn association_ancestry_and_end_redefinition_are_generic_and_deterministic() {
    for parents in [
        vec![&[][..], &[1][..]],
        vec![&[][..], &[1][..], &[2][..]],
        vec![&[][..], &[1][..], &[1][..], &[2, 3][..]],
    ] {
        let set = associations(&parents);
        let last = parents.len() as u128;
        let registry = MetamodelRegistry::from_descriptors(set.clone()).unwrap();
        assert_eq!(
            registry.association_ancestry(aid(last)).unwrap().len(),
            parents.len()
        );
        let effective = registry.effective_association_ends(aid(last)).unwrap();
        assert!(effective.contains(&end(last, false)));
        assert!(!effective.contains(&end(1, false)));
        assert_eq!(effective.len(), parents.len() + 1);
        let mut shuffled = set;
        shuffled.associations.reverse();
        shuffled.properties.reverse();
        assert_eq!(
            effective,
            MetamodelRegistry::from_descriptors(shuffled)
                .unwrap()
                .effective_association_ends(aid(last))
                .unwrap()
        );
    }
    assert!(matches!(
        MetamodelRegistry::from_descriptors(associations(&[&[9]])),
        Err(MetamodelError::UnknownAssociation(_))
    ));
    assert!(matches!(
        MetamodelRegistry::from_descriptors(associations(&[&[2], &[1]])),
        Err(MetamodelError::AssociationInheritanceCycle(_))
    ));
}

#[test]
fn occurrences_have_one_fact_two_navigations_order_provenance_and_atomic_deletion() {
    let mut set = associations(&[&[]]);
    set.associations[0]
        .navigable_owned_ends
        .insert(end(1, true));
    for p in &mut set.properties {
        p.ordered = true;
        p.unique = false;
    }
    let base = Snapshot::new(Arc::new(MetamodelRegistry::from_descriptors(set).unwrap()));
    let a = ElementId::from_u128(1);
    let b = ElementId::from_u128(2);
    let l1 = AssociationOccurrenceId::from_u128(1);
    let l2 = AssociationOccurrenceId::from_u128(2);
    let mut changes = base.change_set();
    changes
        .create(a, ELEMENT, authored())
        .create(b, ELEMENT, authored());
    for (link, pos) in [(l2, 1), (l1, 0)] {
        changes.link(
            link,
            aid(1),
            BTreeMap::from([(end(1, false), b), (end(1, true), a)]),
            BTreeMap::from([(end(1, false), pos), (end(1, true), pos)]),
            authored(),
        );
    }
    let snapshot = base.apply(&changes).unwrap();
    assert!(base.model().is_empty());
    assert_eq!(snapshot.model().association_occurrences().count(), 2);
    assert_eq!(snapshot.model().incident_associations(b).count(), 2);
    let nav = snapshot.model().navigation_slot(a, end(1, false)).unwrap();
    assert_eq!(
        nav.value(),
        &SlotValue::Ordered(vec![Value::Reference(b), Value::Reference(b)])
    );
    assert_eq!(
        nav.origin(),
        &Origin::AssociationOccurrences(BTreeSet::from([l1, l2]))
    );
    assert!(
        snapshot
            .model()
            .element(a)
            .unwrap()
            .slots()
            .next()
            .is_none()
    );
    let mut failed = snapshot.change_set();
    failed.remove(b);
    assert!(snapshot.apply(&failed).is_err());
    assert!(snapshot.model().element(b).is_some());
    let mut reordered = snapshot.change_set();
    for (link, pos) in [(l1, 1), (l2, 0)] {
        reordered.reorder_link(
            link,
            BTreeMap::from([(end(1, false), pos), (end(1, true), pos)]),
            authored(),
        );
    }
    let branch = snapshot.apply(&reordered).unwrap();
    assert_ne!(
        snapshot
            .model()
            .association_occurrence(l1)
            .unwrap()
            .positions(),
        branch
            .model()
            .association_occurrence(l1)
            .unwrap()
            .positions()
    );
    let mut deleted = snapshot.change_set();
    deleted.unlink(l1).unlink(l2).remove(b);
    let after = snapshot.apply(&deleted).unwrap();
    assert_eq!(after.model().association_occurrences().count(), 0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let s = &snapshot;
            scope.spawn(move || {
                for _ in 0..100 {
                    assert_eq!(s.model().incident_associations(a).count(), 2);
                }
            });
        }
    });
}

#[test]
fn derived_states_and_cyclic_union_frontiers_remain_distinct() {
    let mut p = property(
        NAME,
        "union",
        ELEMENT,
        ValueKind::String,
        Multiplicity::MANY,
    );
    p.derived = true;
    p.derived_union = true;
    p.subsets.insert(TAGS);
    let mut q = property(
        TAGS,
        "subset",
        ELEMENT,
        ValueKind::String,
        Multiplicity::MANY,
    );
    q.derived = true;
    q.subsets.extend([NAME, TAGS]);
    let registry = Arc::new(
        MetamodelRegistry::new(
            [model_descriptor()],
            [class(ELEMENT, "Element", &[])],
            [p, q],
        )
        .unwrap(),
    );
    let frontier = registry.derived_union_frontier(NAME, ELEMENT).unwrap();
    assert_eq!(frontier.contributors, BTreeSet::from([NAME, TAGS]));
    assert_eq!(frontier.inspected, frontier.contributors);
    let base = Snapshot::new(registry);
    let id = ElementId::from_u128(10);
    let mut changes = base.change_set();
    changes.create(id, ELEMENT, authored());
    let snapshot = base.apply(&changes).unwrap();
    assert!(matches!(
        snapshot.model().property_state(id, NAME).unwrap(),
        PropertyState::NotComputed
    ));
    let evidence = Explanation {
        rule: RuleId::from_u128(1),
        dependencies: BTreeSet::new(),
    };
    let mut computed = DerivationBuilder::new(snapshot.clone());
    computed.property(id, NAME, SlotValue::Set(BTreeSet::new()), evidence.clone());
    let computed = computed.build().unwrap();
    assert!(matches!(
        computed.model().property_state(id, NAME).unwrap(),
        PropertyState::Computed(_)
    ));
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder
        .failure(
            id,
            NAME,
            ComputationFailure::Incomplete {
                reason: IncompleteReason::MissingInput,
                explanation: evidence.clone(),
                searches: BTreeSet::from([StructuralSearch::DescriptorGraph]),
            },
        )
        .unwrap();
    builder
        .failure(
            id,
            TAGS,
            ComputationFailure::Invalid {
                diagnostic: "invalid input".into(),
                explanation: evidence,
                searches: BTreeSet::new(),
            },
        )
        .unwrap();
    let failed = builder.build().unwrap();
    assert!(matches!(
        failed.model().property_state(id, NAME).unwrap(),
        PropertyState::Incomplete(_)
    ));
    assert!(matches!(
        failed.model().property_state(id, TAGS).unwrap(),
        PropertyState::Invalid(_)
    ));
    assert!(matches!(
        snapshot.model().property_state(id, NAME).unwrap(),
        PropertyState::NotComputed
    ));
}

#[test]
fn ownership_combinations_keep_raw_edges_separate_from_class_replacement() {
    for (base_class, child_class) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut set = associations(&[&[], &[1]]);
        set.classes.push(class(TYPE, "Child", &[ELEMENT]));
        for p in &mut set.properties {
            if p.id == end(1, false) && base_class {
                p.owner = PropertyOwner::Class(ELEMENT);
            }
            if p.id == end(2, false) && child_class {
                p.owner = PropertyOwner::Class(TYPE);
            }
            if p.id == end(2, true) {
                p.value_kind = ValueKind::Reference(TYPE);
            }
        }
        if base_class {
            set.associations[0].navigable_owned_ends.clear();
        }
        if child_class {
            set.associations[1].navigable_owned_ends.clear();
        }
        let registry = MetamodelRegistry::from_descriptors(set).unwrap();
        assert_eq!(
            registry.declared_redefinitions(end(2, false)).unwrap(),
            &BTreeSet::from([end(1, false)])
        );
        assert_eq!(registry.is_legal(TYPE, end(2, false)).unwrap(), child_class);
        assert_eq!(
            registry.is_legal(TYPE, end(1, false)).unwrap(),
            base_class && !child_class
        );
        assert_eq!(
            registry
                .effective_redefinitions_for_class(end(2, false), TYPE)
                .unwrap()
                .contains(&end(1, false)),
            base_class && child_class
        );
    }
}

#[test]
fn descriptor_traversals_terminate_on_deep_graphs_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let count = 10_000_u128;
            let properties = (1..=count)
                .map(|n| {
                    let id = PropertyId::from_u128(n);
                    let mut p = property(
                        id,
                        &format!("p{n}"),
                        ELEMENT,
                        ValueKind::String,
                        Multiplicity::MANY,
                    );
                    p.derived = true;
                    p.subsets
                        .insert(PropertyId::from_u128(if n == count { 1 } else { n + 1 }));
                    p
                })
                .collect::<Vec<_>>();
            let registry = MetamodelRegistry::new(
                [model_descriptor()],
                [class(ELEMENT, "Element", &[])],
                properties,
            )
            .unwrap();
            assert_eq!(registry.subset_closure(NAME).unwrap().len(), count as usize);
            assert_eq!(
                registry
                    .derived_union_frontier(NAME, ELEMENT)
                    .unwrap()
                    .contributors
                    .len(),
                count as usize
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn primitive_descriptor_identity_and_numeric_carrier_are_both_checked() {
    let domain = PrimitiveDomainId::from_u128(1);
    let p = property(
        NAME,
        "integer",
        ELEMENT,
        ValueKind::Primitive(domain),
        Multiplicity::ONE,
    );
    let set = DescriptorSet {
        models: vec![model_descriptor()],
        classes: vec![class(ELEMENT, "Element", &[])],
        properties: vec![p],
        primitives: vec![PrimitiveDescriptor {
            id: domain,
            name: "integer".into(),
            representation: PrimitiveRepresentation::Integer,
        }],
        ..Default::default()
    };
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(set.clone()).unwrap(),
    ));
    for value in [Value::String("1".into()), Value::Real("1".parse().unwrap())] {
        let mut edit = base.change_set();
        edit.create(ENGINE, ELEMENT, authored()).set(
            ENGINE,
            NAME,
            SlotValue::Scalar(value),
            authored(),
        );
        assert!(matches!(
            base.apply(&edit),
            Err(ModelError::ValueKind { .. })
        ));
    }
    let mut edit = base.change_set();
    edit.create(ENGINE, ELEMENT, authored()).set(
        ENGINE,
        NAME,
        SlotValue::Scalar(Value::Integer(
            "999999999999999999999999999999999999".parse().unwrap(),
        )),
        authored(),
    );
    assert!(base.apply(&edit).is_ok());
    let mut bad = set;
    bad.primitives.clear();
    assert!(matches!(
        MetamodelRegistry::from_descriptors(bad),
        Err(MetamodelError::UnknownPrimitive(_))
    ));
}

#[test]
fn derived_association_results_have_states_and_evidence_without_class_slots() {
    let mut set = associations(&[&[]]);
    set.properties[0].derived = true;
    let base = Snapshot::new(Arc::new(MetamodelRegistry::from_descriptors(set).unwrap()));
    let mut edit = base.change_set();
    edit.create(ENGINE, ELEMENT, authored())
        .create(VEHICLE, ELEMENT, authored());
    let snapshot = base.apply(&edit).unwrap();
    let property = end(1, false);
    assert!(matches!(
        snapshot.model().property_state(ENGINE, property).unwrap(),
        PropertyState::NotComputed
    ));
    let evidence = Explanation {
        rule: RuleId::from_u128(1),
        dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(VEHICLE))]),
    };
    let fact = FactKey::Property {
        element: ENGINE,
        property,
    };
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.property(
        ENGINE,
        property,
        SlotValue::Set(BTreeSet::from([Value::Reference(VEHICLE)])),
        evidence.clone(),
    );
    builder.searches(fact, BTreeSet::from([StructuralSearch::DescriptorGraph]));
    let overlay = builder.build().unwrap();
    assert!(
        overlay
            .model()
            .element(ENGINE)
            .unwrap()
            .slot(property)
            .is_none()
    );
    assert!(matches!(
        overlay.model().property_state(ENGINE, property).unwrap(),
        PropertyState::Computed(_)
    ));
    assert_eq!(
        overlay.model().incoming(VEHICLE).next().unwrap().carrier,
        ReferenceCarrier::DerivedNavigation
    );
    assert_eq!(overlay.model().computation_searches().count(), 1);
    assert_incoming_property_index(snapshot.model());
    assert_incoming_property_index(overlay.model());
    let dependency = Snapshot::with_immutable_dependency(Arc::new(overlay.clone()));
    assert_incoming_property_index(dependency.model());
    for key in [fact, FactKey::Element(ENGINE), FactKey::Element(VEHICLE)] {
        let scanned: Vec<_> = overlay
            .model()
            .computation_searches()
            .filter(|(candidate, _)| **candidate == key)
            .flat_map(|(_, searches)| searches.iter())
            .collect();
        let indexed: Vec<_> = overlay.model().computation_searches_for(key).collect();
        assert_eq!(indexed, scanned);
    }
    assert!(
        overlay
            .explain(fact)
            .unwrap()
            .dependencies
            .contains(&Dependency::Declared(FactKey::Element(ENGINE)))
    );
    let mut invalid = DerivationBuilder::new(snapshot.clone());
    invalid.property(
        ENGINE,
        property,
        SlotValue::Set(BTreeSet::from([Value::Reference(ElementId::from_u128(
            999,
        ))])),
        evidence.clone(),
    );
    assert!(matches!(
        invalid.build(),
        Err(DerivationError::Model(ModelError::DanglingReference { .. }))
    ));
    let mut failed = DerivationBuilder::new(snapshot);
    failed
        .failure(
            ENGINE,
            property,
            ComputationFailure::Incomplete {
                reason: IncompleteReason::UnsupportedRuntimeSemantics,
                explanation: evidence,
                searches: BTreeSet::new(),
            },
        )
        .unwrap();
    assert!(matches!(
        failed
            .build()
            .unwrap()
            .model()
            .property_state(ENGINE, property)
            .unwrap(),
        PropertyState::Incomplete(_)
    ));
}

#[test]
fn occurrence_inverse_bounds_duplicates_and_bad_positions_roll_back() {
    for case in 0..3 {
        let mut set = associations(&[&[]]);
        if case == 0 {
            set.properties[1].multiplicity = Multiplicity::ONE;
        }
        if case == 2 {
            set.properties[0].ordered = true;
        }
        let base = Snapshot::new(Arc::new(MetamodelRegistry::from_descriptors(set).unwrap()));
        let mut edits = base.change_set();
        for element in [ENGINE, VEHICLE, SPORTS] {
            edits.create(element, ELEMENT, authored());
        }
        let snapshot = base.apply(&edits).unwrap();
        let mut bad = snapshot.change_set();
        for n in 0..2 {
            let (source, target) = match case {
                0 => (if n == 0 { ENGINE } else { SPORTS }, VEHICLE),
                1 => (ENGINE, VEHICLE),
                _ => (ENGINE, if n == 0 { VEHICLE } else { SPORTS }),
            };
            bad.link(
                AssociationOccurrenceId::from_u128(n + 1),
                aid(1),
                BTreeMap::from([(end(1, false), target), (end(1, true), source)]),
                if case == 2 {
                    BTreeMap::from([(end(1, false), 0)])
                } else {
                    BTreeMap::new()
                },
                authored(),
            );
        }
        assert!(snapshot.apply(&bad).is_err());
        assert_eq!(snapshot.model().association_occurrences().count(), 0);
        assert_eq!(snapshot.model().len(), 3);
    }
}
