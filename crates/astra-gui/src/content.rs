use crate::color::Color;

/// Content that can be displayed in a node
///
/// Content nodes are leaf nodes that cannot have children. They represent
/// actual UI elements like text, inputs, images, etc.
#[derive(Debug, Clone)]
pub enum Content {
    /// Text content with styling
    Text(TextContent),
}

/// Text content configuration
#[derive(Debug, Clone)]
pub struct TextContent {
    /// The text to display
    pub text: String,
    /// Font size in pixels
    pub font_size: f32,
    /// Text color
    pub color: Color,
    /// Horizontal alignment within the node
    pub h_align: HorizontalAlign,
    /// Vertical alignment within the node
    pub v_align: VerticalAlign,
}

impl TextContent {
    /// Create new text content with default styling
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 16.0,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }

    /// Set the font size
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set horizontal alignment
    pub fn with_h_align(mut self, align: HorizontalAlign) -> Self {
        self.h_align = align;
        self
    }

    /// Set vertical alignment
    pub fn with_v_align(mut self, align: VerticalAlign) -> Self {
        self.v_align = align;
        self
    }
}

/// Horizontal text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

/// Vertical text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}
