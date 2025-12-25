use astra_gui::{CornerShape, StyledRect};

/// Instance data for SDF-based rectangle rendering.
///
/// Each instance represents a single rectangle with all the parameters needed
/// to render it using signed distance fields (SDFs) in the fragment shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectInstance {
    /// Center position in screen-space pixels
    pub center: [f32; 2],
    /// Half-size (width/2, height/2) in pixels
    pub half_size: [f32; 2],
    /// Fill color (RGBA, normalized to 0-255)
    pub fill_color: [u8; 4],
    /// Stroke color (RGBA, normalized to 0-255)
    pub stroke_color: [u8; 4],
    /// Stroke width in pixels (0 = no stroke)
    pub stroke_width: f32,
    /// Corner type: 0=None, 1=Round, 2=Cut, 3=InverseRound, 4=Squircle
    pub corner_type: u32,
    /// First corner parameter (radius or extent)
    pub corner_param1: f32,
    /// Second corner parameter (smoothness for squircle, unused for others)
    pub corner_param2: f32,
    /// Padding for 16-byte alignment
    pub _padding: [u32; 2],
}

impl RectInstance {
    /// Vertex buffer layout for instance attributes
    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: &[wgpu::VertexAttribute] = &[
            // center: vec2<f32> at location 1
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            // half_size: vec2<f32> at location 2
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
            // fill_color: vec4<f32> at location 3 (Unorm8x4)
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                shader_location: 3,
                format: wgpu::VertexFormat::Unorm8x4,
            },
            // stroke_color: vec4<f32> at location 4 (Unorm8x4)
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2 + std::mem::size_of::<[u8; 4]>())
                    as wgpu::BufferAddress,
                shader_location: 4,
                format: wgpu::VertexFormat::Unorm8x4,
            },
            // stroke_width: f32 at location 5
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2 + std::mem::size_of::<[u8; 4]>() * 2)
                    as wgpu::BufferAddress,
                shader_location: 5,
                format: wgpu::VertexFormat::Float32,
            },
            // corner_type: u32 at location 6
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2
                    + std::mem::size_of::<[u8; 4]>() * 2
                    + std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                shader_location: 6,
                format: wgpu::VertexFormat::Uint32,
            },
            // corner_param1: f32 at location 7
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2
                    + std::mem::size_of::<[u8; 4]>() * 2
                    + std::mem::size_of::<f32>()
                    + std::mem::size_of::<u32>()) as wgpu::BufferAddress,
                shader_location: 7,
                format: wgpu::VertexFormat::Float32,
            },
            // corner_param2: f32 at location 8
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2
                    + std::mem::size_of::<[u8; 4]>() * 2
                    + std::mem::size_of::<f32>() * 2
                    + std::mem::size_of::<u32>()) as wgpu::BufferAddress,
                shader_location: 8,
                format: wgpu::VertexFormat::Float32,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}

impl From<&StyledRect> for RectInstance {
    fn from(rect: &StyledRect) -> Self {
        // Calculate center and half-size
        let center = [
            (rect.rect.min[0] + rect.rect.max[0]) * 0.5,
            (rect.rect.min[1] + rect.rect.max[1]) * 0.5,
        ];
        let half_size = [
            (rect.rect.max[0] - rect.rect.min[0]) * 0.5,
            (rect.rect.max[1] - rect.rect.min[1]) * 0.5,
        ];

        // Convert fill color
        let fill_color = [
            (rect.fill.r * 255.0).round().clamp(0.0, 255.0) as u8,
            (rect.fill.g * 255.0).round().clamp(0.0, 255.0) as u8,
            (rect.fill.b * 255.0).round().clamp(0.0, 255.0) as u8,
            (rect.fill.a * 255.0).round().clamp(0.0, 255.0) as u8,
        ];

        // Convert stroke (if present)
        let (stroke_color, stroke_width) = if let Some(stroke) = &rect.stroke {
            (
                [
                    (stroke.color.r * 255.0).round().clamp(0.0, 255.0) as u8,
                    (stroke.color.g * 255.0).round().clamp(0.0, 255.0) as u8,
                    (stroke.color.b * 255.0).round().clamp(0.0, 255.0) as u8,
                    (stroke.color.a * 255.0).round().clamp(0.0, 255.0) as u8,
                ],
                stroke.width,
            )
        } else {
            ([0, 0, 0, 0], 0.0)
        };

        // Convert corner shape to type + parameters
        let (corner_type, param1, param2) = match rect.corner_shape {
            CornerShape::None => (0, 0.0, 0.0),
            CornerShape::Round(radius) => (1, radius, 0.0),
            CornerShape::Cut(distance) => (2, distance, 0.0),
            CornerShape::InverseRound(radius) => (3, radius, 0.0),
            CornerShape::Squircle { radius, smoothness } => (4, radius, smoothness),
        };

        Self {
            center,
            half_size,
            fill_color,
            stroke_color,
            stroke_width,
            corner_type,
            corner_param1: param1,
            corner_param2: param2,
            _padding: [0, 0],
        }
    }
}
