use bevy::prelude::*;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy_catppuccin::CatppuccinTheme;
use particles_core::ParticleBuffer;

pub struct ParticlesRenderPlugin;

impl Plugin for ParticlesRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_clear_color)
            .add_systems(Update, render_particles);
    }
}

fn setup_clear_color(mut commands: Commands, theme: Res<CatppuccinTheme>) {
    commands.insert_resource(ClearColor(theme.flavor.base));
}

/// Marker component for particle render entities
#[derive(Component)]
struct ParticleRender {
    index: usize,
}

fn render_particles(
    mut commands: Commands,
    particle_buffer: Res<ParticleBuffer>,
    query: Query<(Entity, &ParticleRender)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<CatppuccinTheme>,
) {
    // Clean up old particle entities if particle count changed
    let current_entities = query.iter().count();
    if current_entities != particle_buffer.particle_count() {
        for (entity, _) in query.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new particle render entities with Catppuccin colors
        let sphere_mesh = meshes.add(Sphere::new(2.0));

        // Create materials with different Catppuccin colors
        let colors = [
            theme.flavor.mauve,
            theme.flavor.lavender,
            theme.flavor.blue,
            theme.flavor.sky,
            theme.flavor.teal,
            theme.flavor.green,
            theme.flavor.yellow,
            theme.flavor.peach,
            theme.flavor.maroon,
            theme.flavor.red,
        ];

        for (index, particle) in particle_buffer.particles.iter().enumerate() {
            let color = colors[index % colors.len()];
            let material = materials.add(StandardMaterial {
                base_color: color,
                unlit: true, // Make particles unlit for better visibility
                ..default()
            });

            commands.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(particle.position),
                ParticleRender { index },
            ));
        }
    } else {
        // Update existing particle positions
        for (entity, particle_render) in query.iter() {
            if let Some(particle) = particle_buffer.particles.get(particle_render.index) {
                commands.entity(entity).insert(
                    Transform::from_translation(particle.position)
                );
            }
        }
    }
}
