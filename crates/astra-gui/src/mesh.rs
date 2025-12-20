use crate::color::Color;

/// Vertex format for UI rendering with position and color
#[derive(Copy, Clone, Debug)]
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
}

/// A mesh consisting of vertices and indices for triangle rendering
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(index_capacity),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}
