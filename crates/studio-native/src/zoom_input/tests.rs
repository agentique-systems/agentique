use super::*;
use agq_studio_scene::{SceneOptions, SemanticScene, Size, SpatialIndex, fixtures};
use egui::{Modifiers, Rect, Sense, Vec2};

struct Harness {
    ctx: egui::Context,
    camera: Camera2D,
    viewport: Rect,
    time: f64,
    legacy: bool,
}
impl Harness {
    fn new(legacy: bool) -> Self {
        Self {
            ctx: egui::Context::default(),
            camera: Camera2D::default(),
            viewport: Rect::NOTHING,
            time: 0.0,
            legacy,
        }
    }
    fn pass(&mut self, events: Vec<Event>, dt: f64, dpi: f32) {
        self.time += dt;
        let mut input = egui::RawInput {
            events,
            time: Some(self.time),
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1600.0 / dpi, 1000.0 / dpi),
            )),
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(dpi);
        let ctx = self.ctx.clone();
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
                self.viewport = rect;
                self.camera.viewport = Size::new(rect.width(), rect.height());
                if self.legacy {
                    if response.hovered()
                        && let Some(pointer) = response.hover_pos()
                    {
                        let wheel = ui.input(|input| input.smooth_scroll_delta.y);
                        if wheel.abs() > 0.01 {
                            self.camera.zoom_at(
                                Point::new(pointer.x - rect.left(), pointer.y - rect.top()),
                                (wheel * 0.0025).exp(),
                            );
                        }
                    }
                } else {
                    apply(ui, &response, &mut self.camera);
                }
            });
        });
    }
    fn world_at(&self, pointer: Pos2) -> Point {
        self.camera.screen_to_world(Point::new(
            pointer.x - self.viewport.left(),
            pointer.y - self.viewport.top(),
        ))
    }
}
fn wheel(delta: f32) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, delta),
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn reproduces_legacy_zoom_anchor_loss_when_pointer_moves_during_scroll_tail() {
    let mut errors = Vec::new();
    for legacy in [true, false] {
        let mut harness = Harness::new(legacy);
        for _ in 0..3 {
            harness.pass(vec![], 1.0 / 165.0, 1.0);
        }
        let pointer = harness.viewport.min + harness.viewport.size() * 0.7;
        harness.camera.zoom = 0.05;
        let before = harness.world_at(pointer);
        for _ in 0..30 {
            harness.pass(
                vec![Event::PointerMoved(pointer), wheel(8.0)],
                1.0 / 165.0,
                1.0,
            );
        }
        for _ in 0..30 {
            harness.pass(
                vec![Event::PointerMoved(pointer + Vec2::new(-600.0, -350.0))],
                1.0 / 165.0,
                1.0,
            );
        }
        errors.push(before.distance(harness.world_at(pointer)));
    }
    println!(
        "Legacy anchor error = {}; event-owned anchor error = {} world units",
        errors[0], errors[1]
    );
    assert!(
        errors[0] > 100.0,
        "the former production path must actually reproduce the defect"
    );
    assert!(
        errors[1] < 0.025,
        "the unchanged event sequence must preserve its wheel anchor"
    );
}

#[test]
fn ordered_wheel_anchor_survives_12000_native_input_cycles() {
    let mut harness = Harness::new(false);
    let mut maximum = 0.0_f32;
    for cycle in 0..12_000 {
        let dpi = [1.0, 1.25, 1.5, 2.0, 2.5][cycle % 5];
        let dt = [1.0 / 165.0, 1.0 / 30.0, 0.25, 1.0 / 60.0, 1.0 / 240.0][cycle % 5];
        // Settle geometry before taking the anchor; a DPI resize deliberately
        // changes the viewport's visible extent, but never the semantic point.
        harness.pass(vec![], dt, dpi);
        harness.camera.zoom = [0.025, 0.04, 0.44, 1.0, 7.99][cycle % 5];
        harness.camera.center =
            Point::new((cycle % 173) as f32 * 130.0, (cycle % 89) as f32 * 110.0);
        let fraction = Vec2::new(
            0.15 + (cycle % 7) as f32 * 0.1,
            0.2 + (cycle % 5) as f32 * 0.1,
        );
        let pointer = harness.viewport.min + harness.viewport.size() * fraction;
        let before = harness.world_at(pointer);
        // Pointer moves after wheel in the same batch, then throughout settling.
        harness.pass(
            vec![
                Event::PointerMoved(pointer),
                wheel(if cycle % 2 == 0 { 8.0 } else { -20.0 }),
                Event::PointerMoved(harness.viewport.center()),
            ],
            dt,
            dpi,
        );
        harness.pass(
            vec![Event::PointerMoved(
                harness.viewport.min + Vec2::splat(16.0),
            )],
            dt,
            dpi,
        );
        let error = before.distance(harness.world_at(pointer));
        maximum = maximum.max(error);
        assert!(
            error <= 0.025,
            "cycle {cycle}, DPI {dpi}, dt {dt}, error {error}"
        );
    }
    println!(
        "12000 event-order/DPI/cadence/zoom cycles; maximum anchor error {maximum} world units"
    );
}

#[test]
fn mixed_dpi_scene_hit_ports_and_popup_anchor_use_logical_coordinates() {
    let projection = fixtures::architecture();
    let scene =
        SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
    let index = SpatialIndex::build(&scene);
    let node = scene.nodes.iter().find(|node| !node.is_container).unwrap();
    let mut camera = Camera2D::default();
    camera.fit(scene.bounds(), 42.0);
    let saved = serde_json::to_string(&camera).unwrap();
    for dpi in [1.0, 1.25, 1.5, 2.0, 2.5, 1.0] {
        camera.viewport = Size::new(1800.0 / dpi, 1200.0 / dpi);
        let logical = camera.world_to_screen(node.bounds.center());
        let physical = Point::new(logical.x * dpi, logical.y * dpi);
        let popup = Point::new(physical.x / dpi, physical.y / dpi);
        let world = camera.screen_to_world(popup);
        assert!(world.distance(node.bounds.center()) < 0.01);
        assert_eq!(
            index
                .hit_test(world, 6.0 / camera.zoom)
                .unwrap()
                .element_id(),
            Some(node.id())
        );
        for port in &scene.ports {
            let logical = camera.world_to_screen(port.position);
            assert!(camera.screen_to_world(logical).distance(port.position) < 0.01);
        }
        let restored: Camera2D = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.center, camera.center);
        assert_eq!(restored.zoom, camera.zoom);
    }
}
