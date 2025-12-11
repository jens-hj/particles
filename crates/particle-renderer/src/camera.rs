//! Camera system for 3D visualization

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

/// Camera uniform for GPU
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub position: [f32; 3],
    pub particle_size: f32,
    pub time: f32,
    pub lod_shell_fade_start: f32,
    pub lod_shell_fade_end: f32,
    pub lod_bond_fade_start: f32,
    pub lod_bond_fade_end: f32,
    pub lod_quark_fade_start: f32,
    pub lod_quark_fade_end: f32,
    pub _padding: f32,
}

/// Camera for 3D scene navigation
pub struct Camera {
    pub distance: f32,
    pub rotation: Quat,
    pub target: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        let rotation = Quat::from_rotation_x(0.3);

        Self {
            distance: 200.0,
            rotation,
            target: Vec3::ZERO,
            aspect: width as f32 / height as f32,
            fovy: 45.0_f32.to_radians(),
            znear: 0.1,
            zfar: 100000.0,
        }
    }

    pub fn position(&self) -> Vec3 {
        let offset = self.rotation * Vec3::new(0.0, 0.0, self.distance);
        self.target + offset
    }

    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        let up = self.rotation * Vec3::Y;
        let yaw_rotation = Quat::from_axis_angle(up, delta_x);

        let right = self.rotation * Vec3::X;
        let pitch_rotation = Quat::from_axis_angle(right, -delta_y);

        self.rotation = yaw_rotation * pitch_rotation * self.rotation;
        self.rotation = self.rotation.normalize();
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance + delta).clamp(1.0, 50000.0);
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let position = self.position();
        let rotation_matrix = Mat4::from_quat(self.rotation.conjugate());
        let translation_matrix = Mat4::from_translation(-position);
        let view = rotation_matrix * translation_matrix;
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }

    pub fn to_uniform(
        &self,
        particle_size: f32,
        time: f32,
        lod_shell_fade_start: f32,
        lod_shell_fade_end: f32,
        lod_bond_fade_start: f32,
        lod_bond_fade_end: f32,
        lod_quark_fade_start: f32,
        lod_quark_fade_end: f32,
    ) -> CameraUniform {
        CameraUniform {
            view_proj: self.build_view_projection_matrix().to_cols_array_2d(),
            position: self.position().to_array(),
            particle_size,
            time,
            lod_shell_fade_start,
            lod_shell_fade_end,
            lod_bond_fade_start,
            lod_bond_fade_end,
            lod_quark_fade_start,
            lod_quark_fade_end,
            _padding: 0.0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
}
