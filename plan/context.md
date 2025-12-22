# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

Continue **text node implementation** in `crates/astra-gui` and the **WGPU backend text rendering** in `crates/astra-gui-wgpu`, with emphasis on:
- rendering `Content::Text` nodes reliably
- then making clipping (`ClippedShape::clip_rect`) correct (scissor per clip rect)

Key goal: get `Content::Text` nodes to render text via the WGPU backend, while keeping `astra-gui` backend-agnostic and minimal.

---

## What exists right now

### `astra-gui` (core)

- Text-related core types already exist:
  - `Content::Text(TextContent)` in `crates/astra-gui/src/content.rs`
  - `TextShape` and `Shape::Text(TextShape)` in `crates/astra-gui/src/primitives.rs`
- Node tree emits text shapes:
  - `Node::collect_shapes()` pushes `Shape::Text(TextShape::new(content_rect, text_content))` when `node.content` is `Content::Text(...)`.
  - `content_rect` is computed from the node rect minus padding.
- `FullOutput::from_node_with_debug()` gathers shapes and wraps them into `ClippedShape`s.
  - For `Shape::Text`, it preserves the `TextShape` and provides a `clip_rect`.
- `Tessellator` is geometry-only:
  - `Shape::Rect` gets tessellated into triangles.
  - `Shape::Text` is intentionally skipped with a comment indicating backend-rendered text.

### `astra-gui-wgpu` (backend)

- Geometry pipeline exists (tessellated triangles via `src/shaders/ui.wgsl`) and continues to work.
- A first-cut text pipeline now exists and is wired:
  - `src/shaders/text.wgsl` samples an `R8Unorm` atlas and tints by vertex color.
  - CPU side generates per-glyph quads and uploads an `R8` glyph bitmap into the atlas.
  - Currently uses a tiny built-in `debug_font` as a temporary rasterizer (not cosmic-text raster yet).

Important behavior / limitation:
- The text renderer currently does **one batched draw call** for all text quads.
- Because scissor/clipping is render-pass state, a single draw can only have one scissor rect.
- We recently hit a bug where only the footer text was visible due to scissor state issues; as a quick correctness fix, the renderer now draws batched text with a **full-screen scissor** so all text appears.
- Consequence: per-container clipping is currently **not implemented** for text. The red “clipping demo” panel won’t clip text yet.

Examples:
- `examples/text.rs` builds a UI showcasing:
  - header title/subtitle
  - alignment grid
  - varying font sizes
  - a long-line “clipping candidate”
  - footer keybind hint
- `examples/layout_nodes.rs` / `examples/corner_shapes.rs` remain node-based layout demos with debug overlays.

---

## Recent progress

- Fixed "only footer text renders" issue in `examples/text.rs`:
  - Root cause: render pass scissor state was not compatible with a single batched text draw.
  - Quick fix: set full-screen scissor for the text draw so all text is visible.
- Verified:
  - `cargo fmt`
  - `cargo check`
  - `cargo run -p astra-gui-wgpu --example text` (visual inspection: text now renders across the UI)

---

## What’s missing / next work items

### 1) Correct clipping for text (highest priority)

Implement clipping using `ClippedShape::clip_rect` in the WGPU backend:

- Batch text quads by clip rect (scissor):
  - Option A (simple, correct): one draw per `Shape::Text` (per clip rect)
  - Option B (better): coalesce consecutive shapes with identical clip rects into fewer draw calls
- Implementation approach:
  - While building the text buffers, record draw ranges `(start_index..end_index)` per clip batch.
  - After uploading the buffers once, issue `draw_indexed` per range with `set_scissor_rect` set appropriately.
- Ensure scissor conversion:
  - clamp to framebuffer bounds
  - avoid zero-sized scissors

### 2) Replace debug font with `cosmic-text` rasterization (next)

- Use `cosmic-text` for shaping + rasterization to glyph bitmaps
- Add stable glyph cache keys that include font identity, glyph id, size, and subpixel positioning if needed
- Address `queue.write_texture` row padding as needed (some platforms may require 256-byte aligned `bytes_per_row`)

---

## Constraints / project rules to follow

- Use conventional commits when committing.
- Run:
  - `cargo fmt`
  - `cargo check`
  - `cargo run` (so the result can be inspected)
- Avoid warnings.
- Keep `astra-gui` minimal / backend-only text logic remains in `astra-gui-wgpu`.
- Update this `plan/context.md` regularly while implementing.

---

## Immediate next steps (recommended order)

1. Commit the current “text scissor correctness fix” (text now visible everywhere).
2. Implement proper text clipping by batching by `clip_rect` with multiple draw calls.
3. Run:
   - `cargo fmt`
   - `cargo check`
   - `cargo run -p astra-gui-wgpu --example text`
4. Re-check diagnostics and update this file with the final clipping behavior and any remaining known issues.

---

## Known unknowns / items to confirm while implementing

- `cosmic-text 0.12.x` raster API details for producing glyph bitmaps.
- Atlas growth + eviction strategy (current atlas is simple/grow-only).
- `queue.write_texture` alignment requirements across backends for small glyph uploads.