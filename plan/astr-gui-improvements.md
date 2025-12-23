# Astra-GUI Optimization & Consistency Improvements Plan

## Overview
This plan addresses optimization, API consistency, and proper separation of concerns between `astra-gui` (core) and `astra-gui-wgpu` (backend) based on comprehensive codebase analysis and modern Rust GUI best practices.

## Current State Assessment

### ✅ Strengths
- **Excellent backend separation**: Zero WGPU dependencies in core
- **Solid layout algorithm**: Proper flexbox-like implementation with margin collapsing
- **Clean architecture**: Well-organized module structure
- **Good tessellation**: Efficient triangle generation for shapes

### ⚠️ Issues Identified

#### API Consistency Problems
1. **Mixed mutation styles**: Public fields + builder methods create two ways to do everything
2. **Runtime assertions**: Content vs children mutual exclusion not enforced at compile time
3. **Leaky Size abstraction**: `Size::resolve()` has misleading fallback behavior

#### Performance Issues
1. **Redundant measurements**: FitContent nodes measured multiple times
2. **Shape cloning overhead**: Every shape cloned when collecting for output
3. **Vec allocations**: `measure_children()` allocates unnecessarily
4. **No layout caching**: Static UIs recompute layout every frame
5. **No geometry caching**: WGPU backend re-uploads unchanged geometry every frame

#### Backend Separation
1. **Stub code**: `cosmic/mod.rs` in astra-gui-wgpu is vestigial (MINOR)
2. **All other code properly separated** ✅

## Proposed Improvements

### Phase 1: Core API Consistency (astra-gui)

**Priority: HIGH | Impact: HIGH | Effort: MEDIUM**

#### 1.1 Make Node fields private, enforce builder pattern
**Files**: `crates/astra-gui/src/node.rs`

Current problem:
```rust
// Two ways to do the same thing:
node.width = Size::px(100.0);           // Direct mutation
node.with_width(Size::px(100.0));       // Builder
```

Solution: Make all fields private except `computed`, keep only builder methods.

**Breaking change**: Yes, but improves consistency

#### 1.2 Add `Spacing` convenience methods
**Files**: `crates/astra-gui/src/layout.rs`

Add commonly-used constructors:
```rust
impl Spacing {
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self
    pub fn from_trbl(top: f32, right: f32, bottom: f32, left: f32) -> Self
}
```

**Breaking change**: No (additive only)

#### 1.3 Fix Size::resolve() documentation
**Files**: `crates/astra-gui/src/layout.rs`

Either:
- Remove misleading Fill/FitContent fallbacks and panic with clear message
- Or rename to `resolve_or_fallback()` and add proper `try_resolve() -> Option<f32>`

**Breaking change**: Potentially, if we panic instead of fallback

### Phase 2: Core Performance Optimizations (astra-gui)

**Priority: HIGH | Impact: HIGH | Effort: MEDIUM**

#### 2.1 Cache measurements in compute_layout
**Files**: `crates/astra-gui/src/node.rs` (~lines 270-350)

Current issue:
```rust
// In compute_layout_with_parent_size_and_measurer
if self.width.is_fit_content() {
    let measured = self.measure_node(measurer);  // Call 1
    // ...
}
if self.height.is_fit_content() {
    let measured = self.measure_node(measurer);  // Call 2 (duplicate!)
    // ...
}
```

Solution: Call `measure_node()` once, store result:
```rust
let measured_size = if self.width.is_fit_content() || self.height.is_fit_content() {
    Some(self.measure_node(measurer))
} else {
    None
};
```

**Breaking change**: No (internal optimization)

#### 2.2 Avoid Vec allocation in measure_children
**Files**: `crates/astra-gui/src/node.rs` (~lines 200-250)

Current code allocates a Vec for all child measurements.

Solution: Iterate twice (once for max, once for sum) or use a small stack buffer for common cases.

**Breaking change**: No (internal optimization)

#### 2.3 Use Cow<Shape> to avoid cloning
**Files**: `crates/astra-gui/src/output.rs` (~line 130)

Current:
```rust
let mut shape_with_opacity = shape.clone();  // Unnecessary allocation
shape_with_opacity.apply_opacity(combined_opacity);
```

Solution: Either:
- Apply opacity without cloning (modify in-place if owned)
- Or use `Cow<Shape>` to clone only when needed

**Breaking change**: No (internal optimization)

### Phase 3: WGPU Backend Performance (astra-gui-wgpu)

**Priority: HIGH | Impact: VERY HIGH | Effort: HIGH**

#### 3.1 Pre-allocate buffers based on previous frame
**Files**: `crates/astra-gui-wgpu/src/lib.rs`

Add fields to track previous frame:
```rust
pub struct Renderer {
    // ... existing fields
    last_frame_vertex_count: usize,
    last_frame_index_count: usize,
}
```

In render method:
```rust
self.wgpu_vertices.clear();
self.wgpu_vertices.reserve(self.last_frame_vertex_count);
// ... collect vertices
self.last_frame_vertex_count = self.wgpu_vertices.len();
```

**Breaking change**: No (internal optimization)

#### 3.2 Convert vertex colors to u8 for memory bandwidth
**Files**: `crates/astra-gui-wgpu/src/vertex.rs`, `crates/astra-gui-wgpu/src/shaders/ui.wgsl`

Current vertex: 24 bytes (pos: 8 bytes + color: 16 bytes)
Optimized vertex: 12 bytes (pos: 8 bytes + color: 4 bytes)

Change vertex format to use `[u8; 4]` color, convert to `f32` in shader.

**Breaking change**: No (internal to wgpu backend)

#### 3.3 Batch draw calls by scissor rect
**Files**: `crates/astra-gui-wgpu/src/lib.rs` (~lines 400-500)

Current: One draw call per shape with scissor
Optimized: Group shapes by identical scissor, merge consecutive index ranges

Impact: Fewer draw calls, better GPU utilization

**Breaking change**: No (rendering optimization)

#### 3.4 Remove vestigial cosmic/mod.rs stub
**Files**: `crates/astra-gui-wgpu/src/text/cosmic/mod.rs`

This file is a stub - actual cosmic-text integration is in `astra-gui-text`.

Action: Delete the file if it serves no purpose.

**Breaking change**: No (cleanup)

### Phase 4: Advanced Optimizations (OPTIONAL - Future Work)

**Priority: MEDIUM | Impact: VERY HIGH | Effort: VERY HIGH**

#### 4.1 GPU compute tessellation for complex shapes
**Files**: New compute shader for squircles/rounded corners

Move CPU tessellation to GPU compute shader, especially for:
- Squircle (most complex math)
- InverseRound corners
- High-segment-count rounded corners

Benefit: 50%+ reduction in CPU load for shape-heavy UIs

#### 4.2 Layout caching with dirty tracking
**Files**: `crates/astra-gui/src/node.rs`, new `cache.rs` module

Add node IDs and dirty flags to cache computed layouts between frames.

Benefit: Massive performance improvement for static UIs

#### 4.3 SDF text rendering
**Files**: `crates/astra-gui-wgpu/src/text/`, new SDF shader

Replace coverage-mask atlas with signed distance field rendering for resolution-independent text scaling.

Benefit: Better text quality at all scales, especially for zoom/transforms

## Implementation Order

### ✅ Immediate (High value, low effort) - COMPLETED
1. ✅ Pre-allocate buffers based on previous frame (3.1)
2. ✅ Cache measurements to avoid duplicate measure_node() (2.1)
3. ✅ Remove cosmic/mod.rs stub (3.4)
4. ✅ Avoid Vec allocation in measure_children (2.2)
5. ✅ Add Spacing convenience methods (1.2) - improved with `symmetric()` and `trbl()`

### Short-term (High value, medium effort) - TODO
1. Make Node fields private (1.1)
2. Convert vertex colors to u8 (3.2)
3. Batch draw calls by scissor rect (3.3)
4. Use Cow<Shape> to avoid cloning (2.3)

### Medium-term (High value, high effort) - TODO
1. Fix Size::resolve() semantics (1.3)

### Future work (Deferred)
11. GPU compute tessellation (4.1)
12. Layout caching system (4.2)
13. SDF text rendering (4.3)

## Critical Files to Modify

### astra-gui (core)
- `crates/astra-gui/src/node.rs` - Fields privacy, measurement caching, measure_children optimization
- `crates/astra-gui/src/layout.rs` - Spacing convenience methods, Size::resolve() fix
- `crates/astra-gui/src/output.rs` - Shape cloning optimization

### astra-gui-wgpu (backend)
- `crates/astra-gui-wgpu/src/lib.rs` - Buffer pre-allocation, draw call batching
- `crates/astra-gui-wgpu/src/vertex.rs` - Color format change to u8
- `crates/astra-gui-wgpu/src/shaders/ui.wgsl` - Shader changes for u8 color
- `crates/astra-gui-wgpu/src/text/cosmic/mod.rs` - DELETE (vestigial stub)

## Breaking Changes Summary

**API Breaking Changes:**
1. Making Node fields private (1.1) - **BREAKING**
2. Size::resolve() behavior change (1.3) - **POTENTIALLY BREAKING**

**Non-Breaking Changes:**
- All performance optimizations are internal
- Convenience methods are additive
- Backend changes don't affect public API

## Success Metrics

1. **Performance**: 30-50% reduction in frame time for typical UIs
2. **Memory**: 50% reduction in vertex buffer bandwidth (u8 colors)
3. **API Consistency**: Single, clear way to configure nodes (builder pattern)
4. **Code Quality**: No vestigial code, cleaner separation of concerns

## Notes

- The codebase already follows excellent architectural practices
- Main opportunities are in performance optimization and API polish
- No core logic needs to move between crates (separation is already correct)
- All optimizations are low-risk with high potential impact
