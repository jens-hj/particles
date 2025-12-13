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
    hadron: vec4<f32>, // x: binding_distance, y: breakup_distance, z: quark_electron_repulsion, w: quark_electron_radius
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
    return particles[p_idx].color_and_flags.z != 0u;
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
    // To avoid duplicates, we assign specific "Leaders" to search for partners.
    // - Red Quarks search for Green + Blue (Baryons)
    // - If Baryon fails, Red/Green/Blue search for Anti-Color (Mesons)
    //   (Note: This simple logic might miss some mesons if the Red is consumed by a failed baryon search,
    //    but it's good enough for visualization).

    var found_baryon = false;

    // --- BARYON SEARCH (Red looks for Green + Blue) ---
    if (my_color == COLOR_RED) {
        var closest_green = 0xFFFFFFFFu;
        var closest_blue = 0xFFFFFFFFu;
        let binding_dist = params.hadron.x;
        var min_dist_sq_green = binding_dist * binding_dist;
        var min_dist_sq_blue = binding_dist * binding_dist;

        // Find closest Green and Blue neighbors (only unbound quarks)
        for (var i = 0u; i < num_particles; i++) {
            if (i == index || !is_quark(i) || is_bound(i)) { continue; }

            let d_sq = get_dist_sq(index, i);
            if (d_sq > binding_dist * binding_dist) { continue; }

            let c = get_color(i);
            if (c == COLOR_GREEN) {
                if (d_sq < min_dist_sq_green) {
                    min_dist_sq_green = d_sq;
                    closest_green = i;
                }
            } else if (c == COLOR_BLUE) {
                if (d_sq < min_dist_sq_blue) {
                    min_dist_sq_blue = d_sq;
                    closest_blue = i;
                }
            }
        }

        // If both found, check if they are also close to each other
        if (closest_green != 0xFFFFFFFFu && closest_blue != 0xFFFFFFFFu) {
            let d_gb_sq = get_dist_sq(closest_green, closest_blue);
            if (d_gb_sq < binding_dist * binding_dist) {
                // Try to acquire locks
                let l1 = atomicCompareExchangeWeak(&locks[index], 0u, 1u).exchanged;
                var l2 = false;
                var l3 = false;

                if (l1) {
                    l2 = atomicCompareExchangeWeak(&locks[closest_green], 0u, 1u).exchanged;
                    if (l2) {
                        l3 = atomicCompareExchangeWeak(&locks[closest_blue], 0u, 1u).exchanged;
                    }
                }

                if (l1 && l2 && l3) {
                    // Found a Baryon! Find a free hadron slot
                    let h_idx = find_free_slot();
                    if (h_idx != 0xFFFFFFFFu) {
                        let p1 = particles[index];
                        let p2 = particles[closest_green];
                        let p3 = particles[closest_blue];

                        // Calculate center of mass (assuming equal mass for simplicity or use .w)
                        let center = (p1.position.xyz + p2.position.xyz + p3.position.xyz) / 3.0;
                        let velocity = (p1.velocity.xyz + p2.velocity.xyz + p3.velocity.xyz) / 3.0;

                        // Calculate radius (max distance from center)
                        let r1 = distance(center, p1.position.xyz);
                        let r2 = distance(center, p2.position.xyz);
                        let r3 = distance(center, p3.position.xyz);
                        let radius = max(r1, max(r2, r3)) + 0.2; // + padding

                        var h: Hadron;
                        h.indices_type = vec4<u32>(
                            index,
                            closest_green,
                            closest_blue,
                            identify_baryon(index, closest_green, closest_blue)
                        );
                        h.center = vec4<f32>(center, radius);
                        h.velocity = vec4<f32>(velocity, 0.0);

                        hadrons[h_idx] = h;

                        // Set hadron_id on constituent particles (1-indexed)
                        particles[index].color_and_flags.z = h_idx + 1u;
                        particles[closest_green].color_and_flags.z = h_idx + 1u;
                        particles[closest_blue].color_and_flags.z = h_idx + 1u;
                    }
                    found_baryon = true;
                } else {
                    // Failed, release locks
                    if (l1) { atomicStore(&locks[index], 0u); }
                    if (l2) { atomicStore(&locks[closest_green], 0u); }
                    if (l3) { atomicStore(&locks[closest_blue], 0u); }
                }
            }
        }
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
                }
            } else {
                // Failed, release locks
                if (l1) { atomicStore(&locks[index], 0u); }
                if (l2) { atomicStore(&locks[closest_anti], 0u); }
            }
        }
    }
}
