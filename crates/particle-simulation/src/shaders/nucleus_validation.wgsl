// Compute shader for validating existing nuclei
// Checks if constituent hadrons (nucleons) are still bound, breaks up nuclei if not

// Maximum number of nucleons in a nucleus (must match Rust)
const MAX_NUCLEONS: u32 = 16u;

// Hadron Types
const HADRON_MESON: u32 = 0u;
const HADRON_PROTON: u32 = 1u;
const HADRON_NEUTRON: u32 = 2u;
const HADRON_BARYON_OTHER: u32 = 3u;

struct Hadron {
    indices_type: vec4<u32>, // x=p1, y=p2, z=p3, w=type_id
    center: vec4<f32>,       // xyz = center of mass, w = radius
    velocity: vec4<f32>,     // xyz = velocity, w = nucleus_id (as f32, 0 = unbound)
}

struct Nucleus {
    hadron_indices: array<u32, MAX_NUCLEONS>, // Indices of constituent hadrons, 0xFFFFFFFF = unused
    nucleon_count: u32,
    proton_count: u32,
    neutron_count: u32,
    type_id: u32,       // Atomic number (Z) or 0xFFFFFFFF for invalid
    center: vec4<f32>,  // xyz = center of mass, w = radius
    velocity: vec4<f32>, // xyz = velocity, w = padding
}

struct NucleusCounter {
    count: atomic<u32>,
    _pad: vec3<u32>,
}

struct PhysicsParams {
    constants: vec4<f32>,
    strong_force: vec4<f32>,
    repulsion: vec4<f32>,
    integration: vec4<f32>,
    nucleon: vec4<f32>, // x: binding_strength, y: binding_range, z: exclusion_strength, w: exclusion_radius
    electron: vec4<f32>,
    hadron: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read_write> hadrons: array<Hadron>;

@group(0) @binding(1)
var<storage, read_write> nuclei: array<Nucleus>;

@group(0) @binding(2)
var<storage, read_write> counter: NucleusCounter;

@group(0) @binding(3)
var<storage, read_write> locks: array<atomic<u32>>;

@group(0) @binding(4)
var<uniform> params: PhysicsParams;

// Check if hadron is valid
fn is_valid_hadron(h_idx: u32) -> bool {
    if (h_idx >= arrayLength(&hadrons) || h_idx == 0xFFFFFFFFu) {
        return false;
    }
    return hadrons[h_idx].indices_type.w != 0xFFFFFFFFu;
}

// Check if hadron is a nucleon
fn is_nucleon(type_id: u32) -> bool {
    return type_id == HADRON_PROTON || type_id == HADRON_NEUTRON;
}

// Get distance between two hadrons
fn get_distance_sq(h1_idx: u32, h2_idx: u32) -> f32 {
    let pos1 = hadrons[h1_idx].center.xyz;
    let pos2 = hadrons[h2_idx].center.xyz;
    let diff = pos2 - pos1;
    return dot(diff, diff);
}

// Mark nucleus as invalid and clear hadron nucleus_ids
fn invalidate_nucleus(n_idx: u32) {
    let nucleus = nuclei[n_idx];

    // Clear nucleus_id from all constituent hadrons
    for (var i = 0u; i < nucleus.nucleon_count; i++) {
        let h_idx = nucleus.hadron_indices[i];
        if (h_idx < arrayLength(&hadrons) && h_idx != 0xFFFFFFFFu) {
            hadrons[h_idx].velocity.w = 0.0; // Clear nucleus_id
        }
    }

    nuclei[n_idx].type_id = 0xFFFFFFFFu;
    nuclei[n_idx].nucleon_count = 0u;
    nuclei[n_idx].proton_count = 0u;
    nuclei[n_idx].neutron_count = 0u;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let n_idx = global_id.x;
    let num_nuclei = atomicLoad(&counter.count);

    if (n_idx >= num_nuclei) {
        return;
    }

    let nucleus = nuclei[n_idx];

    // Skip already invalid nuclei
    if (nucleus.type_id == 0xFFFFFFFFu) {
        return;
    }

    // Nucleus must have at least 2 nucleons
    if (nucleus.nucleon_count < 2u) {
        invalidate_nucleus(n_idx);
        return;
    }

    // Check if all constituent hadrons still exist and are valid nucleons
    var valid_count = 0u;
    var proton_count = 0u;
    var neutron_count = 0u;

    for (var i = 0u; i < nucleus.nucleon_count; i++) {
        let h_idx = nucleus.hadron_indices[i];

        if (!is_valid_hadron(h_idx)) {
            invalidate_nucleus(n_idx);
            return;
        }

        let hadron_type = hadrons[h_idx].indices_type.w;
        if (!is_nucleon(hadron_type)) {
            invalidate_nucleus(n_idx);
            return;
        }

        valid_count++;

        if (hadron_type == HADRON_PROTON) {
            proton_count++;
        } else if (hadron_type == HADRON_NEUTRON) {
            neutron_count++;
        }
    }

    // Check distances between all pairs of nucleons
    // Use nucleon exclusion radius * 2 as breakup threshold (nucleons should be touching)
    let breakup_dist = params.nucleon.w * 3.0; // 3x exclusion radius as breakup threshold
    let breakup_sq = breakup_dist * breakup_dist;

    for (var i = 0u; i < nucleus.nucleon_count; i++) {
        let h1_idx = nucleus.hadron_indices[i];

        for (var j = i + 1u; j < nucleus.nucleon_count; j++) {
            let h2_idx = nucleus.hadron_indices[j];

            let dist_sq = get_distance_sq(h1_idx, h2_idx);

            if (dist_sq > breakup_sq) {
                invalidate_nucleus(n_idx);
                return;
            }
        }
    }

    // Nucleus is still valid - update center of mass, velocity, and radius
    var center_sum = vec3<f32>(0.0);
    var velocity_sum = vec3<f32>(0.0);
    var max_dist = 0.0;

    for (var i = 0u; i < nucleus.nucleon_count; i++) {
        let h_idx = nucleus.hadron_indices[i];
        let hadron = hadrons[h_idx];

        center_sum += hadron.center.xyz;
        velocity_sum += hadron.velocity.xyz;
    }

    let center = center_sum / f32(nucleus.nucleon_count);
    let velocity = velocity_sum / f32(nucleus.nucleon_count);

    // Calculate radius (max distance from center + padding)
    for (var i = 0u; i < nucleus.nucleon_count; i++) {
        let h_idx = nucleus.hadron_indices[i];
        let hadron = hadrons[h_idx];
        let dist = distance(center, hadron.center.xyz);
        max_dist = max(max_dist, dist + hadron.center.w); // Include hadron radius
    }

    // Update nucleus
    nuclei[n_idx].proton_count = proton_count;
    nuclei[n_idx].neutron_count = neutron_count;
    nuclei[n_idx].type_id = proton_count; // Atomic number Z = proton count
    nuclei[n_idx].center = vec4<f32>(center, max_dist + 0.5); // + padding
    nuclei[n_idx].velocity = vec4<f32>(velocity, 0.0);
}
