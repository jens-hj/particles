// Compute shader for detecting nucleus formation
// Detects clusters of nucleons (protons and neutrons = hadrons) that are bound together

// Maximum number of nucleons in a nucleus (must match Rust)
const MAX_NUCLEONS: u32 = 16u;

// Hadron Types (must match Rust/other shaders)
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

// Check if hadron is a nucleon (proton or neutron)
fn is_nucleon(type_id: u32) -> bool {
    return type_id == HADRON_PROTON || type_id == HADRON_NEUTRON;
}

// Check if hadron is valid (not marked as invalid)
fn is_valid_hadron(h_idx: u32) -> bool {
    if (h_idx >= arrayLength(&hadrons)) {
        return false;
    }
    return hadrons[h_idx].indices_type.w != 0xFFFFFFFFu;
}

// Check if hadron is already part of a nucleus (fast version using nucleus_id)
fn is_bound_to_nucleus(h_idx: u32) -> bool {
    let nucleus_id = u32(hadrons[h_idx].velocity.w);
    return nucleus_id != 0u;
}

// Get distance between two hadrons
fn get_distance(h1_idx: u32, h2_idx: u32) -> f32 {
    let pos1 = hadrons[h1_idx].center.xyz;
    let pos2 = hadrons[h2_idx].center.xyz;
    return distance(pos1, pos2);
}

// Get relative velocity magnitude
fn get_relative_velocity(h1_idx: u32, h2_idx: u32) -> f32 {
    let vel1 = hadrons[h1_idx].velocity.xyz;
    let vel2 = hadrons[h2_idx].velocity.xyz;
    return length(vel1 - vel2);
}

// Find a free nucleus slot
fn find_free_slot() -> u32 {
    let current_count = atomicLoad(&counter.count);
    let max_nuclei = arrayLength(&nuclei);

    // First, look for invalid slots to reuse
    for (var i = 0u; i < current_count; i++) {
        if (nuclei[i].type_id == 0xFFFFFFFFu) {
            return i;
        }
    }

    // No invalid slots found, try to allocate new one
    if (current_count < max_nuclei) {
        return atomicAdd(&counter.count, 1u);
    }

    // No space available
    return 0xFFFFFFFFu;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_hadrons = arrayLength(&hadrons);

    if (index >= num_hadrons) {
        return;
    }

    // Only nucleons can initiate nucleus search
    if (!is_valid_hadron(index)) {
        return;
    }

    let my_type = hadrons[index].indices_type.w;
    if (!is_nucleon(my_type)) {
        return;
    }

    // Skip hadrons already bound to a nucleus
    if (is_bound_to_nucleus(index)) {
        return;
    }

    // Search for nearby nucleons to form a nucleus
    // Use nucleon binding range from params
    let binding_range = params.nucleon.y;
    let max_relative_velocity = 5.0; // Threshold for stable binding

    var nearby_nucleons: array<u32, MAX_NUCLEONS>;
    var nearby_count = 0u;

    // Add self
    nearby_nucleons[nearby_count] = index;
    nearby_count++;

    // Find nearby nucleons
    for (var i = 0u; i < num_hadrons && nearby_count < MAX_NUCLEONS; i++) {
        if (i == index) {
            continue;
        }

        if (!is_valid_hadron(i)) {
            continue;
        }

        let other_type = hadrons[i].indices_type.w;
        if (!is_nucleon(other_type)) {
            continue;
        }

        if (is_bound_to_nucleus(i)) {
            continue;
        }

        let dist = get_distance(index, i);
        if (dist > binding_range) {
            continue;
        }

        let rel_vel = get_relative_velocity(index, i);
        if (rel_vel > max_relative_velocity) {
            continue;
        }

        nearby_nucleons[nearby_count] = i;
        nearby_count++;
    }

    // Need at least 2 nucleons to form a nucleus
    if (nearby_count < 2u) {
        return;
    }

    // Try to acquire locks on all nearby nucleons
    var locks_acquired: array<bool, MAX_NUCLEONS>;
    var all_locked = true;

    for (var i = 0u; i < nearby_count; i++) {
        let h_idx = nearby_nucleons[i];
        locks_acquired[i] = atomicCompareExchangeWeak(&locks[h_idx], 0u, 1u).exchanged;
        if (!locks_acquired[i]) {
            all_locked = false;
            break;
        }
    }

    if (!all_locked) {
        // Release any locks we did acquire
        for (var i = 0u; i < nearby_count; i++) {
            if (locks_acquired[i]) {
                atomicStore(&locks[nearby_nucleons[i]], 0u);
            }
        }
        return;
    }

    // Re-check that none of the hadrons were bound by another thread after we checked but before we locked
    for (var i = 0u; i < nearby_count; i++) {
        if (is_bound_to_nucleus(nearby_nucleons[i])) {
            // One of the hadrons is now bound, release all locks and abort
            for (var j = 0u; j < nearby_count; j++) {
                atomicStore(&locks[nearby_nucleons[j]], 0u);
            }
            return;
        }
    }

    // Successfully locked all nucleons and verified they're all unbound - form a nucleus
    let n_idx = find_free_slot();
    if (n_idx == 0xFFFFFFFFu) {
        // No space for nucleus, release locks
        for (var i = 0u; i < nearby_count; i++) {
            atomicStore(&locks[nearby_nucleons[i]], 0u);
        }
        return;
    }

    // Calculate nucleus properties
    var center_sum = vec3<f32>(0.0);
    var velocity_sum = vec3<f32>(0.0);
    var proton_count = 0u;
    var neutron_count = 0u;
    var max_dist = 0.0;

    for (var i = 0u; i < nearby_count; i++) {
        let h_idx = nearby_nucleons[i];
        let hadron = hadrons[h_idx];

        center_sum += hadron.center.xyz;
        velocity_sum += hadron.velocity.xyz;

        if (hadron.indices_type.w == HADRON_PROTON) {
            proton_count++;
        } else if (hadron.indices_type.w == HADRON_NEUTRON) {
            neutron_count++;
        }
    }

    let center = center_sum / f32(nearby_count);
    let velocity = velocity_sum / f32(nearby_count);

    // Calculate radius (max distance from center + padding)
    for (var i = 0u; i < nearby_count; i++) {
        let h_idx = nearby_nucleons[i];
        let dist = distance(center, hadrons[h_idx].center.xyz);
        max_dist = max(max_dist, dist + hadrons[h_idx].center.w); // Include hadron radius
    }

    // Create nucleus
    var nucleus: Nucleus;
    for (var i = 0u; i < MAX_NUCLEONS; i++) {
        if (i < nearby_count) {
            nucleus.hadron_indices[i] = nearby_nucleons[i];
        } else {
            nucleus.hadron_indices[i] = 0xFFFFFFFFu;
        }
    }
    nucleus.nucleon_count = nearby_count;
    nucleus.proton_count = proton_count;
    nucleus.neutron_count = neutron_count;
    nucleus.type_id = proton_count; // Atomic number Z = proton count
    nucleus.center = vec4<f32>(center, max_dist + 0.5); // + padding
    nucleus.velocity = vec4<f32>(velocity, 0.0);

    nuclei[n_idx] = nucleus;

    // Set nucleus_id on all constituent hadrons (1-indexed, 0 = unbound)
    for (var i = 0u; i < nearby_count; i++) {
        let h_idx = nearby_nucleons[i];
        hadrons[h_idx].velocity.w = f32(n_idx + 1u);
    }

    // Release locks
    for (var i = 0u; i < nearby_count; i++) {
        atomicStore(&locks[nearby_nucleons[i]], 0u);
    }
}
