//! Slider component for interactive UI
//!
//! Provides a draggable slider for selecting values within a range.

use astra_gui::{
    catppuccin::mocha, Color, CornerShape, Layout, Node, NodeId, Offset, Rect, Shape, Size, Style,
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
            filled_color: mocha::LAVENDER,
            thumb_color: mocha::BASE,
            thumb_hover_color: mocha::SURFACE0,
            thumb_active_color: mocha::MAUVE.with_alpha(0.0),
            track_width: 200.0,
            track_height: 30.0,
            thumb_size: 26.0,
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
    let thumb_inset = (style.track_height - style.thumb_size) / 2.0;
    let usable_width =
        style.track_width - style.thumb_size - (style.track_height - style.thumb_size) * 2.0;
    let thumb_offset_x =
        (usable_width - (style.thumb_size - style.track_height)) * percentage + thumb_inset;

    // Calculate filled width
    let filled_width = thumb_offset_x + style.track_height - thumb_inset;

    // Create a wrapper that just handles the sizing
    // We'll use a single interactive container that captures all events
    Node::new()
        .with_width(Size::px(style.track_width))
        .with_height(Size::px(style.thumb_size.max(style.track_height)))
        .with_layout_direction(Layout::Stack) // Stack the visual elements
        // Track background (unfilled) - no ID so events go to container
        .with_children(vec![
            Node::new()
                .with_width(Size::px(style.track_width))
                .with_height(Size::px(style.track_height))
                .with_offset(Offset::new(
                    0.0,
                    (style.thumb_size - style.track_height) / 2.0,
                ))
                .with_shape(Shape::Rect(StyledRect {
                    rect: Rect::default(),
                    corner_shape: CornerShape::Round(style.track_height / 2.0),
                    fill: style.track_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.track_color),
                    ..Default::default()
                })
                .with_disabled(disabled)
                .with_transition(Transition::quick()),
            // Filled portion of track
            Node::new()
                .with_width(Size::px(filled_width))
                .with_height(Size::px(style.track_height))
                .with_offset(astra_gui::Offset::new(0.0, -thumb_inset))
                .with_shape(astra_gui::Shape::Rect(
                    StyledRect::new(Default::default(), style.filled_color)
                        .with_corner_shape(CornerShape::Round(style.track_height / 2.0)),
                ))
                .with_style(Style {
                    fill_color: Some(style.filled_color),
                    ..Default::default()
                })
                .with_disabled_style(Style {
                    fill_color: Some(mocha::SURFACE1),
                    ..Default::default()
                })
                .with_disabled(disabled)
                .with_transition(Transition::quick()),
            Node::new()
                .with_width(Size::px(style.thumb_size))
                .with_height(Size::px(style.thumb_size))
                .with_offset(Offset::new(thumb_offset_x, 0.0))
                .with_shape(Shape::Rect(StyledRect {
                    rect: Rect::default(),
                    corner_shape: CornerShape::Round(style.thumb_size / 2.0),
                    fill: style.thumb_color,
                    stroke: None,
                }))
                .with_style(Style {
                    fill_color: Some(style.thumb_color),
                    opacity: Some(1.0),
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
                    opacity: Some(0.0),
                    ..Default::default()
                })
                .with_disabled(disabled)
                .with_transition(Transition::quick()),
            // Hitbox node
            Node::new()
                .with_id(NodeId::new(format!("{}_hitbox", id_str)))
                .with_width(Size::Fill)
                .with_height(Size::Fill)
                .with_disabled(disabled),
        ])
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
/// * `step` - Optional step size. If provided, values will snap to multiples of this increment
///
/// # Returns
/// `true` if the value was changed, `false` otherwise
pub fn slider_drag(
    slider_id: &str,
    value: &mut f32,
    range: &RangeInclusive<f32>,
    events: &[TargetedEvent],
    style: &SliderStyle,
    step: Option<f32>,
) -> bool {
    let container_id = format!("{}_hitbox", slider_id);

    // Only handle events from container
    // Stack layout causes coordinate issues with thumb events during drag
    for event in events {
        let target_str = event.target.as_str();

        // Only process container events
        if target_str != container_id {
            continue;
        }

        match &event.event {
            InteractionEvent::Click { .. }
            | InteractionEvent::DragStart { .. }
            | InteractionEvent::DragMove { .. } => {
                let local_x = event.local_position.x;

                // Adjust for thumb half-width so clicking centers the thumb at cursor
                let usable_width = style.track_width - style.thumb_size;
                let adjusted_x = (local_x - style.thumb_size / 2.0).clamp(0.0, usable_width);
                let percentage = if usable_width > 0.0 {
                    (adjusted_x / usable_width).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                let range_size = range.end() - range.start();
                let mut new_value = range.start() + range_size * percentage;

                // Apply step if provided
                if let Some(step_size) = step {
                    if step_size > 0.0 {
                        // Snap to range boundaries if we're very close (within 2% of slider)
                        // This allows reaching min/max even when they're not divisible by step
                        if percentage < 0.02 {
                            new_value = *range.start();
                        } else if percentage > 0.98 {
                            new_value = *range.end();
                        } else {
                            // Round to nearest step
                            let steps_from_start =
                                ((new_value - range.start()) / step_size).round();
                            new_value = range.start() + steps_from_start * step_size;
                            // Clamp to range in case rounding pushed us out of bounds
                            new_value = new_value.clamp(*range.start(), *range.end());
                        }
                    }
                }

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
    let container_id = format!("{}_container", slider_id);

    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Hover { .. })
            && (e.target.as_str() == slider_id || e.target.as_str() == container_id)
    })
}

/// Check if a slider with the given ID is currently being dragged
pub fn slider_dragging(slider_id: &str, events: &[TargetedEvent]) -> bool {
    let container_id = format!("{}_container", slider_id);

    events.iter().any(|e| {
        matches!(
            e.event,
            InteractionEvent::DragStart { .. } | InteractionEvent::DragMove { .. }
        ) && (e.target.as_str() == slider_id || e.target.as_str() == container_id)
    })
}
