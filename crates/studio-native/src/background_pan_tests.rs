//! Actual egui passes reproduce the dismissed-window drag capture regression.
//! This tests native gesture mechanics, not semantic candidate responsiveness.
use super::{background_pan_start, click};
use agq_studio_scene::{Camera2D, Point, Size};
use eframe::egui::{self, Align2, Event, Pos2, Rect, Sense, Vec2};

struct GestureHarness {
    context: egui::Context,
    camera: Camera2D,
    passes: u64,
    dialog_open: bool,
    prepare_clicked: bool,
    viewport: Option<Rect>,
    prepare_button: Option<Rect>,
    last_dialog: Option<Rect>,
    dragged_passes: usize,
}

impl GestureHarness {
    fn new() -> Self {
        Self {
            context: egui::Context::default(),
            camera: Camera2D::default(),
            passes: 0,
            dialog_open: true,
            prepare_clicked: false,
            viewport: None,
            prepare_button: None,
            last_dialog: None,
            dragged_passes: 0,
        }
    }

    fn pass(&mut self, events: Vec<Event>) {
        self.passes += 1;
        let context = self.context.clone();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0))),
            time: Some(self.passes as f64 / 60.0),
            events,
            ..Default::default()
        };
        let _ = context.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
                self.viewport = Some(rect);
                self.camera.viewport = Size::new(rect.width(), rect.height());
                // The production viewport uses this exact ordinary response and
                // pointer delta. No test invokes camera movement directly.
                if response.dragged() {
                    self.dragged_passes += 1;
                    let delta = ui.input(|input| input.pointer.delta());
                    self.camera.pan_screen(Point::new(delta.x, delta.y));
                }
            });
            if self.dialog_open {
                let window = egui::Window::new("Create nested part")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .default_width(440.0)
                    .show(ctx, |ui| {
                        ui.label("Inside ModelingPlatform");
                        ui.add_space(70.0);
                        ui.label("A Working candidate will be reconstructed.");
                        ui.add_space(16.0);
                        let prepare = ui.button("Prepare candidate");
                        self.prepare_button = Some(prepare.rect);
                        if prepare.clicked() {
                            self.prepare_clicked = true;
                            // As in part_edit_dialog -> prepare_part, the
                            // Window was registered during this closing pass.
                            self.dialog_open = false;
                        }
                    })
                    .expect("the real egui Window is visible");
                self.last_dialog = Some(window.response.rect);
            }
        });
    }

    fn pointer_button(&mut self, position: Pos2, pressed: bool) {
        let mut input = egui::RawInput::default();
        click(&mut input, position, pressed);
        self.pass(input.events);
    }

    fn prepare_through_actual_button(&mut self) {
        // Complete the Window's initial sizing/animation before its real click.
        for _ in 0..12 {
            self.pass(vec![]);
        }
        let prepare = self
            .prepare_button
            .expect("rendered Prepare button")
            .center();
        self.pointer_button(prepare, true);
        self.pointer_button(prepare, false);
        assert!(
            self.prepare_clicked,
            "the actual button must receive the click"
        );
        assert!(!self.dialog_open);
        assert_eq!(self.dragged_passes, 0);
    }

    fn drag(&mut self, start: Pos2) {
        self.pointer_button(start, true);
        self.pass(vec![Event::PointerMoved(start + Vec2::new(35.0, 20.0))]);
        self.pass(vec![Event::PointerMoved(start + Vec2::new(70.0, 40.0))]);
        self.pointer_button(start + Vec2::new(70.0, 40.0), false);
        self.pass(vec![]);
    }
}

#[test]
fn immediately_dragging_dismissed_dialog_center_does_not_retarget_viewport() {
    let mut harness = GestureHarness::new();
    harness.prepare_through_actual_button();
    let start = harness.viewport.unwrap().center();
    assert!(harness.last_dialog.unwrap().contains(start));
    let before = harness.camera;

    harness.drag(start);

    assert_eq!(harness.dragged_passes, 0);
    assert_eq!(
        harness.camera, before,
        "moving after a press captured by the old Window must not be credited as viewport pan"
    );
}

#[test]
fn settled_corner_gesture_moves_camera_through_real_egui_drag_response() {
    let mut harness = GestureHarness::new();
    harness.prepare_through_actual_button();
    let start = background_pan_start(harness.viewport.unwrap()).unwrap();
    assert!(!harness.last_dialog.unwrap().contains(start));
    for _ in 0..2 {
        harness.pass(vec![Event::PointerMoved(start)]);
    }
    assert!(!harness.context.input(|input| input.pointer.any_down()));
    let before = harness.camera;

    harness.drag(start);

    assert!(harness.dragged_passes >= 2);
    let expected = Point::new(
        before.center.x - 70.0 / before.zoom,
        before.center.y - 40.0 / before.zoom,
    );
    assert!(harness.camera.center.distance(expected) < 0.001);
    assert!(harness.camera.center.distance(before.center) > 1.0);
    assert_eq!(harness.camera.zoom, before.zoom);
    assert!(!harness.context.input(|input| input.pointer.any_down()));
}

#[test]
fn background_pan_start_requires_the_entire_gesture_inside_the_viewport() {
    let minimum = Rect::from_min_size(Pos2::new(200.0, 150.0), Vec2::new(104.0, 88.0));
    let start = background_pan_start(minimum).unwrap();
    assert_eq!(start, minimum.left_top() + Vec2::splat(24.0));
    assert!(minimum.shrink(10.0).contains(start));
    assert!(minimum.shrink(10.0).contains(start + Vec2::new(70.0, 40.0)));
    for size in [Vec2::new(103.0, 88.0), Vec2::new(104.0, 87.0)] {
        assert!(background_pan_start(Rect::from_min_size(Pos2::ZERO, size)).is_err());
    }
}
