use crate::node::Node;
use crate::primitives::{ClippedShape, Rect, Shape};

/// Output from the UI system containing all shapes to render
#[derive(Clone, Debug, Default)]
pub struct FullOutput {
    pub shapes: Vec<ClippedShape>,
}

impl FullOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_shapes(shapes: Vec<ClippedShape>) -> Self {
        Self { shapes }
    }

    /// Create output from a node tree
    ///
    /// `window_size` is the (width, height) of the window
    pub fn from_node(root: Node, window_size: (f32, f32)) -> Self {
        Self::from_node_with_debug(root, window_size, None)
    }

    /// Create output from a node tree with optional debug visualization
    ///
    /// `window_size` is the (width, height) of the window
    /// `debug_options` configures which debug visualizations to show
    pub fn from_node_with_debug(
        mut root: Node,
        window_size: (f32, f32),
        debug_options: Option<crate::debug::DebugOptions>,
    ) -> Self {
        // Compute layout starting from the full window
        let window_rect = Rect::new([0.0, 0.0], [window_size.0, window_size.1]);
        root.compute_layout(window_rect);

        // Collect all shapes
        let mut collected_shapes = Vec::new();
        root.collect_shapes(&mut collected_shapes);

        // Add debug shapes if enabled
        if let Some(options) = debug_options {
            if options.is_enabled() {
                root.collect_debug_shapes(&mut collected_shapes, &options);
            }
        }

        // Convert to ClippedShapes
        let shapes = collected_shapes
            .into_iter()
            .map(|(rect, shape)| {
                // For now, use the node's rect as the clip rect
                let clip_rect = rect;

                // Apply the rect to the shape if it's a StyledRect.
                // Text already carries its own bounding rect internally (TextShape::rect).
                let shape_with_rect = match shape {
                    Shape::Rect(mut styled_rect) => {
                        styled_rect.rect = rect;
                        Shape::Rect(styled_rect)
                    }
                    Shape::Text(text_shape) => Shape::Text(text_shape),
                };

                ClippedShape::new(clip_rect, shape_with_rect)
            })
            .collect();

        Self { shapes }
    }
}
