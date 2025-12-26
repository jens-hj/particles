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
    /// Uses proper sRGB gamma correction (ITU-R BT.709)
    #[inline]
    pub const fn from_srgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        const fn srgb_to_linear(c: u8) -> f32 {
            let x = c as f32 / 255.0;
            // Standard sRGB to linear conversion (ITU-R BT.709)
            if x <= 0.04045 {
                x / 12.92
            } else {
                // Approximate ((x + 0.055) / 1.055)^2.4
                // Using simple polynomial approximation (original)
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

    /// with alpha builder method taking u8
    pub fn with_alpha_u8(mut self, alpha: u8) -> Self {
        self.a = alpha as f32 / 255.0;
        self
    }

    /// with alpha builder method taking f32
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}

/// CSS color constants
pub mod css {
    use super::Color;

    pub const AQUA: Color = Color::from_srgba(0, 255, 255, 255);
    pub const BLACK: Color = Color::from_srgba(0, 0, 0, 255);
    pub const BLUE: Color = Color::from_srgba(0, 0, 255, 255);
    pub const FUCHSIA: Color = Color::from_srgba(255, 0, 255, 255);
    pub const GRAY: Color = Color::from_srgba(128, 128, 128, 255);
    pub const GREEN: Color = Color::from_srgba(0, 128, 0, 255);
    pub const LIME: Color = Color::from_srgba(0, 255, 0, 255);
    pub const MAROON: Color = Color::from_srgba(128, 0, 0, 255);
    pub const NAVY: Color = Color::from_srgba(0, 0, 128, 255);
    pub const OLIVE: Color = Color::from_srgba(128, 128, 0, 255);
    pub const PURPLE: Color = Color::from_srgba(128, 0, 128, 255);
    pub const RED: Color = Color::from_srgba(255, 0, 0, 255);
    pub const SILVER: Color = Color::from_srgba(192, 192, 192, 255);
    pub const TEAL: Color = Color::from_srgba(0, 128, 128, 255);
    pub const WHITE: Color = Color::from_srgba(255, 255, 255, 255);
    pub const YELLOW: Color = Color::from_srgba(255, 255, 0, 255);
}

/// Catppuccin color palette
pub mod catppuccin {
    use super::Color;

    pub mod mocha {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgba(245, 224, 220, 255);
        pub const FLAMINGO: Color = Color::from_srgba(242, 205, 205, 255);
        pub const PINK: Color = Color::from_srgba(245, 194, 231, 255);
        pub const MAUVE: Color = Color::from_srgba(203, 166, 247, 255);
        pub const RED: Color = Color::from_srgba(243, 139, 168, 255);
        pub const MAROON: Color = Color::from_srgba(235, 160, 172, 255);
        pub const PEACH: Color = Color::from_srgba(250, 179, 135, 255);
        pub const YELLOW: Color = Color::from_srgba(249, 226, 175, 255);
        pub const GREEN: Color = Color::from_srgba(166, 227, 161, 255);
        pub const TEAL: Color = Color::from_srgba(148, 226, 213, 255);
        pub const SKY: Color = Color::from_srgba(137, 220, 235, 255);
        pub const SAPPHIRE: Color = Color::from_srgba(116, 199, 236, 255);
        pub const BLUE: Color = Color::from_srgba(137, 180, 250, 255);
        pub const LAVENDER: Color = Color::from_srgba(180, 190, 254, 255);
        pub const TEXT: Color = Color::from_srgba(205, 214, 244, 255);
        pub const SUBTEXT1: Color = Color::from_srgba(186, 194, 222, 255);
        pub const SUBTEXT0: Color = Color::from_srgba(166, 173, 200, 255);
        pub const OVERLAY2: Color = Color::from_srgba(147, 153, 178, 255);
        pub const OVERLAY1: Color = Color::from_srgba(127, 132, 156, 255);
        pub const OVERLAY0: Color = Color::from_srgba(108, 112, 134, 255);
        pub const SURFACE2: Color = Color::from_srgba(88, 91, 112, 255);
        pub const SURFACE1: Color = Color::from_srgba(69, 71, 90, 255);
        pub const SURFACE0: Color = Color::from_srgba(49, 50, 68, 255);
        pub const BASE: Color = Color::from_srgba(30, 30, 46, 255);
        pub const MANTLE: Color = Color::from_srgba(24, 24, 37, 255);
        pub const CRUST: Color = Color::from_srgba(17, 17, 27, 255);
    }

    pub mod latte {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgba(220, 138, 120, 255);
        pub const FLAMINGO: Color = Color::from_srgba(221, 120, 120, 255);
        pub const PINK: Color = Color::from_srgba(234, 118, 203, 255);
        pub const MAUVE: Color = Color::from_srgba(136, 57, 239, 255);
        pub const RED: Color = Color::from_srgba(210, 15, 57, 255);
        pub const MAROON: Color = Color::from_srgba(230, 69, 83, 255);
        pub const PEACH: Color = Color::from_srgba(254, 100, 11, 255);
        pub const YELLOW: Color = Color::from_srgba(223, 142, 29, 255);
        pub const GREEN: Color = Color::from_srgba(64, 160, 43, 255);
        pub const TEAL: Color = Color::from_srgba(23, 146, 153, 255);
        pub const SKY: Color = Color::from_srgba(4, 165, 229, 255);
        pub const SAPPHIRE: Color = Color::from_srgba(32, 159, 181, 255);
        pub const BLUE: Color = Color::from_srgba(30, 102, 245, 255);
        pub const LAVENDER: Color = Color::from_srgba(114, 135, 253, 255);
        pub const TEXT: Color = Color::from_srgba(76, 79, 105, 255);
        pub const SUBTEXT1: Color = Color::from_srgba(92, 95, 119, 255);
        pub const SUBTEXT0: Color = Color::from_srgba(108, 111, 133, 255);
        pub const OVERLAY2: Color = Color::from_srgba(124, 127, 147, 255);
        pub const OVERLAY1: Color = Color::from_srgba(140, 143, 161, 255);
        pub const OVERLAY0: Color = Color::from_srgba(156, 160, 176, 255);
        pub const SURFACE2: Color = Color::from_srgba(172, 176, 190, 255);
        pub const SURFACE1: Color = Color::from_srgba(188, 192, 204, 255);
        pub const SURFACE0: Color = Color::from_srgba(204, 208, 218, 255);
        pub const BASE: Color = Color::from_srgba(239, 241, 245, 255);
        pub const MANTLE: Color = Color::from_srgba(230, 233, 239, 255);
        pub const CRUST: Color = Color::from_srgba(220, 224, 232, 255);
    }

    pub mod frappe {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgba(242, 213, 207, 255);
        pub const FLAMINGO: Color = Color::from_srgba(238, 190, 190, 255);
        pub const PINK: Color = Color::from_srgba(244, 184, 228, 255);
        pub const MAUVE: Color = Color::from_srgba(202, 158, 230, 255);
        pub const RED: Color = Color::from_srgba(231, 130, 132, 255);
        pub const MAROON: Color = Color::from_srgba(234, 153, 156, 255);
        pub const PEACH: Color = Color::from_srgba(239, 159, 118, 255);
        pub const YELLOW: Color = Color::from_srgba(229, 200, 144, 255);
        pub const GREEN: Color = Color::from_srgba(166, 209, 137, 255);
        pub const TEAL: Color = Color::from_srgba(129, 200, 190, 255);
        pub const SKY: Color = Color::from_srgba(153, 209, 219, 255);
        pub const SAPPHIRE: Color = Color::from_srgba(133, 193, 220, 255);
        pub const BLUE: Color = Color::from_srgba(140, 170, 238, 255);
        pub const LAVENDER: Color = Color::from_srgba(186, 187, 241, 255);
        pub const TEXT: Color = Color::from_srgba(198, 208, 245, 255);
        pub const SUBTEXT1: Color = Color::from_srgba(181, 191, 226, 255);
        pub const SUBTEXT0: Color = Color::from_srgba(165, 173, 206, 255);
        pub const OVERLAY2: Color = Color::from_srgba(148, 156, 187, 255);
        pub const OVERLAY1: Color = Color::from_srgba(131, 139, 167, 255);
        pub const OVERLAY0: Color = Color::from_srgba(115, 121, 148, 255);
        pub const SURFACE2: Color = Color::from_srgba(98, 104, 128, 255);
        pub const SURFACE1: Color = Color::from_srgba(81, 87, 109, 255);
        pub const SURFACE0: Color = Color::from_srgba(65, 69, 89, 255);
        pub const BASE: Color = Color::from_srgba(48, 52, 70, 255);
        pub const MANTLE: Color = Color::from_srgba(41, 44, 60, 255);
        pub const CRUST: Color = Color::from_srgba(35, 38, 52, 255);
    }

    pub mod macchiato {
        use super::Color;

        pub const ROSEWATER: Color = Color::from_srgba(244, 219, 214, 255);
        pub const FLAMINGO: Color = Color::from_srgba(240, 198, 198, 255);
        pub const PINK: Color = Color::from_srgba(245, 189, 230, 255);
        pub const MAUVE: Color = Color::from_srgba(198, 160, 246, 255);
        pub const RED: Color = Color::from_srgba(237, 135, 150, 255);
        pub const MAROON: Color = Color::from_srgba(238, 153, 160, 255);
        pub const PEACH: Color = Color::from_srgba(245, 169, 127, 255);
        pub const YELLOW: Color = Color::from_srgba(238, 212, 159, 255);
        pub const GREEN: Color = Color::from_srgba(166, 218, 149, 255);
        pub const TEAL: Color = Color::from_srgba(139, 213, 202, 255);
        pub const SKY: Color = Color::from_srgba(145, 215, 227, 255);
        pub const SAPPHIRE: Color = Color::from_srgba(125, 196, 228, 255);
        pub const BLUE: Color = Color::from_srgba(138, 173, 244, 255);
        pub const LAVENDER: Color = Color::from_srgba(183, 189, 248, 255);
        pub const TEXT: Color = Color::from_srgba(202, 211, 245, 255);
        pub const SUBTEXT1: Color = Color::from_srgba(184, 192, 224, 255);
        pub const SUBTEXT0: Color = Color::from_srgba(165, 173, 203, 255);
        pub const OVERLAY2: Color = Color::from_srgba(147, 154, 183, 255);
        pub const OVERLAY1: Color = Color::from_srgba(128, 135, 162, 255);
        pub const OVERLAY0: Color = Color::from_srgba(110, 115, 141, 255);
        pub const SURFACE2: Color = Color::from_srgba(91, 96, 120, 255);
        pub const SURFACE1: Color = Color::from_srgba(73, 77, 100, 255);
        pub const SURFACE0: Color = Color::from_srgba(54, 58, 79, 255);
        pub const BASE: Color = Color::from_srgba(36, 39, 58, 255);
        pub const MANTLE: Color = Color::from_srgba(30, 32, 48, 255);
        pub const CRUST: Color = Color::from_srgba(24, 25, 38, 255);
    }
}
