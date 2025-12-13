// Compute shader for integrating particle motion
// Uses Velocity Verlet integration for better energy conservation

struct PhysicsParams {
    constants: vec4<f32>,    // x: G, y: K_electric, z: G_weak, w: weak_force_range
    strong_force: vec4<f32>, // x: strong_short_range, y: strong_confinement, z: strong_range, w: padding
    repulsion: vec4<f32>,    // x: core_repulsion, y: core_radius, z: softening, w: max_force
    integration: vec4<f32>,  // x: dt, y: damping, z: padding, w: padding
}

@group(0) @binding(2)
var<uniform> params: PhysicsParams;

struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z = hadron_id, w = padding
}

struct Force {
    force: vec3<f32>,
    potential: f32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read> forces: array<Force>;

// Simple pseudo-random number generator
fn rand(seed: vec2<f32>) -> f32 {
    return fract(sin(dot(seed, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn is_quark(particle_type_f: f32) -> bool {
    let particle_type = u32(particle_type_f);
    return particle_type == 0u || particle_type == 1u; // QuarkUp or QuarkDown
}

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
    let new_velocity = particle.velocity.xyz + acceleration * params.integration.x;

    // Apply damping for numerical stability
    var damped_velocity = new_velocity * params.integration.y;

    // Electromagnetic radiation damping (Larmor formula approximation)
    // Accelerating charges radiate energy, causing velocity-dependent damping
    // This prevents locked pairs and helps systems settle into bound states
    let charge = particle.data.x;
    let accel_magnitude = length(acceleration);
    let velocity_magnitude = length(damped_velocity);

    // Enhanced damping for high-speed charged particles (prevents jitter)
    // Radiation damping proportional to charge² and (acceleration + velocity)
    let base_damping = 0.02 * abs(charge) * accel_magnitude;
    let velocity_damping = 0.005 * abs(charge) * velocity_magnitude;
    let total_radiation_damping = base_damping + velocity_damping;

    damped_velocity *= (1.0 - min(total_radiation_damping, 0.15)); // Cap at 15% per step

    // x(t + dt) = x(t) + v(t + dt) * dt
    let new_position = particle.position.xyz + damped_velocity * params.integration.x;

    // Update particle (preserve .w components)
    particle.position = vec4<f32>(new_position, particle.position.w);
    particle.velocity = vec4<f32>(damped_velocity, mass);

    // Random color change (Gluon interaction simulation)
    // Quarks change color to minimize strong force potential (find stable bound states)
    if is_quark(particle.position.w) {
        let potential = forces[index].potential;

        // Based on forces.wgsl calculation:
        // Positive Potential = Unstable (Repelling)
        // Negative Potential = Stable (Attracting)

        var prob = 0.0;
        if potential > 0.5 {
            // Net Repelling state: chance to flip to find a partner
            prob = 0.05;
        } else if potential > -0.5 {
            // Weakly bound or isolated: very small chance to flip
            prob = 0.001;
        }
        // Else (potential <= -0.5): Stable (bound), do not flip

        // Use position and velocity as seed for randomness
        let seed = vec2<f32>(
            particle.position.x * particle.velocity.y + particle.position.z + params.integration.z,
            particle.position.y * particle.velocity.x + f32(index) * 0.1 - params.integration.z
        );

        if rand(seed) < prob {
            // Pick a new random color (0, 1, or 2)
            let r = rand(seed + vec2<f32>(1.0, 1.0));
            let new_color = u32(r * 3.0); // 0, 1, or 2
            particle.color_and_flags.x = new_color;
        }
    }

    particles[index] = particle;
}
