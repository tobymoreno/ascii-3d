use crate::math::{Mat4, Vec3};

use super::{BehaviorConfig, PhysicsBodyConfig};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct A3dObject {
    pub id: String,
    pub asset: AssetRef,

    #[serde(default)]
    pub transform: Transform,

    #[serde(default)]
    pub render: RenderConfig,

    #[serde(default)]
    pub behaviors: Vec<BehaviorConfig>,

    #[serde(default)]
    pub physics: Option<PhysicsBodyConfig>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssetRef {
    Mesh {
        path: String,
    },
    Word {
        path: String,
    },
    Glyph {
        path: String,
        metadata: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    #[serde(default)]
    pub position: [f32; 3],

    #[serde(default = "Transform::default_rotation")]
    pub rotation_degrees: [f32; 3],

    #[serde(default = "Transform::default_scale")]
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation_degrees: Self::default_rotation(),
            scale: Self::default_scale(),
        }
    }
}

impl Transform {
    pub const fn default_rotation() -> [f32; 3] {
        [0.0, 0.0, 0.0]
    }

    pub const fn default_scale() -> [f32; 3] {
        [1.0, 1.0, 1.0]
    }

    pub fn position_vec3(self) -> Vec3 {
        Vec3::new(self.position[0], self.position[1], self.position[2])
    }

    pub fn set_position_vec3(&mut self, position: Vec3) {
        self.position = [position.x, position.y, position.z];
    }

    pub fn matrix(self) -> Mat4 {
        Mat4::translation(self.position[0], self.position[1], self.position[2])
            * Mat4::rotation_x(self.rotation_degrees[0].to_radians())
            * Mat4::rotation_y(self.rotation_degrees[1].to_radians())
            * Mat4::rotation_z(self.rotation_degrees[2].to_radians())
            * Mat4::scale(self.scale[0], self.scale[1], self.scale[2])
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenderConfig {
    #[serde(default = "RenderConfig::default_visible")]
    pub visible: bool,

    #[serde(default)]
    pub stroke_character: Option<char>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            visible: Self::default_visible(),
            stroke_character: None,
        }
    }
}

impl RenderConfig {
    pub const fn default_visible() -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct SceneObject {
    pub id: String,
    pub asset: AssetRef,
    pub transform: Transform,
    pub render: RenderConfig,
    pub behaviors: Vec<BehaviorConfig>,
    pub physics: Option<PhysicsBodyConfig>,
}

impl From<A3dObject> for SceneObject {
    fn from(object: A3dObject) -> Self {
        Self {
            id: object.id,
            asset: object.asset,
            transform: object.transform,
            render: object.render,
            behaviors: object.behaviors,
            physics: object.physics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Transform;

    #[test]
    fn transform_defaults_to_identity_values() {
        let transform = Transform::default();

        assert_eq!(transform.position, [0.0, 0.0, 0.0]);
        assert_eq!(transform.rotation_degrees, [0.0, 0.0, 0.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }
}
