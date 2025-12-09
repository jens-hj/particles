// Shader for rendering hadrons (bonds and shells)

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    particle_size: f32,
}

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    data: vec4<f32>,
    color_and_flags: vec4<u32>,
}

struct Hadron {
    indices_type: vec4<u32>, // x=p1, y=p2, z=p3, w=type_id
    center: vec4<f32>, // xyz, w=radius
}

struct HadronCounter {
    count: u32,
    _pad: vec3<u32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<storage, read> hadrons: array<Hadron>;

@group(0) @binding(2)
var<storage, read> particles: array<Particle>;

@group(0) @binding(3)
var<storage, read> counter: HadronCounter;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) dist_to_cam: f32,
}

// --- COLORS ---
fn get_hadron_color(type_id: u32) -> vec4<f32> {
    switch (type_id) {
        case 0u: { return vec4<f32>(0.976, 0.890, 0.494, 0.3); } // Meson (Yellow)
        case 1u: { return vec4<f32>(0.647, 0.859, 0.627, 0.3); } // Proton (Green)
        case 2u: { return vec4<f32>(0.549, 0.753, 0.984, 0.3); } // Neutron (Blue)
        default: { return vec4<f32>(0.8, 0.8, 0.8, 0.3); }
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

    let hadron = hadrons[instance_index];

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
    let center = hadron.center.xyz;
    let radius = hadron.center.w;
    let to_camera = normalize(camera.position - center);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, to_camera));
    let billboard_up = cross(to_camera, right);

    let world_pos = center + (right * pos_offset.x + billboard_up * pos_offset.y) * radius;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    out.color = get_hadron_color(hadron.indices_type.w);
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

    // LOD: Fade in shell when far away
    // < 5.0: Invisible (show lines)
    // > 10.0: Fully opaque
    let alpha_factor = smoothstep(5.0, 10.0, in.dist_to_cam);

    let final_alpha = in.color.a * alpha_factor;
    if (final_alpha < 0.01) {
        discard;
    }

    let lighting = 0.5 + diffuse * 0.5;
    return vec4<f32>(in.color.rgb * lighting, final_alpha);
}

// --- BOND RENDERER (Lines) ---

@vertex
fn vs_bond(
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {
    var out: VertexOutput;

    let hadron_idx = vertex_index / 6u;
    let line_idx = vertex_index % 6u; // 0-1, 2-3, 4-5

    if (hadron_idx >= counter.count) {
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    }

    let hadron = hadrons[hadron_idx];
    var pos = vec3<f32>(0.0, 0.0, 0.0);

    // Determine which particle position to use
    // Line 1: p1 -> p2
    // Line 2: p2 -> p3
    // Line 3: p3 -> p1

    // Helper to get position safely
    let p1 = particles[hadron.indices_type.x].position.xyz;
    let p2 = particles[hadron.indices_type.y].position.xyz;
    // For p3, check if it exists (Mesons have p3 = 0xFFFFFFFF)
    var p3 = p1;
    if (hadron.indices_type.z != 0xFFFFFFFFu) {
        p3 = particles[hadron.indices_type.z].position.xyz;
    }

    switch (line_idx) {
        case 0u: { pos = p1; }
        case 1u: { pos = p2; }
        case 2u: { pos = p2; }
        case 3u: { pos = p3; }
        case 4u: { pos = p3; }
        case 5u: { pos = p1; }
        default: {}
    }

    // If Meson, collapse lines 2 and 3 (p2->p3, p3->p1) to degenerate lines
    if (hadron.indices_type.z == 0xFFFFFFFFu && line_idx >= 2u) {
        pos = p1; // Collapse to point
    }

    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.color = vec4<f32>(1.0, 1.0, 1.0, 0.5); // White lines
    out.uv = vec2<f32>(0.0, 0.0); // Unused
    out.dist_to_cam = distance(camera.position, hadron.center.xyz);

    return out;
}

@fragment
fn fs_bond(in: VertexOutput) -> @location(0) vec4<f32> {
    // LOD: Fade out lines when far away
    // < 5.0: Fully visible
    // > 10.0: Invisible
    let alpha_factor = 1.0 - smoothstep(5.0, 10.0, in.dist_to_cam);

    if (alpha_factor < 0.01) {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha_factor);
}
