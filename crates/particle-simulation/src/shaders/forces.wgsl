// Compute shader for calculating forces between particles
// Implements the four fundamental forces on GPU

struct PhysicsParams {
    constants: vec4<f32>,    // x: G, y: K_electric, z: G_weak, w: weak_force_range
    strong_force: vec4<f32>, // x: strong_short_range, y: strong_confinement, z: strong_range, w: padding
    repulsion: vec4<f32>,    // x: core_repulsion, y: core_radius, z: softening, w: max_force
    integration: vec4<f32>,  // x: dt, y: damping, z: time/seed, w: nucleon_damping
    nucleon: vec4<f32>,      // x: binding_strength, y: binding_range, z: exclusion_strength, w: exclusion_radius
    electron: vec4<f32>,     // x: exclusion_strength, y: exclusion_radius, z: padding, w: padding
}

@group(0) @binding(2)
var<uniform> params: PhysicsParams;

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
    potential: f32,
}

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> forces: array<Force>;

struct Hadron {
    indices_type: vec4<u32>, // x=p1, y=p2, z=p3, w=type_id
    center: vec4<f32>,       // xyz = center of mass, w = radius
    velocity: vec4<f32>,     // xyz = velocity, w = padding
}

struct HadronCounter {
    count: u32,
    _pad: vec3<u32>,
}

@group(0) @binding(3)
var<storage, read> hadrons: array<Hadron>;

@group(0) @binding(4)
var<storage, read> hadron_counter: HadronCounter;

// Check if particle is a quark
fn is_quark(particle_type_f: f32) -> bool {
    let particle_type = u32(particle_type_f);
    return particle_type == 0u || particle_type == 1u; // QuarkUp or QuarkDown
}

// Check if particle is a gluon
fn is_gluon(particle_type_f: f32) -> bool {
    return u32(particle_type_f) == 3u;
}

// Check if particle is an electron
fn is_electron(particle_type_f: f32) -> bool {
    return u32(particle_type_f) == 2u;
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
    if (len > params.repulsion.w) {
        return normalize(f) * params.repulsion.w;
    }
    return f;
}

// Calculate gravitational force
fn gravitational_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r_sq: f32) -> vec3<f32> {
    let force_mag = params.constants.x * p1.velocity.w * p2.velocity.w / r_sq; // mass in .w
    return normalize(r_vec) * force_mag;
}

// Calculate electromagnetic force
// Positive product (like charges) = repulsive force (away from p2)
// Negative product (opposite charges) = attractive force (towards p2)
fn electromagnetic_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r_sq: f32) -> vec3<f32> {
    let charge_product = p1.data.x * p2.data.x; // charge in data.x
    let force_mag = params.constants.y * abs(charge_product) / r_sq;

    // If charges have same sign (product > 0), repel (force away from p2)
    // If charges have opposite signs (product < 0), attract (force towards p2)
    if charge_product > 0.0 {
        return -normalize(r_vec) * force_mag; // Repel
    } else {
        return normalize(r_vec) * force_mag;  // Attract
    }
}

// Calculate strong force (Cornell potential)
// Returns vec4: xyz = force, w = potential energy (stress)
fn strong_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32) -> vec4<f32> {
    // Only applies to quarks (particle_type in position.w)
    if !is_quark(p1.position.w) || !is_quark(p2.position.w) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Color factor (color_charge in color_and_flags.x)
    // 1.0 = attract, -1.0 = repel
    var color_factor = -1.0;
    if color_charges_attract(p1.color_and_flags.x, p2.color_and_flags.x) {
        color_factor = 1.0;
    }

    // Stability metric:
    // Attracting (color_factor = 1.0) -> -1.0 (Stable)
    // Repelling (color_factor = -1.0) -> +1.0 (Unstable)
    let potential = -color_factor;

    // Short-range repulsion (Hard core)
    if r < params.repulsion.y {
        let push = params.repulsion.x * (1.0 - r / params.repulsion.y);
        // High potential at core overlap
        return vec4<f32>(-normalize(r_vec) * push, potential);
    }

    // Range cutoff
    if r > params.strong_force.z {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Cornell potential: -a/r² + b (Force magnitude)
    let short_range = params.strong_force.x / (r * r);
    let confinement = params.strong_force.y;
    let force_mag = color_factor * (short_range + confinement);

    return vec4<f32>(normalize(r_vec) * force_mag, potential);
}

// Calculate weak force (Yukawa potential)
fn weak_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r: f32, r_sq: f32) -> vec3<f32> {
    // Gluons don't participate in weak force in this simulation (and are too light)
    if is_gluon(p1.position.w) || is_gluon(p2.position.w) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    if r > params.constants.w * 3.0 {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    let exp_term = exp(-r / params.constants.w);
    let force_mag = params.constants.z * exp_term / r_sq;

    return normalize(r_vec) * force_mag;
}

// Calculate nucleon-nucleon forces (Residual Strong + Exclusion)
fn nucleon_force(h1: Hadron, h2: Hadron) -> vec3<f32> {
    let r_vec = h2.center.xyz - h1.center.xyz;
    let r_sq = dot(r_vec, r_vec);
    let r = sqrt(r_sq);

    if (r < 0.001) { return vec3<f32>(0.0); }

    var f = vec3<f32>(0.0);
    let dir = normalize(r_vec);

    // 1. Exclusion Force (Hard Sphere / Pauli)
    // Acts when hadrons overlap.
    let combined_radius = h1.center.w + h2.center.w;
    let exclusion_radius = combined_radius * params.nucleon.w;

    if (r < exclusion_radius) {
        let overlap = exclusion_radius - r;
        // Quadratic repulsion for stiffness
        let push = params.nucleon.z * overlap * (1.0 + overlap);
        f -= dir * push;
    }

    // Damping throughout binding range to stabilize nuclei
    if (r < params.nucleon.y * 3.0) {
        let v_rel = h2.velocity.xyz - h1.velocity.xyz;
        let v_closing = dot(v_rel, dir);
        if (v_closing < 0.0) { // Moving towards each other
            let damping_strength = params.integration.w;
            f += dir * v_closing * damping_strength;
        }
    }

    // 2. Residual Strong Force (Yukawa)
    // Attractive at short range.
    if (r < params.nucleon.y * 3.0) {
        let exp_term = exp(-r / params.nucleon.y);
        // Cap minimum distance for attraction calculation to avoid singularity
        let eff_r_sq = max(r * r, 0.5);
        let pull = params.nucleon.x * exp_term / eff_r_sq;

        // Dampen attraction inside exclusion zone to prevent instability
        let damp = smoothstep(exclusion_radius * 0.5, exclusion_radius, r);

        f += dir * pull * damp;
    }

    return f;
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
    var total_potential = 0.0;

    // Calculate forces from all other particles (N-body)
    for (var i = 0u; i < num_particles; i = i + 1u) {
        if i == index {
            continue;
        }

        let p2 = particles[i];
        let r_vec = p2.position.xyz - p1.position.xyz; // Use .xyz for position
        let r_sq = dot(r_vec, r_vec);

        if r_sq < params.repulsion.z * params.repulsion.z {
            continue;
        }

        let r = sqrt(r_sq);

        // Sum all four fundamental forces
        var f = vec3<f32>(0.0, 0.0, 0.0);
        f += gravitational_force(p1, p2, r_vec, r_sq);
        f += electromagnetic_force(p1, p2, r_vec, r_sq);

        // Electron Exclusion (Pauli-like repulsion from nucleus)
        if (is_electron(p1.position.w) && is_quark(p2.position.w)) ||
           (is_quark(p1.position.w) && is_electron(p2.position.w)) {
            if (r < params.electron.y) {
                let overlap = params.electron.y - r;
                let push = params.electron.x * overlap * overlap;
                f -= normalize(r_vec) * push;
            }
        }

        let strong = strong_force(p1, p2, r_vec, r);
        f += strong.xyz;
        total_potential += strong.w;

        f += weak_force(p1, p2, r_vec, r, r_sq);

        total_force += clamp_force(f);
    }

    // Nucleon Forces (Inter-Hadron)
    if (is_quark(p1.position.w)) {
        let num_hadrons = hadron_counter.count;
        var my_hadron_idx = -1;

        // Find my hadron
        for (var h = 0u; h < num_hadrons; h++) {
            let hadron = hadrons[h];
            if (hadron.indices_type.x == index ||
                hadron.indices_type.y == index ||
                hadron.indices_type.z == index) {
                my_hadron_idx = i32(h);
                break;
            }
        }

        if (my_hadron_idx != -1) {
            let my_hadron = hadrons[u32(my_hadron_idx)];
            var hadron_force = vec3<f32>(0.0);

            for (var h = 0u; h < num_hadrons; h++) {
                if (i32(h) == my_hadron_idx) { continue; }

                let other_hadron = hadrons[h];
                hadron_force += nucleon_force(my_hadron, other_hadron);
            }

            // Distribute force to constituents
            var num_constituents = 3.0;
            if (my_hadron.indices_type.w == 0u) { // Meson
                num_constituents = 2.0;
            }

            total_force += clamp_force(hadron_force / num_constituents);
        }
    }

    forces[index].force = clamp_force(total_force);
    forces[index].potential = total_potential;
}
