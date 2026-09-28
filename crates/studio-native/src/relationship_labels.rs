//! Screen-space placement of relationship labels on the Surface: each label
//! beside its route, clear of cards, of earlier labels and of dense routes.
//! Routes and semantic identity are untouched; drawing is the Surface's.
use agq_studio_scene::{Point, Rect, Size};
use std::collections::HashMap;

/// Space between a label's text and its outline; `place` takes the text size
/// plus twice this.
pub(crate) const PADDING: Size = Size {
    width: 8.0,
    height: 4.0,
};

fn rect_centered(center: Point, size: Size) -> Rect {
    Rect::new(
        center.x - size.width * 0.5,
        center.y - size.height * 0.5,
        size.width,
        size.height,
    )
}

fn along(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn distance_sq(a: Point, b: Point) -> f32 {
    (b.x - a.x).powi(2) + (b.y - a.y).powi(2)
}

/// Screen-space cells bound the work of avoiding dense routes. This contains
/// drawing segments only, never model elements or relationships.
#[derive(Default)]
pub(crate) struct RouteObstacles {
    cells: HashMap<(i32, i32), Vec<[Point; 2]>>,
}
impl RouteObstacles {
    const CELL: f32 = 64.0;
    pub fn insert(&mut self, a: Point, b: Point, viewport: Rect) {
        let Some((a, b)) = clip(a, b, viewport) else {
            return;
        };
        let bounds = Rect::from_points(a, b);
        for x in (bounds.min.x / Self::CELL).floor() as i32..=(bounds.max.x / Self::CELL).floor() as i32 {
            for y in (bounds.min.y / Self::CELL).floor() as i32..=(bounds.max.y / Self::CELL).floor() as i32 {
                self.cells.entry((x, y)).or_default().push([a, b]);
            }
        }
    }
    fn intersects(&self, bounds: Rect) -> bool {
        for x in (bounds.min.x / Self::CELL).floor() as i32..=(bounds.max.x / Self::CELL).floor() as i32 {
            for y in (bounds.min.y / Self::CELL).floor() as i32..=(bounds.max.y / Self::CELL).floor() as i32 {
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
    pub anchor: Point,
}

/// Prefer long visible route segments and keep automatic annotations clear of
/// cards and earlier, higher-priority labels. Explicit inspection can fall back
/// to a bounded callout; its exact edge is still available in the Inspector.
pub(crate) fn place(
    route: &[Point],
    size: Size,
    viewport: Rect,
    obstacles: &[Rect],
    previous: &[Rect],
    explicit: bool,
    routes: &RouteObstacles,
) -> Option<Placement> {
    let viewport = viewport.inflate(-4.0);
    if size.width > viewport.width() || size.height > viewport.height() {
        return None;
    }
    let mut segments: Vec<_> = route
        .windows(2)
        .filter_map(|pair| clip(pair[0], pair[1], viewport))
        .filter(|(a, b)| distance_sq(*a, *b) >= 4.0)
        .collect();
    // Stable sorting preserves route order when parallel segments have equal length.
    segments.sort_by(|(a, b), (c, d)| distance_sq(*c, *d).total_cmp(&distance_sq(*a, *b)));
    let mut fallback = None;
    for (a, b) in segments.into_iter().take(8) {
        let horizontal = (b.x - a.x).abs() >= (b.y - a.y).abs();
        let normal = if horizontal {
            Point::new(0.0, 1.0)
        } else {
            Point::new(1.0, 0.0)
        };
        let offset = if horizontal { size.height } else { size.width } * 0.5 + 5.0;
        for fraction in [0.5, 0.3, 0.7, 0.15, 0.85] {
            let anchor = along(a, b, fraction);
            for side in [-1.0, 1.0] {
                let center = Point::new(
                    anchor.x + normal.x * offset * side,
                    anchor.y + normal.y * offset * side,
                );
                let bounds = rect_centered(center, size);
                fallback.get_or_insert_with(|| Placement {
                    bounds: rect_centered(
                        Point::new(
                            center.x.clamp(
                                viewport.min.x + size.width * 0.5,
                                viewport.max.x - size.width * 0.5,
                            ),
                            center.y.clamp(
                                viewport.min.y + size.height * 0.5,
                                viewport.max.y - size.height * 0.5,
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
                        .any(|obstacle| obstacle.inflate(3.0).intersects(bounds))
                    && !routes.intersects(bounds.inflate(2.0))
                {
                    return Some(Placement { bounds, anchor });
                }
            }
        }
    }
    explicit.then_some(fallback).flatten()
}

fn clip(a: Point, b: Point, bounds: Rect) -> Option<(Point, Point)> {
    let delta = Point::new(b.x - a.x, b.y - a.y);
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for (start, direction, min, max) in [
        (a.x, delta.x, bounds.min.x, bounds.max.x),
        (a.y, delta.y, bounds.min.y, bounds.max.y),
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
    Some((along(a, b, enter), along(a, b, leave)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_points(Point::new(x0, y0), Point::new(x1, y1))
    }

    #[test]
    fn offscreen_long_route_uses_visible_segment_and_avoids_cards_and_prior_labels() {
        let viewport = rect(0.0, 0.0, 400.0, 300.0);
        let route = [Point::new(-2000.0, 160.0), Point::new(300.0, 160.0)];
        let card = rect(100.0, 100.0, 210.0, 155.0);
        let size = Size::new(80.0, 24.0);
        let first = place(
            &route,
            size,
            viewport,
            &[card],
            &[],
            false,
            &RouteObstacles::default(),
        )
        .unwrap();
        assert!(viewport.contains(first.anchor));
        assert!(viewport.contains_rect(first.bounds));
        assert!(!card.inflate(3.0).intersects(first.bounds));
        let second = place(
            &route,
            size,
            viewport,
            &[card],
            &[first.bounds],
            false,
            &RouteObstacles::default(),
        )
        .unwrap();
        assert!(!first.bounds.inflate(3.0).intersects(second.bounds));
        assert!(!card.inflate(3.0).intersects(second.bounds));
    }

    #[test]
    fn crowded_automatic_labels_yield_but_explicit_inspection_keeps_visible_callout() {
        let viewport = rect(0.0, 0.0, 300.0, 200.0);
        let route = [Point::new(-600.0, 100.0), Point::new(500.0, 100.0)];
        let size = Size::new(90.0, 24.0);
        assert!(
            place(
                &route,
                size,
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
            size,
            viewport,
            &[viewport],
            &[],
            true,
            &RouteObstacles::default(),
        )
        .unwrap();
        assert!(viewport.contains_rect(explicit.bounds));
        assert!(viewport.contains(explicit.anchor));
        let outside = [Point::new(-200.0, -100.0), Point::new(500.0, -100.0)];
        assert!(
            place(
                &outside,
                size,
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
        let viewport = rect(0.0, 0.0, 800.0, 500.0);
        let mut routes = RouteObstacles::default();
        for index in 0..20 {
            let y = 110.0 + index as f32 * 12.0;
            routes.insert(Point::new(20.0, y), Point::new(780.0, y), viewport);
            let x = 140.0 + index as f32 * 24.0;
            routes.insert(Point::new(x, 40.0), Point::new(x, 460.0), viewport);
        }
        let route = [Point::new(20.0, 180.0), Point::new(780.0, 180.0)];
        let size = Size::new(120.0, 28.0);
        let automatic = place(&route, size, viewport, &[], &[], false, &routes);
        assert!(
            automatic.is_none(),
            "dense context labels must yield instead of hiding paths"
        );
        let explicit = place(&route, size, viewport, &[], &[], true, &routes).unwrap();
        assert!(
            viewport.contains_rect(explicit.bounds),
            "explicit inspection retains its bounded callout"
        );
        let clear_route = [Point::new(20.0, 495.0), Point::new(780.0, 495.0)];
        let clear = place(
            &clear_route,
            Size::new(100.0, 24.0),
            viewport,
            &[],
            &[],
            false,
            &routes,
        )
        .unwrap();
        assert!(!routes.intersects(clear.bounds.inflate(2.0)));
    }
}
