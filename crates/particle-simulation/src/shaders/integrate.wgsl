// Compute shader for integrating particle motion
// Uses Velocity Verlet integration for better energy conservation

const DAMPING: f32 = 0.995;
const DT: f32 = 0.001; // Time step

struct Particle {
    position: vec3<f32>,
    particle_type: u32,
    velocity: vec3<f32>,
    mass: f32,
    charge: f32,
    color_charge: u32,
    flags: u32,
    size: f32,
}

struct Force {
    force: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read> forces: array<Force>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = arrayLength(&particles);
    
    if index >= num_particles {
        return;
    }
    
    var particle = particles[index];
    let force = forces[index].force;
    
    // F = ma, so a = F/m
    let acceleration = force / particle.mass;
    
    // Velocity Verlet integration
    // v(t + dt) = v(t) + a(t) * dt
    particle.velocity += acceleration * DT;
    
    // Apply damping for numerical stability
    particle.velocity *= DAMPING;
    
    // x(t + dt) = x(t) + v(t + dt) * dt
    particle.position += particle.velocity * DT;
    
    particles[index] = particle;
}
