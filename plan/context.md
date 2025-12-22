# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

Move text shaping / rasterization and bundled fonts into backend-agnostic crates, then wire `astra-gui-wgpu` to use them:

- New crate: `crates/astra-gui-fonts` (bundled fonts; Inter for now)
- New crate: `crates/astra-gui-text` (backend-agnostic shaping+raster API, cosmic-text implementation)
- Backend (`astra-gui-wgpu`) should keep only WGPU specifics: atlas texture/upload, pipelines, buffers, scissor/draws.

Key goal: get `Content::Text` nodes to render via a real font (Inter) and a backend-agnostic text engine, while keeping `astra-gui` core backend-agnostic and minimal.

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
- Text pipeline exists and is wired:
  - `src/shaders/text.wgsl` samples an `R8Unorm` atlas and tints by vertex color.
  - CPU side generates per-glyph quads and uploads an `R8` glyph bitmap into the atlas.

Clipping behavior:
- Text now draws with **per-shape scissor** using `ClippedShape::clip_rect`:
  - indices for each text shape are recorded as `(start..end)` ranges
  - `draw_indexed` is issued per-range with the correct scissor rect
- This fixes the earlier regression where only the footer text was visible and enables the “clipping demo” panel to clip long text.

Text rasterization status:
- The backend still contains a temporary `debug_font` fallback path.
- Next step is to switch glyph shaping/raster to the new backend-agnostic `astra-gui-text` crate and remove the fallback.

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

- Added backend-agnostic crates:
  - `crates/astra-gui-fonts`
    - Embeds Inter variable fonts (roman + italic) via `include_bytes!`
    - Includes Inter OFL text via `include_str!`
  - `crates/astra-gui-text`
    - Defines backend-agnostic API (`TextEngine`, `GlyphKey`, `GlyphBitmap`, `shape_line` outputs)
    - Cosmic-text-backed engine stub that loads Inter into a `FontSystem` and shapes a single line
    - Rasterization is still TODO (bitmap generation not yet implemented)
- Added font assets to the repo:
  - `assets/fonts/inter/Inter-VariableFont_opsz,wght.ttf`
  - `assets/fonts/inter/Inter-Italic-VariableFont_opsz,wght.ttf`
  - `assets/fonts/inter/OFL.txt`
  - (JetBrains Mono variable fonts were also added under `assets/fonts/jetbrainsmono/` for later)
- Text clipping is now correct in `astra-gui-wgpu` using per-shape scissor draws.
- Verified:
  - `cargo fmt`
  - `cargo check`
  - `cargo run -p astra-gui-wgpu --example text` (visual inspection)

---

## What’s missing / next work items

### 1) Switch text rendering to backend-agnostic `astra-gui-text` (highest priority)

- Wire `astra-gui-wgpu` to use `astra-gui-text` for shaping + rasterization:
  - `shape_line` for glyph positioning + line metrics
  - `rasterize_glyph` for `R8` glyph bitmaps
- Remove the `debug_font` fallback path once `rasterize_glyph` is implemented.

### 2) Implement cosmic-text glyph rasterization in `astra-gui-text`

- Implement `TextEngine::rasterize_glyph` for the cosmic engine:
  - stable mapping from `FontId` to a real font face in `FontSystem` / fontdb
  - choose opsz/wght defaults for Inter variable font (e.g. opsz ~= font_px, wght=400)
  - produce `GlyphBitmap` with coverage + bearing + advance
- Ensure cache keys are stable:
  - include `font_id`, `glyph_id`, `px_size`, and optional subpixel position
- Make atlas uploads robust:
  - handle `queue.write_texture` alignment requirements (256-byte row padding) if needed.

### 3) Font selection (later)

- Keep Inter as default.
- Later: add JetBrains Mono support via `astra-gui-fonts` and a `TextContent`/engine-level way to pick families/styles.

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