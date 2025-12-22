# Project context / working state

This file tracks the current working state so you (or another AI) can pick up the work quickly.

## Current focus

Finish migrating text rendering to backend-agnostic crates and remove remaining backend-only fallbacks:

- `crates/astra-gui-fonts`: bundled fonts (Inter variable fonts; JetBrains Mono present for later)
- `crates/astra-gui-text`: backend-agnostic shaping+raster API (cosmic-text implementation)
- `crates/astra-gui-wgpu`: WGPU-only parts (atlas texture/upload, pipelines, buffers, scissor/draws)

Key goal: render `Content::Text` nodes via a real font (Inter) through `astra-gui-text`, keeping `astra-gui` core backend-agnostic and minimal.

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
- The WGPU backend is now wired to use `astra-gui-text` for shaping and rasterization (Inter via `astra-gui-fonts`).
- The old `debug_font` fallback has been removed from the backend.
- Current state is WIP: text renders as real glyphs in some cases, but vertical placement is still incorrect for many sizes (often clipped upward so only bottoms of glyphs are visible).

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
    - Cosmic-text-backed engine that loads Inter into a `FontSystem`
    - Rasterization implemented via cosmic-text `SwashCache` (produces `R8` mask bitmaps + bearing)
    - Shaping updated to use cosmic-text “physical glyph” positioning to better match swash raster placement (WIP; still has vertical offset issues)
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

### 1) Fix text vertical placement (highest priority)

`astra-gui-wgpu` is now using `astra-gui-text` for shaping + rasterization (Inter). Remaining blocker is incorrect vertical placement causing many labels to be clipped upward.

Next steps:
- Confirm consistent coordinate convention between:
  - cosmic-text physical glyph output (`LayoutGlyph::physical`)
  - swash placement (`SwashImage::placement.{left,top}`) and the cache key’s integer offsets
  - the renderer’s quad placement formula (`origin + glyph_pos + bearing`)
- Adjust `astra-gui-text` to emit glyph positions in the exact space expected by the raster bearing, so no per-font-size hacks are needed.

### 2) Stabilize glyph identity + caching (next)

- Make `FontId` map deterministically to a specific `fontdb::ID` (currently best-effort “find Inter” lookup).
- Make cache keys stable and correct:
  - include subpixel binning if needed (currently not surfaced in `GlyphKey`)
  - ensure `px_size` matches what cosmic-text uses for rasterization/hinting
- Make atlas uploads robust:
  - handle `queue.write_texture` row padding requirements (256-byte alignment) if/when it fails on stricter backends.

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

1. Fix text vertical placement so all labels render without being clipped (no hardcoded per-size shifts).
2. Verify clipping/scissoring still works per `ClippedShape::clip_rect` after placement fixes.
3. Run:
   - `cargo fmt`
   - `cargo check`
   - `cargo run -p astra-gui-wgpu --example text`
4. Re-check diagnostics and update this file with outcomes + any remaining known issues.

---

## Known unknowns / items to confirm while implementing

- `cosmic-text 0.12.x` raster API details for producing glyph bitmaps.
- Atlas growth + eviction strategy (current atlas is simple/grow-only).
- `queue.write_texture` alignment requirements across backends for small glyph uploads.
