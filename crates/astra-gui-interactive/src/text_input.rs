//! Text input component for interactive UI
//!
//! Provides an editable text input field with cursor, selection, and keyboard support.

use astra_gui::{
    catppuccin::mocha, Color, Content, CornerShape, HorizontalAlign, Layout, Node, NodeId, Offset,
    Rect, Shape, Size, Spacing, Style, StyledRect, TextContent, Transition, VerticalAlign,
};
use astra_gui_wgpu::{InteractionEvent, Key, NamedKey, TargetedEvent};
use std::time::{Duration, Instant};

/// Cursor shape for text input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    /// Vertical line (classic text cursor)
    Line,
    /// Underline under the character
    Underline,
    /// Block covering the character
    Block,
}

/// Cursor/caret styling for text input
#[derive(Debug, Clone)]
pub struct CursorStyle {
    /// Shape of the cursor
    pub shape: CursorShape,
    /// Cursor color (if None, uses text color)
    pub color: Option<Color>,
    /// Cursor width (for Line shape)
    pub width: f32,
    /// Blink interval (time between blinks)
    pub blink_interval: Duration,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Line,
            color: None, // Use text color
            width: 2.0,
            blink_interval: Duration::from_millis(530), // Standard blink rate
        }
    }
}

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
    /// Cursor/caret styling
    pub cursor_style: CursorStyle,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            idle_color: mocha::SURFACE0,
            focused_color: mocha::SURFACE1,
            disabled_color: mocha::SURFACE0.with_alpha(0.5),
            text_color: mocha::TEXT,
            placeholder_color: mocha::SUBTEXT0,
            disabled_text_color: mocha::SUBTEXT0,
            padding: Spacing::symmetric(16.0, 12.0),
            border_radius: 8.0,
            font_size: 24.0,
            cursor_style: CursorStyle::default(),
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
/// * `cursor_position` - Character index where the cursor should be positioned
/// * `cursor_visible` - Whether the cursor should be visible (for blink animation)
/// * `measurer` - ContentMeasurer for calculating text width
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
    cursor_position: usize,
    cursor_visible: bool,
    measurer: &mut impl astra_gui::ContentMeasurer,
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

    // Determine cursor color (falls back to text color)
    let cursor_color = style.cursor_style.color.unwrap_or(style.text_color);

    // Calculate cursor x position by measuring text up to cursor position
    let text_before_cursor = value_str.chars().take(cursor_position).collect::<String>();
    let cursor_x_offset = if !text_before_cursor.is_empty() {
        let text_width = measurer.measure_text(astra_gui::MeasureTextRequest {
            text: &text_before_cursor,
            font_size: style.font_size,
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Center,
            family: None,
        });
        text_width.width
    } else {
        0.0
    };

    let mut children = vec![
        // Text content
        Node::new()
            .with_width(Size::Fill)
            .with_height(Size::Fill)
            .with_content(Content::Text(TextContent {
                text: display_text,
                font_size: style.font_size,
                color: text_color,
                h_align: HorizontalAlign::Left,
                v_align: VerticalAlign::Center,
            }))
            .with_style(Style {
                text_color: Some(text_color),
                ..Default::default()
            })
            .with_disabled_style(Style {
                text_color: Some(style.disabled_text_color),
                ..Default::default()
            })
            .with_disabled(disabled)
            .with_transition(Transition::quick()),
    ];

    // Add cursor if focused and visible
    if focused && cursor_visible && !disabled {
        let cursor_node = match style.cursor_style.shape {
            CursorShape::Line => {
                // Vertical line cursor
                Node::new()
                    .with_width(Size::px(style.cursor_style.width))
                    .with_height(Size::px(style.font_size))
                    .with_offset(Offset::x(cursor_x_offset))
                    .with_shape(Shape::Rect(StyledRect::new(Rect::default(), cursor_color)))
            }
            CursorShape::Underline => {
                // Underline cursor
                Node::new()
                    .with_width(Size::px(style.font_size * 0.6))
                    .with_height(Size::px(style.cursor_style.width))
                    .with_offset(Offset::new(
                        cursor_x_offset,
                        style.padding.top + style.font_size * 0.8,
                    ))
                    .with_shape(Shape::Rect(StyledRect::new(Rect::default(), cursor_color)))
            }
            CursorShape::Block => {
                // Block cursor
                Node::new()
                    .with_width(Size::px(style.font_size * 0.6))
                    .with_height(Size::px(style.font_size))
                    .with_offset(Offset::x(cursor_x_offset))
                    .with_shape(Shape::Rect(StyledRect::new(
                        Rect::default(),
                        cursor_color.with_alpha(0.3), // Semi-transparent
                    )))
            }
        };
        children.push(cursor_node);
    }

    Node::new()
        .with_id(NodeId::new(id))
        .with_width(Size::px(300.0))
        .with_height(Size::px(style.font_size + style.padding.get_vertical()))
        .with_padding(style.padding)
        .with_layout_direction(Layout::Stack)
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
        .with_style(Style {
            fill_color: Some(if focused {
                style.focused_color
            } else {
                style.idle_color
            }),
            ..Default::default()
        })
        .with_disabled_style(Style {
            fill_color: Some(style.disabled_color),
            ..Default::default()
        })
        .with_disabled(disabled)
        .with_transition(Transition::quick())
        .with_disabled(disabled)
        .with_children(children)
}

/// Cursor blink state tracker
///
/// Maintains the blink animation state for a text input cursor.
/// Should be stored in your application state and updated each frame.
#[derive(Debug, Clone)]
pub struct CursorBlinkState {
    last_blink: Instant,
    visible: bool,
}

impl CursorBlinkState {
    /// Create a new cursor blink state (starts visible)
    pub fn new() -> Self {
        Self {
            last_blink: Instant::now(),
            visible: true,
        }
    }

    /// Update the blink state based on time and return whether cursor should be visible
    ///
    /// Call this each frame when the text input is focused.
    pub fn update(&mut self, blink_interval: Duration) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_blink) >= blink_interval {
            self.visible = !self.visible;
            self.last_blink = now;
        }
        self.visible
    }

    /// Reset the blink state (makes cursor visible and restarts blink timer)
    ///
    /// Call this when the user types or moves the cursor to ensure visibility.
    pub fn reset(&mut self) {
        self.visible = true;
        self.last_blink = Instant::now();
    }

    /// Get the current visibility state without updating
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for CursorBlinkState {
    fn default() -> Self {
        Self::new()
    }
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
/// * `cursor_blink` - Cursor blink state (will be reset on any edit)
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
    cursor_blink: &mut CursorBlinkState,
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
            cursor_blink.reset(); // Reset cursor blink on edit
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
                    cursor_blink.reset();
                }
            }
            Key::Named(NamedKey::Delete) => {
                if *cursor_pos < value.len() {
                    value.remove(*cursor_pos);
                    changed = true;
                    cursor_blink.reset();
                }
            }
            Key::Named(NamedKey::ArrowLeft) => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    while *cursor_pos > 0 && !value.is_char_boundary(*cursor_pos) {
                        *cursor_pos -= 1;
                    }
                    cursor_blink.reset();
                }
            }
            Key::Named(NamedKey::ArrowRight) => {
                if *cursor_pos < value.len() {
                    *cursor_pos += 1;
                    while *cursor_pos < value.len() && !value.is_char_boundary(*cursor_pos) {
                        *cursor_pos += 1;
                    }
                    cursor_blink.reset();
                }
            }
            Key::Named(NamedKey::Home) => {
                *cursor_pos = 0;
                cursor_blink.reset();
            }
            Key::Named(NamedKey::End) => {
                *cursor_pos = value.len();
                cursor_blink.reset();
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
