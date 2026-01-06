pub struct NucleusRenderer {
    shell_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl NucleusRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        _camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Nucleus Renderer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/nucleus.wgsl").into()),
        });

        // Bind group layout for nucleus data
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Nucleus Bind Group Layout"),
            entries: &[
                // Camera (Uniform) - Binding 0
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new({
                                let sz = std::mem::size_of::<crate::camera::CameraUniform>() as u64;
                                // Uniforms use 16-byte alignment rules; round up so validation matches WGSL layout.
                                ((sz + 15) / 16) * 16
                            })
                            .unwrap(),
                        ),
                    },
                    count: None,
                },
                // Nuclei (Storage) - Binding 1
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
                // Counter (Storage) - Binding 2
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Nucleus Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Shell pipeline (Instanced Quads for nucleus shells)
        let shell_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Nucleus Shell Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shell"),
                buffers: &[],
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
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            shell_pipeline,
            bind_group_layout,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass,
        camera_buffer: &wgpu::Buffer,
        nucleus_buffer: &wgpu::Buffer,
        nucleus_count_buffer: &wgpu::Buffer,
        max_nuclei: u32,
        show_shells: bool,
    ) {
        if !show_shells || max_nuclei == 0 {
            return;
        }

        // Create bind group for this frame
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Nucleus Render Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nucleus_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: nucleus_count_buffer.as_entire_binding(),
                },
            ],
        });

        render_pass.set_pipeline(&self.shell_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);

        // Each nucleus shell is rendered as a quad (6 vertices)
        render_pass.draw(0..6, 0..max_nuclei);
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}
