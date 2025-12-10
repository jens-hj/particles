//! Physical constants for particle simulation
//!
//! These are simplified constants scaled for real-time visualization while
//! maintaining relative physical relationships.

/// Speed of light in simulation units
pub const C: f32 = 1.0;

/// Gravitational constant (extremely weak, may not be visible at particle scale)
pub const G: f32 = 6.674e-11;

/// Coulomb constant for electromagnetic force (k = 1/(4πε₀))
/// Scaled for visualization
pub const K_ELECTRIC: f32 = 8.99;

/// Strong force coupling constant
/// In QCD this is αs ≈ 0.1-1.0 depending on energy scale
pub const ALPHA_STRONG: f32 = 0.3;

/// Strong force confinement strength (linear term in Cornell potential)
/// This creates the "string" that confines quarks
pub const STRONG_CONFINEMENT: f32 = 1.0;

/// Strong force short-range strength (Coulomb-like term)
pub const STRONG_SHORT_RANGE: f32 = 0.5;

/// Cutoff range for strong force
pub const STRONG_RANGE: f32 = 3.0;

/// Strength of short-range repulsion (hard core)
pub const CORE_REPULSION: f32 = 150.0;

/// Radius for hard-core repulsion
pub const CORE_RADIUS: f32 = 0.35;

/// Weak force coupling constant (much weaker than electromagnetic)
pub const G_WEAK: f32 = 1.0e-5;

/// Weak force range (very short, ~10^-18 m in reality)
/// In simulation units, this limits weak force to very close range
pub const WEAK_FORCE_RANGE: f32 = 0.1;

/// Elementary charge (for quarks: +2/3 or -1/3, for electrons: -1)
pub const E_CHARGE: f32 = 1.0;

/// Quark mass (simplified - in reality up ~2.3 MeV, down ~4.8 MeV)
/// Using simulation-friendly values (scaled: 1.0 = 1000 MeV approx)
pub const QUARK_UP_MASS: f32 = 0.0023;
pub const QUARK_DOWN_MASS: f32 = 0.0048;

/// Electron mass (in simulation units)
/// ~0.511 MeV
pub const ELECTRON_MASS: f32 = 0.000511;

/// Proton mass (should emerge from quark binding, but used for reference)
/// ~938 MeV
pub const PROTON_MASS: f32 = 0.938;

/// Neutron mass
/// ~940 MeV
pub const NEUTRON_MASS: f32 = 0.940;

/// Damping factor for numerical stability
pub const DAMPING: f32 = 0.995;

/// Softening parameter to prevent singularities at r→0
pub const SOFTENING: f32 = 0.01;

// Particle sizes for visualization (scaled for visibility)
/// Quark size (very small, ~10^-18 m in reality)
pub const QUARK_SIZE: f32 = 0.03;

/// Electron size (~10^-15 m classical radius)
pub const ELECTRON_SIZE: f32 = 0.03;

/// Gluon mass (simulation units)
/// Theoretically massless, but small non-zero value for integration stability
pub const GLUON_MASS: f32 = 0.0001;

/// Gluon size (force carrier, similar to photons)
/// Point-like particle
pub const GLUON_SIZE: f32 = 0.04;

/// Proton size (~10^-15 m, 1 femtometer)
pub const PROTON_SIZE: f32 = 2.0;

/// Neutron size (similar to proton)
pub const NEUTRON_SIZE: f32 = 2.0;
