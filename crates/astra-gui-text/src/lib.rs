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

use astra_gui::{
    ContentMeasurer, HorizontalAlign, IntrinsicSize, MeasureTextRequest, Rect, VerticalAlign,
};

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

impl ContentMeasurer for Engine {
    fn measure_text(&mut self, request: MeasureTextRequest<'_>) -> IntrinsicSize {
        match self {
            #[cfg(feature = "cosmic")]
            Self::Cosmic(engine) => engine.measure_text(request),
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

    use astra_gui::{ContentMeasurer, IntrinsicSize, MeasureTextRequest, Rect};
    use cosmic_text::{fontdb, Attrs, Buffer, FontSystem, Metrics, Shaping};

    /// Concrete engine backed by `cosmic-text`.
    pub struct CosmicEngine {
        font_system: FontSystem,

        // Raster cache for swash (used by cosmic-text under the hood).
        swash_cache: cosmic_text::SwashCache,

        // NOTE: For now we treat font identity as a single default font. We'll expand this to
        // multiple faces/families later once we define a stable mapping from family/style -> fontdb::ID.
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
            font_system
                .db_mut()
                .load_font_data(astra_gui_fonts::inter::variable_opsz_wght().to_vec());
            font_system
                .db_mut()
                .load_font_data(astra_gui_fonts::inter::italic_variable_opsz_wght().to_vec());

            // We don't currently have a stable font ID mapping surfaced in this crate.
            // Use a constant engine-local ID for now; callers should treat it as opaque.
            let default_font_id = FontId(0);

            Self {
                font_system,
                swash_cache: cosmic_text::SwashCache::new(),
                default_font_id,
            }
        }

        /// Access the underlying `FontSystem` if callers want to customize further.
        pub fn font_system_mut(&mut self) -> &mut FontSystem {
            &mut self.font_system
        }

        fn make_attrs(&self, req: &ShapeLineRequest<'_>) -> Attrs<'static> {
            // `Attrs` holds references internally, so returning `Attrs<'static>` must not borrow
            // from `req`. Keep this simple for now:
            // - ignore caller-supplied family until we define an owned/stable font selection API
            // - default to Inter by family name
            let attrs = Attrs::new().family(cosmic_text::Family::Name("Inter"));
            let _req = req;
            attrs
        }

        fn find_fontdb_id_for_default(&mut self) -> Option<fontdb::ID> {
            // Prefer Inter by family name; fall back to any available face.
            let db = self.font_system.db();
            let mut out = None;

            db.faces().for_each(|face| {
                if out.is_some() {
                    return;
                }

                let family_name = face
                    .families
                    .first()
                    .map(|f| f.0.as_str())
                    .unwrap_or_default();

                if family_name.eq_ignore_ascii_case("Inter") {
                    out = Some(face.id);
                }
            });

            out.or_else(|| db.faces().next().map(|face| face.id))
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
            //
            // IMPORTANT:
            // We must use cosmic-text's *physical* positioning to correctly align glyph bitmaps.
            // `LayoutGlyph::{x,y}` are hitbox offsets, and the bitmap placement returned from swash
            // is computed relative to a `CacheKey` that includes subpixel bins and hinting decisions.
            //
            // `LayoutGlyph::physical(offset, scale)` returns:
            // - `cache_key`: the exact cache key to rasterize with `SwashCache`
            // - `x`,`y`: integer placement offsets that must be applied to the bitmap placement
            //
            // We emit `PositionedGlyph::{x_px,y_px}` in LINE-TOP-LEFT space:
            // - x/y are relative to the top-left of the line box (not baseline)
            // - y increases downward
            //
            // Then the renderer can place quads with:
            //   quad_pos = origin_px + (glyph.x_px, glyph.y_px) + bearing_px
            // where `bearing_px` is derived from swash placement.
            if let Some(run) = buffer.layout_runs().next() {
                out.metrics.width_px = out.metrics.width_px.max(run.line_w);
                out.metrics.height_px = run.line_height;

                // Baseline offset measured from the top of the run’s line box.
                // Useful for future baseline-aware layout but not required for quad placement here.
                let baseline_px = (run.line_y - run.line_top).max(0.0);
                out.metrics.baseline_px = baseline_px;

                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((0.0, 0.0), 1.0);

                    // Encode the physical cache key data into our backend-agnostic key.
                    // NOTE: For now we still treat `FontId` as a single default font.
                    let key = GlyphKey::new(
                        self.default_font_id,
                        physical.cache_key.glyph_id as u32,
                        f32::from_bits(physical.cache_key.font_size_bits)
                            .round()
                            .max(1.0) as u16,
                        0,
                    );

                    // Convert to line-top-left space:
                    // - `run.line_y` is the baseline y from the top of the line box
                    // - `physical.y` is in baseline space
                    // Therefore: top-left y = baseline_y + physical_y.
                    out.glyphs.push(PositionedGlyph {
                        key,
                        x_px: physical.x as f32,
                        y_px: run.line_y + physical.y as f32,
                    });
                }

                // Single line requested; use the first visible run.
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

        fn rasterize_glyph(&mut self, key: GlyphKey) -> Option<GlyphBitmap> {
            // NOTE: This is a first cut. We currently:
            // - map our opaque `FontId` to a fontdb::ID by picking a default face (Inter)
            // - rasterize to an Alpha mask (R8) via cosmic-text SwashCache
            //
            // Later we should:
            // - make `FontId` map to a specific fontdb::ID deterministically
            // - thread through weight / italic / variations
            // - consider subpixel bins in our public `GlyphKey`
            let font_id = self.find_fontdb_id_for_default()?;

            // Build a cosmic CacheKey and rasterize using SwashCache.
            //
            // `CacheKey::new` returns (cache_key, x, y) where x/y are the integer placement offsets.
            // Those offsets must be applied when placing the bitmap quad.
            let (cache_key, x, y) = cosmic_text::CacheKey::new(
                font_id,
                key.glyph_id as u16,
                key.px_size as f32,
                (0.0, 0.0),
                fontdb::Weight(400),
                cosmic_text::CacheKeyFlags::empty(),
            );

            let image_opt = self
                .swash_cache
                .get_image(&mut self.font_system, cache_key)
                .clone();

            let image = image_opt?;

            // We only support coverage masks for now (which is what the WGPU shader expects).
            // If we ever encounter color glyphs, implement conversion or switch the atlas format.
            if image.content != cosmic_text::SwashContent::Mask {
                return None;
            }

            let w = image.placement.width;
            let h = image.placement.height;

            let pixels = image.data;

            // Coordinate convention: x right, y down.
            // Swash placement uses:
            // - left: x offset to the left edge of bitmap
            // - top: distance from baseline to top edge (positive up)
            //
            // Our convention wants bearing (left, top) in a y-down space, so top becomes -top.
            //
            // Also apply the integer placement offsets returned by `CacheKey::new`:
            // - `x`/`y` shift the bitmap box for pixel-grid alignment.
            let bearing_px = [image.placement.left + x, -image.placement.top + y];

            // Advance isn't directly available from the image; for now the renderer should rely on
            // shaped positioning. Keep a reasonable default.
            let advance_px = [0.0, 0.0];

            Some(GlyphBitmap {
                key,
                size_px: [w, h],
                bearing_px,
                advance_px,
                pixels,
            })
        }
    }

    impl ContentMeasurer for CosmicEngine {
        fn measure_text(&mut self, request: MeasureTextRequest<'_>) -> IntrinsicSize {
            // Use a dummy rect for measurement - we only care about the metrics
            let dummy_rect = Rect::new([0.0, 0.0], [f32::MAX, f32::MAX]);

            let shape_request = ShapeLineRequest {
                text: request.text,
                rect: dummy_rect,
                font_px: request.font_size,
                h_align: request.h_align,
                v_align: request.v_align,
                family: request.family,
            };

            let (shaped_line, _placement) = self.shape_line(shape_request);

            IntrinsicSize::new(shaped_line.metrics.width_px, shaped_line.metrics.height_px)
        }
    }
}
