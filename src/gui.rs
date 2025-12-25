use astra_gui::{
    catppuccin::mocha, Content, FullOutput as AstraFullOutput, LayoutDirection, Node, Rect, Shape,
    Spacing, StyledRect, TextContent,
};
use egui::Context;
use egui_wgpu::Renderer;
use egui_winit::State;
use particle_simulation::PhysicsParams;
use wgpu::{Device, TextureFormat};
use winit::{event::WindowEvent, window::Window};

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

pub struct Gui {
    context: Context,
    state: State,
    renderer: Renderer,
}

impl Gui {
    pub fn new(device: &Device, output_color_format: TextureFormat, window: &Window) -> Self {
        let context = Context::default();
        let id = context.viewport_id();

        let state = State::new(
            context.clone(),
            id,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );

        let renderer = Renderer::new(
            device,
            output_color_format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            context,
            state,
            renderer,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        view: &wgpu::TextureView,
        ui_state: &mut UiState,
    ) {
        let raw_input = self.state.take_egui_input(window);

        let full_output = self.context.run(raw_input, |ctx| {
            self.ui(ctx, ui_state);
        });

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let clipped_primitives = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let size = window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Egui Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // SAFETY: Workaround for lifetime issues with egui-wgpu render pass
        let render_pass: &mut wgpu::RenderPass<'static> =
            unsafe { std::mem::transmute(&mut render_pass) };

        self.renderer
            .render(render_pass, &clipped_primitives, &screen_descriptor);

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }

    fn ui(&self, ctx: &Context, state: &mut UiState) {
        // Handle keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            state.is_paused = !state.is_paused;
        }

        if state.is_paused {
            let step_pressed = ctx.input(|i| {
                i.modifiers.ctrl
                    && (i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D))
            });
            if step_pressed {
                state.steps_remaining += state.steps_to_play;
            }
        }

        // Drive simulation stepping
        if state.steps_remaining > 0 {
            state.step_one_frame = true;
            state.steps_remaining -= 1;
        }

        // Diagnostics Panel (Top Left)
        egui::Window::new("Diagnostics")
            .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {:.1}", state.fps));
                ui.label(format!("Frame Time: {:.2} ms", state.frame_time));
            });

        // Statistics Panel (Top Right)
        egui::Window::new("Statistics")
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.heading("Particles");
                ui.label(format!("Total: {}", state.particle_count));
                ui.separator();

                ui.heading("Hadrons");
                ui.label(format!("Total: {}", state.hadron_count));
                ui.label(format!("Protons: {}", state.proton_count));
                ui.label(format!("Neutrons: {}", state.neutron_count));
                ui.label(format!("Other: {}", state.other_hadron_count));
                ui.separator();

                ui.heading("Rendering");
                ui.checkbox(&mut state.show_shells, "Show Hadrons");
                ui.checkbox(&mut state.show_bonds, "Show Quark Bonds");
                ui.checkbox(&mut state.show_nuclei, "Show Nuclei");

                ui.separator();
                ui.label("Shell LOD (Fade In):");
                ui.add(
                    egui::Slider::new(&mut state.lod_shell_fade_start, 5.0..=200.0)
                        .text("Shell Start")
                        .step_by(5.0),
                );
                ui.add(
                    egui::Slider::new(&mut state.lod_shell_fade_end, 5.0..=200.0)
                        .text("Shell End")
                        .step_by(5.0),
                );
                if state.lod_shell_fade_end < state.lod_shell_fade_start {
                    state.lod_shell_fade_end = state.lod_shell_fade_start;
                }

                ui.separator();
                ui.label("Bound Hadron LOD (Fade Out):");
                ui.add(
                    egui::Slider::new(&mut state.lod_bound_hadron_fade_start, 10.0..=300.0)
                        .text("Bound Start")
                        .step_by(10.0),
                );
                ui.add(
                    egui::Slider::new(&mut state.lod_bound_hadron_fade_end, 10.0..=300.0)
                        .text("Bound End")
                        .step_by(10.0),
                );
                if state.lod_bound_hadron_fade_end < state.lod_bound_hadron_fade_start {
                    state.lod_bound_hadron_fade_end = state.lod_bound_hadron_fade_start;
                }

                ui.separator();
                ui.label("Bond LOD (Fade Out):");
                ui.add(
                    egui::Slider::new(&mut state.lod_bond_fade_start, 5.0..=200.0)
                        .text("Bond Start")
                        .step_by(5.0),
                );
                ui.add(
                    egui::Slider::new(&mut state.lod_bond_fade_end, 5.0..=200.0)
                        .text("Bond End")
                        .step_by(5.0),
                );
                if state.lod_bond_fade_end < state.lod_bond_fade_start {
                    state.lod_bond_fade_end = state.lod_bond_fade_start;
                }

                ui.separator();
                ui.label("Quark LOD (Fade Out):");
                ui.add(
                    egui::Slider::new(&mut state.lod_quark_fade_start, 5.0..=200.0)
                        .text("Quark Start")
                        .step_by(5.0),
                );
                ui.add(
                    egui::Slider::new(&mut state.lod_quark_fade_end, 5.0..=200.0)
                        .text("Quark End")
                        .step_by(5.0),
                );
                if state.lod_quark_fade_end < state.lod_quark_fade_start {
                    state.lod_quark_fade_end = state.lod_quark_fade_start;
                }

                ui.separator();
                ui.label("Nucleus LOD (Fade In):");
                ui.add(
                    egui::Slider::new(&mut state.lod_nucleus_fade_start, 10.0..=300.0)
                        .text("Nucleus Start")
                        .step_by(10.0),
                );
                ui.add(
                    egui::Slider::new(&mut state.lod_nucleus_fade_end, 10.0..=300.0)
                        .text("Nucleus End")
                        .step_by(10.0),
                );
                if state.lod_nucleus_fade_end < state.lod_nucleus_fade_start {
                    state.lod_nucleus_fade_end = state.lod_nucleus_fade_start;
                }
            });

        // Physics Controls (Bottom Left)
        egui::Window::new("Physics Controls")
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .resizable(false)
            .collapsible(true)
            .default_open(false)
            .show(ctx, |ui| {
                ui.heading("Forces");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.constants[0], 0.0..=1.0e-9)
                            .text("Gravity (G)")
                            .logarithmic(true),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.constants[1], 0.0..=20.0)
                            .text("Electric (K)"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                ui.separator();
                ui.heading("Strong Force");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.strong_force[0], 0.0..=5.0)
                            .text("Short Range")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.strong_force[1], 0.0..=5.0)
                            .text("Confinement")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.strong_force[2], 0.0..=10.0)
                            .text("Range Cutoff")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                ui.separator();
                ui.heading("Repulsion");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.repulsion[0], 0.0..=500.0)
                            .text("Core Strength"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.repulsion[1], 0.0..=1.0)
                            .text("Core Radius"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.repulsion[2], 0.001..=0.1)
                            .text("Softening"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.repulsion[3], 10.0..=200.0)
                            .text("Max Force"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                ui.separator();
                ui.heading("Integration");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.integration[0], 0.0001..=0.01)
                            .text("Time Step (dt)")
                            .logarithmic(true),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.integration[1], 0.9..=1.0)
                            .text("Damping"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                ui.separator();
                ui.heading("Nucleon Physics");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.nucleon[0], 0.0..=200.0)
                            .text("Binding Strength"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.nucleon[1], 0.1..=10.0)
                            .text("Binding Range"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.nucleon[2], 0.0..=300.0)
                            .text("Exclusion Strength"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.nucleon[3], 0.5..=3.0)
                            .text("Exclusion Radius (x Hadron R)"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.integration[3], 0.0..=100.0)
                            .text("Nucleon Damping"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                ui.separator();
                ui.heading("Electron Physics");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.electron[0], 0.0..=200.0)
                            .text("Attraction Strength"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.electron[1], 0.1..=10.0)
                            .text("Attraction Range"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.electron[2], 0.0..=300.0)
                            .text("Exclusion Strength"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.electron[3], 0.5..=3.0)
                            .text("Exclusion Radius"),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                ui.separator();
                ui.heading("Hadron Formation");
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.hadron[0], 0.1..=3.0)
                            .text("Binding Distance")
                            .step_by(0.05),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.hadron[1], 0.1..=5.0)
                            .text("Breakup Distance")
                            .step_by(0.05),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.hadron[2], 0.1..=5.0)
                            .text("Confinement Range Mult")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.hadron[3], 0.1..=5.0)
                            .text("Confinement Strength Mult")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                // Keep invariants sane (avoid immediate breakup right after formation).
                if state.physics_params.hadron[1] < state.physics_params.hadron[0] {
                    state.physics_params.hadron[1] = state.physics_params.hadron[0];
                    state.physics_params_dirty = true;
                }
            });

        // Time Controls (Bottom Right)
        egui::Window::new("Time Controls")
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
            .resizable(false)
            .collapsible(true)
            .default_open(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(if state.is_paused {
                            "▶ Resume (Space)"
                        } else {
                            "⏸ Pause (Space)"
                        })
                        .clicked()
                    {
                        state.is_paused = !state.is_paused;
                    }
                });

                // Improvement retained: keep dt control in this quick-access panel.
                if ui
                    .add(
                        egui::Slider::new(&mut state.physics_params.integration[0], 0.0001..=0.01)
                            .text("Time Step (dt)")
                            .step_by(0.0001),
                    )
                    .changed()
                {
                    state.physics_params_dirty = true;
                }

                if state.is_paused {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Steps:");
                        ui.add(
                            egui::DragValue::new(&mut state.steps_to_play)
                                .speed(1)
                                .range(1..=1000),
                        );
                        if ui.button("Step ⏭ (Ctrl+Right/D)").clicked() {
                            state.steps_remaining += state.steps_to_play;
                        }
                    });
                }
            });

        // Atom Card (Center - only shown when nucleus is selected)
        if let Some(atomic_number) = state.selected_nucleus_atomic_number {
            egui::Window::new("Atom Card")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    // Element name from atomic number
                    let element_name = get_element_name(atomic_number);
                    let element_symbol = get_element_symbol(atomic_number);

                    ui.heading(format!("{} ({})", element_name, element_symbol));
                    ui.separator();

                    ui.label(format!("Atomic Number (Z): {}", atomic_number));

                    if let Some(protons) = state.selected_nucleus_proton_count {
                        ui.label(format!("Protons: {}", protons));
                    }

                    if let Some(neutrons) = state.selected_nucleus_neutron_count {
                        ui.label(format!("Neutrons: {}", neutrons));
                    }

                    if let Some(nucleons) = state.selected_nucleus_nucleon_count {
                        ui.label(format!("Total Nucleons (A): {}", nucleons));
                    }

                    // Show isotope notation if we have the data
                    if let (Some(_neutrons), Some(nucleons)) = (
                        state.selected_nucleus_neutron_count,
                        state.selected_nucleus_nucleon_count,
                    ) {
                        ui.separator();
                        ui.label(format!("Isotope: {}-{}", element_name, nucleons));
                    }
                });
        }
    }
}

// Complete periodic table - all 118 elements
const ELEMENT_NAMES: [&str; 119] = [
    "", // 0 (invalid)
    "Hydrogen",
    "Helium",
    "Lithium",
    "Beryllium",
    "Boron",
    "Carbon",
    "Nitrogen",
    "Oxygen",
    "Fluorine",
    "Neon",
    "Sodium",
    "Magnesium",
    "Aluminum",
    "Silicon",
    "Phosphorus",
    "Sulfur",
    "Chlorine",
    "Argon",
    "Potassium",
    "Calcium",
    "Scandium",
    "Titanium",
    "Vanadium",
    "Chromium",
    "Manganese",
    "Iron",
    "Cobalt",
    "Nickel",
    "Copper",
    "Zinc",
    "Gallium",
    "Germanium",
    "Arsenic",
    "Selenium",
    "Bromine",
    "Krypton",
    "Rubidium",
    "Strontium",
    "Yttrium",
    "Zirconium",
    "Niobium",
    "Molybdenum",
    "Technetium",
    "Ruthenium",
    "Rhodium",
    "Palladium",
    "Silver",
    "Cadmium",
    "Indium",
    "Tin",
    "Antimony",
    "Tellurium",
    "Iodine",
    "Xenon",
    "Cesium",
    "Barium",
    "Lanthanum",
    "Cerium",
    "Praseodymium",
    "Neodymium",
    "Promethium",
    "Samarium",
    "Europium",
    "Gadolinium",
    "Terbium",
    "Dysprosium",
    "Holmium",
    "Erbium",
    "Thulium",
    "Ytterbium",
    "Lutetium",
    "Hafnium",
    "Tantalum",
    "Tungsten",
    "Rhenium",
    "Osmium",
    "Iridium",
    "Platinum",
    "Gold",
    "Mercury",
    "Thallium",
    "Lead",
    "Bismuth",
    "Polonium",
    "Astatine",
    "Radon",
    "Francium",
    "Radium",
    "Actinium",
    "Thorium",
    "Protactinium",
    "Uranium",
    "Neptunium",
    "Plutonium",
    "Americium",
    "Curium",
    "Berkelium",
    "Californium",
    "Einsteinium",
    "Fermium",
    "Mendelevium",
    "Nobelium",
    "Lawrencium",
    "Rutherfordium",
    "Dubnium",
    "Seaborgium",
    "Bohrium",
    "Hassium",
    "Meitnerium",
    "Darmstadtium",
    "Roentgenium",
    "Copernicium",
    "Nihonium",
    "Flerovium",
    "Moscovium",
    "Livermorium",
    "Tennessine",
    "Oganesson",
];

const ELEMENT_SYMBOLS: [&str; 119] = [
    "", // 0 (invalid)
    "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S", "Cl",
    "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge", "As",
    "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd", "In",
    "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd", "Tb",
    "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg", "Tl",
    "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm", "Bk",
    "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh",
    "Fl", "Mc", "Lv", "Ts", "Og",
];

fn get_element_name(z: u32) -> &'static str {
    ELEMENT_NAMES.get(z as usize).copied().unwrap_or("Unknown")
}

fn get_element_symbol(z: u32) -> &'static str {
    ELEMENT_SYMBOLS.get(z as usize).copied().unwrap_or("?")
}

/// Build diagnostics panel using astra-gui
pub fn build_diagnostics_panel(ui_state: &UiState, window_size: [f32; 2]) -> AstraFullOutput {
    // Container with padding and background
    let container = Node::new()
        .with_padding(Spacing::all(10.0))
        .with_gap(5.0)
        .with_layout_direction(LayoutDirection::Vertical)
        .with_shape(Shape::Rect(StyledRect::new(
            Rect::new([10.0, 10.0], [200.0, 150.0]),
            mocha::SURFACE0,
        )))
        .with_children(vec![
            // Title
            Node::new().with_content(Content::Text(
                TextContent::new("Diagnostics")
                    .with_font_size(18.0)
                    .with_color(mocha::TEXT),
            )),
            // FPS label
            Node::new().with_content(Content::Text(
                TextContent::new(format!("FPS: {:.1}", ui_state.fps))
                    .with_font_size(16.0)
                    .with_color(mocha::TEXT),
            )),
            // Frame time label
            Node::new().with_content(Content::Text(
                TextContent::new(format!("Frame Time: {:.2} ms", ui_state.frame_time))
                    .with_font_size(16.0)
                    .with_color(mocha::TEXT),
            )),
        ]);

    AstraFullOutput::from_node_with_debug(container, (window_size[0], window_size[1]), None)
}
