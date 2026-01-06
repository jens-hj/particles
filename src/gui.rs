#![allow(dead_code)]

use winit::event::WindowEvent;

use astra_gui::{
    catppuccin::mocha, Content, CornerShape, DebugOptions, FullOutput as AstraFullOutput,
    HorizontalAlign, Layout, Node, Place, Size, Spacing, Stroke, Style, TextContent, VerticalAlign,
};
use astra_gui_interactive::{
    button, button_clicked, collapsible, collapsible_clicked, slider_with_value,
    slider_with_value_update, toggle, toggle_clicked, ButtonStyle, CollapsibleStyle,
    DragValueStyle, SliderStyle, ToggleStyle,
};
use astra_gui_text::Engine as TextEngine;
use astra_gui_wgpu::{EventDispatcher, InputState, InteractiveStateManager, TargetedEvent};
use particle_simulation::PhysicsParams;

use crate::gui_data::{element_name, element_symbol};

/// UI runtime state owned by the app.
///
/// This remains the single source of truth for UI-exposed values during the migration.
/// The actual UI rendering is currently a minimal astra-gui overlay (no interactions yet).
pub struct UiState {
    pub fps: f32,
    pub frame_time: f32,
    pub particle_count: usize,

    // Hadrons
    pub hadron_count: u32,
    pub proton_count: u32,
    pub neutron_count: u32,
    pub other_hadron_count: u32,

    // Selected nucleus info (for atom card UI)
    pub selected_nucleus_atomic_number: Option<u32>, // Z (proton count / type_id)
    pub selected_nucleus_proton_count: Option<u32>,
    pub selected_nucleus_neutron_count: Option<u32>,
    pub selected_nucleus_nucleon_count: Option<u32>, // Total nucleons

    pub physics_params: PhysicsParams,
    pub physics_params_dirty: bool,
    pub show_shells: bool,
    pub show_bonds: bool,
    pub show_nuclei: bool,
    pub is_paused: bool,
    pub step_one_frame: bool,
    pub steps_to_play: u32,
    pub steps_remaining: u32,

    // LOD controls
    pub lod_shell_fade_start: f32,
    pub lod_shell_fade_end: f32,
    pub lod_bound_hadron_fade_start: f32,
    pub lod_bound_hadron_fade_end: f32,
    pub lod_bond_fade_start: f32,
    pub lod_bond_fade_end: f32,
    pub lod_quark_fade_start: f32,
    pub lod_quark_fade_end: f32,
    pub lod_nucleus_fade_start: f32,
    pub lod_nucleus_fade_end: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time: 0.0,
            particle_count: 0,

            hadron_count: 0,
            proton_count: 0,
            neutron_count: 0,
            other_hadron_count: 0,

            selected_nucleus_atomic_number: None,
            selected_nucleus_proton_count: None,
            selected_nucleus_neutron_count: None,
            selected_nucleus_nucleon_count: None,

            physics_params: PhysicsParams::default(),
            physics_params_dirty: true, // Initial upload needed
            show_shells: true,
            show_bonds: true,
            show_nuclei: true,
            is_paused: false,
            step_one_frame: false,
            steps_to_play: 1,
            steps_remaining: 0,

            lod_shell_fade_start: 10.0,
            lod_shell_fade_end: 30.0,
            lod_bound_hadron_fade_start: 40.0,
            lod_bound_hadron_fade_end: 70.0,
            lod_bond_fade_start: 10.0,
            lod_bond_fade_end: 30.0,
            lod_quark_fade_start: 10.0,
            lod_quark_fade_end: 30.0,
            lod_nucleus_fade_start: 40.0, // Nuclei appear further out than hadrons
            lod_nucleus_fade_end: 70.0,
        }
    }
}

/// Temporary astra-gui wrapper while migrating off egui.
///
/// This keeps the surface area small:
/// - No event handling yet (interactive astra-gui wiring comes next)
/// - `build()` produces an astra-gui `FullOutput` that the app renders via `astra-gui-wgpu`
///
/// IMPORTANT: we keep a `TextEngine` around and pass it as a measurer so `Size::FitContent` and
/// text intrinsic sizing work correctly.
pub struct Gui {
    text_engine: TextEngine,

    // Interactive state (astra-gui-wgpu event pipeline)
    input_state: InputState,
    event_dispatcher: EventDispatcher,
    interactive_state_manager: InteractiveStateManager,

    // Collapsible state (per-panel)
    stats_panel_expanded: bool,
    render_lod_panel_expanded: bool,
    physics_panel_expanded: bool,
    time_panel_expanded: bool,
    atom_card_expanded: bool,

    // Per-widget state (these are required for interactive widgets to behave correctly)
    render_shells: bool,
    render_bonds: bool,
    render_nuclei: bool,

    lod_shell_fade_start: f32,
    lod_shell_fade_end: f32,
    lod_bound_hadron_fade_start: f32,
    lod_bound_hadron_fade_end: f32,
    lod_bond_fade_start: f32,
    lod_bond_fade_end: f32,
    lod_quark_fade_start: f32,
    lod_quark_fade_end: f32,
    lod_nucleus_fade_start: f32,
    lod_nucleus_fade_end: f32,

    // slider_with_value per-slider input state
    lod_shell_fade_start_text: String,
    lod_shell_fade_start_cursor: usize,
    lod_shell_fade_start_selection: Option<(usize, usize)>,
    lod_shell_fade_start_focused: bool,
    lod_shell_fade_start_drag_accumulator: f32,

    // Physics controls (per-slider state)
    phys_constants_g_text: String,
    phys_constants_g_cursor: usize,
    phys_constants_g_selection: Option<(usize, usize)>,
    phys_constants_g_focused: bool,
    phys_constants_g_drag_accumulator: f32,

    phys_constants_k_text: String,
    phys_constants_k_cursor: usize,
    phys_constants_k_selection: Option<(usize, usize)>,
    phys_constants_k_focused: bool,
    phys_constants_k_drag_accumulator: f32,

    phys_constants_gweak_text: String,
    phys_constants_gweak_cursor: usize,
    phys_constants_gweak_selection: Option<(usize, usize)>,
    phys_constants_gweak_focused: bool,
    phys_constants_gweak_drag_accumulator: f32,

    phys_constants_weak_range_text: String,
    phys_constants_weak_range_cursor: usize,
    phys_constants_weak_range_selection: Option<(usize, usize)>,
    phys_constants_weak_range_focused: bool,
    phys_constants_weak_range_drag_accumulator: f32,

    phys_strong_short_text: String,
    phys_strong_short_cursor: usize,
    phys_strong_short_selection: Option<(usize, usize)>,
    phys_strong_short_focused: bool,
    phys_strong_short_drag_accumulator: f32,

    phys_strong_confinement_text: String,
    phys_strong_confinement_cursor: usize,
    phys_strong_confinement_selection: Option<(usize, usize)>,
    phys_strong_confinement_focused: bool,
    phys_strong_confinement_drag_accumulator: f32,

    phys_strong_range_text: String,
    phys_strong_range_cursor: usize,
    phys_strong_range_selection: Option<(usize, usize)>,
    phys_strong_range_focused: bool,
    phys_strong_range_drag_accumulator: f32,

    phys_repulsion_strength_text: String,
    phys_repulsion_strength_cursor: usize,
    phys_repulsion_strength_selection: Option<(usize, usize)>,
    phys_repulsion_strength_focused: bool,
    phys_repulsion_strength_drag_accumulator: f32,

    phys_repulsion_radius_text: String,
    phys_repulsion_radius_cursor: usize,
    phys_repulsion_radius_selection: Option<(usize, usize)>,
    phys_repulsion_radius_focused: bool,
    phys_repulsion_radius_drag_accumulator: f32,

    phys_repulsion_softening_text: String,
    phys_repulsion_softening_cursor: usize,
    phys_repulsion_softening_selection: Option<(usize, usize)>,
    phys_repulsion_softening_focused: bool,
    phys_repulsion_softening_drag_accumulator: f32,

    phys_repulsion_max_force_text: String,
    phys_repulsion_max_force_cursor: usize,
    phys_repulsion_max_force_selection: Option<(usize, usize)>,
    phys_repulsion_max_force_focused: bool,
    phys_repulsion_max_force_drag_accumulator: f32,

    phys_integration_damping_text: String,
    phys_integration_damping_cursor: usize,
    phys_integration_damping_selection: Option<(usize, usize)>,
    phys_integration_damping_focused: bool,
    phys_integration_damping_drag_accumulator: f32,

    phys_integration_nucleon_damping_text: String,
    phys_integration_nucleon_damping_cursor: usize,
    phys_integration_nucleon_damping_selection: Option<(usize, usize)>,
    phys_integration_nucleon_damping_focused: bool,
    phys_integration_nucleon_damping_drag_accumulator: f32,

    phys_nucleon_binding_strength_text: String,
    phys_nucleon_binding_strength_cursor: usize,
    phys_nucleon_binding_strength_selection: Option<(usize, usize)>,
    phys_nucleon_binding_strength_focused: bool,
    phys_nucleon_binding_strength_drag_accumulator: f32,

    phys_nucleon_binding_range_text: String,
    phys_nucleon_binding_range_cursor: usize,
    phys_nucleon_binding_range_selection: Option<(usize, usize)>,
    phys_nucleon_binding_range_focused: bool,
    phys_nucleon_binding_range_drag_accumulator: f32,

    phys_nucleon_exclusion_strength_text: String,
    phys_nucleon_exclusion_strength_cursor: usize,
    phys_nucleon_exclusion_strength_selection: Option<(usize, usize)>,
    phys_nucleon_exclusion_strength_focused: bool,
    phys_nucleon_exclusion_strength_drag_accumulator: f32,

    phys_nucleon_exclusion_radius_text: String,
    phys_nucleon_exclusion_radius_cursor: usize,
    phys_nucleon_exclusion_radius_selection: Option<(usize, usize)>,
    phys_nucleon_exclusion_radius_focused: bool,
    phys_nucleon_exclusion_radius_drag_accumulator: f32,

    phys_electron_exclusion_strength_text: String,
    phys_electron_exclusion_strength_cursor: usize,
    phys_electron_exclusion_strength_selection: Option<(usize, usize)>,
    phys_electron_exclusion_strength_focused: bool,
    phys_electron_exclusion_strength_drag_accumulator: f32,

    phys_electron_exclusion_radius_text: String,
    phys_electron_exclusion_radius_cursor: usize,
    phys_electron_exclusion_radius_selection: Option<(usize, usize)>,
    phys_electron_exclusion_radius_focused: bool,
    phys_electron_exclusion_radius_drag_accumulator: f32,

    phys_hadron_binding_distance_text: String,
    phys_hadron_binding_distance_cursor: usize,
    phys_hadron_binding_distance_selection: Option<(usize, usize)>,
    phys_hadron_binding_distance_focused: bool,
    phys_hadron_binding_distance_drag_accumulator: f32,

    phys_hadron_breakup_distance_text: String,
    phys_hadron_breakup_distance_cursor: usize,
    phys_hadron_breakup_distance_selection: Option<(usize, usize)>,
    phys_hadron_breakup_distance_focused: bool,
    phys_hadron_breakup_distance_drag_accumulator: f32,

    phys_hadron_conf_range_mult_text: String,
    phys_hadron_conf_range_mult_cursor: usize,
    phys_hadron_conf_range_mult_selection: Option<(usize, usize)>,
    phys_hadron_conf_range_mult_focused: bool,
    phys_hadron_conf_range_mult_drag_accumulator: f32,

    phys_hadron_conf_strength_mult_text: String,
    phys_hadron_conf_strength_mult_cursor: usize,
    phys_hadron_conf_strength_mult_selection: Option<(usize, usize)>,
    phys_hadron_conf_strength_mult_focused: bool,
    phys_hadron_conf_strength_mult_drag_accumulator: f32,

    lod_shell_fade_end_text: String,
    lod_shell_fade_end_cursor: usize,
    lod_shell_fade_end_selection: Option<(usize, usize)>,
    lod_shell_fade_end_focused: bool,
    lod_shell_fade_end_drag_accumulator: f32,

    lod_bound_hadron_fade_start_text: String,
    lod_bound_hadron_fade_start_cursor: usize,
    lod_bound_hadron_fade_start_selection: Option<(usize, usize)>,
    lod_bound_hadron_fade_start_focused: bool,
    lod_bound_hadron_fade_start_drag_accumulator: f32,

    lod_bound_hadron_fade_end_text: String,
    lod_bound_hadron_fade_end_cursor: usize,
    lod_bound_hadron_fade_end_selection: Option<(usize, usize)>,
    lod_bound_hadron_fade_end_focused: bool,
    lod_bound_hadron_fade_end_drag_accumulator: f32,

    lod_bond_fade_start_text: String,
    lod_bond_fade_start_cursor: usize,
    lod_bond_fade_start_selection: Option<(usize, usize)>,
    lod_bond_fade_start_focused: bool,
    lod_bond_fade_start_drag_accumulator: f32,

    lod_bond_fade_end_text: String,
    lod_bond_fade_end_cursor: usize,
    lod_bond_fade_end_selection: Option<(usize, usize)>,
    lod_bond_fade_end_focused: bool,
    lod_bond_fade_end_drag_accumulator: f32,

    lod_quark_fade_start_text: String,
    lod_quark_fade_start_cursor: usize,
    lod_quark_fade_start_selection: Option<(usize, usize)>,
    lod_quark_fade_start_focused: bool,
    lod_quark_fade_start_drag_accumulator: f32,

    lod_quark_fade_end_text: String,
    lod_quark_fade_end_cursor: usize,
    lod_quark_fade_end_selection: Option<(usize, usize)>,
    lod_quark_fade_end_focused: bool,
    lod_quark_fade_end_drag_accumulator: f32,

    lod_nucleus_fade_start_text: String,
    lod_nucleus_fade_start_cursor: usize,
    lod_nucleus_fade_start_selection: Option<(usize, usize)>,
    lod_nucleus_fade_start_focused: bool,
    lod_nucleus_fade_start_drag_accumulator: f32,

    lod_nucleus_fade_end_text: String,
    lod_nucleus_fade_end_cursor: usize,
    lod_nucleus_fade_end_selection: Option<(usize, usize)>,
    lod_nucleus_fade_end_focused: bool,
    lod_nucleus_fade_end_drag_accumulator: f32,

    physics_dt_text: String,
    physics_dt_cursor: usize,
    physics_dt_selection: Option<(usize, usize)>,
    physics_dt_focused: bool,
    physics_dt_drag_accumulator: f32,

    time_steps_to_play_text: String,
    time_steps_to_play_cursor: usize,
    time_steps_to_play_selection: Option<(usize, usize)>,
    time_steps_to_play_focused: bool,
    time_steps_to_play_drag_accumulator: f32,

    is_paused: bool,
    steps_to_play: f32,

    // Events emitted by the interactive system for the most recent frame
    last_events: Vec<TargetedEvent>,
    // Whether UI consumed pointer/scroll input this frame (used to gate camera/picking)
    ui_consumed_pointer: bool,

    // Dirty flags (used by the app to trigger GPU uploads / sim reconfiguration)
    physics_params_dirty: bool,
    step_one_frame: bool,
}

impl Gui {
    pub fn new() -> Self {
        Self {
            text_engine: TextEngine::new_default(),

            input_state: InputState::new(),
            event_dispatcher: EventDispatcher::new(),
            interactive_state_manager: InteractiveStateManager::new(),

            // Defaults: start expanded so behavior matches the current always-visible panels.
            stats_panel_expanded: true,
            render_lod_panel_expanded: false,
            physics_panel_expanded: false,
            time_panel_expanded: true,
            atom_card_expanded: true,

            // Defaults mirror UiState::default() so the UI behaves predictably.
            render_shells: true,
            render_bonds: true,
            render_nuclei: true,

            lod_shell_fade_start: 10.0,
            lod_shell_fade_end: 30.0,
            lod_bound_hadron_fade_start: 40.0,
            lod_bound_hadron_fade_end: 70.0,
            lod_bond_fade_start: 10.0,
            lod_bond_fade_end: 30.0,
            lod_quark_fade_start: 10.0,
            lod_quark_fade_end: 30.0,
            lod_nucleus_fade_start: 40.0,
            lod_nucleus_fade_end: 70.0,

            lod_shell_fade_start_text: String::new(),
            lod_shell_fade_start_cursor: 0,
            lod_shell_fade_start_selection: None,
            lod_shell_fade_start_focused: false,
            lod_shell_fade_start_drag_accumulator: 10.0,

            lod_shell_fade_end_text: String::new(),
            lod_shell_fade_end_cursor: 0,
            lod_shell_fade_end_selection: None,
            lod_shell_fade_end_focused: false,
            lod_shell_fade_end_drag_accumulator: 30.0,

            lod_bound_hadron_fade_start_text: String::new(),
            lod_bound_hadron_fade_start_cursor: 0,
            lod_bound_hadron_fade_start_selection: None,
            lod_bound_hadron_fade_start_focused: false,
            lod_bound_hadron_fade_start_drag_accumulator: 40.0,

            lod_bound_hadron_fade_end_text: String::new(),
            lod_bound_hadron_fade_end_cursor: 0,
            lod_bound_hadron_fade_end_selection: None,
            lod_bound_hadron_fade_end_focused: false,
            lod_bound_hadron_fade_end_drag_accumulator: 70.0,

            lod_bond_fade_start_text: String::new(),
            lod_bond_fade_start_cursor: 0,
            lod_bond_fade_start_selection: None,
            lod_bond_fade_start_focused: false,
            lod_bond_fade_start_drag_accumulator: 10.0,

            lod_bond_fade_end_text: String::new(),
            lod_bond_fade_end_cursor: 0,
            lod_bond_fade_end_selection: None,
            lod_bond_fade_end_focused: false,
            lod_bond_fade_end_drag_accumulator: 30.0,

            lod_quark_fade_start_text: String::new(),
            lod_quark_fade_start_cursor: 0,
            lod_quark_fade_start_selection: None,
            lod_quark_fade_start_focused: false,
            lod_quark_fade_start_drag_accumulator: 10.0,

            lod_quark_fade_end_text: String::new(),
            lod_quark_fade_end_cursor: 0,
            lod_quark_fade_end_selection: None,
            lod_quark_fade_end_focused: false,
            lod_quark_fade_end_drag_accumulator: 30.0,

            lod_nucleus_fade_start_text: String::new(),
            lod_nucleus_fade_start_cursor: 0,
            lod_nucleus_fade_start_selection: None,
            lod_nucleus_fade_start_focused: false,
            lod_nucleus_fade_start_drag_accumulator: 40.0,

            lod_nucleus_fade_end_text: String::new(),
            lod_nucleus_fade_end_cursor: 0,
            lod_nucleus_fade_end_selection: None,
            lod_nucleus_fade_end_focused: false,
            lod_nucleus_fade_end_drag_accumulator: 70.0,

            physics_dt_text: String::new(),
            physics_dt_cursor: 0,
            physics_dt_selection: None,
            physics_dt_focused: false,
            physics_dt_drag_accumulator: 0.0,

            // Physics controls (per-slider state)
            phys_constants_g_text: String::new(),
            phys_constants_g_cursor: 0,
            phys_constants_g_selection: None,
            phys_constants_g_focused: false,
            phys_constants_g_drag_accumulator: 0.0,

            phys_constants_k_text: String::new(),
            phys_constants_k_cursor: 0,
            phys_constants_k_selection: None,
            phys_constants_k_focused: false,
            phys_constants_k_drag_accumulator: 0.0,

            phys_constants_gweak_text: String::new(),
            phys_constants_gweak_cursor: 0,
            phys_constants_gweak_selection: None,
            phys_constants_gweak_focused: false,
            phys_constants_gweak_drag_accumulator: 0.0,

            phys_constants_weak_range_text: String::new(),
            phys_constants_weak_range_cursor: 0,
            phys_constants_weak_range_selection: None,
            phys_constants_weak_range_focused: false,
            phys_constants_weak_range_drag_accumulator: 0.0,

            phys_strong_short_text: String::new(),
            phys_strong_short_cursor: 0,
            phys_strong_short_selection: None,
            phys_strong_short_focused: false,
            phys_strong_short_drag_accumulator: 0.0,

            phys_strong_confinement_text: String::new(),
            phys_strong_confinement_cursor: 0,
            phys_strong_confinement_selection: None,
            phys_strong_confinement_focused: false,
            phys_strong_confinement_drag_accumulator: 0.0,

            phys_strong_range_text: String::new(),
            phys_strong_range_cursor: 0,
            phys_strong_range_selection: None,
            phys_strong_range_focused: false,
            phys_strong_range_drag_accumulator: 0.0,

            phys_repulsion_strength_text: String::new(),
            phys_repulsion_strength_cursor: 0,
            phys_repulsion_strength_selection: None,
            phys_repulsion_strength_focused: false,
            phys_repulsion_strength_drag_accumulator: 0.0,

            phys_repulsion_radius_text: String::new(),
            phys_repulsion_radius_cursor: 0,
            phys_repulsion_radius_selection: None,
            phys_repulsion_radius_focused: false,
            phys_repulsion_radius_drag_accumulator: 0.0,

            phys_repulsion_softening_text: String::new(),
            phys_repulsion_softening_cursor: 0,
            phys_repulsion_softening_selection: None,
            phys_repulsion_softening_focused: false,
            phys_repulsion_softening_drag_accumulator: 0.0,

            phys_repulsion_max_force_text: String::new(),
            phys_repulsion_max_force_cursor: 0,
            phys_repulsion_max_force_selection: None,
            phys_repulsion_max_force_focused: false,
            phys_repulsion_max_force_drag_accumulator: 0.0,

            phys_integration_damping_text: String::new(),
            phys_integration_damping_cursor: 0,
            phys_integration_damping_selection: None,
            phys_integration_damping_focused: false,
            phys_integration_damping_drag_accumulator: 0.0,

            phys_integration_nucleon_damping_text: String::new(),
            phys_integration_nucleon_damping_cursor: 0,
            phys_integration_nucleon_damping_selection: None,
            phys_integration_nucleon_damping_focused: false,
            phys_integration_nucleon_damping_drag_accumulator: 0.0,

            phys_nucleon_binding_strength_text: String::new(),
            phys_nucleon_binding_strength_cursor: 0,
            phys_nucleon_binding_strength_selection: None,
            phys_nucleon_binding_strength_focused: false,
            phys_nucleon_binding_strength_drag_accumulator: 0.0,

            phys_nucleon_binding_range_text: String::new(),
            phys_nucleon_binding_range_cursor: 0,
            phys_nucleon_binding_range_selection: None,
            phys_nucleon_binding_range_focused: false,
            phys_nucleon_binding_range_drag_accumulator: 0.0,

            phys_nucleon_exclusion_strength_text: String::new(),
            phys_nucleon_exclusion_strength_cursor: 0,
            phys_nucleon_exclusion_strength_selection: None,
            phys_nucleon_exclusion_strength_focused: false,
            phys_nucleon_exclusion_strength_drag_accumulator: 0.0,

            phys_nucleon_exclusion_radius_text: String::new(),
            phys_nucleon_exclusion_radius_cursor: 0,
            phys_nucleon_exclusion_radius_selection: None,
            phys_nucleon_exclusion_radius_focused: false,
            phys_nucleon_exclusion_radius_drag_accumulator: 0.0,

            phys_electron_exclusion_strength_text: String::new(),
            phys_electron_exclusion_strength_cursor: 0,
            phys_electron_exclusion_strength_selection: None,
            phys_electron_exclusion_strength_focused: false,
            phys_electron_exclusion_strength_drag_accumulator: 0.0,

            phys_electron_exclusion_radius_text: String::new(),
            phys_electron_exclusion_radius_cursor: 0,
            phys_electron_exclusion_radius_selection: None,
            phys_electron_exclusion_radius_focused: false,
            phys_electron_exclusion_radius_drag_accumulator: 0.0,

            phys_hadron_binding_distance_text: String::new(),
            phys_hadron_binding_distance_cursor: 0,
            phys_hadron_binding_distance_selection: None,
            phys_hadron_binding_distance_focused: false,
            phys_hadron_binding_distance_drag_accumulator: 0.0,

            phys_hadron_breakup_distance_text: String::new(),
            phys_hadron_breakup_distance_cursor: 0,
            phys_hadron_breakup_distance_selection: None,
            phys_hadron_breakup_distance_focused: false,
            phys_hadron_breakup_distance_drag_accumulator: 0.0,

            phys_hadron_conf_range_mult_text: String::new(),
            phys_hadron_conf_range_mult_cursor: 0,
            phys_hadron_conf_range_mult_selection: None,
            phys_hadron_conf_range_mult_focused: false,
            phys_hadron_conf_range_mult_drag_accumulator: 0.0,

            phys_hadron_conf_strength_mult_text: String::new(),
            phys_hadron_conf_strength_mult_cursor: 0,
            phys_hadron_conf_strength_mult_selection: None,
            phys_hadron_conf_strength_mult_focused: false,
            phys_hadron_conf_strength_mult_drag_accumulator: 0.0,

            time_steps_to_play_text: String::new(),
            time_steps_to_play_cursor: 0,
            time_steps_to_play_selection: None,
            time_steps_to_play_focused: false,
            time_steps_to_play_drag_accumulator: 1.0,

            is_paused: false,
            steps_to_play: 1.0,

            last_events: Vec::new(),
            ui_consumed_pointer: false,

            physics_params_dirty: false,
            step_one_frame: false,
        }
    }

    /// Placeholder: egui used to consume window events. During migration we let the app handle all
    /// events normally (camera/picking/etc). Once interactive astra-gui is wired in, this will
    /// become `dispatch()` + "is pointer over UI?" gating.
    /// Feed winit events into astra's input tracker.
    ///
    /// Returns `true` when the UI consumed pointer/scroll input this frame.
    /// Note: actual per-event consumption decisions are based on hit-testing, which
    /// happens during `build()` when we have the laid-out node tree.
    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        self.input_state.handle_event(event);
        self.ui_consumed_pointer
    }

    /// Build a minimal UI overlay node tree.
    ///
    /// Note: this does not use a text measurer yet; sizes are explicit so it stays robust.
    pub fn build(
        &mut self,
        ui_state: &mut UiState,
        window_size: [f32; 2],
        debug_options: DebugOptions,
    ) -> AstraFullOutput {
        // Begin frame for interactive transitions + clear per-frame flags.
        self.interactive_state_manager.begin_frame();
        self.step_one_frame = false;

        // Keep widget state in sync with the app-owned UiState (single source of truth).
        // During migration we still treat UiState as authoritative and just reflect it here.
        self.render_shells = ui_state.show_shells;
        self.render_bonds = ui_state.show_bonds;
        self.render_nuclei = ui_state.show_nuclei;

        self.lod_shell_fade_start = ui_state.lod_shell_fade_start;
        self.lod_shell_fade_end = ui_state.lod_shell_fade_end;
        self.lod_bound_hadron_fade_start = ui_state.lod_bound_hadron_fade_start;
        self.lod_bound_hadron_fade_end = ui_state.lod_bound_hadron_fade_end;
        self.lod_bond_fade_start = ui_state.lod_bond_fade_start;
        self.lod_bond_fade_end = ui_state.lod_bond_fade_end;
        self.lod_quark_fade_start = ui_state.lod_quark_fade_start;
        self.lod_quark_fade_end = ui_state.lod_quark_fade_end;
        self.lod_nucleus_fade_start = ui_state.lod_nucleus_fade_start;
        self.lod_nucleus_fade_end = ui_state.lod_nucleus_fade_end;

        self.is_paused = ui_state.is_paused;
        self.steps_to_play = ui_state.steps_to_play as f32;

        self.physics_params_dirty = ui_state.physics_params_dirty;

        // Build the UI tree with the requested panel placements.
        //
        // Note: during this stage we still treat UiState as the source of truth. We *render* from
        // local widget state (so widgets can be interactive), then we apply events to UiState via
        // `apply_events_to_state(...)` below.
        let mut root = Node::new()
            .with_zoom(1.5)
            .with_id("ui_root")
            .with_layout_direction(Layout::Stack)
            .with_width(Size::Fill)
            .with_height(Size::Fill)
            .with_padding(Spacing::all(Size::lpx(12.0)))
            .with_children(vec![
                // Statistics (top-left)
                self.stats_panel(ui_state).with_place(Place::Alignment {
                    h_align: HorizontalAlign::Left,
                    v_align: VerticalAlign::Top,
                }),
                // Render + LOD (top-right)
                self.render_lod_panel().with_place(Place::Alignment {
                    h_align: HorizontalAlign::Right,
                    v_align: VerticalAlign::Top,
                }),
                // Physics params (bottom-left)
                self.physics_params_panel(ui_state)
                    .with_place(Place::Alignment {
                        h_align: HorizontalAlign::Left,
                        v_align: VerticalAlign::Bottom,
                    }),
                // Time controls (bottom-right)
                self.time_controls_panel(ui_state)
                    .with_place(Place::Alignment {
                        h_align: HorizontalAlign::Right,
                        v_align: VerticalAlign::Bottom,
                    }),
                // Atom card (top-center)
                self.atom_card(ui_state).with_place(Place::Alignment {
                    h_align: HorizontalAlign::Center,
                    v_align: VerticalAlign::Top,
                }),
            ]);

        // Layout (with measurer) so we can hit-test for interaction.
        //
        // NOTE: root-level zoom is already accounted for by the layout/output pipeline.
        // We avoid peeking `root.zoom()` here since it is not a public API.
        let window_rect = astra_gui::Rect::new([0.0, 0.0], [window_size[0], window_size[1]]);
        root.compute_layout_with_measurer(window_rect, &mut self.text_engine);

        // Restore scroll positions before dispatch so hit-testing matches what the user sees.
        self.event_dispatcher.restore_scroll_state(&mut root);

        // Dispatch input → targeted events + compute interaction states for styling/animation.
        let (events, interaction_states) =
            self.event_dispatcher.dispatch(&self.input_state, &mut root);
        self.last_events = events;

        // Estimate whether UI is consuming pointer/scroll input this frame (used for camera/picking gating).
        self.ui_consumed_pointer = !self.last_events.is_empty();

        // Update transitions and capture dimensions for next frame (hover/active animations).
        self.interactive_state_manager
            .inject_dimension_overrides(&mut root);
        self.interactive_state_manager
            .update_transitions(&mut root, &interaction_states);

        // Apply interaction events to widget/app state.
        self.apply_events_to_state(ui_state);

        // Sync scroll state for persistence across frames.
        self.event_dispatcher.sync_scroll_state(&root);

        // Build shapes from the laid-out node tree (and include optional debug shapes).
        let output = AstraFullOutput::from_laid_out_node(
            root,
            (window_size[0], window_size[1]),
            if debug_options.is_enabled() {
                Some(debug_options)
            } else {
                None
            },
        );

        // End frame: clear one-frame input deltas so presses/releases/type/scroll don't stick.
        self.input_state.begin_frame();

        output
    }

    fn panel_frame() -> Style {
        Style {
            fill_color: Some(mocha::BASE.with_alpha(0.98)),
            stroke: Some(Stroke::new(Size::lpx(1.0), mocha::SURFACE2)),
            corner_shape: Some(CornerShape::Round(Size::lpx(20.0))),
            ..Default::default()
        }
    }

    fn title_text(text: impl Into<String>) -> Node {
        Node::new().with_content(Content::Text(
            TextContent::new(text.into())
                .with_color(mocha::TEXT)
                .with_font_size(Size::lpx(18.0)),
        ))
    }

    fn line_text(text: impl Into<String>) -> Node {
        Node::new().with_content(Content::Text(
            TextContent::new(text.into())
                .with_color(mocha::SUBTEXT1)
                .with_font_size(Size::lpx(14.0)),
        ))
    }

    fn stats_panel(&mut self, ui_state: &UiState) -> Node {
        // Positioned by the root stack via per-child alignment.
        let inner = Node::new()
            .with_id("stats_panel_body")
            .with_layout_direction(Layout::Vertical)
            .with_gap(Size::lpx(6.0))
            .with_children(vec![
                Self::line_text(format!("FPS: {:.0}", ui_state.fps)),
                Self::line_text(format!("Frame: {:.2} ms", ui_state.frame_time)),
                Self::line_text(format!("Particles: {}", ui_state.particle_count)),
                Self::line_text(format!("Hadrons: {}", ui_state.hadron_count)),
                Self::line_text(format!("Protons: {}", ui_state.proton_count)),
                Self::line_text(format!("Neutrons: {}", ui_state.neutron_count)),
                Self::line_text(format!("Other: {}", ui_state.other_hadron_count)),
            ]);

        Node::new()
            .with_id("stats_panel")
            .with_width(Size::lpx(220.0))
            .with_padding(Spacing::all(Size::lpx(6.0)))
            .with_child(collapsible(
                "stats_panel_collapsible",
                "Statistics",
                self.stats_panel_expanded,
                false,
                vec![inner],
                &CollapsibleStyle::default()
                    .with_title_font_size(18.0)
                    .with_header_padding(Spacing::all(Size::lpx(10.0)))
                    .with_content_padding(Spacing::trbl(
                        Size::lpx(6.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                    )),
            ))
    }

    fn panel_section_title(text: impl Into<String>) -> Node {
        Node::new().with_content(Content::Text(
            TextContent::new(text.into())
                .with_color(mocha::SUBTEXT0)
                .with_font_size(Size::lpx(13.0)),
        ))
    }

    fn labeled_row(label: impl Into<String>, value: Node) -> Node {
        Node::new()
            .with_layout_direction(Layout::Horizontal)
            .with_gap(Size::lpx(10.0))
            .with_children(vec![
                Node::new()
                    .with_width(Size::lpx(120.0))
                    .with_content(Content::Text(
                        TextContent::new(label.into())
                            .with_color(mocha::SUBTEXT1)
                            .with_font_size(Size::lpx(13.0)),
                    )),
                value,
            ])
    }

    fn slider_with_value_row(
        label: &'static str,
        slider_id: &'static str,
        value_id: &'static str,
        value: f32,
        range: std::ops::RangeInclusive<f32>,
        focused: bool,
        text_buffer: &str,
        cursor_pos: usize,
        selection: Option<(usize, usize)>,
        text_engine: &mut TextEngine,
        event_dispatcher: &mut EventDispatcher,
    ) -> Node {
        Self::labeled_row(
            label,
            slider_with_value(
                slider_id,
                value_id,
                value,
                range,
                focused,
                false,
                &SliderStyle::default(),
                &DragValueStyle::default(),
                text_buffer,
                cursor_pos,
                selection,
                text_engine,
                event_dispatcher,
            ),
        )
    }

    fn toggle_row(id: &'static str, label: &'static str, checked: bool) -> Node {
        Self::labeled_row(label, toggle(id, checked, false, &ToggleStyle::default()))
    }

    fn render_lod_panel(&mut self) -> Node {
        // Always render the header; only render the heavy/interactive body when expanded.
        let inner_children = if self.render_lod_panel_expanded {
            vec![
                Self::panel_section_title("Render"),
                Self::toggle_row("toggle_shells", "Show shells", self.render_shells),
                Self::toggle_row("toggle_bonds", "Show bonds", self.render_bonds),
                Self::toggle_row("toggle_nuclei", "Show nuclei", self.render_nuclei),
                Self::panel_section_title("LOD (fade start/end)"),
                Self::slider_with_value_row(
                    "Shell start",
                    "lod_shell_fade_start",
                    "lod_shell_fade_start_value",
                    self.lod_shell_fade_start,
                    0.0..=200.0,
                    self.lod_shell_fade_start_focused,
                    &self.lod_shell_fade_start_text,
                    self.lod_shell_fade_start_cursor,
                    self.lod_shell_fade_start_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Shell end",
                    "lod_shell_fade_end",
                    "lod_shell_fade_end_value",
                    self.lod_shell_fade_end,
                    0.0..=200.0,
                    self.lod_shell_fade_end_focused,
                    &self.lod_shell_fade_end_text,
                    self.lod_shell_fade_end_cursor,
                    self.lod_shell_fade_end_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Hadron start",
                    "lod_bound_hadron_fade_start",
                    "lod_bound_hadron_fade_start_value",
                    self.lod_bound_hadron_fade_start,
                    0.0..=200.0,
                    self.lod_bound_hadron_fade_start_focused,
                    &self.lod_bound_hadron_fade_start_text,
                    self.lod_bound_hadron_fade_start_cursor,
                    self.lod_bound_hadron_fade_start_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Hadron end",
                    "lod_bound_hadron_fade_end",
                    "lod_bound_hadron_fade_end_value",
                    self.lod_bound_hadron_fade_end,
                    0.0..=200.0,
                    self.lod_bound_hadron_fade_end_focused,
                    &self.lod_bound_hadron_fade_end_text,
                    self.lod_bound_hadron_fade_end_cursor,
                    self.lod_bound_hadron_fade_end_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Bond start",
                    "lod_bond_fade_start",
                    "lod_bond_fade_start_value",
                    self.lod_bond_fade_start,
                    0.0..=200.0,
                    self.lod_bond_fade_start_focused,
                    &self.lod_bond_fade_start_text,
                    self.lod_bond_fade_start_cursor,
                    self.lod_bond_fade_start_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Bond end",
                    "lod_bond_fade_end",
                    "lod_bond_fade_end_value",
                    self.lod_bond_fade_end,
                    0.0..=200.0,
                    self.lod_bond_fade_end_focused,
                    &self.lod_bond_fade_end_text,
                    self.lod_bond_fade_end_cursor,
                    self.lod_bond_fade_end_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Quark start",
                    "lod_quark_fade_start",
                    "lod_quark_fade_start_value",
                    self.lod_quark_fade_start,
                    0.0..=200.0,
                    self.lod_quark_fade_start_focused,
                    &self.lod_quark_fade_start_text,
                    self.lod_quark_fade_start_cursor,
                    self.lod_quark_fade_start_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Quark end",
                    "lod_quark_fade_end",
                    "lod_quark_fade_end_value",
                    self.lod_quark_fade_end,
                    0.0..=200.0,
                    self.lod_quark_fade_end_focused,
                    &self.lod_quark_fade_end_text,
                    self.lod_quark_fade_end_cursor,
                    self.lod_quark_fade_end_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Nucleus start",
                    "lod_nucleus_fade_start",
                    "lod_nucleus_fade_start_value",
                    self.lod_nucleus_fade_start,
                    0.0..=200.0,
                    self.lod_nucleus_fade_start_focused,
                    &self.lod_nucleus_fade_start_text,
                    self.lod_nucleus_fade_start_cursor,
                    self.lod_nucleus_fade_start_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Nucleus end",
                    "lod_nucleus_fade_end",
                    "lod_nucleus_fade_end_value",
                    self.lod_nucleus_fade_end,
                    0.0..=200.0,
                    self.lod_nucleus_fade_end_focused,
                    &self.lod_nucleus_fade_end_text,
                    self.lod_nucleus_fade_end_cursor,
                    self.lod_nucleus_fade_end_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
            ]
        } else {
            Vec::new()
        };

        let inner = Node::new()
            .with_id("render_lod_panel_body")
            .with_layout_direction(Layout::Vertical)
            .with_gap(Size::lpx(10.0))
            .with_children(inner_children);

        Node::new()
            .with_id("render_lod_panel")
            .with_width(Size::lpx(455.0))
            .with_padding(Spacing::all(Size::lpx(6.0)))
            .with_child(collapsible(
                "render_lod_panel_collapsible",
                "Render + LOD",
                self.render_lod_panel_expanded,
                false,
                vec![inner],
                &CollapsibleStyle::default()
                    .with_title_font_size(18.0)
                    .with_header_padding(Spacing::all(Size::lpx(10.0)))
                    .with_content_padding(Spacing::trbl(
                        Size::lpx(6.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                    )),
            ))
    }

    fn physics_params_panel(&mut self, ui_state: &UiState) -> Node {
        let params = ui_state.physics_params;

        // Always render the header; only build the heavy/interactive body when expanded.
        let inner_children = if self.physics_panel_expanded {
            vec![
                Self::panel_section_title("Forces"),
                // constants: x: G, y: K_electric, z: G_weak, w: weak_force_range
                Self::slider_with_value_row(
                    "Gravity (G)",
                    "phys_constants_g",
                    "phys_constants_g_value",
                    params.constants[0],
                    0.0..=1.0e-9,
                    self.phys_constants_g_focused,
                    &self.phys_constants_g_text,
                    self.phys_constants_g_cursor,
                    self.phys_constants_g_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Electric (K)",
                    "phys_constants_k",
                    "phys_constants_k_value",
                    params.constants[1],
                    0.0..=20.0,
                    self.phys_constants_k_focused,
                    &self.phys_constants_k_text,
                    self.phys_constants_k_cursor,
                    self.phys_constants_k_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Weak (G)",
                    "phys_constants_gweak",
                    "phys_constants_gweak_value",
                    params.constants[2],
                    0.0..=1.0e-3,
                    self.phys_constants_gweak_focused,
                    &self.phys_constants_gweak_text,
                    self.phys_constants_gweak_cursor,
                    self.phys_constants_gweak_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Weak range",
                    "phys_constants_weak_range",
                    "phys_constants_weak_range_value",
                    params.constants[3],
                    0.0..=5.0,
                    self.phys_constants_weak_range_focused,
                    &self.phys_constants_weak_range_text,
                    self.phys_constants_weak_range_cursor,
                    self.phys_constants_weak_range_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::panel_section_title("Strong Force"),
                // strong_force: x: strong_short_range, y: strong_confinement, z: strong_range, w: padding
                Self::slider_with_value_row(
                    "Short Range",
                    "phys_strong_short",
                    "phys_strong_short_value",
                    params.strong_force[0],
                    0.0..=5.0,
                    self.phys_strong_short_focused,
                    &self.phys_strong_short_text,
                    self.phys_strong_short_cursor,
                    self.phys_strong_short_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Confinement",
                    "phys_strong_confinement",
                    "phys_strong_confinement_value",
                    params.strong_force[1],
                    0.0..=5.0,
                    self.phys_strong_confinement_focused,
                    &self.phys_strong_confinement_text,
                    self.phys_strong_confinement_cursor,
                    self.phys_strong_confinement_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Range Cutoff",
                    "phys_strong_range",
                    "phys_strong_range_value",
                    params.strong_force[2],
                    0.0..=10.0,
                    self.phys_strong_range_focused,
                    &self.phys_strong_range_text,
                    self.phys_strong_range_cursor,
                    self.phys_strong_range_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::panel_section_title("Repulsion"),
                // repulsion: x: core_repulsion, y: core_radius, z: softening, w: max_force
                Self::slider_with_value_row(
                    "Core Strength",
                    "phys_repulsion_strength",
                    "phys_repulsion_strength_value",
                    params.repulsion[0],
                    0.0..=500.0,
                    self.phys_repulsion_strength_focused,
                    &self.phys_repulsion_strength_text,
                    self.phys_repulsion_strength_cursor,
                    self.phys_repulsion_strength_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Core Radius",
                    "phys_repulsion_radius",
                    "phys_repulsion_radius_value",
                    params.repulsion[1],
                    0.0..=1.0,
                    self.phys_repulsion_radius_focused,
                    &self.phys_repulsion_radius_text,
                    self.phys_repulsion_radius_cursor,
                    self.phys_repulsion_radius_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Softening",
                    "phys_repulsion_softening",
                    "phys_repulsion_softening_value",
                    params.repulsion[2],
                    0.001..=0.1,
                    self.phys_repulsion_softening_focused,
                    &self.phys_repulsion_softening_text,
                    self.phys_repulsion_softening_cursor,
                    self.phys_repulsion_softening_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Max Force",
                    "phys_repulsion_max_force",
                    "phys_repulsion_max_force_value",
                    params.repulsion[3],
                    10.0..=200.0,
                    self.phys_repulsion_max_force_focused,
                    &self.phys_repulsion_max_force_text,
                    self.phys_repulsion_max_force_cursor,
                    self.phys_repulsion_max_force_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::panel_section_title("Integration"),
                // integration: x: dt, y: damping, z: time/seed, w: nucleon_damping
                Self::slider_with_value_row(
                    "Damping",
                    "phys_integration_damping",
                    "phys_integration_damping_value",
                    params.integration[1],
                    0.9..=1.0,
                    self.phys_integration_damping_focused,
                    &self.phys_integration_damping_text,
                    self.phys_integration_damping_cursor,
                    self.phys_integration_damping_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Nucleon damp",
                    "phys_integration_nucleon_damping",
                    "phys_integration_nucleon_damping_value",
                    params.integration[3],
                    0.0..=5.0,
                    self.phys_integration_nucleon_damping_focused,
                    &self.phys_integration_nucleon_damping_text,
                    self.phys_integration_nucleon_damping_cursor,
                    self.phys_integration_nucleon_damping_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::panel_section_title("Nucleon Physics"),
                // nucleon: x: binding_strength, y: binding_range, z: exclusion_strength, w: exclusion_radius
                Self::slider_with_value_row(
                    "Bind strength",
                    "phys_nucleon_binding_strength",
                    "phys_nucleon_binding_strength_value",
                    params.nucleon[0],
                    0.0..=500.0,
                    self.phys_nucleon_binding_strength_focused,
                    &self.phys_nucleon_binding_strength_text,
                    self.phys_nucleon_binding_strength_cursor,
                    self.phys_nucleon_binding_strength_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Bind range",
                    "phys_nucleon_binding_range",
                    "phys_nucleon_binding_range_value",
                    params.nucleon[1],
                    0.0..=10.0,
                    self.phys_nucleon_binding_range_focused,
                    &self.phys_nucleon_binding_range_text,
                    self.phys_nucleon_binding_range_cursor,
                    self.phys_nucleon_binding_range_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Excl strength",
                    "phys_nucleon_exclusion_strength",
                    "phys_nucleon_exclusion_strength_value",
                    params.nucleon[2],
                    0.0..=500.0,
                    self.phys_nucleon_exclusion_strength_focused,
                    &self.phys_nucleon_exclusion_strength_text,
                    self.phys_nucleon_exclusion_strength_cursor,
                    self.phys_nucleon_exclusion_strength_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Excl radius",
                    "phys_nucleon_exclusion_radius",
                    "phys_nucleon_exclusion_radius_value",
                    params.nucleon[3],
                    0.0..=5.0,
                    self.phys_nucleon_exclusion_radius_focused,
                    &self.phys_nucleon_exclusion_radius_text,
                    self.phys_nucleon_exclusion_radius_cursor,
                    self.phys_nucleon_exclusion_radius_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::panel_section_title("Electron Physics"),
                // electron: x: exclusion_strength, y: exclusion_radius, z: padding, w: padding
                Self::slider_with_value_row(
                    "Excl strength",
                    "phys_electron_exclusion_strength",
                    "phys_electron_exclusion_strength_value",
                    params.electron[0],
                    0.0..=500.0,
                    self.phys_electron_exclusion_strength_focused,
                    &self.phys_electron_exclusion_strength_text,
                    self.phys_electron_exclusion_strength_cursor,
                    self.phys_electron_exclusion_strength_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Excl radius",
                    "phys_electron_exclusion_radius",
                    "phys_electron_exclusion_radius_value",
                    params.electron[1],
                    0.0..=5.0,
                    self.phys_electron_exclusion_radius_focused,
                    &self.phys_electron_exclusion_radius_text,
                    self.phys_electron_exclusion_radius_cursor,
                    self.phys_electron_exclusion_radius_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::panel_section_title("Hadron Formation"),
                // hadron: x: binding_distance, y: breakup_distance, z: confinement_range_mult, w: confinement_strength_mult
                Self::slider_with_value_row(
                    "Bind dist",
                    "phys_hadron_binding_distance",
                    "phys_hadron_binding_distance_value",
                    params.hadron[0],
                    0.0..=5.0,
                    self.phys_hadron_binding_distance_focused,
                    &self.phys_hadron_binding_distance_text,
                    self.phys_hadron_binding_distance_cursor,
                    self.phys_hadron_binding_distance_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Break dist",
                    "phys_hadron_breakup_distance",
                    "phys_hadron_breakup_distance_value",
                    params.hadron[1],
                    0.0..=5.0,
                    self.phys_hadron_breakup_distance_focused,
                    &self.phys_hadron_breakup_distance_text,
                    self.phys_hadron_breakup_distance_cursor,
                    self.phys_hadron_breakup_distance_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Conf range",
                    "phys_hadron_conf_range_mult",
                    "phys_hadron_conf_range_mult_value",
                    params.hadron[2],
                    0.0..=5.0,
                    self.phys_hadron_conf_range_mult_focused,
                    &self.phys_hadron_conf_range_mult_text,
                    self.phys_hadron_conf_range_mult_cursor,
                    self.phys_hadron_conf_range_mult_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Conf strength",
                    "phys_hadron_conf_strength_mult",
                    "phys_hadron_conf_strength_mult_value",
                    params.hadron[3],
                    0.0..=5.0,
                    self.phys_hadron_conf_strength_mult_focused,
                    &self.phys_hadron_conf_strength_mult_text,
                    self.phys_hadron_conf_strength_mult_cursor,
                    self.phys_hadron_conf_strength_mult_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::line_text(if self.physics_params_dirty {
                    "Pending: upload needed"
                } else {
                    "Synced"
                }),
            ]
        } else {
            Vec::new()
        };

        let inner = Node::new()
            .with_id("physics_params_panel_body")
            .with_layout_direction(Layout::Vertical)
            .with_gap(Size::lpx(10.0))
            .with_children(inner_children);

        Node::new()
            .with_id("physics_params_panel")
            .with_width(Size::lpx(455.0))
            .with_padding(Spacing::all(Size::lpx(6.0)))
            .with_child(collapsible(
                "physics_params_panel_collapsible",
                "Physics Controls",
                self.physics_panel_expanded,
                false,
                vec![inner],
                &CollapsibleStyle::default()
                    .with_title_font_size(18.0)
                    .with_header_padding(Spacing::all(Size::lpx(10.0)))
                    .with_content_padding(Spacing::trbl(
                        Size::lpx(6.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                    )),
            ))
    }

    fn time_controls_panel(&mut self, ui_state: &UiState) -> Node {
        let steps_remaining = ui_state.steps_remaining;

        // Always render the header; only build the interactive body when expanded.
        let inner_children = if self.time_panel_expanded {
            vec![
                Self::title_text("Time"),
                Node::new()
                    .with_layout_direction(Layout::Horizontal)
                    .with_gap(Size::lpx(10.0))
                    .with_children(vec![
                        button(
                            "time_pause_toggle",
                            if self.is_paused { "Play" } else { "Pause" },
                            false,
                            &ButtonStyle::default(),
                        ),
                        button("time_step", "Step", false, &ButtonStyle::default()),
                    ]),
                Self::slider_with_value_row(
                    "dt",
                    "physics_dt",
                    "physics_dt_value",
                    ui_state.physics_params.integration[0],
                    0.0001..=0.05,
                    self.physics_dt_focused,
                    &self.physics_dt_text,
                    self.physics_dt_cursor,
                    self.physics_dt_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::slider_with_value_row(
                    "Steps/play",
                    "time_steps_to_play",
                    "time_steps_to_play_value",
                    self.steps_to_play,
                    1.0..=240.0,
                    self.time_steps_to_play_focused,
                    &self.time_steps_to_play_text,
                    self.time_steps_to_play_cursor,
                    self.time_steps_to_play_selection,
                    &mut self.text_engine,
                    &mut self.event_dispatcher,
                ),
                Self::line_text(format!("Remaining: {steps_remaining}")),
            ]
        } else {
            Vec::new()
        };

        let inner = Node::new()
            .with_id("time_controls_panel_body")
            .with_layout_direction(Layout::Vertical)
            .with_gap(Size::lpx(10.0))
            .with_children(inner_children);

        Node::new()
            .with_id("time_controls_panel")
            .with_width(Size::lpx(455.0))
            .with_padding(Spacing::all(Size::lpx(6.0)))
            .with_child(collapsible(
                "time_controls_panel_collapsible",
                "Time Controls",
                self.time_panel_expanded,
                false,
                vec![inner],
                &CollapsibleStyle::default()
                    .with_title_font_size(18.0)
                    .with_header_padding(Spacing::all(Size::lpx(10.0)))
                    .with_content_padding(Spacing::trbl(
                        Size::lpx(6.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                    )),
            ))
    }

    fn apply_events_to_state(&mut self, ui_state: &mut UiState) {
        // Per-panel collapsibles
        if collapsible_clicked("stats_panel_collapsible", &self.last_events) {
            self.stats_panel_expanded = !self.stats_panel_expanded;
        }
        if collapsible_clicked("render_lod_panel_collapsible", &self.last_events) {
            self.render_lod_panel_expanded = !self.render_lod_panel_expanded;
        }
        if collapsible_clicked("physics_params_panel_collapsible", &self.last_events) {
            self.physics_panel_expanded = !self.physics_panel_expanded;
        }
        if collapsible_clicked("time_controls_panel_collapsible", &self.last_events) {
            self.time_panel_expanded = !self.time_panel_expanded;
        }
        if collapsible_clicked("atom_card_collapsible", &self.last_events) {
            self.atom_card_expanded = !self.atom_card_expanded;
        }

        // Render toggles
        if toggle_clicked("toggle_shells", &self.last_events) {
            self.render_shells = !self.render_shells;
            ui_state.show_shells = self.render_shells;
        }
        if toggle_clicked("toggle_bonds", &self.last_events) {
            self.render_bonds = !self.render_bonds;
            ui_state.show_bonds = self.render_bonds;
        }
        if toggle_clicked("toggle_nuclei", &self.last_events) {
            self.render_nuclei = !self.render_nuclei;
            ui_state.show_nuclei = self.render_nuclei;
        }

        // LOD sliders (continuous, with drag-value)
        if slider_with_value_update(
            "lod_shell_fade_start",
            "lod_shell_fade_start_value",
            &mut self.lod_shell_fade_start,
            &mut self.lod_shell_fade_start_text,
            &mut self.lod_shell_fade_start_cursor,
            &mut self.lod_shell_fade_start_selection,
            &mut self.lod_shell_fade_start_focused,
            &mut self.lod_shell_fade_start_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_shell_fade_start = self.lod_shell_fade_start;
        }

        if slider_with_value_update(
            "lod_shell_fade_end",
            "lod_shell_fade_end_value",
            &mut self.lod_shell_fade_end,
            &mut self.lod_shell_fade_end_text,
            &mut self.lod_shell_fade_end_cursor,
            &mut self.lod_shell_fade_end_selection,
            &mut self.lod_shell_fade_end_focused,
            &mut self.lod_shell_fade_end_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_shell_fade_end = self.lod_shell_fade_end;
        }

        if slider_with_value_update(
            "lod_bound_hadron_fade_start",
            "lod_bound_hadron_fade_start_value",
            &mut self.lod_bound_hadron_fade_start,
            &mut self.lod_bound_hadron_fade_start_text,
            &mut self.lod_bound_hadron_fade_start_cursor,
            &mut self.lod_bound_hadron_fade_start_selection,
            &mut self.lod_bound_hadron_fade_start_focused,
            &mut self.lod_bound_hadron_fade_start_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_bound_hadron_fade_start = self.lod_bound_hadron_fade_start;
        }

        if slider_with_value_update(
            "lod_bound_hadron_fade_end",
            "lod_bound_hadron_fade_end_value",
            &mut self.lod_bound_hadron_fade_end,
            &mut self.lod_bound_hadron_fade_end_text,
            &mut self.lod_bound_hadron_fade_end_cursor,
            &mut self.lod_bound_hadron_fade_end_selection,
            &mut self.lod_bound_hadron_fade_end_focused,
            &mut self.lod_bound_hadron_fade_end_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_bound_hadron_fade_end = self.lod_bound_hadron_fade_end;
        }

        if slider_with_value_update(
            "lod_bond_fade_start",
            "lod_bond_fade_start_value",
            &mut self.lod_bond_fade_start,
            &mut self.lod_bond_fade_start_text,
            &mut self.lod_bond_fade_start_cursor,
            &mut self.lod_bond_fade_start_selection,
            &mut self.lod_bond_fade_start_focused,
            &mut self.lod_bond_fade_start_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_bond_fade_start = self.lod_bond_fade_start;
        }

        if slider_with_value_update(
            "lod_bond_fade_end",
            "lod_bond_fade_end_value",
            &mut self.lod_bond_fade_end,
            &mut self.lod_bond_fade_end_text,
            &mut self.lod_bond_fade_end_cursor,
            &mut self.lod_bond_fade_end_selection,
            &mut self.lod_bond_fade_end_focused,
            &mut self.lod_bond_fade_end_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_bond_fade_end = self.lod_bond_fade_end;
        }

        if slider_with_value_update(
            "lod_quark_fade_start",
            "lod_quark_fade_start_value",
            &mut self.lod_quark_fade_start,
            &mut self.lod_quark_fade_start_text,
            &mut self.lod_quark_fade_start_cursor,
            &mut self.lod_quark_fade_start_selection,
            &mut self.lod_quark_fade_start_focused,
            &mut self.lod_quark_fade_start_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_quark_fade_start = self.lod_quark_fade_start;
        }

        if slider_with_value_update(
            "lod_quark_fade_end",
            "lod_quark_fade_end_value",
            &mut self.lod_quark_fade_end,
            &mut self.lod_quark_fade_end_text,
            &mut self.lod_quark_fade_end_cursor,
            &mut self.lod_quark_fade_end_selection,
            &mut self.lod_quark_fade_end_focused,
            &mut self.lod_quark_fade_end_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_quark_fade_end = self.lod_quark_fade_end;
        }

        if slider_with_value_update(
            "lod_nucleus_fade_start",
            "lod_nucleus_fade_start_value",
            &mut self.lod_nucleus_fade_start,
            &mut self.lod_nucleus_fade_start_text,
            &mut self.lod_nucleus_fade_start_cursor,
            &mut self.lod_nucleus_fade_start_selection,
            &mut self.lod_nucleus_fade_start_focused,
            &mut self.lod_nucleus_fade_start_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_nucleus_fade_start = self.lod_nucleus_fade_start;
        }

        if slider_with_value_update(
            "lod_nucleus_fade_end",
            "lod_nucleus_fade_end_value",
            &mut self.lod_nucleus_fade_end,
            &mut self.lod_nucleus_fade_end_text,
            &mut self.lod_nucleus_fade_end_cursor,
            &mut self.lod_nucleus_fade_end_selection,
            &mut self.lod_nucleus_fade_end_focused,
            &mut self.lod_nucleus_fade_end_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=200.0,
            0.05,
            None,
        ) {
            ui_state.lod_nucleus_fade_end = self.lod_nucleus_fade_end;
        }

        // Time step dt (physics_params.integration.x)
        let mut dt = ui_state.physics_params.integration[0];
        if slider_with_value_update(
            "physics_dt",
            "physics_dt_value",
            &mut dt,
            &mut self.physics_dt_text,
            &mut self.physics_dt_cursor,
            &mut self.physics_dt_selection,
            &mut self.physics_dt_focused,
            &mut self.physics_dt_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0001..=0.05,
            0.00002,
            None,
        ) {
            ui_state.physics_params.integration[0] = dt;
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // Time controls
        if button_clicked("time_pause_toggle", &self.last_events) {
            self.is_paused = !self.is_paused;
            ui_state.is_paused = self.is_paused;
        }
        if button_clicked("time_step", &self.last_events) {
            ui_state.step_one_frame = true;
            self.step_one_frame = true;
        }

        if slider_with_value_update(
            "time_steps_to_play",
            "time_steps_to_play_value",
            &mut self.steps_to_play,
            &mut self.time_steps_to_play_text,
            &mut self.time_steps_to_play_cursor,
            &mut self.time_steps_to_play_selection,
            &mut self.time_steps_to_play_focused,
            &mut self.time_steps_to_play_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            1.0..=240.0,
            0.05,
            Some(1.0),
        ) {
            ui_state.steps_to_play = self.steps_to_play.round().clamp(1.0, 240.0) as u32;
        }

        // Physics controls (write-through to UiState + mark dirty)
        // constants: x: G, y: K_electric, z: G_weak, w: weak_force_range
        if slider_with_value_update(
            "phys_constants_g",
            "phys_constants_g_value",
            &mut ui_state.physics_params.constants[0],
            &mut self.phys_constants_g_text,
            &mut self.phys_constants_g_cursor,
            &mut self.phys_constants_g_selection,
            &mut self.phys_constants_g_focused,
            &mut self.phys_constants_g_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=1.0e-9,
            1.0e-12,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_constants_k",
            "phys_constants_k_value",
            &mut ui_state.physics_params.constants[1],
            &mut self.phys_constants_k_text,
            &mut self.phys_constants_k_cursor,
            &mut self.phys_constants_k_selection,
            &mut self.phys_constants_k_focused,
            &mut self.phys_constants_k_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=20.0,
            0.05,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_constants_gweak",
            "phys_constants_gweak_value",
            &mut ui_state.physics_params.constants[2],
            &mut self.phys_constants_gweak_text,
            &mut self.phys_constants_gweak_cursor,
            &mut self.phys_constants_gweak_selection,
            &mut self.phys_constants_gweak_focused,
            &mut self.phys_constants_gweak_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=1.0e-3,
            0.00001,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_constants_weak_range",
            "phys_constants_weak_range_value",
            &mut ui_state.physics_params.constants[3],
            &mut self.phys_constants_weak_range_text,
            &mut self.phys_constants_weak_range_cursor,
            &mut self.phys_constants_weak_range_selection,
            &mut self.phys_constants_weak_range_focused,
            &mut self.phys_constants_weak_range_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // strong_force: x: strong_short_range, y: strong_confinement, z: strong_range
        if slider_with_value_update(
            "phys_strong_short",
            "phys_strong_short_value",
            &mut ui_state.physics_params.strong_force[0],
            &mut self.phys_strong_short_text,
            &mut self.phys_strong_short_cursor,
            &mut self.phys_strong_short_selection,
            &mut self.phys_strong_short_focused,
            &mut self.phys_strong_short_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_strong_confinement",
            "phys_strong_confinement_value",
            &mut ui_state.physics_params.strong_force[1],
            &mut self.phys_strong_confinement_text,
            &mut self.phys_strong_confinement_cursor,
            &mut self.phys_strong_confinement_selection,
            &mut self.phys_strong_confinement_focused,
            &mut self.phys_strong_confinement_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_strong_range",
            "phys_strong_range_value",
            &mut ui_state.physics_params.strong_force[2],
            &mut self.phys_strong_range_text,
            &mut self.phys_strong_range_cursor,
            &mut self.phys_strong_range_selection,
            &mut self.phys_strong_range_focused,
            &mut self.phys_strong_range_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=10.0,
            0.02,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // repulsion: x: core_repulsion, y: core_radius, z: softening, w: max_force
        if slider_with_value_update(
            "phys_repulsion_strength",
            "phys_repulsion_strength_value",
            &mut ui_state.physics_params.repulsion[0],
            &mut self.phys_repulsion_strength_text,
            &mut self.phys_repulsion_strength_cursor,
            &mut self.phys_repulsion_strength_selection,
            &mut self.phys_repulsion_strength_focused,
            &mut self.phys_repulsion_strength_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=500.0,
            0.2,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_repulsion_radius",
            "phys_repulsion_radius_value",
            &mut ui_state.physics_params.repulsion[1],
            &mut self.phys_repulsion_radius_text,
            &mut self.phys_repulsion_radius_cursor,
            &mut self.phys_repulsion_radius_selection,
            &mut self.phys_repulsion_radius_focused,
            &mut self.phys_repulsion_radius_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=1.0,
            0.005,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_repulsion_softening",
            "phys_repulsion_softening_value",
            &mut ui_state.physics_params.repulsion[2],
            &mut self.phys_repulsion_softening_text,
            &mut self.phys_repulsion_softening_cursor,
            &mut self.phys_repulsion_softening_selection,
            &mut self.phys_repulsion_softening_focused,
            &mut self.phys_repulsion_softening_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.001..=0.1,
            0.00025,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_repulsion_max_force",
            "phys_repulsion_max_force_value",
            &mut ui_state.physics_params.repulsion[3],
            &mut self.phys_repulsion_max_force_text,
            &mut self.phys_repulsion_max_force_cursor,
            &mut self.phys_repulsion_max_force_selection,
            &mut self.phys_repulsion_max_force_focused,
            &mut self.phys_repulsion_max_force_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            10.0..=200.0,
            0.2,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // integration: y: damping, w: nucleon_damping  (dt is in Time panel; z is time/seed)
        if slider_with_value_update(
            "phys_integration_damping",
            "phys_integration_damping_value",
            &mut ui_state.physics_params.integration[1],
            &mut self.phys_integration_damping_text,
            &mut self.phys_integration_damping_cursor,
            &mut self.phys_integration_damping_selection,
            &mut self.phys_integration_damping_focused,
            &mut self.phys_integration_damping_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.9..=1.0,
            0.0001,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_integration_nucleon_damping",
            "phys_integration_nucleon_damping_value",
            &mut ui_state.physics_params.integration[3],
            &mut self.phys_integration_nucleon_damping_text,
            &mut self.phys_integration_nucleon_damping_cursor,
            &mut self.phys_integration_nucleon_damping_selection,
            &mut self.phys_integration_nucleon_damping_focused,
            &mut self.phys_integration_nucleon_damping_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // nucleon: x/y/z/w
        if slider_with_value_update(
            "phys_nucleon_binding_strength",
            "phys_nucleon_binding_strength_value",
            &mut ui_state.physics_params.nucleon[0],
            &mut self.phys_nucleon_binding_strength_text,
            &mut self.phys_nucleon_binding_strength_cursor,
            &mut self.phys_nucleon_binding_strength_selection,
            &mut self.phys_nucleon_binding_strength_focused,
            &mut self.phys_nucleon_binding_strength_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=500.0,
            0.2,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_nucleon_binding_range",
            "phys_nucleon_binding_range_value",
            &mut ui_state.physics_params.nucleon[1],
            &mut self.phys_nucleon_binding_range_text,
            &mut self.phys_nucleon_binding_range_cursor,
            &mut self.phys_nucleon_binding_range_selection,
            &mut self.phys_nucleon_binding_range_focused,
            &mut self.phys_nucleon_binding_range_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=10.0,
            0.02,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_nucleon_exclusion_strength",
            "phys_nucleon_exclusion_strength_value",
            &mut ui_state.physics_params.nucleon[2],
            &mut self.phys_nucleon_exclusion_strength_text,
            &mut self.phys_nucleon_exclusion_strength_cursor,
            &mut self.phys_nucleon_exclusion_strength_selection,
            &mut self.phys_nucleon_exclusion_strength_focused,
            &mut self.phys_nucleon_exclusion_strength_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=500.0,
            0.2,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_nucleon_exclusion_radius",
            "phys_nucleon_exclusion_radius_value",
            &mut ui_state.physics_params.nucleon[3],
            &mut self.phys_nucleon_exclusion_radius_text,
            &mut self.phys_nucleon_exclusion_radius_cursor,
            &mut self.phys_nucleon_exclusion_radius_selection,
            &mut self.phys_nucleon_exclusion_radius_focused,
            &mut self.phys_nucleon_exclusion_radius_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // electron: x/y
        if slider_with_value_update(
            "phys_electron_exclusion_strength",
            "phys_electron_exclusion_strength_value",
            &mut ui_state.physics_params.electron[0],
            &mut self.phys_electron_exclusion_strength_text,
            &mut self.phys_electron_exclusion_strength_cursor,
            &mut self.phys_electron_exclusion_strength_selection,
            &mut self.phys_electron_exclusion_strength_focused,
            &mut self.phys_electron_exclusion_strength_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=500.0,
            0.2,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_electron_exclusion_radius",
            "phys_electron_exclusion_radius_value",
            &mut ui_state.physics_params.electron[1],
            &mut self.phys_electron_exclusion_radius_text,
            &mut self.phys_electron_exclusion_radius_cursor,
            &mut self.phys_electron_exclusion_radius_selection,
            &mut self.phys_electron_exclusion_radius_focused,
            &mut self.phys_electron_exclusion_radius_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        // hadron: x/y/z/w
        if slider_with_value_update(
            "phys_hadron_binding_distance",
            "phys_hadron_binding_distance_value",
            &mut ui_state.physics_params.hadron[0],
            &mut self.phys_hadron_binding_distance_text,
            &mut self.phys_hadron_binding_distance_cursor,
            &mut self.phys_hadron_binding_distance_selection,
            &mut self.phys_hadron_binding_distance_focused,
            &mut self.phys_hadron_binding_distance_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_hadron_breakup_distance",
            "phys_hadron_breakup_distance_value",
            &mut ui_state.physics_params.hadron[1],
            &mut self.phys_hadron_breakup_distance_text,
            &mut self.phys_hadron_breakup_distance_cursor,
            &mut self.phys_hadron_breakup_distance_selection,
            &mut self.phys_hadron_breakup_distance_focused,
            &mut self.phys_hadron_breakup_distance_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_hadron_conf_range_mult",
            "phys_hadron_conf_range_mult_value",
            &mut ui_state.physics_params.hadron[2],
            &mut self.phys_hadron_conf_range_mult_text,
            &mut self.phys_hadron_conf_range_mult_cursor,
            &mut self.phys_hadron_conf_range_mult_selection,
            &mut self.phys_hadron_conf_range_mult_focused,
            &mut self.phys_hadron_conf_range_mult_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }

        if slider_with_value_update(
            "phys_hadron_conf_strength_mult",
            "phys_hadron_conf_strength_mult_value",
            &mut ui_state.physics_params.hadron[3],
            &mut self.phys_hadron_conf_strength_mult_text,
            &mut self.phys_hadron_conf_strength_mult_cursor,
            &mut self.phys_hadron_conf_strength_mult_selection,
            &mut self.phys_hadron_conf_strength_mult_focused,
            &mut self.phys_hadron_conf_strength_mult_drag_accumulator,
            &self.last_events,
            &self.input_state,
            &mut self.event_dispatcher,
            0.0..=5.0,
            0.01,
            None,
        ) {
            ui_state.physics_params_dirty = true;
            self.physics_params_dirty = true;
        }
    }

    fn atom_card(&mut self, ui_state: &UiState) -> Node {
        // Top-center, only when a nucleus is selected.
        let Some(z) = ui_state.selected_nucleus_atomic_number else {
            return Node::new()
                .with_id("atom_card_hidden")
                .with_h_align(HorizontalAlign::Center)
                .with_v_align(VerticalAlign::Top);
        };

        let name = element_name(z);
        let symbol = element_symbol(z);

        let mut children = vec![
            Self::line_text(format!("{name} ({symbol})")),
            Self::line_text(format!("Atomic Number (Z): {z}")),
        ];

        if let Some(p) = ui_state.selected_nucleus_proton_count {
            children.push(Self::line_text(format!("Protons: {p}")));
        }
        if let Some(n) = ui_state.selected_nucleus_neutron_count {
            children.push(Self::line_text(format!("Neutrons: {n}")));
        }
        if let Some(a) = ui_state.selected_nucleus_nucleon_count {
            children.push(Self::line_text(format!("Total Nucleons (A): {a}")));
            children.push(Self::line_text(format!("Isotope: {name}-{a}")));
        }

        let inner = Node::new()
            .with_id("atom_card_body")
            .with_layout_direction(Layout::Vertical)
            .with_gap(Size::lpx(6.0))
            .with_width(Size::lpx(240.0))
            .with_children(children);

        Node::new()
            .with_id("atom_card")
            .with_width(Size::lpx(270.0))
            .with_style(Self::panel_frame())
            .with_padding(Spacing::all(Size::lpx(6.0)))
            .with_child(collapsible(
                "atom_card_collapsible",
                "Atom",
                self.atom_card_expanded,
                false,
                vec![inner],
                &CollapsibleStyle::default()
                    .with_title_font_size(18.0)
                    .with_header_padding(Spacing::all(Size::lpx(10.0)))
                    .with_content_padding(Spacing::trbl(
                        Size::lpx(6.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                        Size::lpx(10.0),
                    )),
            ))
    }
}
