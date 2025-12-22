# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

Continue **text node implementation** in `crates/astra-gui` and add a **text example** in `crates/astra-gui-wgpu` demonstrating actual text nodes (and later, actual text rendering).

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
  - For `Shape::Text`, it currently preserves the `TextShape` as-is; clip rect is the node rect.
- `Tessellator` is geometry-only:
  - `Shape::Rect` gets tessellated into triangles.
  - `Shape::Text` is intentionally skipped with a comment indicating backend-rendered text.
- Small recent edit: clarified in `crates/astra-gui/src/tessellate.rs` that text stays backend-only (no UV quad emitted).

### `astra-gui-wgpu` (backend)

- Renderer currently only draws tessellated geometry:
  - Pipeline: single-color vertex shader + fragment shader in `src/shaders/ui.wgsl`.
  - Vertex data carries `pos` + `color` only.
  - `Renderer::render()` tessellates using `astra_gui::Tessellator` and draws the resulting mesh.
- `cosmic-text = "0.12"` is present as an optional dependency (feature work-in-progress), but **text rendering is not wired up** yet.

Backend text scaffolding exists (not yet wired into rendering):
- `crates/astra-gui-wgpu/src/text/` (atlas packer, text vertex, cosmic shaping stub)
- `crates/astra-gui-wgpu/src/shaders/text.wgsl` (alpha mask atlas sampling + tint shader)

Examples:
- `examples/layout_nodes.rs` includes debug keybinds (D/M/P/B/C) and debug visualization support.
- `examples/text.rs` exists and includes debug keybinds (D/M/P/B/C). It builds text nodes, but you’ll only see rectangles until `Shape::Text` rendering is implemented.
- `examples/corner_shapes.rs` has been updated to use the **Node layout system** (not raw `Shape`s), so debug overlays now apply there too.

---

## Recent progress

- Converted `crates/astra-gui-wgpu/examples/corner_shapes.rs` from a raw shape-based demo to a Node-based layout demo:
  - Uses `Node` rows/columns with `Size::Fill` and gaps/padding
  - Builds the same corner shape showcase via node `Shape::Rect(...)`
  - Debug overlays (margins/padding/borders/content) now work there via `FullOutput::from_node_with_debug(...)`
- Kept debug keybind behavior consistent across examples (D/M/P/B/C, Esc to quit).

---

## What’s missing / next work items

### 1) WGPU text rendering path (backend responsibility)

Implement rendering of `Shape::Text(TextShape)` in `astra-gui-wgpu`:

- Add a **glyph atlas texture** + **sampler** + **text render pipeline** that samples the atlas and tints by text color.
- Use `cosmic-text` to shape and rasterize text into the atlas.
- Produce per-glyph quads (pos + UV) into a vertex buffer (or instance buffer).
- Respect alignment fields:
  - `TextShape::{h_align, v_align}`
  - Place the shaped line(s) within `TextShape::rect` accordingly.
- Respect clipping (`ClippedShape::clip_rect`):
  - Preferred: use `scissor_rect` per shape (convert to integer pixels and clamp to framebuffer bounds).
- Handle DPI properly:
  - `winit` scale factor -> choose a consistent convention (physical pixels recommended).

### 2) Keep `astra-gui` minimal / crate decomposition guidance

- `astra-gui` should not gain rendering logic or backend-specific text shaping.
- If text rendering code becomes non-trivial, consider a dedicated backend-side crate, but avoid bloating the main crates.

---

## Constraints / project rules to follow

- Use conventional commits when you commit.
- Keep the main crate minimal and push real logic into appropriate crates.
- Run:
  - `cargo fmt`
  - `cargo check`
  - `cargo run` (so the result can be inspected)
- Avoid warnings (use `_unused` patterns etc. when needed).
- Prefer performant approaches (GPU where it makes sense; for text, atlas + quads is expected).
- Update this `plan/context.md` regularly while implementing.

---

## Immediate next steps (recommended order)

1. Implement the text pipeline in `astra-gui-wgpu`:
   - ensure `text.wgsl` is compiled/used
   - create atlas texture + sampler bind group
   - introduce a text vertex buffer format with UVs
2. Extend `Renderer::render()` to:
   - draw geometry as today
   - draw `Shape::Text` via the new pipeline (with scissor per `ClippedShape`)
3. Run `cargo fmt`, `cargo check`, and `cargo run -p astra-gui-wgpu --example text`.
4. Re-check diagnostics and update this file with outcomes and any known issues.

---

## Known unknowns / items to confirm while implementing

- Exact approach to glyph rasterization with `cosmic-text 0.12.x` (API details; likely needs a dedicated integration pass).
- How to manage atlas growth + eviction strategy (start simple: grow-only atlas).
- Whether to batch text across shapes into one draw call vs per-shape (start simple; optimize later).
- Proper scissor rectangle calculation when nodes extend outside window.