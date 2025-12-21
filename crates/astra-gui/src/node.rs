use crate::layout::{ComputedLayout, LayoutDirection, Offset, Size, Spacing};
use crate::primitives::{Rect, Shape};

/// A UI node that can contain a shape and children
pub struct Node {
    /// Width of the node
    pub width: Size,
    /// Height of the node
    pub height: Size,
    /// Offset from the default position
    pub offset: Offset,
    /// Padding inside the node
    pub padding: Spacing,
    /// Margin outside the node
    pub margin: Spacing,
    /// Layout direction for children
    pub layout_direction: LayoutDirection,
    /// Optional shape to render for this node
    pub shape: Option<Shape>,
    /// Child nodes
    pub children: Vec<Node>,
    /// Computed layout (filled during layout pass)
    computed: Option<ComputedLayout>,
}

impl Node {
    /// Create a new node with default settings
    pub fn new() -> Self {
        Self {
            width: Size::Fixed(0.0),
            height: Size::Fixed(0.0),
            offset: Offset::zero(),
            padding: Spacing::zero(),
            margin: Spacing::zero(),
            layout_direction: LayoutDirection::Vertical,
            shape: None,
            children: Vec::new(),
            computed: None,
        }
    }

    /// Set the width
    pub fn with_width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    /// Set the height
    pub fn with_height(mut self, height: Size) -> Self {
        self.height = height;
        self
    }

    /// Set both width and height to fixed pixel values
    pub fn with_size(self, width: f32, height: f32) -> Self {
        self.with_width(Size::px(width))
            .with_height(Size::px(height))
    }

    /// Set the offset
    pub fn with_offset(mut self, offset: Offset) -> Self {
        self.offset = offset;
        self
    }

    /// Set the padding
    pub fn with_padding(mut self, padding: Spacing) -> Self {
        self.padding = padding;
        self
    }

    /// Set the margin
    pub fn with_margin(mut self, margin: Spacing) -> Self {
        self.margin = margin;
        self
    }

    /// Set the layout direction
    pub fn with_layout_direction(mut self, direction: LayoutDirection) -> Self {
        self.layout_direction = direction;
        self
    }

    /// Set the shape
    pub fn with_shape(mut self, shape: Shape) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Add a child node
    pub fn with_child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children.extend(children);
        self
    }

    /// Get the computed layout (if available)
    pub fn computed_layout(&self) -> Option<&ComputedLayout> {
        self.computed.as_ref()
    }

    /// Compute layout for this node and all children
    ///
    /// `available_rect` is the space available for this node (typically parent's content area)
    pub fn compute_layout(&mut self, available_rect: Rect) {
        let available_width = available_rect.width();
        let available_height = available_rect.height();

        // Resolve width and height
        let width = self.width.resolve(available_width);
        let height = self.height.resolve(available_height);

        // Apply margin to get the outer rect
        let outer_x = available_rect.min[0] + self.margin.left + self.offset.x;
        let outer_y = available_rect.min[1] + self.margin.top + self.offset.y;

        // Content area (after subtracting padding)
        let content_x = outer_x + self.padding.left;
        let content_y = outer_y + self.padding.top;
        let content_width = width - self.padding.left - self.padding.right;
        let content_height = height - self.padding.top - self.padding.bottom;

        // Store computed layout for this node
        self.computed = Some(ComputedLayout::new(Rect::new(
            [outer_x, outer_y],
            [outer_x + width, outer_y + height],
        )));

        // Layout children
        let mut current_x = content_x;
        let mut current_y = content_y;

        for child in &mut self.children {
            let child_available_rect = match self.layout_direction {
                LayoutDirection::Horizontal => {
                    // In horizontal layout, each child gets remaining width and full height
                    Rect::new(
                        [current_x, current_y],
                        [content_x + content_width, content_y + content_height],
                    )
                }
                LayoutDirection::Vertical => {
                    // In vertical layout, each child gets full width and remaining height
                    Rect::new(
                        [current_x, current_y],
                        [content_x + content_width, content_y + content_height],
                    )
                }
            };

            child.compute_layout(child_available_rect);

            // Advance position for next child
            if let Some(child_layout) = child.computed_layout() {
                let child_rect = child_layout.rect;
                match self.layout_direction {
                    LayoutDirection::Horizontal => {
                        current_x = child_rect.max[0] + child.margin.right;
                    }
                    LayoutDirection::Vertical => {
                        current_y = child_rect.max[1] + child.margin.bottom;
                    }
                }
            }
        }
    }

    /// Collect all shapes from this node tree for rendering
    pub fn collect_shapes(&self, shapes: &mut Vec<(Rect, Shape)>) {
        if let (Some(layout), Some(shape)) = (&self.computed, &self.shape) {
            shapes.push((layout.rect, shape.clone()));
        }

        for child in &self.children {
            child.collect_shapes(shapes);
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}
