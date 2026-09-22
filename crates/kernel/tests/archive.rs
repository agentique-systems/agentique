mod common;
use agq_kernel::{archive::*, derived::*, metamodel::*, provenance::*, value::*, *};
use common::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
    sync::Arc,
};

fn key(output: u128) -> DerivationKey {
    DerivationKey {
        rule: RuleId::from_u128(80),
        subject: VEHICLE,
        output: OutputKey::from_u128(output),
    }
}
fn proof() -> Explanation {
    Explanation {
        rule: key(0).rule,
        dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(VEHICLE))]),
    }
}
fn overlay() -> DerivedOverlay {
    let base = vertical();
    let mut edit = base.change_set();
    edit.set(OWNS, SOURCES, ordered_refs(&[VEHICLE]), authored());
    edit.set(
        ENGINE,
        BAG,
        SlotValue::Bag(vec![
            Value::Integer("123456789012345678901234567890".parse().unwrap()),
            Value::Integer((-2).into()),
        ]),
        authored(),
    );
    let source = SourceOrigin {
        document: DocumentId::from_u128(91),
        revision: SourceRevisionId::from_u128(92),
        range: ByteRange::new(2, 19).unwrap(),
        syntax_node: Some(SyntaxNodeId::from_u128(93)),
    };
    edit.set(
        ENGINE,
        NAME,
        text("exact source"),
        DeclaredOrigin::Authored {
            source: Some(source),
        },
    );
    let base = base.apply(&edit).unwrap();
    let mut build = DerivationBuilder::new(base);
    let evidence = Arc::new(proof());
    let searches = Arc::new(BTreeSet::from([
        StructuralSearch::Incoming(ENGINE),
        StructuralSearch::Element(ElementId::from_u128(999)),
    ]));
    for output in 1..=40 {
        build.element_with_explanation(
            key(output),
            PART_DEF,
            [(NAME, text("shared"))],
            evidence.clone(),
        );
        build.searches_shared(FactKey::Element(key(output).element_id()), searches.clone());
    }
    build.extend_ordered_references(OWNS, SOURCES, vec![key(1).element_id()], proof());
    build.property(
        VEHICLE,
        COUNT,
        SlotValue::Scalar(Value::Integer(40.into())),
        proof(),
    );
    build
        .failure(
            ENGINE,
            COUNT,
            ComputationFailure::Incomplete {
                reason: IncompleteReason::MissingInput,
                explanation: proof(),
                searches: (*searches).clone(),
            },
        )
        .unwrap();
    build
        .failure(
            SPORTS,
            COUNT,
            ComputationFailure::Invalid {
                diagnostic: "fixture invalid".into(),
                explanation: proof(),
                searches: BTreeSet::new(),
            },
        )
        .unwrap();
    build.build().unwrap()
}

#[test]
fn roundtrip_preserves_declared_overlay_values_provenance_searches_and_sharing() {
    let original = overlay();
    let mut bytes = vec![];
    write_overlay(&original, &mut bytes).unwrap();
    let restored = read_overlay(Cursor::new(&bytes), registry()).unwrap();
    assert_eq!(restored.base_revision(), original.base_revision());
    assert!(
        restored
            .declared()
            .model()
            .elements()
            .eq(original.declared().model().elements())
    );
    assert!(restored.model().elements().eq(original.model().elements()));
    assert!(restored.facts().eq(original.facts()));
    assert!(
        restored
            .model()
            .computation_failures()
            .eq(original.model().computation_failures())
    );
    assert!(
        restored
            .model()
            .computation_searches()
            .eq(original.model().computation_searches())
    );
    assert_incoming_property_index(restored.model());
    let a = FactKey::Element(key(1).element_id());
    let b = FactKey::Element(key(2).element_id());
    assert!(std::ptr::eq(
        restored.explain(a).unwrap(),
        restored.explain(b).unwrap()
    ));
    assert!(Arc::ptr_eq(
        restored.model().computation_searches_shared(a).unwrap(),
        restored.model().computation_searches_shared(b).unwrap()
    ));
    let mut rewritten = vec![];
    write_overlay(&restored, &mut rewritten).unwrap();
    assert_eq!(bytes, rewritten);
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("{\"Proof\""))
            .count()
            < original.facts().count()
    );
    let immutable = Snapshot::with_immutable_dependency(Arc::new(restored));
    let mut mutation = immutable.change_set();
    mutation.set(ENGINE, NAME, text("forbidden"), authored());
    assert!(matches!(
        immutable.apply(&mutation),
        Err(ModelError::ImmutableDependency(_))
    ));
    assert!(
        write_snapshot(&immutable, vec![]).is_err(),
        "dependency boundary must never be flattened"
    );
}

#[test]
fn retired_element_and_occurrence_ids_remain_reserved_and_registry_is_exact() {
    let empty = Snapshot::new(registry());
    let retired = ElementId::from_u128(500);
    let mut add = empty.change_set();
    add.create(retired, PART_DEF, authored());
    let active = empty.apply(&add).unwrap();
    let mut remove = active.change_set();
    remove.remove(retired);
    let before = active.apply(&remove).unwrap();
    let mut bytes = vec![];
    write_snapshot(&before, &mut bytes).unwrap();
    let after = read_snapshot(Cursor::new(&bytes), registry()).unwrap();
    assert_eq!(before.revision(), after.revision());
    let mut reuse = after.change_set();
    reuse.create(retired, PART_DEF, authored());
    assert!(after.apply(&reuse).is_err());
    let (classes, mut properties) = descriptors();
    properties[0].name = "changed contract".into();
    let changed =
        Arc::new(MetamodelRegistry::new([model_descriptor()], classes, properties).unwrap());
    assert!(read_snapshot(Cursor::new(&bytes), changed).is_err());
    let mut rewrite = vec![];
    write_snapshot(&after, &mut rewrite).unwrap();
    assert_eq!(bytes, rewrite);
}

#[test]
fn corruption_rejects_missing_proofs_missing_dependencies_cycles_and_declared_replacement() {
    let original = overlay();
    let mut bytes = vec![];
    write_overlay(&original, &mut bytes).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let no_proof = text.replacen("\"Proof\":0", "\"Proof\":999999", 1);
    assert_ne!(no_proof, text);
    assert!(read_overlay(Cursor::new(no_proof), registry()).is_err());
    let missing = text.replacen(
        &format!("\"Declared\":{{\"Element\":\"{VEHICLE}\"}}"),
        &format!(
            "\"Declared\":{{\"Element\":\"{}\"}}",
            ElementId::from_u128(987654321)
        ),
        1,
    );
    assert_ne!(missing, text);
    assert!(read_overlay(Cursor::new(missing), registry()).is_err());
    let cycle = text.replacen(
        &format!("\"Declared\":{{\"Element\":\"{VEHICLE}\"}}"),
        &format!("\"Derived\":{{\"Element\":\"{}\"}}", key(1).element_id()),
        1,
    );
    assert_ne!(cycle, text);
    assert!(matches!(
        read_overlay(Cursor::new(cycle), registry()),
        Err(ArchiveError::Derivation(DerivationError::DependencyCycle(
            _
        )))
    ));
    let truncated = text
        .rsplit_once('\n')
        .unwrap()
        .0
        .rsplit_once('\n')
        .unwrap()
        .0;
    assert!(read_overlay(Cursor::new(truncated), registry()).is_err());
    let extra = format!("{text}{{}}\n");
    assert!(read_overlay(Cursor::new(extra), registry()).is_err());
    let corrupt_range = text.replacen("\"range\":[2,19]", "\"range\":[19,2]", 1);
    assert_ne!(corrupt_range, text);
    assert!(read_overlay(Cursor::new(corrupt_range), registry()).is_err());
    // Overlay delta for OWNS preserves declared container and member slots. A
    // syntactically valid change to a declared name must fail before any seal.
    let (declared, delta) = text.split_once("\"Overlay\"\n").unwrap();
    let bad = delta.replacen(
        &format!("\"Reference\":\"{VEHICLE}\""),
        &format!("\"Reference\":\"{SPORTS}\""),
        1,
    );
    assert_ne!(bad, delta);
    assert!(
        read_overlay(
            Cursor::new(format!("{declared}\"Overlay\"\n{bad}")),
            registry()
        )
        .is_err()
    );
}

const ASSOCIATION: AssociationId = AssociationId::from_u128(400);
const LEFT: PropertyId = PropertyId::from_u128(401);
const RIGHT: PropertyId = PropertyId::from_u128(402);
fn association_registry(derived: bool) -> Arc<MetamodelRegistry> {
    let mut properties = vec![];
    for (id, opposite) in [(LEFT, RIGHT), (RIGHT, LEFT)] {
        let mut p = property(
            id,
            "end",
            ELEMENT,
            ValueKind::Reference(ELEMENT),
            Multiplicity::MANY,
        );
        p.owner = PropertyOwner::Association(ASSOCIATION);
        p.association = Some(ASSOCIATION);
        p.opposite_ends.insert(opposite);
        p.ordered = true;
        p.derived = derived;
        properties.push(p);
    }
    Arc::new(
        MetamodelRegistry::from_descriptors(DescriptorSet {
            models: vec![model_descriptor()],
            classes: vec![class(ELEMENT, "Element", &[])],
            properties,
            associations: vec![AssociationDescriptor {
                id: ASSOCIATION,
                name: "pair".into(),
                package: vec![],
                metamodel: MM,
                member_ends: vec![LEFT, RIGHT],
                navigable_owned_ends: BTreeSet::from([LEFT, RIGHT]),
                direct_supertypes: BTreeSet::new(),
                is_abstract: false,
            }],
            ..Default::default()
        })
        .unwrap(),
    )
}
#[test]
fn occurrences_positions_derived_navigation_and_retired_occurrences_roundtrip() {
    let registry = association_registry(false);
    let base = Snapshot::new(registry.clone());
    let mut add = base.change_set();
    for id in [ENGINE, VEHICLE, SPORTS] {
        add.create(id, ELEMENT, authored());
    }
    let retired = AssociationOccurrenceId::from_u128(500);
    let ends = BTreeMap::from([(LEFT, ENGINE), (RIGHT, VEHICLE)]);
    let positions = BTreeMap::from([(LEFT, 0), (RIGHT, 0)]);
    add.link(
        retired,
        ASSOCIATION,
        ends.clone(),
        positions.clone(),
        authored(),
    );
    let base = base.apply(&add).unwrap();
    let mut remove = base.change_set();
    remove.unlink(retired);
    let base = base.apply(&remove).unwrap();
    let declared = AssociationOccurrenceId::from_u128(501);
    let mut link = base.change_set();
    link.link(
        declared,
        ASSOCIATION,
        ends.clone(),
        positions.clone(),
        authored(),
    );
    let base = base.apply(&link).unwrap();
    let mut build = DerivationBuilder::new(base);
    build.association_occurrence(
        key(1),
        ASSOCIATION,
        BTreeMap::from([(LEFT, SPORTS), (RIGHT, VEHICLE)]),
        BTreeMap::from([(LEFT, 1), (RIGHT, 0)]),
        BTreeSet::new(),
    );
    let overlay = build.build().unwrap();
    let mut bytes = vec![];
    write_overlay(&overlay, &mut bytes).unwrap();
    let restored = read_overlay(Cursor::new(&bytes), registry.clone()).unwrap();
    assert!(
        overlay
            .model()
            .association_occurrences()
            .eq(restored.model().association_occurrences())
    );
    assert_eq!(
        overlay.model().navigation_slot(VEHICLE, LEFT),
        restored.model().navigation_slot(VEHICLE, LEFT)
    );
    assert_incoming_property_index(restored.model());
    let mut reuse = restored.declared().change_set();
    reuse.link(retired, ASSOCIATION, ends, positions, authored());
    assert!(restored.declared().apply(&reuse).is_err());
    let mut again = vec![];
    write_overlay(&restored, &mut again).unwrap();
    assert_eq!(bytes, again);

    let registry = association_registry(true);
    let base = Snapshot::new(registry.clone());
    let mut add = base.change_set();
    for id in [ENGINE, VEHICLE] {
        add.create(id, ELEMENT, authored());
    }
    let base = base.apply(&add).unwrap();
    let mut build = DerivationBuilder::new(base);
    build.property(VEHICLE, LEFT, ordered_refs(&[ENGINE]), proof());
    let overlay = build.build().unwrap();
    let mut bytes = vec![];
    write_overlay(&overlay, &mut bytes).unwrap();
    let restored = read_overlay(Cursor::new(&bytes), registry).unwrap();
    assert!(
        overlay
            .model()
            .derived_navigation_results()
            .eq(restored.model().derived_navigation_results())
    );
    assert_incoming_property_index(restored.model());
}

#[test]
fn portable_ids_and_source_ranges_survive_json_values_without_numeric_loss() {
    let id = ElementId::from_u128(u128::MAX);
    let value = serde_json::to_value(id).unwrap();
    assert_eq!(value.as_str(), Some("ffffffff-ffff-ffff-ffff-ffffffffffff"));
    assert_eq!(serde_json::from_value::<ElementId>(value).unwrap(), id);
    assert!(serde_json::from_value::<ByteRange>(serde_json::json!([8, 3])).is_err());
    assert!(serde_json::from_value::<ElementId>(serde_json::json!("invalid")).is_err());
}
