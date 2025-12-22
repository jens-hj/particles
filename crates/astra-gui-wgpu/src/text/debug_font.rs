//! Tiny built-in bitmap font used as a temporary fallback for text rendering.
//!
//! This is intentionally simple: ASCII-ish coverage, monospaced metrics, and a
//! CPU raster that outputs an `R8` coverage mask suitable for uploading into an
//! `R8Unorm` WGPU texture atlas.
//!
//! NOTE: This module is a stopgap until proper shaping + rasterization is wired
//! through `cosmic-text`.

#![cfg(feature = "text-cosmic")]

/// Monospace font metrics in pixels (at the font's *base* pixel size).
#[derive(Clone, Copy, Debug)]
pub struct DebugFontMetrics {
    /// Base glyph advance (monospace cell width) in pixels.
    pub advance_px: u32,
    /// Base glyph cell height in pixels.
    pub height_px: u32,
    /// Baseline offset from the top of the cell in pixels.
    pub baseline_from_top_px: u32,
}

/// A single rasterized glyph bitmap and its placement metrics.
#[derive(Clone, Debug)]
pub struct GlyphBitmap {
    pub ch: char,
    /// Bitmap dimensions in pixels (width, height).
    pub size_px: [u32; 2],
    /// Pixels in row-major order, one byte per pixel (coverage 0..=255).
    pub pixels: Vec<u8>,
    /// Bearing (left, top) in pixels relative to the pen position.
    ///
    /// For this debug font we keep it simple: (0, -height) so that y grows down.
    pub bearing_px: [i32; 2],
    /// Advance (x, y) in pixels to apply to the pen after drawing this glyph.
    pub advance_px: [f32; 2],
}

/// Tiny builtin "font".
///
/// This produces a crude but readable bitmap for a subset of ASCII. For unknown
/// glyphs, it renders a placeholder box.
#[derive(Clone, Debug)]
pub struct DebugFont {
    metrics: DebugFontMetrics,
}

impl DebugFont {
    /// Create a new debug font with fixed base metrics.
    ///
    /// Base cell is 8x12 by default; baseline is at 9px from the top.
    pub fn new() -> Self {
        Self {
            metrics: DebugFontMetrics {
                advance_px: 8,
                height_px: 12,
                baseline_from_top_px: 9,
            },
        }
    }

    pub fn metrics(&self) -> DebugFontMetrics {
        self.metrics
    }

    /// Rasterize a glyph at an integer scale.
    ///
    /// - `scale` is clamped to >= 1.
    /// - Output is an `R8` mask where 0 is transparent and 255 is fully covered.
    pub fn rasterize_glyph(&self, ch: char, scale: u32) -> GlyphBitmap {
        let scale = scale.max(1);

        let w = self.metrics.advance_px * scale;
        let h = self.metrics.height_px * scale;
        let mut pixels = vec![0u8; (w * h) as usize];

        // A very small 5x7-esque stroke font embedded at runtime (procedural).
        // We draw into the cell with some padding.
        let pad_x = 1 * scale;
        let pad_y = 2 * scale;

        let box_w = w.saturating_sub(2 * pad_x).max(1);
        let box_h = h.saturating_sub(2 * pad_y).max(1);

        // Render known glyphs. This is intentionally minimal, but it should cover
        // the demo strings well enough.
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' => {
                self.draw_alnum(&mut pixels, w, h, ch, pad_x, pad_y, box_w, box_h, scale);
            }
            ' ' => {
                // Space: nothing.
            }
            ':' | ';' | '.' | ',' | '!' | '?' | '-' | '_' | '/' | '\\' | '(' | ')' | '[' | ']'
            | '{' | '}' | '\'' | '"' | '+' | '=' | '*' | '#' => {
                self.draw_punct(&mut pixels, w, h, ch, pad_x, pad_y, box_w, box_h, scale);
            }
            _ => {
                // Placeholder box for unknown glyphs.
                self.draw_box(&mut pixels, w, h, pad_x, pad_y, box_w, box_h, scale);
            }
        }

        GlyphBitmap {
            ch,
            size_px: [w, h],
            pixels,
            // top-left origin with y-down: draw at pen, but shift up by height to approximate baseline use.
            bearing_px: [0, -(h as i32)],
            advance_px: [self.metrics.advance_px as f32 * scale as f32, 0.0],
        }
    }

    fn set_px(pixels: &mut [u8], w: u32, h: u32, x: u32, y: u32, v: u8) {
        if x >= w || y >= h {
            return;
        }
        let idx = (y * w + x) as usize;
        pixels[idx] = pixels[idx].saturating_add(v);
    }

    fn hline(&self, pixels: &mut [u8], w: u32, h: u32, x0: u32, x1: u32, y: u32, thickness: u32) {
        let x0 = x0.min(w);
        let x1 = x1.min(w);
        for t in 0..thickness {
            let yy = y.saturating_add(t);
            for x in x0..x1 {
                Self::set_px(pixels, w, h, x, yy, 255);
            }
        }
    }

    fn vline(&self, pixels: &mut [u8], w: u32, h: u32, x: u32, y0: u32, y1: u32, thickness: u32) {
        let y0 = y0.min(h);
        let y1 = y1.min(h);
        for t in 0..thickness {
            let xx = x.saturating_add(t);
            for y in y0..y1 {
                Self::set_px(pixels, w, h, xx, y, 255);
            }
        }
    }

    fn draw_box(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        pad_x: u32,
        pad_y: u32,
        box_w: u32,
        box_h: u32,
        scale: u32,
    ) {
        let t = 1 * scale;
        let x0 = pad_x;
        let y0 = pad_y;
        let x1 = pad_x + box_w;
        let y1 = pad_y + box_h;
        self.hline(pixels, w, h, x0, x1, y0, t);
        self.hline(pixels, w, h, x0, x1, y1.saturating_sub(t), t);
        self.vline(pixels, w, h, x0, y0, y1, t);
        self.vline(pixels, w, h, x1.saturating_sub(t), y0, y1, t);
    }

    fn draw_alnum(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        ch: char,
        pad_x: u32,
        pad_y: u32,
        box_w: u32,
        box_h: u32,
        scale: u32,
    ) {
        // Extremely crude "segment" glyphs: vertical strokes + horizontal strokes.
        // It's not pretty, but it's readable at demo sizes, and stable.
        let t = 1 * scale;

        let x0 = pad_x;
        let y0 = pad_y;
        let x1 = pad_x + box_w;
        let y1 = pad_y + box_h;

        let mid_y = y0 + (box_h / 2);
        let mid_x = x0 + (box_w / 2);

        // Common segments
        let top = (y0, x0, x1);
        let mid = (mid_y, x0, x1);
        let bot = (y1.saturating_sub(t), x0, x1);
        let left = (x0, y0, y1);
        let right = (x1.saturating_sub(t), y0, y1);

        // NOTE: Avoid closures that capture `&mut pixels` multiple times; it creates overlapping
        // mutable borrows. Use explicit calls instead.
        let dot = |pixels: &mut [u8], dx: i32, dy: i32| {
            let xx = (mid_x as i32 + dx).max(0) as u32;
            let yy = (mid_y as i32 + dy).max(0) as u32;
            for sx in 0..t {
                for sy in 0..t {
                    Self::set_px(
                        pixels,
                        w,
                        h,
                        xx.saturating_add(sx),
                        yy.saturating_add(sy),
                        255,
                    );
                }
            }
        };

        match ch {
            // Digits with classic 7-segment approximations.
            '0' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            '1' => {
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            '2' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                // Right upper, left lower
                self.vline(pixels, w, h, right.0, y0, mid_y, t);
                self.vline(pixels, w, h, left.0, mid_y, y1, t);
            }
            '3' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            '4' => {
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            '5' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, y0, mid_y, t);
                self.vline(pixels, w, h, right.0, mid_y, y1, t);
            }
            '6' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, mid_y, y1, t);
            }
            '7' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            '8' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            '9' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
                self.vline(pixels, w, h, left.0, y0, mid_y, t);
            }

            // Letters: crude approximations using box strokes.
            c if c == 'A' || c == 'a' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            c if c == 'B' || c == 'b' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, right.0, y0, mid_y, t);
                self.vline(pixels, w, h, right.0, mid_y, y1, t);
            }
            c if c == 'C' || c == 'c' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
            }
            c if c == 'D' || c == 'd' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            c if c == 'E' || c == 'e' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
            }
            c if c == 'F' || c == 'f' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
            }
            c if c == 'G' || c == 'g' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, mid_x, x1, mid_y, t);
                self.vline(pixels, w, h, right.0, mid_y, y1, t);
            }
            c if c == 'H' || c == 'h' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
            }
            c if c == 'I' || c == 'i' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, mid_x, y0, y1, t);
            }
            c if c == 'L' || c == 'l' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
            }
            c if c == 'M' || c == 'm' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
                dot(pixels, -((box_w as i32) / 4), -((box_h as i32) / 4));
                dot(pixels, (box_w as i32) / 4, -((box_h as i32) / 4));
            }
            c if c == 'N' || c == 'n' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
                dot(pixels, 0, -((box_h as i32) / 4));
                dot(pixels, 0, (box_h as i32) / 4);
            }
            c if c == 'O' || c == 'o' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
            }
            c if c == 'P' || c == 'p' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.vline(pixels, w, h, right.0, y0, mid_y, t);
            }
            c if c == 'R' || c == 'r' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.vline(pixels, w, h, right.0, y0, mid_y, t);
                dot(pixels, 0, (box_h as i32) / 4);
            }
            c if c == 'S' || c == 's' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                self.vline(pixels, w, h, left.0, y0, mid_y, t);
                self.vline(pixels, w, h, right.0, mid_y, y1, t);
            }
            c if c == 'T' || c == 't' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.vline(pixels, w, h, mid_x, y0, y1, t);
            }
            c if c == 'U' || c == 'u' => {
                self.vline(pixels, w, h, left.0, left.1, left.2, t);
                self.vline(pixels, w, h, right.0, right.1, right.2, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
            }
            c if c == 'X' || c == 'x' => {
                dot(pixels, -((box_w as i32) / 4), -((box_h as i32) / 4));
                dot(pixels, (box_w as i32) / 4, -((box_h as i32) / 4));
                dot(pixels, -((box_w as i32) / 4), (box_h as i32) / 4);
                dot(pixels, (box_w as i32) / 4, (box_h as i32) / 4);
            }
            c if c == 'Y' || c == 'y' => {
                self.hline(pixels, w, h, mid.1, mid.2, mid.0, t);
                self.vline(pixels, w, h, mid_x, mid_y, y1, t);
                dot(pixels, -((box_w as i32) / 4), -((box_h as i32) / 4));
                dot(pixels, (box_w as i32) / 4, -((box_h as i32) / 4));
            }
            c if c == 'Z' || c == 'z' => {
                self.hline(pixels, w, h, top.1, top.2, top.0, t);
                self.hline(pixels, w, h, bot.1, bot.2, bot.0, t);
                dot(pixels, (box_w as i32) / 4, -((box_h as i32) / 4));
                dot(pixels, -((box_w as i32) / 4), (box_h as i32) / 4);
            }
            _ => {
                // Default for any other alnum: just draw a simple box.
                self.draw_box(pixels, w, h, pad_x, pad_y, box_w, box_h, scale);
            }
        }
    }

    fn draw_punct(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        ch: char,
        pad_x: u32,
        pad_y: u32,
        box_w: u32,
        box_h: u32,
        scale: u32,
    ) {
        let t = 1 * scale;
        let x0 = pad_x;
        let y0 = pad_y;
        let x1 = pad_x + box_w;
        let y1 = pad_y + box_h;

        let mid_x = x0 + (box_w / 2);
        let mid_y = y0 + (box_h / 2);

        match ch {
            '-' => self.hline(pixels, w, h, x0, x1, mid_y, t),
            '_' => self.hline(pixels, w, h, x0, x1, y1.saturating_sub(t), t),
            '.' => Self::set_px(pixels, w, h, mid_x, y1.saturating_sub(2 * t), 255),
            ',' => {
                Self::set_px(pixels, w, h, mid_x, y1.saturating_sub(2 * t), 255);
                Self::set_px(
                    pixels,
                    w,
                    h,
                    mid_x.saturating_sub(t),
                    y1.saturating_sub(t),
                    255,
                );
            }
            ':' => {
                Self::set_px(pixels, w, h, mid_x, mid_y.saturating_sub(2 * t), 255);
                Self::set_px(pixels, w, h, mid_x, mid_y.saturating_add(2 * t), 255);
            }
            '!' => {
                self.vline(pixels, w, h, mid_x, y0, y1.saturating_sub(3 * t), t);
                Self::set_px(pixels, w, h, mid_x, y1.saturating_sub(2 * t), 255);
            }
            '?' => {
                self.hline(pixels, w, h, x0, x1, y0, t);
                self.vline(pixels, w, h, x1.saturating_sub(t), y0, mid_y, t);
                self.hline(pixels, w, h, x0, x1, mid_y, t);
                Self::set_px(pixels, w, h, mid_x, y1.saturating_sub(2 * t), 255);
            }
            '/' => {
                // crude diagonal
                let steps = box_h.max(1);
                for i in 0..steps {
                    let xx = x1.saturating_sub(1).saturating_sub(i * box_w / steps);
                    let yy = y0.saturating_add(i);
                    Self::set_px(pixels, w, h, xx, yy, 255);
                }
            }
            '\\' => {
                let steps = box_h.max(1);
                for i in 0..steps {
                    let xx = x0.saturating_add(i * box_w / steps);
                    let yy = y0.saturating_add(i);
                    Self::set_px(pixels, w, h, xx, yy, 255);
                }
            }
            '+' => {
                self.hline(pixels, w, h, x0, x1, mid_y, t);
                self.vline(pixels, w, h, mid_x, y0, y1, t);
            }
            '=' => {
                self.hline(pixels, w, h, x0, x1, mid_y.saturating_sub(2 * t), t);
                self.hline(pixels, w, h, x0, x1, mid_y.saturating_add(2 * t), t);
            }
            '(' => {
                self.vline(pixels, w, h, x0, y0, y1, t);
                Self::set_px(pixels, w, h, x0.saturating_add(t), y0, 255);
                Self::set_px(
                    pixels,
                    w,
                    h,
                    x0.saturating_add(t),
                    y1.saturating_sub(t),
                    255,
                );
            }
            ')' => {
                self.vline(pixels, w, h, x1.saturating_sub(t), y0, y1, t);
                Self::set_px(pixels, w, h, x1.saturating_sub(2 * t), y0, 255);
                Self::set_px(
                    pixels,
                    w,
                    h,
                    x1.saturating_sub(2 * t),
                    y1.saturating_sub(t),
                    255,
                );
            }
            '[' => {
                self.vline(pixels, w, h, x0, y0, y1, t);
                self.hline(pixels, w, h, x0, x0 + (box_w / 2), y0, t);
                self.hline(pixels, w, h, x0, x0 + (box_w / 2), y1.saturating_sub(t), t);
            }
            ']' => {
                self.vline(pixels, w, h, x1.saturating_sub(t), y0, y1, t);
                self.hline(pixels, w, h, x1 - (box_w / 2), x1, y0, t);
                self.hline(pixels, w, h, x1 - (box_w / 2), x1, y1.saturating_sub(t), t);
            }
            '{' | '}' | '*' | '#' | '\'' | '"' | ';' => {
                // fallback: box for these
                self.draw_box(pixels, w, h, pad_x, pad_y, box_w, box_h, scale);
            }
            _ => self.draw_box(pixels, w, h, pad_x, pad_y, box_w, box_h, scale),
        }
    }
}
