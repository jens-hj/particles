use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};

/// Marker component for orbit camera
#[derive(Component)]
pub struct OrbitCamera {
    /// Point the camera orbits around
    pub target: Vec3,
    /// Distance from target
    pub radius: f32,
    /// Rotation as a quaternion (avoids gimbal lock)
    pub rotation: Quat,
    /// Zoom speed
    pub zoom_speed: f32,
    /// Rotation speed
    pub rotation_speed: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 500.0,
            rotation: Quat::IDENTITY,
            zoom_speed: 50.0,
            rotation_speed: 0.005,
        }
    }
}

impl OrbitCamera {
    pub fn new(target: Vec3, radius: f32) -> Self {
        Self {
            target,
            radius,
            ..default()
        }
    }

    /// Calculate the camera position based on orbit parameters
    fn calculate_position(&self) -> Vec3 {
        // Start with a base offset (camera looking from +Z toward origin)
        let base_offset = Vec3::new(0.0, 0.0, self.radius);

        // Apply the rotation quaternion to the offset
        let rotated_offset = self.rotation * base_offset;

        // Add to target position
        self.target + rotated_offset
    }
}

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (orbit_camera_control, update_camera_transform));
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        OrbitCamera::new(Vec3::ZERO, 800.0),
    ));
}

fn orbit_camera_control(
    mut query: Query<&mut OrbitCamera>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
) {
    let mut orbit_camera = match query.single_mut() {
        Ok(cam) => cam,
        Err(_) => return,
    };

    // Handle mouse wheel for zooming
    for wheel in mouse_wheel.read() {
        orbit_camera.radius -= wheel.y * orbit_camera.zoom_speed;
        orbit_camera.radius = orbit_camera.radius.max(10.0);
    }

    // Handle right mouse button for rotation
    if mouse_button.pressed(MouseButton::Right) {
        for motion in mouse_motion.read() {
            let delta_x = -motion.delta.x * orbit_camera.rotation_speed;
            let delta_y = -motion.delta.y * orbit_camera.rotation_speed;

            // Rotate around global Y axis for horizontal movement (yaw)
            let yaw_rotation = Quat::from_rotation_y(delta_x);

            // Rotate around camera's local X axis for vertical movement (pitch)
            // Get the camera's right vector by rotating the global X axis
            let right = orbit_camera.rotation * Vec3::X;
            let pitch_rotation = Quat::from_axis_angle(right, delta_y);

            // Apply rotations: first pitch (local), then yaw (global)
            orbit_camera.rotation = yaw_rotation * pitch_rotation * orbit_camera.rotation;

            // Normalize to prevent drift over time
            orbit_camera.rotation = orbit_camera.rotation.normalize();
        }
    } else {
        // Clear the event reader to avoid accumulation
        mouse_motion.clear();
    }
}

fn update_camera_transform(
    mut query: Query<(&OrbitCamera, &mut Transform), With<Camera>>,
) {
    for (orbit_camera, mut transform) in query.iter_mut() {
        let position = orbit_camera.calculate_position();
        transform.translation = position;

        // Calculate camera direction from position to target
        let direction = (orbit_camera.target - position).normalize();

        // Derive up vector from the quaternion's local Y axis
        // This ensures the up vector is never parallel to the view direction
        let up = orbit_camera.rotation * Vec3::Y;

        // Set rotation using direction and the quaternion-derived up vector
        transform.look_to(direction, up);
    }
}
