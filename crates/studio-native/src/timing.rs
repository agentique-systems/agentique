//! Bounded local CPU timing samples. These are not GPU or physical-input timers.
use serde::Serialize;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const SAMPLE_WINDOW: usize = 240;
const FRAME_WARMUP_INTERVALS: usize = 60;

#[derive(Clone, Copy)]
pub enum InputKind {
    Pan,
    Zoom,
    Selection,
}
impl InputKind {
    const fn index(self) -> usize {
        match self {
            Self::Pan => 0,
            Self::Zoom => 1,
            Self::Selection => 2,
        }
    }
}

/// `None` means unmeasured. Counts distinguish one observation from a distribution.
#[derive(Debug, Serialize)]
pub struct SampleSummary {
    pub window_samples: usize,
    pub total_samples: usize,
    pub median: Option<f64>,
    pub p95: Option<f64>,
    pub last: Option<f64>,
}

#[derive(Clone, Default)]
pub(crate) struct Samples {
    values: VecDeque<f64>,
    total: usize,
}
impl Samples {
    pub(crate) fn push(&mut self, value: f64) {
        if self.values.len() == SAMPLE_WINDOW {
            self.values.pop_front();
        }
        self.values.push_back(value);
        self.total += 1;
    }
    pub(crate) fn summary(&self) -> SampleSummary {
        let mut sorted: Vec<_> = self.values.iter().copied().collect();
        sorted.sort_by(f64::total_cmp);
        let count = sorted.len();
        SampleSummary {
            window_samples: count,
            total_samples: self.total,
            median: if count == 0 {
                None
            } else if count.is_multiple_of(2) {
                Some((sorted[count / 2 - 1] + sorted[count / 2]) / 2.0)
            } else {
                Some(sorted[count / 2])
            },
            p95: count
                .checked_sub(1)
                .map(|_| sorted[(count * 95).div_ceil(100) - 1]),
            last: self.values.back().copied(),
        }
    }
}

#[derive(Default)]
pub struct FrameTiming {
    frames: Samples,
    hits: Samples,
    inputs: [Samples; 3],
    intervals_seen: usize,
    pub scene_ms: f64,
    pub layout_ms: f64,
    pub visible_nodes: usize,
    pub total_nodes: usize,
    last_frame: Option<Instant>,
    pending_inputs: [Option<Instant>; 3],
}
impl FrameTiming {
    /// Marks handling in the viewport. The next `frame` records elapsed CPU wall
    /// time to the next UI update, not completion/presentation of the current frame.
    pub fn input(&mut self, kind: InputKind) {
        self.input_at(kind, Instant::now());
    }
    fn input_at(&mut self, kind: InputKind, now: Instant) {
        // A second event in one update must not erase the first event's wait.
        self.pending_inputs[kind.index()].get_or_insert(now);
    }
    pub fn hit(&mut self, elapsed: Duration) {
        self.hits.push(elapsed.as_secs_f64() * 1_000_000.0);
    }
    pub fn frame(&mut self) {
        self.frame_at(Instant::now());
    }
    fn frame_at(&mut self, now: Instant) {
        if let Some(previous) = self.last_frame.replace(now) {
            self.intervals_seen += 1;
            if self.intervals_seen > FRAME_WARMUP_INTERVALS {
                self.frames
                    .push(now.duration_since(previous).as_secs_f64() * 1000.0);
            }
        }
        for (pending, samples) in self.pending_inputs.iter_mut().zip(&mut self.inputs) {
            if let Some(input) = pending.take() {
                samples.push(now.duration_since(input).as_secs_f64() * 1000.0);
            }
        }
    }
    pub fn frame_summary(&self) -> SampleSummary {
        self.frames.summary()
    }
    pub fn hit_summary(&self) -> SampleSummary {
        self.hits.summary()
    }
    pub fn input_summary(&self, kind: InputKind) -> SampleSummary {
        self.inputs[kind.index()].summary()
    }
    pub fn discarded_frame_intervals(&self) -> usize {
        self.intervals_seen.min(FRAME_WARMUP_INTERVALS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_measurements_are_unavailable_including_serialized_output() {
        let timing = FrameTiming::default();
        for summary in [
            timing.frame_summary(),
            timing.hit_summary(),
            timing.input_summary(InputKind::Pan),
        ] {
            assert_eq!(summary.window_samples, 0);
            let json = serde_json::to_value(summary).unwrap();
            assert!(json["median"].is_null());
            assert!(json["p95"].is_null());
            assert!(json["last"].is_null());
        }
    }
    #[test]
    fn gestures_keep_earliest_event_and_remain_separate() {
        let mut timing = FrameTiming::default();
        let start = Instant::now();
        timing.input_at(InputKind::Pan, start);
        timing.input_at(InputKind::Pan, start + Duration::from_millis(5));
        timing.input_at(InputKind::Zoom, start + Duration::from_millis(6));
        timing.frame_at(start + Duration::from_millis(16));
        assert_eq!(timing.input_summary(InputKind::Pan).last, Some(16.0));
        assert_eq!(timing.input_summary(InputKind::Zoom).last, Some(10.0));
        assert_eq!(timing.input_summary(InputKind::Selection).last, None);
        timing.frame_at(start + Duration::from_millis(32));
        assert_eq!(timing.input_summary(InputKind::Pan).total_samples, 1);
    }
    #[test]
    fn startup_stalls_are_discarded_and_storage_stays_bounded() {
        let mut timing = FrameTiming::default();
        let mut now = Instant::now();
        timing.frame_at(now);
        for _ in 0..FRAME_WARMUP_INTERVALS {
            now += Duration::from_secs(1);
            timing.frame_at(now);
        }
        assert_eq!(timing.frame_summary().median, None);
        for _ in 0..300 {
            now += Duration::from_millis(16);
            timing.frame_at(now);
        }
        let frames = timing.frame_summary();
        assert_eq!(frames.window_samples, SAMPLE_WINDOW);
        assert_eq!(frames.total_samples, 300);
        assert_eq!(frames.median, Some(16.0));
        assert_eq!(frames.p95, Some(16.0));
        assert_eq!(timing.discarded_frame_intervals(), FRAME_WARMUP_INTERVALS);
    }
    #[test]
    fn tail_latency_survives_the_summary() {
        let mut timing = FrameTiming::default();
        for _ in 0..90 {
            timing.hit(Duration::from_micros(4));
        }
        for _ in 0..10 {
            timing.hit(Duration::from_micros(40));
        }
        assert_eq!(timing.hit_summary().median, Some(4.0));
        assert_eq!(timing.hit_summary().p95, Some(40.0));
    }
}
