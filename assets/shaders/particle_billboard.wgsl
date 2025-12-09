// Render shader for billboard particles
// Draws circles on camera-facing quads

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    particle_size: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Particle {
    position: vec3<f32>,
    size: f32,
    color: vec3<f32>,
    alpha: f32,
}

@group(0) @binding(1)
var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) alpha: f32,
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

    // Combine global size with per-particle size multiplier
    let size = camera.particle_size * particle.size;
    let world_pos = particle.position + (right * pos_offset.x + billboard_up * pos_offset.y) * size;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // // Add tiny depth offset based on instance index to prevent z-fighting
    // // for particles at exactly the same position
    // let depth_offset = f32(instance_index) * 0.00000001;
    // out.clip_position.z += depth_offset;

    out.uv = uv;
    out.color = particle.color;
    out.alpha = particle.alpha;
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

    return vec4<f32>(input.color, input.alpha);
}
