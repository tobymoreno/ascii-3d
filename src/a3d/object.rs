use std::path::PathBuf;

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

    #[serde(default)]
    pub editor_composite: bool,
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
    Group {
        path: String,
    },
    GeoJsonMap {
        path: String,

        #[serde(default = "default_geo_json_map_radius_scale")]
        radius_scale: f32,
    },
}

const fn default_geo_json_map_radius_scale() -> f32 {
    1.018
}

impl AssetRef {
    pub fn resolve_paths(&mut self, root: &std::path::Path) {
        fn resolve(root: &std::path::Path, value: &mut String) {
            let path = std::path::Path::new(value);
            if !path.is_absolute() {
                *value = root.join(path).to_string_lossy().into_owned();
            }
        }

        match self {
            Self::Mesh { path }
            | Self::Word { path }
            | Self::Group { path }
            | Self::GeoJsonMap { path, .. } => resolve(root, path),
            Self::Glyph { path, metadata } => {
                resolve(root, path);
                if let Some(metadata) = metadata {
                    resolve(root, metadata);
                }
            }
        }
    }
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
pub struct AsciiSimplifyConfig {
    #[serde(default = "AsciiSimplifyConfig::default_enabled")]
    pub enabled: bool,
    pub grid_size: f32,
}

impl AsciiSimplifyConfig {
    const fn default_enabled() -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenderConfig {
    #[serde(default = "RenderConfig::default_visible")]
    pub visible: bool,

    #[serde(default)]
    pub stroke_character: Option<char>,

    #[serde(default)]
    pub mode: Option<String>,

    #[serde(default = "RenderConfig::default_edge_stride")]
    pub edge_stride: usize,

    #[serde(default)]
    pub ascii_simplify: Option<AsciiSimplifyConfig>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            visible: Self::default_visible(),
            stroke_character: None,
            mode: None,
            edge_stride: Self::default_edge_stride(),
            ascii_simplify: None,
        }
    }
}

impl RenderConfig {
    pub const fn default_visible() -> bool {
        true
    }

    pub const fn default_edge_stride() -> usize {
        1
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
    pub parent_matrix: Mat4,
    pub editor_composite: bool,
    pub editor_hidden: bool,
    pub source_root: PathBuf,
}

impl SceneObject {
    pub fn world_matrix(&self) -> Mat4 {
        self.parent_matrix * self.transform.matrix()
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
