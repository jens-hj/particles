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
// Using CPK/Jmol coloring scheme (PubChem) with alpha for translucent shells
fn get_nucleus_color(atomic_number: u32) -> vec4<f32> {
    let alpha = 0.25;
    switch (atomic_number) {
        case 1u: { return vec4<f32>(1.000, 1.000, 1.000, alpha); }  // H - White
        case 2u: { return vec4<f32>(0.851, 1.000, 1.000, alpha); }  // He - Cyan
        case 3u: { return vec4<f32>(0.800, 0.502, 1.000, alpha); }  // Li - Purple
        case 4u: { return vec4<f32>(0.761, 1.000, 0.000, alpha); }  // Be - Green
        case 5u: { return vec4<f32>(1.000, 0.710, 0.710, alpha); }  // B - Peach
        case 6u: { return vec4<f32>(0.565, 0.565, 0.565, alpha); }  // C - Gray
        case 7u: { return vec4<f32>(0.188, 0.314, 0.973, alpha); }  // N - Blue
        case 8u: { return vec4<f32>(1.000, 0.051, 0.051, alpha); }  // O - Red
        case 9u: { return vec4<f32>(0.565, 0.878, 0.314, alpha); }  // F - Green
        case 10u: { return vec4<f32>(0.702, 0.890, 0.961, alpha); } // Ne - Cyan
        case 11u: { return vec4<f32>(0.671, 0.361, 0.949, alpha); } // Na - Purple
        case 12u: { return vec4<f32>(0.541, 1.000, 0.000, alpha); } // Mg - Green
        case 13u: { return vec4<f32>(0.749, 0.651, 0.651, alpha); } // Al - Pink gray
        case 14u: { return vec4<f32>(0.941, 0.784, 0.627, alpha); } // Si - Goldenrod
        case 15u: { return vec4<f32>(1.000, 0.502, 0.000, alpha); } // P - Orange
        case 16u: { return vec4<f32>(1.000, 1.000, 0.188, alpha); } // S - Yellow
        case 17u: { return vec4<f32>(0.122, 0.941, 0.122, alpha); } // Cl - Green
        case 18u: { return vec4<f32>(0.502, 0.820, 0.890, alpha); } // Ar - Cyan
        case 19u: { return vec4<f32>(0.561, 0.251, 0.831, alpha); } // K - Purple
        case 20u: { return vec4<f32>(0.239, 1.000, 0.000, alpha); } // Ca - Green
        case 21u: { return vec4<f32>(0.902, 0.902, 0.902, alpha); } // Sc - Gray
        case 22u: { return vec4<f32>(0.749, 0.761, 0.780, alpha); } // Ti - Gray
        case 23u: { return vec4<f32>(0.651, 0.651, 0.671, alpha); } // V - Gray
        case 24u: { return vec4<f32>(0.541, 0.600, 0.780, alpha); } // Cr - Steel blue
        case 25u: { return vec4<f32>(0.612, 0.478, 0.780, alpha); } // Mn - Purple
        case 26u: { return vec4<f32>(0.878, 0.400, 0.200, alpha); } // Fe - Orange
        case 27u: { return vec4<f32>(0.941, 0.565, 0.627, alpha); } // Co - Pink
        case 28u: { return vec4<f32>(0.314, 0.816, 0.314, alpha); } // Ni - Green
        case 29u: { return vec4<f32>(0.784, 0.502, 0.200, alpha); } // Cu - Brown
        case 30u: { return vec4<f32>(0.490, 0.502, 0.690, alpha); } // Zn - Blue gray
        case 31u: { return vec4<f32>(0.761, 0.561, 0.561, alpha); } // Ga - Pink
        case 32u: { return vec4<f32>(0.400, 0.561, 0.561, alpha); } // Ge - Gray green
        case 33u: { return vec4<f32>(0.741, 0.502, 0.890, alpha); } // As - Purple
        case 34u: { return vec4<f32>(1.000, 0.631, 0.000, alpha); } // Se - Orange
        case 35u: { return vec4<f32>(0.651, 0.161, 0.161, alpha); } // Br - Brown
        case 36u: { return vec4<f32>(0.361, 0.722, 0.820, alpha); } // Kr - Cyan
        case 37u: { return vec4<f32>(0.439, 0.180, 0.690, alpha); } // Rb - Purple
        case 38u: { return vec4<f32>(0.000, 1.000, 0.000, alpha); } // Sr - Green
        case 39u: { return vec4<f32>(0.580, 1.000, 1.000, alpha); } // Y - Cyan
        case 40u: { return vec4<f32>(0.580, 0.878, 0.878, alpha); } // Zr - Cyan
        case 41u: { return vec4<f32>(0.451, 0.761, 0.788, alpha); } // Nb - Cyan
        case 42u: { return vec4<f32>(0.329, 0.710, 0.710, alpha); } // Mo - Cyan
        case 43u: { return vec4<f32>(0.231, 0.620, 0.620, alpha); } // Tc - Teal
        case 44u: { return vec4<f32>(0.141, 0.561, 0.561, alpha); } // Ru - Teal
        case 45u: { return vec4<f32>(0.039, 0.490, 0.549, alpha); } // Rh - Teal
        case 46u: { return vec4<f32>(0.000, 0.412, 0.522, alpha); } // Pd - Teal
        case 47u: { return vec4<f32>(0.753, 0.753, 0.753, alpha); } // Ag - Silver
        case 48u: { return vec4<f32>(1.000, 0.851, 0.561, alpha); } // Cd - Gold
        case 49u: { return vec4<f32>(0.651, 0.459, 0.451, alpha); } // In - Brown
        case 50u: { return vec4<f32>(0.400, 0.502, 0.502, alpha); } // Sn - Gray
        case 51u: { return vec4<f32>(0.620, 0.388, 0.710, alpha); } // Sb - Purple
        case 52u: { return vec4<f32>(0.831, 0.478, 0.000, alpha); } // Te - Orange
        case 53u: { return vec4<f32>(0.580, 0.000, 0.580, alpha); } // I - Purple
        case 54u: { return vec4<f32>(0.259, 0.620, 0.690, alpha); } // Xe - Cyan
        case 55u: { return vec4<f32>(0.341, 0.090, 0.561, alpha); } // Cs - Purple
        case 56u: { return vec4<f32>(0.000, 0.788, 0.000, alpha); } // Ba - Green
        case 57u: { return vec4<f32>(0.439, 0.831, 1.000, alpha); } // La - Blue
        case 58u: { return vec4<f32>(1.000, 1.000, 0.780, alpha); } // Ce - Yellow
        case 59u: { return vec4<f32>(0.851, 1.000, 0.780, alpha); } // Pr - Yellow green
        case 60u: { return vec4<f32>(0.780, 1.000, 0.780, alpha); } // Nd - Green
        case 61u: { return vec4<f32>(0.639, 1.000, 0.780, alpha); } // Pm - Green
        case 62u: { return vec4<f32>(0.561, 1.000, 0.780, alpha); } // Sm - Green
        case 63u: { return vec4<f32>(0.380, 1.000, 0.780, alpha); } // Eu - Green
        case 64u: { return vec4<f32>(0.271, 1.000, 0.780, alpha); } // Gd - Green
        case 65u: { return vec4<f32>(0.188, 1.000, 0.780, alpha); } // Tb - Green
        case 66u: { return vec4<f32>(0.122, 1.000, 0.780, alpha); } // Dy - Green
        case 67u: { return vec4<f32>(0.000, 1.000, 0.612, alpha); } // Ho - Green
        case 68u: { return vec4<f32>(0.000, 0.902, 0.459, alpha); } // Er - Green
        case 69u: { return vec4<f32>(0.000, 0.831, 0.322, alpha); } // Tm - Green
        case 70u: { return vec4<f32>(0.000, 0.749, 0.220, alpha); } // Yb - Green
        case 71u: { return vec4<f32>(0.000, 0.671, 0.141, alpha); } // Lu - Green
        case 72u: { return vec4<f32>(0.302, 0.761, 1.000, alpha); } // Hf - Blue
        case 73u: { return vec4<f32>(0.302, 0.651, 1.000, alpha); } // Ta - Blue
        case 74u: { return vec4<f32>(0.129, 0.580, 0.839, alpha); } // W - Blue
        case 75u: { return vec4<f32>(0.149, 0.490, 0.671, alpha); } // Re - Blue
        case 76u: { return vec4<f32>(0.149, 0.400, 0.588, alpha); } // Os - Blue
        case 77u: { return vec4<f32>(0.090, 0.329, 0.529, alpha); } // Ir - Blue
        case 78u: { return vec4<f32>(0.816, 0.816, 0.878, alpha); } // Pt - Silver
        case 79u: { return vec4<f32>(1.000, 0.820, 0.137, alpha); } // Au - Gold
        case 80u: { return vec4<f32>(0.722, 0.722, 0.816, alpha); } // Hg - Blue gray
        case 81u: { return vec4<f32>(0.651, 0.329, 0.302, alpha); } // Tl - Brown
        case 82u: { return vec4<f32>(0.341, 0.349, 0.380, alpha); } // Pb - Gray
        case 83u: { return vec4<f32>(0.620, 0.310, 0.710, alpha); } // Bi - Purple
        case 84u: { return vec4<f32>(0.671, 0.361, 0.000, alpha); } // Po - Brown
        case 85u: { return vec4<f32>(0.459, 0.310, 0.271, alpha); } // At - Brown
        case 86u: { return vec4<f32>(0.259, 0.510, 0.588, alpha); } // Rn - Teal
        case 87u: { return vec4<f32>(0.259, 0.000, 0.400, alpha); } // Fr - Purple
        case 88u: { return vec4<f32>(0.000, 0.490, 0.000, alpha); } // Ra - Green
        case 89u: { return vec4<f32>(0.439, 0.671, 0.980, alpha); } // Ac - Blue
        case 90u: { return vec4<f32>(0.000, 0.729, 1.000, alpha); } // Th - Cyan
        case 91u: { return vec4<f32>(0.000, 0.631, 1.000, alpha); } // Pa - Blue
        case 92u: { return vec4<f32>(0.000, 0.561, 1.000, alpha); } // U - Blue
        case 93u: { return vec4<f32>(0.000, 0.502, 1.000, alpha); } // Np - Blue
        case 94u: { return vec4<f32>(0.000, 0.420, 1.000, alpha); } // Pu - Blue
        case 95u: { return vec4<f32>(0.329, 0.361, 0.949, alpha); } // Am - Purple
        case 96u: { return vec4<f32>(0.471, 0.361, 0.890, alpha); } // Cm - Purple
        case 97u: { return vec4<f32>(0.541, 0.310, 0.890, alpha); } // Bk - Purple
        case 98u: { return vec4<f32>(0.631, 0.212, 0.831, alpha); } // Cf - Purple
        case 99u: { return vec4<f32>(0.702, 0.122, 0.831, alpha); } // Es - Purple
        case 100u: { return vec4<f32>(0.702, 0.122, 0.729, alpha); } // Fm - Purple
        case 101u: { return vec4<f32>(0.702, 0.051, 0.651, alpha); } // Md - Purple
        case 102u: { return vec4<f32>(0.741, 0.051, 0.529, alpha); } // No - Purple
        case 103u: { return vec4<f32>(0.780, 0.000, 0.400, alpha); } // Lr - Magenta
        case 104u: { return vec4<f32>(0.800, 0.000, 0.349, alpha); } // Rf - Magenta
        case 105u: { return vec4<f32>(0.820, 0.000, 0.310, alpha); } // Db - Magenta
        case 106u: { return vec4<f32>(0.851, 0.000, 0.271, alpha); } // Sg - Magenta
        case 107u: { return vec4<f32>(0.878, 0.000, 0.220, alpha); } // Bh - Magenta
        case 108u: { return vec4<f32>(0.902, 0.000, 0.180, alpha); } // Hs - Magenta
        case 109u: { return vec4<f32>(0.922, 0.000, 0.149, alpha); } // Mt - Magenta
        // Elements 110-118: Catppuccin-compatible gradient continuation
        case 110u: { return vec4<f32>(0.945, 0.000, 0.125, alpha); } // Ds - Red/magenta
        case 111u: { return vec4<f32>(0.965, 0.000, 0.102, alpha); } // Rg - Red
        case 112u: { return vec4<f32>(0.980, 0.000, 0.082, alpha); } // Cn - Red
        case 113u: { return vec4<f32>(0.996, 0.106, 0.106, alpha); } // Nh - Red
        case 114u: { return vec4<f32>(0.996, 0.184, 0.184, alpha); } // Fl - Light red
        case 115u: { return vec4<f32>(0.996, 0.243, 0.259, alpha); } // Mc - Pink red
        case 116u: { return vec4<f32>(0.996, 0.302, 0.333, alpha); } // Lv - Pink
        case 117u: { return vec4<f32>(0.996, 0.361, 0.408, alpha); } // Ts - Pink
        case 118u: { return vec4<f32>(0.996, 0.420, 0.482, alpha); } // Og - Light pink
        default: { return vec4<f32>(0.8, 0.8, 0.8, alpha); }       // Fallback gray
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
