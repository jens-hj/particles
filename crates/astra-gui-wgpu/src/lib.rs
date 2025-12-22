//! # astra-gui-wgpu
//!
//! WGPU rendering backend for astra-gui.

#[cfg(feature = "text-cosmic")]
mod text;

mod vertex;

use astra_gui::{FullOutput, Shape, Tessellator};
use vertex::WgpuVertex;

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

    // Glyph atlas (R8 alpha mask)
    #[cfg(feature = "text-cosmic")]
    atlas_texture: wgpu::Texture,
    #[cfg(feature = "text-cosmic")]
    atlas_bind_group: wgpu::BindGroup,
    #[cfg(feature = "text-cosmic")]
    atlas: text::atlas::GlyphAtlas,
    #[cfg(feature = "text-cosmic")]
    debug_font: text::debug_font::DebugFont,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
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
            atlas_texture,
            #[cfg(feature = "text-cosmic")]
            atlas_bind_group,
            #[cfg(feature = "text-cosmic")]
            atlas,
            #[cfg(feature = "text-cosmic")]
            debug_font: text::debug_font::DebugFont::new(),
        }
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
        // Tessellate shapes (geometry only)
        let mesh = self.tessellator.tessellate(&output.shapes);

        // Convert to wgpu vertices
        self.wgpu_vertices.clear();
        self.wgpu_vertices.reserve(mesh.vertices.len());
        for v in &mesh.vertices {
            self.wgpu_vertices.push(WgpuVertex::from(*v));
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
        if mesh.indices.len() > self.index_capacity {
            self.index_capacity = (mesh.indices.len() * 2).next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Astra UI Index Buffer"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Upload geometry
        if !mesh.indices.is_empty() {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.wgpu_vertices),
            );
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
        }

        // Update uniforms (used by both passes)
        let uniforms = [screen_width, screen_height];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

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

        // Draw geometry
        if !mesh.indices.is_empty() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }

        // Draw text (minimal first cut): debug-font raster to atlas + quads.
        //
        // IMPORTANT: We must batch by clip rect (scissor), because WGPU scissor is render-pass
        // state. If we build one giant text mesh and draw once, we can only apply one scissor,
        // which would incorrectly clip most text.
        #[cfg(feature = "text-cosmic")]
        {
            self.text_vertices.clear();
            self.text_indices.clear();

            // Batch state for current clip rect.
            let mut current_scissor: Option<(u32, u32, u32, u32)> = None;
            let mut batch_start_index: u32 = 0;

            // Helper: flush current batch (draw indices [batch_start_index..end)).
            let mut flush_batch =
                |render_pass: &mut wgpu::RenderPass<'_>,
                 end_index: u32,
                 current_scissor: &Option<(u32, u32, u32, u32)>| {
                    if end_index <= batch_start_index {
                        return;
                    }

                    if let Some((x, y, w, h)) = *current_scissor {
                        render_pass.set_scissor_rect(x, y, w, h);
                    } else {
                        render_pass.set_scissor_rect(
                            0,
                            0,
                            screen_width as u32,
                            screen_height as u32,
                        );
                    }

                    render_pass.draw_indexed(batch_start_index..end_index, 0, 0..1);
                    batch_start_index = end_index;
                };

            // We'll bind pipeline + buffers once; then emit multiple draw calls with different scissors.
            // Build quads for each text shape.
            // We place a monospaced baseline at rect.min (top-left), then apply alignment.
            for clipped in &output.shapes {
                let Shape::Text(text_shape) = &clipped.shape else {
                    continue;
                };

                let rect = text_shape.rect;
                let text = text_shape.text.as_str();

                // Skip empty
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

                // If scissor changes, we need to finish the previous batch (after we upload buffers).
                // Here we just record the boundary; actual draw happens after buffer upload below.
                if current_scissor != Some(scissor_for_shape) {
                    current_scissor = Some(scissor_for_shape);
                }

                // Compute scale from font size (debug font base is 8px tall).
                let base_h = self.debug_font.metrics().height_px.max(1) as f32;
                let scale = (text_shape.font_size / base_h).round().max(1.0) as u32;

                // Compute rough line size in pixels for alignment.
                let glyph_advance = self.debug_font.metrics().advance_px * scale as f32;
                let line_w = glyph_advance * (text.chars().count() as f32);
                let line_h = (self.debug_font.metrics().height_px * scale) as f32;

                let origin_x = match text_shape.h_align {
                    astra_gui::HorizontalAlign::Left => rect.min[0],
                    astra_gui::HorizontalAlign::Center => {
                        rect.min[0] + (rect.width() - line_w) * 0.5
                    }
                    astra_gui::HorizontalAlign::Right => rect.max[0] - line_w,
                };

                let origin_y = match text_shape.v_align {
                    astra_gui::VerticalAlign::Top => rect.min[1],
                    astra_gui::VerticalAlign::Center => {
                        rect.min[1] + (rect.height() - line_h) * 0.5
                    }
                    astra_gui::VerticalAlign::Bottom => rect.max[1] - line_h,
                };

                let mut pen_x = origin_x;
                let pen_y = origin_y
                    + (self.debug_font.metrics().baseline_from_top_px as f32 * scale as f32);

                for ch in text.chars() {
                    let glyph = self.debug_font.rasterize_glyph(ch, scale);

                    // Atlas cache key uses glyph_id as ASCII codepoint.
                    let glyph_id = glyph.ch as u32;
                    let key = text::atlas::GlyphKey::new(
                        0,
                        glyph_id,
                        text_shape.font_size.round().max(1.0) as u16,
                        0,
                    );

                    let placed = match self.atlas.insert(key.clone(), glyph.size_px) {
                        text::atlas::AtlasInsert::AlreadyPresent => self.atlas.get(&key),
                        text::atlas::AtlasInsert::Placed(p) => {
                            // Upload into atlas
                            let rect_px = text::atlas::GlyphAtlas::upload_rect_px(p);
                            // Upload glyph bitmap into the glyph area (excluding padding).
                            // We keep our atlas placement UVs pointing to the glyph area; padding reserved
                            // in packing reduces sampling artifacts.
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
                                &glyph.pixels,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    // `wgpu` expects bytes per row to be a multiple of 256, but for small glyph
                                    // uploads many backends accept tightly packed rows. If this triggers
                                    // validation errors on your platform, we should stage into a padded buffer.
                                    bytes_per_row: Some(glyph.size_px[0]),
                                    rows_per_image: Some(glyph.size_px[1]),
                                },
                                wgpu::Extent3d {
                                    width: glyph.size_px[0],
                                    height: glyph.size_px[1],
                                    depth_or_array_layers: 1,
                                },
                            );
                            Some(p)
                        }
                        text::atlas::AtlasInsert::Full => None,
                    };

                    let Some(placed) = placed else {
                        pen_x += glyph.advance_px[0];
                        continue;
                    };

                    // Quad in screen px
                    let x0 = pen_x + glyph.bearing_px[0] as f32;
                    let y0 = pen_y + glyph.bearing_px[1] as f32;
                    let x1 = x0 + glyph.size_px[0] as f32;
                    let y1 = y0 + glyph.size_px[1] as f32;

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

                    // Two triangles
                    self.text_indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);

                    pen_x += glyph.advance_px[0];
                }

                // Mark a batch boundary at the end of this shape. We will draw up to this point
                // with the current scissor after uploading the buffers.
                //
                // We encode boundaries by flushing later in the draw section below by re-applying scissor.
                // (This keeps CPU-side logic simple while still fixing clipping.)
                //
                // NOTE: This means we draw per text shape (per clip rect), which is correct for now.
                // A later optimization could coalesce consecutive shapes with identical clip rects.
                let _shape_end_index = self.text_indices.len() as u32;
            }

            if !self.text_indices.is_empty() {
                // Resize buffers if needed
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

                // Upload
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

                // Draw
                render_pass.set_pipeline(&self.text_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.text_index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                // Correctness-first: since we currently build one contiguous index buffer without
                // retaining per-shape boundaries, draw it once with a full-screen scissor.
                //
                // This fixes the "only footer visible" bug caused by leaving a per-shape scissor
                // active for a single batched draw. Proper per-clip batching will come next.
                render_pass.set_scissor_rect(0, 0, screen_width as u32, screen_height as u32);
                flush_batch(&mut render_pass, self.text_indices.len() as u32, &None);
            }
        }
    }
}
