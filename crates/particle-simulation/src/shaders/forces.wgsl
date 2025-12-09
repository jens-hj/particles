// Compute shader for calculating forces between particles
// Implements the four fundamental forces on GPU

// Physical constants (matching constants.rs)
const G: f32 = 6.674e-11;
const K_ELECTRIC: f32 = 8.99;
const STRONG_SHORT_RANGE: f32 = 0.5;
const STRONG_CONFINEMENT: f32 = 1.0;
const G_WEAK: f32 = 1.0e-5;
const WEAK_FORCE_RANGE: f32 = 0.1;
const SOFTENING: f32 = 0.01;

// Particle structure (must match Rust struct)
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
fn is_quark(particle_type: u32) -> bool {
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

// Calculate gravitational force
fn gravitational_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec3<f32> {
    let force_mag = G * p1.mass * p2.mass / (r * r);
    return normalize(r_vec) * force_mag;
}

// Calculate electromagnetic force
// Positive product (like charges) = repulsive force (away from p2)
// Negative product (opposite charges) = attractive force (towards p2)
fn electromagnetic_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec3<f32> {
    let charge_product = p1.charge * p2.charge;
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
    // Only applies to quarks
    if !is_quark(p1.particle_type) || !is_quark(p2.particle_type) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }
    
    // Color factor
    var color_factor = 1.0;
    if color_charges_attract(p1.color_charge, p2.color_charge) {
        color_factor = -1.0;
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
        let r_vec = p2.position - p1.position;
        let r = length(r_vec) + SOFTENING;
        
        if r < SOFTENING * 2.0 {
            continue;
        }
        
        // Sum all four fundamental forces
        total_force += gravitational_force(p1, p2, r_vec, r);
        total_force += electromagnetic_force(p1, p2, r_vec, r);
        total_force += strong_force(p1, p2, r_vec, r);
        total_force += weak_force(p1, p2, r_vec, r);
    }
    
    forces[index].force = total_force;
}
