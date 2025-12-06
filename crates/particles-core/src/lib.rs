use bevy::prelude::*;
use rand::Rng;

/// Represents a single particle with 3D position
#[derive(Component, Clone, Copy, Debug)]
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

/// Resource to hold all particles for GPU-efficient updates
#[derive(Resource, Default)]
pub struct ParticleBuffer {
    pub particles: Vec<Particle>,
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

pub struct ParticlesCorePlugin;

impl Plugin for ParticlesCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleBuffer>()
            .add_systems(Startup, initialize_particles);
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
