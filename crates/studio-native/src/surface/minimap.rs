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

const WIDTH: f32 = 176.0;
const HEIGHT: f32 = 112.0;
/// Room between the model and the minimap's edge.
const INSET: f32 = 8.0;
const MOST: usize = 600;

/// How the model maps into the minimap: scaled to fit and centred, in a
/// panel that takes the model's shape within the largest size.
#[derive(Clone, Copy)]
struct Map {
    scale: f32,
    offset: (f32, f32),
    min: Point,
    size: (f32, f32),
}

impl Map {
    fn new(bounds: agq_studio_scene::Rect) -> Map {
        let (w, h) = (WIDTH - 2.0 * INSET, HEIGHT - 2.0 * INSET);
        let scale = (w / bounds.width().max(1.0)).min(h / bounds.height().max(1.0));
        let size = (
            (bounds.width() * scale + 2.0 * INSET).clamp(72.0, WIDTH),
            (bounds.height() * scale + 2.0 * INSET).clamp(56.0, HEIGHT),
        );
        Map {
            scale,
            offset: (
                (size.0 - bounds.width() * scale) * 0.5,
                (size.1 - bounds.height() * scale) * 0.5,
            ),
            min: bounds.min,
            size,
        }
    }
    fn to_map(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.offset.0 + (x - self.min.x) * self.scale,
            self.offset.1 + (y - self.min.y) * self.scale,
        )
    }
    fn to_world(self, x: f32, y: f32) -> Point {
        Point::new(
            self.min.x + (x - self.offset.0) / self.scale,
            self.min.y + (y - self.offset.1) / self.scale,
        )
    }
}

#[derive(IntoElement)]
pub struct Minimap {
    studio: Entity<Studio>,
}

impl Minimap {
    pub fn new(studio: Entity<Studio>) -> Minimap {
        Minimap { studio }
    }
}

impl RenderOnce for Minimap {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let bounds = studio.scene.bounds();
        let view = studio.camera.visible_rect();
        let narrow = f32::from(window.viewport_size().width) < WIDTH + 560.0;
        if studio.scene.nodes.is_empty() || view.contains_rect(bounds) || !bounds.finite() || narrow
        {
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
                    let map = Map::new(studio.scene.bounds());
                    studio.camera.center = map.to_world(f32::from(local.x), f32::from(local.y));
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
            .w(px(Map::new(bounds).size.0))
            .h(px(Map::new(bounds).size.1))
            .rounded(r(crate::tokens::radius::MENU))
            .bg(theme.overlay.opacity(0.94))
            .border_1()
            .border_color(theme.border)
            .shadow(theme.shadow_overlay())
            .overflow_hidden()
            .occlude()
            .cursor_pointer()
            .role(gpui::Role::Figure)
            .aria_label("The whole model; click or drag to move the view")
            .tooltip(move |window, cx| {
                crate::ui::tooltip::text("The whole model; click or drag to move the view", None)(
                    window, cx,
                )
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
                        let map = Map::new(bounds);
                        let at = |x: f32, y: f32| {
                            let (mx, my) = map.to_map(x, y);
                            point(area.origin.x + px(mx), area.origin.y + px(my))
                        };
                        let rect = |r: agq_studio_scene::Rect| {
                            Bounds::from_corners(at(r.min.x, r.min.y), at(r.max.x, r.max.y))
                        };
                        let mut shown: Vec<_> =
                            scene.nodes.iter().filter(|node| node.depth <= 1).collect();
                        if shown.len() > MOST {
                            shown.retain(|node| node.depth == 0 || node.is_container);
                        }
                        window.paint_layer(area, |window| {
                            for node in shown.into_iter().take(MOST) {
                                let r = rect(node.bounds);
                                if node.is_container {
                                    window.paint_quad(quad(
                                        r,
                                        px(2.0),
                                        theme.text_faint.opacity(0.06),
                                        px(1.0),
                                        theme.border_strong.opacity(0.7),
                                        BorderStyle::Solid,
                                    ));
                                } else {
                                    window.paint_quad(quad(
                                        r,
                                        px(1.5),
                                        super::paint::category_colour(node.category, &theme)
                                            .opacity(0.28),
                                        px(0.0),
                                        gpui::transparent_black(),
                                        BorderStyle::Solid,
                                    ));
                                }
                            }
                        });
                        // Outside the view is dimmed; the view is a clear
                        // window with an accent edge.
                        let seen = rect(view).intersect(&area);
                        let dim = theme.canvas.opacity(0.55);
                        let (l, t) = (seen.origin.x, seen.origin.y);
                        let (r, b) = (
                            seen.origin.x + seen.size.width,
                            seen.origin.y + seen.size.height,
                        );
                        let (al, at_, ar, ab) = (
                            area.origin.x,
                            area.origin.y,
                            area.origin.x + area.size.width,
                            area.origin.y + area.size.height,
                        );
                        for piece in [
                            Bounds::from_corners(point(al, at_), point(ar, t)),
                            Bounds::from_corners(point(al, b), point(ar, ab)),
                            Bounds::from_corners(point(al, t), point(l, b)),
                            Bounds::from_corners(point(r, t), point(ar, b)),
                        ] {
                            if piece.size.width > px(0.0) && piece.size.height > px(0.0) {
                                window.paint_quad(fill(piece, dim));
                            }
                        }
                        window.paint_quad(quad(
                            seen,
                            px(3.0),
                            theme.accent.solid.opacity(0.06),
                            px(1.5),
                            theme.accent.solid.opacity(0.9),
                            BorderStyle::Solid,
                        ));
                    },
                )
                .size_full(),
            )
            .into_any_element()
    }
}
