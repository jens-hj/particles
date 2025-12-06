use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{
    Buffer, BufferInitDescriptor, BufferUsages, ShaderType,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{Render, RenderApp, RenderSystems};
use rand::Rng;

/// Represents a single particle with 3D position
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct Particle {
    pub position: Vec3,
}

impl Particle {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
        }
    }
}

/// Resource to hold all particles in main world
#[derive(Resource, Default, Clone)]
pub struct ParticleBuffer {
    pub particles: Vec<Particle>,
}

impl ExtractResource for ParticleBuffer {
    type Source = ParticleBuffer;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

impl ParticleBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_particle(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

/// GPU buffer for particle data in render world
#[derive(Resource)]
pub struct GpuParticleBuffer {
    pub buffer: Buffer,
    pub particle_count: usize,
}

pub struct ParticlesCorePlugin;

impl Plugin for ParticlesCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleBuffer>()
            .add_plugins(ExtractResourcePlugin::<ParticleBuffer>::default())
            .add_systems(Startup, initialize_particles);

        // Set up render world systems
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, prepare_particle_buffer.in_set(RenderSystems::Prepare));
    }
}

fn initialize_particles(mut particle_buffer: ResMut<ParticleBuffer>) {
    let mut rng = rand::rng();
    let num_particles = 1000;
    let sphere_radius = 200.0;

    // Generate random particles within a sphere using rejection sampling
    for _ in 0..num_particles {
        loop {
            // Generate random point in [-1, 1]³
            let x = rng.random_range(-1.0..1.0);
            let y = rng.random_range(-1.0..1.0);
            let z = rng.random_range(-1.0..1.0);

            // Check if point is inside unit sphere
            let length_squared = x * x + y * y + z * z;
            if length_squared <= 1.0 {
                // Scale by sphere radius
                let particle = Particle::new(
                    x * sphere_radius,
                    y * sphere_radius,
                    z * sphere_radius,
                );
                particle_buffer.add_particle(particle);
                break;
            }
        }
    }

    info!("Initialized {} particles in sphere", particle_buffer.particle_count());
}

/// System that runs in render world to create/update GPU buffer
fn prepare_particle_buffer(
    particle_buffer: Res<ParticleBuffer>,
    mut gpu_buffer: Option<ResMut<GpuParticleBuffer>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut commands: Commands,
) {
    if particle_buffer.particles.is_empty() {
        return;
    }

    let particle_count = particle_buffer.particle_count();

    // Convert particle data to bytes
    let particle_data: Vec<f32> = particle_buffer
        .particles
        .iter()
        .flat_map(|p| [p.position.x, p.position.y, p.position.z, 0.0]) // Pad to vec4 for alignment
        .collect();

    let byte_data = bytemuck::cast_slice(&particle_data);

    match gpu_buffer.as_mut() {
        Some(gpu_buffer) => {
            // Update existing buffer
            render_queue.write_buffer(&gpu_buffer.buffer, 0, byte_data);
        }
        None => {
            // Create new buffer
            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("particle_buffer"),
                contents: byte_data,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::VERTEX,
            });

            commands.insert_resource(GpuParticleBuffer {
                buffer,
                particle_count,
            });
        }
    }
}
