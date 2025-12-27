//! Interactive components example
//!
//! Demonstrates button, toggle, and slider components with hover and click states.
//!
//! Controls:
//! - Click +/- buttons to change counter
//! - Click toggle to enable/disable buttons
//! - Drag slider to adjust value
//! - ESC: quit

use astra_gui::{
    catppuccin::mocha, Content, DebugOptions, FullOutput, HorizontalAlign, Layout, Node, Rect,
    Shape, Size, Spacing, StyledRect, TextContent, VerticalAlign,
};
use astra_gui_interactive::{
    button, button_clicked, slider, slider_drag, text_input, text_input_update, toggle,
    toggle_clicked, ButtonStyle, CursorShape, CursorStyle, SliderStyle, TextInputStyle,
    ToggleStyle,
};
use astra_gui_text::Engine as TextEngine;
use astra_gui_wgpu::{
    EventDispatcher, InputState, InteractiveStateManager, Key, NamedKey, RenderMode, Renderer,
};

const DEBUG_HELP_TEXT: &str = "Debug controls:
  M - Toggle margins (red overlay)
  P - Toggle padding (blue overlay)
  B - Toggle borders (green outline)
  C - Toggle content area (yellow outline)
  R - Toggle clip rects (red outline)
  G - Toggle gaps (purple overlay)
  D - Toggle all debug visualizations
  S - Toggle render mode (SDF/Mesh)
  ESC - Exit";

const DEBUG_HELP_TEXT_ONELINE: &str = "M:Margins | P:Padding | B:Borders | C:Content | R:ClipRects | G:Gaps | D:All | S:RenderMode | ESC:Exit";

fn handle_debug_keybinds(
    event: &WindowEvent,
    debug_options: &mut DebugOptions,
    renderer: Option<&mut Renderer>,
) -> bool {
    let WindowEvent::KeyboardInput {
        event:
            KeyEvent {
                physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                state: ElementState::Pressed,
                ..
            },
        ..
    } = event
    else {
        return false;
    };

    match *key_code {
        winit::keyboard::KeyCode::KeyM => {
            debug_options.show_margins = !debug_options.show_margins;
            println!("Margins: {}", debug_options.show_margins);
            true
        }
        winit::keyboard::KeyCode::KeyP => {
            debug_options.show_padding = !debug_options.show_padding;
            println!("Padding: {}", debug_options.show_padding);
            true
        }
        winit::keyboard::KeyCode::KeyB => {
            debug_options.show_borders = !debug_options.show_borders;
            println!("Borders: {}", debug_options.show_borders);
            true
        }
        winit::keyboard::KeyCode::KeyC => {
            debug_options.show_content_area = !debug_options.show_content_area;
            println!("Content area: {}", debug_options.show_content_area);
            true
        }
        winit::keyboard::KeyCode::KeyR => {
            debug_options.show_clip_rects = !debug_options.show_clip_rects;
            println!("Clip rects: {}", debug_options.show_clip_rects);
            true
        }
        winit::keyboard::KeyCode::KeyG => {
            debug_options.show_gaps = !debug_options.show_gaps;
            println!("Gaps: {}", debug_options.show_gaps);
            true
        }
        winit::keyboard::KeyCode::KeyD => {
            if debug_options.is_enabled() {
                *debug_options = DebugOptions::none();
                println!("Debug: OFF");
            } else {
                *debug_options = DebugOptions::all();
                println!("Debug: ALL ON");
            }
            true
        }
        winit::keyboard::KeyCode::KeyS => {
            if let Some(renderer) = renderer {
                let new_mode = match renderer.render_mode() {
                    RenderMode::Sdf | RenderMode::Auto => RenderMode::Mesh,
                    RenderMode::Mesh => RenderMode::Sdf,
                };
                renderer.set_render_mode(new_mode);
                println!("Render mode: {:?}", new_mode);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

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
    nodes_disabled: bool,
    slider_value: f32,
    continuous_slider_value: f32,
    text_input_value: String,
    text_input_cursor: usize,
    text_input_selection: Option<(usize, usize)>,
    debug_options: DebugOptions,
    last_frame_time: std::time::Instant,
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
            nodes_disabled: false,
            slider_value: 7.0,
            continuous_slider_value: 50.0,
            text_input_value: String::new(),
            text_input_cursor: 0,
            text_input_selection: None,
            debug_options: DebugOptions::none(),
            last_frame_time: std::time::Instant::now(),
        }
    }

    fn render(&mut self) {
        // Calculate delta time
        let now = std::time::Instant::now();
        let _delta_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Update frame time for transitions
        self.interactive_state_manager.begin_frame();

        // Build UI
        let mut ui = self.build_ui();

        // Get window size
        let size = match &self.window {
            Some(window) => window.inner_size(),
            None => return,
        };
        let window_rect = Rect::from_min_size([0.0, 0.0], [size.width as f32, size.height as f32]);
        ui.compute_layout_with_measurer(window_rect, &mut self.text_engine);

        // Generate events and interaction states from input
        // (auto-IDs are assigned automatically inside dispatch)
        let (events, interaction_states) =
            self.event_dispatcher.dispatch(&self.input_state, &mut ui);

        // Apply interactive styles with transitions
        self.interactive_state_manager
            .apply_styles(&mut ui, &interaction_states);

        // Update text input (handles focus, unfocus, and keyboard input automatically)
        if text_input_update(
            "text_input",
            &mut self.text_input_value,
            &mut self.text_input_cursor,
            &mut self.text_input_selection,
            &events,
            &self.input_state,
            &mut self.event_dispatcher,
        ) {
            println!("Text input value: {}", self.text_input_value);
        }

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
            self.nodes_disabled = !self.nodes_disabled;
            println!(
                "Toggle clicked! Buttons are now {}",
                if self.nodes_disabled {
                    "disabled"
                } else {
                    "enabled"
                }
            );
        }

        // Handle stepped slider drag
        if slider_drag(
            "stepped_slider",
            &mut self.slider_value,
            &(0.0..=30.0),
            &events,
            &SliderStyle::default(),
            Some(7.0), // Step by 5.0
        ) {
            println!("Stepped slider value: {:.1}", self.slider_value);
        }

        // Handle continuous slider drag
        if slider_drag(
            "continuous_slider",
            &mut self.continuous_slider_value,
            &(0.0..=100.0),
            &events,
            &SliderStyle::default(),
            None, // No stepping - continuous
        ) {
            println!(
                "Continuous slider value: {:.1}",
                self.continuous_slider_value
            );
        }

        // Render
        let output = FullOutput::from_node_with_debug_and_measurer(
            ui,
            (size.width as f32, size.height as f32),
            if self.debug_options.is_enabled() {
                Some(self.debug_options)
            } else {
                None
            },
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
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }

        // Clear frame-specific input state for next frame
        self.input_state.begin_frame();
    }

    fn build_ui(&mut self) -> Node {
        Node::new()
            .with_width(Size::Fill)
            .with_height(Size::Fill)
            .with_layout_direction(Layout::Vertical)
            .with_gap(24.0)
            .with_children(vec![
                // Spacer
                Node::new().with_height(Size::Fill),
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
                // Centered button container
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(Layout::Horizontal)
                    .with_gap(16.0)
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    )
                    .with_children(vec![
                        // Decrement Button
                        button(
                            "decrement_btn",
                            "-",
                            self.nodes_disabled,
                            &ButtonStyle::default(),
                        ),
                        // Increment Button
                        button(
                            "increment_btn",
                            "+",
                            self.nodes_disabled,
                            &ButtonStyle::default(),
                        ),
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ]),
                // Toggle container
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(Layout::Horizontal)
                    .with_gap(16.0)
                    .with_children(vec![
                        // Spacer
                        Node::new().with_width(Size::Fill),
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
                        // Toggle Switch
                        toggle(
                            "enable_toggle",
                            !self.nodes_disabled, // Toggle is ON when buttons are enabled
                            false,                // Toggle itself is never disabled
                            &ToggleStyle::default(),
                        ),
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ]),
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
                // Text input section
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(Layout::Horizontal)
                    .with_gap(16.0)
                    .with_children(vec![
                        // Spacer
                        Node::new().with_width(Size::Fill),
                        // Text Input
                        text_input(
                            "text_input",
                            &self.text_input_value,
                            "Type something...",
                            self.event_dispatcher
                                .focused_node()
                                .map(|id| id.as_str() == "text_input")
                                .unwrap_or(false),
                            self.nodes_disabled,
                            &TextInputStyle {
                                cursor_style: CursorStyle {
                                    shape: CursorShape::Underline,
                                    thickness: 3.0,
                                    ..CursorStyle::default()
                                },
                                ..TextInputStyle::default()
                            },
                            self.text_input_cursor,
                            self.text_input_selection,
                            &mut self.text_engine,
                            &mut self.event_dispatcher,
                        ),
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ]),
                // Stepped slider section
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(Layout::Horizontal)
                    .with_gap(16.0)
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    )
                    .with_children(vec![
                        // Label
                        Node::new()
                            .with_width(Size::px(150.0))
                            .with_height(Size::FitContent)
                            .with_content(Content::Text(TextContent {
                                text: "Stepped (7):".to_string(),
                                font_size: 20.0,
                                color: mocha::TEXT,
                                h_align: HorizontalAlign::Right,
                                v_align: VerticalAlign::Center,
                            })),
                        // Slider
                        slider(
                            "stepped_slider",
                            self.slider_value,
                            0.0..=30.0,
                            self.nodes_disabled,
                            &SliderStyle::default(),
                        ),
                        // Value display
                        Node::new()
                            .with_width(Size::px(55.0))
                            .with_height(Size::FitContent)
                            .with_content(Content::Text(TextContent {
                                text: format!("{:.0}", self.slider_value),
                                font_size: 20.0,
                                color: mocha::LAVENDER,
                                h_align: HorizontalAlign::Right,
                                v_align: VerticalAlign::Center,
                            })),
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ]),
                // Continuous slider section
                Node::new()
                    .with_width(Size::Fill)
                    .with_layout_direction(Layout::Horizontal)
                    .with_gap(16.0)
                    .with_child(
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    )
                    .with_children(vec![
                        // Label
                        Node::new()
                            .with_width(Size::px(150.0))
                            .with_height(Size::FitContent)
                            .with_content(Content::Text(TextContent {
                                text: "Continuous:".to_string(),
                                font_size: 20.0,
                                color: mocha::TEXT,
                                h_align: HorizontalAlign::Right,
                                v_align: VerticalAlign::Center,
                            })),
                        // Slider
                        slider(
                            "continuous_slider",
                            self.continuous_slider_value,
                            0.0..=100.0,
                            self.nodes_disabled,
                            &SliderStyle::default(),
                        ),
                        // Value display
                        Node::new()
                            .with_width(Size::px(55.0))
                            .with_height(Size::FitContent)
                            .with_content(Content::Text(TextContent {
                                text: format!("{:.2}", self.continuous_slider_value),
                                font_size: 20.0,
                                color: mocha::LAVENDER,
                                h_align: HorizontalAlign::Right,
                                v_align: VerticalAlign::Center,
                            })),
                        // Spacer
                        Node::new().with_width(Size::Fill),
                    ]),
                // Spacer
                Node::new().with_height(Size::Fill),
                // Help bar
                Node::new()
                    .with_width(Size::Fill)
                    .with_height(Size::px(30.0))
                    .with_padding(Spacing::horizontal(10.0))
                    .with_shape(Shape::Rect(StyledRect::new(
                        Default::default(),
                        mocha::SURFACE0,
                    )))
                    .with_content(Content::Text(
                        TextContent::new(DEBUG_HELP_TEXT_ONELINE)
                            .with_font_size(16.0)
                            .with_color(mocha::TEXT)
                            .with_h_align(HorizontalAlign::Left)
                            .with_v_align(VerticalAlign::Center),
                    )),
            ])
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Interactive Components - Astra GUI")
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
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event: ref key_event,
                ..
            } if matches!(
                key_event.physical_key,
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
            ) && key_event.state == ElementState::Pressed =>
            {
                // Only exit on ESC if nothing is focused
                if self.event_dispatcher.focused_node().is_none() {
                    event_loop.exit();
                } else {
                    // Pass to keyboard handler to unfocus
                    self.input_state.handle_event(&event);
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::KeyboardInput {
                event: ref key_event,
                ..
            } if !matches!(
                key_event.physical_key,
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
            ) =>
            {
                // First, pass keyboard events to input state
                self.input_state.handle_event(&event);

                // Check if ESC was just pressed to unfocus
                let escape_pressed = self
                    .input_state
                    .keys_just_pressed
                    .iter()
                    .any(|key| matches!(key, Key::Named(NamedKey::Escape)));

                // Only handle debug shortcuts if something is focused and ESC was pressed
                // In that case, ESC should unfocus instead of closing the app
                let has_focus = self.event_dispatcher.focused_node().is_some();
                let should_handle_debug = !(has_focus && escape_pressed);

                if should_handle_debug {
                    let renderer = self.gpu_state.as_mut().map(|s| &mut s.renderer);
                    let _handled = handle_debug_keybinds(&event, &mut self.debug_options, renderer);
                }

                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
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

        if let Some(window) = &self.window {
            window.request_redraw();
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

    println!("{}", DEBUG_HELP_TEXT);

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
