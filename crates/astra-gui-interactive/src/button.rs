//! Button component for interactive UI
//!
//! Provides a clickable button widget with hover and press states.

use astra_gui::{
    catppuccin::mocha, Color, Content, CornerShape, HorizontalAlign, Node, NodeId, Size, Spacing,
    Style, StyledRect, TextContent, Transition, VerticalAlign,
};
use astra_gui_wgpu::{InteractionEvent, TargetedEvent};

/// Visual state of a button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Button is idle (not being interacted with)
    Idle,
    /// Mouse is hovering over the button
    Hovered,
    /// Button is being pressed
    Pressed,
    /// Button is disabled (not interactive)
    Disabled,
}

impl ButtonState {
    /// Update the button state based on interaction flags
    ///
    /// # Arguments
    /// * `is_hovered` - Whether the button is currently hovered
    /// * `is_pressed` - Whether the button is currently pressed
    /// * `enabled` - Whether the button is enabled
    pub fn update(&mut self, is_hovered: bool, is_pressed: bool, enabled: bool) {
        if !enabled {
            *self = ButtonState::Disabled;
        } else if is_pressed {
            *self = ButtonState::Pressed;
        } else if is_hovered {
            *self = ButtonState::Hovered;
        } else {
            *self = ButtonState::Idle;
        }
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        ButtonState::Idle
    }
}

/// Visual styling for a button
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    /// Background color when idle
    pub idle_color: Color,
    /// Background color when hovered
    pub hover_color: Color,
    /// Background color when pressed
    pub pressed_color: Color,
    /// Background color when disabled
    pub disabled_color: Color,
    /// Text color
    pub text_color: Color,
    /// Disabled text color
    pub disabled_text_color: Color,
    /// Internal padding
    pub padding: Spacing,
    /// Corner radius for rounded corners
    pub border_radius: f32,
    /// Font size
    pub font_size: f32,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            idle_color: mocha::LAVENDER,
            hover_color: mocha::LAVENDER.with_alpha(0.8),
            pressed_color: mocha::MAUVE,
            disabled_color: mocha::SURFACE0,
            text_color: mocha::BASE,
            disabled_text_color: mocha::OVERLAY0,
            padding: Spacing::symmetric(32.0, 16.0),
            border_radius: 24.0,
            font_size: 32.0,
        }
    }
}

/// Create a button node with declarative hover/active/disabled states
///
/// This version uses the new style system with automatic state management.
/// No need to manually track button state - hover and active states are
/// applied automatically based on mouse interaction.
///
/// # Arguments
/// * `id` - Unique identifier for the button (used for event targeting)
/// * `label` - Text label displayed on the button
/// * `disabled` - Whether the button is disabled (cannot be interacted with)
/// * `style` - Visual styling configuration
///
/// # Returns
/// A configured `Node` representing the button with automatic state transitions
pub fn button(
    id: impl Into<String>,
    label: impl Into<String>,
    disabled: bool,
    style: &ButtonStyle,
) -> Node {
    Node::new()
        .with_id(NodeId::new(id))
        .with_width(Size::FitContent)
        .with_height(Size::FitContent)
        .with_padding(style.padding)
        .with_shape(astra_gui::Shape::Rect(StyledRect {
            rect: astra_gui::Rect::default(), // Will be filled during layout
            corner_shape: CornerShape::Round(style.border_radius),
            fill: style.idle_color, // Will be overridden by style system
            stroke: None,
        }))
        .with_content(Content::Text(TextContent {
            text: label.into(),
            font_size: style.font_size,
            color: style.text_color, // Will be overridden by style system
            h_align: HorizontalAlign::Center,
            v_align: VerticalAlign::Center,
        }))
        // Declarative styles - no manual state tracking needed!
        .with_style(Style {
            fill_color: Some(style.idle_color),
            text_color: Some(style.text_color),
            corner_radius: Some(style.border_radius),
            ..Default::default()
        })
        .with_hover_style(Style {
            fill_color: Some(style.hover_color),
            ..Default::default()
        })
        .with_active_style(Style {
            fill_color: Some(style.pressed_color),
            ..Default::default()
        })
        .with_disabled_style(Style {
            fill_color: Some(style.disabled_color),
            text_color: Some(style.disabled_text_color),
            corner_radius: Some(style.border_radius),
            ..Default::default()
        })
        .with_disabled(disabled)
        .with_transition(Transition::quick())
}

/// Check if a button with the given ID was clicked this frame
///
/// # Arguments
/// * `button_id` - The ID of the button to check
/// * `events` - Slice of targeted events from this frame
///
/// # Returns
/// `true` if the button was clicked, `false` otherwise
pub fn button_clicked(button_id: &str, events: &[TargetedEvent]) -> bool {
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Click { .. }) && e.target.as_str() == button_id
    })
}

/// Check if a button with the given ID is currently hovered
///
/// # Arguments
/// * `button_id` - The ID of the button to check
/// * `events` - Slice of targeted events from this frame
///
/// # Returns
/// `true` if the button is hovered, `false` otherwise
pub fn button_hovered(button_id: &str, events: &[TargetedEvent]) -> bool {
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Hover { .. }) && e.target.as_str() == button_id
    })
}
