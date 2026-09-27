mod common;
use agq_studio_scene::*;
use common::{flat, grid, options};

fn graph() -> SceneOptions {
    options(LayoutKind::Graph)
}

#[test]
fn pin_precedes_soft_positions_and_survives_expansion_and_new_generation() {
    let mut input = flat(5, 6);
    let before = Scene::build(&input, &graph(), None).unwrap();
    let pinned_id = fixtures::id(5);
    let position = before.node(fixtures::id(1)).unwrap().bounds;
    let mut memory = before.memory().clone();
    memory.pin(pinned_id, position).unwrap();
    input.generation = 6;
    let mut added = input.nodes[0].clone();
    added.id = fixtures::id(7);
    added.name = "Expanded neighbor".into();
    input.nodes.push(added);
    let after = Scene::build(&input, &graph(), Some(&memory)).unwrap();
    assert_eq!(after.generation, 6);
    assert_eq!(after.node(pinned_id).unwrap().bounds.min, position.min);
    assert!(after.memory().is_pinned(pinned_id));
    for (index, node) in after.nodes.iter().enumerate() {
        assert_eq!(
            node.semantic,
            *input.nodes.iter().find(|n| n.id == node.id()).unwrap(),
            "pinning must not change the input records"
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
    let input = flat(5, 6);
    let before = Scene::build(&input, &graph(), None).unwrap();
    let pin = fixtures::id(5);
    let target = before.node(fixtures::id(1)).unwrap().bounds;
    let mut memory = before.memory().clone();
    memory.pin(pin, target).unwrap();
    memory.pin(fixtures::id(999), target).unwrap();
    let mut filtered = input.clone();
    filtered.nodes.retain(|node| node.id != pin);
    let hidden = Scene::build(&filtered, &graph(), Some(&memory)).unwrap();
    assert_eq!(hidden.node(fixtures::id(1)).unwrap().bounds, target);
    assert!(hidden.memory().is_pinned(pin));
    let restored = Scene::build(&input, &graph(), Some(hidden.memory())).unwrap();
    assert_eq!(restored.node(pin).unwrap().bounds.min, target.min);
    assert!(restored.node(fixtures::id(999)).is_none());
}

#[test]
fn conflicting_active_pins_are_explicit_and_unpin_recovers() {
    let input = flat(3, 3);
    let before = Scene::build(&input, &graph(), None).unwrap();
    let mut memory = before.memory().clone();
    let bounds = Rect::new(80.0, 120.0, 232.0, 118.0);
    memory.pin(fixtures::id(1), bounds).unwrap();
    memory.pin(fixtures::id(2), bounds).unwrap();
    assert!(matches!(Scene::build(&input, &graph(), Some(&memory)),
        Err(SceneError::GraphPin(PinError::Conflict { first, second }))
        if first == fixtures::id(1) && second == fixtures::id(2)));
    assert!(memory.unpin(fixtures::id(2)));
    assert!(!memory.unpin(fixtures::id(2)));
    let recovered = Scene::build(&input, &graph(), Some(&memory)).unwrap();
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
    let mut input = flat(2, 1);
    let mut memory = LayoutMemory::default();
    memory
        .pin(fixtures::id(1), Rect::new(0.0, 0.0, 232.0, 118.0))
        .unwrap();
    memory
        .pin(fixtures::id(2), Rect::new(0.0, 160.0, 232.0, 118.0))
        .unwrap();
    Scene::build(&input, &graph(), Some(&memory)).unwrap();
    input.nodes[0].ports = (0..24)
        .map(|index| fixtures::port(80_000 + index, &format!("p{index}")))
        .collect();
    assert!(matches!(
        Scene::build(&input, &graph(), Some(&memory)),
        Err(SceneError::GraphPin(PinError::Conflict { .. }))
    ));
    memory.unpin(fixtures::id(2));
    let grown = Scene::build(&input, &graph(), Some(&memory)).unwrap();
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
    let input = grid();
    let expected = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    let mut memory = LayoutMemory::default();
    memory
        .pinned
        .insert(fixtures::id(21), Point::new(-1000.0, 8000.0));
    let scene = Scene::build(&input, &SceneOptions::default(), Some(&memory)).unwrap();
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
fn invalid_pin_geometry_is_rejected_without_changing_layout_memory() {
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
        Scene::build(&flat(1, 0), &graph(), Some(&memory)),
        Err(SceneError::GraphPin(PinError::InvalidBounds(_)))
    ));
}
