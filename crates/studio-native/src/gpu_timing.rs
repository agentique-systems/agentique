//! Optional scene-pass GPU timestamps with bounded asynchronous readback.
//! Resolution and mapping happen in later frames, never a blocking frame wait.
use crate::gpu::GpuStats;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

/// Diagnostic categories are independent of the measured pass durations.
/// A valid zero-duration interval is retained, not classified as an error.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Diagnostics {
    pub zero_duration_samples: usize,
    pub non_monotonic_samples: usize,
    pub map_errors: usize,
    pub poll_errors: usize,
    pub surface_invalidations: usize,
}

fn record_sample(stats: &mut GpuStats, start: u64, end: u64, period_ns: f64) {
    if let Some(delta) = end.checked_sub(start) {
        stats
            .timestamp_ms
            .push(delta as f64 * period_ns / 1_000_000.0);
        if delta == 0 {
            stats.timestamp_diagnostics.zero_duration_samples += 1;
        }
    } else {
        stats.timestamp_errors += 1;
        stats.timestamp_diagnostics.non_monotonic_samples += 1;
    }
}

pub fn features() -> wgpu::Features {
    wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
}

enum State {
    Idle,
    Drawn,
    Resolved,
    Mapping(Arc<AtomicU8>),
}
struct Slot {
    buffer: wgpu::Buffer,
    state: State,
    written: AtomicBool,
}
pub struct GpuTiming {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    current: Option<usize>,
    period_ns: f64,
}

impl GpuTiming {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(features()) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Scene GPU timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 8,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scene timestamp resolution"),
            size: 4 * wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..4)
            .map(|_| Slot {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Scene timestamp readback"),
                    size: 16,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                state: State::Idle,
                written: AtomicBool::new(false),
            })
            .collect();
        Some(Self {
            queries,
            resolve,
            slots,
            current: None,
            period_ns: f64::from(queue.get_timestamp_period()),
        })
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut GpuStats,
    ) {
        // Poll services completed maps without waiting for the GPU.
        if device.poll(wgpu::PollType::Poll).is_err() {
            stats.timestamp_errors += 1;
            stats.timestamp_diagnostics.poll_errors += 1;
        }
        self.current = None;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            match &slot.state {
                State::Mapping(done) => match done.load(Ordering::Acquire) {
                    1 => {
                        let bytes = slot.buffer.slice(..).get_mapped_range();
                        let start =
                            u64::from_le_bytes(bytes[0..8].try_into().expect("timestamp size"));
                        let end =
                            u64::from_le_bytes(bytes[8..16].try_into().expect("timestamp size"));
                        record_sample(stats, start, end, self.period_ns);
                        drop(bytes);
                        slot.buffer.unmap();
                        slot.state = State::Idle;
                    }
                    2 => {
                        stats.timestamp_errors += 1;
                        stats.timestamp_diagnostics.map_errors += 1;
                        slot.state = State::Idle;
                    }
                    _ => {}
                },
                State::Resolved => {
                    // This copy was submitted by eframe in the previous frame.
                    // Mapping before that submission would invalidate the copy.
                    let done = Arc::new(AtomicU8::new(0));
                    let callback = done.clone();
                    slot.buffer
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |result| {
                            callback.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release)
                        });
                    slot.state = State::Mapping(done);
                }
                State::Drawn => {
                    if slot.written.swap(false, Ordering::AcqRel) {
                        let offset = index as u64 * wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT;
                        encoder.resolve_query_set(
                            &self.queries,
                            index as u32 * 2..index as u32 * 2 + 2,
                            &self.resolve,
                            offset,
                        );
                        encoder.copy_buffer_to_buffer(&self.resolve, offset, &slot.buffer, 0, 16);
                        slot.state = State::Resolved;
                    } else {
                        slot.state = State::Idle;
                    }
                }
                State::Idle => {}
            }
        }
        if let Some(index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot.state, State::Idle))
        {
            self.slots[index].state = State::Drawn;
            self.current = Some(index);
        }
    }

    pub fn begin(&self, pass: &mut wgpu::RenderPass<'static>) {
        if let Some(index) = self.current {
            pass.write_timestamp(&self.queries, index as u32 * 2);
        }
    }
    pub fn end(&self, pass: &mut wgpu::RenderPass<'static>) {
        if let Some(index) = self.current {
            pass.write_timestamp(&self.queries, index as u32 * 2 + 1);
            self.slots[index].written.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_duration_measurements_are_retained_without_hiding_invalid_pairs() {
        let mut stats = GpuStats::default();
        record_sample(&mut stats, 100, 100, 1024.0);
        record_sample(&mut stats, 100, 101, 1024.0);
        record_sample(&mut stats, 101, 100, 1024.0);
        let summary = stats.timestamp_ms.summary();
        assert_eq!(summary.total_samples, 2);
        assert_eq!(summary.median, Some(0.000512));
        assert_eq!(summary.last, Some(0.001024));
        assert_eq!(stats.timestamp_diagnostics.zero_duration_samples, 1);
        assert_eq!(stats.timestamp_diagnostics.non_monotonic_samples, 1);
        assert_eq!(stats.timestamp_errors, 1);
        assert_eq!(stats.timestamp_diagnostics.map_errors, 0);
        assert_eq!(stats.timestamp_diagnostics.poll_errors, 0);
        assert_eq!(stats.timestamp_diagnostics.surface_invalidations, 0);
    }
}
