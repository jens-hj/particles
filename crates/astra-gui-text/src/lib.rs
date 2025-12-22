//! Backend-agnostic text shaping + glyph rasterization for `astra-gui`.
//!
//! # Design goals
//! - **Backend-agnostic**: no `wgpu`, no renderer-specific types.
//! - **Practical**: provides CPU-side glyph bitmaps and positioned glyph runs.
//! - **Cache-friendly**: stable-ish `GlyphKey` so renderers can atlas/cache glyphs.
//!
//! # Current implementation
//! The `cosmic` feature provides an implementation using `cosmic-text`.
//! Fonts are provided by `astra-gui-fonts` (Inter by default).
//!
//! Renderers are expected to:
//! 1. Call [`TextEngine::shape_line`] (or later multi-line APIs) for each `TextShape`.
//! 2. Ensure glyphs are present in their atlas by calling [`TextEngine::rasterize_glyph`].
//! 3. Build quads from [`PositionedGlyph`] + atlas UVs and apply clipping via scissor.
//!
//! NOTE: This crate intentionally does not manage an atlas; that is backend-specific.

#![deny(warnings)]

use astra_gui::{HorizontalAlign, Rect, VerticalAlign};

/// A stable identifier for a font face known to the text engine.
///
/// This is intentionally opaque to the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub u64);

/// A stable-ish key for caching a glyph bitmap.
///
/// Renderers can use this as a cache key + atlas lookup.
///
/// Notes:
/// - `font_id` is engine owned.
/// - `glyph_id` is the font-specific glyph index (not Unicode scalar value).
/// - `px_size` is the requested font size in pixels (rounded).
/// - `subpixel_x_64` allows subpixel positioning when/if we choose to support it.
///   For now we default to 0 in the cosmic implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: FontId,
    pub glyph_id: u32,
    pub px_size: u16,
    pub subpixel_x_64: i16,
}

impl GlyphKey {
    pub fn new(font_id: FontId, glyph_id: u32, px_size: u16, subpixel_x_64: i16) -> Self {
        Self {
            font_id,
            glyph_id,
            px_size,
            subpixel_x_64,
        }
    }
}

/// CPU-side glyph coverage bitmap suitable for uploading into an `R8Unorm` atlas.
#[derive(Clone, Debug)]
pub struct GlyphBitmap {
    pub key: GlyphKey,
    /// Bitmap dimensions in pixels: (width, height).
    pub size_px: [u32; 2],
    /// Glyph bearing in pixels (left, top) relative to the pen position.
    ///
    /// Coordinate convention:
    /// - x grows right
    /// - y grows down
    pub bearing_px: [i32; 2],
    /// Glyph advance in pixels to apply to the pen after drawing.
    pub advance_px: [f32; 2],
    /// Row-major coverage bytes (0..=255), length = `width * height`.
    pub pixels: Vec<u8>,
}

/// A shaped glyph positioned in pixel space relative to a line origin.
///
/// Renderers typically use:
/// - `x_px`, `y_px` + bitmap bearing to compute the quad in screen space
/// - `key` to obtain atlas UVs for the glyph coverage bitmap
#[derive(Clone, Copy, Debug)]
pub struct PositionedGlyph {
    pub key: GlyphKey,
    pub x_px: f32,
    pub y_px: f32,
}

/// Metric information for placing a shaped line inside a rectangular layout box.
#[derive(Clone, Copy, Debug, Default)]
pub struct LineMetrics {
    pub width_px: f32,
    pub height_px: f32,
    /// Baseline offset from top of line box, in pixels.
    pub baseline_px: f32,
}

/// A shaped single-line run.
#[derive(Clone, Debug, Default)]
pub struct ShapedLine {
    pub glyphs: Vec<PositionedGlyph>,
    pub metrics: LineMetrics,
}

/// Input describing a single-line shaping request.
///
/// This intentionally stays close to `astra-gui`'s current `TextShape` information.
/// Multi-line/wrapping will likely need a separate API later.
#[derive(Clone, Debug)]
pub struct ShapeLineRequest<'a> {
    pub text: &'a str,
    /// Layout box (content rect) to align within.
    pub rect: Rect,
    pub font_px: f32,
    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
    /// Optional family name (engine-defined meaning). For now, Inter is used by default.
    pub family: Option<&'a str>,
}

/// Output describing how to place a line in a rectangle.
///
/// `origin_px` is the top-left of the shaped line box in screen space.
/// Renderers add each glyph's `x_px/y_px` to `origin_px`.
#[derive(Clone, Copy, Debug, Default)]
pub struct LinePlacement {
    pub origin_px: [f32; 2],
}

/// A backend-agnostic text engine.
///
/// This provides shaping and rasterization. Renderers are expected to do caching and atlas uploads.
pub trait TextEngine {
    /// Shape a single line and compute its placement within `req.rect` according to alignment.
    fn shape_line(&mut self, req: ShapeLineRequest<'_>) -> (ShapedLine, LinePlacement);

    /// Rasterize the glyph bitmap for the given `key`, if available.
    ///
    /// Engines may internally cache bitmaps; callers should still keep an atlas cache.
    fn rasterize_glyph(&mut self, key: GlyphKey) -> Option<GlyphBitmap>;
}

/// A convenient concrete engine selection.
///
/// Currently only cosmic-text is supported.
pub enum Engine {
    #[cfg(feature = "cosmic")]
    Cosmic(cosmic::CosmicEngine),
}

impl Engine {
    /// Create a default engine.
    ///
    /// With the `cosmic` feature enabled, this loads Inter (variable) from `astra-gui-fonts`.
    #[cfg(feature = "cosmic")]
    pub fn new_default() -> Self {
        Self::Cosmic(cosmic::CosmicEngine::new_default())
    }
}

impl TextEngine for Engine {
    fn shape_line(&mut self, req: ShapeLineRequest<'_>) -> (ShapedLine, LinePlacement) {
        match self {
            #[cfg(feature = "cosmic")]
            Self::Cosmic(engine) => engine.shape_line(req),
        }
    }

    fn rasterize_glyph(&mut self, key: GlyphKey) -> Option<GlyphBitmap> {
        match self {
            #[cfg(feature = "cosmic")]
            Self::Cosmic(engine) => engine.rasterize_glyph(key),
        }
    }
}

/// Helper: compute alignment origin for a line box within a rect.
fn align_origin(
    rect: Rect,
    line_w: f32,
    line_h: f32,
    h: HorizontalAlign,
    v: VerticalAlign,
) -> [f32; 2] {
    let x = match h {
        HorizontalAlign::Left => rect.min[0],
        HorizontalAlign::Center => rect.min[0] + (rect.width() - line_w) * 0.5,
        HorizontalAlign::Right => rect.max[0] - line_w,
    };

    let y = match v {
        VerticalAlign::Top => rect.min[1],
        VerticalAlign::Center => rect.min[1] + (rect.height() - line_h) * 0.5,
        VerticalAlign::Bottom => rect.max[1] - line_h,
    };

    [x, y]
}

#[cfg(feature = "cosmic")]
pub mod cosmic {
    //! `cosmic-text` implementation of shaping and glyph rasterization.
    //!
    //! This is intentionally conservative and focused on:
    //! - shaping a single line (no wrapping yet)
    //! - rasterizing glyph coverage masks suitable for an `R8` atlas
    //!
    //! As this stabilizes, we can extend to multi-line shaping, wrapping, and richer font selection.

    use super::{
        align_origin, FontId, GlyphBitmap, GlyphKey, LineMetrics, LinePlacement, PositionedGlyph,
        ShapeLineRequest, ShapedLine, TextEngine,
    };

    use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

    /// Concrete engine backed by `cosmic-text`.
    pub struct CosmicEngine {
        font_system: FontSystem,
        // In a more complete implementation we would keep:
        // - a font database handle / mapping from family to `FontId`
        // - a raster cache (swash cache) to avoid re-rasterizing
        //
        // For now, we keep it minimal and defer caching to later work.
        default_font_id: FontId,
    }

    impl CosmicEngine {
        /// Create the engine and load default Inter fonts.
        pub fn new_default() -> Self {
            let mut font_system = FontSystem::new();

            // Load Inter from astra-gui-fonts (bundled bytes).
            // We currently include both roman and italic to allow future style selection.
            //
            // NOTE: `FontSystem` API accepts raw font bytes; cosmic-text parses and stores them.
            // We intentionally do not pin an external file path.
            let _ = font_system
                .db_mut()
                .load_font_data(astra_gui_fonts::inter::variable_opsz_wght().to_vec());
            let _ = font_system
                .db_mut()
                .load_font_data(astra_gui_fonts::inter::italic_variable_opsz_wght().to_vec());

            // We don't currently have a stable font ID from cosmic-text that we can expose.
            // Use a constant engine-local ID for now; callers should treat it as opaque.
            let default_font_id = FontId(0);

            Self {
                font_system,
                default_font_id,
            }
        }

        /// Access the underlying `FontSystem` if callers want to customize further.
        pub fn font_system_mut(&mut self) -> &mut FontSystem {
            &mut self.font_system
        }

        fn make_attrs(&self, req: &ShapeLineRequest<'_>) -> Attrs<'static> {
            // `Attrs` holds references internally, so returning `Attrs<'static>` must not borrow
            // from `req`. We keep this backend-agnostic and simple for now:
            // - If a caller specifies a family, we ignore it until we introduce a stable owned
            //   font selection mechanism in this crate.
            // - Default to Inter by name.
            let attrs = Attrs::new().family(cosmic_text::Family::Name("Inter"));
            let _req = req;
            attrs
        }
    }

    impl TextEngine for CosmicEngine {
        fn shape_line(&mut self, req: ShapeLineRequest<'_>) -> (ShapedLine, LinePlacement) {
            let metrics = Metrics::new(req.font_px, req.font_px * 1.2);
            let mut buffer = Buffer::new(&mut self.font_system, metrics);

            // Prevent wrapping: set a huge width and line height from metrics.
            buffer.set_size(
                &mut self.font_system,
                Some(f32::MAX),
                Some(metrics.line_height),
            );

            let attrs = self.make_attrs(&req);

            buffer.set_text(
                &mut self.font_system,
                req.text,
                &attrs,
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            let mut out = ShapedLine {
                glyphs: Vec::new(),
                metrics: LineMetrics {
                    width_px: 0.0,
                    height_px: metrics.line_height,
                    baseline_px: 0.0,
                },
            };

            // `layout_runs()` may yield multiple runs even for one line; we treat them as one line.
            for run in buffer.layout_runs() {
                out.metrics.width_px = out.metrics.width_px.max(run.line_w);

                for glyph in run.glyphs.iter() {
                    // `glyph_id` is the font glyph index.
                    // `font_id` is currently a placeholder until we define a stable font identity mapping.
                    let key = GlyphKey::new(
                        self.default_font_id,
                        glyph.glyph_id as u32,
                        req.font_px.round().max(1.0) as u16,
                        0,
                    );
                    out.glyphs.push(PositionedGlyph {
                        key,
                        x_px: glyph.x,
                        y_px: glyph.y,
                    });
                }
            }

            let origin_px = align_origin(
                req.rect,
                out.metrics.width_px,
                out.metrics.height_px,
                req.h_align,
                req.v_align,
            );

            (out, LinePlacement { origin_px })
        }

        fn rasterize_glyph(&mut self, _key: GlyphKey) -> Option<GlyphBitmap> {
            // TODO: Implement using cosmic-text + swash cache.
            //
            // This requires:
            // - stable mapping from our `FontId` to a cosmic font face in the DB
            // - selecting size and potentially variations (opsz/wght/slnt)
            // - producing an R8 coverage mask + bearing/advance
            //
            // For now, leave as unimplemented; the WGPU backend currently still uses its fallback.
            None
        }
    }
}
