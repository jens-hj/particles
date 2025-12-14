// Shader for rendering nucleus shells

const MAX_NUCLEONS: u32 = 16u;

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    particle_size: f32,
    time: f32,
    lod_shell_fade_start: f32,
    lod_shell_fade_end: f32,
    lod_bound_hadron_fade_start: f32,
    lod_bound_hadron_fade_end: f32,
    lod_bond_fade_start: f32,
    lod_bond_fade_end: f32,
    lod_quark_fade_start: f32,
    lod_quark_fade_end: f32,
    lod_nucleus_fade_start: f32,
    lod_nucleus_fade_end: f32,

    // Uniforms are laid out in 16-byte chunks; use 16-byte padding to avoid rounding up to 144 bytes.
    _pad: vec4<f32>,
}

struct Nucleus {
    hadron_indices: array<u32, MAX_NUCLEONS>,
    nucleon_count: u32,
    proton_count: u32,
    neutron_count: u32,
    type_id: u32,       // Atomic number (Z)
    center: vec4<f32>,  // xyz = center, w = radius
    velocity: vec4<f32>,
}

struct NucleusCounter {
    count: u32,
    _pad: vec3<u32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<storage, read> nuclei: array<Nucleus>;

@group(0) @binding(2)
var<storage, read> counter: NucleusCounter;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) dist_to_cam: f32,
}

// Color nucleus shells based on atomic number (Z)
// Using Catppuccin Mocha colors with alpha for translucent shells
fn get_nucleus_color(atomic_number: u32) -> vec4<f32> {
    switch (atomic_number) {
        case 1u: { return vec4<f32>(0.976, 0.890, 0.686, 0.25); }  // Hydrogen (H) - Yellow
        case 2u: { return vec4<f32>(0.980, 0.702, 0.529, 0.25); }  // Helium (He) - Peach
        case 3u: { return vec4<f32>(0.961, 0.718, 0.741, 0.25); }  // Lithium (Li) - Flamingo
        case 4u: { return vec4<f32>(0.961, 0.718, 0.741, 0.25); }  // Beryllium (Be) - Flamingo
        case 5u: { return vec4<f32>(0.953, 0.545, 0.659, 0.25); }  // Boron (B) - Red
        case 6u: { return vec4<f32>(0.647, 0.859, 0.627, 0.25); }  // Carbon (C) - Green
        case 7u: { return vec4<f32>(0.549, 0.753, 0.984, 0.25); }  // Nitrogen (N) - Blue
        case 8u: { return vec4<f32>(0.953, 0.545, 0.659, 0.25); }  // Oxygen (O) - Red
        default: {
            // For higher Z, use a gradient from green to purple
            let t = f32(atomic_number) / 20.0;
            let r = 0.6 + t * 0.3;
            let g = 0.8 - t * 0.3;
            let b = 0.9;
            return vec4<f32>(r, g, b, 0.25);
        }
    }
}

// --- SHELL RENDERER (Instanced Quads) ---

@vertex
fn vs_shell(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var out: VertexOutput;

    // Discard if out of range
    if (instance_index >= counter.count) {
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    }

    let nucleus = nuclei[instance_index];

    // Skip invalid nuclei
    if (nucleus.type_id == 0xFFFFFFFFu) {
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    }

    // Generate quad vertices
    var uv = vec2<f32>(0.0, 0.0);
    var pos_offset = vec2<f32>(0.0, 0.0);

    switch (vertex_index) {
        case 0u, 3u: { uv = vec2<f32>(0.0, 0.0); pos_offset = vec2<f32>(-1.0, -1.0); }
        case 1u: { uv = vec2<f32>(1.0, 0.0); pos_offset = vec2<f32>(1.0, -1.0); }
        case 2u, 4u: { uv = vec2<f32>(1.0, 1.0); pos_offset = vec2<f32>(1.0, 1.0); }
        case 5u: { uv = vec2<f32>(0.0, 1.0); pos_offset = vec2<f32>(-1.0, 1.0); }
        default: {}
    }

    // Billboard calculation
    let center = nucleus.center.xyz;
    let radius = nucleus.center.w;
    let to_camera = normalize(camera.position - center);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, to_camera));
    let billboard_up = cross(to_camera, right);

    let world_pos = center + (right * pos_offset.x + billboard_up * pos_offset.y) * radius;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    out.color = get_nucleus_color(nucleus.type_id); // type_id = atomic number (Z)
    out.dist_to_cam = distance(camera.position, center);

    return out;
}

@fragment
fn fs_shell(in: VertexOutput) -> @location(0) vec4<f32> {
    // Draw sphere
    let center = vec2<f32>(0.5, 0.5);
    let uv_centered = in.uv - center;
    let dist = length(uv_centered);

    if (dist > 0.5) {
        discard;
    }

    // Fake 3D normal
    let z = sqrt(max(0.0, 0.25 - dist * dist)) * 2.0;
    let normal = normalize(vec3<f32>(uv_centered.x, uv_centered.y, z));
    let light_dir = normalize(vec3<f32>(0.5, 0.5, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);

    // LOD: Fade in nucleus shells based on distance (using nucleus-specific LOD values)
    let final_alpha = smoothstep(camera.lod_nucleus_fade_start, camera.lod_nucleus_fade_end, in.dist_to_cam);

    if (final_alpha < 0.01) {
        discard;
    }

    let lighting = 0.5 + diffuse * 0.5;
    // At `lod_nucleus_fade_end`, the nucleus is fully opaque (alpha = 1).
    return vec4<f32>(in.color.rgb * lighting, final_alpha);
}
