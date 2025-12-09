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
/// Aligned for WGSL struct compatibility
#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
pub struct Particle {
    /// Position in 3D space
    pub position: [f32; 3],
    /// Particle type (as u32, maps to ParticleType enum)
    pub particle_type: u32,

    /// Velocity vector
    pub velocity: [f32; 3],
    /// Mass of the particle
    pub mass: f32,

    /// Electric charge (in units of elementary charge e)
    pub charge: f32,
    /// Color charge (for quarks, 0-5 maps to ColorCharge enum)
    pub color_charge: u32,
    /// Flags and additional properties
    pub flags: u32,
    /// Size for rendering
    pub size: f32,
}

impl Particle {
    /// Create a new up quark
    pub fn new_up_quark(position: Vec3, color: ColorCharge) -> Self {
        Self {
            position: position.to_array(),
            particle_type: ParticleType::QuarkUp as u32,
            velocity: [0.0; 3],
            mass: crate::constants::QUARK_UP_MASS,
            charge: 2.0 / 3.0, // Up quark has +2/3 e charge
            color_charge: color as u32,
            flags: 0,
            size: crate::constants::QUARK_SIZE,
        }
    }
    
    /// Create a new down quark
    pub fn new_down_quark(position: Vec3, color: ColorCharge) -> Self {
        Self {
            position: position.to_array(),
            particle_type: ParticleType::QuarkDown as u32,
            velocity: [0.0; 3],
            mass: crate::constants::QUARK_DOWN_MASS,
            charge: -1.0 / 3.0, // Down quark has -1/3 e charge
            color_charge: color as u32,
            flags: 0,
            size: crate::constants::QUARK_SIZE,
        }
    }
    
    /// Create a new electron
    pub fn new_electron(position: Vec3) -> Self {
        Self {
            position: position.to_array(),
            particle_type: ParticleType::Electron as u32,
            velocity: [0.0; 3],
            mass: crate::constants::ELECTRON_MASS,
            charge: -1.0, // Electron has -1 e charge
            color_charge: 0, // Electrons don't have color charge
            flags: 0,
            size: crate::constants::ELECTRON_SIZE,
        }
    }
    
    /// Get particle type
    pub fn get_type(&self) -> Option<ParticleType> {
        match self.particle_type {
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
        match self.color_charge {
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
