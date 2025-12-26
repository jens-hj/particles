//! Slider component for interactive UI
//!
//! Provides a draggable slider for selecting values within a range.

use astra_gui::{
    catppuccin::mocha, Color, CornerShape, LayoutDirection, Node, NodeId, Rect, Size, Style,
    StyledRect, Transition,
};
use astra_gui_wgpu::{InteractionEvent, TargetedEvent};
use std::ops::RangeInclusive;

/// Visual styling for a slider
#[derive(Debug, Clone)]
pub struct SliderStyle {
    /// Color of the track (unfilled portion)
    pub track_color: Color,
    /// Color of the filled portion of the track
    pub filled_color: Color,
    /// Color of the draggable thumb
    pub thumb_color: Color,
    /// Color of the thumb when hovered
    pub thumb_hover_color: Color,
    /// Color of the thumb when being dragged
    pub thumb_active_color: Color,
    /// Width of the slider track
    pub track_width: f32,
    /// Height of the slider track
    pub track_height: f32,
    /// Diameter of the thumb
    pub thumb_size: f32,
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: mocha::SURFACE0,
            filled_color: mocha::MAUVE,
            thumb_color: mocha::TEXT,
            thumb_hover_color: mocha::LAVENDER,
            thumb_active_color: mocha::MAUVE,
            track_width: 200.0,
            track_height: 8.0,
            thumb_size: 20.0,
        }
    }
}

/// Create a slider node
///
/// The slider consists of:
/// - A track (background)
/// - A filled portion showing the current value
/// - A draggable thumb
///
/// # Arguments
/// * `id` - Unique identifier for the slider (used for event targeting)
/// * `value` - Current value (should be within the range)
/// * `range` - The valid range of values
/// * `disabled` - Whether the slider is disabled
/// * `style` - Visual styling configuration
///
/// # Returns
/// A configured `Node` representing the slider
pub fn slider(
    id: impl Into<String>,
    value: f32,
    range: RangeInclusive<f32>,
    disabled: bool,
    style: &SliderStyle,
) -> Node {
    let id_str = id.into();

    // Calculate percentage (0.0 to 1.0)
    let range_size = range.end() - range.start();
    let percentage = if range_size > 0.0 {
        ((value - range.start()) / range_size).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Calculate thumb position
    let usable_width = style.track_width - style.thumb_size;
    let thumb_offset_x = usable_width * percentage;

    // Calculate filled width
    let filled_width = style.track_width * percentage;

    // Track container - positions everything
    Node::new()
        .with_id(NodeId::new(format!("{}_container", id_str)))
        .with_width(Size::px(style.track_width))
        .with_height(Size::px(style.thumb_size.max(style.track_height)))
        .with_layout_direction(LayoutDirection::Horizontal)
        // Track background (unfilled)
        .with_child(
            Node::new()
                .with_id(NodeId::new(format!("{}_track", id_str)))
                .with_width(Size::px(style.track_width))
                .with_height(Size::px(style.track_height))
                .with_offset(astra_gui::Offset::new(
                    0.0,
                    (style.thumb_size - style.track_height) / 2.0,
                ))
                .with_shape(astra_gui::Shape::Rect(StyledRect {
                    rect: Rect::default(),
                    corner_shape: CornerShape::Round(style.track_height / 2.0),
                    fill: style.track_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.track_color),
                    corner_radius: Some(style.track_height / 2.0),
                    ..Default::default()
                })
                .with_disabled_style(Style {
                    opacity: Some(0.5),
                    ..Default::default()
                })
                .with_disabled(disabled),
        )
        // Filled portion of track
        .with_child(
            Node::new()
                .with_id(NodeId::new(format!("{}_filled", id_str)))
                .with_width(Size::px(filled_width.max(style.track_height))) // Min width for rounded ends
                .with_height(Size::px(style.track_height))
                .with_offset(astra_gui::Offset::new(
                    0.0,
                    (style.thumb_size - style.track_height) / 2.0,
                ))
                .with_shape(astra_gui::Shape::Rect(StyledRect {
                    rect: Rect::default(),
                    corner_shape: CornerShape::Round(style.track_height / 2.0),
                    fill: style.filled_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.filled_color),
                    corner_radius: Some(style.track_height / 2.0),
                    ..Default::default()
                })
                .with_disabled_style(Style {
                    fill_color: Some(mocha::SURFACE1),
                    opacity: Some(0.5),
                    ..Default::default()
                })
                .with_disabled(disabled),
        )
        // Thumb (draggable circle)
        .with_child(
            Node::new()
                .with_id(NodeId::new(id_str.clone()))
                .with_width(Size::px(style.thumb_size))
                .with_height(Size::px(style.thumb_size))
                .with_offset(astra_gui::Offset::new(thumb_offset_x, 0.0))
                .with_shape(astra_gui::Shape::Rect(StyledRect {
                    rect: Rect::default(),
                    corner_shape: CornerShape::Round(style.thumb_size / 2.0),
                    fill: style.thumb_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.thumb_color),
                    corner_radius: Some(style.thumb_size / 2.0),
                    ..Default::default()
                })
                .with_hover_style(Style {
                    fill_color: Some(style.thumb_hover_color),
                    ..Default::default()
                })
                .with_active_style(Style {
                    fill_color: Some(style.thumb_active_color),
                    ..Default::default()
                })
                .with_disabled_style(Style {
                    fill_color: Some(mocha::SURFACE1),
                    opacity: Some(0.5),
                    ..Default::default()
                })
                .with_disabled(disabled)
                .with_transition(Transition::quick()),
        )
}

/// Update slider value from drag events
///
/// Call this each frame with the events to update the slider value based on
/// drag interactions.
///
/// # Arguments
/// * `slider_id` - The ID of the slider (thumb ID)
/// * `value` - Current slider value (will be modified if dragged)
/// * `range` - The valid range of values
/// * `events` - Slice of targeted events from this frame
/// * `style` - The slider style (needed for track width calculation)
///
/// # Returns
/// `true` if the value was changed, `false` otherwise
pub fn slider_drag(
    slider_id: &str,
    value: &mut f32,
    range: &RangeInclusive<f32>,
    events: &[TargetedEvent],
    style: &SliderStyle,
) -> bool {
    let track_id = format!("{}_track", slider_id);
    let container_id = format!("{}_container", slider_id);
    let filled_id = format!("{}_filled", slider_id);

    for event in events {
        // Check if this event targets any part of the slider
        let is_slider_event = event.target.as_str() == slider_id
            || event.target.as_str() == track_id
            || event.target.as_str() == container_id
            || event.target.as_str() == filled_id;

        if !is_slider_event {
            continue;
        }

        match &event.event {
            InteractionEvent::DragMove { .. } | InteractionEvent::DragStart { .. } => {
                // Calculate percentage based on position within track
                let local_x = event.local_position.x;
                let percentage = (local_x / style.track_width).clamp(0.0, 1.0);

                let range_size = range.end() - range.start();
                let new_value = range.start() + range_size * percentage;

                if (*value - new_value).abs() > f32::EPSILON {
                    *value = new_value;
                    return true;
                }
            }
            InteractionEvent::Click { .. } => {
                // Also handle click to set value directly
                let local_x = event.local_position.x;
                let percentage = (local_x / style.track_width).clamp(0.0, 1.0);

                let range_size = range.end() - range.start();
                let new_value = range.start() + range_size * percentage;

                if (*value - new_value).abs() > f32::EPSILON {
                    *value = new_value;
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

/// Check if a slider with the given ID is currently being hovered
pub fn slider_hovered(slider_id: &str, events: &[TargetedEvent]) -> bool {
    let track_id = format!("{}_track", slider_id);
    let container_id = format!("{}_container", slider_id);
    let filled_id = format!("{}_filled", slider_id);

    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Hover { .. })
            && (e.target.as_str() == slider_id
                || e.target.as_str() == track_id
                || e.target.as_str() == container_id
                || e.target.as_str() == filled_id)
    })
}

/// Check if a slider with the given ID is currently being dragged
pub fn slider_dragging(slider_id: &str, events: &[TargetedEvent]) -> bool {
    let track_id = format!("{}_track", slider_id);
    let container_id = format!("{}_container", slider_id);
    let filled_id = format!("{}_filled", slider_id);

    events.iter().any(|e| {
        matches!(
            e.event,
            InteractionEvent::DragStart { .. } | InteractionEvent::DragMove { .. }
        ) && (e.target.as_str() == slider_id
            || e.target.as_str() == track_id
            || e.target.as_str() == container_id
            || e.target.as_str() == filled_id)
    })
}
