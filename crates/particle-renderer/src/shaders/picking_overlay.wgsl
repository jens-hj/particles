// Fullscreen shader to visualize the packed picking ID texture.
//
// Expected input texture format: RGBA8 (UNORM) where the shader that produced the picking layer
// wrote a packed u32 id as RGBA bytes:
//   id = r | (g<<8) | (b<<16) | (a<<24)
//
// This overlay decodes that ID and displays a stable, high-contrast pseudocolor,
// with background (id==0) shown transparent.
//
// Notes:
// - This is intended for debugging only.
// - The renderer should draw a fullscreen triangle/quad and blend this over the main scene.
// - For best results: use alpha blending and an `opacity` uniform.
//
// Bindings:
// @group(0) @binding(0): sampled 2D texture containing packed ids (rgba8unorm)
// @group(0) @binding(1): sampler (nearest recommended)
// @group(0) @binding(2): uniform params (opacity, etc.)

struct Params {
    // 0..1 overlay opacity
    opacity: f32,
    // If true-ish, draw a faint grid to help see pixel alignment (set to 1.0 to enable).
    grid: f32,
    // Padding/unused
    _pad0: vec2<f32>,
}

@group(0) @binding(0)
var id_tex: texture_2d<f32>;

@group(0) @binding(1)
var id_samp: sampler;

@group(0) @binding(2)
var<uniform> params: Params;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle (covers [0,1] UV)
    var out: VsOut;

    // Positions in clip space
    // ( -1, -1 ), ( 3, -1 ), ( -1, 3 )
    var p = vec2<f32>(0.0, 0.0);
    if (vi == 0u) {
        p = vec2<f32>(-1.0, -1.0);
    } else if (vi == 1u) {
        p = vec2<f32>(3.0, -1.0);
    } else {
        p = vec2<f32>(-1.0, 3.0);
    }

    out.pos = vec4<f32>(p, 0.0, 1.0);

    // Map clip space to UV:
    // For the chosen triangle: uv = (pos.xy + 1) * 0.5 but needs to work for the 3x triangle.
    out.uv = (p + vec2<f32>(1.0, 1.0)) * 0.5;

    return out;
}

fn hash_u32_to_rgb(id: u32) -> vec3<f32> {
    // Simple integer hash -> 0..1 RGB
    // Designed to be stable and visually distinct across many IDs.
    let r = f32((id * 1664525u + 1013904223u) & 255u) / 255.0;
    let g = f32((id * 22695477u + 1u) & 255u) / 255.0;
    let b = f32((id * 1103515245u + 12345u) & 255u) / 255.0;
    return vec3<f32>(r, g, b);
}

fn unpack_rgba8_to_u32(c: vec4<f32>) -> u32 {
    // id texture is RGBA8 UNORM sampled as float 0..1.
    // Convert to bytes with rounding to nearest (robust against quantization).
    let r = u32(round(clamp(c.r, 0.0, 1.0) * 255.0));
    let g = u32(round(clamp(c.g, 0.0, 1.0) * 255.0));
    let b = u32(round(clamp(c.b, 0.0, 1.0) * 255.0));
    let a = u32(round(clamp(c.a, 0.0, 1.0) * 255.0));

    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn grid_term(uv: vec2<f32>) -> f32 {
    // Faint pixel grid based on texture dimensions.
    // This uses derivatives so it behaves reasonably at different resolutions.
    let dims = vec2<f32>(textureDimensions(id_tex, 0));
    let px = uv * dims;

    // Distance to nearest pixel edge
    let fx = abs(fract(px.x) - 0.5);
    let fy = abs(fract(px.y) - 0.5);
    let d = min(fx, fy);

    // Sharpen line around edges
    let w = 0.02; // line width in pixel fraction
    return 1.0 - smoothstep(0.5 - w, 0.5, d);
}

@fragment
fn fs_overlay(in: VsOut) -> @location(0) vec4<f32> {
    // Sample packed ID
    let c = textureSample(id_tex, id_samp, in.uv);
    let id = unpack_rgba8_to_u32(c);

    if (id == 0u) {
        // Background: transparent
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Pseudocolor per id
    var rgb = hash_u32_to_rgb(id);

    // Optional grid overlay
    if (params.grid > 0.5) {
        let g = grid_term(in.uv);
        // Blend towards white on grid lines
        rgb = rgb * (1.0 - 0.35 * g) + vec3<f32>(1.0, 1.0, 1.0) * (0.35 * g);
    }

    return vec4<f32>(rgb, clamp(params.opacity, 0.0, 1.0));
}
