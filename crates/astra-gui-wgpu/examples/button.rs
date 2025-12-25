//! Interactive button and toggle example
//!
//! Demonstrates button and toggle components with hover and click states.
//!
//! Click buttons to change the counter, use toggle to enable/disable.
//!
//! Controls:
//! - Click +/- buttons to change counter
//! - Click toggle to enable/disable buttons
//! - ESC: quit

use astra_gui::{
    catppuccin::mocha, Content, FullOutput, HorizontalAlign, LayoutDirection, Node, Rect, Size,
    Spacing, TextContent, VerticalAlign,
};
use astra_gui_interactive::{
    button, button_clicked, toggle, toggle_clicked, ButtonStyle, ToggleStyle,
};
use astra_gui_text::Engine as TextEngine;
use astra_gui_wgpu::{EventDispatcher, InputState, InteractiveStateManager, RenderMode, Renderer};
use std::sync::Arc;
use wgpu::Trace;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    text_engine: TextEngine,

    // Input & interaction
    input_state: InputState,
    event_dispatcher: EventDispatcher,
    interactive_state_manager: InteractiveStateManager,

    // Application state
    counter: i32,
    buttons_disabled: bool,
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gpu_state: None,
            text_engine: TextEngine::new_default(),
            input_state: InputState::new(),
            event_dispatcher: EventDispatcher::new(),
            interactive_state_manager: InteractiveStateManager::new(),
            counter: 0,
            buttons_disabled: false,
        }
    }

    fn render(&mut self) {
        let Some(ref window) = self.window else {
            return;
        };

        // Update frame time for transitions
        self.interactive_state_manager.begin_frame();

        // Build UI
        let mut ui = self.build_ui();

        // Compute layout
        let size = window.inner_size();
        let window_rect = Rect::from_min_size([0.0, 0.0], [size.width as f32, size.height as f32]);
        ui.compute_layout_with_measurer(window_rect, &mut self.text_engine);

        // Generate events and interaction states from input
        let (events, interaction_states) = self.event_dispatcher.dispatch(&self.input_state, &ui);

        // Apply interactive styles with transitions
        self.interactive_state_manager
            .apply_styles(&mut ui, &interaction_states);

        // Handle button clicks
        if button_clicked("increment_btn", &events) {
            self.counter += 1;
            println!("Increment clicked! Counter: {}", self.counter);
        }

        if button_clicked("decrement_btn", &events) {
            self.counter -= 1;
            println!("Decrement clicked! Counter: {}", self.counter);
        }

        if toggle_clicked("enable_toggle", &events) {
            self.buttons_disabled = !self.buttons_disabled;
            println!(
                "Toggle clicked! Buttons are now {}",
                if self.buttons_disabled {
                    "disabled"
                } else {
                    "enabled"
                }
            );
        }

        // Render
        let output = FullOutput::from_node_with_debug_and_measurer(
            ui,
            (size.width as f32, size.height as f32),
            None,
            Some(&mut self.text_engine),
        );

        // Get gpu_state after building UI to avoid borrow checker issues
        let Some(ref mut gpu_state) = self.gpu_state else {
            return;
        };

        let frame = gpu_state
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            gpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Clear the screen with a dark background color
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: mocha::BASE.r as f64,
                        g: mocha::BASE.g as f64,
                        b: mocha::BASE.b as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        gpu_state.renderer.render(
            &gpu_state.device,
            &gpu_state.queue,
            &mut encoder,
            &view,
            size.width as f32,
            size.height as f32,
            &output,
        );

        gpu_state.queue.submit(Some(encoder.finish()));
        frame.present();

        // Request redraw if there are active transitions
        if self.interactive_state_manager.has_active_transitions() {
            window.request_redraw();
        }

        // Clear frame-specific input state for next frame
        self.input_state.begin_frame();
    }

    fn build_ui(&self) -> Node {
        Node::new()
            .with_width(Size::Fill)
            .with_height(Size::Fill)
            .with_layout_direction(LayoutDirection::Vertical)
            .with_gap(24.0)
            .with_padding(Spacing::all(48.0))
            .with_child(
                // Title
                Node::new()
                    .with_width(Size::Fill)
                    .with_content(Content::Text(TextContent {
                        text: "Interactive Button Example".to_string(),
                        font_size: 32.0,
                        color: mocha::TEXT,
                        h_align: HorizontalAlign::Center,
                        v_align: VerticalAlign::Center,
                    })),
            )
            .with_child(
                // Counter display
                Node::new()
                    .with_width(Size::Fill)
                    .with_content(Content::Text(TextContent {
                        text: format!("Count: {}", self.counter),
                        font_size: 48.0,
                        color: mocha::LAVENDER,
                        h_align: HorizontalAlign::Center,
                        v_align: VerticalAlign::Center,
                    })),
            )
            .with_child(
                // Centered button container
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(LayoutDirection::Horizontal)
                    .with_gap(16.0)
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    )
                    .with_child(
                        // Decrement Button
                        button(
                            "decrement_btn",
                            "-",
                            self.buttons_disabled,
                            &ButtonStyle::default(),
                        ),
                    )
                    .with_child(
                        // Increment Button
                        button(
                            "increment_btn",
                            "+",
                            self.buttons_disabled,
                            &ButtonStyle::default(),
                        ),
                    )
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ),
            )
            .with_child(
                // Toggle container
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(LayoutDirection::Horizontal)
                    .with_gap(16.0)
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    )
                    .with_child(
                        // Label
                        Node::new()
                            .with_width(Size::FitContent)
                            .with_height(Size::FitContent)
                            .with_content(Content::Text(TextContent {
                                text: "Enable buttons:".to_string(),
                                font_size: 20.0,
                                color: mocha::TEXT,
                                h_align: HorizontalAlign::Center,
                                v_align: VerticalAlign::Center,
                            })),
                    )
                    .with_child(
                        // Toggle Switch
                        toggle(
                            "enable_toggle",
                            !self.buttons_disabled, // Toggle is ON when buttons are enabled
                            false,                  // Toggle itself is never disabled
                            &ToggleStyle::default(),
                        ),
                    )
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ),
            )
            .with_child(
                // Instructions
                Node::new()
                    .with_width(Size::Fill)
                    .with_content(Content::Text(TextContent {
                        text: "Use the toggle switch to enable/disable the counter buttons!"
                            .to_string(),
                        font_size: 16.0,
                        color: mocha::SUBTEXT0,
                        h_align: HorizontalAlign::Center,
                        v_align: VerticalAlign::Center,
                    })),
            )
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Interactive Button Example - Astra GUI")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        self.window = Some(window.clone());

        let gpu_state = pollster::block_on(GpuState::new(window));
        self.gpu_state = Some(gpu_state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key_code {
                winit::keyboard::KeyCode::Escape => {
                    event_loop.exit();
                }
                winit::keyboard::KeyCode::KeyS => {
                    if let Some(gpu_state) = &mut self.gpu_state {
                        let new_mode = match gpu_state.renderer.render_mode() {
                            RenderMode::Sdf | RenderMode::Auto => RenderMode::Mesh,
                            RenderMode::Mesh => RenderMode::Sdf,
                        };
                        gpu_state.renderer.set_render_mode(new_mode);
                        println!("Render mode: {:?}", new_mode);
                    }
                }
                _ => {}
            },
            WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. } => {
                self.input_state.handle_event(&event);
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(ref mut gpu_state) = self.gpu_state {
                    gpu_state.config.width = new_size.width.max(1);
                    gpu_state.config.height = new_size.height.max(1);
                    gpu_state
                        .surface
                        .configure(&gpu_state.device, &gpu_state.config);
                }
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, surface_format);

        Self {
            surface,
            device,
            queue,
            config,
            renderer,
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
