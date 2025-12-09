//! # Particle Simulation Engine
//!
//! GPU-based N-body simulation using compute shaders for the four fundamental forces.

pub mod params;
pub mod simulation;

pub use params::*;
pub use simulation::*;
