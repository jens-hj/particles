//! Toggle (switch) component for interactive UI
//!
//! Provides an iOS-style toggle switch with smooth animations.

use astra_gui::{
    catppuccin::mocha, Color, CornerShape, Layout, Node, NodeId, Size, Spacing, Style, StyledRect,
    Transition,
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
    pub knob_width: f32,
    /// Margin between knob and track edges
    pub knob_margin: f32,
}

impl Default for ToggleStyle {
    fn default() -> Self {
        Self {
            off_color: mocha::SURFACE0,
            on_color: mocha::LAVENDER,
            knob_color: mocha::BASE,
            track_width: 50.0,
            track_height: 30.0,
            knob_width: 26.0,
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
    let id_str = id.into();
    let knob_offset_x = if value {
        style.track_width - style.knob_width - style.knob_margin * 2.0
    } else {
        0.0
    };

    // Track (background)
    Node::new()
        .with_id(NodeId::new(id_str.clone()))
        .with_width(Size::px(style.track_width))
        .with_height(Size::px(style.track_height))
        .with_layout_direction(Layout::Horizontal)
        .with_padding(Spacing::all(style.knob_margin))
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
            opacity: Some(1.0),
            ..Default::default()
        })
        .with_hover_style(Style {
            fill_color: Some(mocha::SURFACE1),
            opacity: Some(0.9),
            ..Default::default()
        })
        .with_active_style(Style {
            opacity: Some(0.7),
            ..Default::default()
        })
        .with_disabled_style(Style {
            fill_color: Some(mocha::SURFACE0),
            opacity: Some(0.5),
            ..Default::default()
        })
        .with_disabled(disabled)
        .with_transition(Transition::quick())
        .with_child(
            // Knob (sliding circle with smooth offset animation)
            // Note: we use style offset instead of with_offset() so it can be animated
            // The knob needs an ID so InteractiveStateManager can track its transitions
            Node::new()
                .with_id(NodeId::new(format!("{}_knob", id_str)))
                .with_width(Size::px(style.knob_width))
                .with_height(Size::Fill)
                .with_shape(astra_gui::Shape::Rect(StyledRect {
                    rect: astra_gui::Rect::default(),
                    corner_shape: CornerShape::Round(style.knob_width / 2.0),
                    fill: style.knob_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.knob_color),
                    corner_radius: Some(style.knob_width / 2.0),
                    offset_x: Some(knob_offset_x),
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
    let knob_id = format!("{}_knob", toggle_id);
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Click { .. })
            && (e.target.as_str() == toggle_id || e.target.as_str() == knob_id)
    })
}
