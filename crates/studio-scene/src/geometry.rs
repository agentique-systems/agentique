//! Logical-pixel world geometry. Physical DPI belongs to the renderer.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> f32 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}
impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}
impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            min: Point::new(x, y),
            max: Point::new(x + width, y + height),
        }
    }
    pub fn from_points(a: Point, b: Point) -> Self {
        Self {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }
    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }
    pub fn size(self) -> Size {
        Size::new(self.width(), self.height())
    }
    pub fn center(self) -> Point {
        Point::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn contains_rect(self, r: Self) -> bool {
        self.contains(r.min) && self.contains(r.max)
    }
    pub fn intersects(self, r: Self) -> bool {
        self.min.x <= r.max.x
            && self.max.x >= r.min.x
            && self.min.y <= r.max.y
            && self.max.y >= r.min.y
    }
    pub fn union(self, r: Self) -> Self {
        Self {
            min: Point::new(self.min.x.min(r.min.x), self.min.y.min(r.min.y)),
            max: Point::new(self.max.x.max(r.max.x), self.max.y.max(r.max.y)),
        }
    }
    pub fn inflate(self, amount: f32) -> Self {
        Self::new(
            self.min.x - amount,
            self.min.y - amount,
            self.width() + 2.0 * amount,
            self.height() + 2.0 * amount,
        )
    }
    pub fn translate(self, delta: Point) -> Self {
        Self::new(
            self.min.x + delta.x,
            self.min.y + delta.y,
            self.width(),
            self.height(),
        )
    }
    pub fn finite(self) -> bool {
        [self.min.x, self.min.y, self.max.x, self.max.y]
            .iter()
            .all(|v| v.is_finite())
            && self.width().is_finite()
            && self.width() >= 0.0
            && self.height().is_finite()
            && self.height() >= 0.0
    }
}
pub fn segment_distance(point: Point, a: Point, b: Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = dx * dx + dy * dy;
    if length <= f32::EPSILON {
        return point.distance(a);
    }
    let t = ((point.x - a.x) * dx + (point.y - a.y) * dy) / length;
    let t = t.clamp(0.0, 1.0);
    point.distance(Point::new(a.x + t * dx, a.y + t * dy))
}
