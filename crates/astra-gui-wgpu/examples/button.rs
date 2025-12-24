//! Interactive button example
//!
//! Demonstrates the button component with hover and click states.
//!
//! Click the button to increment the counter!
//!
//! Controls:
//! - Click button to increment counter
//! - ESC: quit

use astra_gui::{
    Color, Content, FullOutput, HorizontalAlign, LayoutDirection, Node, Rect, Size, Spacing,
    TextContent, VerticalAlign,
};
use astra_gui_interactive::{button, button_clicked, button_hovered, ButtonState, ButtonStyle};
use astra_gui_text::Engine as TextEngine;
use astra_gui_wgpu::{EventDispatcher, InputState, Renderer};
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

    // Application state
    counter: i32,
    increment_button_state: ButtonState,
    decrement_button_state: ButtonState,
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
            counter: 0,
            increment_button_state: ButtonState::Idle,
            decrement_button_state: ButtonState::Idle,
        }
    }

    fn render(&mut self) {
        let Some(ref window) = self.window else {
            return;
        };

        // Build UI
        let mut ui = self.build_ui();

        // Compute layout
        let size = window.inner_size();
        let window_rect = Rect::from_min_size([0.0, 0.0], [size.width as f32, size.height as f32]);
        ui.compute_layout_with_measurer(window_rect, &mut self.text_engine);

        // Generate events from input
        let events = self.event_dispatcher.dispatch(&self.input_state, &ui);

        // Update increment button state
        let inc_hovered = button_hovered("increment_btn", &events);
        let inc_pressed = self.input_state.is_button_down(MouseButton::Left) && inc_hovered;
        self.increment_button_state
            .update(inc_hovered, inc_pressed, true);

        // Update decrement button state
        let dec_hovered = button_hovered("decrement_btn", &events);
        let dec_pressed = self.input_state.is_button_down(MouseButton::Left) && dec_hovered;
        self.decrement_button_state
            .update(dec_hovered, dec_pressed, true);

        // Handle button clicks
        if button_clicked("increment_btn", &events) {
            self.counter += 1;
            println!("Increment clicked! Counter: {}", self.counter);
        }

        if button_clicked("decrement_btn", &events) {
            self.counter -= 1;
            println!("Decrement clicked! Counter: {}", self.counter);
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
                        r: 0.1,
                        g: 0.1,
                        b: 0.15,
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
                        color: Color::rgb(1.0, 1.0, 1.0),
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
                        color: Color::rgb(0.8, 0.9, 1.0),
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
                            self.decrement_button_state,
                            &ButtonStyle::default(),
                        ),
                    )
                    .with_child(
                        // Increment Button
                        button(
                            "increment_btn",
                            "+",
                            self.increment_button_state,
                            &ButtonStyle::default(),
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
                        text: "Click + to increment or - to decrement the counter!".to_string(),
                        font_size: 16.0,
                        color: Color::rgb(0.7, 0.7, 0.7),
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
