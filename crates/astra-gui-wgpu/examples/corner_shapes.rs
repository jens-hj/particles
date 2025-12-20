//! Demonstrates all available corner shapes for rectangles

use astra_gui::{ClippedShape, Color, CornerShape, FullOutput, Rect, Shape, Stroke, StyledRect};
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

        // Render shapes
        let shapes = create_demo_shapes(self.config.width as f32, self.config.height as f32);

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
            &shapes,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();
        Ok(())
    }
}

fn create_demo_shapes(width: f32, height: f32) -> FullOutput {
    let clip_rect = Rect::new([0.0, 0.0], [width, height]);
    let mut shapes = Vec::new();

    let rect_width = 500.0;
    let rect_height = 300.0;
    let margin = 100.0;
    let spacing = 600.0;
    let corner_size = 50.0;
    let stroke_width = 20.0;

    let start_x = margin;
    let start_y = margin;

    // Row 1: None, Round, Cut
    // None (sharp corners)
    shapes.push(ClippedShape::new(
        clip_rect,
        Shape::Rect(
            StyledRect::new(
                Rect::new(
                    [start_x, start_y],
                    [start_x + rect_width, start_y + rect_height],
                ),
                Color::new(0.8, 0.3, 0.3, 1.0),
            )
            .with_corner_shape(CornerShape::None)
            .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
        ),
    ));

    // Round
    shapes.push(ClippedShape::new(
        clip_rect,
        Shape::Rect(
            StyledRect::new(
                Rect::new(
                    [start_x + spacing, start_y],
                    [start_x + spacing + rect_width, start_y + rect_height],
                ),
                Color::new(0.3, 0.8, 0.3, 1.0),
            )
            .with_corner_shape(CornerShape::Round(corner_size))
            .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
        ),
    ));

    // Cut
    shapes.push(ClippedShape::new(
        clip_rect,
        Shape::Rect(
            StyledRect::new(
                Rect::new(
                    [start_x + spacing * 2.0, start_y],
                    [start_x + spacing * 2.0 + rect_width, start_y + rect_height],
                ),
                Color::new(0.3, 0.3, 0.8, 1.0),
            )
            .with_corner_shape(CornerShape::Cut(corner_size))
            // .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
        ),
    ));

    // Row 2: InverseRound, Squircle
    let row2_y = start_y + rect_height + margin;

    // InverseRound
    shapes.push(ClippedShape::new(
        clip_rect,
        Shape::Rect(
            StyledRect::new(
                Rect::new(
                    [start_x, row2_y],
                    [start_x + rect_width, row2_y + rect_height],
                ),
                Color::new(0.8, 0.8, 0.3, 1.0),
            )
            .with_corner_shape(CornerShape::InverseRound(corner_size))
            .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
        ),
    ));

    // Squircle (low smoothness - more circular)
    shapes.push(ClippedShape::new(
        clip_rect,
        Shape::Rect(
            StyledRect::new(
                Rect::new(
                    [start_x + spacing, row2_y],
                    [start_x + spacing + rect_width, row2_y + rect_height],
                ),
                Color::new(0.8, 0.3, 0.8, 1.0),
            )
            .with_corner_shape(CornerShape::Squircle {
                radius: corner_size,
                smoothness: 0.5,
            })
            .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
        ),
    ));

    // Squircle (high smoothness - more square-like)
    shapes.push(ClippedShape::new(
        clip_rect,
        Shape::Rect(
            StyledRect::new(
                Rect::new(
                    [start_x + spacing * 2.0, row2_y],
                    [start_x + spacing * 2.0 + rect_width, row2_y + rect_height],
                ),
                Color::new(0.3, 0.8, 0.8, 1.0),
            )
            .with_corner_shape(CornerShape::Squircle {
                radius: corner_size,
                smoothness: 3.0,
            })
            .with_stroke(Stroke::new(stroke_width, Color::new(1.0, 1.0, 1.0, 1.0))),
        ),
    ));

    FullOutput::with_shapes(shapes)
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Astra GUI - Corner Shapes Demo")
                .with_inner_size(winit::dpi::LogicalSize::new(800, 400));

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

    event_loop.run_app(&mut app).unwrap();
}
