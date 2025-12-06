// Vertex shader for instanced particle rendering
struct VertexInput {
    @location(0) position: vec3<f32>,  // Base sphere vertex position
    @location(1) particle_pos: vec3<f32>,  // Instance data: particle position
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Scale the base sphere and offset by particle position
    let world_pos = input.position + input.particle_pos;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_position = world_pos;

    return out;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple white color for now
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
