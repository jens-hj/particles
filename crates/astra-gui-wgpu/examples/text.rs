//! Demonstrates rendering text nodes.
//!
//! This example exercises the `astra-gui-wgpu` backend's `Shape::Text` rendering path,
//! including alignment, padding/content rect behavior, and scissor-based clipping.

use astra_gui::{
    catppuccin::mocha, Color, Content, CornerShape, DebugOptions, FullOutput, HorizontalAlign,
    Layout, Node, Offset, Overflow, Rect, Shape, Size, Spacing, Stroke, StyledRect, TextContent,
    VerticalAlign,
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

        // Clear pass
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Clear Encoder"),
                });

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
        }

        // UI pass
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

fn label(
    text: impl Into<String>,
    font_size: f32,
    color: Color,
    h: HorizontalAlign,
    v: VerticalAlign,
) -> Node {
    Node::new().with_content(Content::Text(
        TextContent::new(text)
            .with_font_size(font_size)
            .with_color(color)
            .with_h_align(h)
            .with_v_align(v),
    ))
}

fn panel(fill: Color) -> Shape {
    Shape::Rect(
        StyledRect::new(Default::default(), fill)
            .with_corner_shape(CornerShape::Round(18.0))
            .with_stroke(Stroke::new(2.0, mocha::SURFACE1)),
    )
}

fn create_demo_ui(width: f32, height: f32, debug_options: &DebugOptions) -> FullOutput {
    let _window_rect = Rect::new([0.0, 0.0], [width, height]);

    // Root: whole window, a little padding
    let root = Node::new()
        .with_padding(Spacing::all(24.0))
        .with_gap(18.0)
        .with_layout_direction(Layout::Vertical)
        .with_shape(Shape::Rect(
            StyledRect::new(Default::default(), Color::transparent())
                .with_corner_shape(CornerShape::Round(24.0))
                .with_stroke(Stroke::new(2.0, mocha::SURFACE0)),
        ))
        .with_children(vec![
            // Header
            Node::new()
                .with_height(Size::px(110.0))
                .with_padding(Spacing::all(18.0))
                .with_shape(panel(mocha::SURFACE0))
                .with_children(vec![
                    // Title: large, left/top aligned
                    label(
                        "astra-gui: text nodes",
                        34.0,
                        mocha::TEXT,
                        HorizontalAlign::Left,
                        VerticalAlign::Top,
                    )
                    .with_height(Size::Fill),
                    // Subtitle: smaller
                    label(
                        "alignment, padding content rects, and clipping (when implemented)",
                        16.0,
                        mocha::SUBTEXT0,
                        HorizontalAlign::Left,
                        VerticalAlign::Bottom,
                    )
                    .with_height(Size::Fill),
                ]),
            // Main area: 2 columns
            Node::new()
                .with_height(Size::Fill)
                .with_gap(18.0)
                .with_layout_direction(Layout::Horizontal)
                .with_children(vec![
                    // Left: alignment grid
                    Node::new()
                        .with_width(Size::fraction(0.55))
                        .with_padding(Spacing::all(16.0))
                        .with_gap(12.0)
                        .with_shape(panel(mocha::MANTLE))
                        .with_layout_direction(Layout::Vertical)
                        .with_children(vec![
                            label(
                                "Alignment grid (L/C/R × T/C/B)",
                                18.0,
                                mocha::TEXT,
                                HorizontalAlign::Left,
                                VerticalAlign::Top,
                            )
                            .with_height(Size::px(24.0)),
                            Node::new()
                                .with_height(Size::Fill)
                                .with_gap(12.0)
                                .with_layout_direction(Layout::Vertical)
                                .with_children(vec![
                                    alignment_row("Top", VerticalAlign::Top),
                                    alignment_row("Center", VerticalAlign::Center),
                                    alignment_row("Bottom", VerticalAlign::Bottom),
                                ]),
                        ]),
                    // Right: varied sizes and clipping candidate
                    Node::new()
                        .with_width(Size::fraction(0.45))
                        .with_padding(Spacing::all(16.0))
                        .with_gap(14.0)
                        .with_shape(panel(mocha::MANTLE))
                        .with_layout_direction(Layout::Vertical)
                        .with_children(vec![
                            label(
                                "Font sizes + padding behavior",
                                18.0,
                                mocha::TEXT,
                                HorizontalAlign::Left,
                                VerticalAlign::Top,
                            )
                            .with_height(Size::px(24.0)),
                            Node::new()
                                .with_height(Size::px(70.0))
                                .with_padding(Spacing::all(10.0))
                                .with_shape(panel(mocha::SURFACE0))
                                .with_children(vec![label(
                                    "Small (14px) in padded panel",
                                    14.0,
                                    mocha::SKY,
                                    HorizontalAlign::Left,
                                    VerticalAlign::Top,
                                )
                                .with_height(Size::Fill)]),
                            Node::new()
                                .with_height(Size::px(90.0))
                                .with_padding(Spacing::all(10.0))
                                .with_shape(panel(mocha::SURFACE0))
                                .with_children(vec![label(
                                    "Medium (22px)",
                                    22.0,
                                    mocha::PEACH,
                                    HorizontalAlign::Left,
                                    VerticalAlign::Center,
                                )
                                .with_height(Size::Fill)]),
                            Node::new()
                                .with_height(Size::px(120.0))
                                .with_padding(Spacing::all(10.0))
                                .with_shape(panel(mocha::SURFACE0))
                                .with_children(vec![label(
                                    "Large (42px)",
                                    42.0,
                                    mocha::MAUVE,
                                    HorizontalAlign::Left,
                                    VerticalAlign::Bottom,
                                )
                                .with_height(Size::Fill)]),
                            // Clipping candidate: a tight box with long text.
                            Node::new()
                                .with_height(Size::px(80.0))
                                .with_padding(Spacing::all(10.0))
                                .with_overflow(Overflow::Hidden)
                                .with_shape(Shape::Rect(
                                    StyledRect::new(Default::default(), mocha::CRUST)
                                        .with_corner_shape(CornerShape::Round(14.0))
                                        .with_stroke(Stroke::new(2.0, mocha::SURFACE0)),
                                ))
                                // Optional offset to demonstrate bounds interactions.
                                .with_offset(Offset::new(0.0, 0.0))
                                .with_children(vec![label(
                                    "This string is intentionally very long to demonstrate clipping/scissoring. So let's make this even longer to make sure it clips.",
                                    18.0,
                                    mocha::RED,
                                    HorizontalAlign::Left,
                                    VerticalAlign::Top,
                                )
                                .with_height(Size::Fill)]),
                        ]),
                ]),
            // Help bar
            Node::new()
                .with_height(Size::px(30.0))
                .with_padding(Spacing::horizontal(10.0))
                .with_shape(panel(mocha::SURFACE0))
                .with_content(Content::Text(
                    TextContent::new(DEBUG_HELP_TEXT_ONELINE)
                        .with_font_size(16.0)
                        .with_color(mocha::TEXT)
                        .with_h_align(HorizontalAlign::Left)
                        .with_v_align(VerticalAlign::Center),
                )),
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

fn alignment_cell(h: HorizontalAlign, v: VerticalAlign, label_text: &'static str) -> Node {
    Node::new()
        .with_width(Size::Fill)
        .with_height(Size::Fill)
        .with_padding(Spacing::all(10.0))
        .with_shape(Shape::Rect(
            StyledRect::new(Default::default(), mocha::SURFACE0)
                .with_corner_shape(CornerShape::Round(14.0))
                .with_stroke(Stroke::new(2.0, mocha::SURFACE2)),
        ))
        .with_children(vec![
            label(label_text, 16.0, mocha::TEXT, h, v).with_height(Size::Fill)
        ])
}

fn alignment_row(v_name: &'static str, v: VerticalAlign) -> Node {
    Node::new()
        .with_height(Size::Fill)
        .with_gap(12.0)
        .with_layout_direction(Layout::Horizontal)
        .with_children(vec![
            alignment_cell(
                HorizontalAlign::Left,
                v,
                match v_name {
                    "Top" => "L / Top",
                    "Center" => "L / Center",
                    "Bottom" => "L / Bottom",
                    _ => "L",
                },
            ),
            alignment_cell(
                HorizontalAlign::Center,
                v,
                match v_name {
                    "Top" => "C / Top",
                    "Center" => "C / Center",
                    "Bottom" => "C / Bottom",
                    _ => "C",
                },
            ),
            alignment_cell(
                HorizontalAlign::Right,
                v,
                match v_name {
                    "Top" => "R / Top",
                    "Center" => "R / Center",
                    "Bottom" => "R / Bottom",
                    _ => "R",
                },
            ),
        ])
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Astra GUI - Text Demo")
                .with_inner_size(winit::dpi::LogicalSize::new(1100, 700));

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
                // If the event wasn't one of our debug keybinds, ignore it.
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
        debug_options: DebugOptions::none(),
    };

    println!("{}", DEBUG_HELP_TEXT);

    event_loop.run_app(&mut app).unwrap();
}
