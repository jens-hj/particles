///! Demonstrates the node-based layout system with nested elements
use astra_gui::{
    catppuccin::mocha, Content, CornerShape, DebugOptions, FullOutput, HorizontalAlign, Layout,
    Node, Offset, Shape, Size, Spacing, Stroke, StyledRect, TextContent, VerticalAlign,
};
use astra_gui_wgpu::{RenderMode, Renderer};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
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

fn child() -> Node {
    return Node::new().with_height(Size::Fill).with_shape(Shape::Rect(
        StyledRect::new(Default::default(), mocha::SURFACE1)
            .with_corner_shape(CornerShape::Round(30.0))
            .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
    ));
}

fn create_demo_ui(width: f32, height: f32, debug_options: &DebugOptions) -> FullOutput {
    // Root container - full window with padding
    let root = Node::new()
        .with_padding(Spacing::all(20.0))
        .with_gap(25.0)
        .with_layout_direction(Layout::Vertical)
        .with_children(vec![
            // Header
            Node::new()
                .with_layout_direction(Layout::Horizontal)
                .with_gap(10.0)
                .with_height(Size::fraction(0.15))
                .with_shape(Shape::Rect(
                    StyledRect::new(Default::default(), mocha::SURFACE0)
                        .with_corner_shape(CornerShape::Round(50.0))
                        .with_stroke(Stroke::new(3.0, mocha::BLUE)),
                ))
                .with_padding(Spacing::all(20.0))
                .with_children(vec![
                    Node::new()
                        .with_width(Size::Relative(0.7))
                        .with_shape(Shape::Rect(
                            StyledRect::new(Default::default(), mocha::SURFACE1)
                                .with_corner_shape(CornerShape::Round(30.0))
                                .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                        )),
                    Node::new()
                        .with_width(Size::Fill)
                        .with_layout_direction(Layout::Vertical)
                        .with_gap(10.0)
                        .with_children(vec![
                            Node::new()
                                .with_shape(Shape::Rect(
                                    StyledRect::new(Default::default(), mocha::SURFACE1)
                                        .with_corner_shape(CornerShape::Round(30.0))
                                        .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                                ))
                                .with_height(Size::Fill),
                            Node::new()
                                .with_shape(Shape::Rect(
                                    StyledRect::new(Default::default(), mocha::SURFACE1)
                                        .with_corner_shape(CornerShape::Round(30.0))
                                        .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                                ))
                                .with_height(Size::Fill),
                        ]),
                ]),
            // Main content area - horizontal layout
            Node::new()
                .with_height(Size::fraction(0.75))
                .with_gap(25.0)
                .with_layout_direction(Layout::Horizontal)
                .with_children(vec![
                    // Left sidebar - 25% width
                    Node::new()
                        .with_width(Size::fraction(0.25))
                        .with_padding(Spacing::all(20.0))
                        .with_gap(10.0)
                        .with_shape(Shape::Rect(
                            StyledRect::new(Default::default(), mocha::SURFACE0)
                                .with_corner_shape(CornerShape::Round(50.0))
                                .with_stroke(Stroke::new(3.0, mocha::MAUVE)),
                        ))
                        .with_layout_direction(Layout::Vertical)
                        .with_children(vec![
                            // Sidebar items
                            child(),
                            child(),
                            child(),
                            child(),
                            child(),
                            child(),
                            child(),
                            child(),
                            child(),
                            child(),
                        ]),
                    // Right of sidebar
                    Node::new()
                        .with_width(Size::fraction(0.75))
                        .with_padding(Spacing::all(20.0))
                        .with_gap(20.0)
                        .with_shape(Shape::Rect(
                            StyledRect::new(Default::default(), mocha::SURFACE0)
                                .with_corner_shape(CornerShape::Round(50.0))
                                .with_stroke(Stroke::new(3.0, mocha::PEACH)),
                        ))
                        .with_layout_direction(Layout::Vertical)
                        .with_children(vec![
                            // Content cards in vertical layout
                            Node::new()
                                .with_height(Size::fraction(0.3))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(Default::default(), mocha::SURFACE1)
                                        .with_corner_shape(CornerShape::Round(30.0))
                                        .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                                )),
                            // Horizontal row of smaller cards
                            Node::new()
                                .with_height(Size::fraction(0.3))
                                .with_gap(20.0)
                                .with_layout_direction(Layout::Horizontal)
                                .with_children(vec![
                                    Node::new().with_width(Size::fraction(0.5)).with_shape(
                                        Shape::Rect(
                                            StyledRect::new(Default::default(), mocha::SURFACE1)
                                                .with_corner_shape(CornerShape::Cut(30.0))
                                                .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                                        ),
                                    ),
                                    Node::new().with_width(Size::fraction(0.5)).with_shape(
                                        Shape::Rect(
                                            StyledRect::new(Default::default(), mocha::SURFACE1)
                                                .with_corner_shape(CornerShape::Cut(30.0))
                                                .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                                        ),
                                    ),
                                ]),
                            Node::new()
                                .with_height(Size::fraction(0.4))
                                .with_shape(Shape::Rect(
                                    StyledRect::new(Default::default(), mocha::SURFACE1)
                                        .with_corner_shape(CornerShape::Round(30.0))
                                        .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                                )),
                        ]),
                ]),
            // Footer - 10% height with three Fill children laid out horizontally with gap
            Node::new()
                .with_height(Size::fraction(0.1))
                .with_offset(Offset::new(0.0, 0.0))
                .with_padding(Spacing::all(20.0))
                .with_gap(20.0)
                .with_layout_direction(Layout::Horizontal)
                .with_shape(Shape::Rect(
                    StyledRect::new(Default::default(), mocha::SURFACE0)
                        .with_corner_shape(CornerShape::Round(50.0))
                        .with_stroke(Stroke::new(3.0, mocha::BLUE)),
                ))
                .with_children(vec![
                    Node::new().with_width(Size::Fill).with_shape(Shape::Rect(
                        StyledRect::new(Default::default(), mocha::SURFACE1)
                            .with_corner_shape(CornerShape::Round(30.0))
                            .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                    )),
                    Node::new().with_width(Size::Fill).with_shape(Shape::Rect(
                        StyledRect::new(Default::default(), mocha::SURFACE1)
                            .with_corner_shape(CornerShape::Round(30.0))
                            .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                    )),
                    Node::new().with_width(Size::Fill).with_shape(Shape::Rect(
                        StyledRect::new(Default::default(), mocha::SURFACE1)
                            .with_corner_shape(CornerShape::Round(30.0))
                            .with_stroke(Stroke::new(3.0, mocha::SURFACE2)),
                    )),
                ]),
        ]);

    // Create help bar at the bottom
    let help_text = Node::new()
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
        ));

    let final_root = Node::new()
        .with_width(Size::Fill)
        .with_height(Size::Fill)
        .with_layout_direction(Layout::Vertical)
        .with_children(vec![root, help_text]);

    FullOutput::from_node_with_debug(
        final_root,
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

            WindowEvent::KeyboardInput { .. } => {
                // Debug controls (D/M/P/B/C/S).
                let renderer = self.gpu_state.as_mut().map(|s| &mut s.renderer);
                let _handled = handle_debug_keybinds(&event, &mut self.debug_options, renderer);
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
        debug_options: DebugOptions::none(), // Start with debug off
    };

    println!("{}", DEBUG_HELP_TEXT);

    event_loop.run_app(&mut app).unwrap();
}
