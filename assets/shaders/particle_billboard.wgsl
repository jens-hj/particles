// Render shader for billboard particles
// Draws circles on camera-facing quads

struct Camera {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
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

    // Catppuccin mauve color
    let color = vec3<f32>(0.8, 0.7, 0.9);

    return vec4<f32>(color, alpha);
}
