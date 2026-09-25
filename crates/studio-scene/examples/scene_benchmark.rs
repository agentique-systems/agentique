//! CPU scene fixture benchmark. GPU metrics belong to the native renderer.
use agq_studio_scene::*;
use std::{collections::BTreeSet, time::Instant};
fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
fn main() {
    for count in [1_000, 10_000] {
        let start = Instant::now();
        let projection = fixtures::stress(count, count * 2);
        let fixture_ms = elapsed_ms(start);
        let options = SceneOptions {
            hierarchy: false,
            ..Default::default()
        };
        let start = Instant::now();
        let input = LayoutInput::from_projection(&projection, &options).unwrap();
        let input_ms = elapsed_ms(start);
        let start = Instant::now();
        let layout = GraphLayout::default().layout(&input, None);
        let layout_ms = elapsed_ms(start);
        std::hint::black_box(layout);
        let start = Instant::now();
        let scene = SemanticScene::from_projection(&projection, &options, None).unwrap();
        let scene_ms = elapsed_ms(start);
        let start = Instant::now();
        let index = SpatialIndex::build(&scene);
        let index_ms = elapsed_ms(start);
        let start = Instant::now();
        let lookup = SceneLookup::build(&scene);
        let lookup_ms = elapsed_ms(start);
        let start = Instant::now();
        let mut hits = 0;
        for i in 0..10000 {
            let n = &scene.nodes[(i * 97) % count];
            if index.hit_test(n.bounds.center(), 5.0).is_some() {
                hits += 1;
            }
        }
        let hit_us = elapsed_ms(start) * 1000.0 / 10000.0;
        let start = Instant::now();
        let mut visible = 0;
        for i in 0..100 {
            let p = scene.nodes[(i * 97) % count].bounds.min;
            visible += index.visible(Rect::new(p.x, p.y, 1200.0, 800.0)).len();
        }
        let cull_us = elapsed_ms(start) * 1000.0 / 100.0;
        let p = scene.nodes[count / 2].bounds.min;
        let visible_targets: BTreeSet<_> = index
            .visible(Rect::new(p.x, p.y, 1200.0, 800.0))
            .into_iter()
            .collect();
        let start = Instant::now();
        for _ in 0..1000 {
            std::hint::black_box(lookup.visible(&scene, &visible_targets));
        }
        let visible_lookup_us = elapsed_ms(start);
        // Reference reproduces the previous UI's full-scene visibility filter,
        // including semantic identity construction. It is never used by lookup.
        let start = Instant::now();
        let mut scanned = 0;
        for _ in 0..100 {
            for n in &scene.nodes {
                if visible_targets.contains(&SceneTarget::Node(n.id())) {
                    scanned += 1;
                }
            }
            for p in &scene.ports {
                if visible_targets.contains(&SceneTarget::Port(p.id)) {
                    scanned += 1;
                }
            }
            for e in &scene.edges {
                if visible_targets.contains(&SceneTarget::Edge(e.semantic.id.clone())) {
                    scanned += 1;
                }
            }
        }
        let full_scan_us = elapsed_ms(start) * 10.0;
        std::hint::black_box(scanned);
        let start = Instant::now();
        let mut camera = Camera2D::default();
        for _ in 0..10000 {
            camera.pan_screen(Point::new(1.0, -1.0));
            camera.zoom_at(Point::new(240.0, 170.0), 1.00001);
            std::hint::black_box(camera);
        }
        let camera_us = elapsed_ms(start) * 1000.0 / 10000.0;
        println!(
            "{}",
            serde_json::json!({"fixture":true,"nodes":count,"edges":scene.edges.len(),"fixture_build_ms":fixture_ms,"projection_adapter_ms":input_ms,"layout_ms":layout_ms,"scene_build_including_layout_and_routing_ms":scene_ms,"spatial_index_ms":index_ms,"scene_lookup_build_ms":lookup_ms,"visible_lookup_mean_us":visible_lookup_us,"full_scene_scan_reference_mean_us":full_scan_us,"lookup_visible_targets":visible_targets.len(),"hit_test_mean_us":hit_us,"hit_test_successes":hits,"culling_mean_us":cull_us,"culling_total_targets":visible,"pan_zoom_math_mean_us":camera_us,"obstructed_routes":scene.edges.iter().filter(|e|e.quality==RouteQuality::Obstructed).count(),"gpu_upload_ms":null,"render_fps":null})
        );
    }
}
