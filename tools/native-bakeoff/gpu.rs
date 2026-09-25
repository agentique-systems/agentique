use crate::wgpu;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct View {
    pub size: [f32; 2],
    pub zoom: f32,
    pub selected: f32,
    pub pan: [f32; 2],
    pub dark: f32,
    pub pad: f32,
}
pub struct Gpu {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform: wgpu::Buffer,
    pub bind: wgpu::BindGroup,
}
impl Gpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bakeoff-camera"),
            contents: bytemuck::bytes_of(&View::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shared-semantic-scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
        });
        #[cfg(not(feature = "wgpu30"))]
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        #[cfg(feature = "wgpu30")]
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            #[cfg(not(feature = "wgpu30"))]
            multiview: None,
            #[cfg(feature = "wgpu30")]
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            uniform,
            bind,
        }
    }
    pub fn update(&self, queue: &wgpu::Queue, view: View) {
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&view));
    }
    pub fn paint(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..6, 0..3000);
    }
}
