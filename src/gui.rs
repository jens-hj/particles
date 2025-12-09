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
            });

        // Physics Controls (Bottom Left)
        egui::Window::new("Physics Controls")
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .resizable(false)
            .collapsible(true)
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
            });
    }
}
