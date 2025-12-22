#![allow(dead_code)]
//! Glyph atlas placement + light-weight CPU-side cache.
//!
//! This module is intentionally backend-agnostic w.r.t. the shaping/rasterization library.
//! It only answers:
//! - where to place a new glyph bitmap in a 2D atlas
//! - how to look it up later
//!
//! The WGPU renderer can then upload the glyph bitmap into the atlas texture at the returned
//! coordinates.
//!
//! Design goals:
//! - fast insertion and lookup
//! - predictable behavior
//! - no allocations during steady-state beyond the user’s glyph keys
//!
//! Current approach: simple row-based shelf packer.
//! - Atlas is partitioned into horizontal shelves (rows).
//! - Each insertion goes into the first shelf that fits, otherwise a new shelf is created.
//! - This is not optimal packing, but it is simple and very fast.

use std::collections::HashMap;

/// Atlas coordinates in pixels (top-left origin).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AtlasPx {
    pub x: u32,
    pub y: u32,
}

impl AtlasPx {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// Rectangle in atlas pixel coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AtlasRectPx {
    pub min: AtlasPx,
    pub size: [u32; 2],
}

impl AtlasRectPx {
    pub const fn new(min: AtlasPx, size: [u32; 2]) -> Self {
        Self { min, size }
    }

    #[inline]
    pub const fn width(self) -> u32 {
        self.size[0]
    }

    #[inline]
    pub const fn height(self) -> u32 {
        self.size[1]
    }

    #[inline]
    pub const fn max_x(self) -> u32 {
        self.min.x + self.size[0]
    }

    #[inline]
    pub const fn max_y(self) -> u32 {
        self.min.y + self.size[1]
    }
}

/// A placed glyph (including padding) in the atlas.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Rectangle (including padding) where the glyph bitmap was stored.
    pub rect_px: AtlasRectPx,

    /// Padding in pixels that was reserved around the glyph.
    pub padding_px: u32,

    /// UV rectangle corresponding to the glyph bitmap area (excluding padding).
    ///
    /// This is expressed in normalized coordinates [0, 1] for convenience in rendering.
    pub uv: UvRect,
}

/// UV rectangle (normalized texture coords).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct UvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl UvRect {
    pub const fn new(min: [f32; 2], max: [f32; 2]) -> Self {
        Self { min, max }
    }
}

/// Key for a glyph.
///
// NOTE:
// We keep this small and hashable. The renderer/shaper is expected to define how to
// map its glyph identity to this key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Font identifier (e.g. family or an internal ID).
    pub font_id: u64,
    /// Glyph id within the font.
    pub glyph_id: u32,
    /// Font size in pixels (or any consistent scale used during rasterization).
    pub font_px: u16,
    /// Optional extra variant bits (e.g. subpixel/AA mode, weight).
    pub variant: u16,
}

impl GlyphKey {
    pub const fn new(font_id: u64, glyph_id: u32, font_px: u16, variant: u16) -> Self {
        Self {
            font_id,
            glyph_id,
            font_px,
            variant,
        }
    }
}

/// Result of trying to insert into the atlas.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AtlasInsert {
    /// The glyph was already present; no upload needed.
    AlreadyPresent,
    /// The atlas has space and returned placement. The caller should upload the bitmap.
    Placed(PlacedGlyph),
    /// Atlas does not have space (no eviction strategy implemented here).
    Full,
}

/// A single shelf (row) in the atlas.
#[derive(Copy, Clone, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    x_cursor: u32,
}

/// A simple atlas allocator + placement cache.
///
/// The allocator reserves a padding border around each glyph to reduce sampling artifacts.
pub struct GlyphAtlas {
    width: u32,
    height: u32,
    padding_px: u32,

    shelves: Vec<Shelf>,
    next_shelf_y: u32,

    // Cache: glyph key -> placement.
    cache: HashMap<GlyphKey, PlacedGlyph>,
}

impl GlyphAtlas {
    /// Create an atlas of fixed size.
    ///
    /// `padding_px` is reserved around each glyph. A value of 1 is typical for linear sampling.
    pub fn new(width: u32, height: u32, padding_px: u32) -> Self {
        Self {
            width,
            height,
            padding_px,
            shelves: Vec::new(),
            next_shelf_y: 0,
            cache: HashMap::new(),
        }
    }

    #[inline]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub const fn padding_px(&self) -> u32 {
        self.padding_px
    }

    /// Clear the atlas allocator and cache.
    ///
    /// The caller is responsible for clearing/re-initializing the GPU texture as needed.
    pub fn clear(&mut self) {
        self.shelves.clear();
        self.next_shelf_y = 0;
        self.cache.clear();
    }

    /// Lookup an existing glyph placement.
    #[inline]
    pub fn get(&self, key: &GlyphKey) -> Option<PlacedGlyph> {
        self.cache.get(key).copied()
    }

    /// Insert a glyph if absent.
    ///
    /// `bitmap_size_px` is the glyph bitmap dimensions (width, height) **excluding padding**.
    /// Returns:
    /// - `AlreadyPresent` if cache hit
    /// - `Placed(..)` with placement + UVs if space found
    /// - `Full` if no space remains
    pub fn insert(&mut self, key: GlyphKey, bitmap_size_px: [u32; 2]) -> AtlasInsert {
        if self.cache.contains_key(&key) {
            return AtlasInsert::AlreadyPresent;
        }

        let glyph_w = bitmap_size_px[0];
        let glyph_h = bitmap_size_px[1];

        // If bitmap is empty, we still cache it with a 0-area rect.
        // That allows shaping results that include whitespace glyphs to be handled cleanly.
        if glyph_w == 0 || glyph_h == 0 {
            let placed = PlacedGlyph {
                rect_px: AtlasRectPx::new(AtlasPx::new(0, 0), [0, 0]),
                padding_px: self.padding_px,
                uv: UvRect::new([0.0, 0.0], [0.0, 0.0]),
            };
            self.cache.insert(key, placed);
            return AtlasInsert::Placed(placed);
        }

        // Total reserved size includes padding on all sides.
        let pad = self.padding_px;
        let reserved_w = glyph_w.saturating_add(pad.saturating_mul(2));
        let reserved_h = glyph_h.saturating_add(pad.saturating_mul(2));

        // Quick reject if it can never fit.
        if reserved_w > self.width || reserved_h > self.height {
            return AtlasInsert::Full;
        }

        // Try to fit in existing shelves.
        //
        // NOTE: We can’t call `self.*` helpers while holding a mutable borrow of `self.shelves`,
        // so the shelf placement is inlined here to avoid conflicting borrows.
        for shelf in &mut self.shelves {
            if reserved_h <= shelf.height {
                // Simple left-to-right packing within the shelf.
                let x = shelf.x_cursor;
                if x.saturating_add(reserved_w) <= self.width {
                    shelf.x_cursor = shelf.x_cursor.saturating_add(reserved_w);

                    let min = AtlasPx::new(x, shelf.y);
                    let placed = self.make_placed(min, glyph_w, glyph_h);
                    self.cache.insert(key, placed);
                    return AtlasInsert::Placed(placed);
                }
            }
        }

        // Create a new shelf.
        if self.next_shelf_y.saturating_add(reserved_h) > self.height {
            return AtlasInsert::Full;
        }

        let mut new_shelf = Shelf {
            y: self.next_shelf_y,
            height: reserved_h,
            x_cursor: 0,
        };

        let min = match self.try_place_in_shelf(&mut new_shelf, reserved_w, reserved_h) {
            Some(min) => min,
            None => {
                // Should be impossible because we already checked reserved_w <= width.
                return AtlasInsert::Full;
            }
        };

        self.next_shelf_y = self.next_shelf_y.saturating_add(new_shelf.height);
        self.shelves.push(new_shelf);

        let placed = self.make_placed(min, glyph_w, glyph_h);
        self.cache.insert(key, placed);
        AtlasInsert::Placed(placed)
    }

    /// Returns the pixel rect (including padding) that should be updated in the GPU texture.
    ///
    /// This is typically the region `placed.rect_px`, but it can be helpful to fetch
    /// for upload commands.
    #[inline]
    pub fn upload_rect_px(placed: PlacedGlyph) -> AtlasRectPx {
        placed.rect_px
    }

    fn try_place_in_shelf(
        &self,
        shelf: &mut Shelf,
        reserved_w: u32,
        _reserved_h: u32,
    ) -> Option<AtlasPx> {
        // Simple left-to-right packing.
        let x = shelf.x_cursor;
        if x.saturating_add(reserved_w) > self.width {
            return None;
        }

        shelf.x_cursor = shelf.x_cursor.saturating_add(reserved_w);
        Some(AtlasPx::new(x, shelf.y))
    }

    fn make_placed(&self, min: AtlasPx, glyph_w: u32, glyph_h: u32) -> PlacedGlyph {
        let pad = self.padding_px;

        let reserved_w = glyph_w.saturating_add(pad.saturating_mul(2));
        let reserved_h = glyph_h.saturating_add(pad.saturating_mul(2));

        let rect_px = AtlasRectPx::new(min, [reserved_w, reserved_h]);

        // UVs should point to the glyph bitmap area (excluding padding).
        let glyph_min_x = min.x.saturating_add(pad);
        let glyph_min_y = min.y.saturating_add(pad);
        let glyph_max_x = glyph_min_x.saturating_add(glyph_w);
        let glyph_max_y = glyph_min_y.saturating_add(glyph_h);

        let inv_w = 1.0 / (self.width as f32);
        let inv_h = 1.0 / (self.height as f32);

        let uv = UvRect::new(
            [glyph_min_x as f32 * inv_w, glyph_min_y as f32 * inv_h],
            [glyph_max_x as f32 * inv_w, glyph_max_y as f32 * inv_h],
        );

        PlacedGlyph {
            rect_px,
            padding_px: pad,
            uv,
        }
    }
}
