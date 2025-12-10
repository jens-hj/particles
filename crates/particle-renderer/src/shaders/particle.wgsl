// Particle rendering shader with color mapping for physics visualization

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    particle_size: f32,
    time: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type (as f32)
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z/w = padding
}

@group(0) @binding(1)
var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) particle_type: u32,
}

// Catppuccin Mocha colors (in linear RGB, converted from sRGB)
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

// Color mapping for quarks (by color charge) using Catppuccin colors
fn quark_color(color_charge: u32) -> vec3<f32> {
    switch (color_charge) {
        case 0u: { return srgb_to_linear(vec3<f32>(0.953, 0.545, 0.659)); }  // Red #f38ba8
        case 1u: { return srgb_to_linear(vec3<f32>(0.647, 0.859, 0.627)); }  // Green (green)
        case 2u: { return srgb_to_linear(vec3<f32>(0.549, 0.753, 0.984)); }  // Blue (blue)
        case 3u: { return srgb_to_linear(vec3<f32>(0.961, 0.718, 0.741)); }  // AntiRed (flamingo)
        case 4u: { return srgb_to_linear(vec3<f32>(0.580, 0.886, 0.820)); }  // AntiGreen (teal)
        case 5u: { return srgb_to_linear(vec3<f32>(0.553, 0.827, 0.937)); }  // AntiBlue (sapphire)
        default: { return srgb_to_linear(vec3<f32>(0.803, 0.816, 0.839)); }  // White (text)
    }
}

// Color mapping for composite particles (by particle type)
fn particle_color(particle_type: u32, color_charge: u32) -> vec3<f32> {
    switch (particle_type) {
        case 0u, 1u: {                                      // QuarkUp, QuarkDown
            return quark_color(color_charge);
        }
        case 2u: {                                          // Electron
            return srgb_to_linear(vec3<f32>(0.976, 0.886, 0.686)); // Yellow #f9e2af
        }
        case 3u: {                                          // Gluon
            return srgb_to_linear(vec3<f32>(0.980, 0.702, 0.529)); // Peach #fab387
        }
        case 4u: {                                          // Proton
            return srgb_to_linear(vec3<f32>(0.647, 0.859, 0.627)); // Green (green)
        }
        case 5u: {                                          // Neutron
            return srgb_to_linear(vec3<f32>(0.549, 0.753, 0.984)); // Blue (blue)
        }
        default: {
            return srgb_to_linear(vec3<f32>(0.803, 0.816, 0.839)); // White (text)
        }
    }
}

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let particle = particles[instance_index];

    // Generate quad vertices
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
    let particle_pos = particle.position.xyz;  // Extract position from vec4
    let to_camera = normalize(camera.position - particle_pos);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, to_camera));
    let billboard_up = cross(to_camera, right);

    let size = camera.particle_size * particle.data.y; // size in data.y
    let world_pos = particle_pos + (right * pos_offset.x + billboard_up * pos_offset.y) * size;

    // Extract particle type and color charge
    let particle_type = u32(particle.position.w);
    let color_charge = particle.color_and_flags.x;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    out.color = particle_color(particle_type, color_charge);
    out.particle_type = particle_type;
    return out;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Draw sphere
    let center = vec2<f32>(0.5, 0.5);
    let uv_centered = input.uv - center;
    let dist = length(uv_centered);

    if (dist > 0.5) {
        discard;
    }

    // Hollow center for Down quarks (type 1) to distinguish from Up quarks
    if (input.particle_type == 1u && dist < 0.2) {
        discard;
    }

    // Calculate fake sphere normal and lighting
    // z component based on distance from center (sphere equation: x² + y² + z² = r²)
    let z = sqrt(max(0.0, 0.25 - dist * dist)) * 2.0;
    let normal = normalize(vec3<f32>(uv_centered.x, uv_centered.y, z));

    // Simple directional lighting (light from top-right-front)
    let light_dir = normalize(vec3<f32>(0.5, 0.5, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    let ambient = 0.4;
    let lighting = ambient + diffuse * 0.6;

    let final_color = input.color * lighting;

    return vec4<f32>(final_color, 1.0);
}
