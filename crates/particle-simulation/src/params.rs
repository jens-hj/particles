//! Physics parameters for runtime tuning

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PhysicsParams {
    // Group 1: Fundamental constants
    // x: G, y: K_electric, z: G_weak, w: weak_force_range
    pub constants: [f32; 4],

    // Group 2: Strong force
    // x: strong_short_range, y: strong_confinement, z: strong_range, w: padding
    pub strong_force: [f32; 4],

    // Group 3: Repulsion & Limits
    // x: core_repulsion, y: core_radius, z: softening, w: max_force
    pub repulsion: [f32; 4],

    // Group 4: Integration
    // x: dt, y: damping, z: time/seed, w: nucleon_damping
    pub integration: [f32; 4],

    // Group 5: Nucleon Physics
    // x: binding_strength, y: binding_range, z: exclusion_strength, w: exclusion_radius
    pub nucleon: [f32; 4],

    // Group 6: Electron Physics
    // x: exclusion_strength, y: exclusion_radius, z: padding, w: padding
    pub electron: [f32; 4],

    // Group 7: Hadron Formation & Confinement
    // x: binding_distance, y: breakup_distance, z: confinement_range_mult, w: confinement_strength_mult
    pub hadron: [f32; 4],
}

impl Default for PhysicsParams {
    fn default() -> Self {
        Self {
            constants: [
                6.674e-11, // G
                8.99,      // K_electric
                1.0e-5,    // G_weak
                0.1,       // weak_force_range
            ],
            strong_force: [
                0.5, // strong_short_range
                1.0, // strong_confinement
                3.0, // strong_range
                0.0, // padding
            ],
            repulsion: [
                200.0, // core_repulsion
                0.35,  // core_radius
                0.01,  // softening
                50.0,  // max_force
            ],
            integration: [
                0.0005, // dt
                0.995,  // damping
                0.0,    // time/seed
                1.5,    // nucleon_damping
            ],
            nucleon: [
                100.0, // binding_strength
                2.2,   // binding_range
                130.0, // exclusion_strength
                1.3,   // exclusion_radius
            ],
            electron: [
                100.0, // exclusion_strength
                2.0,   // exclusion_radius
                0.0,   // padding
                0.0,   // padding
            ],
            hadron: [
                0.8, // binding_distance (quarks form hadrons when closer than this)
                1.0, // breakup_distance (hadrons break when quarks exceed this distance)
                5.0, // confinement_range_mult (range multiplier for free quarks, default 1.2x)
                2.0, // confinement_strength_mult (strength multiplier for free quarks, default 1.5x)
            ],
        }
    }
}
