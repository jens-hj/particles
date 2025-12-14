//! GPU-based particle simulation manager
//!
//! NOTE: Hadron detection/validation uses `type_id == 0xFFFFFFFF` as the sentinel for an invalid hadron slot.
//! The buffer is zero-initialized, so without explicitly seeding all slots as invalid, `find_free_slot()` will
//! never find reusable slots and may treat untouched slots as valid hadrons. We initialize all hadron slots as
//! invalid on startup to make slot reuse reliable.

use crate::PhysicsParams;
use bytemuck::{Pod, Zeroable};
use particle_physics::{Hadron, Nucleus, Particle, MAX_NUCLEONS};
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
    nucleus_buffer: wgpu::Buffer,
    nucleus_count_buffer: wgpu::Buffer,
    locks_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,

    // Selection (GPU resolve)
    selection_id_buffer: wgpu::Buffer,
    selection_target_buffer: wgpu::Buffer,
    selection_pipeline: wgpu::ComputePipeline,
    selection_bind_group: wgpu::BindGroup,

    // Compute pipelines
    force_pipeline: wgpu::ComputePipeline,
    integrate_pipeline: wgpu::ComputePipeline,
    hadron_validation_pipeline: wgpu::ComputePipeline,
    hadron_pipeline: wgpu::ComputePipeline,
    nucleus_pipeline: wgpu::ComputePipeline,
    nucleus_reset_pipeline: wgpu::ComputePipeline,

    // Bind groups
    force_bind_group: wgpu::BindGroup,
    integrate_bind_group: wgpu::BindGroup,
    hadron_bind_group: wgpu::BindGroup,
    nucleus_bind_group: wgpu::BindGroup,

    particle_count: u32,
    nucleus_capacity: u32,
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

        // Create nucleus buffer.
        //
        // Similar to hadron buffer, we need to initialize all slots as invalid (type_id = 0xFFFFFFFF).
        // Nuclei can contain up to MAX_NUCLEONS hadrons. We'll allocate space for up to
        // particles.len() / 4 potential nuclei (rough estimate).
        let max_nuclei = particles.len() / 4;
        let invalid_nuclei: Vec<Nucleus> = (0..max_nuclei)
            .map(|_| Nucleus {
                hadron_indices: [0xFFFF_FFFF; MAX_NUCLEONS],
                nucleon_count: 0,
                proton_count: 0,
                neutron_count: 0,
                type_id: 0xFFFF_FFFF,
                center: [0.0; 4],
                velocity: [0.0; 4],
            })
            .collect();

        let nucleus_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Nucleus Buffer"),
            contents: bytemuck::cast_slice(&invalid_nuclei),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        // Create nucleus counter buffer (single u32 + padding)
        // WGSL alignment for atomic<u32> requires 32 bytes total
        let nucleus_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Nucleus Count Buffer"),
            size: 32, // WGSL atomic alignment requirement
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

        // Selection resolve buffers (CPU writes selected ID; GPU resolves to world-space center)
        //
        // selection_id_buffer layout: 16 bytes (u32 + padding) to match WGSL `Selection` uniform.
        let selection_id_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selection ID Buffer"),
            contents: bytemuck::cast_slice(&[0u32, 0u32, 0u32, 0u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // selection_target_buffer layout: vec4<f32> (16 bytes)
        // xyz = selected center, w = kind (0 none, 1 particle, 2 hadron)
        let selection_target_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Selection Target Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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

        let nucleus_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Nucleus Detection Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/nucleus_detection.wgsl").into()),
        });

        let nucleus_reset_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Nucleus Frame Reset Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/nucleus_validation.wgsl").into(),
            ),
        });

        let selection_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Selection Resolve Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/selection_resolve.wgsl").into()),
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

        // Bind group layout for selection resolve compute:
        // 0: selection id (uniform)
        // 1: particles (storage, read)
        // 2: hadrons (storage, read)
        // 3: selection target (storage, write)
        let selection_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Selection Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
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

        let nucleus_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Nucleus Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false }, // Need write access for nucleus_id
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
                    // Hadron Counter (Storage, read-only) - Binding 5
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
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

        log::debug!("Creating selection pipeline layout...");
        let selection_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Selection Pipeline Layout"),
                bind_group_layouts: &[&selection_bind_group_layout],
                push_constant_ranges: &[],
            });

        log::debug!("Creating selection pipeline...");
        let selection_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Selection Pipeline"),
            layout: Some(&selection_pipeline_layout),
            module: &selection_shader,
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

        log::info!("Creating nucleus pipeline layout...");
        let nucleus_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Nucleus Pipeline Layout"),
                bind_group_layouts: &[&nucleus_bind_group_layout],
                push_constant_ranges: &[],
            });

        log::info!("Creating nucleus pipeline...");
        let nucleus_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Nucleus Pipeline"),
            layout: Some(&nucleus_pipeline_layout),
            module: &nucleus_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        log::info!("Creating nucleus reset pipeline...");
        let nucleus_reset_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Nucleus Reset Pipeline"),
                layout: Some(&nucleus_pipeline_layout),
                module: &nucleus_reset_shader,
                entry_point: Some("reset_main"),
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

        let selection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Selection Bind Group"),
            layout: &selection_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: selection_id_buffer.as_entire_binding(),
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
                    resource: selection_target_buffer.as_entire_binding(),
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

        let nucleus_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Nucleus Bind Group"),
            layout: &nucleus_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: hadron_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nucleus_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: nucleus_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: locks_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: hadron_count_buffer.as_entire_binding(),
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
            nucleus_buffer,
            nucleus_count_buffer,
            locks_buffer,
            params_buffer,

            selection_id_buffer,
            selection_target_buffer,
            selection_pipeline,
            selection_bind_group,

            force_pipeline,
            integrate_pipeline,
            hadron_validation_pipeline,
            hadron_pipeline,
            nucleus_pipeline,
            nucleus_reset_pipeline,
            force_bind_group,
            integrate_bind_group,
            hadron_bind_group,
            nucleus_bind_group,
            particle_count,
            nucleus_capacity: max_nuclei as u32,
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

        // Step 5: Per-frame nucleus detection (reset + detect)
        {
            // Reset nucleus counter + invalidate nucleus slots + clear nucleus_id on hadrons.
            encoder.clear_buffer(&self.nucleus_count_buffer, 0, None);
            encoder.clear_buffer(&self.locks_buffer, 0, None);

            let reset_span = self.particle_count.max(self.nucleus_capacity);
            let reset_workgroups = (reset_span + 255) / 256;

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Nucleus Frame Reset Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.nucleus_reset_pipeline);
            compute_pass.set_bind_group(0, &self.nucleus_bind_group, &[]);
            compute_pass.dispatch_workgroups(reset_workgroups, 1, 1);
        }

        // Step 6: Detect nuclei
        {
            // Reset locks for detection.
            encoder.clear_buffer(&self.locks_buffer, 0, None);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Nucleus Detection Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.nucleus_pipeline);
            compute_pass.set_bind_group(0, &self.nucleus_bind_group, &[]);
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

    /// Update the currently selected packed ID (written by GPU picking).
    ///
    /// The ID encoding convention must match the picking shader:
    /// - 0 => none
    /// - (particle_index + 1) => particle
    /// - 0x80000000 | (hadron_index + 1) => hadron
    pub fn set_selected_id(&self, id: u32) {
        let data = [id, 0u32, 0u32, 0u32];
        self.queue
            .write_buffer(&self.selection_id_buffer, 0, bytemuck::cast_slice(&data));
    }

    /// Run the selection resolve compute pass (1 invocation).
    ///
    /// This writes the selected entity center into `selection_target_buffer`.
    pub fn encode_selection_resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Selection Resolve Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.selection_pipeline);
        pass.set_bind_group(0, &self.selection_bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Get the selection target buffer for readback.
    pub fn selection_target_buffer(&self) -> &wgpu::Buffer {
        &self.selection_target_buffer
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

    /// Get reference to nucleus buffer.
    pub fn nucleus_buffer(&self) -> &wgpu::Buffer {
        &self.nucleus_buffer
    }

    /// Get reference to nucleus count buffer.
    pub fn nucleus_count_buffer(&self) -> &wgpu::Buffer {
        &self.nucleus_count_buffer
    }

    /// Update physics parameters
    pub fn update_params(&self, params: &PhysicsParams) {
        self.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[*params]));
    }
}
