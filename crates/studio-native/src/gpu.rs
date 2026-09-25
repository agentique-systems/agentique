//! Retained instanced scene pipeline. Four ordered batches, independent of widget count.
//!
//! Camera motion updates one uniform. Geometry uploads only when the presentation
//! generation/visibility/selection changes. Text uses the toolkit glyph atlas above
//! this pass; it is culled and selected by semantic LOD before shaping.
use bytemuck::{Pod, Zeroable};
use eframe::egui;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};
use std::{
    sync::{Arc, Mutex},
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
    /// Dash period / ink length in world units; zero period is solid.
    pub detail: [f32; 4],
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
}
fn color(value: egui::Color32) -> [f32; 4] {
    value.to_array().map(|v| v as f32 / 255.0)
}

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
}
struct Resources {
    pipeline: wgpu::RenderPipeline,
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
                min_binding_size: std::num::NonZeroU64::new(32),
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Scene pipeline"),
        bind_group_layouts: &[&layout],
        push_constant_ranges: &[],
    });
    let attributes =
        wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32x4];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Scene quads and routes"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Quad>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &attributes,
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
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
        multiview: None,
        cache: None,
    });
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Scene camera uniform"),
        contents: bytemuck::cast_slice(&[0.0f32; 8]),
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

pub struct SceneCallback {
    pub batch: Arc<Batch>,
    /// center x/y, width/height, zoom, dpi, reserved
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
            }
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.timestamp_status = renderer.timing_status;
            if let Some(timing) = &mut renderer.timing {
                timing.prepare(device, encoder, &mut stats);
            }
        }
        queue.write_buffer(&renderer.uniform, 0, bytemuck::cast_slice(&self.camera));
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
                stats.draw_calls = renderer
                    .ranges
                    .iter()
                    .filter(|range| !range.is_empty())
                    .count();
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
        pass.set_pipeline(&renderer.pipeline);
        pass.set_bind_group(0, &renderer.bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.instances.slice(..));
        for range in &renderer.ranges {
            if !range.is_empty() {
                pass.draw(0..6, range.clone());
            }
        }
        if let Some(timing) = &renderer.timing {
            timing.end(pass);
        }
    }
}
