//! Toggle (switch) component for interactive UI
//!
//! Provides an iOS-style toggle switch with smooth animations.

use astra_gui::{
    Color, CornerShape, LayoutDirection, Node, NodeId, Offset, Size, Style, StyledRect, Transition,
};
use astra_gui_wgpu::{InteractionEvent, TargetedEvent};

/// Visual styling for a toggle switch
#[derive(Debug, Clone)]
pub struct ToggleStyle {
    /// Background color when toggle is off
    pub off_color: Color,
    /// Background color when toggle is on
    pub on_color: Color,
    /// Color of the sliding knob
    pub knob_color: Color,
    /// Width of the track
    pub track_width: f32,
    /// Height of the track
    pub track_height: f32,
    /// Diameter of the knob
    pub knob_diameter: f32,
    /// Margin between knob and track edges
    pub knob_margin: f32,
}

impl Default for ToggleStyle {
    fn default() -> Self {
        Self {
            off_color: Color::rgb(0.5, 0.5, 0.5),
            on_color: Color::rgb(0.2, 0.6, 1.0),
            knob_color: Color::rgb(1.0, 1.0, 1.0),
            track_width: 50.0,
            track_height: 30.0,
            knob_diameter: 26.0,
            knob_margin: 2.0,
        }
    }
}

/// Create a toggle switch node
///
/// The toggle uses the declarative style system for smooth transitions between
/// on and off states.
///
/// # Arguments
/// * `id` - Unique identifier for the toggle (used for event targeting)
/// * `value` - Current state of the toggle (true = on, false = off)
/// * `disabled` - Whether the toggle is disabled
/// * `style` - Visual styling configuration
///
/// # Returns
/// A configured `Node` representing the toggle switch with automatic state transitions
pub fn toggle(id: impl Into<String>, value: bool, disabled: bool, style: &ToggleStyle) -> Node {
    let knob_offset_x = if value {
        style.track_width - style.knob_diameter - style.knob_margin
    } else {
        style.knob_margin
    };

    // Track (background)
    Node::new()
        .with_id(NodeId::new(id))
        .with_width(Size::px(style.track_width))
        .with_height(Size::px(style.track_height))
        .with_layout_direction(LayoutDirection::Horizontal)
        .with_shape(astra_gui::Shape::Rect(StyledRect {
            rect: astra_gui::Rect::default(),
            corner_shape: CornerShape::Round(style.track_height / 2.0),
            fill: if value {
                style.on_color
            } else {
                style.off_color
            },
            stroke: None,
        }))
        .with_style(Style {
            fill_color: Some(if value {
                style.on_color
            } else {
                style.off_color
            }),
            corner_radius: Some(style.track_height / 2.0),
            ..Default::default()
        })
        .with_hover_style(Style {
            opacity: Some(0.9),
            ..Default::default()
        })
        .with_active_style(Style {
            opacity: Some(0.8),
            ..Default::default()
        })
        .with_disabled_style(Style {
            fill_color: Some(Color::rgb(0.3, 0.3, 0.3)),
            opacity: Some(0.5),
            ..Default::default()
        })
        .with_disabled(disabled)
        .with_transition(Transition::quick())
        .with_child(
            // Knob (positioned with offset - no animation on position currently)
            // TODO: Add offset animation support to style system for smooth sliding
            Node::new()
                .with_width(Size::px(style.knob_diameter))
                .with_height(Size::px(style.knob_diameter))
                .with_offset(Offset::new(knob_offset_x, style.knob_margin))
                .with_shape(astra_gui::Shape::Rect(StyledRect {
                    rect: astra_gui::Rect::default(),
                    corner_shape: CornerShape::Round(style.knob_diameter / 2.0),
                    fill: style.knob_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.knob_color),
                    corner_radius: Some(style.knob_diameter / 2.0),
                    ..Default::default()
                })
                .with_transition(Transition::quick()),
        )
}

/// Check if a toggle with the given ID was clicked this frame
///
/// # Arguments
/// * `toggle_id` - The ID of the toggle to check
/// * `events` - Slice of targeted events from this frame
///
/// # Returns
/// `true` if the toggle was clicked, `false` otherwise
pub fn toggle_clicked(toggle_id: &str, events: &[TargetedEvent]) -> bool {
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Click { .. }) && e.target.as_str() == toggle_id
    })
}
