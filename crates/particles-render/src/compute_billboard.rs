use bevy::prelude::*;
use bevy::render::{
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSystems,
    extract_resource::ExtractResourcePlugin,
};
use bevy::render::render_resource::binding_types::{
    storage_buffer, storage_buffer_read_only, uniform_buffer,
};
use particles_core::ParticleBuffer;

pub struct ComputeBillboardPlugin;

impl Plugin for ComputeBillboardPlugin {
    fn build(&self, app: &mut App) {
        // Add resource extraction for particle data
        app.add_plugins(ExtractResourcePlugin::<ParticleBuffer>::default());

        // Set up render app
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<BillboardPipelines>()
            .init_resource::<ParticleSize>()
            .add_systems(
                Render,
                prepare_buffers.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                Render,
                prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups),
            );

        info!("ComputeBillboardPlugin initialized");
    }
}

/// Uniform for particle size
#[derive(Resource, Clone, Copy, ShaderType)]
struct ParticleSize {
    size: f32,
}

impl Default for ParticleSize {
    fn default() -> Self {
        Self { size: 4.0 }
    }
}

/// Camera uniform for compute shader
#[derive(Clone, Copy, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct CameraUniform {
    view_proj: Mat4,
    position: Vec3,
    _padding: f32,
}

/// Resource holding all pipeline data
#[derive(Resource)]
struct BillboardPipelines {
    compute_pipeline: Option<CachedComputePipelineId>,
    render_pipeline: Option<CachedRenderPipelineId>,
    compute_bind_group_layout: Option<BindGroupLayout>,
    render_bind_group_layout: Option<BindGroupLayout>,
}

impl FromWorld for BillboardPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create compute bind group layout
        let compute_bind_group_layout = render_device.create_bind_group_layout(
            "particle_compute_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // Particle positions (input storage buffer)
                    storage_buffer_read_only::<Vec4>(false),
                    // Quad vertices (output storage buffer)
                    storage_buffer::<Vec4>(false),
                    // Camera uniform
                    uniform_buffer::<CameraUniform>(true),
                    // Particle size uniform
                    uniform_buffer::<f32>(true),
                ),
            ),
        );

        info!("Compute bind group layout created");

        Self {
            compute_pipeline: None,
            render_pipeline: None,
            compute_bind_group_layout: Some(compute_bind_group_layout),
            render_bind_group_layout: None,
        }
    }
}

/// GPU buffer for particle positions
#[derive(Resource)]
struct GpuParticleBuffer {
    buffer: Buffer,
    count: u32,
}

/// GPU buffer for generated quad vertices
#[derive(Resource)]
struct GpuVertexBuffer {
    buffer: Buffer,
    vertex_count: u32,
}

/// GPU buffer for camera uniform
#[derive(Resource)]
struct GpuCameraBuffer {
    buffer: Buffer,
}

/// GPU buffer for particle size
#[derive(Resource)]
struct GpuParticleSizeBuffer {
    buffer: Buffer,
}

/// Bind group for compute shader
#[derive(Resource)]
struct ComputeBindGroup {
    bind_group: BindGroup,
}

/// Prepare GPU buffers from extracted particle data
fn prepare_buffers(
    particle_buffer: Res<ParticleBuffer>,
    mut gpu_particle_buffer: Option<ResMut<GpuParticleBuffer>>,
    mut gpu_vertex_buffer: Option<ResMut<GpuVertexBuffer>>,
    mut gpu_camera_buffer: Option<ResMut<GpuCameraBuffer>>,
    mut gpu_size_buffer: Option<ResMut<GpuParticleSizeBuffer>>,
    particle_size: Res<ParticleSize>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
) {
    if particle_buffer.particles.is_empty() {
        return;
    }

    let particle_count = particle_buffer.particle_count() as u32;
    let vertex_count = particle_count * 6; // 6 vertices per particle (2 triangles)

    // Prepare particle position data (vec4 for alignment)
    let particle_data: Vec<[f32; 4]> = particle_buffer
        .particles
        .iter()
        .map(|p| [p.position.x, p.position.y, p.position.z, 0.0])
        .collect();

    // Create or update particle buffer
    if gpu_particle_buffer.is_none() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("particle_position_buffer"),
            contents: bytemuck::cast_slice(&particle_data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        commands.insert_resource(GpuParticleBuffer {
            buffer,
            count: particle_count,
        });

        info!("Created GPU particle buffer: {} particles", particle_count);
    }

    // Create or update vertex buffer
    if gpu_vertex_buffer.is_none() {
        // Each vertex: position (vec3) + uv (vec2) = 5 floats, padded to vec4 alignment
        // Actually in shader it's position (vec3) + uv (vec2), let's use proper struct
        let vertex_buffer_size = (vertex_count * std::mem::size_of::<[f32; 8]>() as u32) as u64;

        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("particle_vertex_buffer"),
            size: vertex_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        commands.insert_resource(GpuVertexBuffer {
            buffer,
            vertex_count,
        });

        info!("Created GPU vertex buffer: {} vertices", vertex_count);
    }

    // Create or update camera buffer
    if let Ok((camera, camera_transform)) = camera_query.single() {
        let view = camera_transform.to_matrix().inverse();
        let projection = camera.clip_from_view();
        let view_proj = projection * view;
        let camera_pos = camera_transform.translation();

        let camera_uniform = CameraUniform {
            view_proj,
            position: camera_pos,
            _padding: 0.0,
        };

        if gpu_camera_buffer.is_none() {
            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("camera_uniform_buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            commands.insert_resource(GpuCameraBuffer { buffer });
            info!("Created GPU camera buffer");
        } else if let Some(ref mut buffer_res) = gpu_camera_buffer {
            render_queue.write_buffer(&buffer_res.buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
        }
    }

    // Create particle size buffer
    if gpu_size_buffer.is_none() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("particle_size_buffer"),
            contents: bytemuck::cast_slice(&[particle_size.size]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        commands.insert_resource(GpuParticleSizeBuffer { buffer });
        info!("Created GPU particle size buffer");
    }
}

/// Prepare bind groups for compute shader
fn prepare_bind_groups(
    pipelines: Res<BillboardPipelines>,
    gpu_particle_buffer: Option<Res<GpuParticleBuffer>>,
    gpu_vertex_buffer: Option<Res<GpuVertexBuffer>>,
    gpu_camera_buffer: Option<Res<GpuCameraBuffer>>,
    gpu_size_buffer: Option<Res<GpuParticleSizeBuffer>>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    // Only create bind group if all buffers are ready
    let (Some(particle_buf), Some(vertex_buf), Some(camera_buf), Some(size_buf)) = (
        gpu_particle_buffer.as_ref(),
        gpu_vertex_buffer.as_ref(),
        gpu_camera_buffer.as_ref(),
        gpu_size_buffer.as_ref(),
    ) else {
        return;
    };

    let Some(ref layout) = pipelines.compute_bind_group_layout else {
        return;
    };

    let bind_group = render_device.create_bind_group(
        "particle_compute_bind_group",
        layout,
        &BindGroupEntries::sequential((
            particle_buf.buffer.as_entire_buffer_binding(),
            vertex_buf.buffer.as_entire_buffer_binding(),
            camera_buf.buffer.as_entire_buffer_binding(),
            size_buf.buffer.as_entire_buffer_binding(),
        )),
    );

    commands.insert_resource(ComputeBindGroup { bind_group });
    info!("Created compute bind group");
}

/// Create or get cached compute pipeline
fn get_compute_pipeline(
    pipelines: &mut BillboardPipelines,
    pipeline_cache: &PipelineCache,
    render_device: &RenderDevice,
    asset_server: &AssetServer,
) -> Option<CachedComputePipelineId> {
    if let Some(pipeline_id) = pipelines.compute_pipeline {
        return Some(pipeline_id);
    }

    // Load compute shader
    let shader = asset_server.load::<Shader>("shaders/particle_quad_gen.wgsl");

    let layout = pipelines.compute_bind_group_layout.as_ref()?;

    // Create pipeline descriptor
    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("particle_quad_generation_pipeline".into()),
        layout: vec![layout.clone()],
        push_constant_ranges: vec![],
        shader,
        shader_defs: vec![],
        entry_point: "main".into(),
        zero_initialize_workgroup_memory: false,
    });

    pipelines.compute_pipeline = Some(pipeline_id);
    info!("Queued compute pipeline for caching");

    Some(pipeline_id)
}

/// Dispatch compute shader to generate quads
fn dispatch_compute(
    mut pipelines: ResMut<BillboardPipelines>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    gpu_particle_buffer: Option<Res<GpuParticleBuffer>>,
    compute_bind_group: Option<Res<ComputeBindGroup>>,
    mut render_context: ResMut<bevy::render::renderer::RenderContext>,
) {
    let Some(particle_buf) = gpu_particle_buffer else {
        return;
    };

    let Some(bind_group_res) = compute_bind_group else {
        return;
    };

    // Get or create pipeline
    let Some(pipeline_id) = get_compute_pipeline(
        &mut pipelines,
        &pipeline_cache,
        &render_device,
        &asset_server,
    ) else {
        return;
    };

    // Check if pipeline is ready
    let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
        return; // Pipeline still compiling
    };

    // Dispatch compute shader
    let particle_count = particle_buf.count;
    let workgroup_size = 64;
    let workgroup_count = (particle_count + workgroup_size - 1) / workgroup_size;

    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&bevy::render::render_resource::ComputePassDescriptor {
            label: Some("particle_quad_generation_pass"),
            timestamp_writes: None,
        });

    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group_res.bind_group, &[]);
    pass.dispatch_workgroups(workgroup_count, 1, 1);

    info!("Dispatched compute shader: {} particles, {} workgroups", particle_count, workgroup_count);
}
