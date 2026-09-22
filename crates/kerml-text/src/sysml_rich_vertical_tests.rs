//! Authored graph preparation; these tests make no producer-closure claim.
use super::*;

#[test]
fn rich_vehicle_vertical_parses_and_lowers_with_current_graph_identity() {
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV2,
        DocumentId::from_u128(900_001),
        SourceRevisionId::from_u128(900_002),
        include_str!("../tests/fixtures/vehicle.sysml"),
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    let query = q(&snapshot);
    let count = model.elements().count();
    for (name, class) in [
        ("Quantity", s::ATTRIBUTE_DEFINITION),
        ("Fuel", s::ITEM_DEFINITION),
        ("Vehicle", s::PART_DEFINITION),
        ("FuelPort", s::PORT_DEFINITION),
        ("FuelLine", s::CONNECTION_DEFINITION),
        ("fuelLine", s::CONNECTION_USAGE),
        ("Drive", s::ACTION_DEFINITION),
        ("Ready", s::STATE_DEFINITION),
        ("RoadReady", s::REQUIREMENT_DEFINITION),
        ("inspectionPassed", s::CONSTRAINT_USAGE),
    ] {
        assert_eq!(
            model.element(named(model, name)).unwrap().metaclass(),
            class,
            "{name}"
        );
    }
    for (usage, definition) in [
        ("engine", "Engine"),
        ("performanceEngine", "TurboEngine"),
        ("tank", "FuelTank"),
        ("mass", "Quantity"),
        ("supply", "MeteredFuelPort"),
        ("contents", "Fuel"),
    ] {
        let types = query.direct_feature_types(named(model, usage));
        assert_eq!(
            types.completeness,
            Completeness::Complete,
            "{usage}: {types:?}"
        );
        assert_eq!(types.value, [named(model, definition)], "{usage}");
    }
    let inherited = query.effective_features(named(model, "SportsCar"));
    assert_eq!(
        inherited.completeness,
        Completeness::Complete,
        "{inherited:?}"
    );
    assert!(inherited.value.contains(&named(model, "performanceEngine")));
    assert!(inherited.value.contains(&named(model, "tank")));
    assert!(inherited.value.contains(&named(model, "supply")));
    assert!(
        !inherited.value.contains(&named(model, "engine")),
        "redefinition suppresses the inherited identity"
    );
    let redefined = query.redefined_features(named(model, "performanceEngine"));
    assert_eq!(
        redefined.completeness,
        Completeness::Complete,
        "{redefined:?}"
    );
    assert_eq!(redefined.value, [named(model, "engine")]);
    let port_members = query.effective_features(named(model, "MeteredFuelPort"));
    assert_eq!(
        port_members.completeness,
        Completeness::Complete,
        "{port_members:?}"
    );
    assert!(port_members.value.contains(&named(model, "fuel")));
    assert!(port_members.value.contains(&named(model, "flowRate")));
    let ends = query.connector_endpoints(named(model, "fuelLine"));
    assert_eq!(ends.completeness, Completeness::Complete, "{ends:?}");
    assert_eq!(
        ends.value,
        [named(model, "supply"), named(model, "returnFlow")]
    );
    assert_eq!(
        model.elements().count(),
        count,
        "inherited records are never copied"
    );
    for record in model.elements() {
        assert!(
            draft
                .source_map()
                .contains_key(&FactKey::Element(record.id()))
        );
    }
}
