use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::*;

fn graph() -> SceneOptions {
    SceneOptions {
        hierarchy: false,
        ..Default::default()
    }
}

#[test]
fn pin_precedes_soft_positions_and_survives_expansion_and_revision_change() {
    let mut projection = fixtures::stress(5, 6);
    let before = SemanticScene::from_projection(&projection, &graph(), None).unwrap();
    let pinned_id = fixtures::id(5);
    let position = before.node(fixtures::id(1)).unwrap().bounds;
    let mut memory = before.memory().clone();
    memory.pin(pinned_id, position).unwrap();
    let revision = ProjectRevisionId::from_u128(0xfa170006);
    projection.revision_id = revision;
    for node in &mut projection.nodes {
        node.revision_id = revision;
    }
    for edge in &mut projection.edges {
        edge.revision_id = revision;
    }
    let mut added = projection.nodes[0].clone();
    added.id = ElementId::from_u128(7);
    added.name = "Expanded neighbor".into();
    projection.nodes.push(added);
    let after = SemanticScene::from_projection(&projection, &graph(), Some(&memory)).unwrap();
    assert_eq!(after.revision_id, revision);
    assert_eq!(after.node(pinned_id).unwrap().bounds.min, position.min);
    assert!(after.memory().is_pinned(pinned_id));
    for (index, node) in after.nodes.iter().enumerate() {
        assert_eq!(
            node.semantic,
            *projection.nodes.iter().find(|n| n.id == node.id()).unwrap(),
            "pinning must not mutate projection records"
        );
        for other in &after.nodes[index + 1..] {
            assert!(!node.bounds.intersects(other.bounds));
        }
    }
    assert_ne!(
        after.node(fixtures::id(1)).unwrap().bounds.min,
        position.min,
        "an unpinned retained node must yield to an active pin"
    );
}

#[test]
fn hidden_and_stale_pins_reserve_no_space_but_return_when_visible() {
    let projection = fixtures::stress(5, 6);
    let before = SemanticScene::from_projection(&projection, &graph(), None).unwrap();
    let pin = fixtures::id(5);
    let target = before.node(fixtures::id(1)).unwrap().bounds;
    let mut memory = before.memory().clone();
    memory.pin(pin, target).unwrap();
    memory.pin(fixtures::id(999), target).unwrap();
    let mut filtered = projection.clone();
    filtered.nodes.retain(|node| node.id != pin);
    let hidden = SemanticScene::from_projection(&filtered, &graph(), Some(&memory)).unwrap();
    assert_eq!(hidden.node(fixtures::id(1)).unwrap().bounds, target);
    assert!(hidden.memory().is_pinned(pin));
    let restored =
        SemanticScene::from_projection(&projection, &graph(), Some(hidden.memory())).unwrap();
    assert_eq!(restored.node(pin).unwrap().bounds.min, target.min);
    assert!(restored.node(fixtures::id(999)).is_none());
}

#[test]
fn conflicting_active_pins_are_explicit_and_unpin_recovers() {
    let projection = fixtures::stress(3, 3);
    let before = SemanticScene::from_projection(&projection, &graph(), None).unwrap();
    let mut memory = before.memory().clone();
    let bounds = Rect::new(80.0, 120.0, 232.0, 118.0);
    memory.pin(fixtures::id(1), bounds).unwrap();
    memory.pin(fixtures::id(2), bounds).unwrap();
    assert!(
        matches!(SemanticScene::from_projection(&projection, &graph(), Some(&memory)),
        Err(SceneError::GraphPin(PinError::Conflict { first, second }))
        if first == fixtures::id(1) && second == fixtures::id(2))
    );
    assert!(memory.unpin(fixtures::id(2)));
    assert!(!memory.unpin(fixtures::id(2)));
    let recovered = SemanticScene::from_projection(&projection, &graph(), Some(&memory)).unwrap();
    assert_eq!(
        recovered.node(fixtures::id(1)).unwrap().bounds.min,
        bounds.min
    );
    assert!(
        !recovered
            .node(fixtures::id(1))
            .unwrap()
            .bounds
            .intersects(recovered.node(fixtures::id(2)).unwrap().bounds)
    );
}

#[test]
fn card_growth_keeps_pinned_origin_and_reports_new_pin_conflicts() {
    let mut projection = fixtures::stress(2, 1);
    let mut memory = LayoutMemory::default();
    memory
        .pin(fixtures::id(1), Rect::new(0.0, 0.0, 232.0, 118.0))
        .unwrap();
    memory
        .pin(fixtures::id(2), Rect::new(0.0, 160.0, 232.0, 118.0))
        .unwrap();
    SemanticScene::from_projection(&projection, &graph(), Some(&memory)).unwrap();
    projection.nodes[0].counts.ports = 24;
    assert!(matches!(
        SemanticScene::from_projection(&projection, &graph(), Some(&memory)),
        Err(SceneError::GraphPin(PinError::Conflict { .. }))
    ));
    memory.unpin(fixtures::id(2));
    let grown = SemanticScene::from_projection(&projection, &graph(), Some(&memory)).unwrap();
    assert_eq!(
        grown.node(fixtures::id(1)).unwrap().bounds.min,
        Point::new(0.0, 0.0)
    );
    assert!(grown.node(fixtures::id(1)).unwrap().bounds.height() > 118.0);
}

#[test]
fn old_sessions_deserialize_and_new_sessions_round_trip_pins() {
    let mut memory: LayoutMemory =
        serde_json::from_value(serde_json::json!({"bounds": {}})).unwrap();
    assert!(!memory.is_pinned(fixtures::id(1)));
    memory
        .pin(fixtures::id(1), Rect::new(-75.0, 40.0, 232.0, 118.0))
        .unwrap();
    let restored: LayoutMemory =
        serde_json::from_str(&serde_json::to_string(&memory).unwrap()).unwrap();
    assert_eq!(restored.pinned, memory.pinned);
    assert_eq!(restored.bounds, memory.bounds);
}

#[test]
fn hierarchy_ignores_graph_pins_without_overwriting_their_anchors() {
    let projection = fixtures::architecture();
    let expected =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
    let mut memory = LayoutMemory::default();
    memory
        .pinned
        .insert(fixtures::id(21), Point::new(-1000.0, 8000.0));
    let scene =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), Some(&memory))
            .unwrap();
    assert_eq!(
        scene.node(fixtures::id(21)).unwrap().bounds,
        expected.node(fixtures::id(21)).unwrap().bounds
    );
    assert_eq!(
        scene.memory().pinned[&fixtures::id(21)],
        Point::new(-1000.0, 8000.0)
    );
}

#[test]
fn invalid_pin_geometry_is_rejected_without_mutating_presentation_memory() {
    let mut memory = LayoutMemory::default();
    assert_eq!(
        memory.pin(fixtures::id(1), Rect::new(f32::NAN, 0.0, 232.0, 118.0)),
        Err(PinError::InvalidBounds(fixtures::id(1)))
    );
    assert!(memory.pinned.is_empty());
    assert!(memory.bounds.is_empty());
    memory
        .pinned
        .insert(fixtures::id(1), Point::new(f32::INFINITY, 0.0));
    assert!(matches!(
        SemanticScene::from_projection(&fixtures::stress(1, 0), &graph(), Some(&memory)),
        Err(SceneError::GraphPin(PinError::InvalidBounds(_)))
    ));
}
