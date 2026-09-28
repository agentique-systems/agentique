//! How things move (ROADMAP §3.2, motion tokens): a tween runs a duration
//! token along a Fluent 2 easing curve; a critically damped spring follows a
//! target that may change while it moves, without overshoot. Under reduced
//! motion both arrive at once. The camera's moves use both (`CameraMove`).
use agq_studio_scene::{Camera2D, Point};
use std::sync::OnceLock;
use std::time::Instant;

/// A camera move's duration (§3.2 motion tokens).
pub const CAMERA_SECONDS: f32 = crate::tokens::motion::CAMERA_MS as f32 / 1000.0;
/// How long a change stays highlighted on the Surface: it holds, then fades.
pub const CHANGED_SECONDS: f32 =
    (crate::tokens::motion::CHANGE_HOLD_MS + crate::tokens::motion::CHANGE_FADE_MS) as f32 / 1000.0;

/// Seconds since the Studio started: the one clock for moves and highlights.
pub fn clock() -> f32 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f32()
}

/// How strongly a change highlighted `age` seconds ago shows: it holds, then
/// fades out along the decelerating curve; 0 once it is over.
pub fn highlight(age: f32) -> f32 {
    let hold = crate::tokens::motion::CHANGE_HOLD_MS as f32 / 1000.0;
    if !(0.0..CHANGED_SECONDS).contains(&age) {
        return 0.0;
    }
    if age <= hold {
        return 1.0;
    }
    1.0 - Curve::EASY_EASE.at((age - hold) / (CHANGED_SECONDS - hold))
}

/// A cubic Bézier easing curve from (0, 0) to (1, 1), written as in CSS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl Curve {
    /// Fluent 2 `curveEasyEase`: something moving within the view.
    pub const EASY_EASE: Self = Self::token(crate::tokens::motion::EASY_EASE);

    const fn token((x1, y1, x2, y2): crate::tokens::motion::Curve) -> Self {
        Self::new(x1, y1, x2, y2)
    }

    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// The eased value at time `t` (`0..=1`).
    pub fn at(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        // Solve x(s) = t for the curve parameter s: Newton's method, with
        // bisection when the slope is too flat to trust.
        let bezier = |a: f32, b: f32, s: f32| {
            let r = 1.0 - s;
            3.0 * r * r * s * a + 3.0 * r * s * s * b + s * s * s
        };
        let slope = |a: f32, b: f32, s: f32| {
            let r = 1.0 - s;
            3.0 * r * r * a + 6.0 * r * s * (b - a) + 3.0 * s * s * (1.0 - b)
        };
        let mut s = t;
        for _ in 0..8 {
            let error = bezier(self.x1, self.x2, s) - t;
            let d = slope(self.x1, self.x2, s);
            if error.abs() < 1e-5 || d.abs() < 1e-4 {
                break;
            }
            s = (s - error / d).clamp(0.0, 1.0);
        }
        if (bezier(self.x1, self.x2, s) - t).abs() > 1e-4 {
            let (mut low, mut high) = (0.0, 1.0);
            for _ in 0..30 {
                s = 0.5 * (low + high);
                if bezier(self.x1, self.x2, s) < t {
                    low = s;
                } else {
                    high = s;
                }
            }
        }
        bezier(self.y1, self.y2, s)
    }
}

/// A fixed-length move: `progress` goes from 0 to 1 over `duration` seconds
/// along `curve`. Under reduced motion it is finished at once.
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    start: f32,
    duration: f32,
    curve: Curve,
}

impl Tween {
    pub fn new(start: f32, duration: f32, curve: Curve, reduced_motion: bool) -> Self {
        Self {
            start,
            duration: if reduced_motion { 0.0 } else { duration },
            curve,
        }
    }

    /// Eased progress (`0..=1`) at time `now`.
    pub fn progress(&self, now: f32) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        self.curve.at((now - self.start) / self.duration)
    }

    pub fn finished(&self, now: f32) -> bool {
        now - self.start >= self.duration
    }
}

/// A critically damped spring: it reaches its target as fast as it can
/// without overshooting, and keeps its velocity when the target changes.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    /// Angular frequency, from the settle time.
    omega: f32,
}

/// `(1 + x) e^-x` falls to 0.1% at `x` = 9.23: a spring started at rest is
/// within 0.1% of its target after `SETTLE / omega` seconds.
const SETTLE: f32 = 9.23;

impl Spring {
    /// A spring at rest at `value` that settles in about `seconds`.
    pub fn new(value: f32, seconds: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            omega: SETTLE / seconds.max(1e-3),
        }
    }

    /// Advances `dt` seconds towards `target` (the exact solution, so any
    /// frame time is stable). Under reduced motion it jumps to the target.
    pub fn step(&mut self, target: f32, dt: f32, reduced_motion: bool) {
        if reduced_motion || !dt.is_finite() {
            self.value = target;
            self.velocity = 0.0;
            return;
        }
        let dt = dt.max(0.0);
        let offset = self.value - target;
        let c = self.velocity + self.omega * offset;
        let decay = (-self.omega * dt).exp();
        self.value = target + (offset + c * dt) * decay;
        self.velocity = (self.velocity - self.omega * c * dt) * decay;
    }

    /// Whether it is within `tolerance` of `target` and nearly still.
    pub fn settled(&self, target: f32, tolerance: f32) -> bool {
        (self.value - target).abs() <= tolerance && self.velocity.abs() <= tolerance * self.omega
    }
}

/// A camera move towards a target camera. Fit view is a one-off move: a
/// 300 ms tween (`theme::CAMERA_SECONDS`). Following the selection may be
/// redirected while it runs (several links clicked in a row), so it uses
/// springs, which keep their velocity when the target changes. Zoom moves in
/// log space, so zooming in and out look alike.
#[derive(Clone, Copy, Debug)]
pub enum CameraMove {
    Tween { from: Camera2D, tween: Tween },
    Spring([Spring; 3]),
}

impl CameraMove {
    pub fn tween(from: Camera2D, now: f32, reduced_motion: bool) -> Self {
        Self::Tween {
            from,
            tween: Tween::new(
                now,
                CAMERA_SECONDS,
                Curve::EASY_EASE,
                reduced_motion,
            ),
        }
    }

    /// A spring move from `camera`, keeping the velocity of a spring move
    /// already running.
    pub fn follow(previous: Option<Self>, camera: Camera2D) -> Self {
        match previous {
            Some(spring @ Self::Spring(_)) => spring,
            _ => {
                let seconds = CAMERA_SECONDS;
                Self::Spring([
                    Spring::new(camera.center.x, seconds),
                    Spring::new(camera.center.y, seconds),
                    Spring::new(camera.zoom.max(1e-6).ln(), seconds),
                ])
            }
        }
    }

    /// Moves `camera` towards `target` at time `now`, `dt` seconds after the
    /// last step. Returns true once it has arrived (the camera is then
    /// exactly `target`).
    pub fn step(
        &mut self,
        camera: &mut Camera2D,
        target: Camera2D,
        now: f32,
        dt: f32,
        reduced_motion: bool,
    ) -> bool {
        let arrived = match self {
            Self::Tween { from, tween } => {
                let t = if reduced_motion {
                    1.0
                } else {
                    tween.progress(now)
                };
                let lerp = |a: f32, b: f32| a + (b - a) * t;
                camera.center = Point::new(
                    lerp(from.center.x, target.center.x),
                    lerp(from.center.y, target.center.y),
                );
                camera.zoom = lerp(from.zoom.max(1e-6).ln(), target.zoom.max(1e-6).ln()).exp();
                reduced_motion || tween.finished(now)
            }
            Self::Spring([x, y, zoom]) => {
                let log_zoom = target.zoom.max(1e-6).ln();
                x.step(target.center.x, dt, reduced_motion);
                y.step(target.center.y, dt, reduced_motion);
                zoom.step(log_zoom, dt, reduced_motion);
                camera.center = Point::new(x.value, y.value);
                camera.zoom = zoom.value.exp();
                // Within a tenth of a world unit at the target zoom, and
                // a thousandth of the zoom.
                let near = 0.1 / target.zoom.max(1e-3);
                x.settled(target.center.x, near)
                    && y.settled(target.center.y, near)
                    && zoom.settled(log_zoom, 1e-3)
            }
        };
        if arrived {
            *camera = target;
        }
        arrived
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_studio_scene::Size;

    #[test]
    fn curves_start_and_end_in_place_and_ease() {
        let curve = Curve::EASY_EASE;
        assert_eq!(curve.at(0.0), 0.0);
        assert_eq!(curve.at(1.0), 1.0);
        assert!((curve.at(0.5) - 0.5).abs() < 1e-3, "symmetric");
        assert!(curve.at(0.1) < 0.1, "starts slowly");
        assert!(curve.at(0.9) > 0.9, "ends slowly");
        let mut last = 0.0;
        for step in 1..=100 {
            let value = curve.at(step as f32 / 100.0);
            assert!(value >= last, "monotonic");
            last = value;
        }
        // CSS's `ease-out` (0, 0, 0.58, 1) at x = 0.5 is about 0.6847.
        assert!((Curve::new(0.0, 0.0, 0.58, 1.0).at(0.5) - 0.6847).abs() < 2e-3);
    }

    #[test]
    fn a_tween_takes_its_duration_or_nothing_under_reduced_motion() {
        let tween = Tween::new(10.0, 0.3, Curve::EASY_EASE, false);
        assert_eq!(tween.progress(10.0), 0.0);
        assert!(!tween.finished(10.2));
        assert!(tween.progress(10.15) > 0.4 && tween.progress(10.15) < 0.6);
        assert!(tween.finished(10.3));
        assert_eq!(tween.progress(11.0), 1.0);
        let reduced = Tween::new(10.0, 0.3, Curve::EASY_EASE, true);
        assert!(reduced.finished(10.0));
        assert_eq!(reduced.progress(10.0), 1.0);
    }

    #[test]
    fn a_critically_damped_spring_settles_without_overshoot() {
        let mut spring = Spring::new(0.0, 0.3);
        let mut time = 0.0;
        while time < 0.3 {
            spring.step(100.0, 1.0 / 60.0, false);
            time += 1.0 / 60.0;
            assert!(spring.value <= 100.0 + 1e-3, "no overshoot");
        }
        assert!(spring.settled(100.0, 0.2), "{spring:?}");
        // Frame time does not change where it ends up: one step of 0.3 s
        // lands where eighteen steps of a sixtieth do.
        let mut once = Spring::new(0.0, 0.3);
        once.step(100.0, 0.3, false);
        assert!((once.value - spring.value).abs() < 0.05);
        // A new target keeps the velocity: no sudden stop.
        let mut moving = Spring::new(0.0, 0.3);
        moving.step(100.0, 0.05, false);
        let velocity = moving.velocity;
        moving.step(200.0, 1e-4, false);
        assert!((moving.velocity - velocity).abs() / velocity < 0.1);
        let mut reduced = Spring::new(0.0, 0.3);
        reduced.step(100.0, 1.0 / 60.0, true);
        assert_eq!(reduced.value, 100.0);
    }

    #[test]
    fn camera_moves_arrive_exactly_and_at_once_under_reduced_motion() {
        let from = Camera2D {
            center: Point::new(0.0, 0.0),
            zoom: 0.5,
            viewport: Size::new(800.0, 600.0),
        };
        let target = Camera2D {
            center: Point::new(400.0, -200.0),
            zoom: 2.0,
            ..from
        };
        let mut camera = from;
        let mut fit = CameraMove::tween(from, 0.0, false);
        assert!(!fit.step(&mut camera, target, 0.15, 0.15, false));
        // Halfway in time, halfway in log zoom: 0.5 → 1.0 → 2.0.
        assert!((camera.zoom - 1.0).abs() < 0.05, "{}", camera.zoom);
        assert!(fit.step(&mut camera, target, 0.3, 0.15, false));
        assert_eq!(camera.center, target.center);
        assert_eq!(camera.zoom, target.zoom);

        let mut camera = from;
        let mut follow = CameraMove::follow(None, camera);
        let mut frames = 0;
        while !follow.step(&mut camera, target, 0.0, 1.0 / 60.0, false) {
            frames += 1;
            assert!(frames < 60, "settles within a second");
        }
        assert!(frames > 5, "moves over several frames");
        assert_eq!(camera.zoom, target.zoom);

        for mut motion in [
            CameraMove::tween(from, 0.0, true),
            CameraMove::follow(None, from),
        ] {
            let mut camera = from;
            assert!(motion.step(&mut camera, target, 0.0, 1.0 / 60.0, true));
            assert_eq!(camera.center, target.center);
        }
    }
}
