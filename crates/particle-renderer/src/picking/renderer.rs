//! Picking renderer: draws per-entity IDs into an offscreen render target.
//!
//! This is the rendering/pipeline half of GPU picking. It is designed to be used together with
//! `crate::picking::GpuPicker` (which owns the offscreen texture + readback buffer).
//!
//! The picking pass writes a packed `u32` ID into an RGBA8 render target.
//!
//! ID encoding convention:
//! - 0                => "no hit" / background
//! - (particle_idx+1) => particle hit
//! - 0x8000_0000 | (hadron_idx+1) => hadron hit (top bit marks hadron class)
//!
//! Notes:
//! - This pass should be rendered with a depth buffer to respect occlusion.
//! - The pipeline expects the same camera uniform layout as the normal render shaders.
//! - The particle/hadron SSBO layouts match the existing WGSL shaders.

use crate::camera::{Camera, CameraUniform};

/// Runs an offscreen picking pass producing packed IDs in RGBA8.
pub struct PickingRenderer {
    particle_pipeline: wgpu::RenderPipeline,
    hadron_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,

    /// Depth texture view matching the current surface size (for occlusion in pick pass).
    depth_view: wgpu::TextureView,
    depth_format: wgpu::TextureFormat,

    /// Camera uniform buffer, shared across both picking pipelines.
    pub camera_buffer: wgpu::Buffer,

    /// The surface width/height we currently size the depth buffer to.
    width: u32,
    height: u32,
}

impl PickingRenderer {
    /// Create the picking renderer.
    ///
    /// `color_format` must match the pick target texture format (we currently use `Rgba8Unorm`).
    /// `depth_format` should generally match the main renderer (Depth32Float).
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Picking Camera Buffer"),
            // Uniforms are validated using WGSL layout rules (16-byte aligned). Round up the allocation
            // so `as_entire_binding()` meets any 16-byte-rounded minimum.
            size: {
                let sz = std::mem::size_of::<CameraUniform>() as u64;
                ((sz + 15) / 16) * 16
            },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth_view = create_depth_texture_view(device, depth_format, width, height);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Picking Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/picking.wgsl").into()),
        });

        // Bind group layout:
        // 0: camera uniform
        // 1: particles storage
        // 2: hadrons storage
        // 3: hadron counter storage
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Picking Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new({
                                let sz = std::mem::size_of::<CameraUniform>() as u64;
                                // Uniforms follow 16-byte alignment rules; round up so validation matches WGSL layout.
                                ((sz + 15) / 16) * 16
                            })
                            .unwrap(),
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Picking Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        };

        let depth_stencil = Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Particle Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_pick_particle"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_pick_particle"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    // We want the nearest fragment to win and write its ID; no blending.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive,
            depth_stencil: depth_stencil.clone(),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let hadron_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Hadron Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_pick_hadron"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_pick_hadron"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive,
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            particle_pipeline,
            hadron_pipeline,
            bind_group_layout,
            depth_view,
            depth_format,
            camera_buffer,
            width,
            height,
        }
    }

    /// Resize depth targets as needed (pick target itself is owned by `GpuPicker`).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;
        self.depth_view = create_depth_texture_view(device, self.depth_format, width, height);
    }

    /// Render IDs into `target_view`. Caller selects which pixel to read out later.
    ///
    /// `particle_count` should be the total particle instances to render.
    /// `max_hadrons` is the maximum hadron instances to render (shader discards invalid/out-of-range).
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera: &Camera,
        particle_buffer: &wgpu::Buffer,
        hadron_buffer: &wgpu::Buffer,
        hadron_count_buffer: &wgpu::Buffer,
        particle_count: u32,
        max_hadrons: u32,
        particle_size: f32,
        time: f32,
        lod_shell_fade_start: f32,
        lod_shell_fade_end: f32,
        lod_bound_hadron_fade_start: f32,
        lod_bound_hadron_fade_end: f32,
        lod_bond_fade_start: f32,
        lod_bond_fade_end: f32,
        lod_quark_fade_start: f32,
        lod_quark_fade_end: f32,
        lod_nucleus_fade_start: f32,
        lod_nucleus_fade_end: f32,
    ) {
        // Update camera uniform. We reuse the same struct as regular rendering.
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.to_uniform(
                particle_size,
                time,
                lod_shell_fade_start,
                lod_shell_fade_end,
                lod_bound_hadron_fade_start,
                lod_bound_hadron_fade_end,
                lod_bond_fade_start,
                lod_bond_fade_end,
                lod_quark_fade_start,
                lod_quark_fade_end,
                lod_nucleus_fade_start,
                lod_nucleus_fade_end,
            )]),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Picking Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: hadron_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: hadron_count_buffer.as_entire_binding(),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Picking Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Background ID = 0
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Render particles first, then hadrons, letting depth determine what is visible.
        // Depending on your desired UI, you may reverse this.
        pass.set_bind_group(0, &bind_group, &[]);

        pass.set_pipeline(&self.particle_pipeline);
        pass.draw(0..6, 0..particle_count);

        pass.set_pipeline(&self.hadron_pipeline);
        pass.draw(0..6, 0..max_hadrons);
    }
}

fn create_depth_texture_view(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Picking Depth Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    tex.create_view(&wgpu::TextureViewDescriptor::default())
}
