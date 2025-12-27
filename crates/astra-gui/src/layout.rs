use crate::primitives::Rect;

/// Size specification that can be fixed, relative to parent, or derived from content.
#[derive(Clone, Copy, Debug)]
pub enum Size {
    /// Fixed size in pixels
    Fixed(f32),
    /// Relative size as a fraction of parent (0.0 to 1.0)
    Relative(f32),
    /// Fill all remaining available space
    Fill,
    /// Size to the minimum that fits content (text metrics or children), plus padding.
    ///
    /// NOTE: The layout algorithm must measure intrinsic content size to resolve this.
    FitContent,
}

/// Overflow policy for content/children that exceed the node's bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    /// Content can render outside the node's bounds.
    Visible,
    /// Content is clipped to the node's bounds.
    Hidden,
    /// Content is clipped but can be scrolled (not implemented yet).
    Scroll,
}

impl Size {
    /// Create a fixed size in pixels
    pub const fn px(pixels: f32) -> Self {
        Self::Fixed(pixels)
    }

    /// Create a relative size as a percentage (0.0 to 1.0)
    pub const fn fraction(fraction: f32) -> Self {
        Self::Relative(fraction)
    }

    /// Size to the minimum that fits content.
    pub const fn fit_content() -> Self {
        Self::FitContent
    }

    /// Resolve the size given the parent's dimension
    ///
    /// This only works for `Fixed` and `Relative` sizes. For `Fill` and `FitContent`,
    /// the layout algorithm must compute the size differently:
    /// - `Fill`: Computed based on remaining space after other siblings
    /// - `FitContent`: Computed via intrinsic measurement of content/children
    ///
    /// # Panics
    /// Panics if called on `Fill` or `FitContent` - these must be handled by the layout algorithm.
    pub fn resolve(&self, parent_size: f32) -> f32 {
        match self {
            Size::Fixed(px) => *px,
            Size::Relative(fraction) => parent_size * fraction,
            Size::Fill => panic!("Cannot resolve Size::Fill - must be computed by layout algorithm based on remaining space"),
            Size::FitContent => panic!("Cannot resolve Size::FitContent - must be computed via intrinsic measurement"),
        }
    }

    /// Try to resolve the size, returning None for Fill and FitContent
    ///
    /// This is a non-panicking version of `resolve()` that returns `None`
    /// for sizes that cannot be resolved without additional context.
    pub fn try_resolve(&self, parent_size: f32) -> Option<f32> {
        match self {
            Size::Fixed(px) => Some(*px),
            Size::Relative(fraction) => Some(parent_size * fraction),
            Size::Fill | Size::FitContent => None,
        }
    }

    /// Check if this size is Fill
    pub const fn is_fill(&self) -> bool {
        matches!(self, Size::Fill)
    }

    /// Check if this size is FitContent
    pub const fn is_fit_content(&self) -> bool {
        matches!(self, Size::FitContent)
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::FitContent
    }
}

impl Default for Overflow {
    fn default() -> Self {
        Self::Visible
    }
}

/// Layout mode for arranging children
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Children are arranged horizontally (left to right)
    Horizontal,
    /// Children are arranged vertically (top to bottom)
    Vertical,
    /// Children are stacked in the Z direction (overlapping)
    Stack,
}

impl Default for Layout {
    fn default() -> Self {
        Self::Vertical
    }
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

    pub const fn x(x: f32) -> Self {
        Self { x, y: 0.0 }
    }

    pub const fn y(y: f32) -> Self {
        Self { x: 0.0, y }
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
    /// Create spacing with all sides equal
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create zero spacing
    pub const fn zero() -> Self {
        Self::all(0.0)
    }

    /// Create spacing with symmetric horizontal and vertical values (CSS-style)
    ///
    /// ```
    /// # use astra_gui::Spacing;
    /// let spacing = Spacing::symmetric(10.0, 20.0);
    /// assert_eq!(spacing.left, 10.0);
    /// assert_eq!(spacing.right, 10.0);
    /// assert_eq!(spacing.top, 20.0);
    /// assert_eq!(spacing.bottom, 20.0);
    /// ```
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create spacing from individual top, right, bottom, left values (CSS-style)
    ///
    /// ```
    /// # use astra_gui::Spacing;
    /// let spacing = Spacing::trbl(10.0, 20.0, 30.0, 40.0);
    /// assert_eq!(spacing.top, 10.0);
    /// assert_eq!(spacing.right, 20.0);
    /// assert_eq!(spacing.bottom, 30.0);
    /// assert_eq!(spacing.left, 40.0);
    /// ```
    pub const fn trbl(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn horizontal(horizontal: f32) -> Self {
        Self {
            top: 0.0,
            right: horizontal,
            bottom: 0.0,
            left: horizontal,
        }
    }

    pub const fn vertical(vertical: f32) -> Self {
        Self {
            top: vertical,
            right: 0.0,
            bottom: vertical,
            left: 0.0,
        }
    }

    pub const fn top(top: f32) -> Self {
        Self {
            top,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub const fn right(right: f32) -> Self {
        Self {
            top: 0.0,
            right,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub const fn bottom(bottom: f32) -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom,
            left: 0.0,
        }
    }

    pub const fn left(left: f32) -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left,
        }
    }

    pub const fn get_vertical(&self) -> f32 {
        self.top + self.bottom
    }

    pub const fn get_horizontal(&self) -> f32 {
        self.right + self.left
    }
}
