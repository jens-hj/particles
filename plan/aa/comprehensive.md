# Comprehensive Implementation Plan: Analytic Anti-Aliasing for astra-gui-wgpu

## Executive Summary

This plan details the implementation of GPU-based analytic anti-aliasing for ALL 5 corner shape types in astra-gui-wgpu, replacing the current CPU tessellation approach (8 segments/corner) with signed distance field (SDF) rendering for higher quality and performance.

**Goals:**
- Eliminate jagged edges on all corner types (fills and strokes)
- Reduce vertex count dramatically (from ~36 vertices per rounded rect to 4-6 vertices)
- Maintain backend-agnostic design (all changes in astra-gui-wgpu only)
- Support all 5 corner types: None, Round, Cut, InverseRound, Squircle

**Current State:**
- CPU tessellation: 8 segments per corner = 36+ vertices per rounded rect
- Vertex format: 12 bytes (pos: [f32; 2], color: [u8; 4])
- Pass-through fragment shader (no per-pixel logic)
- Text uses R8 atlas with nearest filtering (needs improvement)

---

## 1. SDF Mathematical Foundations

### 1.1 SDF Functions for Each Corner Type

#### **CornerShape::None** - Sharp 90° Corners
```wgsl
// Simple box SDF
fn sd_box(p: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}
```
**Complexity:** Trivial - just distance to axis-aligned rectangle.

#### **CornerShape::Round(radius)** - Circular Arc Corners
```wgsl
// Rounded box SDF (single radius for all corners)
fn sd_rounded_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

// Rounded box with per-corner radii (more general)
fn sd_rounded_box_per_corner(p: vec2<f32>, size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // radii: xy=top-right, zw=bottom-right, continuing clockwise
    var r: vec2<f32>;
    if p.x > 0.0 {
        r = radii.xy;
    } else {
        r = radii.zw;
    }
    if p.y > 0.0 {
        r.x = r.x;
    } else {
        r.x = r.y;
    }
    
    let q = abs(p) - size + r.x;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r.x;
}
```
**Complexity:** Low - based on Inigo Quilez's proven formula.

#### **CornerShape::Cut(distance)** - Diagonal Chamfered Corners
```wgsl
// Chamfered box (45° cuts at corners)
fn sd_chamfer_box(p: vec2<f32>, size: vec2<f32>, chamfer: f32) -> f32 {
    var p_local = abs(p) - size;
    
    // Swap x/y if needed to always work with the diagonal case
    if p_local.y > p_local.x {
        p_local = vec2<f32>(p_local.y, p_local.x);
    }
    
    p_local.y += chamfer;
    let k = 1.0 - sqrt(2.0) * 0.5; // ~0.293
    
    if p_local.y < 0.0 && p_local.y + p_local.x * k < 0.0 {
        return p_local.x;
    }
    if p_local.x < p_local.y {
        return (p_local.x + p_local.y) * sqrt(0.5);
    }
    return length(p_local);
}
```
**Complexity:** Medium - requires conditional branching for diagonal line segment.

#### **CornerShape::InverseRound(radius)** - Concave Circular Arcs
```wgsl
// Inverse rounded corners (concave)
// Strategy: Combine outer box with SUBTRACTED circles at corners
fn sd_inverse_round_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    // The shape is: rect MINUS circles at each corner
    // This creates concave indentations
    
    // First, check if we're inside the shrunken inner rectangle
    let inner_size = size - vec2<f32>(radius);
    let box_dist = sd_box(p, inner_size);
    
    // For each corner, compute distance to inverted circle
    let corner_offset = size - vec2<f32>(radius);
    let corner_pos = abs(p) - corner_offset;
    
    // If we're in the corner region, use circle distance (inverted)
    if corner_pos.x > 0.0 && corner_pos.y > 0.0 {
        // Distance to circle from this corner
        let circle_dist = length(corner_pos) - radius;
        // Invert: inside circle = outside shape
        return -circle_dist;
    }
    
    // Otherwise use box distance
    return box_dist;
}
```
**Complexity:** High - requires conditional logic and circle-box combination.

#### **CornerShape::Squircle { radius, smoothness }** - Superellipse
```wgsl
// Superellipse SDF (approximation - exact is computationally expensive)
// Formula: |x|^n + |y|^n = r^n where n = 2.0 + smoothness
fn sd_squircle_box(p: vec2<f32>, size: vec2<f32>, radius: f32, smoothness: f32) -> f32 {
    let n = 2.0 + smoothness;
    
    // Work in corner-relative coordinates
    let corner_offset = size - vec2<f32>(radius);
    let p_corner = abs(p) - corner_offset;
    
    // If we're far from corners, use simple box distance
    if p_corner.x <= 0.0 || p_corner.y <= 0.0 {
        return sd_box(p, size);
    }
    
    // In corner region, compute superellipse distance
    // Approximate SDF using power distance metric
    let p_abs = abs(p_corner);
    
    // Power distance (not exact SDF, but close enough for AA)
    let power_sum = pow(p_abs.x, n) + pow(p_abs.y, n);
    let power_dist = pow(power_sum, 1.0 / n);
    
    return power_dist - radius;
}
```
**Complexity:** VERY HIGH - superellipse exact SDF is unsolved problem. We use approximation.
**Alternative:** Pre-tessellate squircle, or use iterative Newton-Raphson root finding (expensive).

---

## 2. Stroke/Border Anti-Aliasing Strategy

### 2.1 Stroke SDF Theory

For strokes (outlines), we need TWO distance fields:
- **Outer boundary:** `sdf(p) + stroke_width/2`
- **Inner boundary:** `sdf(p) - stroke_width/2`

The stroke exists where: `abs(sdf(p)) < stroke_width/2`

```wgsl
fn stroke_alpha(dist: f32, stroke_width: f32, aa_width: f32) -> f32 {
    let half_width = stroke_width * 0.5;
    let outer = dist + half_width;
    let inner = dist - half_width;
    
    // AA on outer edge (outside -> stroke)
    let outer_alpha = 1.0 - smoothstep(-aa_width, aa_width, outer);
    
    // AA on inner edge (stroke -> inside fill)
    let inner_alpha = smoothstep(-aa_width, aa_width, inner);
    
    return outer_alpha * inner_alpha;
}
```

### 2.2 Per-Corner-Type Stroke Complexity

| Corner Type | Stroke Complexity | Notes |
|-------------|------------------|-------|
| **None** | Trivial | Sharp corners work perfectly with box SDF |
| **Round** | Low | Offset circular arcs are still circular |
| **Cut** | Medium | Offset diagonal lines remain straight |
| **InverseRound** | **HIGH** | Offset concave curves are complex |
| **Squircle** | **VERY HIGH** | Offset superellipse is nearly impossible analytically |

**Recommendation:** For InverseRound and Squircle strokes, consider hybrid approach:
- Fill: Analytic SDF
- Stroke: CPU tessellation (but with fewer segments than current)

---

## 3. Vertex Format Design

### 3.1 Challenge: Encoding Corner Type + Parameters

We need to pass to the fragment shader:
1. Rectangle bounds (min/max or center/half-size)
2. Corner type (enum: 0=None, 1=Round, 2=Cut, 3=InverseRound, 4=Squircle)
3. Corner parameters (radius, smoothness, per-corner radii)
4. Stroke parameters (width, color)
5. Fill color

**Problem:** Current vertex format is only 12 bytes. We need MUCH more data per rectangle.

### 3.2 Solution Options

#### **Option A: Instance Data (Recommended)**
Use **instanced rendering** with instance attributes:

```rust
// Vertex: just the 4 corners of a unit quad
#[repr(C)]
struct Vertex {
    pos: [f32; 2],  // [-1,-1] to [1,1] unit quad
}

// Instance data: one per rectangle
#[repr(C)]
struct RectInstance {
    center: [f32; 2],           // 8 bytes
    half_size: [f32; 2],        // 8 bytes
    fill_color: [u8; 4],        // 4 bytes (packed)
    stroke_color: [u8; 4],      // 4 bytes
    stroke_width: f32,          // 4 bytes
    corner_type: u32,           // 4 bytes (0-4)
    corner_param1: f32,         // 4 bytes (radius or smoothness)
    corner_param2: f32,         // 4 bytes (smoothness for squircle)
    _padding: [u32; 2],         // 8 bytes (align to 16)
}
// Total: 48 bytes per instance
```

**Vertex shader:**
```wgsl
struct Vertex {
    @location(0) pos: vec2<f32>,  // Unit quad [-1,1]
}

struct Instance {
    @location(1) center: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) fill_color: vec4<f32>,
    @location(4) stroke_color: vec4<f32>,
    @location(5) stroke_width: f32,
    @location(6) corner_type: u32,
    @location(7) corner_param1: f32,
    @location(8) corner_param2: f32,
}

@vertex
fn vs_main(vert: Vertex, inst: Instance) -> VertexOutput {
    // Transform unit quad to screen-space rectangle
    let world_pos = inst.center + vert.pos * inst.half_size;
    // ... rest of vertex shader
}
```

**Pros:**
- Clean separation: geometry (quad) vs data (instance)
- Efficient: only 4 vertices total (shared across all rects)
- GPU-friendly: instancing is highly optimized

**Cons:**
- More complex batching logic
- Need to sort/batch by corner type for optimal performance

#### **Option B: Uniform Buffer + Vertex Indices**
Store all rectangle data in a uniform buffer, pass index via vertex.

**Pros:**
- Flexible
- Easy to update parameters

**Cons:**
- Uniform buffer size limits (16KB typically = ~340 rects)
- Not scalable for complex UIs

#### **Option C: Storage Buffer (SSBO)**
Like Option B but with storage buffer (larger capacity).

**Pros:**
- Scalable
- Efficient for many rectangles

**Cons:**
- More complex setup
- Still need vertex indices

### 3.3 Recommended Approach: Instanced Rendering

Use **Option A** for fills, with fallback to tessellation for complex strokes.

---

## 4. Shader Architecture

### 4.1 New Shader: `ui_sdf.wgsl`

```wgsl
// ============================================================================
// UI SDF Shader - Analytic Anti-Aliasing for All Corner Types
// ============================================================================

struct Uniforms {
    screen_size: vec2<f32>,
}

struct VertexInput {
    @location(0) pos: vec2<f32>,  // Unit quad [-1, 1]
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

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Expand unit quad to screen-space rectangle
    // Add small padding for stroke (expand by stroke_width)
    let padding = inst.stroke_width;
    let expanded_size = inst.half_size + vec2<f32>(padding);
    out.world_pos = inst.center + vert.pos * expanded_size;
    
    // Convert to NDC
    let ndc = (out.world_pos / uniforms.screen_size) * 2.0 - 1.0;
    out.clip_pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    
    // Pass through data
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

fn sd_box(p: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn sd_rounded_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

fn sd_chamfer_box(p: vec2<f32>, size: vec2<f32>, chamfer: f32) -> f32 {
    var p_local = abs(p) - size;
    
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

fn sd_inverse_round_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let inner_size = size - vec2<f32>(radius);
    let corner_offset = size - vec2<f32>(radius);
    let corner_pos = abs(p) - corner_offset;
    
    if corner_pos.x > 0.0 && corner_pos.y > 0.0 {
        return -(length(corner_pos) - radius);
    }
    
    return sd_box(p, inner_size);
}

fn sd_squircle_box(p: vec2<f32>, size: vec2<f32>, radius: f32, smoothness: f32) -> f32 {
    let n = 2.0 + smoothness;
    let corner_offset = size - vec2<f32>(radius);
    let p_corner = abs(p) - corner_offset;
    
    if p_corner.x <= 0.0 || p_corner.y <= 0.0 {
        return sd_box(p, size);
    }
    
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
    // Compute SDF based on corner type
    var dist: f32;
    
    switch in.corner_type {
        case 0u: {  // None
            dist = sd_box(in.local_pos, in.half_size);
        }
        case 1u: {  // Round
            dist = sd_rounded_box(in.local_pos, in.half_size, in.corner_param1);
        }
        case 2u: {  // Cut
            dist = sd_chamfer_box(in.local_pos, in.half_size, in.corner_param1);
        }
        case 3u: {  // InverseRound
            dist = sd_inverse_round_box(in.local_pos, in.half_size, in.corner_param1);
        }
        case 4u: {  // Squircle
            dist = sd_squircle_box(in.local_pos, in.half_size, in.corner_param1, in.corner_param2);
        }
        default: {
            dist = sd_box(in.local_pos, in.half_size);
        }
    }
    
    // Anti-aliasing width based on screen-space derivative
    let aa_width = length(vec2<f32>(dpdx(dist), dpdy(dist)));
    
    // Compute fill alpha
    let fill_alpha = 1.0 - smoothstep(-aa_width, aa_width, dist);
    
    // Compute stroke alpha
    var stroke_alpha = 0.0;
    if in.stroke_width > 0.0 {
        let half_stroke = in.stroke_width * 0.5;
        let outer = dist + half_stroke;
        let inner = dist - half_stroke;
        let outer_alpha = 1.0 - smoothstep(-aa_width, aa_width, outer);
        let inner_alpha = smoothstep(-aa_width, aa_width, inner);
        stroke_alpha = outer_alpha * inner_alpha;
    }
    
    // Blend fill and stroke
    let fill_contrib = in.fill_color * fill_alpha;
    let stroke_contrib = in.stroke_color * stroke_alpha;
    
    // Stroke on top of fill (pre-multiplied alpha blending)
    let final_color = mix(fill_contrib, stroke_contrib, stroke_alpha);
    
    return final_color;
}
```

### 4.2 Text Rendering Improvements

**Current:** R8 atlas, nearest filtering
**Improved:** Use SDF-based text rendering or bilinear filtering

```wgsl
// In text.wgsl fragment shader, replace:
let cov = textureSample(glyph_atlas, glyph_sampler, in.uv).r;

// With anti-aliased coverage:
let cov_raw = textureSample(glyph_atlas, glyph_sampler, in.uv).r;
let aa_width = fwidth(cov_raw);
let cov = smoothstep(0.5 - aa_width, 0.5 + aa_width, cov_raw);
```

**Better approach:** Switch to **MSDF (Multi-channel Signed Distance Field)** text atlas:
- Store signed distance in RGB channels
- Allows sharp text at any scale
- Requires pre-processing font into MSDF atlas

---

## 5. Implementation Phases

### Phase 1: Foundation & Simple Shapes (Week 1)
**Goal:** Implement SDF rendering for None and Round corner types only.

1. **Create new vertex/instance formats**
   - Add `RectInstance` struct
   - Implement `From<StyledRect>` conversion

2. **Create `ui_sdf.wgsl` shader**
   - Implement `sd_box` and `sd_rounded_box`
   - Basic vertex/fragment shader with instancing
   - Anti-aliasing with `fwidth` + `smoothstep`

3. **Update `Renderer`**
   - Add SDF pipeline alongside existing tessellation pipeline
   - Add instance buffer management
   - Implement batching for SDF instances

4. **Testing**
   - Verify None and Round fills render correctly
   - Verify AA quality at various zoom levels
   - Performance benchmark vs tessellation

**Expected Results:**
- 80% reduction in vertices for rounded rectangles
- Pixel-perfect anti-aliasing
- Smooth edges at all scales

### Phase 2: Cut & InverseRound (Week 2)
**Goal:** Add Cut and InverseRound support.

1. **Implement SDF functions**
   - `sd_chamfer_box` for Cut
   - `sd_inverse_round_box` for InverseRound

2. **Testing**
   - Verify all 4 corner types (None, Round, Cut, InverseRound)
   - Edge cases: very small/large radii
   - Overlap testing

**Challenge:** InverseRound SDF is complex - may require iteration.

### Phase 3: Squircle (Week 3)
**Goal:** Add Squircle support (most complex).

1. **Research optimal squircle SDF**
   - Test approximation vs exact iterative method
   - Performance vs quality tradeoff

2. **Implement chosen approach**
   - Add `sd_squircle_box` function
   - Handle edge cases (smoothness = 0, very high values)

3. **Testing**
   - Visual quality comparison with tessellated version
   - Performance profiling

**Fallback:** If exact SDF is too expensive, keep tessellation for Squircle only.

### Phase 4: Stroke Support (Week 4)
**Goal:** Add stroke/border anti-aliasing.

1. **Implement stroke rendering**
   - Modify fragment shader to handle strokes
   - Test with None, Round, Cut

2. **Hybrid approach for complex strokes**
   - InverseRound strokes: use tessellation
   - Squircle strokes: use tessellation

3. **Testing**
   - All corner types with various stroke widths
   - Thin strokes (< 1px)
   - Thick strokes

### Phase 5: Text Improvements (Week 5)
**Goal:** Improve text anti-aliasing.

1. **Bilinear filtering**
   - Change sampler from Nearest to Linear
   - Test quality improvement

2. **Optional: MSDF text**
   - Pre-process font to MSDF atlas
   - Update text shader for MSDF sampling
   - Benchmark quality vs performance

### Phase 6: Optimization & Polish (Week 6)
**Goal:** Performance optimization and edge case handling.

1. **Batching optimization**
   - Group instances by corner type
   - Minimize pipeline switches
   - Benchmark large UI (1000+ rectangles)

2. **Edge cases**
   - Rects smaller than stroke width
   - Very large corner radii
   - Negative sizes (clamping)

3. **Documentation**
   - Document SDF approach in code comments
   - Update README with performance gains
   - Create visual comparisons (before/after)

---

## 6. Performance Analysis

### 6.1 Expected Vertex Count Reduction

**Current (Tessellation):**
- Sharp corners: 4 vertices
- Round corners (8 segments/corner): 36 vertices
- Squircle (8 segments/corner): 36 vertices

**New (SDF):**
- All corner types: **4 vertices** (unit quad) + **48 bytes instance data**

**Savings:**
- Rounded rect: 36 vertices → 4 vertices = **89% reduction**
- Memory: (36 × 12 bytes) → (4 × 8 bytes + 48 bytes) = 432 → 80 bytes = **81% reduction**

### 6.2 Shader Cost Analysis

**Fragment shader complexity:**
| Corner Type | Instructions | Branches | Notes |
|-------------|-------------|----------|-------|
| None | ~15 | 0 | Simple box SDF |
| Round | ~25 | 1 | Rounded box SDF |
| Cut | ~35 | 3 | Conditional diagonal logic |
| InverseRound | ~40 | 2 | Circle-box combination |
| Squircle | ~50 | 2 | Power function (expensive) |

**Tradeoff:**
- Vertex processing: MUCH cheaper (89% fewer vertices)
- Fragment processing: Moderately more expensive (+30-40 instructions)
- **Net result:** Likely 2-5x faster for typical UIs (vertex-bound currently)

### 6.3 Expected Performance Gains

**Scenarios:**
1. **Simple UI (100 rectangles, mostly rounded):**
   - Current: ~3600 vertices
   - New: ~400 vertices + 100 instances
   - **Gain:** 3-4x faster

2. **Complex UI (1000 rectangles, mixed corner types):**
   - Current: ~36,000 vertices
   - New: ~4000 vertices + 1000 instances
   - **Gain:** 4-6x faster

3. **High-DPI displays:**
   - Current: AA quality degrades (more jaggies visible)
   - New: Perfect AA at any resolution (resolution-independent)
   - **Gain:** Infinite quality improvement

---

## 7. File-by-File Changes

### 7.1 New Files

#### `/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/ui_sdf.wgsl`
- New shader implementing SDF rendering
- All 5 corner type SDF functions
- Instanced vertex shader
- Fragment shader with AA

#### `/home/j/repos/particles/crates/astra-gui-wgpu/src/instance.rs`
- `RectInstance` struct definition
- Conversion from `StyledRect` to `RectInstance`
- Bytemuck traits for GPU upload

### 7.2 Modified Files

#### `/home/j/repos/particles/crates/astra-gui-wgpu/src/lib.rs`
**Changes:**
- Add SDF pipeline alongside tessellation pipeline
- Add instance buffer management
- Add batching logic for SDF instances
- Feature flag: `sdf-rendering` (optional, for gradual rollout)

```rust
pub struct Renderer {
    // Existing tessellation pipeline
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    
    // NEW: SDF pipeline
    sdf_pipeline: wgpu::RenderPipeline,
    sdf_instance_buffer: wgpu::Buffer,
    sdf_instance_capacity: usize,
    unit_quad_vertices: wgpu::Buffer,  // 4 vertices: [-1,-1], [1,-1], [1,1], [-1,1]
    unit_quad_indices: wgpu::Buffer,   // 6 indices: [0,1,2, 0,2,3]
    
    // ... rest
}
```

**Rendering logic:**
```rust
// Decide which pipeline to use per shape
for clipped in &output.shapes {
    match &clipped.shape {
        Shape::Rect(rect) => {
            if should_use_sdf(rect) {
                // Add to SDF instance batch
                sdf_instances.push(RectInstance::from(rect));
            } else {
                // Fallback to tessellation (e.g., for complex strokes)
                tessellate_and_add(rect);
            }
        }
        // ... text, etc.
    }
}

// Draw SDF instances in one call
if !sdf_instances.is_empty() {
    render_pass.set_pipeline(&self.sdf_pipeline);
    render_pass.set_vertex_buffer(0, self.unit_quad_vertices.slice(..));
    render_pass.set_vertex_buffer(1, self.sdf_instance_buffer.slice(..));
    render_pass.set_index_buffer(self.unit_quad_indices.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..6, 0, 0..sdf_instances.len() as u32);
}
```

#### `/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/text.wgsl`
**Changes:**
- Add bilinear filtering to sampler
- Optional: MSDF sampling logic

```wgsl
// Change sampler from Nearest to Linear
let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    mag_filter: wgpu::FilterMode::Linear,  // Changed from Nearest
    min_filter: wgpu::FilterMode::Linear,  // Changed from Nearest
    // ... rest
});

// In fragment shader, add AA
let cov_raw = textureSample(glyph_atlas, glyph_sampler, in.uv).r;
let aa_width = fwidth(cov_raw);
let cov = smoothstep(0.5 - aa_width, 0.5 + aa_width, cov_raw);
```

#### `/home/j/repos/particles/crates/astra-gui-wgpu/src/vertex.rs`
**Changes:**
- Add `RectInstance` struct (or move to new `instance.rs`)

### 7.3 No Changes Required

#### `/home/j/repos/particles/crates/astra-gui/src/tessellate.rs`
- Keep as-is for fallback and backward compatibility
- Used for complex strokes (InverseRound, Squircle)

#### `/home/j/repos/particles/crates/astra-gui/src/primitives.rs`
- No changes - backend-agnostic design preserved

---

## 8. Testing Strategy

### 8.1 Visual Tests

1. **Create test example:** `examples/sdf_aa_test.rs`
   - Grid of all 5 corner types
   - Various sizes (tiny to large)
   - Various zoom levels (test resolution independence)
   - With/without strokes

2. **Visual comparison:**
   - Screenshot before (tessellation)
   - Screenshot after (SDF)
   - Highlight differences

### 8.2 Performance Tests

1. **Benchmark:** `benches/sdf_vs_tessellation.rs`
   - Measure frame time for 1000 rectangles
   - Vary corner types
   - Vary screen resolution

2. **Memory profiling:**
   - Vertex buffer sizes
   - Instance buffer sizes
   - GPU memory usage

### 8.3 Edge Case Tests

Test cases:
- Rectangle smaller than corner radius (should clamp)
- Stroke wider than rectangle (should handle gracefully)
- Zero-width or zero-height rectangles
- Negative corner radii (should clamp to 0)
- Squircle smoothness extremes (0 to 10)

---

## 9. Research Resources & References

### 9.1 SDF Theory
- **Inigo Quilez 2D SDF Library:** https://iquilezles.org/articles/distfunctions2d/
  - Comprehensive catalog of SDF primitives
  - Proven formulas for box, rounded box, chamfered box
  
- **Using fwidth for Anti-Aliasing:** http://www.numb3r23.net/2015/08/17/using-fwidth-for-distance-based-anti-aliasing/
  - Explains `fwidth` + `smoothstep` pattern
  
- **Raph Levien's Blurred Rounded Rectangles:** https://raphlinus.github.io/graphics/2020/04/21/blurred-rounded-rects.html/
  - Discusses superellipse rendering challenges

### 9.2 Squircle SDFs
- **Squircle SDF Challenge:** No exact closed-form solution exists
- **Approximation approaches:**
  1. Power distance metric (used in our plan)
  2. Newton-Raphson iterative root finding (expensive)
  3. Pre-computed lookup table (memory intensive)
  
- **Fallback:** Keep tessellation for Squircle if SDF approximation quality insufficient

### 9.3 WGSL Specifics
- **WebGPU Shading Language Spec:** https://www.w3.org/TR/WGSL/
  - Derivative functions: `dpdx`, `dpdy`, `fwidth`
  - Switch statements (for corner type dispatch)

---

## 10. Risk Assessment & Mitigation

### 10.1 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Squircle SDF too slow | Medium | Medium | Fallback to tessellation for Squircle only |
| InverseRound visual artifacts | Low | High | Iterate on SDF formula; fallback if needed |
| Batching complexity | Medium | Low | Implement simple version first; optimize later |
| Stroke AA quality issues | Low | Medium | Use tessellation for complex strokes |
| WGSL switch performance | Low | Low | Profile; use if-else chain if faster |

### 10.2 Fallback Strategy

**Hybrid Rendering:**
- SDF for fills (all types)
- SDF for simple strokes (None, Round, Cut)
- Tessellation for complex strokes (InverseRound, Squircle)

This ensures quality while allowing gradual migration.

---

## 11. Success Criteria

1. **Visual Quality:**
   - ✓ No visible jaggies at 1080p, 1440p, 4K
   - ✓ Smooth edges at all zoom levels
   - ✓ Correct shape for all corner types

2. **Performance:**
   - ✓ 2-5x faster rendering for typical UIs
   - ✓ 80%+ vertex count reduction
   - ✓ <5ms frame time for 1000 rectangles @ 1080p

3. **Code Quality:**
   - ✓ Backend-agnostic (no changes to astra-gui core)
   - ✓ Well-documented SDF functions
   - ✓ Comprehensive test coverage

4. **Compatibility:**
   - ✓ All existing examples work unchanged
   - ✓ Feature flag for gradual rollout
   - ✓ Fallback to tessellation available

---

## 12. Future Enhancements

After initial implementation, consider:

1. **MSDF Text Rendering:**
   - Pre-process Inter font to MSDF atlas
   - Update text shader
   - Expected: Sharper text at all sizes

2. **Per-Corner Radii:**
   - Extend `CornerShape::Round` to support 4 different radii
   - Requires more complex SDF function

3. **Gradients & Patterns:**
   - Add gradient support to SDF shader
   - Requires texture sampling or procedural gradients

4. **Shadows & Glows:**
   - Use SDF for drop shadows
   - Outer glow = `smoothstep(glow_dist, 0, dist)`

5. **Batching Optimization:**
   - Sort instances by corner type
   - Multi-draw indirect for zero-overhead batching

---

## Critical Files for Implementation

### 1. `/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/ui_sdf.wgsl` (NEW)
**Reason:** Core SDF shader logic - all 5 corner types, AA, strokes

### 2. `/home/j/repos/particles/crates/astra-gui-wgpu/src/lib.rs`
**Reason:** Main renderer - add SDF pipeline, instance batching, rendering logic

### 3. `/home/j/repos/particles/crates/astra-gui-wgpu/src/instance.rs` (NEW)
**Reason:** Instance data structure for SDF rendering

### 4. `/home/j/repos/particles/crates/astra-gui/src/primitives.rs`
**Reason:** Reference for corner type definitions (read-only, understand data model)

### 5. `/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/text.wgsl`
**Reason:** Text AA improvements (bilinear filtering, smoothstep)

---

## Sources

- [Inigo Quilez 2D Distance Functions](https://iquilezles.org/articles/distfunctions2d/)
- [Using fwidth for Distance-Based Anti-Aliasing](http://www.numb3r23.net/2015/08/17/using-fwidth-for-distance-based-anti-aliasing/)
- [Blurred Rounded Rectangles - Raph Levien](https://raphlinus.github.io/graphics/2020/04/21/blurred-rounded-rects.html)
- [Going Round in Squircles - thndl](https://thndl.com/going-round-in-squircles.html)
- [WebGPU Shading Language Specification](https://www.w3.org/TR/WGSL/)
- [Signed Distance Fields - GM Shaders](https://mini.gmshaders.com/p/sdf)
- [Inigo Quilez Rounded Boxes Article](https://iquilezles.org/articles/roundedboxes/)
- [GitHub: glsl-aastep - Anti-Alias Smoothstep](https://github.com/glslify/glsl-aastep)
