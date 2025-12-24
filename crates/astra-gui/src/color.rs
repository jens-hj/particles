/// RGBA color in linear space with values in [0, 1]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn transparent() -> Self {
        Self::rgba(0.0, 0.0, 0.0, 0.0)
    }

    /// Create a color from a hexadecimal value.
    /// Handle hex values with or without alpha channel.
    pub const fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;

        Self::rgba(r, g, b, a)
    }

    /// Create a color from a hex string
    /// Handle with or without alpha channel.
    pub fn from_hex_str(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let hex = hex.parse::<u32>().unwrap_or(0);

        Self::from_hex(hex)
    }

    /// Create a color from rgb u8
    pub const fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }

    /// Create a color from rgba u8
    pub const fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }
}
