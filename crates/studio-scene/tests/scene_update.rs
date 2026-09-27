//! `Scene::update` (S5.1, W5.5, R-28): an edit re-routes only the edges it
//! touches, places cards exactly as a full build does, and leaves unrelated
//! cards where they were.
mod common;
use agq_studio_scene::fixtures::{at, card, edge, id, node, port};
use agq_studio_scene::*;
use common::{grid, grid_change, options};
use std::collections::BTreeSet;

/// One edit of each kind to `grid()`, named, with the cards it is about.
fn edits() -> Vec<(&'static str, SceneInput, BTreeSet<ElementId>)> {
    let before = grid();
    let mut renamed = before.clone();
    renamed.nodes[7].name = "RenamedService".into();
    let mut added = before.clone();
    let mut new = node(24, "Coordinator", NodeCategory::Part, Some(2));
    new.ports = vec![port(2401, "request"), port(2402, "result")];
    added.nodes.push(new);
    added
        .edges
        .push(edge("c9", EdgeKind::Connection, at(24, 2402), at(33, 3301)));
    let mut removed = before.clone();
    removed.nodes.retain(|n| n.id != id(13));
    removed
        .edges
        .retain(|e| e.source.node != id(13) && e.target.node != id(13));
    let mut connected = before.clone();
    connected
        .edges
        .push(edge("c8", EdgeKind::Connection, at(11, 1102), at(33, 3301)));
    connected
        .edges
        .push(edge("t14", EdgeKind::Typing, card(13), card(31)));
    let mut ported = before.clone();
    ported.nodes[3]
        .ports
        .extend([port(1103, "extra"), port(1104, "more"), port(1105, "most")]);
    vec![
        ("rename", renamed, BTreeSet::from([id(22)])),
        ("new card", added, BTreeSet::from([id(24)])),
        ("removed card", removed, BTreeSet::from([id(13)])),
        ("new edges", connected, BTreeSet::new()),
        ("new ports", ported, BTreeSet::from([id(11)])),
        (
            "several",
            grid_change().1,
            BTreeSet::from([id(13), id(22), id(24)]),
        ),
    ]
}

fn cards(scene: &Scene) -> String {
    format!(
        "{:?}\n{:?}\n{:?}",
        scene.nodes, scene.ports, scene.containers
    )
}

#[test]
fn a_build_routes_every_edge() {
    let scene = Scene::build(&grid(), &SceneOptions::default(), None).unwrap();
    assert_eq!(
        scene.routing(),
        Routing {
            routed: scene.edges.len(),
            kept: 0
        }
    );
}

#[test]
fn an_update_places_cards_as_a_build_does_and_routes_as_a_build_does() {
    for layout in [LayoutKind::Hierarchy, LayoutKind::Graph] {
        let options = options(layout);
        let earlier = Scene::build(&grid(), &options, None).unwrap();
        for (name, mut after, _) in edits() {
            after.generation = 2;
            let full = Scene::build(&after, &options, Some(earlier.memory())).unwrap();
            let updated = earlier
                .update(&after, &options, Some(earlier.memory()))
                .unwrap();
            assert_eq!(cards(&updated), cards(&full), "{layout:?} {name}: cards");
            assert_eq!(
                updated.memory().bounds,
                full.memory().bounds,
                "{layout:?} {name}: memory"
            );
            assert_eq!(updated.generation, 2);
            assert_eq!(
                format!("{:?}", updated.edges),
                format!("{:?}", full.edges),
                "{layout:?} {name}: edges"
            );
            assert_eq!(updated.warnings, full.warnings);
            let routing = updated.routing();
            assert_eq!(routing.routed + routing.kept, updated.edges.len());
        }
    }
}

#[test]
fn an_update_of_the_url_shortener_matches_a_build() {
    let (before, after) = fixtures::change();
    for layout in [
        LayoutKind::Hierarchy,
        LayoutKind::Graph,
        LayoutKind::Requirements,
    ] {
        let options = options(layout);
        let (before, after) = if layout == LayoutKind::Requirements {
            (before.requirements_view(), after.requirements_view())
        } else {
            (before.clone(), after.clone())
        };
        let earlier = Scene::build(&before, &options, None).unwrap();
        let full = Scene::build(&after, &options, Some(earlier.memory())).unwrap();
        let updated = earlier
            .update(&after, &options, Some(earlier.memory()))
            .unwrap();
        assert_eq!(cards(&updated), cards(&full), "{layout:?}");
        assert_eq!(updated.edges.len(), full.edges.len());
        // Every route is the build's, or its earlier route, kept because
        // the change does not cross it. A kept route can differ from the
        // build's where a moved card changed a detour the router only
        // considered: here the service grows by 24 units and moves the
        // definitions below it, and in the hierarchy three typing edges
        // keep their detour 24 units above the one a build would choose.
        let mut kept = 0;
        for (now, built) in updated.edges.iter().zip(&full.edges) {
            assert_eq!(now.semantic, built.semantic);
            if now.points != built.points {
                let then = earlier
                    .edges
                    .iter()
                    .find(|e| e.semantic.id == now.semantic.id)
                    .expect("a kept route was in the earlier scene");
                assert_eq!(now.points, then.points, "{layout:?}");
                assert_eq!(now.quality, RouteQuality::Clear, "{layout:?}");
                assert!(!crosses_a_card(&updated, now), "{layout:?}");
                kept += 1;
            }
        }
        assert!(kept <= 3, "{layout:?}: {kept} routes differ from a build");
    }
}

/// Whether a route runs through a card other than its ends, as the router
/// judges it: through the card's inside, 10 units around it.
fn crosses_a_card(scene: &Scene, edge: &SceneEdge) -> bool {
    let ends = [edge.semantic.source.node, edge.semantic.target.node];
    scene
        .nodes
        .iter()
        .filter(|n| !n.is_container && !ends.contains(&n.id()))
        .any(|n| {
            let r = n.bounds.inflate(10.0);
            edge.points.windows(2).any(|pair| {
                let (a, b) = (pair[0], pair[1]);
                if a.x == b.x {
                    a.x > r.min.x
                        && a.x < r.max.x
                        && a.y.max(b.y) > r.min.y
                        && a.y.min(b.y) < r.max.y
                } else {
                    a.y > r.min.y
                        && a.y < r.max.y
                        && a.x.max(b.x) > r.min.x
                        && a.x.min(b.x) < r.max.x
                }
            })
        })
}

#[test]
fn an_update_leaves_unrelated_cards_where_they_were() {
    for layout in [LayoutKind::Hierarchy, LayoutKind::Graph] {
        let options = options(layout);
        let before = grid();
        let earlier = Scene::build(&before, &options, None).unwrap();
        for (name, after, about) in edits() {
            let updated = earlier
                .update(&after, &options, Some(earlier.memory()))
                .unwrap();
            // Owners of an edited card may grow; they stay where they were.
            let owners: BTreeSet<_> = after
                .nodes
                .iter()
                .chain(&before.nodes)
                .filter(|n| about.contains(&n.id))
                .filter_map(|n| n.owner)
                .collect();
            for card in &earlier.nodes {
                let Some(now) = updated.node(card.id()) else {
                    continue;
                };
                if owners.contains(&card.id()) {
                    assert_eq!(now.bounds.min, card.bounds.min, "{layout:?} {name}");
                } else if !about.contains(&card.id()) {
                    assert_eq!(now.bounds, card.bounds, "{layout:?} {name}");
                }
            }
        }
    }
}

/// Cards 1 and 3 connected across the gap where card 2 is or is not, and
/// cards 4 and 5 connected far below, placed by layout memory.
fn gap(with_middle: bool) -> (SceneInput, LayoutMemory) {
    let mut input = common::flat(5, 0);
    input.edges = vec![
        edge("across", EdgeKind::Connection, card(1), card(3)),
        edge("below", EdgeKind::Connection, card(4), card(5)),
    ];
    if !with_middle {
        input.nodes.retain(|n| n.id != id(2));
    }
    let memory = LayoutMemory {
        bounds: [
            (1, 0.0, 0.0),
            (2, 300.0, 0.0),
            (3, 600.0, 0.0),
            (4, 0.0, 1000.0),
            (5, 600.0, 1000.0),
        ]
        .into_iter()
        .map(|(n, x, y)| (id(n), Rect::new(x, y, 232.0, 118.0)))
        .collect(),
        ..Default::default()
    };
    (input, memory)
}

#[test]
fn a_card_added_across_a_route_routes_it_again_and_other_routes_are_kept() {
    let options = options(LayoutKind::Graph);
    let (before, memory) = gap(false);
    let earlier = Scene::build(&before, &options, Some(&memory)).unwrap();
    let straight = earlier.edges[0].points.clone();
    assert!(straight.iter().all(|p| p.y > 0.0 && p.y < 118.0));
    let (after, _) = gap(true);
    let full = Scene::build(&after, &options, Some(&memory)).unwrap();
    let updated = earlier.update(&after, &options, Some(&memory)).unwrap();
    assert_eq!(updated.routing(), Routing { routed: 1, kept: 1 });
    assert_eq!(updated.edges[0].semantic.id, "across");
    assert_ne!(updated.edges[0].points, straight);
    assert_eq!(updated.edges[0].points, full.edges[0].points);
    assert_eq!(updated.edges[0].quality, RouteQuality::Clear);
    assert_eq!(updated.edges[1].points, earlier.edges[1].points);
}

/// The one deliberate difference from a build: routes stay put, like cards.
#[test]
fn a_route_around_a_removed_card_is_kept_until_the_next_build() {
    let options = options(LayoutKind::Graph);
    let (before, memory) = gap(true);
    let earlier = Scene::build(&before, &options, Some(&memory)).unwrap();
    let detour = earlier.edges[0].points.clone();
    assert!(detour.iter().any(|p| p.y < 0.0 || p.y > 118.0));
    let (after, _) = gap(false);
    let updated = earlier.update(&after, &options, Some(&memory)).unwrap();
    assert_eq!(updated.routing().routed, 0);
    assert_eq!(updated.edges[0].points, detour);
    let full = Scene::build(&after, &options, Some(&memory)).unwrap();
    assert_ne!(full.edges[0].points, detour);
}

#[test]
fn an_update_from_another_view_places_cards_as_a_build_does() {
    let hierarchy = Scene::build(&grid(), &options(LayoutKind::Hierarchy), None).unwrap();
    let graph = options(LayoutKind::Graph);
    let full = Scene::build(&grid(), &graph, None).unwrap();
    let updated = hierarchy.update(&grid(), &graph, None).unwrap();
    assert_eq!(cards(&updated), cards(&full));
    assert_eq!(format!("{:?}", updated.edges), format!("{:?}", full.edges));
}

#[test]
fn an_update_reports_invalid_input_as_a_build_does() {
    let earlier = Scene::build(&grid(), &SceneOptions::default(), None).unwrap();
    let mut twice = grid();
    twice.nodes.push(twice.nodes[0].clone());
    assert!(matches!(
        earlier.update(&twice, &SceneOptions::default(), Some(earlier.memory())),
        Err(SceneError::DuplicateElement(_))
    ));
}
