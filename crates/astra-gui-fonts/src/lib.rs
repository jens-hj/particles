//! Bundled font binaries for Astra GUI.
//!
//! This crate is intentionally tiny: it exposes font bytes via `include_bytes!` so that
//! backends and text engines can load fonts without reaching out to the network at runtime.
//!
//! Fonts are stored in this repo under `particles/assets/fonts/*`.
//! Licensing information is included alongside the fonts (see the `OFL.txt` files).

#![deny(warnings)]

/// Inter (variable font, roman).
///
/// Source file in repo:
/// `particles/assets/fonts/inter/Inter-VariableFont_opsz,wght.ttf`
#[cfg(feature = "inter")]
pub mod inter {
    /// Returns the raw bytes for the Inter variable font (roman).
    pub fn variable_opsz_wght() -> &'static [u8] {
        include_bytes!("../../../assets/fonts/inter/Inter-VariableFont_opsz,wght.ttf")
    }

    /// Returns the raw bytes for the Inter variable font (italic).
    pub fn italic_variable_opsz_wght() -> &'static [u8] {
        include_bytes!("../../../assets/fonts/inter/Inter-Italic-VariableFont_opsz,wght.ttf")
    }

    /// Returns the SIL Open Font License text shipped with Inter in this repository.
    pub fn ofl_text() -> &'static str {
        include_str!("../../../assets/fonts/inter/OFL.txt")
    }
}

/// JetBrains Mono (variable font).
///
/// Included for future use. Not enabled by default.
///
/// Source files in repo:
/// `particles/assets/fonts/jetbrainsmono/JetBrainsMono-VariableFont_wght.ttf`
/// `particles/assets/fonts/jetbrainsmono/JetBrainsMono-Italic-VariableFont_wght.ttf`
#[cfg(feature = "jetbrains-mono")]
pub mod jetbrains_mono {
    /// Returns the raw bytes for the JetBrains Mono variable font (roman).
    pub fn variable_wght() -> &'static [u8] {
        include_bytes!("../../../assets/fonts/jetbrainsmono/JetBrainsMono-VariableFont_wght.ttf")
    }

    /// Returns the raw bytes for the JetBrains Mono variable font (italic).
    pub fn italic_variable_wght() -> &'static [u8] {
        include_bytes!(
            "../../../assets/fonts/jetbrainsmono/JetBrainsMono-Italic-VariableFont_wght.ttf"
        )
    }

    /// Returns the SIL Open Font License text shipped with JetBrains Mono in this repository.
    pub fn ofl_text() -> &'static str {
        include_str!("../../../assets/fonts/jetbrainsmono/OFL.txt")
    }
}
