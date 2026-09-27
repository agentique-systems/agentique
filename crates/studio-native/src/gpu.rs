//! Retained instanced scene pipeline. Four ordered batches, independent of widget count.
//!
//! Camera motion updates one uniform. Geometry uploads only when the presentation
//! generation/visibility/selection changes. Text uses the toolkit glyph atlas above
//! this pass; it is culled and selected by semantic LOD before shaping.
//!
//! Visual parameters come from `theme`: cards in the node batch cast a soft
//! shadow, and `Quad::changed` / `Quad::halo` draw glows whose fade runs on the
//! GPU from `clock()`, so a highlight never forces a geometry upload.
use crate::theme;
use bytemuck::{Pod, Zeroable};
use eframe::egui;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};
use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Quad {
    pub rect: [f32; 4],
    pub fill: [f32; 4],
    pub border: [f32; 4],
    /// radius, border width, cos, sin
    pub style: [f32; 4],
    /// Dash period and ink length in world units (zero period is solid),
    /// effect (0 plain, `CHANGED`, `HALO`), effect start on `clock()`.
    pub detail: [f32; 4],
}
const CHANGED: f32 = 1.0;
const HALO: f32 = 2.0;

/// Seconds since the renderer started; the time base of `Quad::changed`.
pub fn clock() -> f32 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f32()
}
impl Quad {
    pub fn rect(
        rect: [f32; 4],
        fill: egui::Color32,
        border: egui::Color32,
        radius: f32,
        width: f32,
    ) -> Self {
        Self {
            rect,
            fill: color(fill),
            border: color(border),
            style: [radius, width, 1.0, 0.0],
            detail: [0.0; 4],
        }
    }
    pub fn segment(a: [f32; 2], b: [f32; 2], width: f32, fill: egui::Color32) -> Option<Self> {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = dx.hypot(dy);
        (length > 0.001).then(|| Self {
            rect: [
                a[0] + dy / length * width * 0.5,
                a[1] - dx / length * width * 0.5,
                length,
                width,
            ],
            fill: color(fill),
            border: color(fill),
            style: [width * 0.5, 0.0, dx / length, dy / length],
            detail: [0.0; 4],
        })
    }
    /// The changed highlight around `rect` (a card's own rectangle and radius):
    /// a ring, a soft outer glow and a light tint in `color` (`Theme::changed`)
    /// that rise and fade over `theme::CHANGED_SECONDS` from `started`
    /// (`clock()` when the change arrived). Push it to `Batch::overlays`, and
    /// request repaints while `theme::changed_intensity(clock() - started) > 0`.
    pub fn changed(rect: [f32; 4], radius: f32, color: egui::Color32, started: f32) -> Self {
        Self {
            detail: [0.0, 0.0, CHANGED, started],
            ..Self::rect(rect, color, color, radius, 0.0)
        }
    }
    /// A static soft glow around `rect` in `color`: selection and focus halos.
    /// Push it before the card it surrounds.
    pub fn halo(rect: [f32; 4], radius: f32, color: egui::Color32) -> Self {
        Self {
            detail: [0.0, 0.0, HALO, 0.0],
            ..Self::rect(rect, color, color, radius, 0.0)
        }
    }
}
fn color(value: egui::Color32) -> [f32; 4] {
    value.to_array().map(|v| v as f32 / 255.0)
}

/// Index of `Batch::nodes` among the four ordered ranges.
const NODES: usize = 2;

#[derive(Default)]
pub struct Batch {
    pub key: u64,
    pub containers: Vec<Quad>,
    pub edges: Vec<Quad>,
    pub nodes: Vec<Quad>,
    pub overlays: Vec<Quad>,
}
#[derive(Default, Debug, Clone)]
pub struct GpuStats {
    pub upload_ms: f64,
    pub uploaded_bytes: usize,
    pub uploads: usize,
    pub instances: usize,
    pub draw_calls: usize,
    pub timestamp_ms: crate::timing::Samples,
    pub timestamp_status: &'static str,
    pub timestamp_errors: usize,
    pub timestamp_diagnostics: crate::gpu_timing::Diagnostics,
}
struct Resources {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    instances: wgpu::Buffer,
    capacity: usize,
    ranges: [std::ops::Range<u32>; 4],
    key: Option<u64>,
    timing: Option<crate::gpu_timing::GpuTiming>,
    timing_status: &'static str,
    recovery: crate::surface_recovery::Recovery,
    surface_epoch: u64,
}

pub fn install(
    cc: &eframe::CreationContext<'_>,
    timestamps: bool,
    recovery: crate::surface_recovery::Recovery,
) -> Result<(), String> {
    let state = cc
        .wgpu_render_state
        .as_ref()
        .ok_or("Native Studio requires a wgpu adapter")?;
    let device = &state.device;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Agentique retained scene"),
        source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Scene camera"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: std::num::NonZeroU64::new(UNIFORM_BYTES),
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Scene pipeline"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let attributes =
        wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32x4];
    let pipeline = |label: &str, vertex: &str, fragment: &str| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(vertex),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(fragment),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.target_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    };
    let shadow_pipeline = pipeline("Scene card shadows", "shadow_vertex", "shadow_fragment");
    let pipeline = pipeline("Scene quads and routes", "vertex", "fragment");
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Scene camera uniform"),
        contents: bytemuck::cast_slice(&[0.0f32; UNIFORM_FLOATS]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scene camera binding"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    });
    let instances = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Retained scene instances"),
        size: 64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    state.renderer.write().callback_resources.insert(Resources {
        pipeline,
        shadow_pipeline,
        bind_group,
        uniform,
        instances,
        capacity: 64,
        ranges: [0..0, 0..0, 0..0, 0..0],
        key: None,
        timing: crate::gpu_timing::GpuTiming::new(device, &state.queue),
        surface_epoch: recovery.epoch(),
        recovery,
        timing_status: if !timestamps {
            "Not requested; use --gpu-timestamps"
        } else if !device.features().contains(crate::gpu_timing::features()) {
            "Adapter lacks timestamps inside render passes"
        } else {
            "Scene GPU pass only; excludes text, chrome, upload and presentation"
        },
    });
    Ok(())
}

const UNIFORM_FLOATS: usize = 16;
const UNIFORM_BYTES: u64 = (UNIFORM_FLOATS * 4) as u64;

/// The shader uniform: camera, scene clock, then the shadow and glow tokens.
fn uniform(camera: &[f32; 8], time: f32) -> [f32; UNIFORM_FLOATS] {
    let mut values = [0.0; UNIFORM_FLOATS];
    values[..6].copy_from_slice(&camera[..6]);
    values[6] = time;
    values[8..].copy_from_slice(&[
        theme::SHADOW_OFFSET,
        theme::SHADOW_BLUR,
        theme::SHADOW_OPACITY_DARK,
        theme::SHADOW_OPACITY_LIGHT,
        theme::GLOW_WIDTH,
        theme::CHANGED_RISE_SECONDS,
        theme::CHANGED_SECONDS,
        0.0,
    ]);
    values
}

pub struct SceneCallback {
    pub batch: Arc<Batch>,
    /// center x/y, width/height, zoom, dpi; the last two are unused
    /// (the renderer supplies the clock itself).
    pub camera: [f32; 8],
    pub stats: Arc<Mutex<GpuStats>>,
}
impl CallbackTrait for SceneCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let renderer = resources
            .get_mut::<Resources>()
            .expect("scene renderer installed");
        if renderer.recovery.device_unavailable() {
            return Vec::new();
        }
        let epoch = renderer.recovery.epoch();
        if renderer.surface_epoch != epoch {
            // eframe runs prepare before acquiring a surface. A failed acquisition
            // drops its encoder without submitting timestamp resolve/copy work.
            renderer.timing = crate::gpu_timing::GpuTiming::new(device, queue);
            renderer.surface_epoch = epoch;
            if let Ok(mut stats) = self.stats.lock() {
                stats.timestamp_errors += 1;
                stats.timestamp_diagnostics.surface_invalidations += 1;
            }
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.timestamp_status = renderer.timing_status;
            if let Some(timing) = &mut renderer.timing {
                timing.prepare(device, encoder, &mut stats);
            }
        }
        queue.write_buffer(
            &renderer.uniform,
            0,
            bytemuck::cast_slice(&uniform(&self.camera, clock())),
        );
        if renderer.key != Some(self.batch.key) {
            let started = Instant::now();
            let lists = [
                &self.batch.containers,
                &self.batch.edges,
                &self.batch.nodes,
                &self.batch.overlays,
            ];
            let all: Vec<Quad> = lists.iter().flat_map(|list| list.iter().copied()).collect();
            let bytes = bytemuck::cast_slice(&all);
            if bytes.len() > renderer.capacity {
                renderer.capacity = bytes.len().next_power_of_two();
                renderer.instances = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Retained scene instances"),
                    size: renderer.capacity as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            if !bytes.is_empty() {
                queue.write_buffer(&renderer.instances, 0, bytes);
            }
            let mut offset = 0u32;
            for (range, list) in renderer.ranges.iter_mut().zip(lists) {
                *range = offset..offset + list.len() as u32;
                offset = range.end;
            }
            renderer.key = Some(self.batch.key);
            if let Ok(mut stats) = self.stats.lock() {
                stats.upload_ms = started.elapsed().as_secs_f64() * 1000.0;
                stats.uploaded_bytes = bytes.len();
                stats.uploads += 1;
                stats.instances = all.len();
                // The node batch is drawn twice: its shadows, then the cards.
                stats.draw_calls = renderer
                    .ranges
                    .iter()
                    .filter(|range| !range.is_empty())
                    .count()
                    + usize::from(!renderer.ranges[NODES].is_empty());
            }
        }
        Vec::new()
    }
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let renderer = resources
            .get::<Resources>()
            .expect("scene renderer installed");
        if renderer.recovery.device_unavailable() {
            return;
        }
        if let Some(timing) = &renderer.timing {
            timing.begin(pass);
        }
        pass.set_bind_group(0, &renderer.bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.instances.slice(..));
        for (index, range) in renderer.ranges.iter().enumerate() {
            if range.is_empty() {
                continue;
            }
            if index == NODES {
                pass.set_pipeline(&renderer.shadow_pipeline);
                pass.draw(0..6, range.clone());
            }
            pass.set_pipeline(&renderer.pipeline);
            pass.draw(0..6, range.clone());
        }
        if let Some(timing) = &renderer.timing {
            timing.end(pass);
        }
    }
}
