// Picking shader: renders entity IDs into an offscreen RGBA8 target.
//
// Encoding convention (32-bit ID packed into RGBA8):
// - id == 0u: "no hit" / background
// - otherwise: application-defined:
//   - for now: particles write (index + 1)
//   - hadrons write 0x8000_0000 | (hadron_index + 1)
//
// IMPORTANT (uniqueness / semantics):
// - IDs are derived from `@builtin(instance_index)` at draw time.
// - This means IDs are unique per *buffer slot* / instance, not per "physical particle identity"
//   (if your simulation compacts/reorders buffers over time, the same physical particle may
//   appear under a different ID in later frames).
// - Within a single picking pass, there should be no accidental ID collisions as long as each
//   instance_index maps to exactly one slot.
//
// NOTE: This shader is intentionally minimal and independent from the visual shaders.
// It shares buffer layouts with `particle.wgsl` / `hadron.wgsl` for compatibility.

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    particle_size: f32,
    time: f32,
    lod_shell_fade_start: f32,
    lod_shell_fade_end: f32,
    lod_bond_fade_start: f32,
    lod_bond_fade_end: f32,
    lod_quark_fade_start: f32,
    lod_quark_fade_end: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type (as f32)
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z = hadron_id (1-indexed), w = padding
}

@group(0) @binding(1)
var<storage, read> particles: array<Particle>;

struct Hadron {
    indices_type: vec4<u32>, // x=p1, y=p2, z=p3, w=type_id
    center: vec4<f32>,       // xyz=center, w=radius
    velocity: vec4<f32>,     // xyz=velocity, w=padding
}

@group(0) @binding(2)
var<storage, read> hadrons: array<Hadron>;

struct HadronCounter {
    // [0] total hadrons (counter range; may include invalid slots)
    // [1] protons
    // [2] neutrons
    // [3] other
    counters: vec4<u32>,
}

@group(0) @binding(3)
var<storage, read> hadron_counter: HadronCounter;

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) id: u32,
    @location(1) uv: vec2<f32>,
}

fn pack_u32_to_rgba8(id: u32) -> vec4<f32> {
    // Pack little-endian: r=LSB ... a=MSB
    let r: u32 = (id >> 0u) & 0xFFu;
    let g: u32 = (id >> 8u) & 0xFFu;
    let b: u32 = (id >> 16u) & 0xFFu;
    let a: u32 = (id >> 24u) & 0xFFu;

    // Write as UNORM (0..1). When copied to buffer, bytes match 0..255.
    return vec4<f32>(
        f32(r) / 255.0,
        f32(g) / 255.0,
        f32(b) / 255.0,
        f32(a) / 255.0
    );
}

fn quad_vertex(vertex_index: u32) -> vec2<f32> {
    // Two triangles (6 verts) covering [-1,1]x[-1,1]
    switch (vertex_index) {
        case 0u: { return vec2<f32>(-1.0, -1.0); }
        case 1u: { return vec2<f32>( 1.0, -1.0); }
        case 2u: { return vec2<f32>( 1.0,  1.0); }
        case 3u: { return vec2<f32>(-1.0, -1.0); }
        case 4u: { return vec2<f32>( 1.0,  1.0); }
        default: { return vec2<f32>(-1.0,  1.0); } // 5
    }
}

fn quad_uv(p: vec2<f32>) -> vec2<f32> {
    return p * 0.5 + vec2<f32>(0.5, 0.5);
}

// Robust billboard basis:
// Avoid degeneracy when `to_cam` is near-parallel to the chosen up axis.
fn billboard_basis(to_cam: vec3<f32>) -> array<vec3<f32>, 2> {
    // Prefer world-up, but switch to a safe axis if nearly parallel.
    var up_axis = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(to_cam, up_axis)) > 0.99) {
        up_axis = vec3<f32>(1.0, 0.0, 0.0);
    }

    let right = normalize(cross(up_axis, to_cam));
    let up = cross(to_cam, right);
    return array<vec3<f32>, 2>(right, up);
}

// -------------------- Particle picking --------------------

@vertex
fn vs_pick_particle(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VsOut {
    var out: VsOut;

    // ID: particle index + 1 (0 reserved for "no hit")
    out.id = instance_index + 1u;

    let p = particles[instance_index];
    let world_center = p.position.xyz;

    // Billboard quad in world space
    let local = quad_vertex(vertex_index);
    let uv = quad_uv(local);

    // Camera-facing basis (robust)
    let to_cam = normalize(camera.position - world_center);
    let basis = billboard_basis(to_cam);
    let right = basis[0];
    let up = basis[1];

    // Match visual particle size exactly:
    // Visual shader uses:
    //   let size = camera.particle_size * particle.data.y;
    //   world_pos = particle_pos + (right * pos_offset.x + billboard_up * pos_offset.y) * size;
    //
    // Here, `local` is the same as `pos_offset` in the visual shader (values in [-1, 1]),
    // so we must multiply by `size` (NOT a "radius") to match the rendered quad footprint.
    let size = camera.particle_size * p.data.y;
    let world_pos = world_center + (right * local.x + up * local.y) * size;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_pick_particle(in: VsOut) -> @location(0) vec4<f32> {
    // Simple circular mask so we pick the particle disc, not the full quad.
    let d = in.uv - vec2<f32>(0.5, 0.5);
    let r2 = dot(d, d);
    if (r2 > 0.25) {
        discard;
    }

    return pack_u32_to_rgba8(in.id);
}

// -------------------- Hadron picking --------------------
//
// We render hadron shells as billboards (like hadron renderer shells) and encode:
// 0x8000_0000 | (hadron_index + 1)

@vertex
fn vs_pick_hadron(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VsOut {
    var out: VsOut;

    let num_hadrons = hadron_counter.counters.x;
    if (instance_index >= num_hadrons) {
        // Push off-screen
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.id = 0u;
        out.uv = vec2<f32>(0.0, 0.0);
        return out;
    }

    let h = hadrons[instance_index];
    if (h.indices_type.w == 0xFFFFffffu) {
        // Invalid slot - don't pick it
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.id = 0u;
        out.uv = vec2<f32>(0.0, 0.0);
        return out;
    }

    out.id = 0x80000000u | (instance_index + 1u);

    let center = h.center.xyz;
    let radius = h.center.w;

    let local = quad_vertex(vertex_index);
    let uv = quad_uv(local);

    // Camera-facing basis (robust)
    let to_cam = normalize(camera.position - center);
    let basis = billboard_basis(to_cam);
    let right = basis[0];
    let up = basis[1];

    let world_pos = center + (right * local.x + up * local.y) * radius;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_pick_hadron(in: VsOut) -> @location(0) vec4<f32> {
    // Shell is a disc in screen-facing quad. For picking, we accept the full disc.
    let d = in.uv - vec2<f32>(0.5, 0.5);
    let r2 = dot(d, d);
    if (r2 > 0.25) {
        discard;
    }

    return pack_u32_to_rgba8(in.id);
}
