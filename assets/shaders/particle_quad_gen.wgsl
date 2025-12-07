// Compute shader to generate camera-facing billboard quads from particle positions

struct Particle {
    position: vec3<f32>,
    _padding1: f32,
    color: vec3<f32>,
    _padding2: f32,
}

struct QuadVertex {
    position: vec3<f32>,
    _padding1: f32,
    color: vec3<f32>,
    _padding2: f32,
    uv: vec2<f32>,
    _padding3: vec2<f32>,
}

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> vertices: array<QuadVertex>;

@group(0) @binding(2)
var<uniform> camera: Camera;

struct ParticleSizeUniform {
    size: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(3)
var<uniform> particle_size_uniform: ParticleSizeUniform;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let particle_index = global_id.x;

    // Bounds check
    if (particle_index >= arrayLength(&particles)) {
        return;
    }

    let particle = particles[particle_index];
    let particle_pos = particle.position;
    let particle_color = particle.color;

    // Calculate camera-facing orientation
    let to_camera = normalize(camera.position - particle_pos);

    // Create orthonormal basis for billboard
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, to_camera));
    let billboard_up = cross(to_camera, right);

    // Quad size
    let size = particle_size_uniform.size;

    // Generate 4 vertices for this particle's quad
    let base_index = particle_index * 6u;  // 6 vertices per particle (2 triangles)

    // Triangle 1: bottom-left, bottom-right, top-right
    vertices[base_index + 0u] = QuadVertex(
        particle_pos - right * size - billboard_up * size,  // Bottom-left
        0.0,  // padding1
        particle_color,
        0.0,  // padding2
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0)  // padding3
    );
    vertices[base_index + 1u] = QuadVertex(
        particle_pos + right * size - billboard_up * size,  // Bottom-right
        0.0,  // padding1
        particle_color,
        0.0,  // padding2
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0)  // padding3
    );
    vertices[base_index + 2u] = QuadVertex(
        particle_pos + right * size + billboard_up * size,  // Top-right
        0.0,  // padding1
        particle_color,
        0.0,  // padding2
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0)  // padding3
    );

    // Triangle 2: bottom-left, top-right, top-left
    vertices[base_index + 3u] = QuadVertex(
        particle_pos - right * size - billboard_up * size,  // Bottom-left
        0.0,  // padding1
        particle_color,
        0.0,  // padding2
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0)  // padding3
    );
    vertices[base_index + 4u] = QuadVertex(
        particle_pos + right * size + billboard_up * size,  // Top-right
        0.0,  // padding1
        particle_color,
        0.0,  // padding2
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0)  // padding3
    );
    vertices[base_index + 5u] = QuadVertex(
        particle_pos - right * size + billboard_up * size,  // Top-left
        0.0,  // padding1
        particle_color,
        0.0,  // padding2
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 0.0)  // padding3
    );
}
