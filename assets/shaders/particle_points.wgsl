// Point-based particle shader
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_functions::mesh_position_local_to_clip

struct ParticlePointMaterial {
    color: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: ParticlePointMaterial;

@vertex
fn vertex(
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    // Transform position directly to clip space
    var model = mesh_functions::get_world_from_local(instance_index);
    out.position = mesh_position_local_to_clip(model, vec4<f32>(position, 1.0));
    out.world_position = vec4<f32>(position, 1.0);

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}
