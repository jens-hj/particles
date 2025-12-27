//! Text input component for interactive UI
//!
//! Provides an editable text input field with cursor, selection, and keyboard support.

use astra_gui::{
    catppuccin::mocha, Color, Content, CornerShape, HorizontalAlign, Node, NodeId, Size, Spacing,
    Style, StyledRect, TextContent, Transition, VerticalAlign,
};
use astra_gui_wgpu::{InteractionEvent, Key, NamedKey, TargetedEvent};

/// Visual styling for a text input
#[derive(Debug, Clone)]
pub struct TextInputStyle {
    /// Background color when idle
    pub idle_color: Color,
    /// Background color when focused
    pub focused_color: Color,
    /// Background color when disabled
    pub disabled_color: Color,
    /// Text color
    pub text_color: Color,
    /// Placeholder text color
    pub placeholder_color: Color,
    /// Disabled text color
    pub disabled_text_color: Color,
    /// Internal padding
    pub padding: Spacing,
    /// Corner radius for rounded corners
    pub border_radius: f32,
    /// Font size
    pub font_size: f32,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            idle_color: mocha::SURFACE0,
            focused_color: mocha::SURFACE1,
            disabled_color: mocha::SURFACE0.with_alpha(0.5),
            text_color: mocha::TEXT,
            placeholder_color: mocha::OVERLAY0,
            disabled_text_color: mocha::OVERLAY0,
            padding: Spacing::symmetric(16.0, 12.0),
            border_radius: 8.0,
            font_size: 24.0,
        }
    }
}

/// Create a text input node
///
/// # Arguments
/// * `id` - Unique identifier for the text input (used for event targeting)
/// * `value` - Current text value
/// * `placeholder` - Placeholder text shown when empty
/// * `focused` - Whether the text input is currently focused
/// * `disabled` - Whether the text input is disabled
/// * `style` - Visual styling configuration
///
/// # Returns
/// A configured `Node` representing the text input
pub fn text_input(
    id: impl Into<String>,
    value: impl Into<String>,
    placeholder: impl Into<String>,
    focused: bool,
    disabled: bool,
    style: &TextInputStyle,
) -> Node {
    let value_str = value.into();
    let placeholder_str = placeholder.into();

    // Determine what text to display
    let display_text = if value_str.is_empty() {
        placeholder_str.clone()
    } else {
        value_str.clone()
    };

    // Determine text color (placeholder vs actual text)
    let text_color = if value_str.is_empty() {
        style.placeholder_color
    } else {
        style.text_color
    };

    Node::new()
        .with_id(NodeId::new(id))
        .with_width(Size::px(300.0))
        .with_height(Size::FitContent)
        .with_padding(style.padding)
        .with_shape(astra_gui::Shape::Rect(StyledRect {
            rect: astra_gui::Rect::default(),
            corner_shape: CornerShape::Round(style.border_radius),
            fill: if focused {
                style.focused_color
            } else {
                style.idle_color
            },
            stroke: None,
        }))
        .with_content(Content::Text(TextContent {
            text: display_text,
            font_size: style.font_size,
            color: text_color,
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Center,
        }))
        .with_style(Style {
            fill_color: Some(if focused {
                style.focused_color
            } else {
                style.idle_color
            }),
            text_color: Some(text_color),
            corner_radius: Some(style.border_radius),
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

/// Handle text input keyboard events and update the value
///
/// Call this each frame with the events and input state to update the text input value
/// based on keyboard input.
///
/// # Arguments
/// * `input_id` - The ID of the text input
/// * `value` - Current text value (will be modified if keys are pressed)
/// * `cursor_pos` - Current cursor position (byte offset, will be modified)
/// * `events` - Slice of targeted events from this frame
/// * `input_state` - Current input state (for keyboard input)
/// * `focused` - Whether this input is currently focused
///
/// # Returns
/// `true` if the value was changed, `false` otherwise
pub fn text_input_update(
    _input_id: &str,
    value: &mut String,
    cursor_pos: &mut usize,
    _events: &[TargetedEvent],
    input_state: &astra_gui_wgpu::InputState,
    focused: bool,
) -> bool {
    let mut changed = false;

    // Only process keyboard input if focused
    if !focused {
        return false;
    }

    // Process typed characters
    for ch in &input_state.characters_typed {
        // Insert character at cursor position
        if *cursor_pos <= value.len() {
            value.insert(*cursor_pos, *ch);
            *cursor_pos += ch.len_utf8();
            changed = true;
        }
    }

    // Process special keys
    for key in &input_state.keys_just_pressed {
        match key {
            Key::Named(NamedKey::Backspace) => {
                if *cursor_pos > 0 && !value.is_empty() {
                    // Find the previous character boundary
                    let mut new_pos = *cursor_pos - 1;
                    while new_pos > 0 && !value.is_char_boundary(new_pos) {
                        new_pos -= 1;
                    }
                    value.remove(new_pos);
                    *cursor_pos = new_pos;
                    changed = true;
                }
            }
            Key::Named(NamedKey::Delete) => {
                if *cursor_pos < value.len() {
                    value.remove(*cursor_pos);
                    changed = true;
                }
            }
            Key::Named(NamedKey::ArrowLeft) => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    while *cursor_pos > 0 && !value.is_char_boundary(*cursor_pos) {
                        *cursor_pos -= 1;
                    }
                }
            }
            Key::Named(NamedKey::ArrowRight) => {
                if *cursor_pos < value.len() {
                    *cursor_pos += 1;
                    while *cursor_pos < value.len() && !value.is_char_boundary(*cursor_pos) {
                        *cursor_pos += 1;
                    }
                }
            }
            Key::Named(NamedKey::Home) => {
                *cursor_pos = 0;
            }
            Key::Named(NamedKey::End) => {
                *cursor_pos = value.len();
            }
            _ => {}
        }
    }

    changed
}

/// Check if a text input with the given ID was clicked this frame
///
/// # Arguments
/// * `input_id` - The ID of the text input to check
/// * `events` - Slice of targeted events from this frame
///
/// # Returns
/// `true` if the text input was clicked, `false` otherwise
pub fn text_input_clicked(input_id: &str, events: &[TargetedEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e.event, InteractionEvent::Click { .. }) && e.target.as_str() == input_id)
}

/// Check if a text input with the given ID received focus this frame
///
/// # Arguments
/// * `input_id` - The ID of the text input to check
/// * `events` - Slice of targeted events from this frame
///
/// # Returns
/// `true` if the text input gained focus, `false` otherwise
pub fn text_input_focused(input_id: &str, events: &[TargetedEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e.event, InteractionEvent::Focus) && e.target.as_str() == input_id)
}

/// Check if a text input with the given ID lost focus this frame
///
/// # Arguments
/// * `input_id` - The ID of the text input to check
/// * `events` - Slice of targeted events from this frame
///
/// # Returns
/// `true` if the text input lost focus, `false` otherwise
pub fn text_input_blurred(input_id: &str, events: &[TargetedEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e.event, InteractionEvent::Blur) && e.target.as_str() == input_id)
}
