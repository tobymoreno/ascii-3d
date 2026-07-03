use std::{fs, io, path::Path};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Quad4SceneConfig {
    pub name: String,
    pub mesh_asset: String,
    pub camera: CameraConfig,
    pub frustum: FrustumConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CameraConfig {
    pub position: [f32; 3],
    pub pitch_amplitude_degrees: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FrustumConfig {
    pub vertical_fov_degrees: f32,
    pub aspect_ratio: f32,
    pub near_distance: f32,
    pub far_distance: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct DisplayConfig {
    pub world_scale: f32,
}

impl Quad4SceneConfig {
    pub fn validate(&self) -> io::Result<()> {
        if self.name.trim().is_empty() {
            return Err(io::Error::other("quad4 scene name cannot be empty"));
        }

        if self.mesh_asset.trim().is_empty() {
            return Err(io::Error::other("quad4 scene mesh_asset cannot be empty"));
        }

        validate_vec3("camera.position", self.camera.position)?;

        if !self.camera.pitch_amplitude_degrees.is_finite() {
            return Err(io::Error::other(
                "camera.pitch_amplitude_degrees must be finite",
            ));
        }

        validate_frustum(self.frustum)?;

        if !self.display.world_scale.is_finite() || self.display.world_scale <= 0.0 {
            return Err(io::Error::other(
                "display.world_scale must be finite and greater than zero",
            ));
        }

        Ok(())
    }
}

fn validate_frustum(frustum: FrustumConfig) -> io::Result<()> {
    if !frustum.vertical_fov_degrees.is_finite()
        || frustum.vertical_fov_degrees <= 0.0
        || frustum.vertical_fov_degrees >= 180.0
    {
        return Err(io::Error::other(
            "frustum.vertical_fov_degrees must be finite and between 0 and 180",
        ));
    }

    if !frustum.aspect_ratio.is_finite() || frustum.aspect_ratio <= 0.0 {
        return Err(io::Error::other(
            "frustum.aspect_ratio must be finite and greater than zero",
        ));
    }

    if !frustum.near_distance.is_finite() || frustum.near_distance <= 0.0 {
        return Err(io::Error::other(
            "frustum.near_distance must be finite and greater than zero",
        ));
    }

    if !frustum.far_distance.is_finite() || frustum.far_distance <= frustum.near_distance {
        return Err(io::Error::other(
            "frustum.far_distance must be finite and greater than near_distance",
        ));
    }

    Ok(())
}

fn validate_vec3(name: &str, value: [f32; 3]) -> io::Result<()> {
    if value.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{name} must contain finite values"
        )))
    }
}

pub fn load_quad4_scene_config(path: impl AsRef<Path>) -> io::Result<Quad4SceneConfig> {
    let path = path.as_ref();

    let text = fs::read_to_string(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to read quad4 scene config {}: {}",
                path.display(),
                error,
            ),
        )
    })?;

    let config: Quad4SceneConfig = serde_json::from_str(&text).map_err(|error| {
        io::Error::other(format!(
            "failed to parse quad4 scene config {}: {}",
            path.display(),
            error,
        ))
    })?;

    config.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::Quad4SceneConfig;

    #[test]
    fn embedded_example_is_valid() {
        let config: Quad4SceneConfig =
            serde_json::from_str(include_str!("../assets/quad4.scene.json"))
                .expect("quad4 scene JSON should parse");

        config.validate().expect("quad4 scene should validate");

        assert_eq!(config.mesh_asset, "quad4.obj");
        assert_eq!(config.frustum.vertical_fov_degrees, 60.0);
        assert_eq!(config.frustum.aspect_ratio, 1.77778);
        assert_eq!(config.frustum.near_distance, 0.25);
        assert_eq!(config.frustum.far_distance, 0.75);
    }
}
