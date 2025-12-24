# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

1) ✅ Backend-agnostic text rendering with intrinsic measurement
2) ✅ Core layout ergonomics with `Size::FitContent` and `Overflow` policy
3) ✅ Performance optimizations and API consistency improvements (Dec 2025)
4) ✅ Interactive components system with declarative styles and smooth transitions (Dec 2025)
5) Next: Advanced optimizations (GPU compute tessellation, layout caching) as needed

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

- Geometry pipeline exists (`src/shaders/ui.wgsl`) and continues to work
- Text pipeline exists and is wired:
  - `src/shaders/text.wgsl` samples an `R8Unorm` atlas and tints by vertex color
  - CPU side generates per-glyph quads and uploads `R8` glyph bitmap into atlas

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

Remaining optimizations from plan (deferred to future):
- GPU compute tessellation (high effort, very high impact)
- Layout caching with dirty tracking (high effort, very high impact)

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
