// Compute shader for calculating forces between particles
// Implements the four fundamental forces on GPU

struct PhysicsParams {
    constants: vec4<f32>,    // x: G, y: K_electric, z: G_weak, w: weak_force_range
    strong_force: vec4<f32>, // x: strong_short_range, y: strong_confinement, z: strong_range, w: padding
    repulsion: vec4<f32>,    // x: core_repulsion, y: core_radius, z: softening, w: max_force
    integration: vec4<f32>,  // x: dt, y: damping, z: time/seed, w: nucleon_damping
    nucleon: vec4<f32>,      // x: binding_strength, y: binding_range, z: exclusion_strength, w: exclusion_radius
    electron: vec4<f32>,     // x: exclusion_strength, y: exclusion_radius, z: padding, w: padding
    hadron: vec4<f32>,       // x: binding_distance, y: breakup_distance, z: confinement_range_mult, w: confinement_strength_mult
}

@group(0) @binding(2)
var<uniform> params: PhysicsParams;

// Particle structure (must match Rust struct)
// Using vec4 for ALL fields to ensure perfect 16-byte alignment
struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type (as f32)
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z = hadron_id, w = padding
}

// Force accumulator
struct Force {
    force: vec3<f32>,
    potential: f32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

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

// (hadron debug counters removed)

// Treat invalid/out-of-range hadron_id as "free".
// hadron_id on particles is 1-indexed (0u means "not in a hadron").
// This prevents quarks from getting stuck in a pseudo-bound state after rapid breakup / slot reuse.
fn is_valid_hadron_id(hadron_id: u32) -> bool {
    if (hadron_id == 0u) {
        return false;
    }

    // Convert to 0-indexed hadron slot.
    let h_idx = hadron_id - 1u;

    // Only consider slots within current count to avoid OOB and stale ids.
    let count = hadron_counter.count;
    if (h_idx >= count) {
        return false;
    }

    // Slot must be marked valid (type_id != 0xFFFFFFFFu).
    return hadrons[h_idx].indices_type.w != 0xFFFFFFFFu;
}

// Check if particle is a quark
fn is_quark(particle_type_f: f32) -> bool {
    let particle_type = u32(particle_type_f);
    return particle_type == 0u || particle_type == 1u; // QuarkUp or QuarkDown
}

// Returns the hadron's net electric charge based on constituent particle charges.
// Meson: uses x/y, Baryon: uses x/y/z.
fn hadron_net_charge(h: Hadron) -> f32 {
    var q = 0.0;

    let p1 = particles[h.indices_type.x];
    let p2 = particles[h.indices_type.y];
    q += p1.data.x;
    q += p2.data.x;

    // type_id: 0u = Meson, otherwise Baryon (3 constituents)
    if (h.indices_type.w != 0u) {
        let p3 = particles[h.indices_type.z];
        q += p3.data.x;
    }

    return q;
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
// Uses smooth saturation to prevent singularities and locked pairs
fn electromagnetic_force(p1: Particle, p2: Particle, r_vec: vec3<f32>, r_sq: f32) -> vec3<f32> {
    let charge_product = p1.data.x * p2.data.x; // charge in data.x

    // Smooth saturation using Yukawa-like modification: r_eff = sqrt(r^2 + r_sat^2)
    // This prevents discontinuities and oscillations at close range
    let saturation_dist = 0.2; // Smaller value for tighter saturation
    let effective_r_sq = r_sq + saturation_dist * saturation_dist;

    let force_mag = params.constants.y * abs(charge_product) / effective_r_sq;

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

    // Color confinement: Strong force only applies when:
    // 1. Both quarks are free (forming hadrons), OR
    // 2. Both quarks are in the SAME hadron (keeping it together)
    // Free quarks should NOT pull on bound quarks!
    // NOTE: hadron_id on particles is 1-indexed (0u means "not in a hadron")
    // hadron_id is 1-indexed: 0u = not in hadron, otherwise (hadron_index + 1)
    let p1_hadron_id = p1.color_and_flags.z;
    let p2_hadron_id = p2.color_and_flags.z;

    // Treat invalid/out-of-range ids as free to avoid "stuck bound" particles.
    let p1_valid_bound = is_valid_hadron_id(p1_hadron_id);
    let p2_valid_bound = is_valid_hadron_id(p2_hadron_id);

    let p1_is_free = !p1_valid_bound;
    let p2_is_free = !p2_valid_bound;
    let both_free = p1_is_free && p2_is_free;

    // Same-hadron only if both are validly bound and ids match.
    let same_hadron = p1_valid_bound && p2_valid_bound && (p1_hadron_id == p2_hadron_id);

    // Only allow strong force when both free OR in same hadron
    if !both_free && !same_hadron {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Color factor (color_charge in color_and_flags.x)
    // 1.0 = attract, -1.0 = repel
    // Color confinement (QCD): Only color-neutral (colorless) combinations can exist
    // Same colors STRONGLY repel (prevents non-neutral clumps)
    var color_factor = -1.0;
    if color_charges_attract(p1.color_and_flags.x, p2.color_and_flags.x) {
        color_factor = 1.0;
    }

    // Extra repulsion for same-color quarks (color neutrality enforcement)
    // In QCD, non-colorless configurations have infinite energy
    let same_color = p1.color_and_flags.x == p2.color_and_flags.x;
    if same_color && p1.color_and_flags.x < 3u {
        color_factor = -2.0; // Stronger repulsion for same colors
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

    // Use the hadron_id checks from above for confinement logic
    let any_free = p1_is_free || p2_is_free;

    // Color confinement: Free quarks experience stronger force at longer range
    // This models the confinement potential that makes free quarks energetically unfavorable
    var range_multiplier = 1.0;
    var strength_multiplier = 1.0;

    if any_free && color_factor > 0.0 {
        // Apply confinement multipliers from parameters
        // Both free: full strength, One free: half strength
        if (p1_is_free && p2_is_free) {
            range_multiplier = params.hadron.z;
            strength_multiplier = params.hadron.w;
        } else {
            range_multiplier = 1.0 + (params.hadron.z - 1.0) * 0.5;
            strength_multiplier = 1.0 + (params.hadron.w - 1.0) * 0.5;
        }
    }

    let effective_range = params.strong_force.z * range_multiplier;

    // Range cutoff (extended for free quarks)
    if r > effective_range {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Cornell potential: -a/r² + b (Force magnitude)
    // Significantly enhanced for free quarks to model confinement
    let short_range = params.strong_force.x / (r * r);
    let confinement = params.strong_force.y;
    let force_mag = color_factor * (short_range + confinement) * strength_multiplier;

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

    // Scrub invalid hadron_id references for this particle (local, per-invocation).
    // This MUST happen before taking the particle snapshot (`let p1 = ...`) so subsequent logic
    // uses a consistent view for free/bound checks and confinement multipliers.
    let p1_type_f = particles[index].position.w;
    if (is_quark(p1_type_f)) {
        let hid = particles[index].color_and_flags.z;

        // Scrub invalid/out-of-range ids.
        if (hid != 0u && !is_valid_hadron_id(hid)) {
            particles[index].color_and_flags.z = 0u;
        } else if (hid != 0u) {
            // Slot is valid; ensure it actually contains this particle index.
            let h_idx = hid - 1u;
            let h = hadrons[h_idx];
            let contained =
                (h.indices_type.x == index) ||
                (h.indices_type.y == index) ||
                (h.indices_type.z == index);

            if (!contained) {
                // Stale bookkeeping: clear to allow re-binding.
                particles[index].color_and_flags.z = 0u;
            }
        }
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

        let p1_is_electron = is_electron(p1.position.w);
        let p2_is_electron = is_electron(p2.position.w);
        let p1_is_quark = is_quark(p1.position.w);
        let p2_is_quark = is_quark(p2.position.w);

        // Sum all four fundamental forces
        var f = vec3<f32>(0.0, 0.0, 0.0);
        f += gravitational_force(p1, p2, r_vec, r_sq);

        // Electromagnetic force: Complex shielding rules
        var skip_em = false;

        // Skip electron-quark interactions (electrons only see hadrons)
        if ((p1_is_electron && p2_is_quark) || (p1_is_quark && p2_is_electron)) {
            skip_em = true;
        }

        // Skip quark-quark EM unless both free or in same hadron
        // Quarks in hadrons are shielded - only the hadron's net charge matters
        if (p1_is_quark && p2_is_quark) {
            // NOTE: hadron_id on particles is 1-indexed (0u means "not in a hadron")
            // hadron_id is 1-indexed: 0u = not in hadron, otherwise (hadron_index + 1)
            let p1_hadron_id = p1.color_and_flags.z;
            let p2_hadron_id = p2.color_and_flags.z;

            // Treat invalid/out-of-range ids as free to avoid "stuck bound" particles.
            let p1_valid_bound = is_valid_hadron_id(p1_hadron_id);
            let p2_valid_bound = is_valid_hadron_id(p2_hadron_id);

            let both_free = !p1_valid_bound && !p2_valid_bound;
            let same_hadron = p1_valid_bound && p2_valid_bound && (p1_hadron_id == p2_hadron_id);

            if (!both_free && !same_hadron) {
                skip_em = true; // Skip if in different hadrons or one free + one bound
            }
        }

        if (!skip_em) {
            f += electromagnetic_force(p1, p2, r_vec, r_sq);
        }

        let strong = strong_force(p1, p2, r_vec, r);
        f += strong.xyz;
        total_potential += strong.w;

        f += weak_force(p1, p2, r_vec, r, r_sq);

        total_force += clamp_force(f);
    }

    // Electron-Hadron Exclusion (electrons repelled from nucleus centers)
    // This keeps electrons in shells AROUND nuclei, not between nucleons
    // Electron-Hadron Electromagnetism + Exclusion
    // - Electrons do NOT interact electromagnetically with individual quarks (shielded within hadrons)
    // - Electrons DO interact with hadrons via hadron net charge (e.g. proton +1, neutron 0)
    // - Exclusion keeps electrons out of the nucleus center so they form shells around it
    if (is_electron(p1.position.w)) {
        let num_hadrons = hadron_counter.count;

        for (var h = 0u; h < num_hadrons; h++) {
            let hadron = hadrons[h];
            let r_vec_hadron = hadron.center.xyz - p1.position.xyz;
            let r_sq_hadron = dot(r_vec_hadron, r_vec_hadron);

            if (r_sq_hadron < params.repulsion.z * params.repulsion.z) {
                continue;
            }

            let r_hadron = sqrt(r_sq_hadron);

            // Exclusion radius scales with hadron size
            // 1) Electromagnetic attraction/repulsion to the hadron's net charge.
            // We model the hadron as a point charge at its center of mass.
            let q_hadron = hadron_net_charge(hadron);

            // Skip near-neutral hadrons (e.g. neutrons) for stability/perf.
            if (abs(q_hadron) > 0.01) {
                var hadron_particle: Particle;
                hadron_particle.position = vec4<f32>(hadron.center.xyz, 0.0);
                hadron_particle.velocity = vec4<f32>(hadron.velocity.xyz, 0.0);
                hadron_particle.data = vec4<f32>(q_hadron, 0.0, 0.0, 0.0);
                hadron_particle.color_and_flags = vec4<u32>(0u, 0u, 0u, 0u);

                total_force += electromagnetic_force(p1, hadron_particle, r_vec_hadron, r_sq_hadron);
            }

            // 2) Exclusion radius scales with hadron size
            let exclusion_dist = hadron.center.w + params.electron.y;

            if (r_hadron < exclusion_dist) {
                let overlap = exclusion_dist - r_hadron;
                // Strong quadratic repulsion to keep electrons out of nucleus
                let push = params.electron.x * overlap * overlap;
                total_force -= normalize(r_vec_hadron) * push;
            }
        }
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
