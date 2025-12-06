use bevy::prelude::*;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy_catppuccin::CatppuccinTheme;
use particles_core::ParticleBuffer;

pub struct ParticlesRenderPlugin;

impl Plugin for ParticlesRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_clear_color, setup_shared_resources))
            .add_systems(Update, render_particles);
    }
}

fn setup_clear_color(mut commands: Commands, theme: Res<CatppuccinTheme>) {
    commands.insert_resource(ClearColor(theme.flavor.base));
}

/// Shared rendering resources to minimize GPU allocations
#[derive(Resource)]
struct ParticleRenderResources {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}

fn setup_shared_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<CatppuccinTheme>,
) {
    // Create single shared mesh for all particles
    let mesh = meshes.add(Sphere::new(2.0).mesh().ico(2).unwrap());

    // Create 10 shared materials with Catppuccin colors
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

    let material_handles: Vec<_> = colors
        .iter()
        .map(|&color| {
            materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                ..default()
            })
        })
        .collect();

    commands.insert_resource(ParticleRenderResources {
        mesh,
        materials: material_handles,
    });
}

/// Marker component for particle render entities
#[derive(Component)]
struct ParticleRender {
    index: usize,
}

fn render_particles(
    mut commands: Commands,
    particle_buffer: Res<ParticleBuffer>,
    resources: Res<ParticleRenderResources>,
    query: Query<(Entity, &ParticleRender)>,
) {
    let current_entities = query.iter().count();

    // Spawn or despawn entities if particle count changed
    if current_entities != particle_buffer.particle_count() {
        // Despawn old entities
        for (entity, _) in query.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new entities with shared resources
        for (index, particle) in particle_buffer.particles.iter().enumerate() {
            let material = resources.materials[index % resources.materials.len()].clone();

            commands.spawn((
                Mesh3d(resources.mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(particle.position),
                ParticleRender { index },
            ));
        }
    } else {
        // Update positions of existing particles
        for (entity, particle_render) in query.iter() {
            if let Some(particle) = particle_buffer.particles.get(particle_render.index) {
                commands
                    .entity(entity)
                    .insert(Transform::from_translation(particle.position));
            }
        }
    }
}
