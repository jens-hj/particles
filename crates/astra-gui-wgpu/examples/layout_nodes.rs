///! Demonstrates the node-based layout system with nested elements
use astra_gui::{
    Color, CornerShape, DebugOptions, FullOutput, LayoutDirection, Node, Offset, Shape, Size,
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
        let output = self.surface.get_current_texture()?;
        let view = output
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
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
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

        // Create UI node tree and render (with debug visualization)
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

        output.present();
        Ok(())
    }
}

fn create_demo_ui(width: f32, height: f32, debug_options: &DebugOptions) -> FullOutput {
    // Root container - full window with padding
    let root = Node::new()
        .with_padding(Spacing::all(20.0))
        .with_gap(25.0)
        .with_layout_direction(LayoutDirection::Vertical)
        .with_shape(Shape::Rect(
            StyledRect::new(Default::default(), Color::transparent())
                .with_corner_shape(CornerShape::Round(25.0))
                .with_stroke(Stroke::new(2.0, Color::new(1.0, 0.0, 0.0, 0.0))),
        ))
        .with_children(vec![
            // Header
            Node::new()
                .with_height(Size::fraction(0.15))
                .with_shape(Shape::Rect(
                    StyledRect::new(Default::default(), Color::new(0.2, 0.3, 0.5, 1.0))
                        .with_corner_shape(CornerShape::Round(50.0))
                        .with_stroke(Stroke::new(2.0, Color::new(0.4, 0.5, 0.7, 1.0))),
                )),
            // Main content area - horizontal layout
            Node::new()
                .with_height(Size::fraction(0.75))
                .with_gap(25.0)
                .with_layout_direction(LayoutDirection::Horizontal)
                .with_children(vec![
                    // Left sidebar - 25% width
                    Node::new()
                        .with_width(Size::fraction(0.25))
                        .with_padding(Spacing::all(10.0))
                        .with_gap(10.0)
                        .with_shape(Shape::Rect(
                            StyledRect::new(Default::default(), Color::new(0.3, 0.2, 0.4, 1.0))
                                .with_corner_shape(CornerShape::Round(50.0)),
                        ))
                        .with_layout_direction(LayoutDirection::Vertical)
                        .with_children(vec![
                            // Sidebar items
                            Node::new()
                                .with_height(Size::px(150.0))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.5, 0.3, 0.6, 1.0),
                                    )
                                    .with_corner_shape(CornerShape::Round(40.0)),
                                )),
                            Node::new()
                                .with_height(Size::px(150.0))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.5, 0.3, 0.6, 1.0),
                                    )
                                    .with_corner_shape(CornerShape::Round(40.0)),
                                )),
                            Node::new()
                                .with_height(Size::px(150.0))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.5, 0.3, 0.6, 1.0),
                                    )
                                    .with_corner_shape(CornerShape::Round(40.0)),
                                )),
                            Node::new()
                                .with_height(Size::px(150.0))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.5, 0.3, 0.6, 1.0),
                                    )
                                    .with_corner_shape(CornerShape::Round(40.0)),
                                )),
                            Node::new()
                                .with_height(Size::px(150.0))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.5, 0.3, 0.6, 1.0),
                                    )
                                    .with_corner_shape(CornerShape::Round(40.0)),
                                )),
                        ]),
                    // Right of sidebar
                    Node::new()
                        .with_width(Size::fraction(0.75))
                        .with_padding(Spacing::all(25.0))
                        .with_gap(25.0)
                        .with_shape(Shape::Rect(
                            StyledRect::new(Default::default(), Color::new(0.15, 0.15, 0.2, 1.0))
                                .with_corner_shape(CornerShape::Round(50.0))
                                .with_stroke(Stroke::new(2.0, Color::new(0.3, 0.3, 0.4, 1.0))),
                        ))
                        .with_layout_direction(LayoutDirection::Vertical)
                        .with_children(vec![
                            // Content cards in vertical layout
                            Node::new()
                                .with_height(Size::fraction(0.3))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.3, 0.5, 0.3, 1.0),
                                    )
                                    .with_corner_shape(CornerShape::Round(25.0)),
                                )),
                            // Horizontal row of smaller cards
                            Node::new()
                                .with_height(Size::fraction(0.3))
                                .with_gap(25.0)
                                .with_layout_direction(LayoutDirection::Horizontal)
                                .with_children(vec![
                                    Node::new().with_width(Size::fraction(0.5)).with_shape(
                                        Shape::Rect(
                                            StyledRect::new(
                                                Default::default(),
                                                Color::new(0.5, 0.3, 0.3, 1.0),
                                            )
                                            .with_corner_shape(CornerShape::Cut(25.0))
                                            .with_stroke(Stroke::new(
                                                5.0,
                                                Color::new(0.4, 0.2, 0.2, 1.0),
                                            )),
                                        ),
                                    ),
                                    Node::new().with_width(Size::fraction(0.5)).with_shape(
                                        Shape::Rect(
                                            StyledRect::new(
                                                Default::default(),
                                                Color::new(0.3, 0.3, 0.5, 1.0),
                                            )
                                            .with_corner_shape(CornerShape::Cut(25.0))
                                            .with_stroke(Stroke::new(
                                                5.0,
                                                Color::new(0.2, 0.2, 0.4, 1.0),
                                            )),
                                        ),
                                    ),
                                ]),
                            Node::new()
                                .with_height(Size::fraction(0.4))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(
                                        Default::default(),
                                        Color::new(0.5, 0.5, 0.3, 1.0),
                                    )
                                    .with_corner_shape(
                                        CornerShape::Squircle {
                                            radius: 50.0,
                                            smoothness: 1.0,
                                        },
                                    ),
                                )),
                        ]),
                ]),
            // Footer - 10% height with three Fill children laid out horizontally with gap
            Node::new()
                .with_height(Size::fraction(0.1))
                .with_offset(Offset::new(0.0, 0.0))
                .with_padding(Spacing::all(10.0))
                .with_gap(10.0)
                .with_layout_direction(LayoutDirection::Horizontal)
                .with_shape(Shape::Rect(
                    StyledRect::new(Default::default(), Color::new(0.2, 0.2, 0.25, 1.0))
                        .with_corner_shape(CornerShape::Round(50.0)),
                ))
                .with_children(vec![
                    Node::new().with_width(Size::Fill).with_shape(Shape::Rect(
                        StyledRect::new(Default::default(), Color::new(0.4, 0.3, 0.5, 1.0))
                            .with_corner_shape(CornerShape::Round(25.0)),
                    )),
                    Node::new().with_width(Size::Fill).with_shape(Shape::Rect(
                        StyledRect::new(Default::default(), Color::new(0.3, 0.4, 0.5, 1.0))
                            .with_corner_shape(CornerShape::Round(25.0)),
                    )),
                    Node::new().with_width(Size::Fill).with_shape(Shape::Rect(
                        StyledRect::new(Default::default(), Color::new(0.5, 0.4, 0.3, 1.0))
                            .with_corner_shape(CornerShape::Round(25.0)),
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
                .with_title("Astra GUI - Layout Nodes Demo")
                .with_inner_size(winit::dpi::LogicalSize::new(1200, 800));

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

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key_code {
                winit::keyboard::KeyCode::KeyM => {
                    self.debug_options.show_margins = !self.debug_options.show_margins;
                    println!("Margins: {}", self.debug_options.show_margins);
                }
                winit::keyboard::KeyCode::KeyP => {
                    self.debug_options.show_padding = !self.debug_options.show_padding;
                    println!("Padding: {}", self.debug_options.show_padding);
                }
                winit::keyboard::KeyCode::KeyB => {
                    self.debug_options.show_borders = !self.debug_options.show_borders;
                    println!("Borders: {}", self.debug_options.show_borders);
                }
                winit::keyboard::KeyCode::KeyC => {
                    self.debug_options.show_content_area = !self.debug_options.show_content_area;
                    println!("Content area: {}", self.debug_options.show_content_area);
                }
                winit::keyboard::KeyCode::KeyD => {
                    if self.debug_options.is_enabled() {
                        self.debug_options = DebugOptions::none();
                        println!("Debug: OFF");
                    } else {
                        self.debug_options = DebugOptions::all();
                        println!("Debug: ALL ON");
                    }
                }
                _ => {}
            },

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
        debug_options: DebugOptions::none(), // Start with debug off
    };

    println!("Debug controls:");
    println!("  D - Toggle all debug visualizations");
    println!("  M - Toggle margins (red)");
    println!("  P - Toggle padding (blue)");
    println!("  B - Toggle borders (green)");
    println!("  C - Toggle content area (yellow)");
    println!("  ESC - Exit");

    event_loop.run_app(&mut app).unwrap();
}
