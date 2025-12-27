use crate::content::Content;
use crate::layout::{ComputedLayout, Layout, Offset, Overflow, Size, Spacing};
use crate::measure::{ContentMeasurer, IntrinsicSize, MeasureTextRequest};
use crate::primitives::{Rect, Shape};
use crate::style::Style;
use crate::transition::Transition;

/// Unique identifier for a node, used for hit-testing and event routing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    /// Create a new NodeId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// A UI node that can contain a shape, content, and/or children
///
/// Nodes can be either:
/// - Container nodes: Have children and can have an optional background shape
/// - Content nodes: Have content (text, inputs, etc.) and cannot have children
/// - Mixed: Have both a shape and children (container with background)
///
/// All fields are private - use the builder pattern methods (`with_*`) to configure nodes.
pub struct Node {
    /// Optional identifier for this node (used for hit-testing and event routing)
    id: Option<NodeId>,
    /// Width of the node
    width: Size,
    /// Height of the node
    height: Size,
    /// Offset from the default position
    offset: Offset,
    /// Padding inside the node
    padding: Spacing,
    /// Margin outside the node
    margin: Spacing,
    /// Gap between children in the layout direction
    gap: f32,
    /// Layout mode for children
    layout_direction: Layout,
    /// How overflow of content/children is handled.
    ///
    /// Default: `Overflow::Hidden`.
    overflow: Overflow,
    /// Opacity of this node and all its children (0.0 = transparent, 1.0 = opaque).
    ///
    /// Default: 1.0 (fully opaque).
    opacity: f32,
    /// Optional shape to render for this node (background)
    shape: Option<Shape>,
    /// Optional content (text, inputs, etc.) - content nodes cannot have children
    content: Option<Content>,
    /// Child nodes (not allowed if content is Some)
    children: Vec<Node>,
    /// Computed layout (filled during layout pass)
    computed: Option<ComputedLayout>,
    /// Base style (always applied)
    base_style: Option<Style>,
    /// Style to apply when hovered (merged with base)
    hover_style: Option<Style>,
    /// Style to apply when active/pressed (merged with base + hover)
    active_style: Option<Style>,
    /// Style to apply when disabled (overrides all other styles)
    disabled_style: Option<Style>,
    /// Whether this node is disabled (cannot be interacted with)
    disabled: bool,
    /// Transition configuration for style changes
    transition: Option<Transition>,
}

impl Node {
    /// Create a new node with default settings
    pub fn new() -> Self {
        Self {
            id: None,
            width: Size::default(),
            height: Size::default(),
            offset: Offset::zero(),
            padding: Spacing::default(),
            margin: Spacing::default(),
            gap: 0.0,
            layout_direction: Layout::default(),
            overflow: Overflow::default(),
            opacity: 1.0,
            shape: None,
            content: None,
            children: Vec::new(),
            computed: None,
            base_style: None,
            hover_style: None,
            active_style: None,
            disabled_style: None,
            disabled: false,
            transition: None,
        }
    }

    /// Check if this is a content node (has content, cannot have children)
    pub fn is_content_node(&self) -> bool {
        self.content.is_some()
    }

    /// Set the node ID (used for hit-testing and event routing)
    pub fn with_id(mut self, id: impl Into<NodeId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Get the node ID, if set
    pub fn id(&self) -> Option<&NodeId> {
        self.id.as_ref()
    }

    /// Set an auto-generated ID (internal use only, for interactive styles)
    #[doc(hidden)]
    pub fn set_auto_id(&mut self, id: NodeId) {
        if self.id.is_none() {
            self.id = Some(id);
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

    /// Set the gap between children
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set the layout mode
    pub fn with_layout_direction(mut self, direction: Layout) -> Self {
        self.layout_direction = direction;
        self
    }

    /// Set how overflow of content/children is handled (default: `Overflow::Hidden`).
    pub fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Set the opacity of this node and all its children (0.0 = transparent, 1.0 = opaque).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the shape
    pub fn with_shape(mut self, shape: Shape) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Set the content (makes this a content node that cannot have children)
    pub fn with_content(mut self, content: Content) -> Self {
        assert!(
            self.children.is_empty(),
            "Cannot set content on a node that already has children"
        );
        self.content = Some(content);
        self
    }

    /// Set the base style (always applied)
    pub fn with_style(mut self, style: Style) -> Self {
        self.base_style = Some(style);
        self
    }

    /// Set the hover style (merged with base when hovered)
    pub fn with_hover_style(mut self, style: Style) -> Self {
        self.hover_style = Some(style);
        self
    }

    /// Set the active style (merged with base + hover when pressed/active)
    pub fn with_active_style(mut self, style: Style) -> Self {
        self.active_style = Some(style);
        self
    }

    /// Set the disabled style (used when node is disabled, overrides other styles)
    pub fn with_disabled_style(mut self, style: Style) -> Self {
        self.disabled_style = Some(style);
        self
    }

    /// Set whether this node is disabled (cannot be interacted with)
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the transition configuration for style changes
    pub fn with_transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// Add a child node
    pub fn with_child(mut self, child: Node) -> Self {
        assert!(
            self.content.is_none(),
            "Cannot add children to a content node"
        );
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        assert!(
            self.content.is_none(),
            "Cannot add children to a content node"
        );
        self.children.extend(children);
        self
    }

    /// Get the computed layout (if available)
    pub fn computed_layout(&self) -> Option<&ComputedLayout> {
        self.computed.as_ref()
    }

    // Internal getters for fields (used by output.rs and other internal modules)

    /// Get the opacity value
    pub(crate) fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Set the opacity value (used by style system)
    pub(crate) fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity;
    }

    /// Get the offset
    pub(crate) fn offset(&self) -> Offset {
        self.offset
    }

    /// Set the offset (used by style system)
    pub(crate) fn set_offset(&mut self, offset: Offset) {
        self.offset = offset;
    }

    /// Get the overflow policy
    pub(crate) fn overflow(&self) -> Overflow {
        self.overflow
    }

    /// Get the shape, if any
    pub(crate) fn shape(&self) -> Option<&Shape> {
        self.shape.as_ref()
    }

    /// Get mutable reference to the shape (used by style system)
    pub(crate) fn shape_mut(&mut self) -> Option<&mut Shape> {
        self.shape.as_mut()
    }

    /// Get the content, if any
    pub(crate) fn content(&self) -> Option<&Content> {
        self.content.as_ref()
    }

    /// Get mutable reference to the content (used by style system)
    pub(crate) fn content_mut(&mut self) -> Option<&mut Content> {
        self.content.as_mut()
    }

    /// Get the padding
    pub(crate) fn padding(&self) -> Spacing {
        self.padding
    }

    /// Get the margin
    pub(crate) fn margin(&self) -> Spacing {
        self.margin
    }

    /// Get the gap between children
    pub(crate) fn gap(&self) -> f32 {
        self.gap
    }

    /// Get the layout mode
    pub(crate) fn layout_direction(&self) -> Layout {
        self.layout_direction
    }

    /// Get the children
    pub(crate) fn children(&self) -> &[Node] {
        &self.children
    }

    /// Get mutable reference to children (used by style system)
    pub fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    /// Get the base style
    pub fn base_style(&self) -> Option<&Style> {
        self.base_style.as_ref()
    }

    /// Get the hover style
    pub fn hover_style(&self) -> Option<&Style> {
        self.hover_style.as_ref()
    }

    /// Get the active style
    pub fn active_style(&self) -> Option<&Style> {
        self.active_style.as_ref()
    }

    /// Get the disabled style
    pub fn disabled_style(&self) -> Option<&Style> {
        self.disabled_style.as_ref()
    }

    /// Check if this node is disabled
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Get the transition configuration
    pub fn transition(&self) -> Option<&Transition> {
        self.transition.as_ref()
    }

    /// Measure the intrinsic size of this node (content + padding, excluding margins).
    ///
    /// This recursively measures children and applies the same margin/gap collapsing
    /// rules as layout to ensure measured sizes match final layout.
    ///
    /// Returns the node's "border-box" size (content + padding), NOT including margins.
    /// Parent is responsible for adding margins when positioning.
    ///
    /// NOTE: This always measures content size, regardless of the node's Size type.
    /// The Size type only matters when the parent is aggregating children for FitContent sizing.
    fn measure_node(&self, measurer: &mut dyn ContentMeasurer) -> IntrinsicSize {
        // Short-circuit: if both dimensions are Fixed, we can return immediately
        if let (Size::Fixed(w), Size::Fixed(h)) = (self.width, self.height) {
            return IntrinsicSize::new(w, h);
        }

        // Measure width - only FitContent measures children
        let width = match self.width {
            Size::Fixed(w) => w,
            Size::FitContent => {
                let content_width = if let Some(content) = &self.content {
                    match content {
                        Content::Text(text_content) => {
                            measurer
                                .measure_text(MeasureTextRequest::from_text_content(text_content))
                                .width
                        }
                    }
                } else if !self.children.is_empty() {
                    self.measure_children(measurer).width
                } else {
                    0.0
                };
                content_width + self.padding.left + self.padding.right
            }
            _ => {
                // Fill/Relative: don't measure children, no intrinsic size
                0.0
            }
        };

        // Measure height - only FitContent measures children
        let height = match self.height {
            Size::Fixed(h) => h,
            Size::FitContent => {
                let content_height = if let Some(content) = &self.content {
                    match content {
                        Content::Text(text_content) => {
                            measurer
                                .measure_text(MeasureTextRequest::from_text_content(text_content))
                                .height
                        }
                    }
                } else if !self.children.is_empty() {
                    self.measure_children(measurer).height
                } else {
                    0.0
                };
                content_height + self.padding.top + self.padding.bottom
            }
            _ => {
                // Fill/Relative: don't measure children, no intrinsic size
                0.0
            }
        };

        IntrinsicSize::new(width, height)
    }

    /// Measure the intrinsic content size of a container based on its children.
    ///
    /// This uses the same margin/gap collapsing logic as layout to ensure consistency.
    /// IMPORTANT: Only aggregates FitContent children. Fill/Relative children are still
    /// measured (for layout purposes) but don't contribute to parent's intrinsic size.
    ///
    /// OPTIMIZATION: Avoids Vec allocation by computing width/height in a single pass
    fn measure_children(&self, measurer: &mut dyn ContentMeasurer) -> IntrinsicSize {
        if self.children.is_empty() {
            return IntrinsicSize::zero();
        }

        // Calculate spacing using the same collapsing rules as layout
        let (total_horizontal_spacing, total_vertical_spacing) = match self.layout_direction {
            Layout::Horizontal => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        total += child.margin.left;
                    }

                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        let collapsed_margin = child.margin.right.max(next_child.margin.left);
                        total += self.gap.max(collapsed_margin);
                    } else {
                        total += child.margin.right;
                    }
                }
                (total, 0.0)
            }
            Layout::Vertical => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        total += child.margin.top;
                    }

                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        let collapsed_margin = child.margin.bottom.max(next_child.margin.top);
                        total += self.gap.max(collapsed_margin);
                    } else {
                        total += child.margin.bottom;
                    }
                }
                (0.0, total)
            }
            Layout::Stack => {
                // In Stack layout, children don't take up space linearly, so no spacing
                (0.0, 0.0)
            }
        };

        // Compute intrinsic size based on layout direction
        // OPTIMIZATION: Measure and aggregate in a single pass to avoid Vec allocation
        match self.layout_direction {
            Layout::Horizontal => {
                // Width: sum of child widths + spacing (main axis)
                // Height: max of child heights (cross axis)
                let mut total_width = 0.0f32;
                let mut max_height = 0.0f32;

                for child in &self.children {
                    let size = child.measure_node(measurer);
                    total_width += size.width;
                    max_height = max_height.max(size.height);
                }

                IntrinsicSize::new(total_width + total_horizontal_spacing, max_height)
            }
            Layout::Vertical => {
                // Height: sum of child heights + spacing (main axis)
                // Width: max of child widths (cross axis)
                let mut total_height = 0.0f32;
                let mut max_width = 0.0f32;

                for child in &self.children {
                    let size = child.measure_node(measurer);
                    total_height += size.height;
                    max_width = max_width.max(size.width);
                }

                IntrinsicSize::new(max_width, total_height + total_vertical_spacing)
            }
            Layout::Stack => {
                // Stack: max of all child sizes (children overlap in Z)
                let mut max_width = 0.0f32;
                let mut max_height = 0.0f32;

                for child in &self.children {
                    let size = child.measure_node(measurer);
                    max_width = max_width.max(size.width);
                    max_height = max_height.max(size.height);
                }

                IntrinsicSize::new(max_width, max_height)
            }
        }
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

    /// Compute layout with a measurer for resolving `Size::FitContent`.
    ///
    /// This is the recommended entry point when using FitContent sizing.
    pub fn compute_layout_with_measurer(
        &mut self,
        available_rect: Rect,
        measurer: &mut dyn ContentMeasurer,
    ) {
        self.compute_layout_with_parent_size_and_measurer(
            available_rect,
            available_rect.width(),
            available_rect.height(),
            measurer,
            Overflow::Visible, // Root has no parent, assume Visible
        );
    }

    fn compute_layout_with_parent_size_and_measurer(
        &mut self,
        available_rect: Rect,
        parent_width: f32,
        parent_height: f32,
        measurer: &mut dyn ContentMeasurer,
        parent_overflow: Overflow,
    ) {
        // Account for this node's margins when calculating available space
        let available_width = (parent_width - self.margin.left - self.margin.right).max(0.0);
        let available_height = (parent_height - self.margin.top - self.margin.bottom).max(0.0);

        // Resolve width and height
        // IMPORTANT: Only measure FitContent dimensions. For Fixed/Relative/Fill, use constraints directly.
        // This prevents children from incorrectly affecting parent sizes when parent has constrained dimensions.
        //
        // OPTIMIZATION: Cache measurement result to avoid calling measure_node() twice when both
        // width and height are FitContent
        let measured_size = if self.width.is_fit_content() || self.height.is_fit_content() {
            Some(self.measure_node(measurer))
        } else {
            None
        };

        let width = if self.width.is_fit_content() {
            let measured_width = measured_size.as_ref().unwrap().width;

            if parent_overflow == Overflow::Visible {
                // Parent allows overflow, so use full measured width
                measured_width
            } else {
                // Parent clips overflow, so clamp to available width
                measured_width.min(available_width)
            }
        } else {
            self.width
                .try_resolve(available_width)
                .unwrap_or(available_width)
        };

        let height = if self.height.is_fit_content() {
            let measured_height = measured_size.as_ref().unwrap().height;

            if parent_overflow == Overflow::Visible {
                // Parent allows overflow, so use full measured height
                measured_height
            } else {
                // Parent clips overflow, so clamp to available height
                measured_height.min(available_height)
            }
        } else {
            self.height
                .try_resolve(available_height)
                .unwrap_or(available_height)
        };

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

        // Layout children (same as original, but passing measurer through)
        let mut current_x = content_x;
        let mut current_y = content_y;

        // Calculate total spacing in the layout direction (margins + gaps)
        let (total_horizontal_spacing, total_vertical_spacing) = match self.layout_direction {
            Layout::Horizontal => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        total += child.margin.left;
                    }

                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        let collapsed_margin = child.margin.right.max(next_child.margin.left);
                        total += self.gap.max(collapsed_margin);
                    } else {
                        total += child.margin.right;
                    }
                }
                (total, 0.0)
            }
            Layout::Vertical => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        total += child.margin.top;
                    }

                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        let collapsed_margin = child.margin.bottom.max(next_child.margin.top);
                        total += self.gap.max(collapsed_margin);
                    } else {
                        total += child.margin.bottom;
                    }
                }
                (0.0, total)
            }
            Layout::Stack => {
                // In Stack layout, children don't take up space linearly, so no spacing
                (0.0, 0.0)
            }
        };

        // Space available for children after subtracting spacing (margins + gaps)
        let available_width = (content_width - total_horizontal_spacing).max(0.0);
        let available_height = (content_height - total_vertical_spacing).max(0.0);

        // Calculate remaining space for Fill children
        let (fill_size_width, fill_size_height) = match self.layout_direction {
            Layout::Horizontal => {
                let mut fill_count = 0;
                let mut used_width = 0.0;

                for child in &self.children {
                    if child.width.is_fill() {
                        fill_count += 1;
                    } else if child.width.is_fit_content() {
                        used_width += child.measure_node(measurer).width;
                    } else {
                        // Must be Fixed or Relative
                        used_width += child.width.try_resolve(available_width).unwrap();
                    }
                }

                let remaining_width = (available_width - used_width).max(0.0);
                let fill_width = if fill_count > 0 {
                    remaining_width / fill_count as f32
                } else {
                    0.0
                };

                (fill_width, available_height)
            }
            Layout::Vertical => {
                let mut fill_count = 0;
                let mut used_height = 0.0;

                for child in &self.children {
                    if child.height.is_fill() {
                        fill_count += 1;
                    } else if child.height.is_fit_content() {
                        used_height += child.measure_node(measurer).height;
                    } else {
                        // Must be Fixed or Relative
                        used_height += child.height.try_resolve(available_height).unwrap();
                    }
                }

                let remaining_height = (available_height - used_height).max(0.0);
                let fill_height = if fill_count > 0 {
                    remaining_height / fill_count as f32
                } else {
                    0.0
                };

                (available_width, fill_height)
            }
            Layout::Stack => {
                // In Stack layout, all children get full available space
                (available_width, available_height)
            }
        };

        let num_children = self.children.len();
        for i in 0..num_children {
            if i == 0 {
                match self.layout_direction {
                    Layout::Horizontal => {
                        current_x += self.children[i].margin.left;
                    }
                    Layout::Vertical => {
                        current_y += self.children[i].margin.top;
                    }
                    Layout::Stack => {
                        // In Stack layout, don't advance position for first child
                    }
                }
            }

            let child_available_rect = match self.layout_direction {
                Layout::Horizontal => Rect::new(
                    [current_x, current_y],
                    [content_x + content_width, content_y + content_height],
                ),
                Layout::Vertical => Rect::new(
                    [current_x, current_y],
                    [content_x + content_width, content_y + content_height],
                ),
                Layout::Stack => {
                    // In Stack layout, all children start at content origin
                    Rect::new(
                        [content_x, content_y],
                        [content_x + content_width, content_y + content_height],
                    )
                }
            };

            let child_parent_width = if self.children[i].width.is_fill() {
                fill_size_width + self.children[i].margin.left + self.children[i].margin.right
            } else {
                available_width + self.children[i].margin.left + self.children[i].margin.right
            };
            let child_parent_height = if self.children[i].height.is_fill() {
                fill_size_height + self.children[i].margin.top + self.children[i].margin.bottom
            } else {
                available_height + self.children[i].margin.top + self.children[i].margin.bottom
            };

            self.children[i].compute_layout_with_parent_size_and_measurer(
                child_available_rect,
                child_parent_width,
                child_parent_height,
                measurer,
                self.overflow, // Pass this node's overflow to children
            );

            if let Some(child_layout) = self.children[i].computed_layout() {
                let child_rect = child_layout.rect;

                if i + 1 < num_children {
                    match self.layout_direction {
                        Layout::Horizontal => {
                            let collapsed_margin = self.children[i]
                                .margin
                                .right
                                .max(self.children[i + 1].margin.left);
                            let spacing = self.gap.max(collapsed_margin);
                            current_x = child_rect.max[0] + spacing;
                        }
                        Layout::Vertical => {
                            let collapsed_margin = self.children[i]
                                .margin
                                .bottom
                                .max(self.children[i + 1].margin.top);
                            let spacing = self.gap.max(collapsed_margin);
                            current_y = child_rect.max[1] + spacing;
                        }
                        Layout::Stack => {
                            // In Stack layout, don't advance position (children overlap)
                        }
                    }
                }
            }
        }
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
        // NOTE: Without a measurer, FitContent falls back to available size
        let width = self
            .width
            .try_resolve(available_width)
            .unwrap_or(available_width);
        let height = self
            .height
            .try_resolve(available_height)
            .unwrap_or(available_height);

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

        // Calculate total spacing in the layout direction (margins + gaps)
        let (total_horizontal_spacing, total_vertical_spacing) = match self.layout_direction {
            Layout::Horizontal => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        // First child: left margin doesn't collapse with parent padding
                        total += child.margin.left;
                    }

                    // Between this child and the next, collapse gap with margins
                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        // Collapsed margin is the max of the two adjacent margins
                        let collapsed_margin = child.margin.right.max(next_child.margin.left);
                        // Collapse gap with margin - use the larger of gap or collapsed margin
                        total += self.gap.max(collapsed_margin);
                    } else {
                        // Last child: just add its right margin
                        total += child.margin.right;
                    }
                }
                (total, 0.0)
            }
            Layout::Vertical => {
                let mut total = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    if i == 0 {
                        // First child: top margin doesn't collapse with parent padding
                        total += child.margin.top;
                    }

                    // Between this child and the next, collapse gap with margins
                    if i + 1 < self.children.len() {
                        let next_child = &self.children[i + 1];
                        // Collapsed margin is the max of the two adjacent margins
                        let collapsed_margin = child.margin.bottom.max(next_child.margin.top);
                        // Collapse gap with margin - use the larger of gap or collapsed margin
                        total += self.gap.max(collapsed_margin);
                    } else {
                        // Last child: just add its bottom margin
                        total += child.margin.bottom;
                    }
                }
                (0.0, total)
            }
            Layout::Stack => {
                // In Stack layout, children don't take up space linearly, so no spacing
                (0.0, 0.0)
            }
        };

        // Space available for children after subtracting spacing (margins + gaps)
        let available_width = (content_width - total_horizontal_spacing).max(0.0);
        let available_height = (content_height - total_vertical_spacing).max(0.0);

        // Calculate remaining space for Fill children
        let (fill_size_width, fill_size_height) = match self.layout_direction {
            Layout::Horizontal => {
                // Count Fill children and calculate space used by non-Fill children
                let mut fill_count = 0;
                let mut used_width = 0.0;

                for child in &self.children {
                    if child.width.is_fill() {
                        fill_count += 1;
                    } else {
                        // For FitContent without measurer, fall back to available width
                        used_width += child
                            .width
                            .try_resolve(available_width)
                            .unwrap_or(available_width);
                    }
                }

                // Fill children divide the remaining space after non-Fill children
                let remaining_width = (available_width - used_width).max(0.0);
                let fill_width = if fill_count > 0 {
                    remaining_width / fill_count as f32
                } else {
                    0.0
                };

                (fill_width, available_height)
            }
            Layout::Vertical => {
                // Count Fill children and calculate space used by non-Fill children
                let mut fill_count = 0;
                let mut used_height = 0.0;

                for child in &self.children {
                    if child.height.is_fill() {
                        fill_count += 1;
                    } else {
                        // For FitContent without measurer, fall back to available height
                        used_height += child
                            .height
                            .try_resolve(available_height)
                            .unwrap_or(available_height);
                    }
                }

                // Fill children divide the remaining space after non-Fill children
                let remaining_height = (available_height - used_height).max(0.0);
                let fill_height = if fill_count > 0 {
                    remaining_height / fill_count as f32
                } else {
                    0.0
                };

                (available_width, fill_height)
            }
            Layout::Stack => {
                // In Stack layout, all children get full available space
                (available_width, available_height)
            }
        };

        let num_children = self.children.len();
        for i in 0..num_children {
            // Apply leading margin for first child or collapsed margin was already added for subsequent children
            if i == 0 {
                match self.layout_direction {
                    Layout::Horizontal => {
                        current_x += self.children[i].margin.left;
                    }
                    Layout::Vertical => {
                        current_y += self.children[i].margin.top;
                    }
                    Layout::Stack => {
                        // In Stack layout, don't advance position for first child
                    }
                }
            }

            let child_available_rect = match self.layout_direction {
                Layout::Horizontal => {
                    // In horizontal layout, each child gets remaining width and full height
                    Rect::new(
                        [current_x, current_y],
                        [content_x + content_width, content_y + content_height],
                    )
                }
                Layout::Vertical => {
                    // In vertical layout, each child gets full width and remaining height
                    Rect::new(
                        [current_x, current_y],
                        [content_x + content_width, content_y + content_height],
                    )
                }
                Layout::Stack => {
                    // In Stack layout, all children start at content origin
                    Rect::new(
                        [content_x, content_y],
                        [content_x + content_width, content_y + content_height],
                    )
                }
            };

            // Pass the available dimensions for size calculations
            // For Fill children, we need to add back their own margins since they'll subtract them
            let child_parent_width = if self.children[i].width.is_fill() {
                fill_size_width + self.children[i].margin.left + self.children[i].margin.right
            } else {
                available_width + self.children[i].margin.left + self.children[i].margin.right
            };
            let child_parent_height = if self.children[i].height.is_fill() {
                fill_size_height + self.children[i].margin.top + self.children[i].margin.bottom
            } else {
                available_height + self.children[i].margin.top + self.children[i].margin.bottom
            };

            self.children[i].compute_layout_with_parent_size(
                child_available_rect,
                child_parent_width,
                child_parent_height,
            );

            // Advance position for next child with collapsed spacing (gap collapsed with margins)
            if let Some(child_layout) = self.children[i].computed_layout() {
                let child_rect = child_layout.rect;

                if i + 1 < num_children {
                    match self.layout_direction {
                        Layout::Horizontal => {
                            // Move to end of current child, then add collapsed spacing
                            let collapsed_margin = self.children[i]
                                .margin
                                .right
                                .max(self.children[i + 1].margin.left);
                            // Collapse gap with margin - use the larger value
                            let spacing = self.gap.max(collapsed_margin);
                            current_x = child_rect.max[0] + spacing;
                        }
                        Layout::Vertical => {
                            // Move to end of current child, then add collapsed spacing
                            let collapsed_margin = self.children[i]
                                .margin
                                .bottom
                                .max(self.children[i + 1].margin.top);
                            // Collapse gap with margin - use the larger value
                            let spacing = self.gap.max(collapsed_margin);
                            current_y = child_rect.max[1] + spacing;
                        }
                        Layout::Stack => {
                            // In Stack layout, don't advance position (children overlap)
                        }
                    }
                }
            }
        }
    }

    /// Collect all shapes from this node tree for rendering
    pub fn collect_shapes(&self, shapes: &mut Vec<(Rect, Shape)>) {
        self.collect_shapes_with_opacity(shapes, 1.0);
    }

    /// Collect shapes with cumulative opacity
    fn collect_shapes_with_opacity(&self, shapes: &mut Vec<(Rect, Shape)>, parent_opacity: f32) {
        let combined_opacity = parent_opacity * self.opacity;

        // Skip rendering if fully transparent
        if combined_opacity <= 0.0 {
            return;
        }

        if let Some(layout) = &self.computed {
            // Add background shape if present
            if let Some(shape) = &self.shape {
                let mut shape_with_opacity = shape.clone();
                shape_with_opacity.apply_opacity(combined_opacity);
                shapes.push((layout.rect, shape_with_opacity));
            }

            // Add content shape if this is a content node
            if let Some(content) = &self.content {
                match content {
                    crate::content::Content::Text(text_content) => {
                        // Calculate content area (after padding)
                        let content_rect = Rect::new(
                            [
                                layout.rect.min[0] + self.padding.left,
                                layout.rect.min[1] + self.padding.top,
                            ],
                            [
                                layout.rect.max[0] - self.padding.right,
                                layout.rect.max[1] - self.padding.bottom,
                            ],
                        );
                        let mut text_shape =
                            crate::primitives::TextShape::new(content_rect, text_content);
                        text_shape.apply_opacity(combined_opacity);
                        shapes.push((layout.rect, Shape::Text(text_shape)));
                    }
                }
            }
        }

        for child in &self.children {
            child.collect_shapes_with_opacity(shapes, combined_opacity);
        }
    }

    /// Collect debug visualization shapes showing margins, padding, and content areas
    pub fn collect_debug_shapes(
        &self,
        shapes: &mut Vec<(Rect, Shape)>,
        options: &crate::debug::DebugOptions,
    ) {
        use crate::color::Color;
        use crate::primitives::{Stroke, StyledRect};

        if let Some(layout) = &self.computed {
            let rect = layout.rect;

            // Draw margin area (outermost, semi-transparent red showing margin space)
            if options.show_margins
                && (self.margin.top > 0.0
                    || self.margin.right > 0.0
                    || self.margin.bottom > 0.0
                    || self.margin.left > 0.0)
            {
                // Draw top margin
                if self.margin.top > 0.0 {
                    shapes.push((
                        Rect::new(
                            [
                                rect.min[0] - self.margin.left,
                                rect.min[1] - self.margin.top,
                            ],
                            [rect.max[0] + self.margin.right, rect.min[1]],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(1.0, 0.0, 0.0, 0.2),
                        )),
                    ));
                }
                // Draw right margin (excluding top and bottom corners)
                if self.margin.right > 0.0 {
                    shapes.push((
                        Rect::new(
                            [rect.max[0], rect.min[1]],
                            [rect.max[0] + self.margin.right, rect.max[1]],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(1.0, 0.0, 0.0, 0.2),
                        )),
                    ));
                }
                // Draw bottom margin (full width including corners)
                if self.margin.bottom > 0.0 {
                    shapes.push((
                        Rect::new(
                            [rect.min[0] - self.margin.left, rect.max[1]],
                            [
                                rect.max[0] + self.margin.right,
                                rect.max[1] + self.margin.bottom,
                            ],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(1.0, 0.0, 0.0, 0.2),
                        )),
                    ));
                }
                // Draw left margin (excluding top and bottom corners)
                if self.margin.left > 0.0 {
                    shapes.push((
                        Rect::new(
                            [rect.min[0] - self.margin.left, rect.min[1]],
                            [rect.min[0], rect.max[1]],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(1.0, 0.0, 0.0, 0.2),
                        )),
                    ));
                }
            }

            // Draw content area (yellow outline - area inside padding)
            if options.show_content_area
                && (self.padding.top > 0.0
                    || self.padding.right > 0.0
                    || self.padding.bottom > 0.0
                    || self.padding.left > 0.0)
            {
                let content_rect = Rect::new(
                    [
                        rect.min[0] + self.padding.left,
                        rect.min[1] + self.padding.top,
                    ],
                    [
                        rect.max[0] - self.padding.right,
                        rect.max[1] - self.padding.bottom,
                    ],
                );
                shapes.push((
                    content_rect,
                    Shape::Rect(
                        StyledRect::new(Default::default(), Color::transparent())
                            .with_stroke(Stroke::new(1.0, Color::new(1.0, 1.0, 0.0, 0.5))),
                    ),
                ));
            }

            // Draw padding area (semi-transparent blue showing the padding inset)
            if options.show_padding
                && (self.padding.top > 0.0
                    || self.padding.right > 0.0
                    || self.padding.bottom > 0.0
                    || self.padding.left > 0.0)
            {
                // Draw top padding (full width)
                if self.padding.top > 0.0 {
                    shapes.push((
                        Rect::new(
                            [rect.min[0], rect.min[1]],
                            [rect.max[0], rect.min[1] + self.padding.top],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(0.0, 0.0, 1.0, 0.2),
                        )),
                    ));
                }
                // Draw right padding (excluding top and bottom corners)
                if self.padding.right > 0.0 {
                    shapes.push((
                        Rect::new(
                            [
                                rect.max[0] - self.padding.right,
                                rect.min[1] + self.padding.top,
                            ],
                            [rect.max[0], rect.max[1] - self.padding.bottom],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(0.0, 0.0, 1.0, 0.2),
                        )),
                    ));
                }
                // Draw bottom padding (full width)
                if self.padding.bottom > 0.0 {
                    shapes.push((
                        Rect::new(
                            [rect.min[0], rect.max[1] - self.padding.bottom],
                            [rect.max[0], rect.max[1]],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(0.0, 0.0, 1.0, 0.2),
                        )),
                    ));
                }
                // Draw left padding (excluding top and bottom corners)
                if self.padding.left > 0.0 {
                    shapes.push((
                        Rect::new(
                            [rect.min[0], rect.min[1] + self.padding.top],
                            [
                                rect.min[0] + self.padding.left,
                                rect.max[1] - self.padding.bottom,
                            ],
                        ),
                        Shape::Rect(StyledRect::new(
                            Default::default(),
                            Color::new(0.0, 0.0, 1.0, 0.2),
                        )),
                    ));
                }
            }

            // Draw node border (green outline for the actual node rect)
            if options.show_borders {
                shapes.push((
                    rect,
                    Shape::Rect(
                        StyledRect::new(Default::default(), Color::transparent())
                            .with_stroke(Stroke::new(1.0, Color::new(0.0, 1.0, 0.0, 0.5))),
                    ),
                ));
            }
        }

        for child in &self.children {
            child.collect_debug_shapes(shapes, options);
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}
