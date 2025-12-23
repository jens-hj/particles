# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

1) Finish backend-agnostic text rendering (already wired; still needs vertical-placement refinement in `astra-gui-text` / renderer math).
2) Improve core layout ergonomics by introducing:
   - `Size::FitContent` (intrinsic sizing)
   - `Overflow` policy on nodes to control clipping vs visible overflow (scroll on roadmap)

---

## What exists right now

### `astra-gui` (core)

Text-related core types already exist:
- `Content::Text(TextContent)` in `crates/astra-gui/src/content.rs`
- `TextShape` and `Shape::Text(TextShape)` in `crates/astra-gui/src/primitives.rs`

Node tree emits text shapes:
- `Node::collect_shapes()` pushes `Shape::Text(TextShape::new(content_rect, text_content))` when `node.content` is `Content::Text(...)`.
- `content_rect` is computed from the node rect minus padding.

#### New: intrinsic sizing + overflow policy (core)

- Added `Size::FitContent` in `crates/astra-gui/src/layout.rs`.
  - This is intended to represent “minimum size that fits content (text metrics or children), plus padding”.
  - Current resolver fallback: `FitContent` resolves to `parent_size` until the layout pass is extended with intrinsic measurement.

- Added `Overflow` in `crates/astra-gui/src/layout.rs`:
  - `Overflow::Visible`
  - `Overflow::Hidden` (**default**)
  - `Overflow::Scroll` (not implemented yet; treated like `Hidden` for clipping purposes)

- Added `Node.overflow` and `Node::with_overflow(...)` in `crates/astra-gui/src/node.rs` (default: `Overflow::Hidden`).

#### New: clip rects derive from overflow policy

`FullOutput::from_node_with_debug()` now builds `ClippedShape`s by walking the node tree and computing `clip_rect` from the ancestor overflow chain:
- If any ancestor is `Hidden` (or `Scroll` for now), `clip_rect` is the intersection of those ancestor rects.
- `Visible` does not restrict the inherited clip rect.
- Nodes fully clipped out are skipped early.

Note: shapes still use `node_rect` for their “shape rect” (e.g. background), and text still uses `content_rect` for the text’s own bounding box; `clip_rect` is controlled separately by the overflow policy.

Tessellation remains geometry-only:
- `Shape::Rect` gets tessellated into triangles.
- `Shape::Text` is still intentionally skipped (backend-rendered).

### `astra-gui-wgpu` (backend)

- Geometry pipeline exists (`src/shaders/ui.wgsl`) and continues to work.
- Text pipeline exists and is wired:
  - `src/shaders/text.wgsl` samples an `R8Unorm` atlas and tints by vertex color.
  - CPU side generates per-glyph quads and uploads an `R8` glyph bitmap into the atlas.

Clipping behavior:
- Text draws with **per-shape scissor** using `ClippedShape::clip_rect`:
  - indices for each text shape are recorded as `(start..end)` ranges
  - `draw_indexed` is issued per-range with the correct scissor rect

Important limitation:
- Geometry rendering is still a single batched draw and does **not** currently honor per-shape `clip_rect`. If we want overflow clipping for rects as well, the geometry path will need per-clip batching similar to the text path.

Examples:
- `crates/astra-gui-wgpu/examples/overflow.rs` showcases `Overflow::{Hidden, Visible, Scroll}` behaviors.
  - Note: the demo focuses on TEXT overflow because text respects scissor clipping today.
  - `Overflow::Scroll` is a placeholder (clips only; no scroll offsets implemented yet).

Text rasterization status:
- WGPU backend uses `astra-gui-text` for shaping and rasterization (Inter via `astra-gui-fonts`).
- Vertical placement is still inconsistent for some sizes (often clipped upward so only bottoms of glyphs are visible).

---

## Recent progress

- Added core layout primitives:
  - `Size::FitContent`
  - `Overflow { Visible, Hidden (default), Scroll (roadmap) }`
  - `Node::with_overflow(...)`
  - `FullOutput` now derives `clip_rect` from overflow policy (intersection of ancestor clips)

- Added a WGPU example to exercise overflow policies:
  - `crates/astra-gui-wgpu/examples/overflow.rs`

- Existing text stack remains:
  - `crates/astra-gui-fonts`: embeds Inter; contains licenses
  - `crates/astra-gui-text`: cosmic-text shaping + swash rasterization outputting `R8` masks + bearing
  - `crates/astra-gui-wgpu`: atlas, uploads, text pipeline, scissor-based clipping for text

---

## What’s missing / next work items

### 1) Implement intrinsic measurement for `Size::FitContent` (layout)

Right now `FitContent` is a semantic placeholder. To make it real we need:
- Text measurement (at least line height; ideally width too) for `Content::Text`
- Children measurement aggregation for container nodes (layout-direction dependent; includes margins + gap collapsing + padding)

This likely means adding a measurement pass or extending `compute_layout` to compute intrinsic sizes before resolving.

### 2) Overflow policy completeness across renderers

- Text: already respects `ClippedShape::clip_rect` via scissor.
- Geometry: needs per-clip batching/scissor (or a separate clipped render pass per clip group) to fully respect `Overflow::Hidden`.

### 3) Scroll (roadmap)

`Overflow::Scroll` is defined but not implemented:
- For now it behaves like `Hidden` (clip only).
- Future: add scroll offset state and adapt clip rect + transform.

### 4) Fix text vertical placement (still highest priority for visuals)

Next steps:
- Confirm consistent coordinate convention between:
  - cosmic-text physical glyph output
  - swash placement + cache key integer offsets
  - renderer quad placement formula

---

## Constraints / project rules to follow

- Use conventional commits when committing.
- Run:
  - `cargo fmt`
  - `cargo check`
  - `cargo run` (so the result can be inspected)
- Avoid warnings.
- Keep `astra-gui` minimal / backend-only rendering logic remains in backend crates.
- Update this `plan/context.md` regularly while implementing.

