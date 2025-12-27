use crate::color::Color;
use crate::style::Style;

/// Easing function type: takes progress (0.0 to 1.0) and returns eased value (0.0 to 1.0)
pub type EasingFn = fn(f32) -> f32;

/// Linear interpolation (no easing)
pub fn linear(t: f32) -> f32 {
    t
}

/// Ease in (quadratic) - slow start, accelerating
pub fn ease_in(t: f32) -> f32 {
    t * t
}

/// Ease out (quadratic) - fast start, decelerating
pub fn ease_out(t: f32) -> f32 {
    t * (2.0 - t)
}

/// Ease in-out (quadratic) - slow start and end, fast middle
pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

/// Ease in (cubic) - stronger slow start effect
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// Ease out (cubic) - stronger fast start effect
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t - 1.0;
    t * t * t + 1.0
}

/// Ease in-out (cubic) - stronger slow start/end effect
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t = t - 1.0;
        1.0 + 4.0 * t * t * t
    }
}

/// Linearly interpolate between two f32 values
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linearly interpolate between two colors
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: lerp_f32(a.r, b.r, t),
        g: lerp_f32(a.g, b.g, t),
        b: lerp_f32(a.b, b.b, t),
        a: lerp_f32(a.a, b.a, t),
    }
}

/// Interpolate between two styles
///
/// For each property, if both styles have a value, interpolate between them.
/// Otherwise, use whichever value is present (or None if neither has a value).
pub fn lerp_style(from: &Style, to: &Style, t: f32) -> Style {
    Style {
        fill_color: match (from.fill_color, to.fill_color) {
            (Some(a), Some(b)) => Some(lerp_color(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        stroke_color: match (from.stroke_color, to.stroke_color) {
            (Some(a), Some(b)) => Some(lerp_color(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        stroke_width: match (from.stroke_width, to.stroke_width) {
            (Some(a), Some(b)) => Some(lerp_f32(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        corner_radius: match (from.corner_radius, to.corner_radius) {
            (Some(a), Some(b)) => Some(lerp_f32(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        opacity: match (from.opacity, to.opacity) {
            (Some(a), Some(b)) => Some(lerp_f32(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        text_color: match (from.text_color, to.text_color) {
            (Some(a), Some(b)) => Some(lerp_color(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        cursor_color: match (from.cursor_color, to.cursor_color) {
            (Some(a), Some(b)) => Some(lerp_color(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        offset_x: match (from.offset_x, to.offset_x) {
            (Some(a), Some(b)) => Some(lerp_f32(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
        offset_y: match (from.offset_y, to.offset_y) {
            (Some(a), Some(b)) => Some(lerp_f32(a, b, t)),
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        },
    }
}

/// Transition configuration
///
/// Defines how long a transition takes and what easing function to use.
#[derive(Debug, Clone, Copy)]
pub struct Transition {
    /// Duration in seconds
    pub duration: f32,

    /// Easing function to apply
    pub easing: EasingFn,
}

impl Transition {
    /// Create a new transition with custom duration and easing
    pub fn new(duration: f32, easing: EasingFn) -> Self {
        Self { duration, easing }
    }

    /// Instant transition (no animation, duration = 0)
    pub fn instant() -> Self {
        Self {
            duration: 0.0,
            easing: linear,
        }
    }

    /// Quick transition (150ms, ease-out)
    ///
    /// Good for hover states and quick feedback
    pub fn quick() -> Self {
        Self {
            duration: 0.15,
            easing: ease_out,
        }
    }

    /// Standard transition (250ms, ease-in-out)
    ///
    /// Good for most state changes
    pub fn standard() -> Self {
        Self {
            duration: 0.25,
            easing: ease_in_out,
        }
    }

    /// Slow transition (400ms, ease-in-out)
    ///
    /// Good for emphasized state changes
    pub fn slow() -> Self {
        Self {
            duration: 0.4,
            easing: ease_in_out,
        }
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_easing() {
        assert_eq!(linear(0.0), 0.0);
        assert_eq!(linear(0.5), 0.5);
        assert_eq!(linear(1.0), 1.0);
    }

    #[test]
    fn test_ease_in() {
        assert_eq!(ease_in(0.0), 0.0);
        assert!(ease_in(0.5) < 0.5); // Slower at start
        assert_eq!(ease_in(1.0), 1.0);
    }

    #[test]
    fn test_ease_out() {
        assert_eq!(ease_out(0.0), 0.0);
        assert!(ease_out(0.5) > 0.5); // Faster at start
        assert_eq!(ease_out(1.0), 1.0);
    }

    #[test]
    fn test_lerp_f32() {
        assert_eq!(lerp_f32(0.0, 100.0, 0.0), 0.0);
        assert_eq!(lerp_f32(0.0, 100.0, 0.5), 50.0);
        assert_eq!(lerp_f32(0.0, 100.0, 1.0), 100.0);
    }

    #[test]
    fn test_lerp_color() {
        let black = Color::rgb(0.0, 0.0, 0.0);
        let white = Color::rgb(1.0, 1.0, 1.0);
        let gray = lerp_color(black, white, 0.5);

        assert_eq!(gray.r, 0.5);
        assert_eq!(gray.g, 0.5);
        assert_eq!(gray.b, 0.5);
    }
}
