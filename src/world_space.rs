#![allow(dead_code)]

use crate::math::Vec3;

/// The global 3D Cartesian space that scene objects live in.
///
/// This is intentionally separate from camera space. 3D objects, words,
/// glyphs, meshes, axes, and future primitives should be positioned here.
#[derive(Debug, Clone, Copy)]
pub struct WorldSpace3D {
    pub name: &'static str,
    pub origin: Vec3,
    pub axis_length: f32,
}

impl WorldSpace3D {
    pub fn new(name: &'static str, origin: Vec3, axis_length: f32) -> Self {
        Self {
            name,
            origin,
            axis_length,
        }
    }

    pub fn default_world() -> Self {
        Self::new("world", Vec3::new(0.0, 0.0, 0.0), 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::WorldSpace3D;

    #[test]
    fn default_world_space_has_cartesian_origin() {
        let world = WorldSpace3D::default_world();

        assert_eq!(world.name, "world");
        assert_eq!(world.origin.x, 0.0);
        assert_eq!(world.origin.y, 0.0);
        assert_eq!(world.origin.z, 0.0);
        assert_eq!(world.axis_length, 10.0);
    }
}
