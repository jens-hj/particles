// Compute shader for particle updates
// For now this is a no-op - just reads and writes the same data

struct Particle {
    position: vec4<f32>,  // xyz = position, w = padding for alignment
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    // Bounds check
    if index >= arrayLength(&particles) {
        return;
    }

    // No-op for now - just read and write the same data
    let particle = particles[index];
    particles[index] = particle;

    // Future: Add velocity, forces, collisions, etc.
}
