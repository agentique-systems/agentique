//! Measures layout correctness and how far unchanged cards move after a local
//! edit, on the fixtures.
use agq_studio_scene::*;
use std::time::Instant;

fn main() {
    let inputs = [
        ("url-shortener", fixtures::architecture()),
        ("requirements", fixtures::architecture().requirements_view()),
        ("dense-ports", fixtures::dense_ports()),
        ("typography", fixtures::typography()),
        ("stress-200", fixtures::stress(200, 400)),
    ];
    for (name, input) in inputs {
        for layout in [LayoutKind::Hierarchy, LayoutKind::Graph] {
            let hierarchy = layout == LayoutKind::Hierarchy;
            let options = SceneOptions {
                layout,
                ..Default::default()
            };
            let start = Instant::now();
            let before = Scene::build(&input, &options, None).unwrap();
            let scene_ms = start.elapsed().as_secs_f64() * 1000.0;
            let mut edited = input.clone();
            let mut added = edited.nodes.last().unwrap().clone();
            added.id = fixtures::id(9_000_000);
            added.name = "LocalAddedPart".into();
            added.ports.clear();
            edited.nodes.push(added);
            let start = Instant::now();
            let after = Scene::build(&edited, &options, Some(before.memory())).unwrap();
            let edit_scene_ms = start.elapsed().as_secs_f64() * 1000.0;
            let mut movement: Vec<_> = before
                .nodes
                .iter()
                .map(|old| {
                    old.bounds
                        .min
                        .distance(after.node(old.id()).unwrap().bounds.min)
                })
                .collect();
            movement.sort_by(f32::total_cmp);
            let overlaps = after
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| {
                    after.nodes[index + 1..]
                        .iter()
                        .filter(|other| {
                            (!hierarchy || node.semantic.owner == other.semantic.owner)
                                && node.bounds.intersects(other.bounds)
                        })
                        .count()
                })
                .sum::<usize>();
            let containment_failures = after
                .nodes
                .iter()
                .filter(|node| {
                    hierarchy
                        && node
                            .semantic
                            .owner
                            .and_then(|owner| after.node(owner))
                            .is_some_and(|owner| !owner.bounds.contains_rect(node.bounds))
                })
                .count();
            println!(
                "{}",
                serde_json::json!({
                    "fixture": name,
                    "layout": if hierarchy { "hierarchy" } else { "graph" },
                    "nodes": after.nodes.len(), "ports": after.ports.len(), "edges": after.edges.len(),
                    "scene_build_ms": scene_ms, "edit_scene_build_ms": edit_scene_ms,
                    "unchanged_node_top_left_displacement_world_units": {
                        "median": movement[movement.len()/2], "p95": movement[(movement.len() * 95 / 100).min(movement.len()-1)]
                    },
                    "sibling_overlaps": overlaps, "containment_failures": containment_failures,
                    "obstructed_routes": after.edges.iter().filter(|edge| edge.quality == RouteQuality::Obstructed).count()
                })
            );
            assert_eq!(overlaps, 0, "{name} hierarchy={hierarchy}");
            assert_eq!(containment_failures, 0, "{name} hierarchy={hierarchy}");
        }
    }
}
