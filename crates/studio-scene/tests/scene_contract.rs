use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::*;

fn architecture() -> SemanticScene {
    SemanticScene::from_projection(&fixtures::architecture(), &SceneOptions::default(), None)
        .unwrap()
}

#[test]
fn revision_bindings_and_identity_are_enforced() {
    let mut projection = fixtures::architecture();
    projection.nodes[0].revision_id = ProjectRevisionId::from_u128(9);
    assert!(matches!(
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None),
        Err(SceneError::MixedRevisions)
    ));
    let mut projection = fixtures::architecture();
    projection.nodes.push(projection.nodes[0].clone());
    assert!(matches!(
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None),
        Err(SceneError::DuplicateElement(_))
    ));
    let mut projection = fixtures::architecture();
    projection.edges.push(projection.edges[0].clone());
    assert!(matches!(
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None),
        Err(SceneError::DuplicateEdge(_))
    ));
}
#[test]
fn ownership_cycles_are_rejected_without_recursive_overflow() {
    let mut projection = fixtures::architecture();
    projection.nodes[0].owner = Some(fixtures::id(11));
    assert!(matches!(
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None),
        Err(SceneError::OwnershipCycle(_))
    ));
}
#[test]
fn hierarchy_contains_children_and_siblings_do_not_overlap() {
    let scene = architecture();
    assert_eq!(scene.nodes.len(), 12);
    assert_eq!(scene.containers.len(), 3);
    assert!(scene.bounds().width() < 1050.0);
    for n in &scene.nodes {
        if let Some(owner) = n.semantic.owner {
            let parent = scene.node(owner).unwrap();
            assert!(parent.bounds.contains_rect(n.bounds));
            assert!(n.bounds.min.y - parent.bounds.min.y >= 60.0);
        }
        for other in scene
            .nodes
            .iter()
            .filter(|other| other.id() > n.id() && other.semantic.owner == n.semantic.owner)
        {
            assert!(!n.bounds.intersects(other.bounds));
        }
    }
}
#[test]
fn rebuild_is_deterministic_and_local_edits_keep_sibling_positions() {
    let before = architecture();
    let projection = fixtures::architecture();
    let rebuilt =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
    assert_eq!(before.memory().bounds, rebuilt.memory().bounds);
    let mut edited = projection;
    let mut added = edited.nodes[6].clone();
    added.id = ElementId::from_u128(7);
    added.name = "New local component".into();
    added.features.clear();
    edited.nodes.push(added);
    let after =
        SemanticScene::from_projection(&edited, &SceneOptions::default(), Some(before.memory()))
            .unwrap();
    for n in &before.nodes {
        assert_eq!(
            after.node(n.id()).unwrap().bounds.min,
            n.bounds.min,
            "retained {} moved",
            n.semantic.name
        );
    }
}
#[test]
fn ports_have_semantic_identity_and_edges_attach_exactly() {
    let scene = architecture();
    assert_eq!(scene.ports.len(), 18);
    let port = scene
        .ports
        .iter()
        .find(|p| p.id == fixtures::id(1102))
        .unwrap();
    let edge = scene
        .edges
        .iter()
        .find(|e| e.semantic.source == port.id)
        .unwrap();
    assert_eq!(edge.points[0], port.position);
    assert_eq!(port.direction, PortDirection::Unspecified);
    let hit = SpatialIndex::build(&scene).hit_test(port.position, 3.0);
    assert_eq!(hit, Some(SceneTarget::Port(port.id)));
}
#[test]
fn routes_are_orthogonal_and_report_obstructions() {
    let scene = architecture();
    for edge in &scene.edges {
        assert!(edge.points.len() >= 2);
        assert!(
            edge.points
                .windows(2)
                .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
        );
    }
    assert_eq!(
        scene
            .edges
            .iter()
            .filter(|e| e.quality == RouteQuality::Obstructed)
            .count(),
        0,
        "architecture corridors should be clear"
    );
}
#[test]
fn camera_pointer_anchor_fit_and_round_trip() {
    let mut camera = Camera2D {
        center: Point::new(-1024.0, 200.0),
        ..Default::default()
    };
    let pointer = Point::new(72.0, 210.0);
    let before = camera.screen_to_world(pointer);
    camera.zoom_at(pointer, 1.75);
    assert!(before.distance(camera.screen_to_world(pointer)) < 0.001);
    let world = Point::new(-450.0, 78.0);
    assert!(world.distance(camera.screen_to_world(camera.world_to_screen(world))) < 0.001);
    let rect = Rect::new(300.0, -10.0, 300.0, 500.0);
    camera.fit(rect, 32.0);
    assert!(camera.visible_rect().contains_rect(rect));
    camera.zoom_at(pointer, f32::NAN);
    assert!(camera.zoom.is_finite());
}
#[test]
fn lod_hysteresis_resists_boundary_noise() {
    let mut lod = LodController::default();
    assert_eq!(lod.update(0.90), LodLevel::Features);
    for zoom in [0.79, 0.81, 0.78, 0.82] {
        assert_eq!(lod.update(zoom), LodLevel::Features);
    }
    assert_eq!(lod.update(0.70), LodLevel::Summary);
    assert_eq!(lod.update(3.0), LodLevel::Evidence);
    assert_eq!(lod.update(0.1), LodLevel::Overview);
}
#[test]
fn culling_and_marquee_use_geometry_including_crossing_edges() {
    let scene = architecture();
    let index = SpatialIndex::build(&scene);
    let n = scene.node(fixtures::id(21)).unwrap();
    assert_eq!(
        index.hit_test(n.bounds.center(), 2.0),
        Some(SceneTarget::Node(n.id()))
    );
    assert!(
        index
            .marquee(n.bounds.inflate(1.0))
            .contains(&SceneTarget::Node(n.id()))
    );
    assert!(
        !index
            .query(Rect::new(-10000.0, -10000.0, 10.0, 10.0))
            .iter()
            .any(|x| matches!(x, SceneTarget::Node(_)))
    );
    let edge = &scene.edges[0];
    let midpoint = Point::new(
        (edge.points[0].x + edge.points[1].x) * 0.5,
        (edge.points[0].y + edge.points[1].y) * 0.5,
    );
    assert!(
        index
            .query(Rect::new(midpoint.x - 1.0, midpoint.y - 1.0, 2.0, 2.0))
            .contains(&SceneTarget::Edge(edge.semantic.id.clone()))
    );
}
#[test]
fn collapsed_and_focused_views_retain_original_ids() {
    let projection = fixtures::architecture();
    let mut options = SceneOptions::default();
    options.collapsed.insert(fixtures::id(2));
    let scene = SemanticScene::from_projection(&projection, &options, None).unwrap();
    assert!(scene.node(fixtures::id(21)).is_none());
    assert!(scene.node(fixtures::id(2)).unwrap().collapsed);
    options.collapsed.clear();
    options.focus = Some(fixtures::id(2));
    let scene = SemanticScene::from_projection(&projection, &options, None).unwrap();
    assert_eq!(scene.nodes.len(), 4);
    assert!(
        scene
            .nodes
            .iter()
            .all(|n| n.id() == fixtures::id(2) || n.semantic.owner == Some(fixtures::id(2)))
    );
}
#[test]
fn diff_retains_ghost_revision_and_does_not_mark_unchanged_revision_ids() {
    let (a, b) = fixtures::revision_diff();
    let before = SemanticScene::from_projection(&a, &SceneOptions::default(), None).unwrap();
    let mut after =
        SemanticScene::from_projection(&b, &SceneOptions::default(), Some(before.memory()))
            .unwrap();
    after.apply_diff(&before);
    assert_eq!(
        after.node(fixtures::id(21)).unwrap().diff,
        DiffMark::Unchanged
    );
    assert_eq!(
        after.node(fixtures::id(22)).unwrap().diff,
        DiffMark::Changed
    );
    assert_eq!(after.node(fixtures::id(24)).unwrap().diff, DiffMark::Added);
    let removed = after.node(fixtures::id(13)).unwrap();
    assert_eq!(removed.diff, DiffMark::Removed);
    assert_eq!(removed.semantic.revision_id, before.revision_id);
}
#[test]
fn graph_layout_keeps_semantic_owners_and_handles_cycles() {
    let projection = fixtures::architecture();
    let options = SceneOptions {
        hierarchy: false,
        ..Default::default()
    };
    let scene = SemanticScene::from_projection(&projection, &options, None).unwrap();
    assert!(scene.containers.is_empty());
    assert_eq!(
        scene.node(fixtures::id(21)).unwrap().semantic.owner,
        Some(fixtures::id(2))
    );
    let cycle = fixtures::stress(100, 200);
    let scene = SemanticScene::from_projection(&cycle, &options, None).unwrap();
    assert_eq!(scene.nodes.len(), 100);
    assert!(scene.bounds().width() < 4000.0);
    assert!(scene.bounds().height() < 4000.0);
}
#[test]
fn unknown_categories_are_never_guessed_by_substring() {
    assert_eq!(
        NodeCategory::from_semantic_kind("MisleadingPartDefinitionWrapper"),
        NodeCategory::Unknown
    );
    // Agent is a presentation participant category, not a pinned SysML metaclass.
    assert_eq!(
        NodeCategory::from_semantic_kind("Agent"),
        NodeCategory::Unknown
    );
}

#[test]
fn parallel_relationships_have_distinct_routes() {
    let mut projection = fixtures::architecture();
    let mut parallel = projection.edges[0].clone();
    parallel.id = "parallel".into();
    projection.edges.push(parallel);
    let scene =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
    let a = scene
        .edges
        .iter()
        .find(|e| e.semantic.id == projection.edges[0].id)
        .unwrap();
    let b = scene
        .edges
        .iter()
        .find(|e| e.semantic.id == "parallel")
        .unwrap();
    assert_ne!(a.points, b.points);
}

#[test]
fn collapsed_layout_restores_hidden_component_positions() {
    let before = architecture();
    let mut options = SceneOptions::default();
    options.collapsed.insert(fixtures::id(2));
    let collapsed =
        SemanticScene::from_projection(&fixtures::architecture(), &options, Some(before.memory()))
            .unwrap();
    let expanded = SemanticScene::from_projection(
        &fixtures::architecture(),
        &SceneOptions::default(),
        Some(collapsed.memory()),
    )
    .unwrap();
    for n in &before.nodes {
        assert_eq!(n.bounds.min, expanded.node(n.id()).unwrap().bounds.min);
    }
}

#[test]
fn routing_detours_around_an_unrelated_node() {
    let mut projection = fixtures::stress(3, 1);
    projection.edges[0].source = projection.nodes[0].id;
    projection.edges[0].target = projection.nodes[2].id;
    let options = SceneOptions {
        hierarchy: false,
        ..Default::default()
    };
    let memory = LayoutMemory {
        bounds: std::collections::BTreeMap::from([
            (projection.nodes[0].id, Rect::new(0.0, 0.0, 232.0, 118.0)),
            (projection.nodes[1].id, Rect::new(300.0, 0.0, 232.0, 118.0)),
            (projection.nodes[2].id, Rect::new(600.0, 0.0, 232.0, 118.0)),
        ]),
    };
    let scene = SemanticScene::from_projection(&projection, &options, Some(&memory)).unwrap();
    assert_eq!(scene.edges[0].quality, RouteQuality::Clear);
    assert!(
        scene.edges[0]
            .points
            .iter()
            .any(|p| p.y < 0.0 || p.y > 118.0)
    );
}

#[test]
fn self_relationship_routes_form_a_visible_loop() {
    let mut projection = fixtures::stress(1, 1);
    projection.edges[0].target = projection.edges[0].source;
    let scene =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
    assert!(scene.edges[0].points.len() >= 5);
    assert!(scene.edges[0].bounds.width() > 0.0 && scene.edges[0].bounds.height() > 0.0);
    assert_eq!(scene.edges[0].quality, RouteQuality::Clear);
}

#[test]
fn huge_spatial_queries_use_overflow_lane_without_integer_overflow() {
    let scene = architecture();
    let index = SpatialIndex::build(&scene);
    let hits = index.visible(Rect::new(-1.0e20, -1.0e20, 2.0e20, 2.0e20));
    assert!(hits.contains(&SceneTarget::Node(fixtures::id(21))));
    assert!(!Rect::from_points(Point::new(-f32::MAX, 0.0), Point::new(f32::MAX, 1.0)).finite());
}

#[test]
fn neighborhood_is_one_hop_order_independent_and_resolves_port_owners() {
    use agq_modeling_view::RelationshipFamily;
    use std::collections::BTreeSet;
    let mut projection = fixtures::architecture();
    let seeds = BTreeSet::from([fixtures::id(21)]);
    let families = BTreeSet::from([RelationshipFamily::Connection]);
    let before = expand_neighborhood(&projection, &seeds, &families, NeighborhoodDirection::Both);
    assert!(before.elements.contains(&fixtures::id(11)));
    assert!(before.elements.contains(&fixtures::id(31)));
    assert!(!before.elements.contains(&fixtures::id(23)));
    assert_eq!(before.relationships.len(), 2);
    projection.edges.reverse();
    assert_eq!(
        before,
        expand_neighborhood(&projection, &seeds, &families, NeighborhoodDirection::Both)
    );
}

#[test]
fn scene_lookup_resolves_only_culled_objects_in_containment_order() {
    let scene = architecture();
    let lookup = SceneLookup::build(&scene);
    let targets = vec![
        SceneTarget::Node(fixtures::id(21)),
        SceneTarget::Container(fixtures::id(2)),
        SceneTarget::Port(fixtures::id(2101)),
        SceneTarget::Node(fixtures::id(21)),
        SceneTarget::Edge(scene.edges[0].semantic.id.clone()),
    ];
    let visible = lookup.visible(&scene, &targets);
    assert_eq!(
        visible.nodes.iter().map(|n| n.id()).collect::<Vec<_>>(),
        vec![fixtures::id(2), fixtures::id(21)]
    );
    assert_eq!(visible.ports.len(), 1);
    assert_eq!(visible.edges.len(), 1);
    assert_eq!(lookup.endpoint_owner(fixtures::id(2101)), fixtures::id(21));
    assert_eq!(lookup.endpoint_owner(fixtures::id(987)), fixtures::id(987));
    assert_eq!(
        lookup.node(&scene, fixtures::id(21)).unwrap().semantic.name,
        "ModelRepository"
    );
    assert_eq!(
        lookup.port(&scene, fixtures::id(2101)).unwrap().id,
        fixtures::id(2101)
    );
    assert_eq!(
        lookup
            .edge(&scene, &scene.edges[0].semantic.id)
            .unwrap()
            .semantic
            .id,
        scene.edges[0].semantic.id
    );
}

#[test]
fn stale_scene_lookup_cannot_return_unrelated_records_or_cross_revisions() {
    let scene = architecture();
    let lookup = SceneLookup::build(&scene);
    let mut reordered = scene.clone();
    reordered.nodes.reverse();
    assert!(lookup.node(&reordered, fixtures::id(21)).is_none());
    let mut projection = fixtures::architecture();
    projection.nodes.truncate(1);
    projection.edges.clear();
    let small =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
    assert!(lookup.node(&small, fixtures::id(21)).is_none());
    let mut other = scene;
    other.revision_id = ProjectRevisionId::from_u128(9);
    assert!(lookup.node(&other, fixtures::id(21)).is_none());
    assert!(
        lookup
            .visible(&other, &[SceneTarget::Node(fixtures::id(21))])
            .nodes
            .is_empty()
    );
}
