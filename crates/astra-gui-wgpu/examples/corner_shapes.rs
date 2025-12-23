//! Demonstrates all available corner shapes for rectangles
//!
//! Updated version: uses the `Node` layout system, so debug overlays (margins/padding/borders/content)
//! can be visualized using the existing `DebugOptions` functionality.

use astra_gui::{
    Color, CornerShape, DebugOptions, FullOutput, LayoutDirection, Node, Overflow, Shape, Size,
    Spacing, Stroke, StyledRect,
};
use astra_gui_wgpu::Renderer;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const DEBUG_HELP_TEXT: &str = "Debug controls:
  D - Toggle all debug visualizations
  M - Toggle margins (red)
  P - Toggle padding (blue)
  B - Toggle borders (green)
  C - Toggle content area (yellow)
  ESC - Exit";

fn handle_debug_keybinds(event: &WindowEvent, debug_options: &mut DebugOptions) -> bool {
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
        _ => false,
    }
}

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    debug_options: DebugOptions,
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

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
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
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

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self, debug_options: &DebugOptions) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Clear the screen
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
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
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Render using nodes so debug overlays can be shown.
        let ui_output = create_demo_ui(
            self.config.width as f32,
            self.config.height as f32,
            debug_options,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Astra GUI Encoder"),
            });

        self.renderer.render(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            self.config.width as f32,
            self.config.height as f32,
            &ui_output,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        surface_texture.present();
        Ok(())
    }
}

fn card(fill: Color, corner_shape: CornerShape, stroke_width: f32) -> Shape {
    Shape::Rect(
        StyledRect::new(Default::default(), fill)
            .with_corner_shape(corner_shape)
            .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
    )
}

fn create_demo_ui(width: f32, height: f32, debug_options: &DebugOptions) -> FullOutput {
    // Layout:
    // Root (padding)
    //  - Row 1: 3 equal-width cards
    //  - Row 2: 3 equal-width cards
    //
    // Sizes are chosen to roughly match the old shape-based showcase.
    let root = Node::new()
        .with_padding(Spacing::all(40.0))
        .with_gap(40.0)
        .with_layout_direction(LayoutDirection::Vertical)
        .with_children(vec![
            Node::new()
                .with_height(Size::Fill)
                .with_gap(40.0)
                .with_layout_direction(LayoutDirection::Horizontal)
                .with_overflow(Overflow::Visible)
                .with_children(vec![
                    // None
                    Node::new()
                        .with_width(Size::Fill)
                        .with_padding(Spacing::all(20.0))
                        .with_shape(card(
                            Color::new(0.8, 0.3, 0.3, 1.0),
                            CornerShape::None,
                            20.0,
                        )),
                    // Round
                    Node::new()
                        .with_width(Size::Fill)
                        .with_padding(Spacing::all(20.0))
                        .with_shape(card(
                            Color::new(0.3, 0.8, 0.3, 1.0),
                            CornerShape::Round(50.0),
                            20.0,
                        )),
                    // Cut
                    Node::new()
                        .with_width(Size::Fill)
                        .with_padding(Spacing::all(20.0))
                        .with_shape(card(
                            Color::new(0.3, 0.3, 0.8, 1.0),
                            CornerShape::Cut(50.0),
                            20.0,
                        )),
                ]),
            Node::new()
                .with_height(Size::Fill)
                .with_gap(40.0)
                .with_layout_direction(LayoutDirection::Horizontal)
                .with_overflow(Overflow::Visible)
                .with_children(vec![
                    // InverseRound
                    Node::new()
                        .with_width(Size::Fill)
                        .with_padding(Spacing::all(20.0))
                        .with_shape(card(
                            Color::new(0.8, 0.8, 0.3, 1.0),
                            CornerShape::InverseRound(50.0),
                            20.0,
                        )),
                    // Squircle low smoothness
                    Node::new()
                        .with_width(Size::Fill)
                        .with_padding(Spacing::all(20.0))
                        .with_shape(card(
                            Color::new(0.8, 0.3, 0.8, 1.0),
                            CornerShape::Squircle {
                                radius: 50.0,
                                smoothness: 0.5,
                            },
                            20.0,
                        )),
                    // Squircle high smoothness
                    Node::new()
                        .with_width(Size::Fill)
                        .with_padding(Spacing::all(20.0))
                        .with_shape(card(
                            Color::new(0.3, 0.8, 0.8, 1.0),
                            CornerShape::Squircle {
                                radius: 50.0,
                                smoothness: 3.0,
                            },
                            20.0,
                        )),
                ]),
        ]);

    FullOutput::from_node_with_debug(
        root,
        (width, height),
        if debug_options.is_enabled() {
            Some(*debug_options)
        } else {
            None
        },
    )
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Astra GUI - Corner Shapes Demo (Nodes)")
                .with_inner_size(winit::dpi::LogicalSize::new(1400, 900));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            self.gpu_state = Some(pollster::block_on(GpuState::new(window)));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key:
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),

            WindowEvent::KeyboardInput { .. } => {
                // Debug controls (D/M/P/B/C).
                let _handled = handle_debug_keybinds(&event, &mut self.debug_options);
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    match gpu_state.render(&self.debug_options) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.window {
                                gpu_state.resize(window.inner_size())
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
            }

            _ => {}
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        gpu_state: None,
        debug_options: DebugOptions::none(),
    };

    println!("{}", DEBUG_HELP_TEXT);

    event_loop.run_app(&mut app).unwrap();
}
