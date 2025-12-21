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
            width: Size::Relative(1.0),
            height: Size::Relative(1.0),
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
        self.compute_layout_with_parent_size(
            available_rect,
            available_rect.width(),
            available_rect.height(),
        );
    }

    fn compute_layout_with_parent_size(
        &mut self,
        available_rect: Rect,
        parent_width: f32,
        parent_height: f32,
    ) {
        // Account for this node's margins when calculating available space
        let available_width = (parent_width - self.margin.left - self.margin.right).max(0.0);
        let available_height = (parent_height - self.margin.top - self.margin.bottom).max(0.0);

        // Resolve width and height from available space (after margins)
        let width = self.width.resolve(available_width);
        let height = self.height.resolve(available_height);

        // Position is already adjusted for margins by parent, don't add them again
        let outer_x = available_rect.min[0];
        let outer_y = available_rect.min[1];

        // Content area (after subtracting padding)
        let content_x = outer_x + self.padding.left;
        let content_y = outer_y + self.padding.top;
        let content_width = width - self.padding.left - self.padding.right;
        let content_height = height - self.padding.top - self.padding.bottom;

        // Store computed layout for this node, with offset applied
        self.computed = Some(ComputedLayout::new(Rect::new(
            [outer_x + self.offset.x, outer_y + self.offset.y],
            [
                outer_x + width + self.offset.x,
                outer_y + height + self.offset.y,
            ],
        )));

        // Layout children
        let mut current_x = content_x;
        let mut current_y = content_y;

        // Calculate total margin space in the layout direction (with collapsing)
        let (total_horizontal_margin, total_vertical_margin) = match self.layout_direction {
            LayoutDirection::Horizontal => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        // First child: left margin doesn't collapse with parent padding
                        total += child.margin.left;
                    }

                    // Between this child and the next, use the max of right and next's left
                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        // Collapsed margin is the max of the two adjacent margins
                        total += child.margin.right.max(next_child.margin.left);
                    } else {
                        // Last child: just add its right margin
                        total += child.margin.right;
                    }
                }
                (total, 0.0)
            }
            LayoutDirection::Vertical => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        // First child: top margin doesn't collapse with parent padding
                        total += child.margin.top;
                    }

                    // Between this child and the next, use the max of bottom and next's top
                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        // Collapsed margin is the max of the two adjacent margins
                        total += child.margin.bottom.max(next_child.margin.top);
                    } else {
                        // Last child: just add its bottom margin
                        total += child.margin.bottom;
                    }
                }
                (0.0, total)
            }
        };

        // Adjusted content dimensions after subtracting margins
        let adjusted_content_width = (content_width - total_horizontal_margin).max(0.0);
        let adjusted_content_height = (content_height - total_vertical_margin).max(0.0);

        // Calculate remaining space for Fill children
        let (fill_size_width, fill_size_height) = match self.layout_direction {
            LayoutDirection::Horizontal => {
                // Count Fill children and calculate space used by non-Fill children
                let mut fill_count = 0;
                let mut used_width = 0.0;

                for child in &self.children {
                    if child.width.is_fill() {
                        fill_count += 1;
                    } else {
                        used_width += child.width.resolve(adjusted_content_width);
                    }
                }

                let remaining_width = (adjusted_content_width - used_width).max(0.0);
                let fill_width = if fill_count > 0 {
                    remaining_width / fill_count as f32
                } else {
                    0.0
                };

                (fill_width, adjusted_content_height)
            }
            LayoutDirection::Vertical => {
                // Count Fill children and calculate space used by non-Fill children
                let mut fill_count = 0;
                let mut used_height = 0.0;

                for child in &self.children {
                    if child.height.is_fill() {
                        fill_count += 1;
                    } else {
                        used_height += child.height.resolve(adjusted_content_height);
                    }
                }

                let remaining_height = (adjusted_content_height - used_height).max(0.0);
                let fill_height = if fill_count > 0 {
                    remaining_height / fill_count as f32
                } else {
                    0.0
                };

                (adjusted_content_width, fill_height)
            }
        };

        let num_children = self.children.len();
        for i in 0..num_children {
            // Apply leading margin for first child or collapsed margin was already added for subsequent children
            if i == 0 {
                match self.layout_direction {
                    LayoutDirection::Horizontal => {
                        current_x += self.children[i].margin.left;
                    }
                    LayoutDirection::Vertical => {
                        current_y += self.children[i].margin.top;
                    }
                }
            }

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

            // Pass the adjusted parent dimensions (after margin deduction) so percentages are calculated correctly
            // For Fill children, use the calculated fill size instead
            let child_parent_width = if self.children[i].width.is_fill() {
                fill_size_width
            } else {
                adjusted_content_width
            };
            let child_parent_height = if self.children[i].height.is_fill() {
                fill_size_height
            } else {
                adjusted_content_height
            };

            self.children[i].compute_layout_with_parent_size(
                child_available_rect,
                child_parent_width,
                child_parent_height,
            );

            // Advance position for next child with collapsed margins
            if let Some(child_layout) = self.children[i].computed_layout() {
                let child_rect = child_layout.rect;

                if i + 1 < num_children {
                    match self.layout_direction {
                        LayoutDirection::Horizontal => {
                            // Move to end of current child, then add collapsed margin
                            let collapsed_margin = self.children[i]
                                .margin
                                .right
                                .max(self.children[i + 1].margin.left);
                            current_x = child_rect.max[0] + collapsed_margin;
                        }
                        LayoutDirection::Vertical => {
                            // Move to end of current child, then add collapsed margin
                            let collapsed_margin = self.children[i]
                                .margin
                                .bottom
                                .max(self.children[i + 1].margin.top);
                            current_y = child_rect.max[1] + collapsed_margin;
                        }
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
