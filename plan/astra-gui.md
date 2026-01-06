# Replace egui UI with astra-gui (concrete plan based on astra-gui sources + examples)

Replace the current egui-based UI with `astra-gui` + `astra-gui-wgpu`, enabling the upgrade from `wgpu 0.27` to `wgpu 0.28`.

This plan is written strictly from what exists in:
- `astra-gui/crates/astra-gui-interactive/src/*`
- `astra-gui/crates/astra-gui-wgpu/examples/*` and `astra-gui/crates/astra-gui-wgpu/src/*`

## Status
**Status**: Updated plan (grounded in actual APIs)  
**Estimated effort**: ~1–2 days  
**Risk**: Low–Medium (wgpu upgrade + input interoperability)

---

## Key corrections vs the previous draft (important)

1. **No `PositionType::Absolute` exists** in astra-gui.
   - Anchoring/panels are done via `Layout::Stack` + `with_h_align(...)` + `with_v_align(...)` and/or `Translation`.
   - `Layout` is exactly: `Layout::{Horizontal, Vertical, Stack}`.
   - Alignment enums are exactly: `HorizontalAlign::{Left, Center, Right}`, `VerticalAlign::{Top, Center, Bottom}`.
   - For a screen overlay root: `Layout::Stack` then a child aligned `Top+Right` is a “top-right panel”.

2. **We do not need custom widgets** for this migration.
   - `astra-gui-interactive` already provides:
     - `button` + `button_clicked`
     - `toggle` + `toggle_clicked`
     - `slider` + `slider_drag` (supports `step: Option<f32>`)
     - `drag_value` + `drag_value_update` (includes click-to-edit, Shift/Ctrl modifiers, Enter/Escape behavior)
     - `slider_with_value` + `slider_with_value_update` (egui-like: slider with a value field to the right)
     - `collapsible` + `collapsible_clicked` (animated expand/collapse)
     - `text_input` + `text_input_update` (used internally by drag_value, or can be used directly)

3. **Event flow is explicit** and matches `astra-gui-wgpu`’s example runner:
   - Build the `Node` tree each frame.
   - Inject transition dimension overrides (from `InteractiveStateManager`) *before layout*.
   - Compute layout (`compute_layout` or `compute_layout_with_measurer`).
   - Dispatch events (`EventDispatcher::dispatch`) which uses `hit_test_point` internally.
   - Update transitions (`InteractiveStateManager::update_transitions`).
   - Handle returned `TargetedEvent`s in your app code to mutate state.
   - Clear input end-of-frame via `InputState::begin_frame()` (yes, “begin_frame” is called at END in the examples).

---

## Relevant astra-gui primitives you will use

### Layout & positioning (real API)
- Panels/overlays:
  - Root: `Layout::Stack`
  - Each overlay panel: `with_h_align(...)` + `with_v_align(...)`
  - Insets: use `with_margin(Spacing::...)` and/or `with_padding(...)`
- Manual offset:
  - `with_translation(Translation::new(Size::Logical(x), Size::Logical(y)))` (see `examples/translation.rs`)
- Z ordering:
  - `with_z_index(ZIndex(...))` / `ZIndex::OVERLAY` (see `examples/zindex.rs`)

### Interactivity (real API)
- Click:
  - `button_clicked("id", events)`
  - `toggle_clicked("id", events)`
  - `collapsible_clicked("id", events)` (targets `"{id}_header"` and `"{id}_indicator"`)
- Drag:
  - `slider_drag("id", &mut value, &range, events, &SliderStyle::default(), step)`
- Complex slider:
  - `slider_with_value_update(...)` handles both slider drag + drag-value edit/drag
- Focus management:
  - Driven by `EventDispatcher` (see `drag_value_update` and example `text_input` usage)

### Input / dispatching (real API)
- Use:
  - `astra_gui_wgpu::InputState`
  - `astra_gui_wgpu::EventDispatcher`
  - `astra_gui_wgpu::InteractiveStateManager`
- Dispatch:
  - `event_dispatcher.dispatch(&input_state, &mut ui_node)` returns `(Vec<TargetedEvent>, HashMap<NodeId, InteractionState>)`

---

## Implementation plan (simplified, concrete)

### Phase 0 — Inventory the existing egui UI (particles repo)
Goal: enumerate controls and state that must be migrated.
- Identify the current `UiState` / “gui state” struct fields:
  - Panel collapsed states
  - Slider values / toggles
  - Time controls (pause/step/dt/steps)
  - Stats values being displayed
- Identify how input is currently blocked from camera control when UI is used.

Deliverable:
- A checklist of panels and their controls (already roughly known, but confirm exact fields/IDs).

---

### Phase 1 — Dependencies: stop pinning wgpu to egui
Goal: make `wgpu = 0.28` possible and remove egui UI dependencies.

1. Update workspace dependency:
   - In `particles/Cargo.toml`: `wgpu = "28.0"` (or the precise `0.28.x` that matches your lockfile expectations).
2. Remove `egui`, `egui-wgpu`, `egui-winit` where not needed.
3. Ensure `astra-gui` + `astra-gui-wgpu` are present (already are in workspace deps).
4. Run the usual checks after each step:
   - `cargo check`

Deliverable:
- Project compiles without egui UI integration.

---

### Phase 2 — Add an astra-gui “UI system” struct (modeled after examples)
Goal: match the proven structure from `astra-gui-wgpu/examples/shared`.

Create a new struct in `particles` (likely in `src/gui.rs` or a new module) similar to:

- `input_state: InputState`
- `event_dispatcher: EventDispatcher`
- `state_manager: InteractiveStateManager`
- `text_engine: astra_gui_text::Engine` (needed because your UI has text and FitContent)
- plus app-owned UI state you already have (`UiState` remains your source of truth)

Core per-frame responsibilities (mirroring `examples/shared/runner.rs`):
1. `state_manager.begin_frame()` (start transitions timing)
2. `build_ui(...) -> Node`
3. `state_manager.inject_dimension_overrides(&mut ui)` (before layout)
4. layout:
   - `ui.compute_layout_with_measurer(window_rect, &mut text_engine)`
5. dispatch:
   - `(events, interaction_states) = event_dispatcher.dispatch(&input_state, &mut ui)`
6. transitions:
   - `state_manager.update_transitions(&mut ui, &interaction_states)`
7. app handles events:
   - `handle_events(&events, ui_state, ...)`
8. output:
   - `FullOutput::from_laid_out_node(ui, (w, h), debug_options)`
9. render via `astra-gui-wgpu` renderer in your render pipeline

Deliverable:
- A frame renders an astra-gui UI tree (start with a single label).

---

### Phase 3 — Build panels using Stack + align (no absolute positioning)
Goal: reproduce the 4 “windows” as overlay panels with correct anchors.

Build a root UI like:

- Root: `Layout::Stack`, fill width/height.
- Child 1: main “overlay layer” (could just be the panels as children).
- Each panel node:
  - `with_h_align(...)`, `with_v_align(...)`
  - `with_margin(...)` for inset from edges
  - `with_padding(...)` internal padding
  - `with_style(Style { fill_color, stroke, corner_shape, ... })` for panel background

Panel anchors:
- Statistics panel: `Top + Right`
- Physics controls: `Bottom + Left`
- Time controls: `Bottom + Right`
- Atom card: `Top + Center` (conditionally inserted)

Deliverable:
- All panels appear in the correct places even on resize (because align is relative).

---

### Phase 4 — Replace controls with existing interactive components
Goal: replace all egui widgets with `astra-gui-interactive`.

#### Toggles (checkbox replacements)
Use:
- `toggle("show_shells_toggle", ui_state.show_shells, disabled, &ToggleStyle::default())`
- In `handle_events`: if `toggle_clicked("show_shells_toggle", events)` then flip boolean.

#### Buttons
Use:
- `button("pause_btn", "...", disabled, &ButtonStyle::default())`
- In `handle_events`: if `button_clicked("pause_btn", events)` then toggle pause.

#### Sliders (simple)
Use:
- `slider("dt_slider", ui_state.dt, min..=max, disabled, &SliderStyle::default())`
- In `handle_events`:
  - `if slider_drag("dt_slider", &mut ui_state.dt, &(min..=max), events, &SliderStyle::default(), Some(step)) { ... }`
- Note: step snapping is supported by `slider_drag(..., step: Option<f32>)`.

#### Slider with value (recommended for most numeric controls)
Use:
- `slider_with_value(...)` to match the “egui slider with value beside it” UX.
- Maintain per-widget state needed by `slider_with_value_update`:
  - `text_buffer: String`
  - `cursor_pos: usize`
  - `selection: Option<(usize, usize)>`
  - `focused: bool`
  - `drag_accumulator: f32`
- This is how the astra example `examples/slider_with_value.rs` structures it.

Practical approach:
- Create a small struct in `particles` similar to the example’s `SliderWithValueState`.
- Store a `HashMap<&'static str, SliderWithValueState>` or explicit fields for each slider you have (explicit fields are faster and simpler in Rust if count is fixed).

Deliverable:
- Every numeric control is implemented using `slider_with_value` unless there’s a strong reason not to.

---

### Phase 5 — Collapsible panels (existing, animated)
Goal: keep your current UX; no “future work” required.

Use:
- `collapsible("stats_panel", "Statistics", expanded, disabled, children, &CollapsibleStyle::default())`
- Store `expanded: bool` for each collapsible in your `UiState` or a UI-only state struct.
- In `handle_events`:
  - if `collapsible_clicked("stats_panel", events)` then `expanded = !expanded`.

Notes from real implementation:
- `collapsible` builds a header with IDs `"{id}_header"` and `"{id}_indicator"` and a content wrapper with animated height changes.
- The content wrapper uses `Overflow::Hidden` with height toggling `FitContent` vs `0` (so visuals animate nicely with the transition system).

Deliverable:
- All 3 collapsible panels behave like before (and animate).

---

### Phase 6 — Camera / simulation input blocking (grounded approach)
Goal: if user interacts with UI, don’t also rotate camera / pick.

What astra-gui-wgpu actually does:
- Event dispatch uses hit-testing (`hit_test_point`) and targets the deepest node under cursor; it does not “consume” the winit event at the OS level for you.

So in `particles`, do this:
- Always feed winit events into `input_state.handle_event(&WindowEvent)` (like the example runner).
- Before processing camera interactions (mouse down/drag/wheel), check if the cursor is currently over any UI node:
  - You can do this by performing a hit test on the laid-out root UI node at the cursor position.
  - The concrete API for hit test is `astra_gui::hit_test_point(root, cursor_pos)` (used by EventDispatcher).
- If hit-test returns *any* hit with an ID (or any hit at all, depending on your policy), skip camera handling for that event/frame.

Deliverable:
- Clicking/dragging on UI does not rotate camera or select particles.

---

### Phase 7 — Keyboard shortcuts (Space, Ctrl+Right/D)
Goal: preserve egui shortcuts.

Implementation approach:
- Handle shortcuts at your winit event level (as you likely already do), but:
  - If a text input is focused (`event_dispatcher.focused_node().is_some()`), consider suppressing global shortcuts so typing/editing works.
  - This mirrors `ExampleApp::handle_escape()` behavior in `examples/shared/example_app.rs`.

Deliverable:
- Space toggles pause; Ctrl+Right/D steps, but not while editing a value field.

---

## Concrete UI structure proposal (no invented APIs)

Use a single `build_ui(width, height) -> Node` that returns:

- Root (Stack, Fill)
  - A “canvas root” node (your world is rendered separately; UI is overlay)
  - Panels layer:
    - Top-right: Statistics collapsible
    - Bottom-left: Physics collapsible (default collapsed)
    - Bottom-right: Time collapsible (default expanded)
    - Top-center: Atom card (conditional)

Each panel is:
- A container node with:
  - `with_layout_direction(Layout::Vertical)`
  - `with_gap(Size::lpx(...))`
  - `with_padding(Spacing::...)`
  - `with_style(Style { fill_color, stroke, corner_shape, .. })`
- Inside: use `collapsible(...)` to wrap panel content.

---

## Logarithmic slider
astra-gui’s slider is linear; there is no built-in log scale.
Keep the existing mapping, but pair it with `slider_with_value`:
- Present a linear “UI value” (0..1) or a log-mapped value directly.
- Convert in `handle_events` when `slider_drag` / `slider_with_value_update` reports a change.

Deliverable:
- Gravity slider behaves like before.

---

## Testing checklist (practical)
- All panels positioned correctly at different window sizes.
- Collapsibles expand/collapse with animation.
- Slider stepping works (verify extremes: the slider logic snaps to min/max near edges).
- DragValue click-to-edit works:
  - Enter commits
  - Escape cancels
  - Shift/Ctrl drag modifiers work
- Camera controls never trigger when cursor is over UI.
- No warnings; run:
  - `cargo fmt`
  - `cargo check`
  - `cargo test`
  - `cargo run`

---

## Notes on performance
astra-gui is designed to be very fast; still:
- Prefer `slider_with_value` and reuse per-widget state structs (avoid allocating strings every frame if you can; keep buffers in state).
- Keep the UI tree reasonably shallow; let panels compose using simple `Node` containers.

---

## What to delete from the previous plan (explicitly)
- Anything referring to `PositionType::Absolute` or `.with_right/.with_left/.with_top/.with_bottom`.
- “Nice to have: animate collapsibles” (they already animate via `collapsible` + transitions).
- “Need custom slider with value beside it” (already exists: `slider_with_value`).

---