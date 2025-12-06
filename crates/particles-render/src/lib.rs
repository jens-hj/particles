use bevy::prelude::*;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use particles_core::ParticleBuffer;

pub struct ParticlesRenderPlugin;

impl Plugin for ParticlesRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, render_particles);
    }
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
) {
    // Clean up old particle entities if particle count changed
    let current_entities = query.iter().count();
    if current_entities != particle_buffer.particle_count() {
        for (entity, _) in query.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new particle render entities
        let sphere_mesh = meshes.add(Sphere::new(2.0));
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true, // Make particles unlit for better visibility
            ..default()
        });

        for (index, particle) in particle_buffer.particles.iter().enumerate() {
            commands.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(material.clone()),
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
