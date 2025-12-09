//! Force calculations for the four fundamental forces
//! 
//! NOTE: These are reference implementations for documentation and testing.
//! The actual simulation uses GPU compute shaders for performance.

use glam::Vec3;
use crate::constants::*;
use crate::particle::{Particle, ParticleType, ColorCharge};

/// Calculate gravitational force between two particles
/// F = G * m1 * m2 / r²
pub fn gravitational_force(p1: &Particle, p2: &Particle) -> Vec3 {
    let pos1 = Vec3::from_array([p1.position[0], p1.position[1], p1.position[2]]);
    let pos2 = Vec3::from_array([p2.position[0], p2.position[1], p2.position[2]]);
    let r_vec = pos2 - pos1;
    let r = r_vec.length() + SOFTENING;

    if r < SOFTENING * 2.0 {
        return Vec3::ZERO;
    }

    let m1 = p1.velocity[3];  // mass stored in velocity.w
    let m2 = p2.velocity[3];
    let force_magnitude = G * m1 * m2 / (r * r);
    r_vec.normalize() * force_magnitude
}

/// Calculate electromagnetic force between two charged particles
/// F = k * q1 * q2 / r²
pub fn electromagnetic_force(p1: &Particle, p2: &Particle) -> Vec3 {
    let pos1 = Vec3::from_array([p1.position[0], p1.position[1], p1.position[2]]);
    let pos2 = Vec3::from_array([p2.position[0], p2.position[1], p2.position[2]]);
    let r_vec = pos2 - pos1;
    let r = r_vec.length() + SOFTENING;

    if r < SOFTENING * 2.0 {
        return Vec3::ZERO;
    }

    let force_magnitude = K_ELECTRIC * p1.data[0] * p2.data[0] / (r * r); // charge in data[0]
    r_vec.normalize() * force_magnitude
}

/// Check if two color charges attract (color + anti-color, or forming color-neutral)
fn color_charges_attract(c1: Option<ColorCharge>, c2: Option<ColorCharge>) -> bool {
    match (c1, c2) {
        (Some(ColorCharge::Red), Some(ColorCharge::AntiRed)) => true,
        (Some(ColorCharge::Green), Some(ColorCharge::AntiGreen)) => true,
        (Some(ColorCharge::Blue), Some(ColorCharge::AntiBlue)) => true,
        (Some(ColorCharge::AntiRed), Some(ColorCharge::Red)) => true,
        (Some(ColorCharge::AntiGreen), Some(ColorCharge::Green)) => true,
        (Some(ColorCharge::AntiBlue), Some(ColorCharge::Blue)) => true,
        // Different colors also attract (to form color-neutral bound states)
        (Some(c1), Some(c2)) if c1 != c2 => true,
        _ => false,
    }
}

/// Calculate strong force between quarks (Cornell potential)
/// V(r) = -a/r + br (combines Coulomb-like and confinement terms)
/// F = -dV/dr = -a/r² + b
pub fn strong_force(p1: &Particle, p2: &Particle) -> Vec3 {
    // Strong force only affects quarks
    let p1_is_quark = matches!(p1.get_type(), Some(ParticleType::QuarkUp) | Some(ParticleType::QuarkDown));
    let p2_is_quark = matches!(p2.get_type(), Some(ParticleType::QuarkUp) | Some(ParticleType::QuarkDown));

    if !p1_is_quark || !p2_is_quark {
        return Vec3::ZERO;
    }

    let pos1 = Vec3::from_array([p1.position[0], p1.position[1], p1.position[2]]);
    let pos2 = Vec3::from_array([p2.position[0], p2.position[1], p2.position[2]]);
    let r_vec = pos2 - pos1;
    let r = r_vec.length() + SOFTENING;

    if r < SOFTENING * 2.0 {
        return Vec3::ZERO;
    }

    let c1 = p1.get_color();
    let c2 = p2.get_color();

    // Color factor: quarks with complementary colors attract
    let color_factor = if color_charges_attract(c1, c2) { -1.0 } else { 1.0 };

    // Cornell potential derivative:
    // Short range: Coulomb-like attraction (-a/r²)
    // Long range: Linear confinement (constant force)
    let short_range_force = STRONG_SHORT_RANGE / (r * r);
    let confinement_force = STRONG_CONFINEMENT;

    let force_magnitude = color_factor * (short_range_force + confinement_force);

    r_vec.normalize() * force_magnitude
}

/// Calculate weak force (very short range, Yukawa potential)
/// F = g * exp(-r/λ) / r²
pub fn weak_force(p1: &Particle, p2: &Particle) -> Vec3 {
    let pos1 = Vec3::from_array([p1.position[0], p1.position[1], p1.position[2]]);
    let pos2 = Vec3::from_array([p2.position[0], p2.position[1], p2.position[2]]);
    let r_vec = pos2 - pos1;
    let r = r_vec.length() + SOFTENING;

    if r < SOFTENING * 2.0 || r > WEAK_FORCE_RANGE * 3.0 {
        return Vec3::ZERO;
    }

    // Yukawa potential derivative (very short range)
    let exp_term = (-r / WEAK_FORCE_RANGE).exp();
    let force_magnitude = G_WEAK * exp_term / (r * r);

    r_vec.normalize() * force_magnitude
}

/// Calculate total force on a particle from another particle
pub fn total_force(p1: &Particle, p2: &Particle) -> Vec3 {
    let f_gravity = gravitational_force(p1, p2);
    let f_em = electromagnetic_force(p1, p2);
    let f_strong = strong_force(p1, p2);
    let f_weak = weak_force(p1, p2);
    
    f_gravity + f_em + f_strong + f_weak
}
