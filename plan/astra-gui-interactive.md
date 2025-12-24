# Astra-GUI Interactive Components Implementation Plan

## Overview
This plan implements interactive UI components (Button, Toggle, Slider) for astra-gui using a hybrid architecture that provides low-level primitives while allowing users to build higher-level patterns on top.

**Scope**: Start with Button as proof-of-concept, then extend to Toggle and Slider.

**Architecture**: Hybrid approach - stateless component builders + external state management
- Core provides: Node IDs, hit-testing, event capture/bubble
- Interactive crate provides: Component builder functions, state structs
- Users manage: Application state, event routing

---

## Design Decisions

### 1. Architecture Pattern: Hybrid (Option C)
**Rationale**: 
- Aligns with astra-gui's existing stateless node tree approach
- Provides primitives without forcing architectural decisions
- Users can build immediate-mode OR message-based patterns on top
- Minimal core changes required

**Data Flow**:
```
User State → Component Builder → Node Tree → Layout → Render
    ↑                                           ↓
    └────────── Event System ←─────────────────┘
                (hit-test + dispatch)
```

### 2. Crate Organization: `astra-gui-interactive`
**Structure**:
- `astra-gui` (core): Add node IDs, hit-testing primitives
- `astra-gui-interactive` (new): Component builders, state structs
- `astra-gui-wgpu`: Add input event handling

### 3. Node Identity: Auto + Optional Override (Option C)
- Automatic: Hash-based ID from tree position
- Override: `with_id("custom_id")` method
- Both stored in `Node` struct

### 4. Toggle Style: iOS-style Pill Toggle (Option A)
- Rounded rect track
- Circle knob that slides
- Smooth transition animation (future enhancement)

### 5. Initial Scope: Button Proof-of-Concept (Option C)
**Phase 1**: Complete button with full interaction
**Phase 2**: Extend to toggle and slider

### 6. Event Propagation: Capture + Bubble (Option A)
- Capture phase: Root → leaf (can intercept/cancel)
- Bubble phase: Leaf → root (default handling)
- Necessary for complex UIs (modals, drag-drop, etc.)

---

## Implementation Phases

### Phase 1: Core Interaction Primitives (astra-gui)

**Goal**: Add minimal infrastructure to support interactive components

#### 1.1 Node Identity System
**File**: `crates/astra-gui/src/node.rs`

**Changes**:
```rust
pub struct Node {
    // Existing fields...
    
    // NEW: Node identification
    id: Option<NodeId>,  // User-provided or auto-generated
}

pub struct NodeId(String);

impl Node {
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(NodeId(id.into()));
        self
    }
    
    pub fn id(&self) -> Option<&NodeId> {
        self.id.as_ref()
    }
    
    // Internal: Generate stable ID from tree path
    fn generate_id(&self, parent_path: &str, index: usize) -> NodeId {
        // Hash-based stable ID
    }
}
```

**Auto-ID Generation Strategy**:
- During layout pass, assign auto-IDs to nodes without explicit IDs
- Use parent path + sibling index for stability
- Store in `ComputedLayout` for hit-testing

#### 1.2 Hit-Testing Module
**File**: `crates/astra-gui/src/hit_test.rs` (new)

**API**:
```rust
pub struct HitTestResult {
    pub node_id: NodeId,
    pub local_pos: Point,  // Position within node
    pub node_rect: Rect,
}

/// Hit-test a point against computed layout tree
/// Returns results in front-to-back order (topmost first)
pub fn hit_test_point(
    root: &Node,
    point: Point,
) -> Vec<HitTestResult> {
    // Traverse tree, check point against computed rects
    // Respect Overflow::Hidden for clipping
}

/// Find deepest node at point (most specific match)
pub fn hit_test_deepest(
    root: &Node,
    point: Point,
) -> Option<HitTestResult> {
    hit_test_point(root, point).last()
}
```

**Implementation Details**:
- Use `node.computed_layout()` for positioned rects
- Respect `Overflow::Hidden` clipping boundaries
- Return results sorted by depth (shallow to deep)

#### 1.3 Geometric Primitives
**File**: `crates/astra-gui/src/primitives.rs`

**Add**:
```rust
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y
    }
    
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        // For clipping calculations
    }
}
```

---

### Phase 2: Input Event System (astra-gui-wgpu)

**Goal**: Capture winit events and provide input state to application

#### 2.1 Input State Module
**File**: `crates/astra-gui-wgpu/src/input.rs` (new)

**API**:
```rust
use winit::event::{WindowEvent, MouseButton, ElementState};

#[derive(Debug, Clone)]
pub struct InputState {
    pub cursor_position: Option<Point>,
    pub buttons_pressed: HashSet<MouseButton>,
    pub buttons_just_pressed: HashSet<MouseButton>,  // This frame
    pub buttons_just_released: HashSet<MouseButton>, // This frame
}

impl InputState {
    pub fn new() -> Self { ... }
    
    /// Call at start of frame
    pub fn begin_frame(&mut self) {
        self.buttons_just_pressed.clear();
        self.buttons_just_released.clear();
    }
    
    /// Process winit WindowEvent
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(Point {
                    x: position.x as f32,
                    y: position.y as f32,
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.buttons_pressed.insert(*button);
                        self.buttons_just_pressed.insert(*button);
                    }
                    ElementState::Released => {
                        self.buttons_pressed.remove(button);
                        self.buttons_just_released.insert(*button);
                    }
                }
            }
            _ => {}
        }
    }
    
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.buttons_pressed.contains(&button)
    }
    
    pub fn is_button_just_pressed(&self, button: MouseButton) -> bool {
        self.buttons_just_pressed.contains(&button)
    }
}
```

#### 2.2 Event Propagation System
**File**: `crates/astra-gui-wgpu/src/events.rs` (new)

**API**:
```rust
use astra_gui::hit_test::{hit_test_point, HitTestResult};

#[derive(Debug, Clone)]
pub enum InteractionEvent {
    Click { button: MouseButton, position: Point },
    Hover { position: Point },
    DragStart { button: MouseButton, position: Point },
    DragMove { position: Point, delta: Point },
    DragEnd { button: MouseButton, position: Point },
}

#[derive(Debug, Clone)]
pub struct TargetedEvent {
    pub event: InteractionEvent,
    pub target: NodeId,
    pub target_rect: Rect,
    pub local_position: Point,
}

pub struct EventDispatcher {
    hovered_nodes: Vec<NodeId>,
    drag_state: Option<DragState>,
}

struct DragState {
    button: MouseButton,
    target: NodeId,
    start_pos: Point,
}

impl EventDispatcher {
    pub fn new() -> Self { ... }
    
    /// Process input state and generate events
    /// Returns events in capture order (root → leaf)
    pub fn dispatch(
        &mut self,
        input: &InputState,
        root: &Node,
    ) -> Vec<TargetedEvent> {
        let mut events = Vec::new();
        
        // Hit-test current cursor position
        if let Some(cursor_pos) = input.cursor_position {
            let hits = hit_test_point(root, cursor_pos);
            
            // Generate hover events
            // ...
            
            // Generate click events
            if input.is_button_just_pressed(MouseButton::Left) {
                if let Some(deepest) = hits.last() {
                    events.push(TargetedEvent {
                        event: InteractionEvent::Click {
                            button: MouseButton::Left,
                            position: cursor_pos,
                        },
                        target: deepest.node_id.clone(),
                        target_rect: deepest.node_rect,
                        local_position: deepest.local_pos,
                    });
                }
            }
            
            // Handle drag state
            // ...
        }
        
        events
    }
}
```

---

### Phase 3: Interactive Component Library (astra-gui-interactive)

**Goal**: Provide reusable component builders and state structs

#### 3.1 Crate Setup
**Create**: `crates/astra-gui-interactive/`

**Cargo.toml**:
```toml
[package]
name = "astra-gui-interactive"
version = "0.1.0"
edition = "2021"

[dependencies]
astra-gui = { path = "../astra-gui" }
```

**Structure**:
```
crates/astra-gui-interactive/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── button.rs     # Button component
│   ├── toggle.rs     # Toggle component (future)
│   └── slider.rs     # Slider component (future)
```

#### 3.2 Button Component
**File**: `crates/astra-gui-interactive/src/button.rs`

**State**:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonState {
    Idle,
    Hovered,
    Pressed,
    Disabled,
}

impl ButtonState {
    /// Update state based on interaction events
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
```

**Builder**:
```rust
use astra_gui::{Node, Content, Shape, StyledRect, Color, Size, Spacing};

pub struct ButtonStyle {
    pub idle_color: Color,
    pub hover_color: Color,
    pub pressed_color: Color,
    pub disabled_color: Color,
    pub text_color: Color,
    pub padding: Spacing,
    pub border_radius: f32,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            idle_color: Color::rgb(0.5, 0.5, 0.5),
            hover_color: Color::rgb(0.6, 0.6, 0.6),
            pressed_color: Color::rgb(0.4, 0.4, 0.5),
            disabled_color: Color::rgb(0.3, 0.3, 0.3),
            text_color: Color::rgb(1.0, 1.0, 1.0),
            padding: Spacing::uniform(8.0),
            border_radius: 4.0,
        }
    }
}

/// Create a button node
pub fn button(
    id: impl Into<String>,
    label: impl Into<String>,
    state: ButtonState,
    style: &ButtonStyle,
) -> Node {
    let bg_color = match state {
        ButtonState::Idle => style.idle_color,
        ButtonState::Hovered => style.hover_color,
        ButtonState::Pressed => style.pressed_color,
        ButtonState::Disabled => style.disabled_color,
    };
    
    Node::new()
        .with_id(id)
        .with_width(Size::FitContent)
        .with_height(Size::FitContent)
        .with_padding(style.padding)
        .with_shape(Shape::Rect(StyledRect {
            fill: bg_color,
            stroke: None,
            corner_shape: CornerShape::Round(style.border_radius),
        }))
        .with_content(Content::Text(TextContent {
            text: label.into(),
            font_size: 16.0,
            color: style.text_color,
            h_align: HorizontalAlign::Center,
            v_align: VerticalAlign::Center,
        }))
}
```

**Helper for Event Handling**:
```rust
/// Check if button was clicked this frame
pub fn button_clicked(
    button_id: &str,
    events: &[TargetedEvent],
) -> bool {
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Click { .. }) &&
        e.target.as_str() == button_id
    })
}

/// Check if button is hovered
pub fn button_hovered(
    button_id: &str,
    events: &[TargetedEvent],
) -> bool {
    events.iter().any(|e| {
        matches!(e.event, InteractionEvent::Hover { .. }) &&
        e.target.as_str() == button_id
    })
}
```

---

### Phase 4: Example Integration

**Goal**: Demonstrate interactive button in working example

#### 4.1 Interactive Button Example
**File**: `crates/astra-gui-wgpu/examples/button.rs` (new)

**Structure**:
```rust
use winit::{application::ApplicationHandler, event::WindowEvent};
use astra_gui::Node;
use astra_gui_interactive::{button, ButtonState, ButtonStyle};
use astra_gui_wgpu::{InputState, EventDispatcher};

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    text_engine: TextEngine,
    
    // Input & interaction
    input_state: InputState,
    event_dispatcher: EventDispatcher,
    
    // Application state
    counter: i32,
    button_state: ButtonState,
}

impl ApplicationHandler for App {
    fn window_event(..., event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { .. } |
            WindowEvent::MouseInput { .. } => {
                self.input_state.handle_event(&event);
                self.window.as_ref().unwrap().request_redraw();
            }
            
            WindowEvent::RedrawRequested => {
                self.render();
            }
            
            // ... other events
        }
    }
}

impl App {
    fn render(&mut self) {
        self.input_state.begin_frame();
        
        // Build UI
        let ui = self.build_ui();
        
        // Compute layout
        let (width, height) = self.window_size();
        ui.compute_layout_with_measurer(
            width, height,
            &mut self.text_engine,
        );
        
        // Generate events
        let events = self.event_dispatcher.dispatch(
            &self.input_state,
            &ui,
        );
        
        // Update button state
        let btn_hovered = button_hovered("increment_btn", &events);
        let btn_pressed = self.input_state.is_button_down(MouseButton::Left)
            && btn_hovered;
        self.button_state.update(btn_hovered, btn_pressed, true);
        
        // Handle clicks
        if button_clicked("increment_btn", &events) {
            self.counter += 1;
            println!("Button clicked! Counter: {}", self.counter);
        }
        
        // Render
        let output = FullOutput::from_node_with_measurer(&ui, &mut self.text_engine);
        self.renderer.render(..., &output);
    }
    
    fn build_ui(&self) -> Node {
        Node::new()
            .with_width(Size::Fill)
            .with_height(Size::Fill)
            .with_layout_direction(LayoutDirection::Vertical)
            .with_gap(16.0)
            .with_padding(Spacing::uniform(32.0))
            .with_child(
                // Counter display
                Node::new()
                    .with_content(Content::Text(TextContent {
                        text: format!("Count: {}", self.counter),
                        font_size: 24.0,
                        color: Color::WHITE,
                        h_align: HorizontalAlign::Center,
                        v_align: VerticalAlign::Center,
                    }))
            )
            .with_child(
                // Interactive button
                button(
                    "increment_btn",
                    "Click Me!",
                    self.button_state,
                    &ButtonStyle::default(),
                )
            )
    }
}
```

---

### Phase 5: Toggle Component (Future)

**File**: `crates/astra-gui-interactive/src/toggle.rs`

**State**:
```rust
pub struct ToggleState {
    pub value: bool,
    pub is_hovered: bool,
    pub is_animating: bool,
    pub animation_progress: f32,  // 0.0 to 1.0
}
```

**Visual Design** (iOS-style pill):
- Track: Rounded rectangle (width: 50px, height: 30px)
  - Off: Gray background
  - On: Accent color background
- Knob: Circle (diameter: 26px, 2px margin)
  - Slides from left (off) to right (on)
  - White color
- Animation: Smooth 200ms ease transition

**Builder**:
```rust
pub fn toggle(
    id: impl Into<String>,
    state: &ToggleState,
    style: &ToggleStyle,
) -> Node {
    // Build track + knob as nested nodes
    // Position knob based on animation_progress
}
```

---

### Phase 6: Slider Component (Future)

**File**: `crates/astra-gui-interactive/src/slider.rs`

**State**:
```rust
pub struct SliderState {
    pub value: f32,              // Current value
    pub range: RangeInclusive<f32>,
    pub is_dragging: bool,
    pub is_hovered: bool,
}
```

**Visual Design**:
- Track: Thin horizontal rectangle
  - Filled portion (left of thumb) in accent color
  - Unfilled portion (right of thumb) in gray
- Thumb: Circle or rounded rectangle
  - Draggable handle
  - Hover/pressed states

**Builder**:
```rust
pub fn slider(
    id: impl Into<String>,
    state: &SliderState,
    style: &SliderStyle,
) -> Node {
    // Build track (background + filled) + thumb
    // Position thumb based on value percentage
}
```

**Drag Handling**:
```rust
/// Update slider value from drag event
pub fn update_slider_from_drag(
    state: &mut SliderState,
    drag_event: &TargetedEvent,
    track_rect: Rect,
) {
    if let InteractionEvent::DragMove { position, .. } = drag_event.event {
        let local_x = position.x - track_rect.min.x;
        let percentage = (local_x / track_rect.width()).clamp(0.0, 1.0);
        state.value = state.range.start() + 
            (state.range.end() - state.range.start()) * percentage;
    }
}
```

---

## Critical Files Summary

### New Files to Create

**astra-gui (core)**:
- `crates/astra-gui/src/hit_test.rs` - Hit-testing module

**astra-gui-wgpu (backend)**:
- `crates/astra-gui-wgpu/src/input.rs` - Input state tracking
- `crates/astra-gui-wgpu/src/events.rs` - Event dispatch system

**astra-gui-interactive (new crate)**:
- `crates/astra-gui-interactive/Cargo.toml`
- `crates/astra-gui-interactive/src/lib.rs`
- `crates/astra-gui-interactive/src/button.rs`
- `crates/astra-gui-interactive/src/toggle.rs` (future)
- `crates/astra-gui-interactive/src/slider.rs` (future)

**Examples**:
- `crates/astra-gui-wgpu/examples/button.rs` - Interactive button demo

### Files to Modify

**astra-gui (core)**:
- `crates/astra-gui/src/node.rs` - Add `id` field, `with_id()` method
- `crates/astra-gui/src/primitives.rs` - Add `Point`, update `Rect`
- `crates/astra-gui/src/lib.rs` - Export new modules

**astra-gui-wgpu (backend)**:
- `crates/astra-gui-wgpu/src/lib.rs` - Export input/events modules

---

## Implementation Order

### Phase 1: Foundation (Button Proof-of-Concept)
1. ✅ Add `NodeId` and `with_id()` to `Node` (astra-gui)
2. ✅ Add `Point` primitive (astra-gui)
3. ✅ Implement `hit_test.rs` module (astra-gui)
4. ✅ Implement `input.rs` module (astra-gui-wgpu)
5. ✅ Implement `events.rs` module (astra-gui-wgpu)
6. ✅ Create `astra-gui-interactive` crate
7. ✅ Implement button component (astra-gui-interactive)
8. ✅ Create interactive button example
9. ✅ Test and refine

### Phase 2: Extend to Toggle (Future)
1. Implement toggle component
2. Add toggle to example
3. Implement animation system (optional)

### Phase 3: Extend to Slider (Future)
1. Implement slider component
2. Add drag handling utilities
3. Add slider to example

### Phase 4: Polish (Future)
1. Accessibility (keyboard navigation, ARIA labels)
2. Theming system
3. Animation framework
4. Focus management
5. More components (text input, checkbox, radio, dropdown, etc.)

---

## Technical Considerations

### 1. Event Capture vs Bubble
**Implementation**: Events generated in **bubble order** (leaf → root) by default
- `hit_test_point()` returns results shallow-to-deep
- Event handler can mark event as "consumed" to stop propagation
- Future: Add capture phase if needed (requires two-pass dispatch)

### 2. Auto-ID Generation
**Strategy**: Hash tree path during layout
```rust
fn assign_auto_ids(node: &mut Node, path: &str, index: usize) {
    if node.id.is_none() {
        let auto_id = format!("{}[{}]", path, index);
        node.id = Some(NodeId(auto_id));
    }
    for (i, child) in node.children.iter_mut().enumerate() {
        assign_auto_ids(child, &node.id.unwrap().0, i);
    }
}
```

**Issue**: Tree restructuring changes IDs
**Solution**: Encourage explicit IDs for interactive components

### 3. Performance
**Concern**: Hit-testing every frame
**Mitigation**:
- Only hit-test when mouse moves or buttons pressed
- Spatial indexing (future optimization)
- Early-out on Overflow::Hidden boundaries

### 4. Focus Management
**Deferred to Phase 4**: Full focus system complex
**Initial approach**: No keyboard nav, mouse-only

### 5. Animation
**Deferred to future**: For now, discrete state changes
**Future**: Tween values in ToggleState, smooth transitions

---

## Success Criteria

### Phase 1 (Button):
- ✅ Button renders with correct visual state (idle/hover/pressed)
- ✅ Click detection works reliably
- ✅ Example shows incrementing counter on button click
- ✅ No frame lag in interaction (feels responsive)
- ✅ Code is clean, well-documented, follows astra-gui patterns

### Phase 2 (Toggle):
- ✅ Toggle switches on click
- ✅ Visual feedback (color change)
- ✅ (Optional) Smooth animation

### Phase 3 (Slider):
- ✅ Drag to change value
- ✅ Visual thumb positioning
- ✅ Value clamped to range

---

## Open Questions / Future Work

1. **Keyboard Navigation**: Tab order, Enter to activate, arrow keys for sliders
2. **Accessibility**: Screen reader support, high contrast themes
3. **Theming**: Global vs per-component styling
4. **Layout Integration**: Should buttons auto-size based on text metrics? (Yes, via FitContent)
5. **Right-click / Context Menus**: Different event type?
6. **Touch Events**: Multi-touch, gestures
7. **Drag and Drop**: Between components, reordering lists
8. **Text Input**: Cursor positioning, selection, clipboard
9. **Tooltips**: Hover delay, positioning relative to cursor
10. **Modals/Popups**: Z-ordering, click-outside-to-close

These will be addressed in future iterations as the component library matures.
