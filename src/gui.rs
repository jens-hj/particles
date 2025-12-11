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
    pub hadron_count: u32,
    pub physics_params: PhysicsParams,
    pub show_shells: bool,
    pub show_bonds: bool,
    pub is_paused: bool,
    pub step_one_frame: bool,
    pub steps_to_play: u32,
    pub steps_remaining: u32,
    pub lod_shell_fade_start: f32,
    pub lod_shell_fade_end: f32,
    pub lod_bond_fade_start: f32,
    pub lod_bond_fade_end: f32,
    pub lod_quark_fade_start: f32,
    pub lod_quark_fade_end: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time: 0.0,
            particle_count: 0,
            hadron_count: 0,
            physics_params: PhysicsParams::default(),
            show_shells: true,
            show_bonds: true,
            is_paused: false,
            step_one_frame: false,
            steps_to_play: 1,
            steps_remaining: 0,
            lod_shell_fade_start: 10.0,
            lod_shell_fade_end: 30.0,
            lod_bond_fade_start: 10.0,
            lod_bond_fade_end: 30.0,
            lod_quark_fade_start: 10.0,
            lod_quark_fade_end: 30.0,
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

        let renderer = Renderer::new(device, output_color_format, None, 1, false);

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
                state.steps_remaining += 1;
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
                ui.label(format!("Detected: {}", state.hadron_count));
                ui.separator();
                ui.heading("Rendering");
                ui.checkbox(&mut state.show_shells, "Show Shells");
                ui.checkbox(&mut state.show_bonds, "Show Bonds");
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
                // Ensure end is always >= start
                if state.lod_shell_fade_end < state.lod_shell_fade_start {
                    state.lod_shell_fade_end = state.lod_shell_fade_start;
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
            });

        // Physics Controls (Bottom Left)
        egui::Window::new("Physics Controls")
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .resizable(false)
            .collapsible(true)
            .default_open(false)
            .show(ctx, |ui| {
                ui.heading("Forces");
                ui.add(
                    egui::Slider::new(&mut state.physics_params.constants[0], 0.0..=1.0e-9)
                        .text("Gravity (G)")
                        .logarithmic(true),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.constants[1], 0.0..=20.0)
                        .text("Electric (K)"),
                );

                ui.separator();
                ui.heading("Strong Force");
                ui.add(
                    egui::Slider::new(&mut state.physics_params.strong_force[0], 0.0..=5.0)
                        .text("Short Range"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.strong_force[1], 0.0..=5.0)
                        .text("Confinement"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.strong_force[2], 0.0..=10.0)
                        .text("Range Cutoff"),
                );

                ui.separator();
                ui.heading("Repulsion");
                ui.add(
                    egui::Slider::new(&mut state.physics_params.repulsion[0], 0.0..=500.0)
                        .text("Core Strength"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.repulsion[1], 0.0..=1.0)
                        .text("Core Radius"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.repulsion[2], 0.001..=0.1)
                        .text("Softening"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.repulsion[3], 10.0..=200.0)
                        .text("Max Force"),
                );

                ui.separator();
                ui.heading("Integration");
                ui.add(
                    egui::Slider::new(&mut state.physics_params.integration[0], 0.0001..=0.01)
                        .text("Time Step (dt)")
                        .logarithmic(true),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.integration[1], 0.9..=1.0)
                        .text("Damping"),
                );

                ui.separator();
                ui.heading("Nucleon Physics");
                ui.add(
                    egui::Slider::new(&mut state.physics_params.nucleon[0], 0.0..=200.0)
                        .text("Binding Strength"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.nucleon[1], 0.1..=10.0)
                        .text("Binding Range"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.nucleon[2], 0.0..=300.0)
                        .text("Exclusion Strength"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.nucleon[3], 0.5..=3.0)
                        .text("Exclusion Radius (x Hadron R)"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.integration[3], 0.0..=100.0)
                        .text("Nucleon Damping"),
                );

                ui.separator();
                ui.heading("Electron Physics");
                ui.add(
                    egui::Slider::new(&mut state.physics_params.electron[0], 0.0..=200.0)
                        .text("Exclusion Strength"),
                );
                ui.add(
                    egui::Slider::new(&mut state.physics_params.electron[1], 0.1..=5.0)
                        .text("Exclusion Radius"),
                );
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
    }
}
