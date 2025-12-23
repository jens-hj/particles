use astra_gui::Vertex;

/// WGPU-specific vertex format with Pod/Zeroable for buffer uploading
///
/// Uses u8 colors (Unorm8x4) instead of f32x4 to reduce vertex size from 24 bytes to 12 bytes.
/// This halves memory bandwidth requirements for geometry rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuVertex {
    pub pos: [f32; 2],  // 8 bytes
    pub color: [u8; 4], // 4 bytes (RGBA, normalized to 0-255)
}

impl From<Vertex> for WgpuVertex {
    fn from(vertex: Vertex) -> Self {
        // Convert f32 color components (0.0-1.0) to u8 (0-255)
        let color = [
            (vertex.color[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (vertex.color[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (vertex.color[2] * 255.0).round().clamp(0.0, 255.0) as u8,
            (vertex.color[3] * 255.0).round().clamp(0.0, 255.0) as u8,
        ];

        Self {
            pos: vertex.pos,
            color,
        }
    }
}

impl WgpuVertex {
    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: &[wgpu::VertexAttribute] = &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Unorm8x4, // u8x4 normalized to 0.0-1.0
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
