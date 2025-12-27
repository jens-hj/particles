//! Text input component for interactive UI
//!
//! Provides an editable text input field with cursor, selection, and keyboard support.

use astra_gui::{
    catppuccin::mocha, Color, Content, CornerShape, HorizontalAlign, Layout, Node, Offset, Rect,
    Shape, Size, Spacing, Style, StyledRect, TextContent, Transition, VerticalAlign,
};
use astra_gui_wgpu::{InteractionEvent, Key, NamedKey, TargetedEvent};
use std::time::Duration;

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
    pub thickness: f32,
    /// Blink interval (time between blinks)
    pub blink_interval: Duration,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Line,
            color: None, // Use text color
            thickness: 2.0,
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
    /// Selection highlight color
    pub selection_color: Color,
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
            selection_color: mocha::LAVENDER.with_alpha(0.3),
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
/// * `selection_range` - Optional (start, end) byte positions for text selection
/// * `measurer` - ContentMeasurer for calculating text width
/// * `event_dispatcher` - EventDispatcher for managing cursor blink state
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
    selection_range: Option<(usize, usize)>,
    measurer: &mut impl astra_gui::ContentMeasurer,
    event_dispatcher: &mut astra_gui_wgpu::EventDispatcher,
) -> Node {
    let id_string = id.into();
    let node_id = astra_gui::NodeId::new(&id_string);
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

    // Update cursor blink state if focused
    let cursor_visible = if focused {
        event_dispatcher.update_cursor_blink(&node_id, style.cursor_style.blink_interval)
    } else {
        false
    };

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

    let mut children = vec![];

    // Add selection highlight if there is a selection range
    if let Some((start, end)) = selection_range {
        if start < end && !value_str.is_empty() {
            // Calculate selection start position
            let text_before_selection = value_str.chars().take(start).collect::<String>();
            let selection_x_offset = if !text_before_selection.is_empty() {
                let text_width = measurer.measure_text(astra_gui::MeasureTextRequest {
                    text: &text_before_selection,
                    font_size: style.font_size,
                    h_align: HorizontalAlign::Left,
                    v_align: VerticalAlign::Center,
                    family: None,
                });
                text_width.width
            } else {
                0.0
            };

            // Calculate selection width
            let selected_text = value_str
                .chars()
                .skip(start)
                .take(end - start)
                .collect::<String>();
            let selection_width = if !selected_text.is_empty() {
                let text_width = measurer.measure_text(astra_gui::MeasureTextRequest {
                    text: &selected_text,
                    font_size: style.font_size,
                    h_align: HorizontalAlign::Left,
                    v_align: VerticalAlign::Center,
                    family: None,
                });
                text_width.width
            } else {
                0.0
            };

            // Add selection rectangle
            children.push(
                Node::new()
                    .with_width(Size::px(selection_width))
                    .with_height(Size::px(style.font_size))
                    .with_offset(Offset::x(selection_x_offset))
                    .with_shape(Shape::Rect(StyledRect {
                        rect: Rect::default(),
                        corner_shape: CornerShape::Round(2.0), // Slightly rounded
                        fill: style.selection_color,
                        stroke: None,
                    })),
            );
        }
    }

    // Text content
    children.push(
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
    );

    // Add cursor if focused and visible
    if focused && cursor_visible && !disabled {
        let cursor_node = match style.cursor_style.shape {
            CursorShape::Line => {
                // Vertical line cursor
                Node::new()
                    .with_width(Size::px(style.cursor_style.thickness))
                    .with_height(Size::px(style.font_size))
                    .with_offset(Offset::x(cursor_x_offset))
                    .with_shape(Shape::Rect(StyledRect::new(Rect::default(), cursor_color)))
            }
            CursorShape::Underline => {
                // Underline cursor - measure character width at cursor
                let cursor_width = if cursor_position == 0 || cursor_position == value_str.len() {
                    style.font_size * 0.6
                } else {
                    let char_at_cursor = value_str.chars().nth(cursor_position).unwrap_or(' ');
                    let char_width = measurer
                        .measure_text(astra_gui::MeasureTextRequest {
                            text: &char_at_cursor.to_string(),
                            font_size: style.font_size,
                            h_align: HorizontalAlign::Left,
                            v_align: VerticalAlign::Center,
                            family: None,
                        })
                        .width;
                    char_width
                };

                Node::new()
                    .with_width(Size::px(cursor_width))
                    .with_height(Size::px(style.cursor_style.thickness))
                    .with_offset(Offset::new(cursor_x_offset, style.font_size))
                    .with_shape(Shape::Rect(StyledRect::new(Rect::default(), cursor_color)))
            }
            CursorShape::Block => {
                // Block cursor - measure character width at cursor
                let cursor_width = if cursor_position == 0 {
                    style.font_size * 0.6
                } else {
                    let char_at_cursor = value_str.chars().nth(cursor_position - 1).unwrap_or(' ');
                    let char_width = measurer
                        .measure_text(astra_gui::MeasureTextRequest {
                            text: &char_at_cursor.to_string(),
                            font_size: style.font_size,
                            h_align: HorizontalAlign::Left,
                            v_align: VerticalAlign::Center,
                            family: None,
                        })
                        .width;
                    char_width
                };

                Node::new()
                    .with_width(Size::px(cursor_width))
                    .with_height(Size::px(style.font_size))
                    .with_offset(Offset::x((cursor_x_offset - cursor_width).max(0.0)))
                    .with_shape(Shape::Rect(StyledRect::new(
                        Rect::default(),
                        cursor_color.with_alpha(0.3), // Semi-transparent
                    )))
            }
        };
        children.push(cursor_node);
    }

    // Add hitbox node to capture all clicks (including on text)
    children.push(
        Node::new()
            .with_id(astra_gui::NodeId::new(format!("{}_hitbox", id_string)))
            .with_width(Size::Fill)
            .with_height(Size::Fill)
            .with_disabled(disabled),
    );

    Node::new()
        .with_id(node_id)
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

/// Find the next word boundary to the left (backward)
fn find_prev_word_boundary(text: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }

    let mut new_pos = pos;

    // Move back one character first
    new_pos -= 1;
    while new_pos > 0 && !text.is_char_boundary(new_pos) {
        new_pos -= 1;
    }

    // Skip whitespace
    while new_pos > 0
        && text[..new_pos]
            .chars()
            .last()
            .map_or(false, |c| c.is_whitespace())
    {
        new_pos -= 1;
        while new_pos > 0 && !text.is_char_boundary(new_pos) {
            new_pos -= 1;
        }
    }

    // Skip non-whitespace (the word itself)
    while new_pos > 0 {
        let prev_char = text[..new_pos].chars().last();
        if prev_char.map_or(false, |c| c.is_whitespace()) {
            break;
        }
        new_pos -= 1;
        while new_pos > 0 && !text.is_char_boundary(new_pos) {
            new_pos -= 1;
        }
    }

    new_pos
}

/// Find the next word boundary to the right (forward)
fn find_next_word_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }

    let mut new_pos = pos;
    let chars: Vec<char> = text.chars().collect();
    let mut char_idx = text[..pos].chars().count();

    // Skip current word (non-whitespace)
    while char_idx < chars.len() && !chars[char_idx].is_whitespace() {
        new_pos += chars[char_idx].len_utf8();
        char_idx += 1;
    }

    // Skip whitespace
    while char_idx < chars.len() && chars[char_idx].is_whitespace() {
        new_pos += chars[char_idx].len_utf8();
        char_idx += 1;
    }

    new_pos
}

/// Handle text input keyboard events, focus/unfocus, and update the value
///
/// Call this each frame with the events and input state. This function handles:
/// - Clicking on the text input to focus it
/// - Clicking outside or pressing ESC to unfocus it
/// - Keyboard input when focused
/// - Cursor blink state management
/// - Text selection with Shift+arrows and Ctrl/Cmd+A
///
/// # Arguments
/// * `input_id` - The ID of the text input
/// * `value` - Current text value (will be modified if keys are pressed)
/// * `cursor_pos` - Current cursor position (byte offset, will be modified)
/// * `selection_range` - Optional selection range (start, end) - will be modified for keyboard selection
/// * `events` - Slice of targeted events from this frame
/// * `input_state` - Current input state (for keyboard and mouse input)
/// * `event_dispatcher` - EventDispatcher for managing focus and cursor blink
///
/// # Returns
/// `true` if the value was changed, `false` otherwise
pub fn text_input_update(
    input_id: &str,
    value: &mut String,
    cursor_pos: &mut usize,
    selection_range: &mut Option<(usize, usize)>,
    events: &[TargetedEvent],
    input_state: &astra_gui_wgpu::InputState,
    event_dispatcher: &mut astra_gui_wgpu::EventDispatcher,
) -> bool {
    let node_id = astra_gui::NodeId::new(input_id);
    let mut changed = false;

    // Handle focus on click
    if text_input_clicked(input_id, events) {
        event_dispatcher.set_focus(Some(node_id.clone()));
    }

    // Handle unfocus: clicking outside or pressing ESC
    use astra_gui_wgpu::MouseButton;
    let mouse_clicked_outside = input_state.is_button_just_pressed(MouseButton::Left)
        && !text_input_clicked(input_id, events);

    let escape_pressed = input_state.keys_just_pressed.iter().any(|key| {
        matches!(
            key,
            astra_gui_wgpu::Key::Named(astra_gui_wgpu::NamedKey::Escape)
        )
    });

    let currently_focused = event_dispatcher
        .focused_node()
        .map(|id| id == &node_id)
        .unwrap_or(false);

    if (mouse_clicked_outside || escape_pressed) && currently_focused {
        event_dispatcher.set_focus(None);
    }

    let focused = event_dispatcher
        .focused_node()
        .map(|id| id == &node_id)
        .unwrap_or(false);

    // Only process keyboard input if focused
    if !focused {
        return false;
    }

    // Check if shift is held for selection
    let shift_held = input_state.shift_held;
    let ctrl_held = input_state.ctrl_held;

    // Track selection anchor point (where selection started)
    let selection_anchor = if let Some((start, end)) = *selection_range {
        // If cursor is at end, anchor is start; if cursor is at start, anchor is end
        if *cursor_pos == end {
            Some(start)
        } else {
            Some(end)
        }
    } else {
        None
    };

    // Process typed characters
    for ch in &input_state.characters_typed {
        // Delete selection if exists before inserting
        if let Some((start, end)) = *selection_range {
            if start < end {
                value.replace_range(start..end, "");
                *cursor_pos = start;
                *selection_range = None;
                changed = true;
            }
        }

        // Insert character at cursor position
        if *cursor_pos <= value.len() {
            value.insert(*cursor_pos, *ch);
            *cursor_pos += ch.len_utf8();
            changed = true;
            event_dispatcher.reset_cursor_blink(&node_id); // Reset cursor blink on edit
        }
    }

    // Process special keys
    for key in &input_state.keys_just_pressed {
        match key {
            // Ctrl/Cmd+A: Select all
            Key::Character(ref ch) if ch == "a" && ctrl_held => {
                if !value.is_empty() {
                    *selection_range = Some((0, value.len()));
                    *cursor_pos = value.len();
                    event_dispatcher.reset_cursor_blink(&node_id);
                }
            }
            Key::Named(NamedKey::Backspace) => {
                // Delete selection if exists
                if let Some((start, end)) = *selection_range {
                    if start < end {
                        value.replace_range(start..end, "");
                        *cursor_pos = start;
                        *selection_range = None;
                        changed = true;
                        event_dispatcher.reset_cursor_blink(&node_id);
                    }
                } else if *cursor_pos > 0 && !value.is_empty() {
                    if ctrl_held {
                        // Delete from cursor to previous word boundary
                        let new_pos = find_prev_word_boundary(value, *cursor_pos);
                        value.replace_range(new_pos..*cursor_pos, "");
                        *cursor_pos = new_pos;
                    } else {
                        // Delete one character backward
                        let mut new_pos = *cursor_pos - 1;
                        while new_pos > 0 && !value.is_char_boundary(new_pos) {
                            new_pos -= 1;
                        }
                        value.remove(new_pos);
                        *cursor_pos = new_pos;
                    }
                    changed = true;
                    event_dispatcher.reset_cursor_blink(&node_id);
                }
            }
            Key::Named(NamedKey::Delete) => {
                // Delete selection if exists
                if let Some((start, end)) = *selection_range {
                    if start < end {
                        value.replace_range(start..end, "");
                        *cursor_pos = start;
                        *selection_range = None;
                        changed = true;
                        event_dispatcher.reset_cursor_blink(&node_id);
                    }
                } else if *cursor_pos < value.len() {
                    if ctrl_held {
                        // Delete from cursor to next word boundary
                        let new_pos = find_next_word_boundary(value, *cursor_pos);
                        value.replace_range(*cursor_pos..new_pos, "");
                    } else {
                        // Delete one character forward
                        value.remove(*cursor_pos);
                    }
                    changed = true;
                    event_dispatcher.reset_cursor_blink(&node_id);
                }
            }
            Key::Named(NamedKey::ArrowLeft) => {
                if *cursor_pos > 0 {
                    let old_pos = *cursor_pos;

                    if ctrl_held {
                        // Jump to previous word boundary
                        *cursor_pos = find_prev_word_boundary(value, *cursor_pos);
                    } else {
                        // Move one character left
                        *cursor_pos -= 1;
                        while *cursor_pos > 0 && !value.is_char_boundary(*cursor_pos) {
                            *cursor_pos -= 1;
                        }
                    }

                    if shift_held {
                        // Extend or create selection
                        if let Some(anchor) = selection_anchor {
                            *selection_range = Some(if *cursor_pos < anchor {
                                (*cursor_pos, anchor)
                            } else {
                                (anchor, *cursor_pos)
                            });
                        } else {
                            *selection_range = Some((*cursor_pos, old_pos));
                        }
                    } else {
                        // Clear selection if not holding shift
                        *selection_range = None;
                    }

                    event_dispatcher.reset_cursor_blink(&node_id);
                }
            }
            Key::Named(NamedKey::ArrowRight) => {
                if *cursor_pos < value.len() {
                    let old_pos = *cursor_pos;

                    if ctrl_held {
                        // Jump to next word boundary
                        *cursor_pos = find_next_word_boundary(value, *cursor_pos);
                    } else {
                        // Move one character right
                        *cursor_pos += 1;
                        while *cursor_pos < value.len() && !value.is_char_boundary(*cursor_pos) {
                            *cursor_pos += 1;
                        }
                    }

                    if shift_held {
                        // Extend or create selection
                        if let Some(anchor) = selection_anchor {
                            *selection_range = Some(if *cursor_pos < anchor {
                                (*cursor_pos, anchor)
                            } else {
                                (anchor, *cursor_pos)
                            });
                        } else {
                            *selection_range = Some((old_pos, *cursor_pos));
                        }
                    } else {
                        // Clear selection if not holding shift
                        *selection_range = None;
                    }

                    event_dispatcher.reset_cursor_blink(&node_id);
                }
            }
            Key::Named(NamedKey::Home) => {
                let old_pos = *cursor_pos;
                *cursor_pos = 0;

                if shift_held {
                    if let Some(anchor) = selection_anchor {
                        *selection_range = Some((0, anchor));
                    } else {
                        *selection_range = Some((0, old_pos));
                    }
                } else {
                    *selection_range = None;
                }

                event_dispatcher.reset_cursor_blink(&node_id);
            }
            Key::Named(NamedKey::End) => {
                let old_pos = *cursor_pos;
                *cursor_pos = value.len();

                if shift_held {
                    if let Some(anchor) = selection_anchor {
                        *selection_range = Some(if value.len() > anchor {
                            (anchor, value.len())
                        } else {
                            (value.len(), anchor)
                        });
                    } else {
                        *selection_range = Some((old_pos, value.len()));
                    }
                } else {
                    *selection_range = None;
                }

                event_dispatcher.reset_cursor_blink(&node_id);
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
    let hitbox_id = format!("{}_hitbox", input_id);
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Click { .. })
            && (e.target.as_str() == input_id || e.target.as_str() == hitbox_id)
    })
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
