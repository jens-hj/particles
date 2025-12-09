// Compute shader for calculating forces between particles
// Implements the four fundamental forces on GPU

// Physical constants (matching constants.rs)
const G: f32 = 6.674e-11;
const K_ELECTRIC: f32 = 8.99;
const STRONG_SHORT_RANGE: f32 = 0.5;
const STRONG_CONFINEMENT: f32 = 1.0;
const STRONG_RANGE: f32 = 3.0;      // Cutoff range for strong force
const CORE_REPULSION: f32 = 150.0;  // Strength of short-range repulsion
const CORE_RADIUS: f32 = 0.35;      // Radius for hard-core repulsion
const G_WEAK: f32 = 1.0e-5;
const WEAK_FORCE_RANGE: f32 = 0.1;
const SOFTENING: f32 = 0.01; // Reduced softening to allow short-range repulsion
const MAX_FORCE: f32 = 50.0; // Clamp maximum force to prevent explosions

// Particle structure (must match Rust struct)
// Using vec4 for ALL fields to ensure perfect 16-byte alignment
struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type (as f32)
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z/w = padding
}

// Force accumulator
struct Force {
    force: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> forces: array<Force>;

// Check if particle is a quark
fn is_quark(particle_type_f: f32) -> bool {
    let particle_type = u32(particle_type_f);
    return particle_type == 0u || particle_type == 1u; // QuarkUp or QuarkDown
}

// Check if two color charges attract
fn color_charges_attract(c1: u32, c2: u32) -> bool {
    // Red(0) + AntiRed(3), Green(1) + AntiGreen(4), Blue(2) + AntiBlue(5)
    if (c1 == 0u && c2 == 3u) || (c1 == 3u && c2 == 0u) { return true; }
    if (c1 == 1u && c2 == 4u) || (c1 == 4u && c2 == 1u) { return true; }
    if (c1 == 2u && c2 == 5u) || (c1 == 5u && c2 == 2u) { return true; }
    // Different colors also attract (for forming color-neutral states)
    if c1 != c2 && c1 < 3u && c2 < 3u { return true; }
    return false;
}

// Helper to clamp force magnitude
fn clamp_force(f: vec3<f32>) -> vec3<f32> {
    let len = length(f);
    if (len > MAX_FORCE) {
        return normalize(f) * MAX_FORCE;
    }
    return f;
}

// Calculate gravitational force
fn gravitational_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec3<f32> {
    let force_mag = G * p1.velocity.w * p2.velocity.w / (r * r); // mass in .w
    return normalize(r_vec) * force_mag;
}

// Calculate electromagnetic force
// Positive product (like charges) = repulsive force (away from p2)
// Negative product (opposite charges) = attractive force (towards p2)
fn electromagnetic_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec3<f32> {
    let charge_product = p1.data.x * p2.data.x; // charge in data.x
    let force_mag = K_ELECTRIC * abs(charge_product) / (r * r);

    // If charges have same sign (product > 0), repel (force away from p2)
    // If charges have opposite signs (product < 0), attract (force towards p2)
    if charge_product > 0.0 {
        return -normalize(r_vec) * force_mag; // Repel
    } else {
        return normalize(r_vec) * force_mag;  // Attract
    }
}

// Calculate strong force (Cornell potential)
fn strong_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec3<f32> {
    // Only applies to quarks (particle_type in position.w)
    if !is_quark(p1.position.w) || !is_quark(p2.position.w) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    // Short-range repulsion (Hard core)
    if r < CORE_RADIUS {
        let push = CORE_REPULSION * (1.0 - r / CORE_RADIUS);
        return -normalize(r_vec) * push;
    }

    // Range cutoff
    if r > STRONG_RANGE {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    // Color factor (color_charge in color_and_flags.x)
    // 1.0 = attract, -1.0 = repel
    var color_factor = -1.0;
    if color_charges_attract(p1.color_and_flags.x, p2.color_and_flags.x) {
        color_factor = 1.0;
    }

    // Cornell potential: -a/r² + b
    let short_range = STRONG_SHORT_RANGE / (r * r);
    let confinement = STRONG_CONFINEMENT;
    let force_mag = color_factor * (short_range + confinement);

    return normalize(r_vec) * force_mag;
}

// Calculate weak force (Yukawa potential)
fn weak_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec3<f32> {
    if r > WEAK_FORCE_RANGE * 3.0 {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    let exp_term = exp(-r / WEAK_FORCE_RANGE);
    let force_mag = G_WEAK * exp_term / (r * r);

    return normalize(r_vec) * force_mag;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if index >= num_particles {
        return;
    }

    let p1 = particles[index];
    var total_force = vec3<f32>(0.0, 0.0, 0.0);

    // Calculate forces from all other particles (N-body)
    for (var i = 0u; i < num_particles; i = i + 1u) {
        if i == index {
            continue;
        }

        let p2 = particles[i];
        let r_vec = p2.position.xyz - p1.position.xyz; // Use .xyz for position
        let r_sq = dot(r_vec, r_vec);
        let r = sqrt(r_sq);

        if r < SOFTENING {
            continue;
        }

        // Sum all four fundamental forces
        var f = vec3<f32>(0.0, 0.0, 0.0);
        f += gravitational_force(p1, p2, r_vec, r);
        f += electromagnetic_force(p1, p2, r_vec, r);
        f += strong_force(p1, p2, r_vec, r);
        f += weak_force(p1, p2, r_vec, r);

        total_force += clamp_force(f);
    }

    forces[index].force = clamp_force(total_force);
}
