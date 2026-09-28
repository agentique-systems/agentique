//! The minimap (§3.2 Surface; D7): the whole model in the Surface's
//! bottom-left corner with the part in view outlined; a click or a drag there
//! moves the view. Shown only when the model does not fit in the view. It
//! draws at most the top two levels of cards, fewer at 10k, so it stays cheap.
use crate::{
    studio::{Dirty, Studio},
    ui::{ActiveTheme, r},
    workspace::StudioExt,
};
use agq_studio_scene::Point;
use gpui::{
    App, BorderStyle, Bounds, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, canvas, div, fill, point, px, quad,
};

const WIDTH: f32 = 184.0;
const HEIGHT: f32 = 120.0;
const MOST: usize = 600;

#[derive(IntoElement)]
pub struct Minimap {
    studio: Entity<Studio>,
}

impl Minimap {
    pub fn new(studio: Entity<Studio>) -> Minimap {
        Minimap { studio }
    }
}

/// Where a point of the minimap is in the model.
fn to_world(studio: &Studio, local: gpui::Point<gpui::Pixels>) -> Point {
    let bounds = studio.scene.bounds();
    let scale = (WIDTH / bounds.width().max(1.0)).min(HEIGHT / bounds.height().max(1.0));
    Point::new(
        bounds.min.x + f32::from(local.x) / scale,
        bounds.min.y + f32::from(local.y) / scale,
    )
}

impl RenderOnce for Minimap {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let bounds = studio.scene.bounds();
        let view = studio.camera.visible_rect();
        let narrow = f32::from(window.viewport_size().width) < WIDTH + 560.0;
        if studio.scene.nodes.is_empty() || view.contains_rect(bounds) || !bounds.finite() || narrow {
            return div().into_any_element();
        }
        let scene = studio.scene.clone();
        let origin = std::rc::Rc::new(std::cell::Cell::new(Bounds::default()));
        let origin_paint = origin.clone();
        let move_view = {
            let studio = self.studio.clone();
            let origin = origin.clone();
            move |position: gpui::Point<gpui::Pixels>, cx: &mut App| {
                let local = position - origin.get().origin;
                studio.act(cx, |studio| {
                    studio.camera.center = to_world(studio, local);
                    studio.camera_target = None;
                    studio.camera_move = None;
                    studio.mark(Dirty::CAMERA);
                });
            }
        };
        let on_down = move_view.clone();
        div()
            .id("minimap")
            .absolute()
            .left(r(12.0))
            .bottom(r(12.0))
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .rounded(r(crate::tokens::radius::MENU))
            .bg(theme.overlay.opacity(0.92))
            .border_1()
            .border_color(theme.border)
            .shadow(theme.shadow_small())
            .overflow_hidden()
            .occlude()
            .cursor_pointer()
            .role(gpui::Role::Figure)
            .aria_label("The whole model; click or drag to move the view")
            .tooltip(move |window, cx| {
                crate::ui::tooltip::text("The whole model; click or drag to move the view", None)(window, cx)
            })
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                cx.stop_propagation();
                on_down(event.position, cx);
            })
            .on_mouse_move(move |event, _, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    cx.stop_propagation();
                    move_view(event.position, cx);
                }
            })
            .child(
                canvas(
                    move |bounds, _, _| origin_paint.set(bounds),
                    move |area, _, window, _| {
                        let scale = (WIDTH / bounds.width().max(1.0)).min(HEIGHT / bounds.height().max(1.0));
                        let to_map = |x: f32, y: f32| {
                            point(
                                area.origin.x + px((x - bounds.min.x) * scale),
                                area.origin.y + px((y - bounds.min.y) * scale),
                            )
                        };
                        let mut shown: Vec<_> = scene.nodes.iter().filter(|node| node.depth <= 1).collect();
                        if shown.len() > MOST {
                            shown.retain(|node| node.depth == 0 || node.is_container);
                        }
                        window.paint_layer(area, |window| {
                            for node in shown.into_iter().take(MOST) {
                                let r = node.bounds;
                                window.paint_quad(fill(
                                    Bounds::from_corners(to_map(r.min.x, r.min.y), to_map(r.max.x, r.max.y)),
                                    if node.is_container {
                                        theme.border.opacity(0.35)
                                    } else {
                                        theme.text_faint.opacity(0.45)
                                    },
                                ));
                            }
                        });
                        let seen = Bounds::from_corners(to_map(view.min.x, view.min.y), to_map(view.max.x, view.max.y))
                            .intersect(&area);
                        window.paint_quad(quad(
                            seen,
                            px(2.0),
                            theme.accent.solid.opacity(0.08),
                            px(1.5),
                            theme.accent.solid,
                            BorderStyle::Solid,
                        ));
                    },
                )
                .size_full(),
            )
            .into_any_element()
    }
}
