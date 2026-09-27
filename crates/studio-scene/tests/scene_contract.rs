mod common;
use agq_studio_scene::fixtures::{at, card, id, port};
use agq_studio_scene::*;
use common::{flat, grid, grid_change, options};
use std::collections::BTreeSet;

fn architecture() -> Scene {
    Scene::build(&grid(), &SceneOptions::default(), None).unwrap()
}

#[test]
fn duplicate_identities_are_rejected() {
    let mut input = grid();
    input.nodes.push(input.nodes[0].clone());
    assert!(matches!(
        Scene::build(&input, &SceneOptions::default(), None),
        Err(SceneError::DuplicateElement(_))
    ));
    let mut input = grid();
    input.edges.push(input.edges[0].clone());
    assert!(matches!(
        Scene::build(&input, &SceneOptions::default(), None),
        Err(SceneError::DuplicateEdge(_))
    ));
}
#[test]
fn ownership_cycles_are_rejected_without_recursive_overflow() {
    let mut input = grid();
    input.nodes[0].owner = Some(id(11));
    assert!(matches!(
        Scene::build(&input, &SceneOptions::default(), None),
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
fn the_url_shortener_nests_parts_and_shows_ports_through_types() {
    let input = fixtures::architecture();
    let named = |name: &str| input.nodes.iter().find(|n| n.name == name).unwrap();
    let service = named("UrlShortenerService");
    let api = named("api");
    assert_eq!(api.owner, Some(service.id));
    assert_eq!(api.detail, ": HttpApi");
    // `api : HttpApi` shows HttpApi's ports with their real identities.
    let definition = named("HttpApi");
    let ids =
        |ports: &[agq_studio_scene::InputPort]| ports.iter().map(|p| p.id).collect::<Vec<_>>();
    assert_eq!(ids(&api.ports), ids(&definition.ports));
    // On the usage they are marked as defined in HttpApi; on HttpApi they are its own.
    assert!(
        api.ports
            .iter()
            .all(|p| p.defined_in == Some(definition.id))
    );
    assert!(definition.ports.iter().all(|p| p.defined_in.is_none()));
    assert!(api.ports.iter().any(|p| p.name == "storage"));
    let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    let storage = scene
        .edges
        .iter()
        .find(|e| e.semantic.kind == EdgeKind::Interface && e.semantic.label.contains("storage"))
        .unwrap();
    assert_eq!(storage.semantic.source.node, api.id);
    let source = scene
        .ports
        .iter()
        .find(|p| p.owner == api.id && Some(p.id) == storage.semantic.source.port)
        .unwrap();
    assert_eq!(storage.points[0], source.position);
    assert!(
        scene
            .nodes
            .iter()
            .all(|n| n.semantic.problems == 0 && !n.semantic.lock.locked())
    );
}
#[test]
fn rebuild_is_deterministic_and_local_edits_keep_sibling_positions() {
    let before = architecture();
    let input = grid();
    let rebuilt = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    assert_eq!(before.memory().bounds, rebuilt.memory().bounds);
    let mut edited = input;
    let mut added = edited.nodes[6].clone();
    added.id = id(7);
    added.name = "New local component".into();
    added.ports.clear();
    edited.nodes.push(added);
    let after = Scene::build(&edited, &SceneOptions::default(), Some(before.memory())).unwrap();
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
fn ports_have_identity_and_edges_attach_exactly() {
    let scene = architecture();
    assert_eq!(scene.ports.len(), 18);
    let port = scene.ports.iter().find(|p| p.id == id(1102)).unwrap();
    assert_eq!(port.owner, id(11));
    let edge = scene
        .edges
        .iter()
        .find(|e| e.semantic.source == at(11, 1102))
        .unwrap();
    assert_eq!(edge.points[0], port.position);
    assert_eq!(port.direction, PortDirection::Unspecified);
    let hit = SpatialIndex::build(&scene).hit_test(port.position, 3.0);
    assert_eq!(hit, Some(SceneTarget::Port(port.owner, port.id)));
}
#[test]
fn the_same_port_on_two_cards_is_two_targets() {
    let mut input = grid();
    // Card 12 shows port 1102 as well, as a usage shows its type's ports.
    input.nodes[4].ports.push(port(1102, "request"));
    let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    let shown: Vec<_> = scene.ports.iter().filter(|p| p.id == id(1102)).collect();
    assert_eq!(shown.len(), 2);
    let index = SpatialIndex::build(&scene);
    for port in shown {
        assert_eq!(
            index.hit_test(port.position, 3.0),
            Some(SceneTarget::Port(port.owner, port.id))
        );
    }
    let edge = scene
        .edges
        .iter()
        .find(|e| e.semantic.source == at(11, 1102))
        .unwrap();
    let on_11 = scene
        .ports
        .iter()
        .find(|p| p.owner == id(11) && p.id == id(1102))
        .unwrap();
    assert_eq!(edge.points[0], on_11.position);
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
    let n = scene.node(id(21)).unwrap();
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
    let input = grid();
    let mut options = SceneOptions::default();
    options.collapsed.insert(id(2));
    let scene = Scene::build(&input, &options, None).unwrap();
    assert!(scene.node(id(21)).is_none());
    assert!(scene.node(id(2)).unwrap().collapsed);
    options.collapsed.clear();
    options.focus = Some(id(2));
    let scene = Scene::build(&input, &options, None).unwrap();
    assert_eq!(scene.nodes.len(), 4);
    assert!(
        scene
            .nodes
            .iter()
            .all(|n| n.id() == id(2) || n.semantic.owner == Some(id(2)))
    );
}
#[test]
fn collapsed_container_shows_external_port_connections_on_its_boundary() {
    let input = grid();
    let mut options = SceneOptions::default();
    options.collapsed.insert(id(2));
    let scene = Scene::build(&input, &options, None).unwrap();
    let proxy = scene.ports.iter().find(|port| port.id == id(2101)).unwrap();
    assert_eq!(proxy.owner, id(2));
    assert_eq!(proxy.proxy_for_owner, Some(id(21)));
    assert_eq!(proxy.direction, PortDirection::Unspecified);
    assert!(proxy.name.contains("Repository"));
    let connection = scene
        .edges
        .iter()
        .find(|edge| edge.semantic.target == at(21, 2101))
        .unwrap();
    assert_eq!(connection.points.last(), Some(&proxy.position));
    assert_eq!(connection.semantic, input.edges[0]);
    assert_eq!(
        scene
            .edges
            .iter()
            .filter(|edge| edge.semantic.kind == EdgeKind::Connection)
            .count(),
        6
    );
}

#[test]
fn diff_does_not_union_old_absolute_container_positions() {
    let (_, changed) = grid_change();
    let before = Scene::build(&changed, &SceneOptions::default(), None).unwrap();
    let mut edited = changed.clone();
    let mut added = edited
        .nodes
        .iter()
        .find(|node| node.id == id(24))
        .unwrap()
        .clone();
    added.id = id(17);
    added.name = "NestedPart".into();
    added.ports.clear();
    edited.nodes.push(added);
    let after = Scene::comparison(
        &changed,
        &edited,
        &BTreeSet::new(),
        &SceneOptions::default(),
        Some(before.memory()),
    )
    .unwrap();
    for node in &after.nodes {
        if let Some(owner) = node.semantic.owner {
            assert!(
                after.node(owner).unwrap().bounds.contains_rect(node.bounds),
                "{} escaped owner",
                node.semantic.name
            );
        }
        for sibling in after
            .nodes
            .iter()
            .filter(|other| other.id() > node.id() && other.semantic.owner == node.semantic.owner)
        {
            assert!(
                !node.bounds.intersects(sibling.bounds),
                "{} overlaps {}",
                node.semantic.name,
                sibling.semantic.name
            );
        }
    }
}

#[test]
fn moved_diff_container_keeps_its_current_position() {
    let before = architecture();
    let mut memory = before.memory().clone();
    for node in &before.nodes {
        if node.id() == id(3) || node.semantic.owner == Some(id(3)) {
            memory
                .bounds
                .insert(node.id(), node.bounds.translate(Point::new(1000.0, 0.0)));
        }
    }
    let current = Scene::build(&grid(), &SceneOptions::default(), Some(&memory)).unwrap();
    let expected = current.node(id(3)).unwrap().bounds;
    let compared = Scene::comparison(
        &grid(),
        &grid(),
        &BTreeSet::new(),
        &SceneOptions::default(),
        Some(&memory),
    )
    .unwrap();
    assert_eq!(compared.node(id(3)).unwrap().bounds, expected);
    assert!(compared.nodes.iter().all(|n| n.diff == DiffMark::Unchanged));
}

#[test]
fn many_ports_have_readable_separation_in_both_layouts() {
    let mut input = grid();
    let node = input
        .nodes
        .iter_mut()
        .find(|node| node.id == id(21))
        .unwrap();
    node.ports = (0..24)
        .map(|index| port(80_000 + index, &format!("pressure_{index}_μPa")))
        .collect();
    for layout in [LayoutKind::Hierarchy, LayoutKind::Graph] {
        let scene = Scene::build(&input, &options(layout), None).unwrap();
        let mut left: Vec<_> = scene
            .ports
            .iter()
            .filter(|port| port.owner == id(21) && port.side == PortSide::Left)
            .collect();
        left.sort_by(|a, b| a.position.y.total_cmp(&b.position.y));
        assert_eq!(left.len(), 12);
        assert!(
            left.windows(2)
                .all(|pair| pair[1].position.y - pair[0].position.y >= 20.0)
        );
    }
}
#[test]
fn diff_marks_changes_and_keeps_removed_ghosts() {
    let (a, b) = grid_change();
    let before = Scene::build(&a, &SceneOptions::default(), None).unwrap();
    let compare = |changed: BTreeSet<_>| {
        Scene::comparison(
            &a,
            &b,
            &changed,
            &SceneOptions::default(),
            Some(before.memory()),
        )
        .unwrap()
    };
    let after = compare(BTreeSet::new());
    assert_eq!(after.node(id(21)).unwrap().diff, DiffMark::Unchanged);
    assert_eq!(after.node(id(22)).unwrap().diff, DiffMark::Changed);
    assert_eq!(after.node(id(24)).unwrap().diff, DiffMark::Added);
    let removed = after.node(id(13)).unwrap();
    assert_eq!(removed.diff, DiffMark::Removed);
    assert_eq!(removed.semantic.name, "Inspector");
    let ghost = after
        .edges
        .iter()
        .find(|e| e.semantic.source == at(13, 1302))
        .unwrap();
    assert_eq!(ghost.diff, DiffMark::Removed);
    // Elements known to have changed in ways the scene does not show are marked.
    let again = compare(BTreeSet::from([id(21)]));
    assert_eq!(again.node(id(21)).unwrap().diff, DiffMark::Changed);
}
#[test]
fn graph_layout_keeps_owners_and_handles_cycles() {
    let scene = Scene::build(&grid(), &options(LayoutKind::Graph), None).unwrap();
    assert!(scene.containers.is_empty());
    assert_eq!(scene.node(id(21)).unwrap().semantic.owner, Some(id(2)));
    let scene = Scene::build(&flat(100, 200), &options(LayoutKind::Graph), None).unwrap();
    assert_eq!(scene.nodes.len(), 100);
    assert!(scene.bounds().width() < 4000.0);
    assert!(scene.bounds().height() < 4000.0);
}

#[test]
fn parallel_relationships_have_distinct_routes() {
    let mut input = grid();
    let mut parallel = input.edges[0].clone();
    parallel.id = "parallel".into();
    input.edges.push(parallel);
    let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    let a = scene
        .edges
        .iter()
        .find(|e| e.semantic.id == input.edges[0].id)
        .unwrap();
    let b = scene
        .edges
        .iter()
        .find(|e| e.semantic.id == "parallel")
        .unwrap();
    assert_ne!(a.points, b.points);
}

#[test]
fn twenty_parallel_routes_select_the_nearest_exact_relationship_across_zoom_and_order() {
    let mut input = flat(2, 1);
    let original = input.edges[0].clone();
    input.edges = (0..20)
        .map(|index| {
            let mut edge = original.clone();
            edge.id = format!("parallel-{index:02}");
            edge.element = Some(id(1000 + index));
            edge
        })
        .collect();
    let options = options(LayoutKind::Graph);
    let scene = Scene::build(&input, &options, None).unwrap();
    let index = SpatialIndex::build(&scene);
    assert_eq!(scene.edges.len(), 20);
    assert!(
        scene.ports.is_empty(),
        "drawing anchors must not invent ports"
    );
    for edge in &scene.edges {
        assert_eq!(
            &edge.semantic,
            input
                .edges
                .iter()
                .find(|e| e.id == edge.semantic.id)
                .unwrap()
        );
        let start = edge.points[0];
        let stub = edge.points[1];
        assert!((start.y - stub.y).abs() < 0.001 && start.distance(stub) >= 12.0);
        let direction = (stub.x - start.x).signum();
        // Dense card-side attachments are about four world units apart. A
        // normal pointer radius contains several routes; identity ordering
        // must not steal a click from the closer visible line.
        for zoom in [0.45, 0.75, 1.0, 2.0, 4.0] {
            for offset in [-0.7, 0.0, 0.7] {
                let point = Point::new(start.x + direction * 10.0, start.y + offset);
                assert_eq!(
                    index.hit_test(point, 6.0 / zoom),
                    Some(SceneTarget::Edge(edge.semantic.id.clone())),
                    "wrong relationship at zoom {zoom}, offset {offset}"
                );
            }
        }
    }
    let mut reordered = input.clone();
    reordered.edges.reverse();
    let reordered = Scene::build(&reordered, &options, None).unwrap();
    let reordered_index = SpatialIndex::build(&reordered);
    for edge in &scene.edges {
        let other = reordered
            .edges
            .iter()
            .find(|other| other.semantic.id == edge.semantic.id)
            .unwrap();
        assert_eq!(edge.points, other.points);
        let point = Point::new(
            edge.points[0].x + (edge.points[1].x - edge.points[0].x).signum() * 10.0,
            edge.points[0].y,
        );
        assert_eq!(
            index.hit_test(point, 12.0),
            reordered_index.hit_test(point, 12.0)
        );
    }
    let node = &scene.nodes[0];
    assert_eq!(
        index.hit_test(node.bounds.center(), 12.0),
        Some(SceneTarget::Node(node.id())),
        "nearest-edge correction must preserve node selection priority"
    );
}

#[test]
fn several_satisfy_edges_into_one_requirement_are_individually_selectable() {
    let mut input = SceneInput {
        generation: 1,
        ..Default::default()
    };
    for requirement in [101, 102, 103] {
        input.nodes.push(fixtures::node(
            requirement,
            &format!("Requirement {requirement}"),
            NodeCategory::Requirement,
            None,
        ));
        for part in [requirement * 10, requirement * 10 + 1] {
            input.nodes.push(fixtures::node(
                part,
                &format!("Part {part}"),
                NodeCategory::Part,
                None,
            ));
            input.edges.push(fixtures::edge(
                &format!("satisfy-{part}"),
                EdgeKind::Satisfy,
                card(part),
                card(requirement),
            ));
        }
    }
    let options = options(LayoutKind::Requirements);
    let scene = Scene::build(&input, &options, None).unwrap();
    let index = SpatialIndex::build(&scene);
    assert!(
        scene.ports.is_empty(),
        "drawing anchors must not invent ports"
    );
    for requirement in [101, 102, 103] {
        let edges: Vec<_> = scene
            .edges
            .iter()
            .filter(|edge| edge.semantic.target == card(requirement))
            .collect();
        assert_eq!(edges.len(), 2);
        assert!(
            edges[0]
                .points
                .last()
                .unwrap()
                .distance(*edges[1].points.last().unwrap())
                >= 12.0,
            "two edges into one card need distinct arrowheads"
        );
        for edge in edges {
            let end = *edge.points.last().unwrap();
            let before = edge.points[edge.points.len() - 2];
            let length = end.distance(before);
            assert!(length >= 12.0);
            let probe = Point::new(
                end.x + (before.x - end.x) / length * 10.0,
                end.y + (before.y - end.y) / length * 10.0,
            );
            assert_eq!(
                index.hit_test(probe, 3.0),
                Some(SceneTarget::Edge(edge.semantic.id.clone())),
                "each relationship must remain independently selectable at its target"
            );
            assert_eq!(edge.quality, RouteQuality::Clear);
        }
    }
    let mut reordered = input;
    reordered.edges.reverse();
    let reordered = Scene::build(&reordered, &options, None).unwrap();
    for edge in &scene.edges {
        assert_eq!(
            edge.points,
            reordered
                .edges
                .iter()
                .find(|other| other.semantic.id == edge.semantic.id)
                .unwrap()
                .points
        );
    }
}

#[test]
fn parallel_card_links_have_separate_endpoints_but_ports_remain_exact() {
    let mut input = flat(2, 1);
    for index in 0..3 {
        let mut edge = input.edges[0].clone();
        edge.id = format!("independent-{index}");
        input.edges.push(edge);
    }
    let scene = Scene::build(&input, &options(LayoutKind::Graph), None).unwrap();
    for (index, edge) in scene.edges.iter().enumerate() {
        for other in &scene.edges[index + 1..] {
            assert_ne!(edge.points.first(), other.points.first());
            assert_ne!(edge.points.last(), other.points.last());
        }
    }
    let mut input = grid();
    let mut extra = input.edges[0].clone();
    extra.id = "same-port".into();
    input.edges.push(extra);
    let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    let position = |end: InputEnd| {
        scene
            .ports
            .iter()
            .find(|port| port.owner == end.node && Some(port.id) == end.port)
            .unwrap()
            .position
    };
    let mut checked = 0;
    for edge in scene
        .edges
        .iter()
        .filter(|edge| edge.semantic.source == at(11, 1102))
    {
        assert_eq!(edge.points.first(), Some(&position(edge.semantic.source)));
        assert_eq!(edge.points.last(), Some(&position(edge.semantic.target)));
        checked += 1;
    }
    assert_eq!(checked, 2);
}

#[test]
fn separated_attachment_stubs_stay_inside_dense_default_layout_gutters() {
    let input = flat(80, 160);
    for layout in [LayoutKind::Hierarchy, LayoutKind::Graph] {
        let scene = Scene::build(&input, &options(layout), None).unwrap();
        assert_eq!(scene.edges.len(), 160);
        assert!(
            scene
                .edges
                .iter()
                .all(|edge| edge.quality == RouteQuality::Clear),
            "attachment staggering must not drive stubs through unrelated neighbours"
        );
    }
}

#[test]
fn collapsed_layout_restores_hidden_component_positions() {
    let before = architecture();
    let mut options = SceneOptions::default();
    options.collapsed.insert(id(2));
    let collapsed = Scene::build(&grid(), &options, Some(before.memory())).unwrap();
    let expanded =
        Scene::build(&grid(), &SceneOptions::default(), Some(collapsed.memory())).unwrap();
    for n in &before.nodes {
        assert_eq!(n.bounds.min, expanded.node(n.id()).unwrap().bounds.min);
    }
}

#[test]
fn routing_detours_around_an_unrelated_node() {
    let mut input = flat(3, 1);
    input.edges[0].source = card(1);
    input.edges[0].target = card(3);
    let memory = LayoutMemory {
        bounds: std::collections::BTreeMap::from([
            (id(1), Rect::new(0.0, 0.0, 232.0, 118.0)),
            (id(2), Rect::new(300.0, 0.0, 232.0, 118.0)),
            (id(3), Rect::new(600.0, 0.0, 232.0, 118.0)),
        ]),
        ..Default::default()
    };
    let scene = Scene::build(&input, &options(LayoutKind::Graph), Some(&memory)).unwrap();
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
    let mut input = flat(1, 1);
    input.edges[0].target = input.edges[0].source;
    let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    assert!(scene.edges[0].points.len() >= 5);
    assert!(scene.edges[0].bounds.width() > 0.0 && scene.edges[0].bounds.height() > 0.0);
    assert_eq!(scene.edges[0].quality, RouteQuality::Clear);
}

#[test]
fn parallel_self_relationships_keep_separate_anchors_and_visible_exterior_loops() {
    for count in [1, 2, 5] {
        let mut input = flat(1, count);
        for edge in &mut input.edges {
            edge.source = card(1);
            edge.target = card(1);
        }
        let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
        let bounds = scene.node(id(1)).unwrap().bounds;
        assert_eq!(scene.edges.len(), count);
        assert!(scene.ports.is_empty(), "drawing anchors are not ports");
        for route in &scene.edges {
            assert_eq!(
                &route.semantic,
                input
                    .edges
                    .iter()
                    .find(|edge| edge.id == route.semantic.id)
                    .unwrap()
            );
            let first = *route.points.first().unwrap();
            let last = *route.points.last().unwrap();
            assert_eq!(first.x, bounds.max.x);
            assert_eq!(last.x, bounds.max.x);
            assert_ne!(first, last, "self-link attachment lanes stay distinct");
            assert!(
                route.points.len() >= 6,
                "a short U is not the self-link loop: {:?}",
                route.points
            );
            assert!(route.points.iter().all(|point| point.x >= bounds.max.x));
            assert!(route.points.iter().any(|point| {
                point.y <= first.y.min(last.y) - 34.0 + 0.001
                    || point.y >= first.y.max(last.y) + 34.0 - 0.001
            }));
            assert!(
                route
                    .points
                    .windows(2)
                    .all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y)
            );
            assert_eq!(route.quality, RouteQuality::Clear);
        }
        for pair in scene.edges.windows(2) {
            assert_ne!(pair[0].points, pair[1].points);
        }
    }
}

#[test]
fn expanded_owner_port_strips_are_clear_bounded_and_keep_port_identity() {
    for count in [1_usize, 2, 4, 5, 24] {
        let mut input = grid();
        let owner = id(2);
        let ports: Vec<_> = (0..count)
            .map(|index| port(80_000 + index as u64, &format!("boundary_{index}")))
            .collect();
        input
            .nodes
            .iter_mut()
            .find(|node| node.id == owner)
            .unwrap()
            .ports = ports.clone();
        let scene = Scene::build(&input, &SceneOptions::default(), None).unwrap();
        let container = scene.node(owner).unwrap();
        let shown: Vec<_> = scene
            .ports
            .iter()
            .filter(|port| port.owner == owner)
            .collect();
        assert_eq!(shown.len(), count);
        assert_eq!(
            shown.iter().map(|port| port.id).collect::<BTreeSet<_>>(),
            ports.iter().map(|port| port.id).collect()
        );
        assert!(
            shown.iter().all(|port| port.proxy_for_owner.is_none()
                && port.direction == PortDirection::Unspecified)
        );
        let child_top = scene
            .nodes
            .iter()
            .filter(|node| node.semantic.owner == Some(owner))
            .map(|node| node.bounds.min.y)
            .reduce(f32::min)
            .unwrap();
        let header = child_top - container.bounds.min.y;
        if count <= 4 {
            assert!((92.0..=116.0).contains(&header));
            assert!(shown.iter().all(|port| port.label_in_header
                && port.position.y >= container.bounds.min.y + 72.0
                && port.position.y + 20.0 <= child_top));
            let index = SpatialIndex::build(&scene);
            for port in &shown {
                assert_eq!(
                    index.hit_test(port.position, 3.0),
                    Some(SceneTarget::Port(owner, port.id))
                );
            }
        } else {
            assert_eq!(
                header, 72.0,
                "dense ports must not grow an unbounded header"
            );
            assert!(shown.iter().all(|port| !port.label_in_header));
        }
        let again = Scene::build(&input, &SceneOptions::default(), Some(scene.memory())).unwrap();
        assert_eq!(
            scene
                .nodes
                .iter()
                .map(|node| (node.id(), node.bounds))
                .collect::<Vec<_>>(),
            again
                .nodes
                .iter()
                .map(|node| (node.id(), node.bounds))
                .collect::<Vec<_>>(),
            "unchanged port strips must not keep moving the layout"
        );
    }
}

#[test]
fn huge_spatial_queries_use_overflow_lane_without_integer_overflow() {
    let scene = architecture();
    let index = SpatialIndex::build(&scene);
    let hits = index.visible(Rect::new(-1.0e20, -1.0e20, 2.0e20, 2.0e20));
    assert!(hits.contains(&SceneTarget::Node(id(21))));
    assert!(!Rect::from_points(Point::new(-f32::MAX, 0.0), Point::new(f32::MAX, 1.0)).finite());
}

#[test]
fn scene_lookup_resolves_only_culled_objects_in_containment_order() {
    let scene = architecture();
    let lookup = SceneLookup::build(&scene);
    let targets = vec![
        SceneTarget::Node(id(21)),
        SceneTarget::Container(id(2)),
        SceneTarget::Port(id(21), id(2101)),
        SceneTarget::Node(id(21)),
        SceneTarget::Edge(scene.edges[0].semantic.id.clone()),
    ];
    let visible = lookup.visible(&scene, &targets);
    assert_eq!(
        visible.nodes.iter().map(|n| n.id()).collect::<Vec<_>>(),
        vec![id(2), id(21)]
    );
    assert_eq!(visible.ports.len(), 1);
    assert_eq!(visible.edges.len(), 1);
    assert_eq!(
        lookup.node(&scene, id(21)).unwrap().semantic.name,
        "Repository"
    );
    assert_eq!(lookup.port(&scene, id(21), id(2101)).unwrap().id, id(2101));
    assert!(lookup.port(&scene, id(22), id(2101)).is_none());
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
fn stale_scene_lookup_cannot_return_unrelated_records_or_cross_generations() {
    let scene = architecture();
    let lookup = SceneLookup::build(&scene);
    let mut reordered = scene.clone();
    reordered.nodes.reverse();
    assert!(lookup.node(&reordered, id(21)).is_none());
    let mut input = grid();
    input.nodes.truncate(1);
    input.edges.clear();
    let small = Scene::build(&input, &SceneOptions::default(), None).unwrap();
    assert!(lookup.node(&small, id(21)).is_none());
    let mut other = scene;
    other.generation = 9;
    assert!(lookup.node(&other, id(21)).is_none());
    assert!(
        lookup
            .visible(&other, &[SceneTarget::Node(id(21))])
            .nodes
            .is_empty()
    );
}
#[test]
fn comparison_preserves_container_envelope_around_removed_ghosts() {
    let (before, after) = grid_change();
    let parent = Scene::build(&before, &SceneOptions::default(), None).unwrap();
    let child = Scene::comparison(
        &before,
        &after,
        &BTreeSet::new(),
        &SceneOptions::default(),
        Some(parent.memory()),
    )
    .unwrap();
    let removed = child
        .nodes
        .iter()
        .find(|n| n.diff == DiffMark::Removed)
        .unwrap();
    let owner = child.node(removed.semantic.owner.unwrap()).unwrap();
    assert!(owner.bounds.contains_rect(removed.bounds));
    assert!(
        owner
            .bounds
            .contains_rect(parent.node(owner.id()).unwrap().bounds)
    );
}

#[test]
fn requirements_view_flattens_and_keeps_satisfy_edges() {
    let input = fixtures::architecture().requirements_view();
    assert!(
        input
            .nodes
            .iter()
            .all(|n| n.owner.is_none() && n.ports.is_empty())
    );
    assert!(input.edges.iter().any(|e| e.kind == EdgeKind::Satisfy));
    let scene = Scene::build(&input, &options(LayoutKind::Requirements), None).unwrap();
    assert!(scene.containers.is_empty());
    assert_eq!(scene.edges.len(), input.edges.len());
}

#[test]
fn removed_cards_never_overlap_current_cards() {
    for (before, after) in [grid_change(), fixtures::change()] {
        let first = Scene::build(&before, &SceneOptions::default(), None).unwrap();
        let scene = Scene::comparison(
            &before,
            &after,
            &BTreeSet::new(),
            &SceneOptions::default(),
            Some(first.memory()),
        )
        .unwrap();
        assert!(scene.nodes.iter().any(|n| n.diff == DiffMark::Removed));
        let owners = |id| {
            let mut out = Vec::new();
            let mut current = scene.node(id).and_then(|n| n.semantic.owner);
            while let Some(owner) = current {
                out.push(owner);
                current = scene.node(owner).and_then(|n| n.semantic.owner);
            }
            out
        };
        for ghost in scene.nodes.iter().filter(|n| n.diff == DiffMark::Removed) {
            for other in scene.nodes.iter().filter(|n| n.id() != ghost.id()) {
                if owners(ghost.id()).contains(&other.id())
                    || owners(other.id()).contains(&ghost.id())
                {
                    continue;
                }
                assert!(
                    !ghost.bounds.intersects(other.bounds),
                    "removed {} overlaps {}",
                    ghost.semantic.name,
                    other.semantic.name
                );
            }
        }
    }
}
