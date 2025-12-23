use crate::color::Color;
use crate::content::{HorizontalAlign, TextContent, VerticalAlign};

/// Stroke definition with width and color
#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

impl Stroke {
    pub const fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

/// Axis-aligned rectangle defined by min and max corners
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Rect {
    pub const fn new(min: [f32; 2], max: [f32; 2]) -> Self {
        Self { min, max }
    }

    pub fn from_min_size(min: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            min,
            max: [min[0] + size[0], min[1] + size[1]],
        }
    }

    pub fn width(&self) -> f32 {
        self.max[0] - self.min[0]
    }

    pub fn height(&self) -> f32 {
        self.max[1] - self.min[1]
    }
}

/// Corner shape for rectangles
#[derive(Clone, Copy, Debug)]
pub enum CornerShape {
    /// No corner modification (sharp 90-degree corners)
    None,
    /// Circular arc rounding with specified radius
    Round(f32),
    /// Diagonal cut with specified distance from corner
    Cut(f32),
    /// Inverse circular arc (concave, like a ticket punch)
    InverseRound(f32),
    /// Squircle (superellipse) with specified radius and smoothness factor
    /// smoothness: 1.0 = circle, higher values = more square-like
    Squircle { radius: f32, smoothness: f32 },
}

impl CornerShape {
    /// Get the maximum distance this corner shape extends from the corner point
    pub fn extent(&self) -> f32 {
        match self {
            CornerShape::None => 0.0,
            CornerShape::Round(r) => *r,
            CornerShape::Cut(d) => *d,
            CornerShape::InverseRound(r) => *r,
            CornerShape::Squircle { radius, .. } => *radius,
        }
    }
}

/// Rectangle with customizable corner shapes, fill, and optional stroke
#[derive(Clone, Debug)]
pub struct StyledRect {
    pub rect: Rect,
    pub corner_shape: CornerShape,
    pub fill: Color,
    pub stroke: Option<Stroke>,
}

impl StyledRect {
    pub fn new(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            corner_shape: CornerShape::None,
            fill,
            stroke: None,
        }
    }

    pub fn with_corner_shape(mut self, corner_shape: CornerShape) -> Self {
        self.corner_shape = corner_shape;
        self
    }

    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Apply opacity by multiplying fill and stroke alpha values
    pub fn apply_opacity(&mut self, opacity: f32) {
        self.fill.a *= opacity;
        if let Some(stroke) = &mut self.stroke {
            stroke.color.a *= opacity;
        }
    }
}

/// Text shape for rendering text content
#[derive(Clone, Debug)]
pub struct TextShape {
    /// Bounding box where the text should be rendered
    pub rect: Rect,
    /// The text content to render
    pub text: String,
    /// Font size in pixels
    pub font_size: f32,
    /// Text color
    pub color: Color,
    /// Horizontal alignment
    pub h_align: HorizontalAlign,
    /// Vertical alignment
    pub v_align: VerticalAlign,
}

impl TextShape {
    /// Create a new text shape from text content and bounding rect
    pub fn new(rect: Rect, content: &TextContent) -> Self {
        Self {
            rect,
            text: content.text.clone(),
            font_size: content.font_size,
            color: content.color,
            h_align: content.h_align,
            v_align: content.v_align,
        }
    }

    /// Apply opacity by multiplying text color alpha
    pub fn apply_opacity(&mut self, opacity: f32) {
        self.color.a *= opacity;
    }
}

/// Shapes that can be rendered
#[derive(Clone, Debug)]
pub enum Shape {
    Rect(StyledRect),
    Text(TextShape),
    // Future: Circle, Line, Mesh, etc.
}

impl Shape {
    /// Apply opacity to this shape by multiplying all color alpha values
    pub fn apply_opacity(&mut self, opacity: f32) {
        match self {
            Shape::Rect(rect) => rect.apply_opacity(opacity),
            Shape::Text(text) => text.apply_opacity(opacity),
        }
    }
}

/// A shape with a clip rectangle
#[derive(Clone, Debug)]
pub struct ClippedShape {
    pub clip_rect: Rect,
    pub shape: Shape,
}

impl ClippedShape {
    pub fn new(clip_rect: Rect, shape: Shape) -> Self {
        Self { clip_rect, shape }
    }
}
