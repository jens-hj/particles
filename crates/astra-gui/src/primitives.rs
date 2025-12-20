/// RGBA color in linear space with values in [0, 1]
#[derive(Clone, Copy, Debug)]
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
}

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
#[derive(Clone, Copy, Debug)]
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

/// Rounded rectangle with fill and optional stroke
#[derive(Clone, Debug)]
pub struct RoundedRect {
    pub rect: Rect,
    pub rounding: f32,
    pub fill: Color,
    pub stroke: Option<Stroke>,
}

impl RoundedRect {
    pub fn new(rect: Rect, rounding: f32, fill: Color) -> Self {
        Self {
            rect,
            rounding,
            fill,
            stroke: None,
        }
    }

    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

/// Shapes that can be rendered
#[derive(Clone, Debug)]
pub enum Shape {
    RoundedRect(RoundedRect),
    // Future: Circle, Line, Mesh, etc.
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
