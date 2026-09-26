//! Measures geometry correctness and mental-map displacement on adversarial fixtures.
use agq_studio_scene::*;
use std::time::Instant;

fn main() {
    for (name, projection) in fixtures::adversarial() {
        for hierarchy in [true, false] {
            let options = SceneOptions {
                hierarchy,
                ..Default::default()
            };
            let start = Instant::now();
            let before = SemanticScene::from_projection(&projection, &options, None).unwrap();
            let scene_ms = start.elapsed().as_secs_f64() * 1000.0;
            let mut edited = projection.clone();
            let mut added = edited.nodes.last().unwrap().clone();
            added.id = fixtures::id(9_000_000);
            added.name = "LocalAddedPart".into();
            added.features.clear();
            added.counts = Default::default();
            edited.nodes.push(added);
            let start = Instant::now();
            let after =
                SemanticScene::from_projection(&edited, &options, Some(before.memory())).unwrap();
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
                    "fixture": name, "semantic_acceptance": false,
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
