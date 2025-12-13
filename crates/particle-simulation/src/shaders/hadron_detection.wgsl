// Compute shader for detecting hadron formation (Baryons and Mesons)
// This runs as a separate pass after the physics integration

// Constants (must match Rust and other shaders)
const STRONG_RANGE: f32 = 3.0;

// Particle Types
const TYPE_QUARK_UP: u32 = 0u;
const TYPE_QUARK_DOWN: u32 = 1u;

// Color Charges
const COLOR_RED: u32 = 0u;
const COLOR_GREEN: u32 = 1u;
const COLOR_BLUE: u32 = 2u;
const COLOR_ANTI_RED: u32 = 3u;
const COLOR_ANTI_GREEN: u32 = 4u;
const COLOR_ANTI_BLUE: u32 = 5u;

fn is_primary_color(c: u32) -> bool {
    return c == COLOR_RED || c == COLOR_GREEN || c == COLOR_BLUE;
}

fn is_anti_color(c: u32) -> bool {
    return c == COLOR_ANTI_RED || c == COLOR_ANTI_GREEN || c == COLOR_ANTI_BLUE;
}

// Returns true only for:
// - (Red, Green, Blue) in any order, OR
// - (AntiRed, AntiGreen, AntiBlue) in any order
//
// Mixed-sign triplets (e.g. AntiRed + Green + Blue) are NOT color singlets and must not form baryons.
fn is_colorless_triplet(c1: u32, c2: u32, c3: u32) -> bool {
    let all_primary = is_primary_color(c1) && is_primary_color(c2) && is_primary_color(c3);
    let all_anti = is_anti_color(c1) && is_anti_color(c2) && is_anti_color(c3);

    if (!all_primary && !all_anti) {
        return false;
    }

    // For either all-primary or all-anti, require one of each distinct color within that set
    return (c1 != c2) && (c1 != c3) && (c2 != c3);
}

// Hadron Types
const HADRON_MESON: u32 = 0u;
const HADRON_PROTON: u32 = 1u;  // uud
const HADRON_NEUTRON: u32 = 2u; // udd
const HADRON_BARYON_OTHER: u32 = 3u; // other combinations

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
    count: atomic<u32>,
    _pad: vec3<u32>,
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

fn get_dist(p1_idx: u32, p2_idx: u32) -> f32 {
    return sqrt(get_dist_sq(p1_idx, p2_idx));
}

fn is_quark(p_idx: u32) -> bool {
    let t = u32(particles[p_idx].position.w);
    return t == TYPE_QUARK_UP || t == TYPE_QUARK_DOWN;
}

fn get_color(p_idx: u32) -> u32 {
    return particles[p_idx].color_and_flags.x;
}

fn get_type(p_idx: u32) -> u32 {
    return u32(particles[p_idx].position.w);
}

fn is_bound(p_idx: u32) -> bool {
    let hadron_id = particles[p_idx].color_and_flags.z;
    if (hadron_id == 0u) {
        return false;
    }

    // Verify the hadron is actually valid
    // hadron_id is 1-indexed, convert to 0-indexed
    let h_idx = hadron_id - 1u;
    if (h_idx >= arrayLength(&hadrons)) {
        // Invalid index, clear it
        particles[p_idx].color_and_flags.z = 0u;
        return false;
    }

    let hadron = hadrons[h_idx];
    if (hadron.indices_type.w == 0xFFFFFFFFu) {
        // Hadron is invalid, clear the reference
        particles[p_idx].color_and_flags.z = 0u;
        return false;
    }

    return true;
}

// Find a free hadron slot (either beyond current count or an invalid slot)
fn find_free_slot() -> u32 {
    let current_count = atomicLoad(&counter.count);
    let max_hadrons = arrayLength(&hadrons);

    // First, look for invalid slots to reuse
    for (var i = 0u; i < current_count; i++) {
        if (hadrons[i].indices_type.w == 0xFFFFFFFFu) {
            return i;
        }
    }

    // No invalid slots found, try to allocate new one
    if (current_count < max_hadrons) {
        return atomicAdd(&counter.count, 1u);
    }

    // No space available
    return 0xFFFFFFFFu;
}

// Determine baryon type based on quark composition
fn identify_baryon(p1: u32, p2: u32, p3: u32) -> u32 {
    var up_count = 0u;
    var down_count = 0u;

    let t1 = get_type(p1);
    if (t1 == TYPE_QUARK_UP) { up_count++; }
    if (t1 == TYPE_QUARK_DOWN) { down_count++; }

    let t2 = get_type(p2);
    if (t2 == TYPE_QUARK_UP) { up_count++; }
    if (t2 == TYPE_QUARK_DOWN) { down_count++; }

    let t3 = get_type(p3);
    if (t3 == TYPE_QUARK_UP) { up_count++; }
    if (t3 == TYPE_QUARK_DOWN) { down_count++; }

    if (up_count == 2u && down_count == 1u) { return HADRON_PROTON; }
    if (up_count == 1u && down_count == 2u) { return HADRON_NEUTRON; }
    return HADRON_BARYON_OTHER;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if (index >= num_particles) {
        return;
    }

    // Only Quarks can initiate a search
    if (!is_quark(index)) {
        return;
    }

    // Skip quarks that are already bound to a hadron
    if (is_bound(index)) {
        return;
    }

    let my_color = get_color(index);

    // STRATEGY:
    // - Baryons: ONLY form from strict color-singlet triplets:
    //   (Red, Green, Blue) or (AntiRed, AntiGreen, AntiBlue).
    //   Mixed-sign triplets are NOT allowed.
    // - Mesons: still use (color + anti-color) pairing.

    var found_baryon = false;

    // --- BARYON SEARCH (strict RGB or AntiRGB only) ---
    {
        let binding_dist = params.hadron.x;
        let binding_sq = binding_dist * binding_dist;

        // Strict sign rule:
        // - If I'm a primary color, I only look for other primaries.
        // - If I'm an anti-color, I only look for other anti-colors.
        let want_primary = is_primary_color(my_color);
        let want_anti = is_anti_color(my_color);

        // If neither (shouldn't happen for quarks), skip.
        if (!want_primary && !want_anti) {
            // no-op
        } else {
            // Determine the exact two missing colors within the same sign set.
            var need_color_1: u32 = 0u;
            var need_color_2: u32 = 0u;

            if (want_primary) {
                if (my_color == COLOR_RED) {
                    need_color_1 = COLOR_GREEN;
                    need_color_2 = COLOR_BLUE;
                } else if (my_color == COLOR_GREEN) {
                    need_color_1 = COLOR_RED;
                    need_color_2 = COLOR_BLUE;
                } else { // COLOR_BLUE
                    need_color_1 = COLOR_RED;
                    need_color_2 = COLOR_GREEN;
                }
            } else { // want_anti
                if (my_color == COLOR_ANTI_RED) {
                    need_color_1 = COLOR_ANTI_GREEN;
                    need_color_2 = COLOR_ANTI_BLUE;
                } else if (my_color == COLOR_ANTI_GREEN) {
                    need_color_1 = COLOR_ANTI_RED;
                    need_color_2 = COLOR_ANTI_BLUE;
                } else { // COLOR_ANTI_BLUE
                    need_color_1 = COLOR_ANTI_RED;
                    need_color_2 = COLOR_ANTI_GREEN;
                }
            }

            var closest_1 = 0xFFFFFFFFu;
            var closest_2 = 0xFFFFFFFFu;
            var min_dist_sq_1 = binding_sq;
            var min_dist_sq_2 = binding_sq;

            for (var i = 0u; i < num_particles; i++) {
                if (i == index || !is_quark(i) || is_bound(i)) { continue; }

                let d_sq = get_dist_sq(index, i);
                if (d_sq > binding_sq) { continue; }

                let c = get_color(i);

                if (c == need_color_1) {
                    if (d_sq < min_dist_sq_1) {
                        min_dist_sq_1 = d_sq;
                        closest_1 = i;
                    }
                } else if (c == need_color_2) {
                    if (d_sq < min_dist_sq_2) {
                        min_dist_sq_2 = d_sq;
                        closest_2 = i;
                    }
                }
            }

        if (closest_1 != 0xFFFFFFFFu && closest_2 != 0xFFFFFFFFu) {
            // Ensure the two partners are also close to each other
            let d_12_sq = get_dist_sq(closest_1, closest_2);
            if (d_12_sq < binding_sq) {
                // Ensure the triplet is strictly colorless (RGB or AntiRGB)
                let c1 = my_color;
                let c2 = get_color(closest_1);
                let c3 = get_color(closest_2);

                if (is_colorless_triplet(c1, c2, c3)) {
                    // Try to acquire locks on the three quarks
                    let l1 = atomicCompareExchangeWeak(&locks[index], 0u, 1u).exchanged;
                    var l2 = false;
                    var l3 = false;

                    if (l1) {
                        l2 = atomicCompareExchangeWeak(&locks[closest_1], 0u, 1u).exchanged;
                        if (l2) {
                            l3 = atomicCompareExchangeWeak(&locks[closest_2], 0u, 1u).exchanged;
                        }
                    }

                    if (l1 && l2 && l3) {
                        let h_idx = find_free_slot();
                        if (h_idx != 0xFFFFFFFFu) {
                            let p1 = particles[index];
                            let p2 = particles[closest_1];
                            let p3 = particles[closest_2];

                            let center = (p1.position.xyz + p2.position.xyz + p3.position.xyz) / 3.0;
                            let velocity = (p1.velocity.xyz + p2.velocity.xyz + p3.velocity.xyz) / 3.0;

                            let r1 = distance(center, p1.position.xyz);
                            let r2 = distance(center, p2.position.xyz);
                            let r3 = distance(center, p3.position.xyz);
                            let radius = max(r1, max(r2, r3)) + 0.2;

                            var h: Hadron;
                            h.indices_type = vec4<u32>(
                                index,
                                closest_1,
                                closest_2,
                                identify_baryon(index, closest_1, closest_2)
                            );
                            h.center = vec4<f32>(center, radius);
                            h.velocity = vec4<f32>(velocity, 0.0);

                            hadrons[h_idx] = h;

                            // Set hadron_id on constituent particles (1-indexed)
                            particles[index].color_and_flags.z = h_idx + 1u;
                            particles[closest_1].color_and_flags.z = h_idx + 1u;
                            particles[closest_2].color_and_flags.z = h_idx + 1u;

                            found_baryon = true;
                        } else {
                            // Slot allocation failed: release locks to avoid "stuck" quarks.
                            atomicStore(&locks[index], 0u);
                            atomicStore(&locks[closest_1], 0u);
                            atomicStore(&locks[closest_2], 0u);
                        }
                    } else {
                        if (l1) { atomicStore(&locks[index], 0u); }
                        if (l2) { atomicStore(&locks[closest_1], 0u); }
                        if (l3) { atomicStore(&locks[closest_2], 0u); }
                    }
                }
            }
        }
    }

        // (obsolete) RGB-only baryon detection block removed:
        // baryon formation is now handled by the generalized colorless-triplet search above.
    }

    // --- MESON SEARCH (Color looks for Anti-Color) ---
    // Only run if we didn't just form a baryon.
    // Leaders: Red, Green, Blue (looking for AntiRed, AntiGreen, AntiBlue)
    if (!found_baryon && my_color <= COLOR_BLUE) {
        let target_anti = my_color + 3u; // Red(0)->AntiRed(3), etc.
        let binding_dist = params.hadron.x;

        var closest_anti = 0xFFFFFFFFu;
        var min_dist_sq = binding_dist * binding_dist;

        for (var i = 0u; i < num_particles; i++) {
            if (i == index || !is_quark(i) || is_bound(i)) { continue; }

            let c = get_color(i);
            if (c == target_anti) {
                let d_sq = get_dist_sq(index, i);
                if (d_sq < min_dist_sq) {
                    min_dist_sq = d_sq;
                    closest_anti = i;
                }
            }
        }

        if (closest_anti != 0xFFFFFFFFu) {
            // Try to acquire locks
            let l1 = atomicCompareExchangeWeak(&locks[index], 0u, 1u).exchanged;
            var l2 = false;

            if (l1) {
                l2 = atomicCompareExchangeWeak(&locks[closest_anti], 0u, 1u).exchanged;
            }

            if (l1 && l2) {
                // Found a Meson! Find a free hadron slot
                let h_idx = find_free_slot();
                if (h_idx != 0xFFFFFFFFu) {
                    let p1 = particles[index];
                    let p2 = particles[closest_anti];

                    let center = (p1.position.xyz + p2.position.xyz) / 2.0;
                    let velocity = (p1.velocity.xyz + p2.velocity.xyz) / 2.0;
                    let radius = distance(center, p1.position.xyz) + 0.2;

                    var h: Hadron;
                    h.indices_type = vec4<u32>(
                        index,
                        closest_anti,
                        0xFFFFFFFFu,
                        HADRON_MESON
                    );
                    h.center = vec4<f32>(center, radius);
                    h.velocity = vec4<f32>(velocity, 0.0);

                    hadrons[h_idx] = h;

                    // Set hadron_id on constituent particles (1-indexed)
                    particles[index].color_and_flags.z = h_idx + 1u;
                    particles[closest_anti].color_and_flags.z = h_idx + 1u;
                } else {
                    // Slot allocation failed: release locks to avoid "stuck" quarks.
                    atomicStore(&locks[index], 0u);
                    atomicStore(&locks[closest_anti], 0u);
                }
            } else {
                // Failed, release locks
                if (l1) { atomicStore(&locks[index], 0u); }
                if (l2) { atomicStore(&locks[closest_anti], 0u); }
            }
        }
    }
}
