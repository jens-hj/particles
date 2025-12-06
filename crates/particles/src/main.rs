use bevy::prelude::*;
use orbit_camera::OrbitCameraPlugin;
use particles_core::ParticlesCorePlugin;
use particles_render::ParticlesRenderPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Particle Simulation".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ParticlesCorePlugin)
        .add_plugins(ParticlesRenderPlugin)
        .add_plugins(OrbitCameraPlugin)
        .run();
}
