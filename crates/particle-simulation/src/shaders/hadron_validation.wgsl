// Compute shader for validating existing hadrons
// Checks if constituent quarks are still bound, breaks up hadrons if not

// Particle Types
const TYPE_QUARK_UP: u32 = 0u;
const TYPE_QUARK_DOWN: u32 = 1u;

struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z = hadron_id, w = padding
}

struct Hadron {
    indices_type: vec4<u32>, // x=p1, y=p2, z=p3, w=type_id
    center: vec4<f32>,       // xyz = center of mass, w = radius
    velocity: vec4<f32>,     // xyz = velocity, w = padding
}

struct HadronCounter {
    // 4x u32 counters:
    // [0] total hadrons (counter range; may include invalid slots)
    // [1] protons
    // [2] neutrons
    // [3] other hadrons (mesons, other baryons, etc.)
    counters: array<atomic<u32>, 4>,
}

struct PhysicsParams {
    constants: vec4<f32>,
    strong_force: vec4<f32>,
    repulsion: vec4<f32>,
    integration: vec4<f32>,
    nucleon: vec4<f32>,
    electron: vec4<f32>,
    hadron: vec4<f32>, // x: binding_distance, y: breakup_distance, z: confinement_range_mult, w: confinement_strength_mult
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> hadrons: array<Hadron>;

@group(0) @binding(2)
var<storage, read_write> counter: HadronCounter;

@group(0) @binding(3)
var<storage, read_write> locks: array<atomic<u32>>;

@group(0) @binding(4)
var<uniform> params: PhysicsParams;

fn get_dist_sq(p1_idx: u32, p2_idx: u32) -> f32 {
    let pos1 = particles[p1_idx].position.xyz;
    let pos2 = particles[p2_idx].position.xyz;
    let diff = pos2 - pos1;
    return dot(diff, diff);
}

fn is_quark(p_idx: u32) -> bool {
    let t = u32(particles[p_idx].position.w);
    return t == TYPE_QUARK_UP || t == TYPE_QUARK_DOWN;
}

// Mark hadron as invalid
fn invalidate_hadron(h_idx: u32) {
    let h = hadrons[h_idx];

    // Clear hadron_id from constituent particles
    let p1 = h.indices_type.x;
    let p2 = h.indices_type.y;
    let p3 = h.indices_type.z;

    if (is_quark(p1)) {
        particles[p1].color_and_flags.z = 0u;
    }
    if (is_quark(p2)) {
        particles[p2].color_and_flags.z = 0u;
    }
    if (p3 != 0xFFFFFFFFu && is_quark(p3)) {
        particles[p3].color_and_flags.z = 0u;
    }

    // Mark hadron as invalid by setting type to max value
    hadrons[h_idx].indices_type.w = 0xFFFFFFFFu;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let h_idx = global_id.x;
    let num_hadrons = atomicLoad(&counter.counters[0]);

    if (h_idx >= num_hadrons) {
        return;
    }

    let h = hadrons[h_idx];

    // Skip already invalid hadrons
    if (h.indices_type.w == 0xFFFFFFFFu) {
        return;
    }

    let p1 = h.indices_type.x;
    let p2 = h.indices_type.y;
    let p3 = h.indices_type.z;
    let is_meson = (h.indices_type.w == 0u);

    // Bounds check
    let num_particles = arrayLength(&particles);
    if (p1 >= num_particles || p2 >= num_particles) {
        invalidate_hadron(h_idx);
        return;
    }
    if (!is_meson && p3 >= num_particles) {
        invalidate_hadron(h_idx);
        return;
    }

    // Check if constituent particles still exist and are quarks
    if (!is_quark(p1) || !is_quark(p2)) {
        invalidate_hadron(h_idx);
        return;
    }

    if (!is_meson && !is_quark(p3)) {
        invalidate_hadron(h_idx);
        return;
    }

    // Check distances between constituents
    let d12_sq = get_dist_sq(p1, p2);
    let breakup_dist = params.hadron.y;
    let breakup_sq = breakup_dist * breakup_dist;

    if (d12_sq > breakup_sq) {
        invalidate_hadron(h_idx);
        return;
    }

    if (!is_meson) {
        let d13_sq = get_dist_sq(p1, p3);
        let d23_sq = get_dist_sq(p2, p3);

        if (d13_sq > breakup_sq || d23_sq > breakup_sq) {
            invalidate_hadron(h_idx);
            return;
        }
    }

    // Hadron is still valid - update center of mass and velocity
    if (is_meson) {
        let center = (particles[p1].position.xyz + particles[p2].position.xyz) / 2.0;
        let velocity = (particles[p1].velocity.xyz + particles[p2].velocity.xyz) / 2.0;
        let radius = distance(center, particles[p1].position.xyz) + 0.2;

        hadrons[h_idx].center = vec4<f32>(center, radius);
        hadrons[h_idx].velocity = vec4<f32>(velocity, 0.0);
    } else {
        let center = (particles[p1].position.xyz + particles[p2].position.xyz + particles[p3].position.xyz) / 3.0;
        let velocity = (particles[p1].velocity.xyz + particles[p2].velocity.xyz + particles[p3].velocity.xyz) / 3.0;

        let r1 = distance(center, particles[p1].position.xyz);
        let r2 = distance(center, particles[p2].position.xyz);
        let r3 = distance(center, particles[p3].position.xyz);
        let radius = max(r1, max(r2, r3)) + 0.2;

        hadrons[h_idx].center = vec4<f32>(center, radius);
        hadrons[h_idx].velocity = vec4<f32>(velocity, 0.0);
    }
}
