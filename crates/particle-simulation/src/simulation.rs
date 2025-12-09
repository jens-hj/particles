//! GPU-based particle simulation manager

use bytemuck::{Pod, Zeroable};
use particle_physics::{Hadron, Particle};
use wgpu::util::DeviceExt;

/// Force accumulator structure (matches WGSL)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Force {
    force: [f32; 3],
    _padding: f32,
}

/// GPU-based particle physics simulation
pub struct ParticleSimulation {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Buffers
    particle_buffer: wgpu::Buffer,
    _force_buffer: wgpu::Buffer,
    hadron_buffer: wgpu::Buffer,
    hadron_count_buffer: wgpu::Buffer,

    // Compute pipelines
    force_pipeline: wgpu::ComputePipeline,
    integrate_pipeline: wgpu::ComputePipeline,
    hadron_pipeline: wgpu::ComputePipeline,

    // Bind groups
    force_bind_group: wgpu::BindGroup,
    integrate_bind_group: wgpu::BindGroup,
    hadron_bind_group: wgpu::BindGroup,

    particle_count: u32,
}

impl ParticleSimulation {
    pub async fn new(device: wgpu::Device, queue: wgpu::Queue, particles: &[Particle]) -> Self {
        let particle_count = particles.len() as u32;

        // Create particle buffer
        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(particles),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        // Create force buffer (zero-initialized)
        let forces = vec![
            Force {
                force: [0.0; 3],
                _padding: 0.0
            };
            particles.len()
        ];
        let force_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Buffer"),
            contents: bytemuck::cast_slice(&forces),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create hadron buffer (zero-initialized)
        // We allocate enough space for every particle to potentially be a hadron leader (overkill but safe)
        let hadron_size = std::mem::size_of::<Hadron>() as u64;
        let hadron_buffer_size = hadron_size * particles.len() as u64;
        let hadron_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hadron Buffer"),
            size: hadron_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create hadron counter buffer
        let hadron_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hadron Count Buffer"),
            size: 4, // u32
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Load compute shaders
        let force_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Force Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/forces.wgsl").into()),
        });

        let integrate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Integration Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/integrate.wgsl").into()),
        });

        let hadron_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hadron Detection Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/hadron_detection.wgsl").into()),
        });

        // Create bind group layout for force computation
        let force_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Force Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create bind group layout for integration
        let integrate_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Integration Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create bind group layout for hadron detection
        let hadron_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Hadron Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create compute pipelines
        let force_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Force Pipeline Layout"),
                bind_group_layouts: &[&force_bind_group_layout],
                push_constant_ranges: &[],
            });

        let force_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Force Pipeline"),
            layout: Some(&force_pipeline_layout),
            module: &force_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let integrate_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Integration Pipeline Layout"),
                bind_group_layouts: &[&integrate_bind_group_layout],
                push_constant_ranges: &[],
            });

        let integrate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Integration Pipeline"),
            layout: Some(&integrate_pipeline_layout),
            module: &integrate_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let hadron_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Hadron Pipeline Layout"),
                bind_group_layouts: &[&hadron_bind_group_layout],
                push_constant_ranges: &[],
            });

        let hadron_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Hadron Pipeline"),
            layout: Some(&hadron_pipeline_layout),
            module: &hadron_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create bind groups
        let force_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Force Bind Group"),
            layout: &force_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: force_buffer.as_entire_binding(),
                },
            ],
        });

        let integrate_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Integration Bind Group"),
            layout: &integrate_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: force_buffer.as_entire_binding(),
                },
            ],
        });

        let hadron_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hadron Bind Group"),
            layout: &hadron_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: hadron_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: hadron_count_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            device,
            queue,
            particle_buffer,
            _force_buffer: force_buffer,
            hadron_buffer,
            hadron_count_buffer,
            force_pipeline,
            integrate_pipeline,
            hadron_pipeline,
            force_bind_group,
            integrate_bind_group,
            hadron_bind_group,
            particle_count,
        }
    }

    /// Step the simulation forward by one timestep
    pub fn step(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Simulation Encoder"),
            });

        // Calculate workgroup count (256 threads per workgroup)
        let workgroup_count = (self.particle_count + 255) / 256;

        // Step 1: Compute forces
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.force_pipeline);
            compute_pass.set_bind_group(0, &self.force_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Step 2: Integrate motion
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Integration Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.integrate_pipeline);
            compute_pass.set_bind_group(0, &self.integrate_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Step 3: Detect Hadrons
        {
            // Reset counter
            self.queue
                .write_buffer(&self.hadron_count_buffer, 0, &[0u8; 4]);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Hadron Detection Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.hadron_pipeline);
            compute_pass.set_bind_group(0, &self.hadron_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Get reference to particle buffer for rendering
    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    /// Get particle count
    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }

    /// Get reference to hadron buffer for rendering
    pub fn hadron_buffer(&self) -> &wgpu::Buffer {
        &self.hadron_buffer
    }

    /// Get reference to hadron count buffer for rendering
    pub fn hadron_count_buffer(&self) -> &wgpu::Buffer {
        &self.hadron_count_buffer
    }
}
