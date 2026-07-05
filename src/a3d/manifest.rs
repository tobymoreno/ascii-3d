use super::{A3dObject, PhysicsWorldConfig};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct A3dManifest {
    pub version: u32,
    pub title: String,

    #[serde(default)]
    pub world: A3dWorld,

    #[serde(default)]
    pub camera: Option<A3dCamera>,

    #[serde(default)]
    pub viewport: A3dViewport,

    #[serde(default)]
    pub objects: Vec<A3dObject>,
}

impl A3dManifest {
    pub const SUPPORTED_VERSION: u32 = 1;

    pub fn validate(&self) -> Result<(), String> {
        if self.version != Self::SUPPORTED_VERSION {
            return Err(format!(
                "unsupported .a3d version {}; supported version is {}",
                self.version,
                Self::SUPPORTED_VERSION
            ));
        }

        if self.title.trim().is_empty() {
            return Err(".a3d title must not be empty".to_string());
        }

        for object in &self.objects {
            if object.id.trim().is_empty() {
                return Err(".a3d object id must not be empty".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct A3dWorld {
    #[serde(default)]
    pub physics: PhysicsWorldConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct A3dCamera {
    pub position: [f32; 3],
    pub target: [f32; 3],

    #[serde(default = "A3dCamera::default_up")]
    pub up: [f32; 3],

    #[serde(default = "A3dCamera::default_fov_degrees")]
    pub fov_degrees: f32,

    #[serde(default = "A3dCamera::default_near")]
    pub near: f32,

    #[serde(default = "A3dCamera::default_far")]
    pub far: f32,
}

impl A3dCamera {
    pub const fn default_up() -> [f32; 3] {
        [0.0, 1.0, 0.0]
    }

    pub const fn default_fov_degrees() -> f32 {
        60.0
    }

    pub const fn default_near() -> f32 {
        0.1
    }

    pub const fn default_far() -> f32 {
        100.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct A3dViewport {
    #[serde(default)]
    pub show_world_debug: bool,

    #[serde(default)]
    pub show_camera3d: bool,
}

impl Default for A3dViewport {
    fn default() -> Self {
        Self {
            show_world_debug: true,
            show_camera3d: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::A3dManifest;

    #[test]
    fn manifest_rejects_unsupported_version() {
        let manifest = A3dManifest {
            version: 999,
            title: "bad".to_string(),
            world: Default::default(),
            camera: None,
            viewport: Default::default(),
            objects: vec![],
        };

        assert!(manifest.validate().is_err());
    }
}
