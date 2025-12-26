//! Demonstrates node overflow behaviors: Hidden vs Visible vs Scroll (placeholder).
//!
//! This example is intentionally minimal: each column contains a single "viewport" container
//! and a single child text node that is positioned to extend beyond the viewport bounds.
//!
//! Notes:
//! - In astra-gui core, `Overflow::Hidden` is the default and enforces clip rect intersection.
//! - `Overflow::Scroll` is currently treated like `Hidden` (clipping only; no scroll offsets yet).
//! - In the WGPU backend, text uses per-shape scissor and respects `ClippedShape::clip_rect`.
//!
//! Controls:
//! - D: toggle all debug overlays
//! - M/P/B/C: toggle individual debug overlays
//! - ESC: quit
//!
//! Tip:
//! - Toggle borders (B) and content area (C) to understand which rect is clipping.

use astra_gui::{
    catppuccin::mocha, Color, Content, ContentMeasurer, CornerShape, DebugOptions, FullOutput,
    HorizontalAlign, Layout, Node, Overflow, Rect, Shape, Size, Spacing, Stroke, StyledRect,
    TextContent, VerticalAlign,
};
use astra_gui_text::Engine as TextEngine;
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
    text_engine: TextEngine,
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

    fn render(
        &mut self,
        debug_options: &DebugOptions,
        measurer: &mut dyn ContentMeasurer,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Clear
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

        // UI
        let ui_output = create_demo_ui(
            self.config.width as f32,
            self.config.height as f32,
            debug_options,
            measurer,
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

fn panel(fill: Color) -> Shape {
    Shape::Rect(
        StyledRect::new(Default::default(), fill)
            .with_corner_shape(CornerShape::Round(14.0))
            .with_stroke(Stroke::new(2.0, mocha::SURFACE1)),
    )
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

fn demo_box(title: &str, overflow_mode: Overflow, color: Color) -> Node {
    // A tight viewport; inside we place a single child whose text is positioned so it
    // extends beyond the viewport bounds.
    //
    // - Hidden/Scroll: the overflowing portion must be clipped.
    // - Visible: the overflowing portion can render outside the viewport bounds.
    //
    // NOTE: `Overflow::Scroll` is currently treated like `Hidden` in core (clip only).
    let long_text =
        "OVERFLOW DEMO →→→ this text extends beyond the viewport bounds →→→ →→→ →→→ →→→";

    Node::new()
        .with_width(Size::Fill)
        .with_height(Size::Fill)
        .with_padding(Spacing::all(16.0))
        .with_gap(14.0)
        .with_layout_direction(Layout::Vertical)
        .with_overflow(overflow_mode)
        .with_shape(panel(color))
        .with_children(vec![
            // Title row
            Node::new()
                .with_height(Size::px(40.0))
                .with_layout_direction(Layout::Horizontal)
                .with_children(vec![label(
                    title,
                    22.0,
                    mocha::TEXT,
                    HorizontalAlign::Left,
                    VerticalAlign::Center,
                )]),
            // Viewport content: one oversized child.
            // NOTE: We must propagate Overflow::Visible down through wrapper nodes,
            // otherwise the default Overflow::Hidden on intermediate nodes will clip.
            Node::new()
                .with_height(Size::Fill)
                .with_width(Size::Fill)
                .with_padding(Spacing::all(14.0))
                .with_shape(panel(mocha::SURFACE0))
                .with_layout_direction(Layout::Horizontal)
                .with_overflow(overflow_mode) // Propagate overflow mode
                .with_children(vec![Node::new()
                    .with_overflow(overflow_mode)
                    .with_children(vec![label(
                        long_text,
                        26.0,
                        mocha::TEXT,
                        HorizontalAlign::Left,
                        VerticalAlign::Top,
                    )])]),
        ])
}

fn create_demo_ui(
    width: f32,
    height: f32,
    debug_options: &DebugOptions,
    measurer: &mut dyn ContentMeasurer,
) -> FullOutput {
    let _window_rect = Rect::new([0.0, 0.0], [width, height]);

    let root = Node::new()
        // Root clips by default (Overflow::Hidden default). Keep it Visible so the
        // "Visible" column can actually show overflow past its own viewport.
        .with_overflow(Overflow::Visible)
        .with_padding(Spacing::all(24.0))
        .with_gap(18.0)
        .with_layout_direction(Layout::Vertical)
        .with_width(Size::Fill)
        .with_height(Size::Fill)
        .with_children(vec![
            // Header
            Node::new()
                .with_height(Size::px(120.0))
                .with_width(Size::Fill)
                .with_padding(Spacing::all(18.0))
                .with_shape(panel(mocha::SURFACE0))
                .with_children(vec![
                    label(
                        "astra-gui: Overflow demo",
                        34.0,
                        mocha::TEXT,
                        HorizontalAlign::Left,
                        VerticalAlign::Top,
                    )
                    .with_height(Size::Fill),
                    label(
                        "Each column has one viewport + one text child that starts outside the viewport. Hidden/Scroll clip; Visible does not.",
                        18.0,
                        mocha::SUBTEXT0,
                        HorizontalAlign::Left,
                        VerticalAlign::Bottom,
                    )
                    .with_height(Size::Fill),
                ]),
            // Columns
            Node::new()
                .with_width(Size::Fill)
                .with_height(Size::Fill)
                .with_gap(18.0)
                .with_layout_direction(Layout::Vertical)
                .with_overflow(Overflow::Visible) // Allow children to overflow
                .with_children(vec![
                    Node::new()
                        .with_width(Size::px(400.0))
                        .with_height(Size::Fill)
                        .with_children(vec![demo_box(
                            "Overflow: Hidden (default)",
                            Overflow::Hidden,
                            mocha::CRUST,
                        )]),
                    Node::new()
                        .with_width(Size::px(400.0))
                        .with_height(Size::Fill)
                        .with_overflow(Overflow::Visible) // Allow child to overflow
                        .with_children(vec![demo_box(
                            "Overflow: Visible",
                            Overflow::Visible,
                            mocha::MANTLE,
                        )]),
                    Node::new()
                        .with_width(Size::px(400.0))
                        .with_height(Size::Fill)
                        .with_children(vec![demo_box(
                            "Overflow: Scroll (placeholder)",
                            Overflow::Scroll,
                            mocha::SURFACE0,
                        )]),
                ]),
            // Help bar
            Node::new()
                .with_height(Size::px(30.0))
                .with_width(Size::Fill)
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

    FullOutput::from_node_with_debug_and_measurer(
        root,
        (width, height),
        if debug_options.is_enabled() {
            Some(*debug_options)
        } else {
            None
        },
        Some(measurer),
    )
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Astra GUI - Overflow Demo")
                .with_inner_size(winit::dpi::LogicalSize::new(1180, 720));

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
                    match gpu_state.render(&self.debug_options, &mut self.text_engine) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            gpu_state.resize(winit::dpi::PhysicalSize::new(
                                gpu_state.config.width,
                                gpu_state.config.height,
                            ));
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("render error: {e:?}"),
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    println!("{DEBUG_HELP_TEXT}");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        gpu_state: None,
        debug_options: DebugOptions::none(),
        text_engine: TextEngine::new_default(),
    };

    event_loop.run_app(&mut app).unwrap();
}
