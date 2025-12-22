//! Minimal text shaping/raster entrypoints for the `astra-gui-wgpu` backend.
//!
//! This entire module is only relevant when the `text-cosmic` feature is enabled.
//! When the feature is disabled, this module is not compiled at all, and the backend
//! remains geometry-only.

#![cfg(feature = "text-cosmic")]

use crate::text::atlas::GlyphKey;

/// A request to shape and rasterize text for a single `TextShape`.
#[derive(Clone, Debug)]
pub struct TextRequest<'a> {
    /// UTF-8 text.
    pub text: &'a str,
    /// Font size in pixels.
    pub font_px: f32,
    /// Optional font family/name (backend-defined meaning).
    pub font_family: Option<&'a str>,
}

/// The output of shaping: a set of positioned glyphs in pixel space.
///
/// Coordinates are typically relative to the top-left of the text’s layout rect,
/// but the caller decides the final placement/alignment (astra-gui already stores alignment).
#[derive(Clone, Debug, Default)]
pub struct ShapedLine {
    pub glyphs: Vec<PositionedGlyph>,
    /// Total advance width in pixels for the shaped line.
    pub width_px: f32,
    /// Line height in pixels.
    pub height_px: f32,
    /// Baseline offset from the top in pixels (top -> baseline).
    pub baseline_px: f32,
}

/// A single shaped glyph with a position.
///
/// `x_px`/`y_px` are where this glyph should be drawn relative to the line origin.
#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub key: GlyphKey,
    pub x_px: f32,
    pub y_px: f32,
}

/// A rasterized glyph bitmap suitable for uploading to an R8 atlas texture.
#[derive(Clone, Debug)]
pub struct GlyphBitmap {
    pub key: GlyphKey,
    /// Width/height in pixels.
    pub size_px: [u32; 2],
    /// Pixel data, row-major, one byte per pixel (coverage 0..255).
    pub pixels: Vec<u8>,
    /// Suggested top-left bearing offset from the glyph origin (in pixels).
    ///
    /// The renderer uses this to place the bitmap quad relative to `PositionedGlyph (x,y)`.
    pub bearing_px: [i32; 2],
    /// Suggested advance in pixels (x advance, y advance).
    pub advance_px: [f32; 2],
}

/// A minimal facade around cosmic-text’s shaping plumbing.
///
/// This owns the cosmic systems so the renderer can reuse caches between frames.
pub struct CosmicText {
    font_system: cosmic_text::FontSystem,
    _swash_cache: cosmic_text::SwashCache,
}

impl CosmicText {
    /// Create a new CosmicText context using cosmic-text defaults.
    pub fn new() -> Self {
        Self {
            font_system: cosmic_text::FontSystem::new(),
            _swash_cache: cosmic_text::SwashCache::new(),
        }
    }

    /// Access the underlying `FontSystem` (e.g. to load fonts / set locale).
    pub fn font_system_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.font_system
    }

    /// Shape a single line of text.
    ///
    /// Current limitations:
    /// - no wrapping
    /// - no multi-line layout
    /// - direction/locale are cosmic defaults
    ///
    /// The returned glyph positions are relative to a (0,0) origin.
    pub fn shape_line(&mut self, req: TextRequest<'_>) -> ShapedLine {
        use cosmic_text::{Attrs, Buffer, Metrics, Shaping};

        let metrics = Metrics::new(req.font_px, req.font_px * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Prevent wrapping by setting huge size; cosmic-text expects `Option<f32>`.
        buffer.set_size(
            &mut self.font_system,
            Some(f32::MAX),
            Some(metrics.line_height),
        );

        let mut attrs = Attrs::new();
        if let Some(family) = req.font_family {
            attrs = attrs.family(cosmic_text::Family::Name(family));
        }

        buffer.set_text(&mut self.font_system, req.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Metrics doesn't expose ascent in 0.12.x; we keep baseline at 0 for now.
        let mut out = ShapedLine {
            glyphs: Vec::new(),
            width_px: 0.0,
            height_px: metrics.line_height,
            baseline_px: 0.0,
        };

        for run in buffer.layout_runs() {
            // `line_w` is the width of the run in px.
            out.width_px = out.width_px.max(run.line_w);

            for glyph in run.glyphs.iter() {
                // Keep glyph keys conservative; we don't have stable font IDs from LayoutRun
                // in cosmic-text 0.12. Use 0 for now and rely on glyph_id + size for caching.
                let font_id = 0_u64;
                let glyph_id = glyph.glyph_id as u32;

                let key = GlyphKey::new(font_id, glyph_id, req.font_px.round().max(1.0) as u16, 0);

                out.glyphs.push(PositionedGlyph {
                    key,
                    x_px: glyph.x,
                    y_px: glyph.y,
                });
            }
        }

        out
    }

    /// Rasterize a glyph coverage mask (R8) for uploading into an atlas.
    ///
    /// This is intentionally stubbed out until we lock down the cosmic-text 0.12.x
    /// raster API usage (fontdb IDs, cache keys, swash integration).
    pub fn rasterize_glyph(&mut self, _key: GlyphKey) -> Option<GlyphBitmap> {
        None
    }
}

impl Default for CosmicText {
    fn default() -> Self {
        Self::new()
    }
}
