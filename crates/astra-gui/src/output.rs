use crate::layout::Overflow;
use crate::measure::ContentMeasurer;
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
        root: Node,
        window_size: (f32, f32),
        debug_options: Option<crate::debug::DebugOptions>,
    ) -> Self {
        Self::from_node_with_debug_and_measurer(root, window_size, debug_options, None)
    }

    /// Create output from a node tree with optional debug visualization and measurer
    ///
    /// `window_size` is the (width, height) of the window
    /// `debug_options` configures which debug visualizations to show
    /// `measurer` enables `Size::FitContent` to resolve to intrinsic content size
    pub fn from_node_with_debug_and_measurer(
        mut root: Node,
        window_size: (f32, f32),
        debug_options: Option<crate::debug::DebugOptions>,
        measurer: Option<&mut dyn ContentMeasurer>,
    ) -> Self {
        // Compute layout starting from the full window
        let window_rect = Rect::new([0.0, 0.0], [window_size.0, window_size.1]);

        if let Some(m) = measurer {
            root.compute_layout_with_measurer(window_rect, m);
        } else {
            root.compute_layout(window_rect);
        }

        // Convert to ClippedShapes (including optional debug shapes), with overflow-aware clip rects.
        //
        // We derive `clip_rect` from the node tree's overflow policy:
        // - If any ancestor has `Overflow::Hidden` (or `Scroll`, for now), shapes are clipped to the
        //   intersection of those ancestor rects.
        // - If all ancestors are `Overflow::Visible`, the clip rect remains the full window rect.
        let mut raw_shapes = Vec::new();
        collect_clipped_shapes(
            &root,
            window_rect,
            window_rect,
            debug_options,
            &mut raw_shapes,
        );

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
    debug_options: Option<crate::debug::DebugOptions>,
    out: &mut Vec<(Rect, Rect, Shape)>,
) {
    collect_clipped_shapes_with_opacity(
        node,
        window_rect,
        inherited_clip_rect,
        debug_options,
        out,
        1.0,
    );
}

// Recursively walk the node tree with cumulative opacity.
fn collect_clipped_shapes_with_opacity(
    node: &Node,
    window_rect: Rect,
    inherited_clip_rect: Rect,
    debug_options: Option<crate::debug::DebugOptions>,
    out: &mut Vec<(Rect, Rect, Shape)>,
    parent_opacity: f32,
) {
    let combined_opacity = parent_opacity * node.opacity();

    // Skip rendering if fully transparent
    if combined_opacity <= 0.0 {
        return;
    }

    let Some(layout) = node.computed_layout() else {
        return;
    };

    let node_rect = layout.rect;

    // Update effective clip rect based on this node's overflow policy.
    let effective_clip_rect = match node.overflow() {
        Overflow::Visible => inherited_clip_rect,
        Overflow::Hidden | Overflow::Scroll => intersect_rect(inherited_clip_rect, node_rect),
    };

    // If a node is fully clipped out, we can early-out (and skip its subtree).
    if is_empty_rect(effective_clip_rect) {
        return;
    }

    // Background shape (if any)
    // The node's own shape uses the inherited clip rect (from parent), not effective_clip_rect.
    // This ensures the container's border/background is not clipped by its own overflow policy.
    if let Some(shape) = node.shape() {
        let mut shape_with_opacity = shape.clone();
        // OPTIMIZATION: Only apply opacity if it's less than 1.0 (most nodes have opacity = 1.0)
        if combined_opacity < 1.0 {
            shape_with_opacity.apply_opacity(combined_opacity);
        }
        out.push((node_rect, inherited_clip_rect, shape_with_opacity));
    }

    // Content (if any)
    if let Some(content) = node.content() {
        match content {
            crate::content::Content::Text(text_content) => {
                // Content uses the node's content rect (after padding) as its bounding box,
                // but still inherits the node/ancestor clip rect.
                let padding = node.padding();
                let content_rect = Rect::new(
                    [
                        node_rect.min[0] + padding.left,
                        node_rect.min[1] + padding.top,
                    ],
                    [
                        node_rect.max[0] - padding.right,
                        node_rect.max[1] - padding.bottom,
                    ],
                );
                let mut text_shape = crate::primitives::TextShape::new(content_rect, text_content);
                // OPTIMIZATION: Only apply opacity if it's less than 1.0
                if combined_opacity < 1.0 {
                    text_shape.apply_opacity(combined_opacity);
                }
                out.push((node_rect, effective_clip_rect, Shape::Text(text_shape)));
            }
        }
    }

    // Debug overlays (if enabled) must also be overflow-clipped consistently.
    if let Some(options) = debug_options {
        if options.is_enabled() {
            collect_debug_shapes_clipped(node, node_rect, effective_clip_rect, &options, out);
        }
    }

    // Collect gap debug shapes between children
    if let Some(options) = debug_options {
        if options.show_gaps && node.gap() > 0.0 {
            collect_gap_debug_shapes(node, effective_clip_rect, &options, out);
        }
    }

    for child in node.children() {
        collect_clipped_shapes_with_opacity(
            child,
            window_rect,
            effective_clip_rect,
            debug_options,
            out,
            combined_opacity,
        );
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

fn collect_debug_shapes_clipped(
    node: &Node,
    node_rect: Rect,
    clip_rect: Rect,
    options: &crate::debug::DebugOptions,
    out: &mut Vec<(Rect, Rect, Shape)>,
) {
    use crate::color::Color;
    use crate::primitives::{Stroke, StyledRect};

    let margin = node.margin();
    let padding = node.padding();

    // Draw margin area (outermost, semi-transparent red showing margin space)
    if options.show_margins
        && (margin.top > 0.0 || margin.right > 0.0 || margin.bottom > 0.0 || margin.left > 0.0)
    {
        // Draw top margin
        if margin.top > 0.0 {
            out.push((
                Rect::new(
                    [
                        node_rect.min[0] - margin.left,
                        node_rect.min[1] - margin.top,
                    ],
                    [node_rect.max[0] + margin.right, node_rect.min[1]],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(1.0, 0.0, 0.0, 0.2),
                )),
            ));
        }
        // Draw right margin (excluding top and bottom corners)
        if margin.right > 0.0 {
            out.push((
                Rect::new(
                    [node_rect.max[0], node_rect.min[1]],
                    [node_rect.max[0] + margin.right, node_rect.max[1]],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(1.0, 0.0, 0.0, 0.2),
                )),
            ));
        }
        // Draw bottom margin (full width including corners)
        if margin.bottom > 0.0 {
            out.push((
                Rect::new(
                    [node_rect.min[0] - margin.left, node_rect.max[1]],
                    [
                        node_rect.max[0] + margin.right,
                        node_rect.max[1] + margin.bottom,
                    ],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(1.0, 0.0, 0.0, 0.2),
                )),
            ));
        }
        // Draw left margin (excluding top and bottom corners)
        if margin.left > 0.0 {
            out.push((
                Rect::new(
                    [node_rect.min[0] - margin.left, node_rect.min[1]],
                    [node_rect.min[0], node_rect.max[1]],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(1.0, 0.0, 0.0, 0.2),
                )),
            ));
        }
    }

    // Draw content area (yellow outline - area inside padding)
    if options.show_content_area
        && (padding.top > 0.0 || padding.right > 0.0 || padding.bottom > 0.0 || padding.left > 0.0)
    {
        let content_rect = Rect::new(
            [
                node_rect.min[0] + padding.left,
                node_rect.min[1] + padding.top,
            ],
            [
                node_rect.max[0] - padding.right,
                node_rect.max[1] - padding.bottom,
            ],
        );
        out.push((
            content_rect,
            clip_rect,
            Shape::Rect(
                StyledRect::new(Default::default(), Color::transparent())
                    .with_stroke(Stroke::new(1.0, Color::new(1.0, 1.0, 0.0, 0.5))),
            ),
        ));
    }

    // Draw padding area (semi-transparent blue showing the padding inset)
    if options.show_padding
        && (padding.top > 0.0 || padding.right > 0.0 || padding.bottom > 0.0 || padding.left > 0.0)
    {
        // Draw top padding (full width)
        if padding.top > 0.0 {
            out.push((
                Rect::new(
                    [node_rect.min[0], node_rect.min[1]],
                    [node_rect.max[0], node_rect.min[1] + padding.top],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(0.0, 0.0, 1.0, 0.2),
                )),
            ));
        }
        // Draw right padding (excluding top and bottom corners)
        if padding.right > 0.0 {
            out.push((
                Rect::new(
                    [
                        node_rect.max[0] - padding.right,
                        node_rect.min[1] + padding.top,
                    ],
                    [node_rect.max[0], node_rect.max[1] - padding.bottom],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(0.0, 0.0, 1.0, 0.2),
                )),
            ));
        }
        // Draw bottom padding (full width)
        if padding.bottom > 0.0 {
            out.push((
                Rect::new(
                    [node_rect.min[0], node_rect.max[1] - padding.bottom],
                    [node_rect.max[0], node_rect.max[1]],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(0.0, 0.0, 1.0, 0.2),
                )),
            ));
        }
        // Draw left padding (excluding top and bottom corners)
        if padding.left > 0.0 {
            out.push((
                Rect::new(
                    [node_rect.min[0], node_rect.min[1] + padding.top],
                    [
                        node_rect.min[0] + padding.left,
                        node_rect.max[1] - padding.bottom,
                    ],
                ),
                clip_rect,
                Shape::Rect(StyledRect::new(
                    Default::default(),
                    Color::new(0.0, 0.0, 1.0, 0.2),
                )),
            ));
        }
    }

    // Draw node border (green outline for the actual node rect)
    if options.show_borders {
        out.push((
            node_rect,
            clip_rect,
            Shape::Rect(
                StyledRect::new(Default::default(), Color::transparent())
                    .with_stroke(Stroke::new(1.0, Color::new(0.0, 1.0, 0.0, 0.5))),
            ),
        ));
    }

    // Draw clip rectangle (red outline showing the clipping boundary)
    if options.show_clip_rects {
        out.push((
            clip_rect,
            clip_rect, // Don't clip the clip rect visualization itself
            Shape::Rect(
                StyledRect::new(Default::default(), Color::transparent())
                    .with_stroke(Stroke::new(2.0, Color::new(1.0, 0.0, 0.0, 0.8))),
            ),
        ));
    }
}

fn collect_gap_debug_shapes(
    node: &Node,
    clip_rect: Rect,
    _options: &crate::debug::DebugOptions,
    out: &mut Vec<(Rect, Rect, Shape)>,
) {
    use crate::color::Color;
    use crate::layout::Layout;
    use crate::primitives::StyledRect;

    let children = node.children();
    if children.len() < 2 {
        return; // No gaps to visualize if fewer than 2 children
    }

    let layout_direction = node.layout_direction();

    // Draw gap rectangles between consecutive children
    for i in 0..children.len() - 1 {
        let current_child = &children[i];
        let next_child = &children[i + 1];

        // Get computed layouts for both children
        let Some(current_layout) = current_child.computed_layout() else {
            continue;
        };
        let Some(next_layout) = next_child.computed_layout() else {
            continue;
        };

        let current_rect = current_layout.rect;
        let next_rect = next_layout.rect;

        // Calculate gap rect based on layout direction
        let gap_rect = match layout_direction {
            Layout::Horizontal => {
                // Gap is between right edge of current and left edge of next
                Rect::new(
                    [current_rect.max[0], current_rect.min[1]],
                    [next_rect.min[0], current_rect.max[1]],
                )
            }
            Layout::Vertical => {
                // Gap is between bottom edge of current and top edge of next
                Rect::new(
                    [current_rect.min[0], current_rect.max[1]],
                    [current_rect.max[0], next_rect.min[1]],
                )
            }
            Layout::Stack => {
                // No gaps in Stack layout (children overlap)
                continue;
            }
        };

        // Draw purple semi-transparent rectangle for gap
        out.push((
            gap_rect,
            clip_rect,
            Shape::Rect(StyledRect::new(
                Default::default(),
                Color::new(0.5, 0.0, 0.5, 0.3), // Purple with 30% opacity
            )),
        ));
    }
}
