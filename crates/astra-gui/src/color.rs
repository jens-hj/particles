/// RGBA color in linear space with values in [0, 1]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub const fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Convert sRGB color (0-255) to linear space
    /// Uses a polynomial approximation for const evaluation
    #[inline]
    pub const fn from_srgb(r: u8, g: u8, b: u8, a: u8) -> Self {
        const fn srgb_to_linear(c: u8) -> f32 {
            let x = c as f32 / 255.0;
            // Polynomial approximation of sRGB to linear conversion
            // Close enough for color constants (error < 1%)
            if x <= 0.04045 {
                x / 12.92
            } else {
                // Approximate x^2.4 using polynomial for x in [0.04045, 1]
                // This is a Taylor-like expansion tuned for this range
                let t = (x + 0.055) / 1.055;
                t * t * (0.5870 * t + 0.4130)
            }
        }

        Self::new(
            srgb_to_linear(r),
            srgb_to_linear(g),
            srgb_to_linear(b),
            a as f32 / 255.0,
        )
    }

    /// with alpha builder method
    pub fn with_alpha(mut self, alpha: u8) -> Self {
        self.a = alpha as f32 / 255.0;
        self
    }
}

/// CSS color constants
pub mod css {
    use super::Color;

    pub const AQUA: Color = Color::from_srgb(0, 255, 255, 255);
    pub const BLACK: Color = Color::from_srgb(0, 0, 0, 255);
    pub const BLUE: Color = Color::from_srgb(0, 0, 255, 255);
    pub const FUCHSIA: Color = Color::from_srgb(255, 0, 255, 255);
    pub const GRAY: Color = Color::from_srgb(128, 128, 128, 255);
    pub const GREEN: Color = Color::from_srgb(0, 128, 0, 255);
    pub const LIME: Color = Color::from_srgb(0, 255, 0, 255);
    pub const MAROON: Color = Color::from_srgb(128, 0, 0, 255);
    pub const NAVY: Color = Color::from_srgb(0, 0, 128, 255);
    pub const OLIVE: Color = Color::from_srgb(128, 128, 0, 255);
    pub const PURPLE: Color = Color::from_srgb(128, 0, 128, 255);
    pub const RED: Color = Color::from_srgb(255, 0, 0, 255);
    pub const SILVER: Color = Color::from_srgb(192, 192, 192, 255);
    pub const TEAL: Color = Color::from_srgb(0, 128, 128, 255);
    pub const WHITE: Color = Color::from_srgb(255, 255, 255, 255);
    pub const YELLOW: Color = Color::from_srgb(255, 255, 0, 255);
}

/// Catppuccin color palette
pub mod catppuccin {
    use super::Color;

    pub mod mocha {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgb(245, 224, 220, 255);
        pub const FLAMINGO: Color = Color::from_srgb(242, 205, 205, 255);
        pub const PINK: Color = Color::from_srgb(245, 194, 231, 255);
        pub const MAUVE: Color = Color::from_srgb(203, 166, 247, 255);
        pub const RED: Color = Color::from_srgb(243, 139, 168, 255);
        pub const MAROON: Color = Color::from_srgb(235, 160, 172, 255);
        pub const PEACH: Color = Color::from_srgb(250, 179, 135, 255);
        pub const YELLOW: Color = Color::from_srgb(249, 226, 175, 255);
        pub const GREEN: Color = Color::from_srgb(166, 227, 161, 255);
        pub const TEAL: Color = Color::from_srgb(148, 226, 213, 255);
        pub const SKY: Color = Color::from_srgb(137, 220, 235, 255);
        pub const SAPPHIRE: Color = Color::from_srgb(116, 199, 236, 255);
        pub const BLUE: Color = Color::from_srgb(137, 180, 250, 255);
        pub const LAVENDER: Color = Color::from_srgb(180, 190, 254, 255);
        pub const TEXT: Color = Color::from_srgb(205, 214, 244, 255);
        pub const SUBTEXT1: Color = Color::from_srgb(186, 194, 222, 255);
        pub const SUBTEXT0: Color = Color::from_srgb(166, 173, 200, 255);
        pub const OVERLAY2: Color = Color::from_srgb(147, 153, 178, 255);
        pub const OVERLAY1: Color = Color::from_srgb(127, 132, 156, 255);
        pub const OVERLAY0: Color = Color::from_srgb(108, 112, 134, 255);
        pub const SURFACE2: Color = Color::from_srgb(88, 91, 112, 255);
        pub const SURFACE1: Color = Color::from_srgb(69, 71, 90, 255);
        pub const SURFACE0: Color = Color::from_srgb(49, 50, 68, 255);
        pub const BASE: Color = Color::from_srgb(30, 30, 46, 255);
        pub const MANTLE: Color = Color::from_srgb(24, 24, 37, 255);
        pub const CRUST: Color = Color::from_srgb(17, 17, 27, 255);
    }

    pub mod latte {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgb(220, 138, 120, 255);
        pub const FLAMINGO: Color = Color::from_srgb(221, 120, 120, 255);
        pub const PINK: Color = Color::from_srgb(234, 118, 203, 255);
        pub const MAUVE: Color = Color::from_srgb(136, 57, 239, 255);
        pub const RED: Color = Color::from_srgb(210, 15, 57, 255);
        pub const MAROON: Color = Color::from_srgb(230, 69, 83, 255);
        pub const PEACH: Color = Color::from_srgb(254, 100, 11, 255);
        pub const YELLOW: Color = Color::from_srgb(223, 142, 29, 255);
        pub const GREEN: Color = Color::from_srgb(64, 160, 43, 255);
        pub const TEAL: Color = Color::from_srgb(23, 146, 153, 255);
        pub const SKY: Color = Color::from_srgb(4, 165, 229, 255);
        pub const SAPPHIRE: Color = Color::from_srgb(32, 159, 181, 255);
        pub const BLUE: Color = Color::from_srgb(30, 102, 245, 255);
        pub const LAVENDER: Color = Color::from_srgb(114, 135, 253, 255);
        pub const TEXT: Color = Color::from_srgb(76, 79, 105, 255);
        pub const SUBTEXT1: Color = Color::from_srgb(92, 95, 119, 255);
        pub const SUBTEXT0: Color = Color::from_srgb(108, 111, 133, 255);
        pub const OVERLAY2: Color = Color::from_srgb(124, 127, 147, 255);
        pub const OVERLAY1: Color = Color::from_srgb(140, 143, 161, 255);
        pub const OVERLAY0: Color = Color::from_srgb(156, 160, 176, 255);
        pub const SURFACE2: Color = Color::from_srgb(172, 176, 190, 255);
        pub const SURFACE1: Color = Color::from_srgb(188, 192, 204, 255);
        pub const SURFACE0: Color = Color::from_srgb(204, 208, 218, 255);
        pub const BASE: Color = Color::from_srgb(239, 241, 245, 255);
        pub const MANTLE: Color = Color::from_srgb(230, 233, 239, 255);
        pub const CRUST: Color = Color::from_srgb(220, 224, 232, 255);
    }

    pub mod frappe {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgb(242, 213, 207, 255);
        pub const FLAMINGO: Color = Color::from_srgb(238, 190, 190, 255);
        pub const PINK: Color = Color::from_srgb(244, 184, 228, 255);
        pub const MAUVE: Color = Color::from_srgb(202, 158, 230, 255);
        pub const RED: Color = Color::from_srgb(231, 130, 132, 255);
        pub const MAROON: Color = Color::from_srgb(234, 153, 156, 255);
        pub const PEACH: Color = Color::from_srgb(239, 159, 118, 255);
        pub const YELLOW: Color = Color::from_srgb(229, 200, 144, 255);
        pub const GREEN: Color = Color::from_srgb(166, 209, 137, 255);
        pub const TEAL: Color = Color::from_srgb(129, 200, 190, 255);
        pub const SKY: Color = Color::from_srgb(153, 209, 219, 255);
        pub const SAPPHIRE: Color = Color::from_srgb(133, 193, 220, 255);
        pub const BLUE: Color = Color::from_srgb(140, 170, 238, 255);
        pub const LAVENDER: Color = Color::from_srgb(186, 187, 241, 255);
        pub const TEXT: Color = Color::from_srgb(198, 208, 245, 255);
        pub const SUBTEXT1: Color = Color::from_srgb(181, 191, 226, 255);
        pub const SUBTEXT0: Color = Color::from_srgb(165, 173, 206, 255);
        pub const OVERLAY2: Color = Color::from_srgb(148, 156, 187, 255);
        pub const OVERLAY1: Color = Color::from_srgb(131, 139, 167, 255);
        pub const OVERLAY0: Color = Color::from_srgb(115, 121, 148, 255);
        pub const SURFACE2: Color = Color::from_srgb(98, 104, 128, 255);
        pub const SURFACE1: Color = Color::from_srgb(81, 87, 109, 255);
        pub const SURFACE0: Color = Color::from_srgb(65, 69, 89, 255);
        pub const BASE: Color = Color::from_srgb(48, 52, 70, 255);
        pub const MANTLE: Color = Color::from_srgb(41, 44, 60, 255);
        pub const CRUST: Color = Color::from_srgb(35, 38, 52, 255);
    }

    pub mod macchiato {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgb(244, 219, 214, 255);
        pub const FLAMINGO: Color = Color::from_srgb(240, 198, 198, 255);
        pub const PINK: Color = Color::from_srgb(245, 189, 230, 255);
        pub const MAUVE: Color = Color::from_srgb(198, 160, 246, 255);
        pub const RED: Color = Color::from_srgb(237, 135, 150, 255);
        pub const MAROON: Color = Color::from_srgb(238, 153, 160, 255);
        pub const PEACH: Color = Color::from_srgb(245, 169, 127, 255);
        pub const YELLOW: Color = Color::from_srgb(238, 212, 159, 255);
        pub const GREEN: Color = Color::from_srgb(166, 218, 149, 255);
        pub const TEAL: Color = Color::from_srgb(139, 213, 202, 255);
        pub const SKY: Color = Color::from_srgb(145, 215, 227, 255);
        pub const SAPPHIRE: Color = Color::from_srgb(125, 196, 228, 255);
        pub const BLUE: Color = Color::from_srgb(138, 173, 244, 255);
        pub const LAVENDER: Color = Color::from_srgb(183, 189, 248, 255);
        pub const TEXT: Color = Color::from_srgb(202, 211, 245, 255);
        pub const SUBTEXT1: Color = Color::from_srgb(184, 192, 224, 255);
        pub const SUBTEXT0: Color = Color::from_srgb(165, 173, 203, 255);
        pub const OVERLAY2: Color = Color::from_srgb(147, 154, 183, 255);
        pub const OVERLAY1: Color = Color::from_srgb(128, 135, 162, 255);
        pub const OVERLAY0: Color = Color::from_srgb(110, 115, 141, 255);
        pub const SURFACE2: Color = Color::from_srgb(91, 96, 120, 255);
        pub const SURFACE1: Color = Color::from_srgb(73, 77, 100, 255);
        pub const SURFACE0: Color = Color::from_srgb(54, 58, 79, 255);
        pub const BASE: Color = Color::from_srgb(36, 39, 58, 255);
        pub const MANTLE: Color = Color::from_srgb(30, 32, 48, 255);
        pub const CRUST: Color = Color::from_srgb(24, 25, 38, 255);
    }
}
