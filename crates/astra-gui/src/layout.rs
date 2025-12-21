use crate::primitives::Rect;

/// Size specification that can be fixed or relative to parent
#[derive(Clone, Copy, Debug)]
pub enum Size {
    /// Fixed size in pixels
    Fixed(f32),
    /// Relative size as a fraction of parent (0.0 to 1.0)
    Relative(f32),
}

impl Size {
    /// Create a fixed size in pixels
    pub const fn px(pixels: f32) -> Self {
        Self::Fixed(pixels)
    }

    /// Create a relative size as a percentage (0.0 to 1.0)
    pub const fn percent(fraction: f32) -> Self {
        Self::Relative(fraction)
    }

    /// Resolve the size given the parent's dimension
    pub fn resolve(&self, parent_size: f32) -> f32 {
        match self {
            Size::Fixed(px) => *px,
            Size::Relative(fraction) => parent_size * fraction,
        }
    }
}

/// Layout direction for arranging children
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDirection {
    /// Children are arranged horizontally (left to right)
    Horizontal,
    /// Children are arranged vertically (top to bottom)
    Vertical,
}

/// Position offset from the parent's origin
#[derive(Clone, Copy, Debug, Default)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

impl Offset {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Computed layout information after tree traversal
#[derive(Clone, Copy, Debug)]
pub struct ComputedLayout {
    /// Absolute position in screen coordinates
    pub rect: Rect,
}

impl ComputedLayout {
    pub fn new(rect: Rect) -> Self {
        Self { rect }
    }
}

/// Spacing/padding around content
#[derive(Clone, Copy, Debug, Default)]
pub struct Spacing {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Spacing {
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn zero() -> Self {
        Self::all(0.0)
    }

    pub const fn horizontal_vertical(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}
