use astra_gui::{ClippedShape, Color, RoundedRect, Shape};
use std::f32::consts::PI;

/// Vertex format for UI rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn new(pos: [f32; 2], color: Color) -> Self {
        Self {
            pos,
            color: [color.r, color.g, color.b, color.a],
        }
    }

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
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

/// Tessellator converts shapes into vertices and indices
pub struct Tessellator {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Tessellator {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn tessellate(&mut self, shapes: &[ClippedShape]) -> (&[Vertex], &[u32]) {
        self.vertices.clear();
        self.indices.clear();

        for clipped in shapes {
            match &clipped.shape {
                Shape::RoundedRect(r) => self.tessellate_rounded_rect(r),
            }
        }

        (&self.vertices, &self.indices)
    }

    fn tessellate_rounded_rect(&mut self, rect: &RoundedRect) {
        let base_idx = self.vertices.len() as u32;

        // Tessellate fill
        if rect.fill.a > 0.0 {
            self.add_rounded_rect_fill(rect, base_idx);
        }

        // Tessellate stroke
        if let Some(stroke) = &rect.stroke {
            if stroke.width > 0.0 && stroke.color.a > 0.0 {
                self.add_rounded_rect_stroke(rect, stroke);
            }
        }
    }

    fn add_rounded_rect_fill(&mut self, rect: &RoundedRect, _base_idx: u32) {
        let min_x = rect.rect.min[0];
        let min_y = rect.rect.min[1];
        let max_x = rect.rect.max[0];
        let max_y = rect.rect.max[1];
        let radius = rect
            .rounding
            .min((max_x - min_x) * 0.5)
            .min((max_y - min_y) * 0.5);

        // Number of segments per corner (more = smoother corners)
        let segments_per_corner = 8;
        let base_idx = self.vertices.len() as u32;

        // Add center vertex
        let center = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
        self.vertices.push(Vertex::new(center, rect.fill));

        // Generate vertices around the rounded rectangle
        // We'll create a triangle fan from the center

        let mut add_corner = |cx: f32, cy: f32, start_angle: f32| {
            for i in 0..=segments_per_corner {
                let angle = start_angle + (i as f32 / segments_per_corner as f32) * (PI / 2.0);
                let x = cx + radius * angle.cos();
                let y = cy + radius * angle.sin();
                self.vertices.push(Vertex::new([x, y], rect.fill));
            }
        };

        // Top-right corner (starting from top edge)
        add_corner(max_x - radius, min_y + radius, -PI / 2.0);

        // Bottom-right corner
        add_corner(max_x - radius, max_y - radius, 0.0);

        // Bottom-left corner
        add_corner(min_x + radius, max_y - radius, PI / 2.0);

        // Top-left corner
        add_corner(min_x + radius, min_y + radius, PI);

        // Close the loop by adding the first perimeter vertex again
        let first_perimeter_idx = base_idx + 1;
        let first_vertex = self.vertices[first_perimeter_idx as usize];
        self.vertices.push(first_vertex);

        // Create triangle fan indices
        let vertex_count = self.vertices.len() as u32 - base_idx;
        for i in 1..(vertex_count - 1) {
            self.indices.push(base_idx);
            self.indices.push(base_idx + i);
            self.indices.push(base_idx + i + 1);
        }
    }

    fn add_rounded_rect_stroke(&mut self, rect: &RoundedRect, stroke: &astra_gui::Stroke) {
        let min_x = rect.rect.min[0];
        let min_y = rect.rect.min[1];
        let max_x = rect.rect.max[0];
        let max_y = rect.rect.max[1];
        let radius = rect
            .rounding
            .min((max_x - min_x) * 0.5)
            .min((max_y - min_y) * 0.5);
        let half_width = stroke.width * 0.5;

        let segments_per_corner = 8;
        let base_idx = self.vertices.len() as u32;

        // Generate outer and inner paths for the stroke
        let mut add_corner_stroke = |cx: f32, cy: f32, start_angle: f32| {
            for i in 0..=segments_per_corner {
                let angle = start_angle + (i as f32 / segments_per_corner as f32) * (PI / 2.0);
                let cos_a = angle.cos();
                let sin_a = angle.sin();

                // Outer vertex
                let outer_x = cx + (radius + half_width) * cos_a;
                let outer_y = cy + (radius + half_width) * sin_a;
                self.vertices
                    .push(Vertex::new([outer_x, outer_y], stroke.color));

                // Inner vertex
                let inner_x = cx + (radius - half_width).max(0.0) * cos_a;
                let inner_y = cy + (radius - half_width).max(0.0) * sin_a;
                self.vertices
                    .push(Vertex::new([inner_x, inner_y], stroke.color));
            }
        };

        // Add all corners
        add_corner_stroke(max_x - radius, min_y + radius, -PI / 2.0);
        add_corner_stroke(max_x - radius, max_y - radius, 0.0);
        add_corner_stroke(min_x + radius, max_y - radius, PI / 2.0);
        add_corner_stroke(min_x + radius, min_y + radius, PI);

        // Close the loop
        let first_outer = self.vertices[base_idx as usize];
        let first_inner = self.vertices[(base_idx + 1) as usize];
        self.vertices.push(first_outer);
        self.vertices.push(first_inner);

        // Create quad strip indices
        let pairs = (self.vertices.len() as u32 - base_idx) / 2;
        for i in 0..(pairs - 1) {
            let idx = base_idx + i * 2;

            // First triangle of quad
            self.indices.push(idx);
            self.indices.push(idx + 1);
            self.indices.push(idx + 2);

            // Second triangle of quad
            self.indices.push(idx + 1);
            self.indices.push(idx + 3);
            self.indices.push(idx + 2);
        }
    }
}

impl Default for Tessellator {
    fn default() -> Self {
        Self::new()
    }
}
