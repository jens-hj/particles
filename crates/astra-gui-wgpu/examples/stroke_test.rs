//! Tests stroke rendering with various widths and corner types

use astra_gui::{
    catppuccin::mocha, Color, CornerShape, FullOutput, LayoutDirection, Node, Overflow, Shape,
    Size, Spacing, Stroke, StyledRect,
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

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

        let ui_output = create_stroke_test_ui(self.config.width as f32, self.config.height as f32);

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

fn rect_with_stroke(
    fill_color: Color,
    stroke_color: Color,
    corner_shape: CornerShape,
    stroke_width: f32,
) -> Shape {
    Shape::Rect(
        StyledRect::new(Default::default(), fill_color)
            .with_corner_shape(corner_shape)
            .with_stroke(Stroke::new(stroke_width, stroke_color)),
    )
}

fn create_stroke_test_ui(width: f32, height: f32) -> FullOutput {
    // Test matrix:
    // - Rows: Different stroke widths (0.5px, 1px, 3px, 10px, 20px)
    // - Columns: Different corner types (None, Round, Cut, InverseRound, Squircle)

    let stroke_widths = vec![0.5, 1.0, 3.0, 10.0, 20.0];

    let corner_types = vec![
        ("None", CornerShape::None),
        ("Round", CornerShape::Round(30.0)),
        ("Cut", CornerShape::Cut(30.0)),
        ("InverseRound", CornerShape::InverseRound(30.0)),
        (
            "Squircle",
            CornerShape::Squircle {
                radius: 30.0,
                smoothness: 1.0,
            },
        ),
    ];

    let mut rows = vec![];

    for stroke_width in stroke_widths {
        let mut cells = vec![];

        for (_, corner_shape) in &corner_types {
            cells.push(
                Node::new()
                    .with_width(Size::Fill)
                    .with_height(Size::px(100.0))
                    .with_shape(rect_with_stroke(
                        mocha::SURFACE0,
                        mocha::BLUE,
                        *corner_shape,
                        stroke_width,
                    )),
            );
        }

        rows.push(
            Node::new()
                .with_height(Size::px(120.0))
                .with_gap(20.0)
                .with_layout_direction(LayoutDirection::Horizontal)
                .with_children(cells),
        );
    }

    let root = Node::new()
        .with_padding(Spacing::all(40.0))
        .with_gap(20.0)
        .with_layout_direction(LayoutDirection::Vertical)
        .with_overflow(Overflow::Visible)
        .with_children(rows);

    FullOutput::from_node(root, (width, height))
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Astra GUI - Stroke Test")
                .with_inner_size(winit::dpi::LogicalSize::new(1600, 900));

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

            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    match gpu_state.render() {
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
    };

    println!("Stroke Test - Testing various stroke widths on all corner types");
    println!("Rows: 0.5px, 1px, 3px, 10px, 20px stroke widths");
    println!("Columns: None, Round, Cut, InverseRound, Squircle");
    println!("Press ESC to exit");

    event_loop.run_app(&mut app).unwrap();
}
