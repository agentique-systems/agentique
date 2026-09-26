//! CPU-only paired visibility characterization. No semantic or GPU latency claim.
use agq_studio_scene::{
    Rect, SceneLookup, SceneOptions, SemanticScene, SpatialIndex, VisibleScene, fixtures,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Instant,
};

fn summary(values: &mut [f64]) -> serde_json::Value {
    values.sort_by(f64::total_cmp);
    serde_json::json!({
        "samples": values.len(), "median_us": values[values.len()/2],
        "p95_us": values[((values.len() as f64 * 0.95).ceil() as usize - 1).min(values.len()-1)],
    })
}

#[derive(Default)]
struct Samples {
    total: Vec<f64>,
    cull_resolve: Vec<f64>,
    hash: Vec<f64>,
    visible_objects_total: usize,
}
impl Samples {
    fn record(&mut self, visible: &VisibleScene<'_>, times: [f64; 3]) {
        self.total.push(times[0]);
        self.cull_resolve.push(times[1]);
        self.hash.push(times[2]);
        self.visible_objects_total +=
            visible.nodes.len() + visible.ports.len() + visible.edges.len();
    }
    fn report(&mut self) -> serde_json::Value {
        serde_json::json!({
            "total":summary(&mut self.total), "cull_resolve":summary(&mut self.cull_resolve),
            "identity_hash":summary(&mut self.hash), "visible_objects_total":self.visible_objects_total,
        })
    }
}

fn measure<'scene>(
    scene: &'scene SemanticScene,
    spatial: &SpatialIndex,
    lookup: &SceneLookup,
    bounds: Rect,
    borrowed: bool,
) -> (VisibleScene<'scene>, [f64; 3]) {
    let started = Instant::now();
    let targets;
    let visible = if borrowed {
        targets = Vec::new();
        spatial.visible_scene(scene, bounds)
    } else {
        targets = spatial.visible(bounds);
        lookup.visible(scene, &targets)
    };
    let after_resolve = Instant::now();
    let mut hasher = DefaultHasher::new();
    if borrowed {
        visible.hash(&mut hasher);
    } else {
        targets.hash(&mut hasher);
    }
    std::hint::black_box(hasher.finish());
    let after_hash = Instant::now();
    let times = [
        (after_hash - started).as_secs_f64() * 1e6,
        (after_resolve - started).as_secs_f64() * 1e6,
        (after_hash - after_resolve).as_secs_f64() * 1e6,
    ];
    (visible, times)
}

fn main() {
    for count in [1_000, 10_000] {
        let projection = fixtures::stress(count, count * 2);
        let scene = SemanticScene::from_projection(
            &projection,
            &SceneOptions {
                hierarchy: false,
                ..Default::default()
            },
            None,
        )
        .unwrap();
        let started = Instant::now();
        let spatial = SpatialIndex::build(&scene);
        let spatial_build_ms = started.elapsed().as_secs_f64() * 1000.0;
        let lookup = SceneLookup::build(&scene);
        for (trace, fraction) in [("fit", 1.0), ("pan", 0.85), ("zoom", 0.3)] {
            let bounds = scene.bounds();
            let width = bounds.width() * fraction;
            let height = bounds.height() * fraction;
            let rectangles: Vec<_> = (0..120)
                .map(|i| {
                    let position = i as f32 / 119.0;
                    Rect::new(
                        bounds.min.x + (bounds.width() - width) * position,
                        bounds.min.y + (bounds.height() - height) * (1.0 - position),
                        width,
                        height,
                    )
                })
                .collect();
            for &rect in rectangles.iter().take(12) {
                for borrowed in [false, true] {
                    std::hint::black_box(measure(&scene, &spatial, &lookup, rect, borrowed));
                }
            }
            let mut owned = Samples::default();
            let mut borrowed = Samples::default();
            for (i, &rect) in rectangles.iter().enumerate() {
                // Alternate first execution to reduce cache/order bias. Equality
                // assertions are outside both measurement intervals.
                let (old, new) = if i % 2 == 0 {
                    let old = measure(&scene, &spatial, &lookup, rect, false);
                    (old, measure(&scene, &spatial, &lookup, rect, true))
                } else {
                    let new = measure(&scene, &spatial, &lookup, rect, true);
                    (measure(&scene, &spatial, &lookup, rect, false), new)
                };
                assert!(
                    old.0
                        .nodes
                        .iter()
                        .map(|n| n.id())
                        .eq(new.0.nodes.iter().map(|n| n.id()))
                );
                assert!(
                    old.0
                        .ports
                        .iter()
                        .map(|p| p.id)
                        .eq(new.0.ports.iter().map(|p| p.id))
                );
                assert!(
                    old.0.edges.iter().map(|e| &e.semantic.id).eq(new
                        .0
                        .edges
                        .iter()
                        .map(|e| &e.semantic.id))
                );
                owned.record(&old.0, old.1);
                borrowed.record(&new.0, new.1);
                std::hint::black_box((old.0, new.0));
            }
            for (implementation, samples) in [
                ("owned_targets", &mut owned),
                ("checked_borrowed", &mut borrowed),
            ] {
                println!(
                    "{}",
                    serde_json::json!({
                        "format":"agentique-visibility-microbenchmark/2", "fixture":true,
                        "nodes":count,"edges":scene.edges.len(),"trace":trace,"viewport_fraction":fraction,
                        "implementation":implementation,"spatial_build_ms":spatial_build_ms,
                        "measurements":samples.report(),
                        "scope":"CPU spatial culling + checked borrowed lookup + presentation identity hashing; excludes final vector destruction, GPU/frame/semantic timing",
                    })
                );
            }
        }
    }
}
