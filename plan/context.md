# Working Context — particles: astra-gui migration

## Update (auto IDs + slider_with_value policy)
- All sliders will be migrated to `slider_with_value` (not plain `slider`).
- astra-gui **does** have automatic ID assignment, but it is **only** for nodes that have interactive styles (hover/active/disabled styles) and **only** when an explicit ID is not provided.
  - The auto IDs are generated from the node tree path (stable as long as the UI tree structure ordering is stable).
  - Important limitation: most interactive component update helpers in `astra-gui-interactive` (e.g. `slider_with_value_update`, `toggle_clicked`, `button_clicked`, `collapsible_clicked`) are written to take **string IDs**. For those, we still need stable identifiers we can reference.
  - Conclusion: auto IDs help hover/active styling “just work” without manually assigning IDs everywhere, but they do **not** remove the need for explicit IDs for the interactive widgets we need to update/handle.

## Goal
Replace the current egui UI in `particles` with `astra-gui` + `astra-gui-wgpu`, so the project can upgrade to `wgpu 0.28` and stop depending on `egui`, `egui-winit`, `egui-wgpu`.

This migration must use **existing astra-gui primitives and components** (no invented APIs, no unnecessary custom widgets):
- Layout: `Layout::{Horizontal, Vertical, Stack}`
- Alignment: `HorizontalAlign::{Left, Center, Right}`, `VerticalAlign::{Top, Center, Bottom}`
- Interactive components: `astra-gui-interactive` (`button`, `toggle`, `slider`, `drag_value`, `slider_with_value`, `collapsible`, `text_input` + their `*_clicked`/`*_update` helpers)
- Event system: `astra_gui_wgpu::{InputState, EventDispatcher, InteractiveStateManager}` (dispatch uses hit test and returns `TargetedEvent`s)

## Current State (as of now)
### UI is split across two systems
1. **egui**: full UI (panels + controls) lives in `particles/src/gui.rs` as `Gui` (egui wrapper).
2. **astra-gui**: only a small diagnostics panel exists via `gui::build_diagnostics_panel(...)` using `astra_gui::Node` and `astra_gui_wgpu::Renderer`.

### Where UI is wired in
- `particles/src/main.rs`
  - `App::window_event` calls `gpu_state.gui.handle_event(window, &event)` and returns early if egui consumed the event.
  - The egui `Gui` is rendered each frame via `gpu_state.gui.render(...)`.
  - Astra diagnostics are rendered by building `FullOutput` from `build_diagnostics_panel(...)` and calling `astra_renderer.render(...)`.

## UI Inventory to Migrate (from egui `Gui::ui`)
### Global / common behavior
- Keyboard shortcuts:
  - `Space`: toggle pause
  - When paused: `Ctrl+Right` or `Ctrl+D` adds `steps_to_play` to `steps_remaining`
- Stepping driver:
  - If `steps_remaining > 0` → `step_one_frame = true`, then decrement `steps_remaining`

### Panels (4)
1. **Statistics** (Top Right, collapsible, default open)
   - Particle counts:
     - `particle_count`
   - Hadron counts:
     - `hadron_count`, `proton_count`, `neutron_count`, `other_hadron_count`
   - Rendering toggles:
     - `show_shells`, `show_bonds`, `show_nuclei`
   - LOD sliders (all with invariants end >= start):
     - `lod_shell_fade_start` (5..=200 step 5)
     - `lod_shell_fade_end` (5..=200 step 5)
     - `lod_bound_hadron_fade_start` (10..=300 step 10)
     - `lod_bound_hadron_fade_end` (10..=300 step 10)
     - `lod_bond_fade_start` (5..=200 step 5)
     - `lod_bond_fade_end` (5..=200 step 5)
     - `lod_quark_fade_start` (5..=200 step 5)
     - `lod_quark_fade_end` (5..=200 step 5)
     - `lod_nucleus_fade_start` (10..=300 step 10)
     - `lod_nucleus_fade_end` (10..=300 step 10)

2. **Physics Controls** (Bottom Left, collapsible, default closed)
   - Sliders update `physics_params_dirty = true` when changed.
   - Sections:
     - Forces:
       - `physics_params.constants[0]` Gravity (egui logarithmic) range `0..=1e-9`
       - `physics_params.constants[1]` Electric range `0..=20`
     - Strong Force:
       - `strong_force[0]` Short Range `0..=5` step 0.1
       - `strong_force[1]` Confinement `0..=5` step 0.1
       - `strong_force[2]` Range Cutoff `0..=10` step 0.1
     - Repulsion:
       - `repulsion[0]` Core Strength `0..=500`
       - `repulsion[1]` Core Radius `0..=1`
       - `repulsion[2]` Softening `0.001..=0.1`
       - `repulsion[3]` Max Force `10..=200`
     - Integration:
       - `integration[0]` Time Step (dt) (egui logarithmic) `0.0001..=0.01`
       - `integration[1]` Damping `0.9..=1.0`
     - Nucleon Physics:
       - `nucleon[0]` Binding Strength `0..=200`
       - `nucleon[1]` Binding Range `0.1..=10`
       - `nucleon[2]` Exclusion Strength `0..=300`
       - `nucleon[3]` Exclusion Radius (x Hadron R) `0.5..=3`
       - `integration[3]` Nucleon Damping `0..=100`
     - Electron Physics:
       - `electron[0]` Attraction Strength `0..=200`
       - `electron[1]` Attraction Range `0.1..=10`
       - `electron[2]` Exclusion Strength `0..=300`
       - `electron[3]` Exclusion Radius `0.5..=3`
     - Hadron Formation (has invariant breakup >= binding):
       - `hadron[0]` Binding Distance `0.1..=3.0` step 0.05
       - `hadron[1]` Breakup Distance `0.1..=5.0` step 0.05
       - `hadron[2]` Confinement Range Mult `0.1..=5.0` step 0.1
       - `hadron[3]` Confinement Strength Mult `0.1..=5.0` step 0.1

3. **Time Controls** (Bottom Right, collapsible, default open)
   - Pause/resume button (label depends on `is_paused`)
   - dt quick slider:
     - `physics_params.integration[0]` `0.0001..=0.01` step 0.0001, sets `physics_params_dirty`
   - When paused:
     - `steps_to_play` `1..=1000` using DragValue
     - Step button adds `steps_to_play` to `steps_remaining`

4. **Atom Card** (Center Top, non-collapsible, conditional)
   - Only shown when `selected_nucleus_atomic_number.is_some()`
   - Displays element name/symbol and the stored nucleus counts:
     - `selected_nucleus_atomic_number` (Z)
     - `selected_nucleus_proton_count`
     - `selected_nucleus_neutron_count`
     - `selected_nucleus_nucleon_count` (A)

## Migration Strategy (high-level)
1. Remove egui wrapper (`Gui`) and event consumption model.
2. Introduce a new astra-based UI system for the *entire* UI.
3. Keep `UiState` as the single source of truth for simulation-visible values.
4. Add additional UI-only state for:
   - collapsible expanded bools
   - per-widget state needed by `slider_with_value` / `drag_value` (text buffers, cursor, selection, focused, drag accumulator)

## Planned UI Implementation Details (grounded in astra examples)
### Frame pipeline (match astra-gui-wgpu examples)
- `InteractiveStateManager::begin_frame()` at start
- Build `Node` UI tree
- `inject_dimension_overrides(&mut ui)` before layout
- Layout: `compute_layout_with_measurer(window_rect, &mut text_engine)`
- Dispatch: `EventDispatcher::dispatch(&input_state, &mut ui)` → `(events, interaction_states)`
- Update transitions: `update_transitions(&mut ui, &interaction_states)`
- Handle `events` to mutate `UiState` + UI-only state
- Convert to output and render with `astra_gui_wgpu::Renderer`
- Clear input at end of frame: `input_state.begin_frame()` (as in examples)

### Panel positioning (no absolute positioning)
Root overlay is `Layout::Stack` filling the window.
Each panel is a child aligned with:
- Stats: `Top + Right`
- Physics: `Bottom + Left`
- Time: `Bottom + Right`
- Atom card: `Top + Center`
Insets via `margin` and internal spacing via `padding`.

### Control choices
- **All sliders**: use `slider_with_value` + `slider_with_value_update` (egui-like value field on the right, no tooltip needed).
- Toggles: `toggle` + `toggle_clicked`
- Collapsible container: `collapsible` + `collapsible_clicked`
- Buttons: `button` + `button_clicked`
- For stepped sliders, use `step: Some(f32)` in the update helper.
- For logarithmic behavior (Gravity/dt): handle mapping in event logic (astra slider itself is linear).

## Input blocking / camera interaction
Current egui model:
- `Gui::handle_event` returns `consumed`, and the app returns early to avoid camera/picking input.

New astra model:
- We must explicitly guard camera/picking input when the cursor is on UI.
- Plan:
  - Always feed winit events into astra `InputState` (in addition to any camera handling).
  - Before doing camera drag, scroll zoom, or click picking, detect whether the cursor is currently over UI by hit testing the laid-out UI tree at the cursor position.
  - If UI is hit, do not process camera/picking for that event/frame.

## Invariants to preserve
- LOD ranges: always enforce `*_end >= *_start`
- Hadron formation: enforce `hadron[1] >= hadron[0]` and mark dirty if clamped
- Simulation stepping:
  - `steps_remaining` decremented each frame while stepping
  - `step_one_frame` asserted for each step
- While editing numeric fields, global shortcuts should not fire (policy: suppress space/ctrl-step when a text input is focused).

## TODO (actionable)
### Inventory & mapping
- [ ] Create an ID scheme for all interactive widgets (stable strings).
  - Note: astra-gui can auto-assign IDs for nodes with interactive styles, but `astra-gui-interactive` update helpers require explicit string IDs, so we still need stable IDs for every interactive widget we want to handle/update.
- [x] Decide which controls use `slider_with_value` vs plain `slider` (all sliders will be `slider_with_value`).

### Implement astra UI system
- [ ] New UI struct in `particles/src/gui.rs` (or a new module) containing:
  - `InputState`, `EventDispatcher`, `InteractiveStateManager`, `TextEngine`
  - UI-only state: collapsible expanded flags, per-slider-with-value state
- [ ] Replace old egui `Gui` integration in `particles/src/main.rs`:
  - Stop early-return consumption.
  - Wire `InputState::handle_event(&WindowEvent)` from the winit loop.
  - Render astra output each frame via existing `astra_renderer`.

### Build UI tree
- [ ] Root overlay: `Layout::Stack`, Fill x/y
- [ ] Top-right Statistics panel (collapsible)
- [ ] Bottom-left Physics panel (collapsible, default collapsed)
- [ ] Bottom-right Time panel (collapsible, default expanded)
- [ ] Top-center Atom card (conditional)

### Event handling
- [ ] Implement handlers:
  - [ ] `Space` toggles pause (unless text input focused)
  - [ ] When paused, ctrl-step adds to `steps_remaining` (unless text input focused)
  - [ ] Buttons/toggles/sliders update appropriate `UiState` fields
  - [ ] Set `physics_params_dirty` on any physics parameter changes
  - [ ] Enforce invariants after updates

### Input blocking
- [ ] Add “is cursor over UI?” gating for:
  - camera rotate (RMB drag)
  - zoom (wheel)
  - picking click (LMB)

### Cleanup
- [ ] Delete egui dependencies and `Gui` wrapper once astra UI is complete
- [ ] Ensure `wgpu` is upgraded to `0.28` across workspace
- [ ] Run: `cargo fmt`, `cargo check`, `cargo test`, `cargo run`

## Notes / constraints
- No new assets should be added as part of this plan.
- Keep UI performant (target: high FPS).
- Debug visualizations (astra-gui): padding (blue), margin (red), border (green), gap (purple), clip rect (red), content-area (yellow).