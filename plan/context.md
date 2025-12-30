# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

1) ✅ Backend-agnostic text rendering with intrinsic measurement
2) ✅ Core layout ergonomics with `Size::FitContent` and `Overflow` policy
3) ✅ Performance optimizations and API consistency improvements (Dec 2025)
4) ✅ Interactive components system with declarative styles and smooth transitions (Dec 2025)
5) ✅ Analytic anti-aliasing with SDF rendering - ALL PHASES COMPLETE (Dec 2025)
   - ✅ Phase 1: Foundation (None, Round corners)
   - ✅ Phase 2: Cut & InverseRound corners
   - ✅ Phase 3: Squircle corners
   - ✅ Phase 4: Stroke support (all corner types)
6) Next: Text AA improvements (Phase 5) or other features

---

## What exists right now

### `astra-gui` (core)

Text-related core types:
- `Content::Text(TextContent)` in `crates/astra-gui/src/content.rs`
- `TextShape` and `Shape::Text(TextShape)` in `crates/astra-gui/src/primitives.rs`

Node tree emits text shapes:
- `Node::collect_shapes()` pushes `Shape::Text(TextShape::new(content_rect, text_content))` when `node.content` is `Content::Text(...)`.
- `content_rect` is computed from the node rect minus padding.

#### ✅ NEW: Intrinsic sizing + overflow policy (core)

- Added `Size::FitContent` in `crates/astra-gui/src/layout.rs`.
  - Represents "minimum size that fits content (text metrics or children), plus padding".
  - **Now fully functional**: resolves to measured intrinsic size via `ContentMeasurer` trait.

- Added `Overflow` in `crates/astra-gui/src/layout.rs`:
  - `Overflow::Visible`
  - `Overflow::Hidden` (**default**)
  - `Overflow::Scroll` (not implemented yet; treated like `Hidden` for clipping purposes)

- Added `Node.overflow` and `Node::with_overflow(...)` in `crates/astra-gui/src/node.rs` (default: `Overflow::Hidden`).

#### ✅ NEW: ContentMeasurer trait (backend-agnostic measurement)

Added `crates/astra-gui/src/measure.rs`:
- `ContentMeasurer` trait: backend-agnostic content measurement interface
- `MeasureTextRequest`: request structure for measuring text intrinsic size
- `IntrinsicSize`: measured width/height result

Layout implementation:
- `Node::measure_node()`: recursively measures intrinsic size (content + padding)
- `Node::measure_children()`: aggregates child measurements with margin/gap collapsing
- `Node::compute_layout_with_measurer()`: layout entry point that uses measurer for `FitContent`
- `compute_layout_with_parent_size_and_measurer()`: internal layout that resolves `FitContent` to measured sizes

The measurement algorithm mirrors the layout spacing rules exactly:
- Same margin/gap collapsing behavior
- Respects layout direction (horizontal/vertical)
- Returns border-box size (content + padding, excluding margins)

#### Clip rects derive from overflow policy

`FullOutput::from_node_with_debug_and_measurer()` builds `ClippedShape`s by walking the node tree:
- Computes `clip_rect` from ancestor overflow chain
- If any ancestor is `Hidden` (or `Scroll`), `clip_rect` is intersection of those ancestor rects
- `Visible` does not restrict the inherited clip rect
- Nodes fully clipped out are skipped early

Tessellation remains geometry-only:
- `Shape::Rect` gets tessellated into triangles
- `Shape::Text` is intentionally skipped (backend-rendered)

### `astra-gui-text` (text backend)

✅ **NEW: Implements `ContentMeasurer` trait**

- `Engine` and `CosmicEngine` both implement `ContentMeasurer`
- `measure_text()`: uses existing `shape_line()` with dummy rect to get metrics
- Returns `IntrinsicSize { width, height }` from `LineMetrics`

This keeps measurement backend-agnostic:
- Core layout depends only on `ContentMeasurer` trait
- Specific text engine (cosmic-text) lives in `astra-gui-text`
- Measurement reuses existing shaping infrastructure

### `astra-gui-wgpu` (backend)

- ✅ **NEW: SDF rendering pipeline** (`src/shaders/ui_sdf.wgsl`) with analytic anti-aliasing:
  - Signed Distance Field (SDF) based rendering for pixel-perfect AA at any scale
  - Instanced rendering: single unit quad (4 vertices) shared across all rectangles
  - Instance data: 48 bytes per rectangle with shape parameters
  - Fragment shader computes SDF and coverage per-pixel using `fwidth()` + `smoothstep()`
  - Currently supports: None (sharp), Round (circular arcs) corner types
  - Ready for: Cut (chamfer), InverseRound (concave), Squircle (superellipse) - shader code exists
  - 89% vertex reduction: 36+ vertices per rounded rect → 4 shared vertices
  - Resolution-independent: perfect AA at any DPI/zoom level

- Geometry pipeline exists (`src/shaders/ui.wgsl`) - kept for fallback/compatibility
- Text pipeline exists and is wired:
  - `src/shaders/text.wgsl` samples an `R8Unorm` atlas and tints by vertex color
  - CPU side generates per-glyph quads and uploads `R8` glyph bitmap into atlas

Architecture:
- `instance.rs`: `RectInstance` struct with bytemuck traits for GPU upload
- `lib.rs`: Dual pipeline setup - SDF for all rectangles, tessellation available as fallback
- All rectangles currently use SDF rendering with dramatic performance improvement

Clipping behavior:
- Text draws with **per-shape scissor** using `ClippedShape::clip_rect`:
  - indices for each text shape are recorded as `(start..end)` ranges
  - `draw_indexed` is issued per-range with the correct scissor rect

Important limitation:
- Geometry rendering is still a single batched draw and does **not** currently honor per-shape `clip_rect`. If we want overflow clipping for rects as well, the geometry path will need per-clip batching similar to the text path.

Examples:
- `crates/astra-gui-wgpu/examples/overflow.rs` showcases `Overflow::{Hidden, Visible, Scroll}` behaviors
  - ✅ **Updated to use `TextEngine` as `ContentMeasurer`** for proper `FitContent` sizing
  - Demo focuses on TEXT overflow because text respects scissor clipping
  - `Overflow::Scroll` is a placeholder (clips only; no scroll offsets implemented yet)

---

## Recent progress

### ✅ Completed: Intrinsic measurement for `Size::FitContent`

Implemented a complete measurement system:

1. **Core trait in `astra-gui`** (`src/measure.rs`):
   - `ContentMeasurer` trait for backend-agnostic content measurement
   - Measurement functions in `Node` that mirror layout spacing rules exactly
   - Layout functions that use measurer to resolve `FitContent`

2. **Implementation in `astra-gui-text`**:
   - `ContentMeasurer` implemented for `Engine` and `CosmicEngine`
   - Measurement reuses existing shaping/metrics infrastructure

3. **Integration in examples**:
   - Updated `overflow.rs` to create `TextEngine` and pass as measurer
   - `FitContent` now resolves to actual text metrics instead of falling back to parent size

The system is:
- **Backend-agnostic**: core doesn't depend on cosmic-text
- **Consistent**: measurement uses same spacing rules as layout
- **Recursive**: handles nested `FitContent` containers correctly
- **Short-circuits**: only measures when needed (Fixed/Relative/Fill skip measurement)

### ✅ Completed: Performance optimizations and API improvements (Dec 2025)

Implemented immediate and short-term optimizations from the improvement plan:

#### Core (`astra-gui`) optimizations:
1. **Measurement caching** (`node.rs`): Cache `measure_node()` result when both width and height are `FitContent` to avoid duplicate measurements
2. **Vec allocation elimination** (`node.rs`): Refactored `measure_children()` to compute size in single pass without allocating intermediate Vec
3. **API improvements** (`layout.rs`): 
   - Replaced `new()` with `trbl()` for clarity
   - Replaced `horizontal_vertical()` with `symmetric()` for CSS-style familiarity
   - Removed duplicate methods

#### Backend (`astra-gui-wgpu`) optimizations:
1. **Buffer pre-allocation** (`lib.rs`): Track previous frame vertex/index counts and pre-allocate buffers to reduce allocations
   - Geometry buffers: `last_frame_vertex_count`, `last_frame_index_count`
   - Text buffers: `last_frame_text_vertex_count`, `last_frame_text_index_count`
2. **Code cleanup** (`text/`): Removed vestigial `cosmic/mod.rs` module (actual integration is via `astra-gui-text` crate)

Performance impact:
- Reduced allocations per frame (especially for UIs with consistent size)
- Eliminated redundant measurement passes
- Cleaner, more maintainable API

#### Short-term optimizations (Dec 2025):
1. **Node fields privacy** (`node.rs`): Made all fields private, enforcing consistent builder pattern API
2. **Vertex color compression** (`vertex.rs`): Changed from `[f32; 4]` (16 bytes) to `[u8; 4]` (4 bytes) using Unorm8x4
   - 50% reduction in vertex buffer bandwidth
3. **Draw call batching** (`lib.rs`): Batch consecutive draws with same scissor rect
   - Reduces GPU overhead for clipped UIs
4. **Opacity optimization** (`output.rs`): Skip color mutations when opacity is 1.0
   - Avoids unnecessary work in common case

#### ✅ Medium-term optimizations (Dec 2025):
1. **Size::resolve() semantics fix** (`layout.rs`): 
   - Changed `resolve()` to panic on Fill/FitContent instead of misleading fallbacks
   - Added `try_resolve()` as non-panicking alternative that returns `Option<f32>`
   - Updated all callsites in `node.rs` to use `try_resolve()` with appropriate fallbacks
   - Enforces clearer semantics: resolve() only for Fixed/Relative, try_resolve() for all cases

### ✅ Completed: Interactive components system with disabled state and toggle (Dec 2025)

Implemented a complete declarative styling system for interactive components:

1. **Style System** (`style.rs`, `transition.rs`):
   - `Style` struct with optional visual properties (fill_color, text_color, opacity, offset, etc.)
   - Style merging for layered states (base → hover → active → disabled)
   - Easing functions (linear, ease-in, ease-out, ease-in-out, cubic variants)
   - `Transition` configuration with duration and easing
   - Style interpolation with `lerp_style()` for smooth animations
   - **Offset animation support**: offset_x and offset_y can be animated for smooth position transitions

2. **Node Integration** (`node.rs`):
   - Added `base_style`, `hover_style`, `active_style`, `disabled_style` fields
   - Added `disabled` boolean field
   - Builder methods: `with_style()`, `with_hover_style()`, `with_active_style()`, `with_disabled_style()`, `with_disabled()`
   - Getter methods for accessing styles and disabled state

3. **Interactive State Management** (`interactive_state.rs`):
   - `InteractionState` enum: Idle, Hovered, Active, Disabled
   - `InteractiveStateManager` tracks transition state per node ID across frames
   - Automatic style interpolation with configurable transitions
   - `apply_styles()` recursively applies computed styles to node tree
   - Disabled nodes always use disabled_style (or fallback with reduced opacity)

4. **Hit Testing** (`hit_test.rs`):
   - Modified `hit_test_recursive()` to skip disabled nodes
   - Disabled nodes don't receive interaction events
   - Children of disabled nodes can still be interactive (if not disabled themselves)

5. **Button Component** (`button.rs`):
   - Updated to accept `disabled` parameter
   - Automatically applies disabled_style when disabled
   - Uses declarative style system - no manual state tracking needed

6. **Toggle Component** (`toggle.rs`):
   - iOS-style toggle switch with smooth sliding knob animation
   - Knob position animates smoothly using offset_x/offset_y style properties
   - Background color smoothly transitions between on/off states
   - Visual feedback for hover and active states
   - Supports disabled state
   - Customizable styling with `ToggleStyle`

7. **Example** (`button.rs` example):
   - Demonstrates increment/decrement buttons with counter
   - Toggle switch to enable/disable counter buttons
   - Shows smooth transitions between all states including disabled
   - Label + toggle component demonstrates layout composition

Features:
- Declarative styling with automatic state transitions
- Smooth animations using easing functions
- No manual state tracking required
- Disabled state prevents all interaction
- Works with multiple interactive components independently
- Toggle component with iOS-style design

Remaining optimizations from plan (deferred to future):
- GPU compute tessellation (high effort, very high impact)
- Layout caching with dirty tracking (high effort, very high impact)

### ✅ Completed: Analytic Anti-Aliasing with SDF Rendering (Phase 1: Foundation - Dec 2025)

Implemented GPU-based analytic anti-aliasing using Signed Distance Fields (SDF) for all GUI rectangles:

1. **SDF Shader Implementation** (`ui_sdf.wgsl`):
   - Complete SDF functions for all 5 corner types:
     - `sd_box()`: Sharp 90° corners (trivial)
     - `sd_rounded_box()`: Circular arc corners (Inigo Quilez formula)
     - `sd_chamfer_box()`: 45° diagonal cut corners (medium complexity)
     - `sd_inverse_round_box()`: Concave circular corners (box minus circles)
     - `sd_squircle_box()`: Superellipse corners with power distance approximation
   - Fragment shader computes distance and applies `fwidth()` + `smoothstep()` for AA
   - Vertex shader transforms unit quad to screen-space rectangle per instance

2. **Instance Data Structure** (`instance.rs`):
   - `RectInstance`: 48-byte structure with shape parameters
   - Packs: center, half_size, colors (u8x4), stroke_width, corner_type, parameters
   - Implements `From<&StyledRect>` for easy conversion
   - Uses bytemuck traits for GPU upload

3. **Rendering Pipeline** (`lib.rs`):
   - Created SDF pipeline alongside existing tessellation pipeline
   - Unit quad buffers: 4 vertices, 6 indices (shared across all rectangles)
   - Instance buffer with dynamic resizing
   - All rectangles currently use SDF rendering (tessellation kept for future fallback)

Performance improvements:
- **89% vertex reduction**: 36+ vertices → 4 shared vertices per rounded rect
- **Memory savings**: 432 bytes → 80 bytes per rounded rect
- **Resolution-independent**: Perfect AA at any DPI/zoom level
- **Expected speedup**: 2-5x faster for typical UIs (vertex-bound workload)

### ✅ Completed: All Phases of Analytic Anti-Aliasing (Phase 1-4 - Dec 2025)

**Phase 1-3: All Corner Types** ✅
- ✅ None (sharp 90° corners)
- ✅ Round (circular arc corners)
- ✅ Cut (45° chamfered corners)
- ✅ InverseRound (concave circular corners)
- ✅ Squircle (superellipse corners)

**Phase 4: Stroke Support** ✅
- ✅ Analytic stroke rendering using SDF ring calculation (`abs(dist)` approach)
- ✅ Works for ALL corner types (no tessellation fallback needed)
- ✅ Consistent stroke-over-fill blending using `mix()`
- ✅ Perfect AA at all stroke widths (0.5px to 20px+)
- ✅ Comprehensive test examples: `stroke_test.rs`, `stroke_simple.rs`

Performance achieved:
- **89% vertex reduction**: 36+ vertices → 4 shared vertices per rounded rect
- **Memory savings**: 432 bytes → 80 bytes per rounded rect
- **Resolution-independent AA**: Perfect quality at any DPI/zoom

Next potential work (Phase 5):
- Text AA improvements (bilinear filtering, optional MSDF)
- Or move to other features

---

## What's missing / next work items

### 1) Overflow policy completeness across renderers

- Text: already respects `ClippedShape::clip_rect` via scissor ✅
- Geometry: needs per-clip batching/scissor (or separate clipped render pass per clip group) to fully respect `Overflow::Hidden`

### 2) Scroll (roadmap)

`Overflow::Scroll` is defined but not implemented:
- For now it behaves like `Hidden` (clip only)
- Future: add scroll offset state and adapt clip rect + transform

### 3) Text vertical placement refinement (if needed)

Next steps if issues arise:
- Confirm consistent coordinate convention between:
  - cosmic-text physical glyph output
  - swash placement + cache key integer offsets
  - renderer quad placement formula

### 4) Multi-line text / wrapping (future)

Current implementation:
- Single-line text only
- `FitContent` measures single line width/height

Future extensions:
- Multi-line shaping with word wrapping
- Measurement with max-width constraints

---

## Constraints / project rules to follow

- Use conventional commits when committing
- Run:
  - `cargo fmt` ✅
  - `cargo check` ✅
  - `cargo run` (so the result can be inspected) ✅
- Avoid warnings ✅
- Keep `astra-gui` minimal / backend-only rendering logic remains in backend crates ✅
- Update this `plan/context.md` regularly while implementing ✅
