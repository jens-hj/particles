// Compute shader for integrating particle motion
// Uses Velocity Verlet integration for better energy conservation

const DAMPING: f32 = 0.995;
const DT: f32 = 0.001; // Time step

struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z/w = padding
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

    // F = ma, so a = F/m (mass in velocity.w)
    let mass = particle.velocity.w;
    let acceleration = force / mass;

    // Velocity Verlet integration
    // v(t + dt) = v(t) + a(t) * dt
    let new_velocity = particle.velocity.xyz + acceleration * DT;

    // Apply damping for numerical stability
    let damped_velocity = new_velocity * DAMPING;

    // x(t + dt) = x(t) + v(t + dt) * dt
    let new_position = particle.position.xyz + damped_velocity * DT;

    // Update particle (preserve .w components)
    particle.position = vec4<f32>(new_position, particle.position.w);
    particle.velocity = vec4<f32>(damped_velocity, mass);

    particles[index] = particle;
}
