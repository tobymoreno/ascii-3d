#![allow(dead_code)]

use crate::math::Vec3;

/// Camera projection mode, following raylib-style terminology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

/// A raylib-inspired 3D camera definition.
///
/// Camera space is not a separate world. It is the camera-relative coordinate
/// system derived from this camera's position, target, and up vector.
#[derive(Debug, Clone, Copy)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fovy_degrees: f32,
    pub projection: CameraProjection,
    pub near: f32,
    pub far: f32,
}

impl Camera3D {
    pub fn new(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fovy_degrees: f32,
        projection: CameraProjection,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            fovy_degrees,
            projection,
            near,
            far,
        }
    }

    /// Starting camera for the larger world-space roadmap.
    ///
    /// The exact pose is only a starter; later pan/tilt/dolly controls will
    /// navigate through world space.
    pub fn default_world_camera() -> Self {
        Self::new(
            Vec3::new(0.0, 0.0, -8.0),
            Vec3::new(4.5, 0.5, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            60.0,
            CameraProjection::Perspective,
            0.1,
            100.0,
        )
    }

    pub fn set_projection(&mut self, projection: CameraProjection) {
        self.projection = projection;
    }
}

#[cfg(test)]
mod tests {
    use super::{Camera3D, CameraProjection};

    #[test]
    fn default_camera_uses_perspective_projection() {
        let camera = Camera3D::default_world_camera();

        assert_eq!(camera.projection, CameraProjection::Perspective);
        assert_eq!(camera.fovy_degrees, 60.0);
        assert_eq!(camera.near, 0.1);
        assert_eq!(camera.far, 100.0);
    }

    #[test]
    fn camera_projection_can_switch_to_orthographic() {
        let mut camera = Camera3D::default_world_camera();

        camera.set_projection(CameraProjection::Orthographic);

        assert_eq!(camera.projection, CameraProjection::Orthographic);
    }
}
