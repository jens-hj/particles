use astra_gui::Vertex;

/// WGPU-specific vertex format with Pod/Zeroable for buffer uploading
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

impl From<Vertex> for WgpuVertex {
    fn from(vertex: Vertex) -> Self {
        Self {
            pos: vertex.pos,
            color: vertex.color,
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
                format: wgpu::VertexFormat::Float32x4,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
