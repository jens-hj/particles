//! # astra-gui-wgpu
//!
//! WGPU rendering backend for astra-gui.

mod events;
mod input;
mod instance;
mod interactive_state;

#[cfg(feature = "text-cosmic")]
mod text;

mod vertex;

pub use events::*;
pub use input::*;
pub use interactive_state::*;

// Re-export keyboard and mouse types for use in interactive components
pub use winit::event::MouseButton;
pub use winit::keyboard::{Key, NamedKey};

use astra_gui::{FullOutput, Shape, Tessellator};
use instance::RectInstance;
use vertex::WgpuVertex;

/// Rendering mode for rectangles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// Use SDF (Signed Distance Field) rendering for analytical anti-aliasing.
    /// Best quality, especially for strokes and rounded corners.
    Sdf,
    /// Use mesh tessellation for rendering.
    /// More compatible but lower quality anti-aliasing.
    Mesh,
    /// Automatically choose based on shape complexity (currently defaults to SDF).
    Auto,
}

#[cfg(feature = "text-cosmic")]
use astra_gui_text as gui_text;
#[cfg(feature = "text-cosmic")]
use gui_text::TextEngine;

/// A draw call with scissor rect for clipped rendering.
#[derive(Clone, Copy, Debug)]
struct ClippedDraw {
    scissor: (u32, u32, u32, u32),
    index_start: u32,
    index_end: u32,
}

const INITIAL_VERTEX_CAPACITY: usize = 1024;
const INITIAL_INDEX_CAPACITY: usize = 2048;

#[cfg(feature = "text-cosmic")]
const INITIAL_TEXT_VERTEX_CAPACITY: usize = 4096;
#[cfg(feature = "text-cosmic")]
const INITIAL_TEXT_INDEX_CAPACITY: usize = 8192;

#[cfg(feature = "text-cosmic")]
const ATLAS_SIZE_PX: u32 = 1024;
#[cfg(feature = "text-cosmic")]
const ATLAS_PADDING_PX: u32 = 1;

/// WGPU renderer for astra-gui
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    tessellator: Tessellator,
    vertex_capacity: usize,
    index_capacity: usize,
    wgpu_vertices: Vec<WgpuVertex>,

    // Performance optimization: track previous frame sizes to pre-allocate buffers
    last_frame_vertex_count: usize,
    last_frame_index_count: usize,

    // Rendering mode configuration
    render_mode: RenderMode,

    // SDF rendering pipeline (analytic anti-aliasing)
    sdf_pipeline: wgpu::RenderPipeline,
    sdf_instance_buffer: wgpu::Buffer,
    sdf_instance_capacity: usize,
    sdf_instances: Vec<RectInstance>,
    sdf_quad_vertex_buffer: wgpu::Buffer,
    sdf_quad_index_buffer: wgpu::Buffer,
    last_frame_sdf_instance_count: usize,

    #[cfg(feature = "text-cosmic")]
    text_pipeline: wgpu::RenderPipeline,
    #[cfg(feature = "text-cosmic")]
    text_vertex_buffer: wgpu::Buffer,
    #[cfg(feature = "text-cosmic")]
    text_index_buffer: wgpu::Buffer,
    #[cfg(feature = "text-cosmic")]
    text_vertex_capacity: usize,
    #[cfg(feature = "text-cosmic")]
    text_index_capacity: usize,
    #[cfg(feature = "text-cosmic")]
    text_vertices: Vec<text::vertex::TextVertex>,
    #[cfg(feature = "text-cosmic")]
    text_indices: Vec<u32>,
    #[cfg(feature = "text-cosmic")]
    last_frame_text_vertex_count: usize,
    #[cfg(feature = "text-cosmic")]
    last_frame_text_index_count: usize,

    // Glyph atlas (R8 alpha mask)
    #[cfg(feature = "text-cosmic")]
    atlas_texture: wgpu::Texture,
    #[cfg(feature = "text-cosmic")]
    atlas_bind_group: wgpu::BindGroup,
    #[cfg(feature = "text-cosmic")]
    atlas: text::atlas::GlyphAtlas,

    // Backend-agnostic text shaping/raster engine (Inter via astra-gui-fonts).
    #[cfg(feature = "text-cosmic")]
    text_engine: gui_text::Engine,
}

impl Renderer {
    /// Create a new renderer with the default render mode (Auto/SDF)
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        Self::with_render_mode(device, surface_format, RenderMode::Auto)
    }

    /// Create a new renderer with a specific render mode
    pub fn with_render_mode(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        render_mode: RenderMode,
    ) -> Self {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Astra UI Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()),
        });

        // Create uniform buffer (screen size)
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Astra UI Uniform Buffer"),
            size: std::mem::size_of::<[f32; 2]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout (globals)
        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Astra UI Globals Bind Group Layout"),
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

        // Create bind group (globals)
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Astra UI Globals Bind Group"),
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout (geometry)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Astra UI Pipeline Layout"),
            bind_group_layouts: &[&globals_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline (geometry)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Astra UI Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[WgpuVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
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
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create initial buffers (geometry)
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Astra UI Vertex Buffer"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<WgpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Astra UI Index Buffer"),
            size: (INITIAL_INDEX_CAPACITY * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create SDF pipeline and buffers
        let sdf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Astra UI SDF Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_sdf.wgsl").into()),
        });

        let sdf_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Astra UI SDF Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sdf_shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer: unit quad
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    },
                    // Instance buffer
                    RectInstance::desc(),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sdf_shader,
                entry_point: Some("fs_main"),
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
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Unit quad vertices: [-1, -1] to [1, 1]
        let quad_vertices: &[[f32; 2]] = &[
            [-1.0, -1.0], // bottom-left
            [1.0, -1.0],  // bottom-right
            [1.0, 1.0],   // top-right
            [-1.0, 1.0],  // top-left
        ];
        let quad_indices: &[u32] = &[0, 1, 2, 0, 2, 3];

        let sdf_quad_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Astra UI SDF Quad Vertex Buffer"),
            size: (quad_vertices.len() * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        sdf_quad_vertex_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(quad_vertices));
        sdf_quad_vertex_buffer.unmap();

        let sdf_quad_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Astra UI SDF Quad Index Buffer"),
            size: (quad_indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        sdf_quad_index_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(quad_indices));
        sdf_quad_index_buffer.unmap();

        const INITIAL_SDF_INSTANCE_CAPACITY: usize = 256;
        let sdf_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Astra UI SDF Instance Buffer"),
            size: (INITIAL_SDF_INSTANCE_CAPACITY * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[cfg(feature = "text-cosmic")]
        let (
            text_pipeline,
            text_vertex_buffer,
            text_index_buffer,
            atlas_texture,
            atlas_bind_group,
            atlas,
        ) = {
            // Load text shader
            let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Astra UI Text Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
            });

            // Atlas texture (R8)
            let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Astra UI Glyph Atlas"),
                size: wgpu::Extent3d {
                    width: ATLAS_SIZE_PX,
                    height: ATLAS_SIZE_PX,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Astra UI Glyph Atlas Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                // Debug atlas is a nearest-neighbor bitmap; keep sampling nearest to avoid
                // filter smearing and edge artifacts at small sizes.
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let atlas_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Astra UI Text Atlas Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

            let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Astra UI Text Atlas Bind Group"),
                layout: &atlas_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                    },
                ],
            });

            // Pipeline layout (text): globals + atlas
            let text_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Astra UI Text Pipeline Layout"),
                    bind_group_layouts: &[&globals_bind_group_layout, &atlas_bind_group_layout],
                    push_constant_ranges: &[],
                });

            let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Astra UI Text Pipeline"),
                layout: Some(&text_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &text_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[text::vertex::TextVertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &text_shader,
                    entry_point: Some("fs_main"),
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
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

            let text_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Astra UI Text Vertex Buffer"),
                size: (INITIAL_TEXT_VERTEX_CAPACITY
                    * std::mem::size_of::<text::vertex::TextVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let text_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Astra UI Text Index Buffer"),
                size: (INITIAL_TEXT_INDEX_CAPACITY * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let atlas =
                text::atlas::GlyphAtlas::new(ATLAS_SIZE_PX, ATLAS_SIZE_PX, ATLAS_PADDING_PX);

            (
                text_pipeline,
                text_vertex_buffer,
                text_index_buffer,
                atlas_texture,
                atlas_bind_group,
                atlas,
            )
        };

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            tessellator: Tessellator::new(),
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            index_capacity: INITIAL_INDEX_CAPACITY,
            wgpu_vertices: Vec::new(),
            last_frame_vertex_count: 0,
            last_frame_index_count: 0,

            render_mode,

            sdf_pipeline,
            sdf_instance_buffer,
            sdf_instance_capacity: INITIAL_SDF_INSTANCE_CAPACITY,
            sdf_instances: Vec::new(),
            sdf_quad_vertex_buffer,
            sdf_quad_index_buffer,
            last_frame_sdf_instance_count: 0,

            #[cfg(feature = "text-cosmic")]
            text_pipeline,
            #[cfg(feature = "text-cosmic")]
            text_vertex_buffer,
            #[cfg(feature = "text-cosmic")]
            text_index_buffer,
            #[cfg(feature = "text-cosmic")]
            text_vertex_capacity: INITIAL_TEXT_VERTEX_CAPACITY,
            #[cfg(feature = "text-cosmic")]
            text_index_capacity: INITIAL_TEXT_INDEX_CAPACITY,
            #[cfg(feature = "text-cosmic")]
            text_vertices: Vec::new(),
            #[cfg(feature = "text-cosmic")]
            text_indices: Vec::new(),
            #[cfg(feature = "text-cosmic")]
            last_frame_text_vertex_count: 0,
            #[cfg(feature = "text-cosmic")]
            last_frame_text_index_count: 0,
            #[cfg(feature = "text-cosmic")]
            atlas_texture,
            #[cfg(feature = "text-cosmic")]
            atlas_bind_group,
            #[cfg(feature = "text-cosmic")]
            atlas,
            #[cfg(feature = "text-cosmic")]
            text_engine: gui_text::Engine::new_default(),
        }
    }

    /// Get the current render mode
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// Set the render mode
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        screen_width: f32,
        screen_height: f32,
        output: &FullOutput,
    ) {
        // Separate shapes into SDF-renderable and tessellated.
        // SDF rendering is used for simple shapes (currently: all fills, simple strokes).
        // OPTIMIZATION: Pre-allocate based on previous frame to reduce allocations
        self.sdf_instances.clear();
        self.sdf_instances
            .reserve(self.last_frame_sdf_instance_count);

        self.wgpu_vertices.clear();
        self.wgpu_vertices.reserve(self.last_frame_vertex_count);

        let mut indices: Vec<u32> = Vec::new();
        indices.reserve(self.last_frame_index_count);

        let mut geometry_draws: Vec<ClippedDraw> = Vec::new();

        for clipped in &output.shapes {
            let Shape::Rect(rect) = &clipped.shape else {
                continue;
            };

            // Decide whether to use SDF or mesh rendering based on render_mode
            let use_sdf = match self.render_mode {
                RenderMode::Sdf => true,
                RenderMode::Mesh => false,
                RenderMode::Auto => true, // Default to SDF for best quality
            };

            if use_sdf {
                // Use SDF rendering (analytical anti-aliasing)
                self.sdf_instances.push(RectInstance::from(rect));
            } else {
                // Use mesh tessellation - collect for batch processing
                // (Tessellator processes all shapes at once)
            }
        }

        // Process mesh shapes if using Mesh render mode
        if self.render_mode == RenderMode::Mesh {
            // Tessellate all shapes using mesh rendering
            let mesh = self.tessellator.tessellate(&output.shapes);

            if !mesh.vertices.is_empty() {
                // Convert mesh vertices to WgpuVertex format
                for vertex in &mesh.vertices {
                    self.wgpu_vertices.push(WgpuVertex {
                        pos: vertex.pos,
                        color: [
                            (vertex.color[0] * 255.0).round().clamp(0.0, 255.0) as u8,
                            (vertex.color[1] * 255.0).round().clamp(0.0, 255.0) as u8,
                            (vertex.color[2] * 255.0).round().clamp(0.0, 255.0) as u8,
                            (vertex.color[3] * 255.0).round().clamp(0.0, 255.0) as u8,
                        ],
                    });
                }

                // Copy indices
                indices.extend_from_slice(&mesh.indices);

                // Create draw calls with scissor rects
                for clipped in &output.shapes {
                    let sc_min_x = clipped.clip_rect.min[0].max(0.0).floor() as i32;
                    let sc_min_y = clipped.clip_rect.min[1].max(0.0).floor() as i32;
                    let sc_max_x = clipped.clip_rect.max[0].min(screen_width).ceil() as i32;
                    let sc_max_y = clipped.clip_rect.max[1].min(screen_height).ceil() as i32;

                    let sc_w = (sc_max_x - sc_min_x).max(0) as u32;
                    let sc_h = (sc_max_y - sc_min_y).max(0) as u32;

                    if sc_w > 0 && sc_h > 0 {
                        // Use the entire mesh for now (TODO: track per-shape indices)
                        geometry_draws.push(ClippedDraw {
                            scissor: (sc_min_x as u32, sc_min_y as u32, sc_w, sc_h),
                            index_start: 0,
                            index_end: indices.len() as u32,
                        });
                    }
                }
            }
        }

        // Resize vertex buffer if needed
        if self.wgpu_vertices.len() > self.vertex_capacity {
            self.vertex_capacity = (self.wgpu_vertices.len() * 2).next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Astra UI Vertex Buffer"),
                size: (self.vertex_capacity * std::mem::size_of::<WgpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Resize index buffer if needed
        if indices.len() > self.index_capacity {
            self.index_capacity = (indices.len() * 2).next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Astra UI Index Buffer"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Upload geometry
        if !indices.is_empty() {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.wgpu_vertices),
            );
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        }

        // Update uniforms (used by both passes)
        let uniforms = [screen_width, screen_height];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        // Upload SDF instances
        if !self.sdf_instances.is_empty() {
            // Resize instance buffer if needed
            if self.sdf_instances.len() > self.sdf_instance_capacity {
                self.sdf_instance_capacity = (self.sdf_instances.len() * 2).next_power_of_two();
                self.sdf_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Astra UI SDF Instance Buffer"),
                    size: (self.sdf_instance_capacity * std::mem::size_of::<RectInstance>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }

            queue.write_buffer(
                &self.sdf_instance_buffer,
                0,
                bytemuck::cast_slice(&self.sdf_instances),
            );
        }

        // Render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Astra UI Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Preserve existing content
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw SDF instances (analytic anti-aliasing)
        if !self.sdf_instances.is_empty() {
            render_pass.set_pipeline(&self.sdf_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sdf_quad_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.sdf_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.sdf_quad_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..6, 0, 0..self.sdf_instances.len() as u32);
        }

        // Draw geometry with batched scissor clipping
        // OPTIMIZATION: Batch consecutive draws with the same scissor rect to reduce draw calls
        if !geometry_draws.is_empty() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            // Batch consecutive draws with the same scissor rect
            let mut current_scissor = geometry_draws[0].scissor;
            let mut batch_start = geometry_draws[0].index_start;
            let mut batch_end = geometry_draws[0].index_end;

            for draw in &geometry_draws[1..] {
                if draw.scissor == current_scissor && draw.index_start == batch_end {
                    // Extend current batch (consecutive indices, same scissor)
                    batch_end = draw.index_end;
                } else {
                    // Flush current batch
                    let (x, y, w, h) = current_scissor;
                    render_pass.set_scissor_rect(x, y, w, h);
                    render_pass.draw_indexed(batch_start..batch_end, 0, 0..1);

                    // Start new batch
                    current_scissor = draw.scissor;
                    batch_start = draw.index_start;
                    batch_end = draw.index_end;
                }
            }

            // Flush final batch
            let (x, y, w, h) = current_scissor;
            render_pass.set_scissor_rect(x, y, w, h);
            render_pass.draw_indexed(batch_start..batch_end, 0, 0..1);

            // Reset scissor to full screen
            render_pass.set_scissor_rect(0, 0, screen_width as u32, screen_height as u32);
        }

        // Draw text: shape (backend-agnostic) + rasterize (backend-agnostic) + atlas upload + quads.
        //
        // IMPORTANT: scissor/clipping is render-pass state. To respect `ClippedShape::clip_rect`,
        // we must issue separate draw calls for distinct clip rect ranges.
        #[cfg(feature = "text-cosmic")]
        {
            // OPTIMIZATION: Pre-allocate based on previous frame to reduce allocations
            self.text_vertices.clear();
            self.text_vertices
                .reserve(self.last_frame_text_vertex_count);

            self.text_indices.clear();
            self.text_indices.reserve(self.last_frame_text_index_count);

            let mut draws: Vec<ClippedDraw> = Vec::new();

            for clipped in &output.shapes {
                let Shape::Text(text_shape) = &clipped.shape else {
                    continue;
                };

                let rect = text_shape.rect;
                let text = text_shape.text.as_str();

                if text.is_empty() {
                    continue;
                }

                // Compute the scissor rect for this shape, clamped to framebuffer bounds.
                let sc_min_x = clipped.clip_rect.min[0].max(0.0).floor() as i32;
                let sc_min_y = clipped.clip_rect.min[1].max(0.0).floor() as i32;
                let sc_max_x = clipped.clip_rect.max[0].min(screen_width).ceil() as i32;
                let sc_max_y = clipped.clip_rect.max[1].min(screen_height).ceil() as i32;

                let sc_w = (sc_max_x - sc_min_x).max(0) as u32;
                let sc_h = (sc_max_y - sc_min_y).max(0) as u32;

                if sc_w == 0 || sc_h == 0 {
                    continue;
                }

                let scissor_for_shape = (sc_min_x as u32, sc_min_y as u32, sc_w, sc_h);

                // Start of this shape's indices in the final index buffer.
                let index_start = self.text_indices.len() as u32;

                // Shape + placement (backend-agnostic).
                let (shaped, placement) = self.text_engine.shape_line(gui_text::ShapeLineRequest {
                    text,
                    rect,
                    font_px: text_shape.font_size,
                    h_align: text_shape.h_align,
                    v_align: text_shape.v_align,
                    family: None,
                });

                for g in &shaped.glyphs {
                    let Some(bitmap) = self.text_engine.rasterize_glyph(g.key) else {
                        continue;
                    };

                    // Map backend-agnostic `GlyphKey` to the atlas key used by this backend.
                    let key = text::atlas::GlyphKey::new(
                        bitmap.key.font_id.0,
                        bitmap.key.glyph_id,
                        bitmap.key.px_size,
                        bitmap.key.subpixel_x_64 as u16,
                    );

                    let placed = match self.atlas.insert(key.clone(), bitmap.size_px) {
                        text::atlas::AtlasInsert::AlreadyPresent => self.atlas.get(&key),
                        text::atlas::AtlasInsert::Placed(p) => {
                            let rect_px = text::atlas::GlyphAtlas::upload_rect_px(p);
                            let pad = p.padding_px;
                            queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &self.atlas_texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d {
                                        x: rect_px.min.x + pad,
                                        y: rect_px.min.y + pad,
                                        z: 0,
                                    },
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &bitmap.pixels,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(bitmap.size_px[0]),
                                    rows_per_image: Some(bitmap.size_px[1]),
                                },
                                wgpu::Extent3d {
                                    width: bitmap.size_px[0],
                                    height: bitmap.size_px[1],
                                    depth_or_array_layers: 1,
                                },
                            );
                            Some(p)
                        }
                        text::atlas::AtlasInsert::Full => None,
                    };

                    let Some(placed) = placed else {
                        continue;
                    };

                    // Quad in screen px (origin from placement + shaped glyph offset).
                    let x0 = placement.origin_px[0] + g.x_px + bitmap.bearing_px[0] as f32;
                    let y0 = placement.origin_px[1] + g.y_px + bitmap.bearing_px[1] as f32;
                    let x1 = x0 + bitmap.size_px[0] as f32;
                    let y1 = y0 + bitmap.size_px[1] as f32;

                    let color = [
                        text_shape.color.r,
                        text_shape.color.g,
                        text_shape.color.b,
                        text_shape.color.a,
                    ];
                    let uv = placed.uv;

                    let base = self.text_vertices.len() as u32;
                    self.text_vertices.push(text::vertex::TextVertex::new(
                        [x0, y0],
                        [uv.min[0], uv.min[1]],
                        color,
                    ));
                    self.text_vertices.push(text::vertex::TextVertex::new(
                        [x1, y0],
                        [uv.max[0], uv.min[1]],
                        color,
                    ));
                    self.text_vertices.push(text::vertex::TextVertex::new(
                        [x1, y1],
                        [uv.max[0], uv.max[1]],
                        color,
                    ));
                    self.text_vertices.push(text::vertex::TextVertex::new(
                        [x0, y1],
                        [uv.min[0], uv.max[1]],
                        color,
                    ));

                    self.text_indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                let index_end = self.text_indices.len() as u32;
                if index_end > index_start {
                    draws.push(ClippedDraw {
                        scissor: scissor_for_shape,
                        index_start,
                        index_end,
                    });
                }
            }

            if !draws.is_empty() {
                if self.text_vertices.len() > self.text_vertex_capacity {
                    self.text_vertex_capacity = (self.text_vertices.len() * 2).next_power_of_two();
                    self.text_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Astra UI Text Vertex Buffer"),
                        size: (self.text_vertex_capacity
                            * std::mem::size_of::<text::vertex::TextVertex>())
                            as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                }

                if self.text_indices.len() > self.text_index_capacity {
                    self.text_index_capacity = (self.text_indices.len() * 2).next_power_of_two();
                    self.text_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Astra UI Text Index Buffer"),
                        size: (self.text_index_capacity * std::mem::size_of::<u32>()) as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                }

                queue.write_buffer(
                    &self.text_vertex_buffer,
                    0,
                    bytemuck::cast_slice(&self.text_vertices),
                );
                queue.write_buffer(
                    &self.text_index_buffer,
                    0,
                    bytemuck::cast_slice(&self.text_indices),
                );

                render_pass.set_pipeline(&self.text_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.text_index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                // OPTIMIZATION: Batch consecutive draws with the same scissor rect
                let mut current_scissor = draws[0].scissor;
                let mut batch_start = draws[0].index_start;
                let mut batch_end = draws[0].index_end;

                for draw in &draws[1..] {
                    if draw.scissor == current_scissor && draw.index_start == batch_end {
                        // Extend current batch
                        batch_end = draw.index_end;
                    } else {
                        // Flush current batch
                        let (x, y, w, h) = current_scissor;
                        render_pass.set_scissor_rect(x, y, w, h);
                        render_pass.draw_indexed(batch_start..batch_end, 0, 0..1);

                        // Start new batch
                        current_scissor = draw.scissor;
                        batch_start = draw.index_start;
                        batch_end = draw.index_end;
                    }
                }

                // Flush final batch
                let (x, y, w, h) = current_scissor;
                render_pass.set_scissor_rect(x, y, w, h);
                render_pass.draw_indexed(batch_start..batch_end, 0, 0..1);

                render_pass.set_scissor_rect(0, 0, screen_width as u32, screen_height as u32);
            }

            // Update frame tracking for next frame's pre-allocation
            self.last_frame_text_vertex_count = self.text_vertices.len();
            self.last_frame_text_index_count = self.text_indices.len();
        }

        // Update frame tracking for geometry buffers
        self.last_frame_vertex_count = self.wgpu_vertices.len();
        self.last_frame_index_count = indices.len();
        self.last_frame_sdf_instance_count = self.sdf_instances.len();
    }
}
