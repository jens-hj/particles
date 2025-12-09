pub struct HadronRenderer {
    shell_pipeline: wgpu::RenderPipeline,
    bond_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl HadronRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        _camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hadron Renderer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/hadron.wgsl").into()),
        });

        // Bind group layout for hadron data
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Hadron Bind Group Layout"),
            entries: &[
                // Camera (Uniform) - Binding 0
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Hadrons (Storage) - Binding 1
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
                // Particles (Storage) - Binding 2
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
                // Counter (Storage) - Binding 3
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
            label: Some("Hadron Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout], // We include camera in this layout for simplicity
            push_constant_ranges: &[],
        });

        // --- SHELL PIPELINE (Instanced Quads) ---
        let shell_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Hadron Shell Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shell"),
                buffers: &[], // No vertex buffers, using vertex_index
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_shell"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Don't cull billboards
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Transparent shells don't write depth
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // --- BOND PIPELINE (Lines) ---
        let bond_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Hadron Bond Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_bond"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_bond"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Transparent lines don't write depth
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            shell_pipeline,
            bond_pipeline,
            bind_group_layout,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass,
        camera_buffer: &wgpu::Buffer,
        hadron_buffer: &wgpu::Buffer,
        particle_buffer: &wgpu::Buffer,
        hadron_count_buffer: &wgpu::Buffer,
        max_hadrons: u32,
        show_shells: bool,
        show_bonds: bool,
    ) {
        // Create bind group for this frame
        // Note: In a real engine, we would cache this or use a BindGroupAllocator
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hadron Render Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: hadron_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: hadron_count_buffer.as_entire_binding(),
                },
            ],
        });

        // Draw Shells
        if show_shells {
            render_pass.set_pipeline(&self.shell_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            // Draw 6 vertices (quad) per instance, max_hadrons instances
            // The shader will discard invalid instances
            render_pass.draw(0..6, 0..max_hadrons);
        }

        // Draw Bonds
        if show_bonds {
            render_pass.set_pipeline(&self.bond_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            // Draw 6 vertices per hadron (3 lines), 1 instance
            // The shader will discard invalid vertices
            render_pass.draw(0..(max_hadrons * 6), 0..1);
        }
    }
}
