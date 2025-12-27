//! Input state tracking for mouse and keyboard events
//!
//! This module provides structures to track input state across frames,
//! handling winit WindowEvent conversion to a form suitable for UI interaction.

use astra_gui::Point;
use std::collections::HashSet;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::Key;

/// Tracks the current state of mouse and keyboard input
///
/// This structure maintains both the current state and frame-specific events
/// (just pressed/just released) to enable easy input handling in the UI.
#[derive(Debug, Clone)]
pub struct InputState {
    /// Current cursor position in window coordinates, if known
    pub cursor_position: Option<Point>,
    /// Set of mouse buttons currently held down
    pub buttons_pressed: HashSet<MouseButton>,
    /// Set of mouse buttons that were pressed this frame
    pub buttons_just_pressed: HashSet<MouseButton>,
    /// Set of mouse buttons that were released this frame
    pub buttons_just_released: HashSet<MouseButton>,
    /// Characters typed this frame (for text input)
    pub characters_typed: Vec<char>,
    /// Keys pressed this frame
    pub keys_just_pressed: Vec<Key>,
    /// Keys released this frame
    pub keys_just_released: Vec<Key>,
    /// Whether Shift is currently held down
    pub shift_held: bool,
    /// Whether Ctrl (or Cmd on macOS) is currently held down
    pub ctrl_held: bool,
}

impl InputState {
    /// Create a new input state with no active input
    pub fn new() -> Self {
        Self {
            cursor_position: None,
            buttons_pressed: HashSet::new(),
            buttons_just_pressed: HashSet::new(),
            buttons_just_released: HashSet::new(),
            characters_typed: Vec::new(),
            keys_just_pressed: Vec::new(),
            keys_just_released: Vec::new(),
            shift_held: false,
            ctrl_held: false,
        }
    }

    /// Call at the start of each frame to clear frame-specific state
    ///
    /// This clears the "just pressed" and "just released" sets so they only
    /// contain events from the current frame.
    pub fn begin_frame(&mut self) {
        self.buttons_just_pressed.clear();
        self.buttons_just_released.clear();
        self.characters_typed.clear();
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
    }

    /// Process a winit WindowEvent and update internal state
    ///
    /// This should be called for each WindowEvent received from winit.
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(Point {
                    x: position.x as f32,
                    y: position.y as f32,
                });
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor_position = None;
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    self.buttons_pressed.insert(*button);
                    self.buttons_just_pressed.insert(*button);
                }
                ElementState::Released => {
                    self.buttons_pressed.remove(button);
                    self.buttons_just_released.insert(*button);
                }
            },
            WindowEvent::KeyboardInput { event, .. } => {
                // Track modifier keys
                use winit::keyboard::NamedKey;
                match &event.logical_key {
                    Key::Named(NamedKey::Shift) => {
                        self.shift_held = event.state == ElementState::Pressed;
                    }
                    Key::Named(NamedKey::Control) | Key::Named(NamedKey::Super) => {
                        self.ctrl_held = event.state == ElementState::Pressed;
                    }
                    _ => {}
                }

                match event.state {
                    ElementState::Pressed => {
                        if !event.repeat {
                            self.keys_just_pressed.push(event.logical_key.clone());
                        }
                        // Handle text input from key events
                        match &event.logical_key {
                            Key::Character(ref text) => {
                                // Only skip if it's a ctrl+key shortcut (ctrl+letter, but not space)
                                let is_shortcut = self.ctrl_held
                                    && text.len() == 1
                                    && text.chars().next().unwrap().is_alphabetic();
                                if !is_shortcut {
                                    for ch in text.chars() {
                                        self.characters_typed.push(ch);
                                    }
                                }
                            }
                            Key::Named(NamedKey::Space) => {
                                // Always allow space, even with modifiers
                                self.characters_typed.push(' ');
                            }
                            _ => {}
                        }
                    }
                    ElementState::Released => {
                        self.keys_just_released.push(event.logical_key.clone());
                    }
                }
            }
            _ => {
                // Ignore other events
            }
        }
    }

    /// Check if a mouse button is currently held down
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    /// Check if a mouse button was pressed this frame
    pub fn is_button_just_pressed(&self, button: MouseButton) -> bool {
        self.buttons_just_pressed.contains(&button)
    }

    /// Check if a mouse button was released this frame
    pub fn is_button_just_released(&self, button: MouseButton) -> bool {
        self.buttons_just_released.contains(&button)
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
