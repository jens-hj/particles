use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const PARTICLE_COUNT: u32 = 100_000_000;
const PARTICLE_SIZE: f32 = 1.0; // Smaller particles for less overlap
const SPHERE_RADIUS: f32 = 8000.0; // Larger distribution sphere
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// Convert sRGB color (0-255) to linear RGB (0.0-1.0) for wgpu::Color
fn srgb_to_linear(value: u8) -> f64 {
    let value = value as f64 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

// Particle structure matching compute shader
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Particle {
    position: [f32; 3],
    _padding1: f32,
    color: [f32; 3],
    _padding2: f32,
}

// Camera uniform matching shaders
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    _padding: f32,
}

// Particle size uniform
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ParticleSizeUniform {
    size: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

struct Camera {
    distance: f32,
    rotation: Quat,
    target: Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn new(width: u32, height: u32) -> Self {
        // Start with a slight tilt looking down at the scene
        let rotation = Quat::from_rotation_x(0.3);

        Self {
            distance: 200.0,
            rotation,
            target: Vec3::ZERO,
            aspect: width as f32 / height as f32,
            fovy: 45.0_f32.to_radians(),
            znear: 0.1,
            zfar: 100000.0,
        }
    }

    fn position(&self) -> Vec3 {
        // Camera looks down -Z by default, so we need to offset by +Z and rotate
        let offset = self.rotation * Vec3::new(0.0, 0.0, self.distance);
        self.target + offset
    }

    fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        // All rotations around camera's local axes for trackball behavior

        // Horizontal rotation (around camera's local Y axis)
        let up = self.rotation * Vec3::Y;
        let yaw_rotation = Quat::from_axis_angle(up, delta_x);

        // Vertical rotation (around camera's local X axis)
        let right = self.rotation * Vec3::X;
        let pitch_rotation = Quat::from_axis_angle(right, -delta_y);

        // Apply both rotations to current orientation
        self.rotation = yaw_rotation * pitch_rotation * self.rotation;
        self.rotation = self.rotation.normalize();
    }

    fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance + delta).clamp(1.0, 50000.0);
    }

    fn build_view_projection_matrix(&self) -> Mat4 {
        // Build view matrix directly from quaternion to preserve orientation
        // View matrix = inverse(camera_transform) = inverse(rotation) * inverse(translation)
        let position = self.position();

        // Quaternion conjugate is the inverse for unit quaternions
        let rotation_matrix = Mat4::from_quat(self.rotation.conjugate());
        let translation_matrix = Mat4::from_translation(-position);

        let view = rotation_matrix * translation_matrix;
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);

        proj * view
    }

    fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            view_proj: self.build_view_projection_matrix().to_cols_array_2d(),
            position: self.position().to_array(),
            _padding: 0.0,
        }
    }
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    depth_texture: wgpu::TextureView,

    // Buffers
    _particle_buffers: Vec<wgpu::Buffer>,
    camera_buffer: wgpu::Buffer,
    _size_buffer: wgpu::Buffer,

    // Pipelines
    render_pipeline: wgpu::RenderPipeline,

    // Bind groups
    render_bind_groups: Vec<wgpu::BindGroup>,

    // State
    camera: Camera,

    // Performance tracking
    frame_times: Vec<f32>,
    last_frame_time: Instant,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone()).unwrap();

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        println!("Using GPU: {}", adapter.get_info().name);

        // Create device and queue with higher buffer limits for large particle counts
        let mut limits = wgpu::Limits::default();
        limits.max_storage_buffer_binding_size = 1024 * 1024 * 1024; // 1 GB
        limits.max_buffer_size = 1024 * 1024 * 1024; // 1 GB

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

        // Configure surface
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
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Generate particles uniformly in a sphere
        let mut rng = rand::rng();
        let mocha = &catppuccin::PALETTE.mocha.colors;
        let colors = [
            mocha.rosewater,
            mocha.flamingo,
            mocha.pink,
            mocha.mauve,
            mocha.red,
            mocha.maroon,
            mocha.peach,
            mocha.yellow,
            mocha.green,
            mocha.teal,
            mocha.sky,
            mocha.sapphire,
            mocha.blue,
            mocha.lavender,
        ];

        let particles: Vec<Particle> = (0..PARTICLE_COUNT)
            .map(|_| {
                let theta = rng.random::<f32>() * std::f32::consts::TAU;
                let cos_phi = rng.random::<f32>() * 2.0 - 1.0; // Uniform in [-1, 1]
                let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
                let r = rng.random::<f32>().powf(1.0 / 3.0) * SPHERE_RADIUS;

                let x = r * sin_phi * theta.cos();
                let y = r * sin_phi * theta.sin();
                let z = r * cos_phi;

                let color = colors[rng.random_range(0..colors.len())];
                let r_linear = srgb_to_linear(color.rgb.r) as f32;
                let g_linear = srgb_to_linear(color.rgb.g) as f32;
                let b_linear = srgb_to_linear(color.rgb.b) as f32;

                Particle {
                    position: [x, y, z],
                    _padding1: 0.0,
                    color: [r_linear, g_linear, b_linear],
                    _padding2: 0.0,
                }
            })
            .collect();

        // Create particle buffers in chunks
        const CHUNK_SIZE: usize = 20_000_000; // 20M particles * 32 bytes = 640MB
        let mut particle_buffers = Vec::new();

        for (i, chunk) in particles.chunks(CHUNK_SIZE).enumerate() {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Particle Buffer {}", i)),
                contents: bytemuck::cast_slice(chunk),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
            particle_buffers.push(buffer);
        }

        // Create camera
        let camera = Camera::new(size.width, size.height);
        let camera_uniform = camera.to_uniform();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create size buffer
        let size_uniform = ParticleSizeUniform {
            size: PARTICLE_SIZE,
            _padding1: 0.0,
            _padding2: 0.0,
            _padding3: 0.0,
        };
        let size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Size Buffer"),
            contents: bytemuck::cast_slice(&[size_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Load shaders
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string("assets/shaders/particle_billboard.wgsl")
                    .expect("Failed to load render shader")
                    .into(),
            ),
        });

        // Create render bind group layout
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout"),
                entries: &[
                    // Camera (uniform)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Particles (storage, read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Particle Size (uniform)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vertex"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create render bind groups (one per chunk)
        let mut render_bind_groups = Vec::new();
        for (i, buffer) in particle_buffers.iter().enumerate() {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Render Bind Group {}", i)),
                layout: &render_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: size_buffer.as_entire_binding(),
                    },
                ],
            });
            render_bind_groups.push(bind_group);
        }

        println!("✓ wgpu initialized: {} particles", PARTICLE_COUNT);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture: depth_view,
            _particle_buffers: particle_buffers,
            camera_buffer,
            _size_buffer: size_buffer,
            render_pipeline,
            render_bind_groups,
            camera,
            frame_times: Vec::with_capacity(100),
            last_frame_time: Instant::now(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth_texture = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
        }
    }

    fn render(&mut self) -> Result<(f32, f32), wgpu::SurfaceError> {
        // Track frame time
        let now = Instant::now();
        let frame_time = (now - self.last_frame_time).as_secs_f32() * 1000.0; // ms
        self.last_frame_time = now;

        self.frame_times.push(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

        // Calculate average FPS
        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        let fps = 1000.0 / avg_frame_time;

        // Update camera buffer
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera.to_uniform()]),
        );

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear({
                            let rgb = catppuccin::PALETTE.mocha.colors.base.rgb;
                            wgpu::Color {
                                r: srgb_to_linear(rgb.r),
                                g: srgb_to_linear(rgb.g),
                                b: srgb_to_linear(rgb.b),
                                a: 1.0,
                            }
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            // Draw chunks
            const CHUNK_SIZE: u32 = 20_000_000;
            for (i, bind_group) in self.render_bind_groups.iter().enumerate() {
                render_pass.set_bind_group(0, bind_group, &[]);

                let start_instance = (i as u32) * CHUNK_SIZE;
                let remaining = PARTICLE_COUNT.saturating_sub(start_instance);
                let count = remaining.min(CHUNK_SIZE);

                if count > 0 {
                    render_pass.draw(0..6, 0..count);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok((fps, avg_frame_time))
    }
}

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f64, f64)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Particles - Raw wgpu")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            // Initialize GPU state
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
            WindowEvent::CloseRequested => {
                println!("Close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Right {
                    self.mouse_pressed = state == ElementState::Pressed;
                    if !self.mouse_pressed {
                        self.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_pressed {
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta_x = (position.x - last_pos.0) as f32;
                        let delta_y = (position.y - last_pos.1) as f32;

                        if let Some(gpu_state) = &mut self.gpu_state {
                            // Invert pitch (delta_y) and apply rotation
                            gpu_state.camera.rotate(-delta_x * 0.005, delta_y * 0.005);
                        }
                    }
                    self.last_mouse_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y * 10.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                };

                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state
                        .camera
                        .zoom(-scroll * gpu_state.camera.distance / 100.0);
                }
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                if let Some(gpu_state) = &mut self.gpu_state {
                    match gpu_state.render() {
                        Ok((fps, frame_time)) => {
                            // Update window title with FPS every frame
                            window.set_title(&format!(
                                "Particles - {:.0} FPS ({:.2}ms) - {}k particles",
                                fps,
                                frame_time,
                                PARTICLE_COUNT / 1000
                            ));
                        }
                        Err(wgpu::SurfaceError::Lost) => gpu_state.resize(gpu_state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
            }
            _ => {}
        }

        // Request redraw continuously
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    println!("Starting raw wgpu particles...");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        gpu_state: None,
        mouse_pressed: false,
        last_mouse_pos: None,
    };

    event_loop.run_app(&mut app).unwrap();
}
