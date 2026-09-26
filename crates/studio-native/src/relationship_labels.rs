//! Screen-space annotation placement. Routes and semantic identity are untouched.
use eframe::egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

/// Screen-space cells bound the work of avoiding dense routes. This contains
/// drawing segments only, never semantic records or graph authority.
#[derive(Default)]
pub(crate) struct RouteObstacles {
    cells: HashMap<(i32, i32), Vec<[Pos2; 2]>>,
}
impl RouteObstacles {
    const CELL: f32 = 64.0;
    pub fn insert(&mut self, a: Pos2, b: Pos2, viewport: Rect) {
        let Some((a, b)) = clip(a, b, viewport) else {
            return;
        };
        let bounds = Rect::from_two_pos(a, b);
        for x in (bounds.left() / Self::CELL).floor() as i32
            ..=(bounds.right() / Self::CELL).floor() as i32
        {
            for y in (bounds.top() / Self::CELL).floor() as i32
                ..=(bounds.bottom() / Self::CELL).floor() as i32
            {
                self.cells.entry((x, y)).or_default().push([a, b]);
            }
        }
    }
    fn intersects(&self, bounds: Rect) -> bool {
        for x in (bounds.left() / Self::CELL).floor() as i32
            ..=(bounds.right() / Self::CELL).floor() as i32
        {
            for y in (bounds.top() / Self::CELL).floor() as i32
                ..=(bounds.bottom() / Self::CELL).floor() as i32
            {
                if self.cells.get(&(x, y)).is_some_and(|segments| {
                    segments
                        .iter()
                        .any(|segment| clip(segment[0], segment[1], bounds).is_some())
                }) {
                    return true;
                }
            }
        }
        false
    }
}

pub(crate) struct Placement {
    pub bounds: Rect,
    pub anchor: Pos2,
}

/// Prefer long visible route segments and keep automatic annotations clear of
/// cards and earlier, higher-priority labels. Explicit inspection can fall back
/// to a bounded callout; its exact edge is still available in the Inspector.
pub(crate) fn place(
    route: &[Pos2],
    size: Vec2,
    viewport: Rect,
    obstacles: &[Rect],
    previous: &[Rect],
    explicit: bool,
    routes: &RouteObstacles,
) -> Option<Placement> {
    let viewport = viewport.shrink(4.0);
    if size.x > viewport.width() || size.y > viewport.height() {
        return None;
    }
    let mut segments: Vec<_> = route
        .windows(2)
        .filter_map(|pair| clip(pair[0], pair[1], viewport))
        .filter(|(a, b)| a.distance_sq(*b) >= 4.0)
        .collect();
    // Stable sorting preserves route order when parallel segments have equal length.
    segments.sort_by(|(a, b), (c, d)| c.distance_sq(*d).total_cmp(&a.distance_sq(*b)));
    let mut fallback = None;
    for (a, b) in segments.into_iter().take(8) {
        let horizontal = (b.x - a.x).abs() >= (b.y - a.y).abs();
        let normal = if horizontal { Vec2::Y } else { Vec2::X };
        let offset = if horizontal { size.y } else { size.x } * 0.5 + 5.0;
        for fraction in [0.5, 0.3, 0.7, 0.15, 0.85] {
            let anchor = a + (b - a) * fraction;
            for side in [-1.0, 1.0] {
                let center = anchor + normal * offset * side;
                let bounds = Rect::from_center_size(center, size);
                fallback.get_or_insert_with(|| Placement {
                    bounds: Rect::from_center_size(
                        Pos2::new(
                            center.x.clamp(
                                viewport.left() + size.x * 0.5,
                                viewport.right() - size.x * 0.5,
                            ),
                            center.y.clamp(
                                viewport.top() + size.y * 0.5,
                                viewport.bottom() - size.y * 0.5,
                            ),
                        ),
                        size,
                    ),
                    anchor,
                });
                if viewport.contains_rect(bounds)
                    && !obstacles
                        .iter()
                        .chain(previous)
                        .any(|obstacle| obstacle.expand(3.0).intersects(bounds))
                    && !routes.intersects(bounds.expand(2.0))
                {
                    return Some(Placement { bounds, anchor });
                }
            }
        }
    }
    explicit.then_some(fallback).flatten()
}

fn clip(a: Pos2, b: Pos2, bounds: Rect) -> Option<(Pos2, Pos2)> {
    let delta = b - a;
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for (start, direction, min, max) in [
        (a.x, delta.x, bounds.left(), bounds.right()),
        (a.y, delta.y, bounds.top(), bounds.bottom()),
    ] {
        if direction.abs() < f32::EPSILON {
            if start < min || start > max {
                return None;
            }
        } else {
            let first = (min - start) / direction;
            let last = (max - start) / direction;
            enter = enter.max(first.min(last));
            leave = leave.min(first.max(last));
            if enter > leave {
                return None;
            }
        }
    }
    Some((a + delta * enter, a + delta * leave))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offscreen_long_route_uses_visible_segment_and_avoids_cards_and_prior_labels() {
        let viewport = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let route = [Pos2::new(-2000.0, 160.0), Pos2::new(300.0, 160.0)];
        let card = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(210.0, 155.0));
        let first = place(
            &route,
            Vec2::new(80.0, 24.0),
            viewport,
            &[card],
            &[],
            false,
            &RouteObstacles::default(),
        )
        .unwrap();
        assert!(viewport.contains(first.anchor));
        assert!(viewport.contains_rect(first.bounds));
        assert!(!card.expand(3.0).intersects(first.bounds));
        let second = place(
            &route,
            Vec2::new(80.0, 24.0),
            viewport,
            &[card],
            &[first.bounds],
            false,
            &RouteObstacles::default(),
        )
        .unwrap();
        assert!(!first.bounds.expand(3.0).intersects(second.bounds));
        assert!(!card.expand(3.0).intersects(second.bounds));
    }
    #[test]
    fn crowded_automatic_labels_yield_but_explicit_inspection_keeps_visible_callout() {
        let viewport = Rect::from_min_max(Pos2::ZERO, Pos2::new(300.0, 200.0));
        let route = [Pos2::new(-600.0, 100.0), Pos2::new(500.0, 100.0)];
        assert!(
            place(
                &route,
                Vec2::new(90.0, 24.0),
                viewport,
                &[viewport],
                &[],
                false,
                &RouteObstacles::default(),
            )
            .is_none()
        );
        let explicit = place(
            &route,
            Vec2::new(90.0, 24.0),
            viewport,
            &[viewport],
            &[],
            true,
            &RouteObstacles::default(),
        )
        .unwrap();
        assert!(viewport.contains_rect(explicit.bounds));
        assert!(viewport.contains(explicit.anchor));
        let outside = [Pos2::new(-200.0, -100.0), Pos2::new(500.0, -100.0)];
        assert!(
            place(
                &outside,
                Vec2::new(90.0, 24.0),
                viewport,
                &[],
                &[],
                true,
                &RouteObstacles::default()
            )
            .is_none()
        );
    }

    #[test]
    fn dense_parallel_and_crossing_routes_do_not_get_automatic_label_obscuration() {
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        let mut routes = RouteObstacles::default();
        for index in 0..20 {
            let y = 110.0 + index as f32 * 12.0;
            routes.insert(Pos2::new(20.0, y), Pos2::new(780.0, y), viewport);
            let x = 140.0 + index as f32 * 24.0;
            routes.insert(Pos2::new(x, 40.0), Pos2::new(x, 460.0), viewport);
        }
        let route = [Pos2::new(20.0, 180.0), Pos2::new(780.0, 180.0)];
        let automatic = place(
            &route,
            Vec2::new(120.0, 28.0),
            viewport,
            &[],
            &[],
            false,
            &routes,
        );
        assert!(
            automatic.is_none(),
            "dense context labels must yield instead of hiding paths"
        );
        let explicit = place(
            &route,
            Vec2::new(120.0, 28.0),
            viewport,
            &[],
            &[],
            true,
            &routes,
        )
        .unwrap();
        assert!(
            viewport.contains_rect(explicit.bounds),
            "explicit inspection retains its bounded callout"
        );
        let clear_route = [Pos2::new(20.0, 495.0), Pos2::new(780.0, 495.0)];
        let clear = place(
            &clear_route,
            Vec2::new(100.0, 24.0),
            viewport,
            &[],
            &[],
            false,
            &routes,
        )
        .unwrap();
        assert!(!routes.intersects(clear.bounds.expand(2.0)));
    }
}
