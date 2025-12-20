use astra_gui::{ClippedShape, Color, CornerShape, Shape, StyledRect};
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
                Shape::Rect(r) => self.tessellate_rect(r),
            }
        }

        (&self.vertices, &self.indices)
    }

    fn tessellate_rect(&mut self, rect: &StyledRect) {
        // Tessellate fill
        if rect.fill.a > 0.0 {
            self.add_rect_fill(rect);
        }

        // Tessellate stroke
        if let Some(stroke) = &rect.stroke {
            if stroke.width > 0.0 && stroke.color.a > 0.0 {
                self.add_rect_stroke(rect, stroke);
            }
        }
    }

    fn add_rect_fill(&mut self, rect: &StyledRect) {
        let min_x = rect.rect.min[0];
        let min_y = rect.rect.min[1];
        let max_x = rect.rect.max[0];
        let max_y = rect.rect.max[1];

        let extent = rect
            .corner_shape
            .extent()
            .min((max_x - min_x) * 0.5)
            .min((max_y - min_y) * 0.5);

        // Number of segments per corner (more = smoother corners)
        let segments_per_corner = 8;
        let base_idx = self.vertices.len() as u32;

        // Add center vertex for triangle fan
        let center = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
        self.vertices.push(Vertex::new(center, rect.fill));

        // Generate vertices around the rectangle based on corner shape
        match rect.corner_shape {
            CornerShape::None => {
                // Simple rectangle - 4 corners
                self.vertices.push(Vertex::new([max_x, min_y], rect.fill)); // Top-right
                self.vertices.push(Vertex::new([max_x, max_y], rect.fill)); // Bottom-right
                self.vertices.push(Vertex::new([min_x, max_y], rect.fill)); // Bottom-left
                self.vertices.push(Vertex::new([min_x, min_y], rect.fill)); // Top-left
                self.vertices.push(Vertex::new([max_x, min_y], rect.fill)); // Close loop
            }
            CornerShape::Round(_) => {
                self.add_corner_vertices_round(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    extent,
                    segments_per_corner,
                    rect.fill,
                );
            }
            CornerShape::Cut(_) => {
                self.add_corner_vertices_cut(min_x, min_y, max_x, max_y, extent, rect.fill);
            }
            CornerShape::InverseRound(_) => {
                self.add_corner_vertices_inverse_round(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    extent,
                    segments_per_corner,
                    rect.fill,
                );
            }
            CornerShape::Squircle { smoothness, .. } => {
                self.add_corner_vertices_squircle(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    extent,
                    smoothness,
                    segments_per_corner,
                    rect.fill,
                );
            }
        }

        // Create triangle fan indices
        let vertex_count = self.vertices.len() as u32 - base_idx;
        for i in 1..(vertex_count - 1) {
            self.indices.push(base_idx);
            self.indices.push(base_idx + i);
            self.indices.push(base_idx + i + 1);
        }
    }

    fn add_corner_vertices_round(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        radius: f32,
        segments: u32,
        color: Color,
    ) {
        let corners = [
            (max_x - radius, min_y + radius, -PI / 2.0), // Top-right
            (max_x - radius, max_y - radius, 0.0),       // Bottom-right
            (min_x + radius, max_y - radius, PI / 2.0),  // Bottom-left
            (min_x + radius, min_y + radius, PI),        // Top-left
        ];

        for (cx, cy, start_angle) in corners {
            for i in 0..=segments {
                let angle = start_angle + (i as f32 / segments as f32) * (PI / 2.0);
                let x = cx + radius * angle.cos();
                let y = cy + radius * angle.sin();
                self.vertices.push(Vertex::new([x, y], color));
            }
        }

        // Close the loop
        let first_perimeter = self.vertices[self.vertices.len() - (4 * (segments + 1)) as usize];
        self.vertices.push(first_perimeter);
    }

    fn add_corner_vertices_cut(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        cut: f32,
        color: Color,
    ) {
        // Each corner has 2 vertices (the two points where the cut intersects the edges)
        // Top-right corner
        self.vertices.push(Vertex::new([max_x - cut, min_y], color));
        self.vertices.push(Vertex::new([max_x, min_y + cut], color));

        // Bottom-right corner
        self.vertices.push(Vertex::new([max_x, max_y - cut], color));
        self.vertices.push(Vertex::new([max_x - cut, max_y], color));

        // Bottom-left corner
        self.vertices.push(Vertex::new([min_x + cut, max_y], color));
        self.vertices.push(Vertex::new([min_x, max_y - cut], color));

        // Top-left corner
        self.vertices.push(Vertex::new([min_x, min_y + cut], color));
        self.vertices.push(Vertex::new([min_x + cut, min_y], color));

        // Close loop
        self.vertices.push(Vertex::new([max_x - cut, min_y], color));
    }

    fn add_corner_vertices_inverse_round(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        radius: f32,
        segments: u32,
        color: Color,
    ) {
        let corners = [
            (max_x, min_y, PI, -PI / 2.0),  // Top-right (inverse)
            (max_x, max_y, -PI / 2.0, 0.0), // Bottom-right (inverse)
            (min_x, max_y, 0.0, PI / 2.0),  // Bottom-left (inverse)
            (min_x, min_y, PI / 2.0, PI),   // Top-left (inverse)
        ];

        for (cx, cy, start_angle, end_angle) in corners {
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let angle = start_angle + t * (end_angle - start_angle);
                let x = cx + radius * angle.cos();
                let y = cy + radius * angle.sin();
                self.vertices.push(Vertex::new([x, y], color));
            }
        }

        // Close the loop
        let first_perimeter = self.vertices[self.vertices.len() - (4 * (segments + 1)) as usize];
        self.vertices.push(first_perimeter);
    }

    fn add_corner_vertices_squircle(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        radius: f32,
        smoothness: f32,
        segments: u32,
        color: Color,
    ) {
        // Squircle using superellipse formula: |x|^n + |y|^n = r^n
        let n = 2.0 + smoothness;

        let corners = [
            (max_x - radius, min_y + radius, 0), // Top-right
            (max_x - radius, max_y - radius, 1), // Bottom-right
            (min_x + radius, max_y - radius, 2), // Bottom-left
            (min_x + radius, min_y + radius, 3), // Top-left
        ];

        for (cx, cy, quadrant) in corners {
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let angle = (quadrant as f32 * PI / 2.0) + t * (PI / 2.0);

                // Superellipse parametric form
                let cos_theta = angle.cos();
                let sin_theta = angle.sin();
                let r = radius / (cos_theta.abs().powf(n) + sin_theta.abs().powf(n)).powf(1.0 / n);

                let x = cx + r * cos_theta;
                let y = cy + r * sin_theta;
                self.vertices.push(Vertex::new([x, y], color));
            }
        }

        // Close the loop
        let first_perimeter = self.vertices[self.vertices.len() - (4 * (segments + 1)) as usize];
        self.vertices.push(first_perimeter);
    }

    fn add_rect_stroke(&mut self, rect: &StyledRect, stroke: &astra_gui::Stroke) {
        // For now, implement stroke for Round corners only
        // Other corner shapes will follow the same pattern
        let min_x = rect.rect.min[0];
        let min_y = rect.rect.min[1];
        let max_x = rect.rect.max[0];
        let max_y = rect.rect.max[1];

        let extent = rect
            .corner_shape
            .extent()
            .min((max_x - min_x) * 0.5)
            .min((max_y - min_y) * 0.5);
        let half_width = stroke.width * 0.5;

        let segments_per_corner = 8;
        let base_idx = self.vertices.len() as u32;

        match rect.corner_shape {
            CornerShape::None => {
                // Simple rectangle stroke - outer and inner quad strip
                let outer = [
                    [max_x, min_y],
                    [max_x, max_y],
                    [min_x, max_y],
                    [min_x, min_y],
                    [max_x, min_y],
                ];
                let inner = [
                    [max_x - half_width, min_y + half_width],
                    [max_x - half_width, max_y - half_width],
                    [min_x + half_width, max_y - half_width],
                    [min_x + half_width, min_y + half_width],
                    [max_x - half_width, min_y + half_width],
                ];

                for i in 0..5 {
                    self.vertices.push(Vertex::new(outer[i], stroke.color));
                    self.vertices.push(Vertex::new(inner[i], stroke.color));
                }
            }
            CornerShape::Round(radius) => {
                let corners = [
                    (max_x - radius, min_y + radius, -PI / 2.0),
                    (max_x - radius, max_y - radius, 0.0),
                    (min_x + radius, max_y - radius, PI / 2.0),
                    (min_x + radius, min_y + radius, PI),
                ];

                for (cx, cy, start_angle) in corners {
                    for i in 0..=segments_per_corner {
                        let angle =
                            start_angle + (i as f32 / segments_per_corner as f32) * (PI / 2.0);
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();

                        let outer_x = cx + (extent + half_width) * cos_a;
                        let outer_y = cy + (extent + half_width) * sin_a;
                        self.vertices
                            .push(Vertex::new([outer_x, outer_y], stroke.color));

                        let inner_x = cx + (extent - half_width).max(0.0) * cos_a;
                        let inner_y = cy + (extent - half_width).max(0.0) * sin_a;
                        self.vertices
                            .push(Vertex::new([inner_x, inner_y], stroke.color));
                    }
                }

                let first_outer = self.vertices[base_idx as usize];
                let first_inner = self.vertices[(base_idx + 1) as usize];
                self.vertices.push(first_outer);
                self.vertices.push(first_inner);
            }
            _ => {
                // For other corner shapes, use simple stroke for now
                // TODO: Implement proper stroke for Cut, InverseRound, Squircle
                let outer = [
                    [max_x, min_y],
                    [max_x, max_y],
                    [min_x, max_y],
                    [min_x, min_y],
                    [max_x, min_y],
                ];
                let inner = [
                    [max_x - half_width, min_y + half_width],
                    [max_x - half_width, max_y - half_width],
                    [min_x + half_width, max_y - half_width],
                    [min_x + half_width, min_y + half_width],
                    [max_x - half_width, min_y + half_width],
                ];

                for i in 0..5 {
                    self.vertices.push(Vertex::new(outer[i], stroke.color));
                    self.vertices.push(Vertex::new(inner[i], stroke.color));
                }
            }
        }

        // Create quad strip indices
        let pairs = (self.vertices.len() as u32 - base_idx) / 2;
        for i in 0..(pairs - 1) {
            let idx = base_idx + i * 2;

            self.indices.push(idx);
            self.indices.push(idx + 1);
            self.indices.push(idx + 2);

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
