//! Particle types and properties for fundamental particle simulation

use bytemuck::Zeroable;
use glam::Vec3;

/// Color charge for quarks (quantum chromodynamics)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCharge {
    Red = 0,
    Green = 1,
    Blue = 2,
    AntiRed = 3,
    AntiGreen = 4,
    AntiBlue = 5,
}

/// Quark flavors
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarkFlavor {
    Up = 0,
    Down = 1,
    // Future: Charm, Strange, Top, Bottom
}

/// Fundamental particle types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    QuarkUp = 0,
    QuarkDown = 1,
    Electron = 2,
    Gluon = 3,
    // Future composite particles (these emerge from quark binding)
    Proton = 4,
    Neutron = 5,
}

/// GPU-compatible particle structure
/// Using vec4 for ALL fields to ensure perfect alignment with WGSL (16-byte aligned)
#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
pub struct Particle {
    /// Position (xyz) and particle type (w component)
    pub position: [f32; 4],

    /// Velocity (xyz) and mass (w component)
    pub velocity: [f32; 4],

    /// Data: x = charge, y = size, z/w = unused padding
    pub data: [f32; 4],

    /// Color and flags: x = color_charge, y = flags, z/w = unused padding
    pub color_and_flags: [u32; 4],
}

impl Particle {
    /// Create a new up quark
    pub fn new_up_quark(position: Vec3, color: ColorCharge) -> Self {
        let pos = position.to_array();
        Self {
            position: [pos[0], pos[1], pos[2], ParticleType::QuarkUp as u32 as f32],
            velocity: [0.0, 0.0, 0.0, crate::constants::QUARK_UP_MASS],
            data: [2.0 / 3.0, crate::constants::QUARK_SIZE, 0.0, 0.0], // charge, size, padding
            color_and_flags: [color as u32, 0, 0, 0], // color_charge, flags, padding
        }
    }

    /// Create a new down quark
    pub fn new_down_quark(position: Vec3, color: ColorCharge) -> Self {
        let pos = position.to_array();
        Self {
            position: [
                pos[0],
                pos[1],
                pos[2],
                ParticleType::QuarkDown as u32 as f32,
            ],
            velocity: [0.0, 0.0, 0.0, crate::constants::QUARK_DOWN_MASS],
            data: [-1.0 / 3.0, crate::constants::QUARK_SIZE, 0.0, 0.0], // charge, size, padding
            color_and_flags: [color as u32, 0, 0, 0], // color_charge, flags, padding
        }
    }

    /// Create a new electron
    pub fn new_electron(position: Vec3) -> Self {
        let pos = position.to_array();
        Self {
            position: [pos[0], pos[1], pos[2], ParticleType::Electron as u32 as f32],
            velocity: [0.0, 0.0, 0.0, crate::constants::ELECTRON_MASS],
            data: [-1.0, crate::constants::ELECTRON_SIZE, 0.0, 0.0], // charge, size, padding
            color_and_flags: [0, 0, 0, 0], // electrons don't have color charge
        }
    }

    /// Get particle type (stored in position.w)
    pub fn get_type(&self) -> Option<ParticleType> {
        match self.position[3] as u32 {
            0 => Some(ParticleType::QuarkUp),
            1 => Some(ParticleType::QuarkDown),
            2 => Some(ParticleType::Electron),
            3 => Some(ParticleType::Gluon),
            4 => Some(ParticleType::Proton),
            5 => Some(ParticleType::Neutron),
            _ => None,
        }
    }

    /// Get color charge
    pub fn get_color(&self) -> Option<ColorCharge> {
        match self.color_and_flags[0] {
            0 => Some(ColorCharge::Red),
            1 => Some(ColorCharge::Green),
            2 => Some(ColorCharge::Blue),
            3 => Some(ColorCharge::AntiRed),
            4 => Some(ColorCharge::AntiGreen),
            5 => Some(ColorCharge::AntiBlue),
            _ => None,
        }
    }
}

// Safety: Particle is repr(C) and all fields are Pod-safe types (f32, u32)
// The padding fields are explicitly zeroed and don't affect safety
unsafe impl bytemuck::Pod for Particle {}

/// Hadron structure for visualization
/// Represents a bound state of quarks (Baryon or Meson)
#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
pub struct Hadron {
    /// Indices of constituent particles (up to 3)
    pub p1: u32,
    pub p2: u32,
    pub p3: u32,

    /// Type of hadron (0=Meson, 1=Proton, 2=Neutron, etc.)
    pub type_id: u32,

    /// Center of mass (xyz) and radius (w)
    pub center: [f32; 4],
}

unsafe impl bytemuck::Pod for Hadron {}
