//! Small local frame instrumentation; no network telemetry.
use std::{collections::VecDeque, time::Instant};

#[derive(Default)]
pub struct FrameTiming {
    frames: VecDeque<f64>,
    pub scene_ms: f64,
    pub layout_ms: f64,
    pub hit_us: f64,
    pub input_to_frame_ms: f64,
    pub visible_nodes: usize,
    pub total_nodes: usize,
    last_frame: Option<Instant>,
    input_at: Option<Instant>,
}
impl FrameTiming {
    pub fn input(&mut self) {
        self.input_at = Some(Instant::now());
    }
    pub fn frame(&mut self) {
        let now = Instant::now();
        if let Some(previous) = self.last_frame.replace(now) {
            if self.frames.len() == 240 {
                self.frames.pop_front();
            }
            self.frames
                .push_back((now - previous).as_secs_f64() * 1000.0);
        }
        if let Some(input) = self.input_at.take() {
            self.input_to_frame_ms = (now - input).as_secs_f64() * 1000.0;
        }
    }
    pub fn median_ms(&self) -> f64 {
        let mut frames: Vec<_> = self.frames.iter().copied().collect();
        frames.sort_by(f64::total_cmp);
        frames.get(frames.len() / 2).copied().unwrap_or(0.0)
    }
    pub fn p95_ms(&self) -> f64 {
        let mut frames: Vec<_> = self.frames.iter().copied().collect();
        frames.sort_by(f64::total_cmp);
        frames
            .get(frames.len().saturating_sub(1) * 95 / 100)
            .copied()
            .unwrap_or(0.0)
    }
}
