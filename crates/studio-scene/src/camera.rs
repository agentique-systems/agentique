use crate::{Point, Rect, Size};
use serde::{Deserialize, Serialize};

/// Presentation/session state only. Coordinates are logical viewport pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera2D {
    pub center: Point,
    pub zoom: f32,
    pub viewport: Size,
}
impl Default for Camera2D {
    fn default() -> Self {
        Self {
            center: Point::default(),
            zoom: 1.0,
            viewport: Size::new(1200.0, 800.0),
        }
    }
}
impl Camera2D {
    pub const MIN_ZOOM: f32 = 0.025;
    pub const MAX_ZOOM: f32 = 8.0;
    pub fn world_to_screen(&self, p: Point) -> Point {
        Point::new(
            (p.x - self.center.x) * self.zoom + self.viewport.width * 0.5,
            (p.y - self.center.y) * self.zoom + self.viewport.height * 0.5,
        )
    }
    pub fn screen_to_world(&self, p: Point) -> Point {
        Point::new(
            (p.x - self.viewport.width * 0.5) / self.zoom + self.center.x,
            (p.y - self.viewport.height * 0.5) / self.zoom + self.center.y,
        )
    }
    pub fn pan_screen(&mut self, delta: Point) {
        if delta.x.is_finite() && delta.y.is_finite() {
            self.center.x -= delta.x / self.zoom;
            self.center.y -= delta.y / self.zoom;
        }
    }
    /// Keep the world point beneath the pointer fixed through zoom.
    pub fn zoom_at(&mut self, pointer: Point, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 || !pointer.x.is_finite() || !pointer.y.is_finite()
        {
            return;
        }
        let before = self.screen_to_world(pointer);
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let after = self.screen_to_world(pointer);
        self.center.x += before.x - after.x;
        self.center.y += before.y - after.y;
    }
    pub fn fit(&mut self, bounds: Rect, padding: f32) {
        if !bounds.finite() {
            return;
        }
        self.center = bounds.center();
        self.zoom = ((self.viewport.width - 2.0 * padding).max(1.0) / bounds.width().max(1.0))
            .min((self.viewport.height - 2.0 * padding).max(1.0) / bounds.height().max(1.0))
            .clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }
    pub fn visible_rect(&self) -> Rect {
        Rect::from_points(
            self.screen_to_world(Point::default()),
            self.screen_to_world(Point::new(self.viewport.width, self.viewport.height)),
        )
    }
}

/// Semantic detail is independent of renderer, DPI and canonical state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LodLevel {
    Overview,
    #[default]
    Summary,
    Features,
    Relationships,
    Evidence,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct LodController {
    level: LodLevel,
}
impl LodController {
    pub fn level(&self) -> LodLevel {
        self.level
    }
    /// Entry/exit thresholds differ by 18%; wheel noise cannot flicker labels.
    pub fn update(&mut self, zoom: f32) -> LodLevel {
        const LEVELS: [LodLevel; 5] = [
            LodLevel::Overview,
            LodLevel::Summary,
            LodLevel::Features,
            LodLevel::Relationships,
            LodLevel::Evidence,
        ];
        const THRESHOLDS: [f32; 4] = [0.32, 0.80, 1.45, 2.25];
        let mut index = self.level as usize;
        while index < 4 && zoom > THRESHOLDS[index] * 1.09 {
            index += 1;
        }
        while index > 0 && zoom < THRESHOLDS[index - 1] * 0.91 {
            index -= 1;
        }
        self.level = LEVELS[index];
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_anchor_property_across_100000_scene_scale_and_zoom_cases() {
        let mut seed = 0x41_47_51_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) as u32) as f32 / u32::MAX as f32
        };
        let mut maximum = 0.0_f32;
        for case in 0..100_000 {
            let extent = [100.0, 1000.0, 10_000.0, 40_000.0][case % 4];
            let mut camera = Camera2D {
                center: Point::new((next() - 0.5) * extent, (next() - 0.5) * extent),
                zoom: (Camera2D::MIN_ZOOM * (1.0_f32 + 319.0 * next()))
                    .clamp(Camera2D::MIN_ZOOM, Camera2D::MAX_ZOOM),
                viewport: Size::new(200.0 + 3000.0 * next(), 160.0 + 1800.0 * next()),
            };
            let pointer = Point::new(
                next() * camera.viewport.width,
                next() * camera.viewport.height,
            );
            let before = camera.screen_to_world(pointer);
            camera.zoom_at(pointer, (next() * 4.0 - 2.0).exp());
            let error = before.distance(camera.screen_to_world(pointer));
            maximum = maximum.max(error);
            assert!(error <= 0.025, "case {case}: error {error}");
        }
        println!("100000 camera cases; maximum anchor error {maximum} world units");
    }

    #[test]
    fn invalid_pointer_or_factor_cannot_corrupt_camera() {
        let initial = Camera2D::default();
        for (pointer, factor) in [
            (Point::new(f32::NAN, 10.0), 2.0),
            (Point::new(1.0, f32::INFINITY), 0.5),
            (Point::default(), f32::NAN),
            (Point::default(), 0.0),
        ] {
            let mut camera = initial;
            camera.zoom_at(pointer, factor);
            assert_eq!(camera, initial);
        }
    }
}
