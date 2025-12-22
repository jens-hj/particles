// Textured glyph shader (alpha mask atlas)
//
// The renderer is expected to provide:
// - positions in screen-space pixels (top-left origin, +Y down)
// - UVs into a single-channel (R8) atlas where the glyph coverage is stored in `.r`
// - a per-vertex RGBA tint color (linear)
//
// Blending should be ALPHA (src over).

struct Globals {
    screen_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(1) @binding(0)
var glyph_atlas: texture_2d<f32>;

@group(1) @binding(1)
var glyph_sampler: sampler;

struct VertexInput {
    @location(0) pos_px: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Convert from screen-space pixels to NDC [-1, 1]
    let ndc = (in.pos_px / globals.screen_size) * 2.0 - 1.0;
    out.position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);

    out.uv = in.uv;
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let cov = textureSample(glyph_atlas, glyph_sampler, in.uv).r;

    // Atlas is a coverage mask; tint alpha is multiplied by coverage.
    // RGB is also pre-multiplied by coverage so standard alpha blending works well.
    let a = in.color.a * cov;
    return vec4<f32>(in.color.rgb * cov, a);
}
