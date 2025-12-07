// Render shader for billboard particles
// Draws circles on camera-facing quads

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Particle {
    position: vec3<f32>,
    _padding1: f32,
    color: vec3<f32>,
    _padding2: f32,
}

@group(0) @binding(1)
var<storage, read> particles: array<Particle>;

struct ParticleSizeUniform {
    size: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(2)
var<uniform> particle_size: ParticleSizeUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let particle = particles[instance_index];

    // Generate quad vertices on the fly
    var uv = vec2<f32>(0.0, 0.0);
    var pos_offset = vec2<f32>(0.0, 0.0);

    switch (vertex_index) {
        case 0u, 3u: {
            uv = vec2<f32>(0.0, 0.0);
            pos_offset = vec2<f32>(-1.0, -1.0);
        }
        case 1u: {
            uv = vec2<f32>(1.0, 0.0);
            pos_offset = vec2<f32>(1.0, -1.0);
        }
        case 2u, 4u: {
            uv = vec2<f32>(1.0, 1.0);
            pos_offset = vec2<f32>(1.0, 1.0);
        }
        case 5u: {
            uv = vec2<f32>(0.0, 1.0);
            pos_offset = vec2<f32>(-1.0, 1.0);
        }
        default: {}
    }

    // Billboard calculation
    let to_camera = normalize(camera.position - particle.position);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, to_camera));
    let billboard_up = cross(to_camera, right);

    let size = particle_size.size;
    let world_pos = particle.position + (right * pos_offset.x + billboard_up * pos_offset.y) * size;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    out.color = particle.color;
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

    return vec4<f32>(input.color, 1.0);
}
