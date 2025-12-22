use crate::color::Color;
use crate::mesh::{Mesh, Vertex};
use crate::primitives::{ClippedShape, CornerShape, Shape, StyledRect};
use std::f32::consts::PI;

// NOTE: This file intentionally remains geometry-only. Text is rendered by backend crates.

/// Tessellator converts shapes into triangle meshes
pub struct Tessellator {
    mesh: Mesh,
}

impl Tessellator {
    pub fn new() -> Self {
        Self { mesh: Mesh::new() }
    }

    /// Tessellate shapes into a mesh. Returns a reference to the internal mesh.
    pub fn tessellate(&mut self, shapes: &[ClippedShape]) -> &Mesh {
        self.mesh.clear();

        for clipped in shapes {
            match &clipped.shape {
                Shape::Rect(r) => self.tessellate_rect(r),
                Shape::Text(_text) => {
                    // Text rendering is handled by the backend.
                    //
                    // IMPORTANT:
                    // - The backend needs the text glyph atlas + rasterization; emitting “a quad”
                    //   here would be misleading without UVs/texture sampling support.
                    // - Keeping the core tessellator geometry-only avoids coupling astra-gui
                    //   to any specific text rendering strategy.
                }
            }
        }

        &self.mesh
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
        let base_idx = self.mesh.vertices.len() as u32;

        // Add center vertex for triangle fan
        let center = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
        self.mesh.vertices.push(Vertex::new(center, rect.fill));

        // Generate vertices around the rectangle based on corner shape
        match rect.corner_shape {
            CornerShape::None => {
                // Simple rectangle - 4 corners
                self.mesh
                    .vertices
                    .push(Vertex::new([max_x, min_y], rect.fill)); // Top-right
                self.mesh
                    .vertices
                    .push(Vertex::new([max_x, max_y], rect.fill)); // Bottom-right
                self.mesh
                    .vertices
                    .push(Vertex::new([min_x, max_y], rect.fill)); // Bottom-left
                self.mesh
                    .vertices
                    .push(Vertex::new([min_x, min_y], rect.fill)); // Top-left
                self.mesh
                    .vertices
                    .push(Vertex::new([max_x, min_y], rect.fill)); // Close loop
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
        let vertex_count = self.mesh.vertices.len() as u32 - base_idx;
        for i in 1..(vertex_count - 1) {
            self.mesh.indices.push(base_idx);
            self.mesh.indices.push(base_idx + i);
            self.mesh.indices.push(base_idx + i + 1);
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
        // Corner centers and start angles for clockwise traversal
        // Each corner sweeps PI/2 radians
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
                self.mesh.vertices.push(Vertex::new([x, y], color));
            }
        }

        // Close the loop
        let first_perimeter =
            self.mesh.vertices[self.mesh.vertices.len() - (4 * (segments + 1)) as usize];
        self.mesh.vertices.push(first_perimeter);
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
        self.mesh
            .vertices
            .push(Vertex::new([max_x - cut, min_y], color));
        self.mesh
            .vertices
            .push(Vertex::new([max_x, min_y + cut], color));

        // Bottom-right corner
        self.mesh
            .vertices
            .push(Vertex::new([max_x, max_y - cut], color));
        self.mesh
            .vertices
            .push(Vertex::new([max_x - cut, max_y], color));

        // Bottom-left corner
        self.mesh
            .vertices
            .push(Vertex::new([min_x + cut, max_y], color));
        self.mesh
            .vertices
            .push(Vertex::new([min_x, max_y - cut], color));

        // Top-left corner
        self.mesh
            .vertices
            .push(Vertex::new([min_x, min_y + cut], color));
        self.mesh
            .vertices
            .push(Vertex::new([min_x + cut, min_y], color));

        // Close loop
        self.mesh
            .vertices
            .push(Vertex::new([max_x - cut, min_y], color));
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
        // For inverse round (concave corners), we need to create a shape that:
        // 1. Has straight edges that are inset by 'radius' from the original rectangle edges
        // 2. Has concave arcs at each corner that curve inward
        //
        // The perimeter goes: straight edge -> concave arc -> straight edge -> ...

        // Top edge: from (min_x + radius, min_y) to (max_x - radius, min_y)
        self.mesh
            .vertices
            .push(Vertex::new([max_x - radius, min_y], color));

        // Top-right concave arc: center at (max_x, min_y), sweeping from PI (left) to PI/2 (down)
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = PI + t * (-PI / 2.0); // PI to PI/2
            let x = max_x + radius * angle.cos();
            let y = min_y + radius * angle.sin();
            self.mesh.vertices.push(Vertex::new([x, y], color));
        }

        // Right edge: from (max_x, min_y + radius) to (max_x, max_y - radius)
        self.mesh
            .vertices
            .push(Vertex::new([max_x, max_y - radius], color));

        // Bottom-right concave arc: center at (max_x, max_y), sweeping from PI/2 (up) to 0 (right)
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = -PI / 2.0 + t * (-PI / 2.0); // PI/2 to 0
            let x = max_x + radius * angle.cos();
            let y = max_y + radius * angle.sin();
            self.mesh.vertices.push(Vertex::new([x, y], color));
        }

        // Bottom edge: from (max_x - radius, max_y) to (min_x + radius, max_y)
        self.mesh
            .vertices
            .push(Vertex::new([min_x + radius, max_y], color));

        // Bottom-left concave arc: center at (min_x, max_y), sweeping from 0 (right) to -PI/2 (down)
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = t * (-PI / 2.0); // 0 to -PI/2
            let x = min_x + radius * angle.cos();
            let y = max_y + radius * angle.sin();
            self.mesh.vertices.push(Vertex::new([x, y], color));
        }

        // Left edge: from (min_x, max_y - radius) to (min_x, min_y + radius)
        self.mesh
            .vertices
            .push(Vertex::new([min_x, min_y + radius], color));

        // Top-left concave arc: center at (min_x, min_y), sweeping from -PI/2 (down) to -PI (left)
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = PI / 2.0 + t * (-PI / 2.0); // -PI/2 to -PI
            let x = min_x + radius * angle.cos();
            let y = min_y + radius * angle.sin();
            self.mesh.vertices.push(Vertex::new([x, y], color));
        }

        // Close the loop - back to the start of top edge
        self.mesh
            .vertices
            .push(Vertex::new([max_x - radius, min_y], color));
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
        // n = 2 gives a circle, higher values give more square-like shapes
        let n = 2.0 + smoothness;

        // Use same corner centers and start angles as Round
        let corners = [
            (max_x - radius, min_y + radius, -PI / 2.0), // Top-right
            (max_x - radius, max_y - radius, 0.0),       // Bottom-right
            (min_x + radius, max_y - radius, PI / 2.0),  // Bottom-left
            (min_x + radius, min_y + radius, PI),        // Top-left
        ];

        for (cx, cy, start_angle) in corners {
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let angle = start_angle + t * (PI / 2.0);

                // Superellipse parametric form
                let cos_theta = angle.cos();
                let sin_theta = angle.sin();

                // Avoid division by zero at cardinal directions
                let cos_abs = cos_theta.abs().max(1e-10);
                let sin_abs = sin_theta.abs().max(1e-10);

                let r = radius / (cos_abs.powf(n) + sin_abs.powf(n)).powf(1.0 / n);

                let x = cx + r * cos_theta;
                let y = cy + r * sin_theta;
                self.mesh.vertices.push(Vertex::new([x, y], color));
            }
        }

        // Close the loop
        let first_perimeter =
            self.mesh.vertices[self.mesh.vertices.len() - (4 * (segments + 1)) as usize];
        self.mesh.vertices.push(first_perimeter);
    }

    fn add_rect_stroke(&mut self, rect: &StyledRect, stroke: &crate::primitives::Stroke) {
        let min_x = rect.rect.min[0];
        let min_y = rect.rect.min[1];
        let max_x = rect.rect.max[0];
        let max_y = rect.rect.max[1];

        let extent = rect
            .corner_shape
            .extent()
            .min((max_x - min_x) * 0.5)
            .min((max_y - min_y) * 0.5);
        let stroke_half_width = stroke.width * 0.5;

        let segments_per_corner = 8;
        let base_idx = self.mesh.vertices.len() as u32;

        match rect.corner_shape {
            CornerShape::None => {
                // Simple rectangle stroke - outer expands outward, inner shrinks inward
                // This centers the stroke on the rectangle edge
                let outer = [
                    [max_x + stroke_half_width, min_y - stroke_half_width],
                    [max_x + stroke_half_width, max_y + stroke_half_width],
                    [min_x - stroke_half_width, max_y + stroke_half_width],
                    [min_x - stroke_half_width, min_y - stroke_half_width],
                    [max_x + stroke_half_width, min_y - stroke_half_width],
                ];
                let inner = [
                    [max_x - stroke_half_width, min_y + stroke_half_width],
                    [max_x - stroke_half_width, max_y - stroke_half_width],
                    [min_x + stroke_half_width, max_y - stroke_half_width],
                    [min_x + stroke_half_width, min_y + stroke_half_width],
                    [max_x - stroke_half_width, min_y + stroke_half_width],
                ];

                for i in 0..5 {
                    self.mesh.vertices.push(Vertex::new(outer[i], stroke.color));
                    self.mesh.vertices.push(Vertex::new(inner[i], stroke.color));
                }
            }
            CornerShape::Round(_) => {
                let corners = [
                    (max_x - extent, min_y + extent, -PI / 2.0),
                    (max_x - extent, max_y - extent, 0.0),
                    (min_x + extent, max_y - extent, PI / 2.0),
                    (min_x + extent, min_y + extent, PI),
                ];

                for (cx, cy, start_angle) in corners {
                    for i in 0..=segments_per_corner {
                        let angle =
                            start_angle + (i as f32 / segments_per_corner as f32) * (PI / 2.0);
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();

                        let outer_x = cx + (extent + stroke_half_width) * cos_a;
                        let outer_y = cy + (extent + stroke_half_width) * sin_a;
                        self.mesh
                            .vertices
                            .push(Vertex::new([outer_x, outer_y], stroke.color));

                        let inner_x = cx + (extent - stroke_half_width).max(0.0) * cos_a;
                        let inner_y = cy + (extent - stroke_half_width).max(0.0) * sin_a;
                        self.mesh
                            .vertices
                            .push(Vertex::new([inner_x, inner_y], stroke.color));
                    }
                }

                let first_outer = self.mesh.vertices[base_idx as usize];
                let first_inner = self.mesh.vertices[(base_idx + 1) as usize];
                self.mesh.vertices.push(first_outer);
                self.mesh.vertices.push(first_inner);
            }
            CornerShape::Cut(_) => {
                // Stroke follows the cut corners
                // For each corner, we have 2 vertices on the cut diagonal
                // The stroke needs to expand outward along the normal to each edge
                // TODO: make it configurable with and angle of the cut
                // let angle = PI / 4.0;
                // let stroke_cut_offset =
                //     f32::tan((PI + angle) / 2.0 - PI / 2.0) * stroke.width / 2.0;
                let stroke_cut_offset = f32::tan(PI / 8.0) * stroke.width / 2.0;

                let cut = extent;

                // Top left outer
                self.mesh.vertices.push(Vertex::new(
                    [min_x + cut - stroke_cut_offset, min_y - stroke_half_width],
                    stroke.color,
                ));
                // Top left inner
                self.mesh.vertices.push(Vertex::new(
                    [min_x + cut + stroke_cut_offset, min_y + stroke_half_width],
                    stroke.color,
                ));

                // Top right outer
                self.mesh.vertices.push(Vertex::new(
                    [max_x - cut + stroke_cut_offset, min_y - stroke_half_width],
                    stroke.color,
                ));
                // Top right inner
                self.mesh.vertices.push(Vertex::new(
                    [max_x - cut - stroke_cut_offset, min_y + stroke_half_width],
                    stroke.color,
                ));

                // Right top outer
                self.mesh.vertices.push(Vertex::new(
                    [max_x + stroke_half_width, min_y + cut - stroke_cut_offset],
                    stroke.color,
                ));
                // Right top inner
                self.mesh.vertices.push(Vertex::new(
                    [max_x - stroke_half_width, min_y + cut + stroke_cut_offset],
                    stroke.color,
                ));

                // Right bottom outer
                self.mesh.vertices.push(Vertex::new(
                    [max_x + stroke_half_width, max_y - cut + stroke_cut_offset],
                    stroke.color,
                ));
                // Right bottom inner
                self.mesh.vertices.push(Vertex::new(
                    [max_x - stroke_half_width, max_y - cut - stroke_cut_offset],
                    stroke.color,
                ));

                // Bottom right outer
                self.mesh.vertices.push(Vertex::new(
                    [max_x - cut + stroke_cut_offset, max_y + stroke_half_width],
                    stroke.color,
                ));
                // Bottom right inner
                self.mesh.vertices.push(Vertex::new(
                    [max_x - cut - stroke_cut_offset, max_y - stroke_half_width],
                    stroke.color,
                ));

                // Bottom left outer
                self.mesh.vertices.push(Vertex::new(
                    [min_x + cut - stroke_cut_offset, max_y + stroke_half_width],
                    stroke.color,
                ));
                // Bottom left inner
                self.mesh.vertices.push(Vertex::new(
                    [min_x + cut + stroke_cut_offset, max_y - stroke_half_width],
                    stroke.color,
                ));

                // Left bottom outer
                self.mesh.vertices.push(Vertex::new(
                    [min_x - stroke_half_width, max_y - cut + stroke_cut_offset],
                    stroke.color,
                ));
                // Left bottom inner
                self.mesh.vertices.push(Vertex::new(
                    [min_x + stroke_half_width, max_y - cut - stroke_cut_offset],
                    stroke.color,
                ));

                // Left top outer
                self.mesh.vertices.push(Vertex::new(
                    [min_x - stroke_half_width, min_y + cut - stroke_cut_offset],
                    stroke.color,
                ));
                // Left top inner
                self.mesh.vertices.push(Vertex::new(
                    [min_x + stroke_half_width, min_y + cut + stroke_cut_offset],
                    stroke.color,
                ));

                // Close the loop - back to top edge start
                // Top left outer
                self.mesh.vertices.push(Vertex::new(
                    [min_x + cut - stroke_cut_offset, min_y - stroke_half_width],
                    stroke.color,
                ));
                // Top left inner
                self.mesh.vertices.push(Vertex::new(
                    [min_x + cut + stroke_cut_offset, min_y + stroke_half_width],
                    stroke.color,
                ));
            }
            CornerShape::InverseRound(_) => {
                // Stroke for inverse round corners
                // The stroke follows the perimeter: straight edge inset by extent -> concave arc -> repeat
                // For each segment, we need to create outer/inner pairs perpendicular to the path

                // Start at top edge
                let mut outer_vertices = Vec::new();
                let mut inner_vertices = Vec::new();

                // Top right outer
                let outer_vertex = Vertex::new(
                    [
                        max_x - extent + stroke_half_width,
                        min_y - stroke_half_width,
                    ],
                    stroke.color,
                );
                outer_vertices.push(outer_vertex);
                outer_vertices.push(outer_vertex);
                // Top right inner
                inner_vertices.push(Vertex::new(
                    [
                        max_x - extent - stroke_half_width,
                        min_y + stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Top right arc: center at (max_x, min_y), sweeping from PI (left) to -PI/2 (down)
                // The arc goes from (max_x - extent, min_y) to (max_x, min_y + extent)
                for i in 0..=segments_per_corner {
                    let t = i as f32 / segments_per_corner as f32;
                    let angle = PI + t * (-PI / 2.0);
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Center is at corner (max_x, min_y)
                    // Arc radius is extent
                    // Normal points away from center
                    let arc_x = max_x + extent * cos_a;
                    let arc_y = min_y + extent * sin_a;

                    // For concave arc, outer is further from center, inner is closer
                    outer_vertices.push(Vertex::new(
                        [
                            arc_x + stroke_half_width * cos_a,
                            arc_y + stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    ));

                    let inner_vertex = Vertex::new(
                        [
                            arc_x - stroke_half_width * cos_a,
                            arc_y - stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    );
                    inner_vertices.push(inner_vertex);
                    if i == 0 {
                        inner_vertices.push(inner_vertex);
                    }
                }

                // Right top outer
                outer_vertices.push(Vertex::new(
                    [
                        max_x + stroke_half_width,
                        min_y + extent - stroke_half_width,
                    ],
                    stroke.color,
                ));
                // Right top inner
                inner_vertices.push(Vertex::new(
                    [
                        max_x - stroke_half_width,
                        min_y + extent + stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Right bottom outer
                let outer_vertex = Vertex::new(
                    [
                        max_x + stroke_half_width,
                        max_y - extent + stroke_half_width,
                    ],
                    stroke.color,
                );
                outer_vertices.push(outer_vertex);
                outer_vertices.push(outer_vertex);
                // Right bottom inner
                inner_vertices.push(Vertex::new(
                    [
                        max_x - stroke_half_width,
                        max_y - extent - stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Bottom right arc: center at (max_x, max_y), sweeping from PI/2 (top) to PI (left)
                // The arc goes from (max_x, max_y - extent) to (max_x - extent, max_y)
                for i in 0..=segments_per_corner {
                    let t = i as f32 / segments_per_corner as f32;
                    let angle = -PI / 2.0 - t * PI / 2.0;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Center is at corner (max_x, max_y)
                    // Arc radius is extent
                    // Normal points away from center
                    let arc_x = max_x + extent * cos_a;
                    let arc_y = max_y + extent * sin_a;

                    // For concave arc, outer is further from center, inner is closer
                    outer_vertices.push(Vertex::new(
                        [
                            arc_x + stroke_half_width * cos_a,
                            arc_y + stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    ));

                    let inner_vertex = Vertex::new(
                        [
                            arc_x - stroke_half_width * cos_a,
                            arc_y - stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    );
                    inner_vertices.push(inner_vertex);
                    if i == 0 {
                        inner_vertices.push(inner_vertex);
                    }
                }

                // Bottom right outer
                outer_vertices.push(Vertex::new(
                    [
                        max_x - extent + stroke_half_width,
                        max_y + stroke_half_width,
                    ],
                    stroke.color,
                ));
                // Bottom right inner
                inner_vertices.push(Vertex::new(
                    [
                        max_x - extent - stroke_half_width,
                        max_y - stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Bottom left outer
                let outer_vertex = Vertex::new(
                    [
                        min_x + extent - stroke_half_width,
                        max_y + stroke_half_width,
                    ],
                    stroke.color,
                );
                outer_vertices.push(outer_vertex);
                outer_vertices.push(outer_vertex);
                // Bottom left inner
                inner_vertices.push(Vertex::new(
                    [
                        min_x + extent + stroke_half_width,
                        max_y - stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Bottom left arc: center at (min_x, max_y), sweeping from PI/2 (top) to PI (left)
                // The arc goes from (min_x + extent, max_y) to (min_x, max_y - extent)
                for i in 0..=segments_per_corner {
                    let t = i as f32 / segments_per_corner as f32;
                    let angle = -t * PI / 2.0;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Center is at corner (min_x, max_y)
                    // Arc radius is extent
                    // Normal points away from center
                    let arc_x = min_x + extent * cos_a;
                    let arc_y = max_y + extent * sin_a;

                    // For concave arc, outer is further from center, inner is closer
                    outer_vertices.push(Vertex::new(
                        [
                            arc_x + stroke_half_width * cos_a,
                            arc_y + stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    ));
                    let inner_vertex = Vertex::new(
                        [
                            arc_x - stroke_half_width * cos_a,
                            arc_y - stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    );
                    inner_vertices.push(inner_vertex);
                    if i == 0 {
                        inner_vertices.push(inner_vertex);
                    }
                }

                // Right bottom outer
                outer_vertices.push(Vertex::new(
                    [
                        min_x - stroke_half_width,
                        max_y - extent + stroke_half_width,
                    ],
                    stroke.color,
                ));
                // Right bottom inner
                inner_vertices.push(Vertex::new(
                    [
                        min_x + stroke_half_width,
                        max_y - extent - stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Right top outer
                let outer_vertex = Vertex::new(
                    [
                        min_x - stroke_half_width,
                        min_y + extent - stroke_half_width,
                    ],
                    stroke.color,
                );
                outer_vertices.push(outer_vertex);
                outer_vertices.push(outer_vertex);
                // Right top inner
                inner_vertices.push(Vertex::new(
                    [
                        min_x + stroke_half_width,
                        min_y + extent + stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Top left arc: center at (min_x, min_y), sweeping from PI/2 (top) to PI (left)
                // The arc goes from (min_x, min_y + extent) to (min_x + extent, min_y)
                for i in 0..=segments_per_corner {
                    let t = i as f32 / segments_per_corner as f32;
                    let angle = PI / 2.0 - t * PI / 2.0;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Center is at corner (min_x, min_y)
                    // Arc radius is extent
                    // Normal points away from center
                    let arc_x = min_x + extent * cos_a;
                    let arc_y = min_y + extent * sin_a;

                    // For concave arc, outer is further from center, inner is closer
                    outer_vertices.push(Vertex::new(
                        [
                            arc_x + stroke_half_width * cos_a,
                            arc_y + stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    ));
                    let inner_vertex = Vertex::new(
                        [
                            arc_x - stroke_half_width * cos_a,
                            arc_y - stroke_half_width * sin_a,
                        ],
                        stroke.color,
                    );
                    inner_vertices.push(inner_vertex);
                    if i == 0 {
                        inner_vertices.push(inner_vertex);
                    }
                }

                // Top right outer
                outer_vertices.push(Vertex::new(
                    [
                        min_x + extent - stroke_half_width,
                        min_y - stroke_half_width,
                    ],
                    stroke.color,
                ));
                inner_vertices.push(Vertex::new(
                    [
                        min_x + extent + stroke_half_width,
                        min_y + stroke_half_width,
                    ],
                    stroke.color,
                ));

                // Add vertices in quad strip order (outer, inner alternating)
                for i in 0..outer_vertices.len() {
                    self.mesh.vertices.push(outer_vertices[i]);
                    self.mesh.vertices.push(inner_vertices[i]);
                }

                // Close the loop
                self.mesh.vertices.push(outer_vertices[0]);
                self.mesh.vertices.push(inner_vertices[0]);
            }
            CornerShape::Squircle { smoothness, .. } => {
                // Stroke for squircle corners
                let n = 2.0 + smoothness;

                let corners = [
                    (max_x - extent, min_y + extent, -PI / 2.0),
                    (max_x - extent, max_y - extent, 0.0),
                    (min_x + extent, max_y - extent, PI / 2.0),
                    (min_x + extent, min_y + extent, PI),
                ];

                for (cx, cy, start_angle) in corners {
                    for i in 0..=segments_per_corner {
                        let t = i as f32 / segments_per_corner as f32;
                        let angle = start_angle + t * (PI / 2.0);

                        let cos_theta = angle.cos();
                        let sin_theta = angle.sin();

                        let cos_abs = cos_theta.abs().max(1e-10);
                        let sin_abs = sin_theta.abs().max(1e-10);

                        let base_r = extent / (cos_abs.powf(n) + sin_abs.powf(n)).powf(1.0 / n);

                        let outer_r = base_r + stroke_half_width;
                        let inner_r = (base_r - stroke_half_width).max(0.0);

                        let outer_x = cx + outer_r * cos_theta;
                        let outer_y = cy + outer_r * sin_theta;
                        self.mesh
                            .vertices
                            .push(Vertex::new([outer_x, outer_y], stroke.color));

                        let inner_x = cx + inner_r * cos_theta;
                        let inner_y = cy + inner_r * sin_theta;
                        self.mesh
                            .vertices
                            .push(Vertex::new([inner_x, inner_y], stroke.color));
                    }
                }

                let first_outer = self.mesh.vertices[base_idx as usize];
                let first_inner = self.mesh.vertices[(base_idx + 1) as usize];
                self.mesh.vertices.push(first_outer);
                self.mesh.vertices.push(first_inner);
            }
        }

        // Create quad strip indices
        let pairs = (self.mesh.vertices.len() as u32 - base_idx) / 2;
        for i in 0..(pairs - 1) {
            let idx = base_idx + i * 2;

            self.mesh.indices.push(idx);
            self.mesh.indices.push(idx + 1);
            self.mesh.indices.push(idx + 2);

            self.mesh.indices.push(idx + 1);
            self.mesh.indices.push(idx + 3);
            self.mesh.indices.push(idx + 2);
        }
    }
}

impl Default for Tessellator {
    fn default() -> Self {
        Self::new()
    }
}
