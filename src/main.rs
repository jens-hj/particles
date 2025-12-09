//! Fundamental Particle Physics Simulation
//!
//! Simulates quarks, electrons, and the four fundamental forces.

use glam::Vec3;
use particle_physics::{ColorCharge, Particle};
use particle_renderer::{Camera, ParticleRenderer};
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

const PARTICLE_COUNT: usize = 1000;
const SPAWN_RADIUS: f32 = 50.0;
const PARTICLE_SCALE: f32 = 3.0; // Global scale multiplier for visibility

/// Initialize particles with quarks and electrons
fn initialize_particles() -> Vec<Particle> {
    let mut rng = rand::rng();
    let mut particles = Vec::with_capacity(PARTICLE_COUNT);
    
    let colors = [
        ColorCharge::Red,
        ColorCharge::Green,
        ColorCharge::Blue,
    ];
    
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
        
        // 80% quarks (equal mix of up/down), 20% electrons
        let particle = if rng.random::<f32>() < 0.8 {
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
    
    println!("✓ Initialized {} particles", PARTICLE_COUNT);
    println!("  First particle: type={}, pos=({:.1}, {:.1}, {:.1}), color={}",
        particles[0].particle_type,
        particles[0].position[0],
        particles[0].position[1],
        particles[0].position[2],
        particles[0].color_charge
    );
    println!("  Particle struct size: {} bytes", std::mem::size_of::<Particle>());
    particles
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    
    simulation: ParticleSimulation,
    renderer: ParticleRenderer,
    camera: Camera,
    
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
        
        println!("✓ Using GPU: {}", adapter.get_info().name);
        
        // Create device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
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
        println!("✓ Simulation initialized");
        
        // Create renderer
        let renderer = ParticleRenderer::new(&device, &config);
        println!("✓ Renderer initialized");
        
        // Create camera
        let camera = Camera::new(size.width, size.height);
        
        Self {
            surface,
            device,
            queue,
            config,
            simulation,
            renderer,
            camera,
            frame_times: Vec::with_capacity(100),
            last_frame_time: Instant::now(),
        }
    }
    
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.renderer.resize(&self.device, &self.config);
            self.camera.resize(new_size.width, new_size.height);
        }
    }
    
    fn render(&mut self) -> Result<(f32, f32), wgpu::SurfaceError> {
        // Track frame time
        let now = Instant::now();
        let frame_time = (now - self.last_frame_time).as_secs_f32() * 1000.0;
        self.last_frame_time = now;
        
        self.frame_times.push(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }
        
        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        let fps = 1000.0 / avg_frame_time;
        
        // Step simulation
        self.simulation.step();
        
        // Render
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        self.renderer.render(
            &self.device,
            &self.queue,
            &view,
            &self.camera,
            self.simulation.particle_buffer(),
            self.simulation.particle_count(),
            PARTICLE_SCALE,
        );
        
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
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            } => event_loop.exit(),
            
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
            }
            
            WindowEvent::CursorMoved { position, .. } => {
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
                    gpu_state.camera.zoom(-scroll * gpu_state.camera.distance / 100.0);
                }
            }
            
            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(gpu_state)) = (&self.window, &mut self.gpu_state) {
                    match gpu_state.render() {
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
    println!("Starting fundamental particle physics simulation...");
    
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
