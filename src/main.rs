//! Fundamental Particle Physics Simulation
//!
//! Simulates quarks, electrons, and the four fundamental forces.

mod gui;

use glam::Vec3;
use gui::{Gui, UiState};
use particle_physics::{ColorCharge, Particle};
use particle_renderer::{Camera, GpuPicker, HadronRenderer, ParticleRenderer, PickingRenderer};
use particle_simulation::ParticleSimulation;
use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const PARTICLE_COUNT: usize = 8000;
const SPAWN_RADIUS: f32 = 50.0;
const PARTICLE_SCALE: f32 = 3.0; // Global scale multiplier for visibility

/// Initialize particles with quarks and electrons
fn initialize_particles() -> Vec<Particle> {
    let mut rng = rand::rng();
    let mut particles = Vec::with_capacity(PARTICLE_COUNT);

    let colors = [ColorCharge::Red, ColorCharge::Green, ColorCharge::Blue];

    // Create particles: mostly quarks, some electrons
    for _ in 0..PARTICLE_COUNT {
        // Random position in sphere
        let theta = rng.random::<f32>() * std::f32::consts::TAU;
        let cos_phi = rng.random::<f32>() * 2.0 - 1.0;
        let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
        let r = rng.random::<f32>().powf(1.0 / 3.0) * SPAWN_RADIUS;

        let x = r * sin_phi * theta.cos();
        let y = r * sin_phi * theta.sin();
        let z = r * cos_phi;
        let pos = Vec3::new(x, y, z);

        // 80% quarks, 20% electrons
        let rand_val = rng.random::<f32>();
        let particle = if rand_val < 0.9 {
            let color = colors[rng.random_range(0..colors.len())];
            if rng.random::<bool>() {
                Particle::new_up_quark(pos, color)
            } else {
                Particle::new_down_quark(pos, color)
            }
        } else {
            Particle::new_electron(pos)
        };

        particles.push(particle);
    }

    log::info!("✓ Initialized {} particles", PARTICLE_COUNT);
    log::info!(
        "  Particle struct size: {} bytes",
        std::mem::size_of::<Particle>()
    );
    log::info!("  First 10 particles:");
    for i in 0..10.min(particles.len()) {
        let p = &particles[i];
        log::info!(
            "    [{}] type={}, color={}, charge={:.2}, size={:.2}",
            i,
            p.position[3] as u32,
            p.color_and_flags[0],
            p.data[0],
            p.data[1]
        );
    }

    particles
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    simulation: ParticleSimulation,
    renderer: ParticleRenderer,
    hadron_renderer: HadronRenderer,
    camera: Camera,

    gui: Gui,
    ui_state: UiState,
    hadron_count_staging_buffer: wgpu::Buffer,

    // GPU picking (ID render + 1px readback)
    picker: GpuPicker,
    picking_renderer: PickingRenderer,

    // Camera lock (follow selected entity)
    camera_lock: Option<CameraLock>,

    // Selection resolve (GPU -> CPU readback for camera target)
    selection_target_staging_buffer: wgpu::Buffer,
    selection_target_cached: Option<[f32; 4]>,

    frame_times: Vec<f32>,
    last_frame_time: Instant,
    frame_counter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraLock {
    Particle { particle_index: u32 },
    Hadron { hadron_index: u32 },
}

fn decode_pick_id(raw: u32) -> Option<CameraLock> {
    if raw == 0 {
        return None;
    }

    let is_hadron = (raw & 0x8000_0000) != 0;
    let idx_1 = raw & 0x7FFF_FFFF;

    if idx_1 == 0 {
        return None;
    }

    let idx0 = idx_1 - 1;

    if is_hadron {
        Some(CameraLock::Hadron { hadron_index: idx0 })
    } else {
        Some(CameraLock::Particle {
            particle_index: idx0,
        })
    }
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

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

        log::info!("✓ Using GPU: {}", adapter.get_info().name);

        // Create device and queue
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
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Initialize particles
        let particles = initialize_particles();

        // Create simulation
        let simulation = ParticleSimulation::new(device.clone(), queue.clone(), &particles).await;
        log::info!("✓ Simulation initialized");

        // Create renderer
        let renderer = ParticleRenderer::new(&device, &config);
        log::info!("✓ Renderer initialized");

        // Create hadron renderer
        let dummy_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Dummy Layout"),
            entries: &[],
        });
        let hadron_renderer = HadronRenderer::new(&device, config.format, &dummy_layout);
        log::info!("✓ Hadron Renderer initialized");

        // Create camera
        let camera = Camera::new(size.width, size.height);

        // Create GUI
        let gui = Gui::new(&device, config.format, &window);
        let ui_state = UiState::default();

        // GPU picking:
        // - ID target is RGBA8 (packed u32 ID)
        // - Depth for occlusion
        let picker = GpuPicker::new(
            &device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let picking_renderer = PickingRenderer::new(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Depth32Float,
            config.width,
            config.height,
        );

        // Create staging buffer for reading hadron counters:
        // [total_hadrons, protons, neutrons, other]
        let hadron_count_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hadron Count Staging Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Selection target readback (vec4<f32> = 16 bytes)
        let selection_target_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Selection Target Staging Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            device,
            queue,
            config,
            simulation,
            renderer,
            hadron_renderer,
            camera,
            gui,
            ui_state,
            hadron_count_staging_buffer,

            picker,
            picking_renderer,
            camera_lock: None,

            selection_target_staging_buffer,
            selection_target_cached: None,

            frame_times: Vec::with_capacity(100),
            last_frame_time: Instant::now(),
            frame_counter: 0,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.renderer.resize(&self.device, &self.config);
            self.camera.resize(new_size.width, new_size.height);

            self.picker
                .resize(&self.device, self.config.width, self.config.height);
            self.picking_renderer
                .resize(&self.device, self.config.width, self.config.height);
        }
    }

    fn render(&mut self, window: &Window) -> Result<(f32, f32), wgpu::SurfaceError> {
        // Track frame time
        let now = Instant::now();
        let frame_time = (now - self.last_frame_time).as_secs_f32() * 1000.0;
        self.last_frame_time = now;

        // Camera lock: keep camera.target following the selected entity every frame.
        //
        // We use the cached selection target produced by the selection-resolve compute pass.
        // Only update the camera target when we have a valid resolved target.
        if self.camera_lock.is_some() {
            if let Some(target) = self.selection_target_cached {
                // target.w = kind (0 none, 1 particle, 2 hadron)
                if target[3] != 0.0 {
                    self.camera.target = Vec3::new(target[0], target[1], target[2]);
                }
            }
        }

        self.frame_times.push(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        let fps = 1000.0 / avg_frame_time;

        self.frame_counter += 1;

        // Update physics parameters from UI
        // Pass accumulated time to shader for random seeding (using integration.z padding)
        if !self.ui_state.is_paused || self.ui_state.step_one_frame {
            self.ui_state.physics_params.integration[2] += frame_time * 0.001;
        }
        self.simulation.update_params(&self.ui_state.physics_params);

        // Step simulation
        if !self.ui_state.is_paused || self.ui_state.step_one_frame {
            self.simulation.step();
            self.ui_state.step_one_frame = false;
        }

        // Read back hadron count (only every 10 frames to avoid blocking)
        if self.frame_counter % 10 == 0 {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Readback Encoder"),
                });

            encoder.copy_buffer_to_buffer(
                self.simulation.hadron_count_buffer(),
                0,
                &self.hadron_count_staging_buffer,
                0,
                16,
            );

            self.queue.submit(std::iter::once(encoder.finish()));

            let slice = self.hadron_count_staging_buffer.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .unwrap();

            {
                let data = slice.get_mapped_range();

                // Layout: 4 little-endian u32 values
                // [0] total hadrons
                // [1] protons
                // [2] neutrons
                // [3] other
                let bytes: &[u8] = &data;

                self.ui_state.hadron_count = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                self.ui_state.proton_count = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
                self.ui_state.neutron_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
                self.ui_state.other_hadron_count =
                    u32::from_le_bytes(bytes[12..16].try_into().unwrap());
            }
            self.hadron_count_staging_buffer.unmap();
        }

        // Update UI state
        self.ui_state.fps = fps;
        self.ui_state.frame_time = avg_frame_time;
        self.ui_state.particle_count = PARTICLE_COUNT;

        // Render
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.renderer.render(
            &self.device,
            &self.queue,
            &view,
            &self.camera,
            self.simulation.particle_buffer(),
            self.simulation.hadron_buffer(),
            self.simulation.hadron_count_buffer(),
            self.simulation.particle_count(),
            PARTICLE_SCALE,
            self.ui_state.physics_params.integration[2],
            self.ui_state.lod_shell_fade_start,
            self.ui_state.lod_shell_fade_end,
            self.ui_state.lod_bond_fade_start,
            self.ui_state.lod_bond_fade_end,
            self.ui_state.lod_quark_fade_start,
            self.ui_state.lod_quark_fade_end,
        );

        // Render Hadrons
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Hadron Render Encoder"),
                });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Hadron Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.renderer.depth_texture,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                self.hadron_renderer.render(
                    &self.device,
                    &mut render_pass,
                    &self.renderer.camera_buffer,
                    self.simulation.hadron_buffer(),
                    self.simulation.particle_buffer(),
                    self.simulation.hadron_count_buffer(),
                    self.simulation.particle_count(),
                    self.ui_state.show_shells,
                    self.ui_state.show_bonds,
                );
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Render GUI
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("GUI Encoder"),
                });

            self.gui.render(
                &self.device,
                &self.queue,
                &mut encoder,
                window,
                &view,
                &mut self.ui_state,
            );

            self.queue.submit(std::iter::once(encoder.finish()));
        }

        output.present();
        Ok((fps, avg_frame_time))
    }
}

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f64, f64)>,

    // Picking
    left_mouse_pressed: bool,
    last_cursor_pos: Option<(f64, f64)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Particle Physics Simulation")
                .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080));

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
        // Handle GUI events
        if let (Some(gpu_state), Some(window)) = (&mut self.gpu_state, &self.window) {
            if gpu_state.gui.handle_event(window, &event) {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyC),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.camera.target = Vec3::ZERO;
                    gpu_state.camera_lock = None;
                    gpu_state.selection_target_cached = None;
                    gpu_state.simulation.set_selected_id(0);
                }
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Right {
                    self.mouse_pressed = state == ElementState::Pressed;
                    if !self.mouse_pressed {
                        self.last_mouse_pos = None;
                    }
                }

                if button == winit::event::MouseButton::Left {
                    self.left_mouse_pressed = state == ElementState::Pressed;

                    // GPU picking: render IDs into an offscreen target then read back the clicked pixel.
                    if state == ElementState::Pressed {
                        let Some((x, y)) = self.last_cursor_pos else {
                            return;
                        };
                        let Some(gpu_state) = &mut self.gpu_state else {
                            return;
                        };
                        let Some(window) = &self.window else {
                            return;
                        };

                        let size = window.inner_size();
                        let w = size.width.max(1) as f64;
                        let h = size.height.max(1) as f64;

                        // Convert window-space -> texture pixel coords (origin top-left in winit).
                        let px = ((x / w) * gpu_state.config.width as f64) as u32;
                        let py = ((y / h) * gpu_state.config.height as f64) as u32;

                        let mut encoder = gpu_state.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Picking Encoder"),
                            },
                        );

                        // Render IDs into offscreen target
                        gpu_state.picking_renderer.render(
                            &gpu_state.device,
                            &gpu_state.queue,
                            &mut encoder,
                            &gpu_state.picker.id_texture_view,
                            &gpu_state.camera,
                            gpu_state.simulation.particle_buffer(),
                            gpu_state.simulation.hadron_buffer(),
                            gpu_state.simulation.hadron_count_buffer(),
                            gpu_state.simulation.particle_count(),
                            gpu_state.simulation.particle_count(), // max_hadrons == particle_count allocation
                            PARTICLE_SCALE,
                            gpu_state.ui_state.physics_params.integration[2],
                            gpu_state.ui_state.lod_shell_fade_start,
                            gpu_state.ui_state.lod_shell_fade_end,
                            gpu_state.ui_state.lod_bond_fade_start,
                            gpu_state.ui_state.lod_bond_fade_end,
                            gpu_state.ui_state.lod_quark_fade_start,
                            gpu_state.ui_state.lod_quark_fade_end,
                        );

                        // Copy clicked pixel into staging buffer
                        gpu_state.picker.encode_read_pixel(&mut encoder, px, py);

                        gpu_state.queue.submit(std::iter::once(encoder.finish()));

                        // Map + blockingly poll for the readback (clicks are rare so this is OK).
                        let slice = gpu_state.picker.staging_buffer().slice(..);
                        slice.map_async(wgpu::MapMode::Read, |_| {});
                        gpu_state
                            .device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: None,
                            })
                            .unwrap();

                        let pick = gpu_state.picker.read_mapped();
                        gpu_state.picker.staging_buffer().unmap();

                        // Update selection ID in the simulation and resolve it to a world-space target.
                        gpu_state.simulation.set_selected_id(pick.id);
                        gpu_state.camera_lock = decode_pick_id(pick.id);

                        // Resolve selection -> target position (GPU compute), then read back vec4<f32>.
                        if gpu_state.camera_lock.is_some() {
                            let mut resolve_encoder = gpu_state.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Selection Resolve Encoder"),
                                },
                            );

                            gpu_state
                                .simulation
                                .encode_selection_resolve(&mut resolve_encoder);

                            resolve_encoder.copy_buffer_to_buffer(
                                gpu_state.simulation.selection_target_buffer(),
                                0,
                                &gpu_state.selection_target_staging_buffer,
                                0,
                                16,
                            );

                            gpu_state
                                .queue
                                .submit(std::iter::once(resolve_encoder.finish()));

                            let slice = gpu_state.selection_target_staging_buffer.slice(..);
                            slice.map_async(wgpu::MapMode::Read, |_| {});
                            gpu_state
                                .device
                                .poll(wgpu::PollType::Wait {
                                    submission_index: None,
                                    timeout: None,
                                })
                                .unwrap();

                            {
                                let data = slice.get_mapped_range();
                                let bytes: &[u8] = &data;

                                let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
                                let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
                                let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
                                let w = f32::from_le_bytes(bytes[12..16].try_into().unwrap());

                                gpu_state.selection_target_cached = Some([x, y, z, w]);

                                // Immediately move camera to target on click.
                                if w != 0.0 {
                                    gpu_state.camera.target = Vec3::new(x, y, z);
                                }
                            }

                            gpu_state.selection_target_staging_buffer.unmap();
                        } else {
                            // Cleared selection
                            gpu_state.selection_target_cached = None;
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = Some((position.x, position.y));

                if self.mouse_pressed {
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta_x = (position.x - last_pos.0) as f32;
                        let delta_y = (position.y - last_pos.1) as f32;

                        if let Some(gpu_state) = &mut self.gpu_state {
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
                if let (Some(window), Some(gpu_state)) = (&self.window, &mut self.gpu_state) {
                    match gpu_state.render(window) {
                        Ok((fps, frame_time)) => {
                            window.set_title(&format!(
                                "Particle Physics - {:.0} FPS ({:.2}ms) - {} particles",
                                fps, frame_time, PARTICLE_COUNT
                            ));
                        }
                        Err(wgpu::SurfaceError::Lost) => gpu_state.resize(window.inner_size()),
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
    // Initialize logger (RUST_LOG=debug for verbose output)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting fundamental particle physics simulation...");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        gpu_state: None,
        mouse_pressed: false,
        last_mouse_pos: None,

        left_mouse_pressed: false,
        last_cursor_pos: None,
    };

    event_loop.run_app(&mut app).unwrap();
}
