// ============================================================================
// UI SDF Shader - Analytic Anti-Aliasing for GUI Components
// ============================================================================
//
// This shader uses signed distance fields (SDFs) to render GUI rectangles
// with pixel-perfect anti-aliasing at any resolution. Instead of tessellating
// shapes into many vertices, we render a simple quad and compute the distance
// to the shape boundary in the fragment shader.

struct Uniforms {
    screen_size: vec2<f32>,
}

struct VertexInput {
    @location(0) pos: vec2<f32>,  // Unit quad vertices: [-1, 1]
}

struct InstanceInput {
    @location(1) center: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) fill_color: vec4<f32>,
    @location(4) stroke_color: vec4<f32>,
    @location(5) stroke_width: f32,
    @location(6) corner_type: u32,
    @location(7) corner_param1: f32,
    @location(8) corner_param2: f32,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
    @location(1) local_pos: vec2<f32>,      // Position relative to rect center
    @location(2) fill_color: vec4<f32>,
    @location(3) stroke_color: vec4<f32>,
    @location(4) stroke_width: f32,
    @location(5) @interpolate(flat) corner_type: u32,
    @location(6) corner_param1: f32,
    @location(7) corner_param2: f32,
    @location(8) half_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// ============================================================================
// Vertex Shader
// ============================================================================

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // Expand unit quad to screen-space rectangle
    // Add padding for stroke (only half stroke width since it's centered on the edge)
    let padding = inst.stroke_width * 0.5;
    let expanded_size = inst.half_size + vec2<f32>(padding);
    out.world_pos = inst.center + vert.pos * expanded_size;

    // Convert to normalized device coordinates (NDC)
    let ndc = (out.world_pos / uniforms.screen_size) * 2.0 - 1.0;
    out.clip_pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);

    // Pass through instance data to fragment shader
    out.local_pos = vert.pos * inst.half_size;  // Position relative to center
    out.fill_color = inst.fill_color;
    out.stroke_color = inst.stroke_color;
    out.stroke_width = inst.stroke_width;
    out.corner_type = inst.corner_type;
    out.corner_param1 = inst.corner_param1;
    out.corner_param2 = inst.corner_param2;
    out.half_size = inst.half_size;

    return out;
}

// ============================================================================
// SDF Functions
// ============================================================================

/// Signed distance to a box (sharp corners)
/// Returns negative inside, positive outside, zero on the boundary
fn sd_box(p: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

/// Signed distance to a rounded box (circular arc corners)
/// Based on Inigo Quilez's formula: https://iquilezles.org/articles/distfunctions2d/
fn sd_rounded_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

/// Signed distance to a chamfered box (diagonal cut corners at 45°)
fn sd_chamfer_box(p: vec2<f32>, size: vec2<f32>, chamfer: f32) -> f32 {
    var p_local = abs(p) - size;

    // Swap x/y to always work with the diagonal case
    if p_local.y > p_local.x {
        p_local = vec2<f32>(p_local.y, p_local.x);
    }

    p_local.y = p_local.y + chamfer;
    let k = 1.0 - sqrt(2.0) * 0.5;

    if p_local.y < 0.0 && p_local.y + p_local.x * k < 0.0 {
        return p_local.x;
    }
    if p_local.x < p_local.y {
        return (p_local.x + p_local.y) * sqrt(0.5);
    }
    return length(p_local);
}

/// Signed distance to an inverse rounded box (concave circular corners)
/// This creates a shape like a ticket with punched corners
fn sd_inverse_round_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    // Inverse rounded corners: circles centered at the corners carve into the rectangle
    // Creating concave (inward-curving) corners like a ticket punch
    let p_abs = abs(p);

    // Distance to corner point
    let to_corner = p_abs - size;

    // Box boundary distance (compute once to avoid discontinuities)
    let box_dist = max(to_corner.x, to_corner.y);

    // In corner region (close enough to corner to be affected by circle)
    if to_corner.x > -radius && to_corner.y > -radius {
        // Circle centered at corner (size.x, size.y)
        let circle_dist = length(p_abs - size) - radius;

        // The shape is the box MINUS circles at corners
        // max(box, -circle) carves the circle out of the box
        return max(box_dist, -circle_dist);
    }

    // Outside corner influence, just use box
    return box_dist;
}

/// Signed distance to a squircle box (superellipse corners)
/// Uses power distance approximation: |x|^n + |y|^n = r^n where n = 2 + smoothness
/// Note: This is an approximation as exact squircle SDF has no closed-form solution
fn sd_squircle_box(p: vec2<f32>, size: vec2<f32>, radius: f32, smoothness: f32) -> f32 {
    let n = 2.0 + smoothness;
    let corner_offset = size - vec2<f32>(radius);
    let p_corner = abs(p) - corner_offset;

    // If we're not in a corner region, use simple box distance
    if p_corner.x <= 0.0 || p_corner.y <= 0.0 {
        return sd_box(p, size);
    }

    // In corner region, compute superellipse approximation
    let p_abs = abs(p_corner);
    let power_sum = pow(p_abs.x, n) + pow(p_abs.y, n);
    let power_dist = pow(power_sum, 1.0 / n);

    return power_dist - radius;
}

// ============================================================================
// Fragment Shader
// ============================================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Compute signed distance based on corner type
    var dist: f32;

    switch in.corner_type {
        case 0u: {  // None (sharp corners)
            dist = sd_box(in.local_pos, in.half_size);
        }
        case 1u: {  // Round (circular arcs)
            dist = sd_rounded_box(in.local_pos, in.half_size, in.corner_param1);
        }
        case 2u: {  // Cut (chamfered at 45°)
            dist = sd_chamfer_box(in.local_pos, in.half_size, in.corner_param1);
        }
        case 3u: {  // InverseRound (concave arcs)
            dist = sd_inverse_round_box(in.local_pos, in.half_size, in.corner_param1);
        }
        case 4u: {  // Squircle (superellipse)
            dist = sd_squircle_box(in.local_pos, in.half_size, in.corner_param1, in.corner_param2);
        }
        default: {
            // Fallback to sharp corners
            dist = sd_box(in.local_pos, in.half_size);
        }
    }

    // Anti-aliasing width based on screen-space derivative
    // This ensures AA is always ~1 pixel wide regardless of zoom level
    let aa_width = length(vec2<f32>(dpdx(dist), dpdy(dist)));

    // Compute fill alpha with smooth transition
    // smoothstep creates a smooth gradient from 0 to 1 across the edge
    let fill_alpha = 1.0 - smoothstep(-aa_width, aa_width, dist);

    // Compute stroke alpha (if stroke width > 0)
    var stroke_alpha = 0.0;
    if in.stroke_width > 0.0 {
        // For a stroke, we want to render a "ring" around the shape boundary
        // The ring extends half_stroke outward and half_stroke inward from dist=0

        // Use the absolute distance from the boundary for stroke calculation
        let stroke_dist = abs(dist) - in.stroke_width * 0.5;

        // Stroke is visible when stroke_dist < 0 (inside the stroke ring)
        stroke_alpha = 1.0 - smoothstep(-aa_width, aa_width, stroke_dist);
    }

    // Blend fill and stroke
    // The stroke should be drawn on top of the fill, completely replacing it in the stroke region

    // First apply fill
    var final_color = in.fill_color * fill_alpha;

    // Then blend stroke on top using standard "over" operator
    // When stroke_alpha = 1, stroke completely replaces fill
    // When stroke_alpha = 0, fill shows through
    final_color = mix(final_color, in.stroke_color, stroke_alpha);

    return final_color;
}
