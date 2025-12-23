use crate::layout::Overflow;
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
        //
        // We derive `clip_rect` from the node tree's overflow policy:
        // - If any ancestor has `Overflow::Hidden` (or `Scroll`, for now), shapes are clipped to the
        //   intersection of those ancestor rects.
        // - If all ancestors are `Overflow::Visible`, the clip rect remains the full window rect.
        let window_rect = Rect::new([0.0, 0.0], [window_size.0, window_size.1]);

        let mut raw_shapes = Vec::new();
        collect_clipped_shapes(&root, window_rect, window_rect, &mut raw_shapes);

        let shapes = raw_shapes
            .into_iter()
            .map(|(rect, clip_rect, shape)| {
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

// Recursively walk the node tree to associate a clip rect with each collected shape.
fn collect_clipped_shapes(
    node: &Node,
    window_rect: Rect,
    inherited_clip_rect: Rect,
    out: &mut Vec<(Rect, Rect, Shape)>,
) {
    let Some(layout) = node.computed_layout() else {
        return;
    };

    let node_rect = layout.rect;

    // Update effective clip rect based on this node's overflow policy.
    let effective_clip_rect = match node.overflow {
        Overflow::Visible => inherited_clip_rect,
        Overflow::Hidden | Overflow::Scroll => intersect_rect(inherited_clip_rect, node_rect),
    };

    // If a node is fully clipped out, we can early-out (and skip its subtree).
    if is_empty_rect(effective_clip_rect) {
        return;
    }

    if let Some(shape) = &node.shape {
        out.push((node_rect, effective_clip_rect, shape.clone()));
    }

    if let Some(content) = &node.content {
        match content {
            crate::content::Content::Text(text_content) => {
                // Content uses the node's content rect (after padding) as its bounding box,
                // but still inherits the node/ancestor clip rect.
                let content_rect = Rect::new(
                    [
                        node_rect.min[0] + node.padding.left,
                        node_rect.min[1] + node.padding.top,
                    ],
                    [
                        node_rect.max[0] - node.padding.right,
                        node_rect.max[1] - node.padding.bottom,
                    ],
                );
                out.push((
                    node_rect,
                    effective_clip_rect,
                    Shape::Text(crate::primitives::TextShape::new(
                        content_rect,
                        text_content,
                    )),
                ));
            }
        }
    }

    for child in &node.children {
        collect_clipped_shapes(child, window_rect, effective_clip_rect, out);
    }
}

fn intersect_rect(a: Rect, b: Rect) -> Rect {
    Rect::new(
        [a.min[0].max(b.min[0]), a.min[1].max(b.min[1])],
        [a.max[0].min(b.max[0]), a.max[1].min(b.max[1])],
    )
}

fn is_empty_rect(r: Rect) -> bool {
    r.max[0] <= r.min[0] || r.max[1] <= r.min[1]
}
