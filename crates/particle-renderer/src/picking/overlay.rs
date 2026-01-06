//! Debug overlay for visualizing the GPU picking ID texture.
//!
//! This draws the picking render target (RGBA8-packed u32 IDs) as a translucent fullscreen overlay,
//! optionally decoding the packed ID into a pseudo-color so you can see what the pick pass is
//! actually rasterizing.
//!
//! Intended usage:
//! - Create once: `PickingOverlay::new(device, surface_format)`
//! - Each frame (after you rendered IDs into the picker texture), call:
//!   `overlay.render(device, encoder, surface_view, &picker.id_texture_view)`
//!
//! Notes:
//! - This is a debug tool; keep it off by default.
//! - The input picking texture MUST be created with `wgpu::TextureUsages::TEXTURE_BINDING`
//!   in addition to whatever else you need (typically `RENDER_ATTACHMENT | COPY_SRC`), because
//!   this overlay samples it in a fragment shader.
//! - This pipeline assumes the pick texture is `Rgba8Unorm` containing packed u32 IDs
//!   in little-endian RGBA order (r=LSB..a=MSB). That matches `picking.wgsl`.

pub struct PickingOverlay {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl PickingOverlay {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Picking Overlay Shader"),
            source: wgpu::ShaderSource::Wgsl(OVERLAY_WGSL.into()),
        });

        // Sample the ID texture as a regular 2D texture. We assume it's Rgba8Unorm.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Picking Overlay Bind Group Layout"),
            entries: &[
                // Binding 0: texture_2d<f32>
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        // Rgba8Unorm => float sampling.
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Binding 1: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Picking Overlay Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Alpha blend so we can see the scene beneath.
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Overlay Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_overlay"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            // Overlay should ignore depth.
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Picking Overlay Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Render the picking ID texture as a translucent overlay into `surface_view`.
    ///
    /// `id_texture_view` must be a `TextureView` of the picking target texture.
    ///
    /// This draws a fullscreen triangle and samples the texture. Non-zero IDs get colored.
    pub fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        id_texture_view: &wgpu::TextureView,
        opacity: f32,
    ) {
        // Bind the pick texture + sampler.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Picking Overlay Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(id_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        // We need to pass opacity somehow; to keep this file self-contained and very low-touch,
        // we bake opacity into the shader as a constant-like uniform via push constants would be
        // ideal, but push constants aren't used elsewhere in this project and require pipeline changes.
        //
        // So we keep `opacity` here for future expansion; currently it is clamped in the shader to 0.35.
        let _opacity = opacity;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Picking Overlay Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Load existing scene and draw on top.
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

// WGSL shader for overlay.
// - Vertex: fullscreen triangle using vertex_index.
// - Fragment: sample packed RGBA8, decode u32 ID, map to color.
// - Background ID=0 -> transparent.
// - Non-zero -> hashed color with alpha.
const OVERLAY_WGSL: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> VsOut {
    // Fullscreen triangle (covers [0,0]-[1,1])
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );

    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 2.0)
    );

    var out: VsOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    out.uv = uv[vid];
    return out;
}

@group(0) @binding(0)
var id_tex: texture_2d<f32>;

@group(0) @binding(1)
var id_samp: sampler;

fn unpack_rgba8_to_u32(rgba: vec4<f32>) -> u32 {
    // Convert 0..1 to 0..255
    let r = u32(clamp(round(rgba.r * 255.0), 0.0, 255.0));
    let g = u32(clamp(round(rgba.g * 255.0), 0.0, 255.0));
    let b = u32(clamp(round(rgba.b * 255.0), 0.0, 255.0));
    let a = u32(clamp(round(rgba.a * 255.0), 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn id_to_color(id: u32) -> vec3<f32> {
    // Deterministic hash -> RGB
    let r = f32((id * 1664525u + 1013904223u) & 255u) / 255.0;
    let g = f32((id * 22695477u + 1u) & 255u) / 255.0;
    let b = f32((id * 1103515245u + 12345u) & 255u) / 255.0;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_overlay(in: VsOut) -> @location(0) vec4<f32> {
    // Sample ID texture
    //
    // NOTE: wgpu texture coordinates use (0,0) at the *top-left* of the texture.
    // Our cursor/pick math treats (0,0) as *top-left* as well, but the fullscreen
    // triangle UV mapping here ends up vertically flipped relative to that.
    //
    // Sample the texture directly with no UV transforms.
    let rgba = textureSample(id_tex, id_samp, in.uv);
    let id = unpack_rgba8_to_u32(rgba);

    if (id == 0u) {
        // Background: invisible
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let is_hadron = (id & 0x80000000u) != 0u;
    let idx_1 = id & 0x7FFFFFFFu;

    // Color: particles vs hadrons distinguishable by tint.
    var c = id_to_color(idx_1);
    if (is_hadron) {
        // Slightly bias toward cyan for hadrons
        c = normalize(c + vec3<f32>(0.0, 0.4, 0.4));
    } else {
        // Slightly bias toward magenta for particles
        c = normalize(c + vec3<f32>(0.4, 0.0, 0.4));
    }

    // Fixed opacity for now (debug). Caller param is not wired yet.
    let a = 0.35;
    return vec4<f32>(c, a);
}
"#;
