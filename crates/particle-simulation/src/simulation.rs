//! GPU-based particle simulation manager
//!
//! NOTE: Hadron detection/validation uses `type_id == 0xFFFFFFFF` as the sentinel for an invalid hadron slot.
//! The buffer is zero-initialized, so without explicitly seeding all slots as invalid, `find_free_slot()` will
//! never find reusable slots and may treat untouched slots as valid hadrons. We initialize all hadron slots as
//! invalid on startup to make slot reuse reliable.

use crate::PhysicsParams;
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
    locks_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,

    // Compute pipelines
    force_pipeline: wgpu::ComputePipeline,
    integrate_pipeline: wgpu::ComputePipeline,
    hadron_validation_pipeline: wgpu::ComputePipeline,
    hadron_pipeline: wgpu::ComputePipeline,

    // Bind groups
    force_bind_group: wgpu::BindGroup,
    integrate_bind_group: wgpu::BindGroup,
    hadron_bind_group: wgpu::BindGroup,

    particle_count: u32,
}

impl ParticleSimulation {
    pub async fn new(device: wgpu::Device, queue: wgpu::Queue, particles: &[Particle]) -> Self {
        log::info!("Initializing ParticleSimulation...");
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

        // Create hadron buffer.
        //
        // Important: The WGSL side treats `indices_type.w == 0xFFFFFFFFu` as "invalid hadron slot".
        // A newly created buffer is zero-initialized, which would look like a *valid* Meson (type_id=0)
        // unless we explicitly seed all slots as invalid.
        //
        // We allocate enough space for every particle to potentially be a hadron leader (overkill but safe).
        let hadron_size = std::mem::size_of::<Hadron>() as u64;
        let _hadron_buffer_size = hadron_size * particles.len() as u64;

        let invalid_hadrons: Vec<Hadron> = (0..particles.len())
            .map(|_| Hadron {
                p1: 0,
                p2: 0,
                p3: 0,
                type_id: 0xFFFF_FFFF,
                center: [0.0; 4],
                velocity: [0.0; 4],
            })
            .collect();

        let hadron_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hadron Buffer"),
            contents: bytemuck::cast_slice(&invalid_hadrons),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        // Create hadron counter buffer.
        //
        // Layout (16 bytes, 4x u32):
        // [0] total hadrons (including invalid slots that are still within counter range)
        // [1] protons
        // [2] neutrons
        // [3] other hadrons (e.g. mesons, other baryons)
        //
        // Note: WGSL uses explicit atomics; alignment here is naturally 4 bytes.
        let hadron_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hadron Count Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create locks buffer
        let locks_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Locks Buffer"),
            size: (particles.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create params buffer
        let params = PhysicsParams::default();
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        log::info!("Buffers created");

        // Load compute shaders
        let force_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Force Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/forces.wgsl").into()),
        });

        let integrate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Integration Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/integrate.wgsl").into()),
        });

        let hadron_validation_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hadron Validation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/hadron_validation.wgsl").into()),
        });

        let hadron_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hadron Detection Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/hadron_detection.wgsl").into()),
        });
        log::info!("Shaders loaded");

        // Create bind group layout for force computation
        let force_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Force Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            // Force shader may scrub invalid hadron_id values, so it must be able to write
                            // back into the particle buffer.
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
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create bind group layout for hadron detection and validation
        let hadron_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Hadron Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        log::info!("Bind group layouts created");

        // Create compute pipelines
        log::info!("Creating force pipeline layout...");
        let force_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Force Pipeline Layout"),
                bind_group_layouts: &[&force_bind_group_layout],
                push_constant_ranges: &[],
            });

        log::info!("Creating force pipeline...");
        let force_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Force Pipeline"),
            layout: Some(&force_pipeline_layout),
            module: &force_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        log::info!("Creating integrate pipeline layout...");
        let integrate_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Integration Pipeline Layout"),
                bind_group_layouts: &[&integrate_bind_group_layout],
                push_constant_ranges: &[],
            });

        log::info!("Creating integrate pipeline...");
        let integrate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Integration Pipeline"),
            layout: Some(&integrate_pipeline_layout),
            module: &integrate_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        log::info!("Creating hadron pipeline layout...");
        let hadron_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Hadron Pipeline Layout"),
                bind_group_layouts: &[&hadron_bind_group_layout],
                push_constant_ranges: &[],
            });

        log::info!("Creating hadron validation pipeline...");
        let hadron_validation_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Hadron Validation Pipeline"),
                layout: Some(&hadron_pipeline_layout),
                module: &hadron_validation_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        log::info!("Creating hadron pipeline...");
        let hadron_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Hadron Pipeline"),
            layout: Some(&hadron_pipeline_layout),
            module: &hadron_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        log::info!("Pipelines created");

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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: hadron_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: hadron_count_buffer.as_entire_binding(),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: locks_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        log::info!("Bind groups created");

        Self {
            device,
            queue,
            particle_buffer,
            _force_buffer: force_buffer,
            hadron_buffer,
            hadron_count_buffer,
            locks_buffer,
            params_buffer,

            force_pipeline,
            integrate_pipeline,
            hadron_validation_pipeline,
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

        // Step 3: Validate existing hadrons
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Hadron Validation Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.hadron_validation_pipeline);
            compute_pass.set_bind_group(0, &self.hadron_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Step 4: Detect new hadrons
        {
            // Reset locks (hadron count persists now)
            encoder.clear_buffer(&self.locks_buffer, 0, None);

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

    /// Get reference to particle buffer (read-only usage is up to the caller).
    ///
    /// This is also used by GPU picking to render IDs.
    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    /// Get particle count
    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }

    /// Get reference to hadron buffer.
    ///
    /// This is also used by GPU picking to render IDs for hadron shells.
    pub fn hadron_buffer(&self) -> &wgpu::Buffer {
        &self.hadron_buffer
    }

    /// Get reference to hadron count buffer.
    ///
    /// This is also used by GPU picking to know how many hadrons are valid.
    pub fn hadron_count_buffer(&self) -> &wgpu::Buffer {
        &self.hadron_count_buffer
    }

    /// Update physics parameters
    pub fn update_params(&self, params: &PhysicsParams) {
        self.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[*params]));
    }
}
