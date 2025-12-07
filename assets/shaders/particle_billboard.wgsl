// Render shader for billboard particles
// Draws circles on camera-facing quads

struct Camera {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct QuadVertex {
    position: vec3<f32>,
    _padding1: f32,
    color: vec3<f32>,
    _padding2: f32,
    uv: vec2<f32>,
    _padding3: vec2<f32>,
}

@group(0) @binding(1)
var<storage, read> vertices: array<QuadVertex>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertex_data = vertices[vertex_index];

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(vertex_data.position, 1.0);
    out.uv = vertex_data.uv;
    out.color = vertex_data.color;
    return out;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Draw circle: discard pixels outside radius 0.5
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(input.uv, center);

    if (dist > 0.5) {
        discard;
    }

    // Soft edge for anti-aliasing
    let edge_softness = 0.02;
    let alpha = smoothstep(0.5, 0.5 - edge_softness, dist);

    return vec4<f32>(input.color, alpha);
}
