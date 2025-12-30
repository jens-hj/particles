## Executive Summary

Implement GPU-based analytic anti-aliasing for all GUI components using Signed Distance Fields (SDF), replacing CPU tessellation with shader-based rendering for pixel-perfect edges at any scale.

**Goals:**
- Analytic AA for all 5 corner types: None, Round, Cut, InverseRound, Squircle
- Both fills and strokes anti-aliased properly
- 80-90% vertex count reduction (36+ vertices → 4 vertices per rounded rect)
- Resolution-independent quality (perfect AA at any DPI/zoom)
- Backend-only changes (astra-gui-wgpu crate only)

**Key Approach:**
- Use **instanced rendering** with unit quad (4 shared vertices)
- Pass shape parameters via instance attributes (48 bytes/instance)
- Fragment shader computes SDF and coverage per-pixel
- Anti-aliasing via `fwidth()` + `smoothstep()` pattern
- Hybrid: analytic for simple strokes, tessellation fallback for complex strokes

---

## 1. Architecture Overview

### Current State
- **Tessellation:** CPU-based in `/home/j/repos/particles/crates/astra-gui/src/tessellate.rs`
  - 8 segments per curved corner = 36+ vertices per rounded rect
  - Triangle fan for fills, quad strips for strokes
- **Vertex:** 12 bytes (pos: [f32; 2], color: [u8; 4])
- **Shader:** Pass-through fragment shader, no AA
- **Text:** R8 atlas with nearest filtering

### New SDF Architecture
- **Instanced rendering:** Single unit quad (4 vertices) shared across all rectangles
- **Instance data:** 48 bytes per rectangle with shape parameters
- **Fragment shader:** Computes SDF based on corner type, calculates coverage
- **AA technique:** Screen-space derivatives (`fwidth`) + `smoothstep`

---

## 2. SDF Functions for All Corner Types

### CornerShape::None (Sharp corners)
```wgsl
fn sd_box(p: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}
```
**Complexity:** Trivial

### CornerShape::Round(radius)
```wgsl
fn sd_rounded_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}
```
**Complexity:** Low (Inigo Quilez proven formula)

### CornerShape::Cut(distance)
```wgsl
fn sd_chamfer_box(p: vec2<f32>, size: vec2<f32>, chamfer: f32) -> f32 {
    var p_local = abs(p) - size;
    if p_local.y > p_local.x { p_local = vec2(p_local.y, p_local.x); }
    p_local.y += chamfer;
    let k = 1.0 - sqrt(2.0) * 0.5;
    if p_local.y < 0.0 && p_local.y + p_local.x * k < 0.0 { return p_local.x; }
    if p_local.x < p_local.y { return (p_local.x + p_local.y) * sqrt(0.5); }
    return length(p_local);
}
```
**Complexity:** Medium (requires conditionals for diagonal)

### CornerShape::InverseRound(radius)
```wgsl
fn sd_inverse_round_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let inner_size = size - vec2(radius);
    let corner_offset = size - vec2(radius);
    let corner_pos = abs(p) - corner_offset;
    
    if corner_pos.x > 0.0 && corner_pos.y > 0.0 {
        return -(length(corner_pos) - radius);  // Inverted circle
    }
    return sd_box(p, inner_size);
}
```
**Complexity:** High (box minus circles composition)

### CornerShape::Squircle { extent, smoothness }
```wgsl
fn sd_squircle_box(p: vec2<f32>, size: vec2<f32>, radius: f32, smoothness: f32) -> f32 {
    let n = 2.0 + smoothness;
    let corner_offset = size - vec2(radius);
    let p_corner = abs(p) - corner_offset;
    
    if p_corner.x <= 0.0 || p_corner.y <= 0.0 {
        return sd_box(p, size);
    }
    
    // Power distance approximation (exact SDF unsolved)
    let p_abs = abs(p_corner);
    let power_sum = pow(p_abs.x, n) + pow(p_abs.y, n);
    return pow(power_sum, 1.0 / n) - radius;
}
```
**Complexity:** Very high (no exact closed-form solution exists)

---

## 3. Instance Data Structure

```rust
// In new file: /home/j/repos/particles/crates/astra-gui-wgpu/src/instance.rs
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectInstance {
    pub center: [f32; 2],        // 8 bytes
    pub half_size: [f32; 2],     // 8 bytes
    pub fill_color: [u8; 4],     // 4 bytes (packed RGBA)
    pub stroke_color: [u8; 4],   // 4 bytes
    pub stroke_width: f32,       // 4 bytes
    pub corner_type: u32,        // 4 bytes (0=None, 1=Round, 2=Cut, 3=InverseRound, 4=Squircle)
    pub corner_param1: f32,      // 4 bytes (radius/extent)
    pub corner_param2: f32,      // 4 bytes (smoothness for squircle)
    pub _padding: [u32; 2],      // 8 bytes (16-byte alignment)
}
// Total: 48 bytes per instance

impl From<&StyledRect> for RectInstance {
    fn from(rect: &StyledRect) -> Self {
        let center = [(rect.rect.min[0] + rect.rect.max[0]) * 0.5,
                      (rect.rect.min[1] + rect.rect.max[1]) * 0.5];
        let half_size = [(rect.rect.max[0] - rect.rect.min[0]) * 0.5,
                         (rect.rect.max[1] - rect.rect.min[1]) * 0.5];
        
        let (corner_type, param1, param2) = match rect.corner_shape {
            CornerShape::None => (0, 0.0, 0.0),
            CornerShape::Round(r) => (1, r, 0.0),
            CornerShape::Cut(d) => (2, d, 0.0),
            CornerShape::InverseRound(r) => (3, r, 0.0),
            CornerShape::Squircle { extent, smoothness } => (4, extent, smoothness),
        };
        
        // ... conversion logic
    }
}
```

---

## 4. Shader Implementation

### New file: `/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/ui_sdf.wgsl`

**Key sections:**
1. **Vertex shader:** Transforms unit quad to screen-space rectangle (with padding for stroke)
2. **Fragment shader:** 
   - Dispatches to correct SDF function based on `corner_type`
   - Computes AA width using `fwidth(distance)`
   - Calculates fill and stroke alpha using `smoothstep`
   - Blends fill + stroke with proper compositing

**Anti-aliasing pattern:**
```wgsl
let dist = sd_function(in.local_pos, in.half_size, params);
let aa_width = length(vec2(dpdx(dist), dpdy(dist)));
let fill_alpha = 1.0 - smoothstep(-aa_width, aa_width, dist);
```

**Stroke rendering:**
```wgsl
let half_stroke = stroke_width * 0.5;
let outer = dist + half_stroke;
let inner = dist - half_stroke;
let outer_alpha = 1.0 - smoothstep(-aa_width, aa_width, outer);
let inner_alpha = smoothstep(-aa_width, aa_width, inner);
let stroke_alpha = outer_alpha * inner_alpha;
```

---

## 5. Stroke Strategy (Hybrid Approach)

**Problem:** Offsetting complex curves analytically is difficult.

**Solution:**
- **Simple strokes (None, Round, Cut):** Use analytic SDF (cheap and high quality)
- **Complex strokes (InverseRound, Squircle):** Fallback to CPU tessellation

This balances quality and implementation complexity while still achieving major vertex reduction for fills.

---

## 6. Implementation Phases

### Phase 1: Foundation ✅ COMPLETED
**Scope:** SDF rendering for ALL 5 corner types

1. ✅ Created `instance.rs` with `RectInstance` struct
2. ✅ Created `ui_sdf.wgsl` with all 5 SDF functions (box, rounded box, chamfer, inverse round, squircle)
3. ✅ Modified `lib.rs`:
   - Added SDF pipeline alongside existing tessellation pipeline
   - Created unit quad vertex/index buffers (4 vertices, 6 indices)
   - Added instance buffer with dynamic resizing
   - Implemented rendering logic to choose SDF vs tessellation
4. ✅ Tested with all corner types

**Success criteria (ALL MET):**
- ✅ All 5 corner shapes render with perfect AA
- ✅ Verified at various resolutions
- ✅ 89% vertex reduction achieved
- ✅ Layout spacing correctly preserved (stroke width fix applied)

### Phase 2: Cut & InverseRound ✅ COMPLETED
**Scope:** Add remaining corner types (except Squircle)

1. ✅ Implemented `sd_chamfer_box` in shader
2. ✅ Implemented `sd_inverse_round_box` in shader (fixed concave corner formula)
3. ✅ Updated instance conversion logic
4. ✅ Tested edge cases (tiny/large radii)

**Success criteria (ALL MET):**
- ✅ All 4 corner types render correctly
- ✅ Visual quality matches tessellated versions

### Phase 3: Squircle ✅ COMPLETED
**Scope:** Add most complex corner type

1. ✅ Implemented `sd_squircle_box` using power distance approximation
2. ✅ Handled edge cases (smoothness extremes)
3. ✅ Quality verified as acceptable

**Success criteria (ALL MET):**
- ✅ Squircle renders correctly with analytic AA
- ✅ Performance acceptable

### Phase 4: Stroke Support ✅ COMPLETED
**Scope:** Anti-aliased borders/strokes

1. ✅ Implemented stroke rendering in fragment shader
2. ✅ Fixed stroke color blending (alpha compositing)
3. ✅ Tested all corner types (None, Round, Cut, InverseRound, Squircle)
4. ✅ Tested various stroke widths (0.5px, 1px, 3px, 10px, 20px)
5. ✅ Created stroke_test.rs example for comprehensive testing

**Success criteria (ALL MET):**
- ✅ All strokes render with perfect AA on all corner types
- ✅ Analytic rendering works for all 5 corner shapes
- ✅ No visual artifacts observed
- ✅ Thin strokes (< 1px) and thick strokes (> 10px) both work correctly

### Phase 5: Text Improvements (Week 5)
**Scope:** Better text anti-aliasing

1. Change text sampler from `FilterMode::Nearest` to `Linear`
2. Add smoothstep to text fragment shader for better AA
3. Test text rendering quality improvement

**Optional:** Research MSDF text rendering for future enhancement

### Phase 6: Optimization & Polish (Week 6)
**Scope:** Performance and edge cases

1. Implement batching optimization (group by corner type)
2. Handle edge cases (rect smaller than stroke, negative sizes)
3. Performance benchmarking (target: 2-5x faster for typical UIs)
4. Documentation and examples

---

## 7. Critical Files to Modify

### New Files
1. **`/home/j/repos/particles/crates/astra-gui-wgpu/src/instance.rs`**
   - `RectInstance` struct + bytemuck traits
   - Conversion from `StyledRect`

2. **`/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/ui_sdf.wgsl`**
   - Complete SDF shader with all 5 corner types
   - Instanced vertex shader
   - AA fragment shader

### Modified Files
3. **`/home/j/repos/particles/crates/astra-gui-wgpu/src/lib.rs`**
   - Add SDF pipeline
   - Add instance buffer management
   - Add unit quad buffers
   - Rendering dispatch logic (SDF vs tessellation)

4. **`/home/j/repos/particles/crates/astra-gui-wgpu/src/shaders/text.wgsl`**
   - Change sampler to Linear filtering
   - Add smoothstep AA

### Reference Files (Read-only)
5. **`/home/j/repos/particles/crates/astra-gui/src/primitives.rs`**
   - Understand `CornerShape` enum
   - Understand `StyledRect` structure

6. **`/home/j/repos/particles/crates/astra-gui/src/tessellate.rs`**
   - Keep for fallback/reference

---

## 8. Performance Expectations

**Vertex Count:**
- Current: 36 vertices per rounded rect
- New: 4 vertices (shared unit quad) + 48 bytes instance data
- **Reduction: 89%**

**Memory:**
- Current: 36 × 12 bytes = 432 bytes
- New: 4 × 8 bytes + 48 bytes = 80 bytes  
- **Reduction: 81%**

**Speed (typical UI with 1000 rounded rects):**
- Vertex processing: **4-6x faster** (89% fewer vertices)
- Fragment processing: ~30-40 more instructions per pixel
- **Net: 2-5x faster** (vertex-bound workload)

**Quality:**
- Resolution-independent perfect AA at any zoom/DPI
- No more visible jaggies at high DPI

---

## 9. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Squircle SDF too complex/slow | Keep tessellation fallback for squircle only |
| InverseRound visual artifacts | Iterate on formula; fallback if needed |
| Complex stroke quality issues | Hybrid approach: tessellation for complex strokes |
| Batching complexity | Start simple; optimize later |

---

## 10. Testing Strategy

**Visual tests:**
- Example: `examples/sdf_aa_test.rs` showing all corner types at various scales
- Screenshot comparison (before/after)
- Zoom/DPI independence testing

**Performance tests:**
- Benchmark: 1000 rectangles at 1080p/1440p/4K
- Memory profiling (vertex/instance buffer sizes)
- Frame time comparison

**Edge cases:**
- Rectangle smaller than corner radius
- Stroke wider than rectangle  
- Zero-width/height rectangles
- Extreme corner parameters

---

## 11. References & Research

- **Inigo Quilez 2D SDF Library:** https://iquilezles.org/articles/distfunctions2d/
- **fwidth Anti-Aliasing:** http://www.numb3r23.net/2015/08/17/using-fwidth-for-distance-based-anti-aliasing/
- **Raph Levien Rounded Rects:** https://raphlinus.github.io/graphics/2020/04/21/blurred-rounded-rects.html
- **WGSL Spec:** https://www.w3.org/TR/WGSL/
