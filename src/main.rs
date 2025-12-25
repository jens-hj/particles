//! Fundamental Particle Physics Simulation
//!
//! Simulates quarks, electrons, and the four fundamental forces.

mod gui;

use astra_gui::DebugOptions;
use astra_gui_wgpu::Renderer as AstraRenderer;
use glam::Vec3;
use gui::{Gui, UiState};
use particle_physics::{ColorCharge, Particle};
use particle_renderer::{
    Camera, GpuPicker, HadronRenderer, NucleusRenderer, ParticleRenderer, PickingRenderer,
};
use particle_simulation::ParticleSimulation;
use rand::Rng;
use std::collections::VecDeque;
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
    nucleus_renderer: NucleusRenderer,
    camera: Camera,

    gui: Gui,
    astra_renderer: AstraRenderer,
    ui_state: UiState,
    hadron_count_staging_buffer: wgpu::Buffer,
    _nucleus_count_staging_buffer: wgpu::Buffer,

    // GPU picking (ID render + 1px readback)
    picker: GpuPicker,
    picking_renderer: PickingRenderer,

    // Camera lock (follow selected entity)
    camera_lock: Option<CameraLock>,

    // Selection resolve (GPU -> CPU readback for camera target)
    selection_target_staging_buffer: wgpu::Buffer,
    selection_target_cached: Option<[f32; 4]>,

    // Selected nucleus readback (for atom card UI)
    nucleus_readback_staging_buffer: wgpu::Buffer,
    nucleus_readback_capacity: u32,

    // Smooth distance target when locking onto a selection.
    camera_distance_target: Option<f32>,

    // If true, the user has manually zoomed while locked. We should not re-arm auto-zoom
    // until a new selection is made (otherwise we fight the user).
    camera_zoom_user_override: bool,

    // Smooth reset target when pressing `C` (avoid snapping).
    camera_reset_target: Option<Vec3>,

    // Shared picking particle size used for BOTH:
    // - click-time picking render+readback
    // - the picking overlay pass (visualization)
    //
    // Keep these in sync so the overlay represents the exact pick colliders.
    picking_particle_size: f32,

    frame_times: VecDeque<f32>,
    last_frame_time: Instant,
    frame_counter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraLock {
    Particle { particle_index: u32 },
    Hadron { hadron_index: u32 },
    Nucleus { anchor_hadron_index: u32 },
}

fn decode_pick_id(raw: u32) -> Option<CameraLock> {
    if raw == 0 {
        return None;
    }

    let is_hadron = (raw & 0x8000_0000) != 0;
    let is_nucleus = (!is_hadron) && ((raw & 0x4000_0000) != 0);

    let idx_1 = if is_hadron {
        raw & 0x7FFF_FFFF
    } else if is_nucleus {
        raw & 0x3FFF_FFFF
    } else {
        raw
    };

    if idx_1 == 0 {
        return None;
    }

    let idx0 = idx_1 - 1;

    if is_hadron {
        Some(CameraLock::Hadron { hadron_index: idx0 })
    } else if is_nucleus {
        Some(CameraLock::Nucleus {
            anchor_hadron_index: idx0,
        })
    } else {
        Some(CameraLock::Particle {
            particle_index: idx0,
        })
    }
}

impl GpuState {
    /// Read back nucleus data for the atom card UI.
    /// Searches through nuclei to find the one with the matching anchor hadron index.
    /// Uses a cached staging buffer with dynamic search range (starts at 50, grows to 1000 if needed).
    fn update_selected_nucleus_data(&mut self, anchor_hadron_index: u32) {
        let nucleus_size = 112u64; // Size of Nucleus struct

        // Start with a small search range, grow dynamically if needed
        let mut search_range = 50u32.min(self.nucleus_readback_capacity);

        // Try up to 3 iterations with increasing search ranges
        for attempt in 0..3 {
            if attempt > 0 {
                // Double the search range, capped at 1000
                search_range = (search_range * 2).min(1000);

                // Resize buffer if needed
                if search_range > self.nucleus_readback_capacity {
                    self.nucleus_readback_capacity = search_range;
                    self.nucleus_readback_staging_buffer =
                        self.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("Nucleus Readback Staging Buffer (Resized)"),
                            size: nucleus_size * search_range as u64,
                            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                }
            }

            let buffer_size = nucleus_size * search_range as u64;

            let mut nucleus_encoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Nucleus Readback Encoder"),
                    });

            // Copy nuclei from GPU buffer using cached staging buffer
            nucleus_encoder.copy_buffer_to_buffer(
                self.simulation.nucleus_buffer(),
                0,
                &self.nucleus_readback_staging_buffer,
                0,
                buffer_size,
            );

            self.queue.submit(std::iter::once(nucleus_encoder.finish()));

            let nucleus_slice = self.nucleus_readback_staging_buffer.slice(..buffer_size);
            nucleus_slice.map_async(wgpu::MapMode::Read, |_| {});
            // TODO: Convert to async ring buffer to avoid blocking GPU pipeline
            // See: https://toji.dev/webgpu-best-practices/buffer-uploads
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .unwrap();

            let mut found = false;
            {
                let data = nucleus_slice.get_mapped_range();
                let bytes: &[u8] = &data;

                // Search through nuclei to find the one with matching anchor hadron
                for i in 0..search_range {
                    let base_offset = (i as usize) * (nucleus_size as usize);
                    if base_offset + nucleus_size as usize > bytes.len() {
                        break;
                    }

                    // Read the first hadron index (the anchor)
                    let first_hadron_idx =
                        u32::from_le_bytes(bytes[base_offset..base_offset + 4].try_into().unwrap());

                    // Check if this nucleus contains our anchor hadron
                    if first_hadron_idx == anchor_hadron_index {
                        // Parse this nucleus's data
                        let data_offset = base_offset + 64; // Skip hadron_indices[16]
                        let nucleon_count = u32::from_le_bytes(
                            bytes[data_offset..data_offset + 4].try_into().unwrap(),
                        );
                        let proton_count = u32::from_le_bytes(
                            bytes[data_offset + 4..data_offset + 8].try_into().unwrap(),
                        );
                        let neutron_count = u32::from_le_bytes(
                            bytes[data_offset + 8..data_offset + 12].try_into().unwrap(),
                        );
                        let type_id = u32::from_le_bytes(
                            bytes[data_offset + 12..data_offset + 16]
                                .try_into()
                                .unwrap(),
                        );

                        // Only update if this is a valid nucleus
                        if type_id != 0xFFFF_FFFF {
                            self.ui_state.selected_nucleus_atomic_number = Some(type_id);
                            self.ui_state.selected_nucleus_proton_count = Some(proton_count);
                            self.ui_state.selected_nucleus_neutron_count = Some(neutron_count);
                            self.ui_state.selected_nucleus_nucleon_count = Some(nucleon_count);

                            found = true;
                            break;
                        }
                    }
                }
            }

            self.nucleus_readback_staging_buffer.unmap();

            if found {
                return; // Success, exit early
            }
        }

        // Nucleus not found after all attempts
        log::debug!(
            "Nucleus with anchor_hadron_index={} not found after searching {} nuclei",
            anchor_hadron_index,
            search_range
        );
    }

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

        let nucleus_renderer = NucleusRenderer::new(&device, config.format, &dummy_layout);
        log::info!("✓ Nucleus Renderer initialized");

        // Create camera
        let camera = Camera::new(size.width, size.height);

        // Create GUI
        let gui = Gui::new(&device, config.format, &window);
        let astra_renderer = AstraRenderer::new(&device, config.format);
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

        // Create staging buffer for reading nucleus counter
        let _nucleus_count_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Nucleus Count Staging Buffer"),
            size: 32, // WGSL atomic alignment requirement
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

        // Selected nucleus readback (for atom card UI)
        // Nucleus struct size: 64 (hadron_indices) + 4*4 (counts/type_id) + 16 (center) + 16 (velocity) = 112 bytes
        let initial_nucleus_capacity = 100u32;
        let nucleus_size = 112u64;
        let nucleus_readback_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Nucleus Readback Staging Buffer"),
            size: nucleus_size * initial_nucleus_capacity as u64,
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
            nucleus_renderer,
            camera,
            gui,
            astra_renderer,
            ui_state,
            hadron_count_staging_buffer,
            _nucleus_count_staging_buffer,

            picker,
            picking_renderer,

            camera_lock: None,

            selection_target_staging_buffer,
            selection_target_cached: None,

            nucleus_readback_staging_buffer,
            nucleus_readback_capacity: initial_nucleus_capacity,

            camera_distance_target: None,
            camera_zoom_user_override: false,
            camera_reset_target: None,

            // Default: match the normal render scale.
            // You can temporarily increase this for debugging (e.g. *8.0) but keep it shared.
            picking_particle_size: PARTICLE_SCALE,

            frame_times: VecDeque::with_capacity(100),
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

    fn render(
        &mut self,
        window: &Window,
        astra_debug_options: &DebugOptions,
    ) -> Result<(f32, f32), wgpu::SurfaceError> {
        // Track frame time
        let now = Instant::now();
        let frame_time = (now - self.last_frame_time).as_secs_f32() * 1000.0;
        self.last_frame_time = now;

        // Camera reset: smoothly return to origin when requested (press `C`).
        if let Some(desired) = self.camera_reset_target {
            // Exponential smoothing (frame-rate independent).
            // Higher values -> snappier reset.
            let reset_rate: f32 = 12.0;
            let dt = (frame_time * 0.001).max(0.0);
            let t = 1.0 - (-reset_rate * dt).exp();

            self.camera.target = self.camera.target.lerp(desired, t);

            // Stop resetting once we're close enough.
            if (self.camera.target - desired).length() < 0.001 {
                self.camera.target = desired;
                self.camera_reset_target = None;
            }
        }

        // Camera lock: smoothly follow the selected entity every frame.
        //
        // IMPORTANT: particles/hadrons move every simulation step, so a click-time resolved
        // `selection_target_cached` will go stale. To truly "follow", we must re-run the
        // selection-resolve compute pass regularly while locked.
        if self.camera_lock.is_some() {
            // Re-resolve selection -> target position (GPU compute), then read back vec4<f32>.
            //
            // This is intentionally "blockingly" polled for now for correctness; if it ever shows
            // up in profiles, we can switch to an async ring buffer of readbacks.
            {
                let mut resolve_encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Selection Resolve Encoder (per-frame follow)"),
                        });

                self.simulation
                    .encode_selection_resolve(&mut resolve_encoder);

                resolve_encoder.copy_buffer_to_buffer(
                    self.simulation.selection_target_buffer(),
                    0,
                    &self.selection_target_staging_buffer,
                    0,
                    16,
                );

                self.queue.submit(std::iter::once(resolve_encoder.finish()));

                let slice = self.selection_target_staging_buffer.slice(..);
                slice.map_async(wgpu::MapMode::Read, |_| {});
                // TODO: Convert to async ring buffer to avoid blocking GPU pipeline
                // See: https://toji.dev/webgpu-best-practices/buffer-uploads
                self.device
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

                    self.selection_target_cached = Some([x, y, z, w]);
                }

                self.selection_target_staging_buffer.unmap();
            }

            // If a nucleus is locked, also re-read its data every 5 frames to update the atom card
            if let Some(CameraLock::Nucleus {
                anchor_hadron_index,
            }) = self.camera_lock
            {
                if self.frame_counter % 5 == 0 {
                    self.update_selected_nucleus_data(anchor_hadron_index);
                }
            }

            if let Some(target) = self.selection_target_cached {
                // target.w = kind (0 none, 1 particle, 2 hadron, 3 nucleus)
                // NOTE: The selection-resolve pass only tells us the kind, not the exact radius.
                // We approximate desired camera distance based on kind. This can be refined later
                // by adding a resolved "size/radius" output from the compute pass.
                if target[3] != 0.0 {
                    let desired = Vec3::new(target[0], target[1], target[2]);

                    // Exponential smoothing (frame-rate independent).
                    // Higher values -> snappier camera.
                    let follow_rate: f32 = 12.0;
                    let dt = (frame_time * 0.001).max(0.0);
                    let t = 1.0 - (-follow_rate * dt).exp();

                    self.camera.target = self.camera.target.lerp(desired, t);

                    // Smooth distance: zoom in for particles/quarks; stay further for hadrons.
                    //
                    // IMPORTANT:
                    // - only set this ONCE per selection acquisition
                    // - and never re-arm it after the user manually zooms while locked
                    //   (otherwise we fight user input).
                    if self.camera_distance_target.is_none() && !self.camera_zoom_user_override {
                        let desired_distance = match target[3].round() as i32 {
                            1 => 5.0,  // particle/quark: close-up
                            2 => 15.0, // hadron shell: larger, keep more distance
                            3 => 50.0, // nucleus shell: treat like hadron for now
                            _ => self.camera.distance,
                        };
                        self.camera_distance_target = Some(desired_distance);
                    }
                }
            }
        }

        // Apply camera zoom smoothing if requested (selection or other systems).
        if let Some(desired_distance) = self.camera_distance_target {
            let zoom_rate: f32 = 10.0;
            let dt = (frame_time * 0.001).max(0.0);
            let t = 1.0 - (-zoom_rate * dt).exp();

            self.camera.distance =
                self.camera.distance + (desired_distance - self.camera.distance) * t;

            if (self.camera.distance - desired_distance).abs() < 0.01 {
                self.camera.distance = desired_distance;
                self.camera_distance_target = None;
            }
        }

        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.pop_front();
        }

        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        let fps = 1000.0 / avg_frame_time;

        self.frame_counter += 1;

        // Update physics parameters from UI
        // Pass accumulated time to shader for random seeding (using integration.z padding)
        if !self.ui_state.is_paused || self.ui_state.step_one_frame {
            self.ui_state.physics_params.integration[2] += frame_time * 0.001;
            self.ui_state.physics_params_dirty = true;
        }

        // Only update GPU buffer when params have changed
        if self.ui_state.physics_params_dirty {
            self.simulation.update_params(&self.ui_state.physics_params);
            self.ui_state.physics_params_dirty = false;
        }

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
            // TODO: Convert to async ring buffer to avoid blocking GPU pipeline
            // See: https://toji.dev/webgpu-best-practices/buffer-uploads
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
            self.ui_state.lod_bound_hadron_fade_start,
            self.ui_state.lod_bound_hadron_fade_end,
            self.ui_state.lod_bond_fade_start,
            self.ui_state.lod_bond_fade_end,
            self.ui_state.lod_quark_fade_start,
            self.ui_state.lod_quark_fade_end,
            self.ui_state.lod_nucleus_fade_start,
            self.ui_state.lod_nucleus_fade_end,
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

                // Render nuclei
                self.nucleus_renderer.render(
                    &self.device,
                    &mut render_pass,
                    &self.renderer.camera_buffer,
                    self.simulation.nucleus_buffer(),
                    self.simulation.nucleus_count_buffer(),
                    self.simulation.particle_count() / 4, // Rough estimate of max nuclei
                    self.ui_state.show_nuclei,
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

        // Render Astra GUI diagnostics panel
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Astra GUI Encoder"),
                });

            let size = window.inner_size();
            let window_size = [size.width as f32, size.height as f32];
            let astra_output =
                gui::build_diagnostics_panel(&self.ui_state, window_size, astra_debug_options);

            self.astra_renderer.render(
                &self.device,
                &self.queue,
                &mut encoder,
                &view,
                window_size[0],
                window_size[1],
                &astra_output,
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

    // Astra GUI debug options
    astra_debug_options: DebugOptions,
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
                    // Smooth reset: request a lerped return to origin instead of snapping.
                    gpu_state.camera_reset_target = Some(Vec3::ZERO);

                    // Clear selection/lock state so follow doesn't fight the reset.
                    gpu_state.camera_lock = None;
                    gpu_state.selection_target_cached = None;
                    gpu_state.camera_distance_target = None;
                    gpu_state.camera_zoom_user_override = false;
                    gpu_state.simulation.set_selected_id(0);
                }
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyP),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if let Some(_gpu_state) = &mut self.gpu_state {
                    log::debug!("picking overlay toggled: (disabled/removed)");
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

                        // IMPORTANT: winit cursor positions are in logical pixels.
                        // `inner_size()` and our swapchain/config are in physical pixels.
                        // If we don't apply the window scale factor, pick coordinates will be wrong
                        // (often "about half the time" depending on DPI, window moves, etc).
                        let scale = 1.0;
                        let physical_x = (x * scale).round();
                        let physical_y = (y * scale).round();

                        let size = window.inner_size();
                        let w = size.width.max(1) as f64;
                        let h = size.height.max(1) as f64;

                        // Convert physical window-space -> texture pixel coords.
                        // Clamp to the valid render target range.
                        let px = ((physical_x / w) * gpu_state.config.width as f64)
                            .floor()
                            .clamp(0.0, (gpu_state.config.width.saturating_sub(1)) as f64)
                            as u32;
                        let py = ((physical_y / h) * gpu_state.config.height as f64)
                            .floor()
                            .clamp(0.0, (gpu_state.config.height.saturating_sub(1)) as f64)
                            as u32;

                        log::debug!(
                            "pick click: cursor_logical=({:.1},{:.1}) scale={:.3} cursor_physical=({:.1},{:.1}) window_physical=({}x{}) cfg=({}x{}) pick_px=({}, {})",
                            x,
                            y,
                            scale,
                            physical_x,
                            physical_y,
                            size.width,
                            size.height,
                            gpu_state.config.width,
                            gpu_state.config.height,
                            px,
                            py
                        );

                        let mut encoder = gpu_state.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Picking Encoder"),
                            },
                        );

                        // Render IDs into offscreen target
                        // IMPORTANT:
                        // The visual particle shader scales billboards by `camera.particle_size * particle.data.y`.
                        // For quarks, `particle.data.y` is very small (~0.03), which makes the visible/on-screen
                        // footprint extremely tiny unless `camera.particle_size` is large enough.
                        //
                        // If the picking pass uses too small a `particle_size`, most clicks will hit background (id=0),
                        // and picking will appear angle-dependent / unreliable.
                        //
                        // Use the shared picking particle size so the click picking render matches
                        // the picking overlay visualization exactly.
                        gpu_state.picking_renderer.render(
                            &gpu_state.device,
                            &gpu_state.queue,
                            &mut encoder,
                            &gpu_state.picker.id_texture_view,
                            &gpu_state.camera,
                            gpu_state.simulation.particle_buffer(),
                            gpu_state.simulation.hadron_buffer(),
                            gpu_state.simulation.hadron_count_buffer(),
                            gpu_state.simulation.nucleus_buffer(),
                            gpu_state.simulation.nucleus_count_buffer(),
                            gpu_state.simulation.particle_count(),
                            gpu_state.simulation.particle_count(), // max_hadrons == particle_count allocation
                            gpu_state.simulation.particle_count() / 4, // match render path's rough max nuclei
                            gpu_state.picking_particle_size,
                            gpu_state.ui_state.physics_params.integration[2],
                            gpu_state.ui_state.lod_shell_fade_start,
                            gpu_state.ui_state.lod_shell_fade_end,
                            gpu_state.ui_state.lod_bound_hadron_fade_start,
                            gpu_state.ui_state.lod_bound_hadron_fade_end,
                            gpu_state.ui_state.lod_bond_fade_start,
                            gpu_state.ui_state.lod_bond_fade_end,
                            gpu_state.ui_state.lod_quark_fade_start,
                            gpu_state.ui_state.lod_quark_fade_end,
                            gpu_state.ui_state.lod_nucleus_fade_start,
                            gpu_state.ui_state.lod_nucleus_fade_end,
                        );

                        // Copy clicked pixel into staging buffer
                        gpu_state.picker.encode_read_pixel(&mut encoder, px, py);

                        gpu_state.queue.submit(std::iter::once(encoder.finish()));

                        // Map + blockingly poll for the readback (clicks are rare so this is OK).
                        let slice = gpu_state.picker.staging_buffer().slice(..);
                        slice.map_async(wgpu::MapMode::Read, |_| {});
                        // TODO: Convert to async ring buffer to avoid blocking GPU pipeline
                        // See: https://toji.dev/webgpu-best-practices/buffer-uploads
                        gpu_state
                            .device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: None,
                            })
                            .unwrap();

                        let pick = gpu_state.picker.read_mapped();
                        gpu_state.picker.staging_buffer().unmap();

                        let decoded = decode_pick_id(pick.id);
                        log::debug!(
                            "pick readback: raw_id=0x{pick_id:08x} ({pick_id}) decoded={decoded:?}",
                            pick_id = pick.id,
                            decoded = decoded
                        );

                        // Update selection ID in the simulation and resolve it to a world-space target.
                        gpu_state.simulation.set_selected_id(pick.id);
                        gpu_state.camera_lock = decoded;

                        // Reset zoom target on new selection so the initial auto-zoom runs again.
                        gpu_state.camera_distance_target = None;
                        gpu_state.camera_zoom_user_override = false;

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
                            // TODO: Convert to async ring buffer to avoid blocking GPU pipeline
                            // See: https://toji.dev/webgpu-best-practices/buffer-uploads
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

                                log::debug!(
                                    "pick resolve: target=({:.3},{:.3},{:.3}) kind_w={:.1}",
                                    x,
                                    y,
                                    z,
                                    w
                                );

                                // Do NOT snap the camera on click.
                                // We only update `selection_target_cached` here; the per-frame camera
                                // follow logic will smoothly lerp `camera.target` toward this value.
                            }

                            gpu_state.selection_target_staging_buffer.unmap();

                            // If a nucleus was selected, read back its data for the atom card UI
                            if let Some(CameraLock::Nucleus {
                                anchor_hadron_index,
                            }) = decoded
                            {
                                gpu_state.update_selected_nucleus_data(anchor_hadron_index);
                            } else {
                                // Not a nucleus selection, clear nucleus UI data
                                gpu_state.ui_state.selected_nucleus_atomic_number = None;
                                gpu_state.ui_state.selected_nucleus_proton_count = None;
                                gpu_state.ui_state.selected_nucleus_neutron_count = None;
                                gpu_state.ui_state.selected_nucleus_nucleon_count = None;
                            }
                        } else {
                            // Cleared selection
                            gpu_state.selection_target_cached = None;
                            gpu_state.camera_distance_target = None;
                            gpu_state.camera_zoom_user_override = false;
                            gpu_state.ui_state.selected_nucleus_atomic_number = None;
                            gpu_state.ui_state.selected_nucleus_proton_count = None;
                            gpu_state.ui_state.selected_nucleus_neutron_count = None;
                            gpu_state.ui_state.selected_nucleus_nucleon_count = None;
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
                    // If the user manually zooms while locked onto a selection:
                    // - cancel any in-progress auto-zoom
                    // - and prevent it from re-arming until a new selection is made.
                    if gpu_state.camera_lock.is_some() {
                        gpu_state.camera_distance_target = None;
                        gpu_state.camera_zoom_user_override = true;
                    }

                    gpu_state
                        .camera
                        .zoom(-scroll * gpu_state.camera.distance / 100.0);
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                // Handle astra-gui debug keybindings
                let handled = match key_code {
                    KeyCode::KeyM => {
                        self.astra_debug_options.show_margins =
                            !self.astra_debug_options.show_margins;
                        println!(
                            "Astra GUI Margins: {}",
                            self.astra_debug_options.show_margins
                        );
                        true
                    }
                    // Note: KeyP conflicts with picking mode toggle, so skip it
                    KeyCode::KeyB => {
                        self.astra_debug_options.show_borders =
                            !self.astra_debug_options.show_borders;
                        println!(
                            "Astra GUI Borders: {}",
                            self.astra_debug_options.show_borders
                        );
                        true
                    }
                    KeyCode::KeyO => {
                        self.astra_debug_options.show_content_area =
                            !self.astra_debug_options.show_content_area;
                        println!(
                            "Astra GUI Content area: {}",
                            self.astra_debug_options.show_content_area
                        );
                        true
                    }
                    KeyCode::KeyL => {
                        self.astra_debug_options.show_clip_rects =
                            !self.astra_debug_options.show_clip_rects;
                        println!(
                            "Astra GUI Clip rects: {}",
                            self.astra_debug_options.show_clip_rects
                        );
                        true
                    }
                    KeyCode::KeyG => {
                        self.astra_debug_options.show_gaps = !self.astra_debug_options.show_gaps;
                        println!("Astra GUI Gaps: {}", self.astra_debug_options.show_gaps);
                        true
                    }
                    KeyCode::KeyA => {
                        if self.astra_debug_options.is_enabled() {
                            self.astra_debug_options = DebugOptions::none();
                            println!("Astra GUI Debug: OFF");
                        } else {
                            self.astra_debug_options = DebugOptions::all();
                            println!("Astra GUI Debug: ALL ON");
                        }
                        true
                    }
                    _ => false,
                };

                if !handled {
                    // Fall through to other keyboard handlers
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(gpu_state)) = (&self.window, &mut self.gpu_state) {
                    match gpu_state.render(window, &self.astra_debug_options) {
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

        astra_debug_options: DebugOptions::none(),
    };

    event_loop.run_app(&mut app).unwrap();
}
