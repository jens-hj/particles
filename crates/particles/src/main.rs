use bevy::prelude::*;
use bevy_catppuccin::CatppuccinPlugin;
use debug_ui::DebugUiPlugin;
use orbit_camera::OrbitCameraPlugin;
use particles_core::ParticlesCorePlugin;
use particles_render::ParticlesRenderPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Particle Simulation".to_string(),
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CatppuccinPlugin)
        .add_plugins(ParticlesCorePlugin)
        .add_plugins(ParticlesRenderPlugin)
        .add_plugins(OrbitCameraPlugin)
        .add_plugins(DebugUiPlugin)
        .run();
}
