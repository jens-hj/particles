// Compute shader: resolve a picked packed ID into a world-space target position.
//
// This is intended to be run after the GPU picking pass has produced a packed u32 ID.
// The CPU writes that ID into `selection.id`, then dispatches this shader with 1 invocation.
// The shader writes the selected entity center into `selection_target.target`.
//
// ID encoding convention (must match picking shader):
// - 0                          => no selection
// - (particle_index + 1)       => particle selection
// - 0x80000000 | (hadron_index + 1) => hadron selection
//
// Output encoding:
// - selection_target.target.xyz = selected world-space center
// - selection_target.target.w   = kind (0.0 = none, 1.0 = particle, 2.0 = hadron)
//
// Notes:
// - Particles are addressed directly by index.
// - Hadrons resolve to the hadron center (`hadron.center.xyz`).
// - We validate indices and invalid hadron slots (`type_id == 0xFFFFFFFFu`).

struct Particle {
    position: vec4<f32>,        // xyz = position, w = particle_type (as f32)
    velocity: vec4<f32>,        // xyz = velocity, w = mass
    data: vec4<f32>,            // x = charge, y = size, z/w = padding
    color_and_flags: vec4<u32>, // x = color_charge, y = flags, z = hadron_id (1-indexed), w = padding
}

struct Hadron {
    indices_type: vec4<u32>, // x=p1, y=p2, z=p3, w=type_id
    center: vec4<f32>,       // xyz = center, w = radius
    velocity: vec4<f32>,     // xyz = velocity, w = padding
}

struct Selection {
    id: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

struct SelectionTarget {
    value: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> selection: Selection;

@group(0) @binding(1)
var<storage, read> particles: array<Particle>;

@group(0) @binding(2)
var<storage, read> hadrons: array<Hadron>;

@group(0) @binding(3)
var<storage, read_write> selection_target: SelectionTarget;

const KIND_NONE: f32 = 0.0;
const KIND_PARTICLE: f32 = 1.0;
const KIND_HADRON: f32 = 2.0;

fn write_none() {
    selection_target.value = vec4<f32>(0.0, 0.0, 0.0, KIND_NONE);
}

@compute @workgroup_size(1)
fn main() {
    let raw_id = selection.id;

    if (raw_id == 0u) {
        write_none();
        return;
    }

    let is_hadron = (raw_id & 0x80000000u) != 0u;
    let idx_1 = raw_id & 0x7FFFFFFFu; // 1-indexed payload

    if (idx_1 == 0u) {
        write_none();
        return;
    }

    let idx0 = idx_1 - 1u;

    if (!is_hadron) {
        // Particle selection
        let n = arrayLength(&particles);
        if (idx0 >= n) {
            write_none();
            return;
        }

        let p = particles[idx0];
        selection_target.value = vec4<f32>(p.position.xyz, KIND_PARTICLE);
        return;
    }

    // Hadron selection
    let h_n = arrayLength(&hadrons);
    if (idx0 >= h_n) {
        write_none();
        return;
    }

    let h = hadrons[idx0];

    // Invalid slot sentinel
    if (h.indices_type.w == 0xFFFFFFFFu) {
        write_none();
        return;
    }

    selection_target.value = vec4<f32>(h.center.xyz, KIND_HADRON);
}
