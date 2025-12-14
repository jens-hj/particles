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

struct HadronCounter {
    // 4x u32 counters (atomics):
    // [0] total hadrons (counter range; may include invalid slots)
    // [1] protons
    // [2] neutrons
    // [3] other
    counters: array<atomic<u32>, 4>,
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

@group(0) @binding(5)
var<storage, read> hadron_counter: HadronCounter;

fn is_bound_to_nucleus(h_idx: u32) -> bool {
    return u32(hadrons[h_idx].velocity.w) != 0u;
}

fn nucleus_id_to_index(nucleus_id: u32) -> u32 {
    // nucleus_id is 1-indexed, 0 means unbound.
    return nucleus_id - 1u;
}

fn contains_hadron(nucleus: Nucleus, h_idx: u32) -> bool {
    for (var i = 0u; i < nucleus.nucleon_count; i++) {
        if (nucleus.hadron_indices[i] == h_idx) {
            return true;
        }
    }
    return false;
}

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

    // Fast skip for invalid nuclei
    if (nuclei[n_idx].type_id == 0xFFFFFFFFu) {
        return;
    }

    let self_id = n_idx + 1u;

    // Find a representative hadron to lock for this nucleus.
    // (Don’t assume hadron_indices[0] is valid.)
    var rep = 0xFFFFFFFFu;
    let pre_nucleus = nuclei[n_idx];
    let pre_count = min(pre_nucleus.nucleon_count, MAX_NUCLEONS);
    for (var i = 0u; i < pre_count; i++) {
        let h_idx = pre_nucleus.hadron_indices[i];
        if (!is_valid_hadron(h_idx)) {
            continue;
        }
        if (!is_nucleon(hadrons[h_idx].indices_type.w)) {
            continue;
        }
        if (u32(hadrons[h_idx].velocity.w) != self_id) {
            continue;
        }
        rep = h_idx;
        break;
    }

    if (rep == 0xFFFFFFFFu) {
        invalidate_nucleus(n_idx);
        return;
    }

    if (!atomicCompareExchangeWeak(&locks[rep], 0u, 1u).exchanged) {
        return;
    }

    // Hysteresis: formation uses `params.nucleon.y` (binding_range),
    // but validation uses a larger breakup distance to reduce flicker.
    let breakup_dist = max(params.nucleon.y * 2.0, params.nucleon.w * 3.0);
    let breakup_sq = breakup_dist * breakup_dist;

    // Absorb/merge distances.
    let attach_dist = params.nucleon.y * 1.25;
    let merge_dist = params.nucleon.y * 1.75;
    let attach_sq = attach_dist * attach_dist;
    let merge_sq = merge_dist * merge_dist;

    // Snapshot indices before we compact.
    var original: array<u32, MAX_NUCLEONS>;
    for (var i = 0u; i < MAX_NUCLEONS; i++) {
        original[i] = nuclei[n_idx].hadron_indices[i];
    }

    // Compact + validate membership.
    var count = 0u;
    var proton_count = 0u;
    var neutron_count = 0u;
    var center_sum = vec3<f32>(0.0);
    var velocity_sum = vec3<f32>(0.0);

    let original_count = min(nuclei[n_idx].nucleon_count, MAX_NUCLEONS);
    for (var i = 0u; i < original_count; i++) {
        let h_idx = original[i];
        if (!is_valid_hadron(h_idx)) {
            continue;
        }
        let t = hadrons[h_idx].indices_type.w;
        if (!is_nucleon(t)) {
            continue;
        }
        // Only keep hadrons that still claim membership in this nucleus.
        if (u32(hadrons[h_idx].velocity.w) != self_id) {
            continue;
        }

        nuclei[n_idx].hadron_indices[count] = h_idx;
        count++;

        center_sum += hadrons[h_idx].center.xyz;
        velocity_sum += hadrons[h_idx].velocity.xyz;
        if (t == HADRON_PROTON) {
            proton_count++;
        } else {
            neutron_count++;
        }
    }

    for (var i = count; i < MAX_NUCLEONS; i++) {
        nuclei[n_idx].hadron_indices[i] = 0xFFFFFFFFu;
    }
    nuclei[n_idx].nucleon_count = count;

    if (count < 1u || proton_count < 1u) {
        invalidate_nucleus(n_idx);
        atomicStore(&locks[rep], 0u);
        return;
    }

    var center = center_sum / f32(count);
    var velocity = velocity_sum / f32(count);

    // Absorb nearby *unbound* nucleons.
    let num_hadrons = min(atomicLoad(&hadron_counter.counters[0]), arrayLength(&hadrons));
    for (var h = 0u; h < num_hadrons && count < MAX_NUCLEONS; h++) {
        if (!is_valid_hadron(h)) {
            continue;
        }
        let t = hadrons[h].indices_type.w;
        if (!is_nucleon(t)) {
            continue;
        }
        if (u32(hadrons[h].velocity.w) != 0u) {
            continue;
        }
        let diff = hadrons[h].center.xyz - center;
        if (dot(diff, diff) > attach_sq) {
            continue;
        }

        if (!atomicCompareExchangeWeak(&locks[h], 0u, 1u).exchanged) {
            continue;
        }

        // Re-check under lock.
        if (is_valid_hadron(h) && is_nucleon(hadrons[h].indices_type.w) && u32(hadrons[h].velocity.w) == 0u) {
            nuclei[n_idx].hadron_indices[count] = h;
            count++;
            nuclei[n_idx].nucleon_count = count;
            hadrons[h].velocity.w = f32(self_id);

            center_sum += hadrons[h].center.xyz;
            velocity_sum += hadrons[h].velocity.xyz;
            if (t == HADRON_PROTON) {
                proton_count++;
            } else {
                neutron_count++;
            }

            center = center_sum / f32(count);
            velocity = velocity_sum / f32(count);
        }

        atomicStore(&locks[h], 0u);
    }

    // Merge with overlapping nuclei. Lower index wins.
    for (var other_idx = 0u; other_idx < num_nuclei && count < MAX_NUCLEONS; other_idx++) {
        if (other_idx == n_idx) {
            continue;
        }
        if (n_idx > other_idx) {
            continue;
        }

        let other = nuclei[other_idx];
        if (other.type_id == 0xFFFFFFFFu || other.nucleon_count < 1u) {
            continue;
        }

        let dc = other.center.xyz - center;
        if (dot(dc, dc) > merge_sq) {
            continue;
        }

        // Need full capacity to merge, otherwise skip (avoid ghost bindings).
        if (count + other.nucleon_count > MAX_NUCLEONS) {
            continue;
        }

        // Find a valid representative for the other nucleus.
        var other_rep = 0xFFFFFFFFu;
        let other_id = other_idx + 1u;
        let other_count0 = min(other.nucleon_count, MAX_NUCLEONS);
        for (var i = 0u; i < other_count0; i++) {
            let h_idx = other.hadron_indices[i];
            if (!is_valid_hadron(h_idx)) {
                continue;
            }
            if (!is_nucleon(hadrons[h_idx].indices_type.w)) {
                continue;
            }
            if (u32(hadrons[h_idx].velocity.w) != other_id) {
                continue;
            }
            other_rep = h_idx;
            break;
        }

        if (other_rep == 0xFFFFFFFFu) {
            continue;
        }

        if (!atomicCompareExchangeWeak(&locks[other_rep], 0u, 1u).exchanged) {
            continue;
        }

        // Re-fetch under lock.
        let other2 = nuclei[other_idx];
        if (other2.type_id == 0xFFFFFFFFu || other2.nucleon_count < 1u) {
            atomicStore(&locks[other_rep], 0u);
            continue;
        }

        if (count + other2.nucleon_count > MAX_NUCLEONS) {
            atomicStore(&locks[other_rep], 0u);
            continue;
        }

        // Snapshot other indices.
        var other_indices: array<u32, MAX_NUCLEONS>;
        for (var i = 0u; i < MAX_NUCLEONS; i++) {
            other_indices[i] = nuclei[other_idx].hadron_indices[i];
        }

        let other_count = min(other2.nucleon_count, MAX_NUCLEONS);
        for (var i = 0u; i < other_count; i++) {
            let h_idx = other_indices[i];
            if (!is_valid_hadron(h_idx)) {
                continue;
            }
            let t = hadrons[h_idx].indices_type.w;
            if (!is_nucleon(t)) {
                continue;
            }
            if (u32(hadrons[h_idx].velocity.w) != other_id) {
                continue;
            }

            nuclei[n_idx].hadron_indices[count] = h_idx;
            count++;
            nuclei[n_idx].nucleon_count = count;
            hadrons[h_idx].velocity.w = f32(self_id);

            center_sum += hadrons[h_idx].center.xyz;
            velocity_sum += hadrons[h_idx].velocity.xyz;
            if (t == HADRON_PROTON) {
                proton_count++;
            } else {
                neutron_count++;
            }

            nuclei[other_idx].hadron_indices[i] = 0xFFFFFFFFu;
        }

        // Invalidate loser nucleus without clearing hadron nucleus_ids (they were reassigned above).
        for (var i = 0u; i < MAX_NUCLEONS; i++) {
            nuclei[other_idx].hadron_indices[i] = 0xFFFFFFFFu;
        }
        nuclei[other_idx].type_id = 0xFFFFFFFFu;
        nuclei[other_idx].nucleon_count = 0u;
        nuclei[other_idx].proton_count = 0u;
        nuclei[other_idx].neutron_count = 0u;

        center = center_sum / f32(count);
        velocity = velocity_sum / f32(count);

        atomicStore(&locks[other_rep], 0u);
    }

    // Final validation: breakup check vs updated center.
    for (var i = 0u; i < count; i++) {
        let h_idx = nuclei[n_idx].hadron_indices[i];
        let diff = hadrons[h_idx].center.xyz - center;
        if (dot(diff, diff) > breakup_sq) {
            invalidate_nucleus(n_idx);
            atomicStore(&locks[rep], 0u);
            return;
        }
    }

    // Radius from updated membership.
    var max_dist = 0.0;
    for (var i = 0u; i < count; i++) {
        let h_idx = nuclei[n_idx].hadron_indices[i];
        let dist = distance(center, hadrons[h_idx].center.xyz);
        max_dist = max(max_dist, dist + hadrons[h_idx].center.w);
    }

    // Update nucleus with consistent counts and center.
    nuclei[n_idx].proton_count = proton_count;
    nuclei[n_idx].neutron_count = neutron_count;
    nuclei[n_idx].type_id = proton_count;
    nuclei[n_idx].center = vec4<f32>(center, max_dist + 0.5);
    nuclei[n_idx].velocity = vec4<f32>(velocity, 0.0);

    atomicStore(&locks[rep], 0u);
}

// Per-frame nucleus reset (used when running detection fresh each frame).
// - Clears hadron.velocity.w nucleus_id for active hadrons
// - Marks all nuclei slots as invalid
@compute @workgroup_size(256)
fn reset_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    // Clear nucleus_id for active hadron slots.
    let num_hadrons = min(atomicLoad(&hadron_counter.counters[0]), arrayLength(&hadrons));
    if (idx < num_hadrons) {
        let v = hadrons[idx].velocity.xyz;
        hadrons[idx].velocity = vec4<f32>(v, 0.0);
    }

    // Mark all nuclei as invalid.
    if (idx < arrayLength(&nuclei)) {
        for (var i = 0u; i < MAX_NUCLEONS; i++) {
            nuclei[idx].hadron_indices[i] = 0xFFFFFFFFu;
        }
        nuclei[idx].nucleon_count = 0u;
        nuclei[idx].proton_count = 0u;
        nuclei[idx].neutron_count = 0u;
        nuclei[idx].type_id = 0xFFFFFFFFu;
        nuclei[idx].center = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        nuclei[idx].velocity = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}
