use std::{fs, io, path::Path};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectionConfig {
    pub name: String,
    pub version: u32,
    pub units: String,
    pub screen_origin: [i32; 2],
    pub axis_vectors: ProjectionAxisVectors,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ProjectionAxisVectors {
    pub x: [f32; 2],
    pub y: [f32; 2],
    pub z: [f32; 2],
}

impl ProjectionConfig {
    pub fn validate(&self) -> io::Result<()> {
        if self.name.trim().is_empty() {
            return Err(io::Error::other("projection name cannot be empty"));
        }

        if self.version != 1 {
            return Err(io::Error::other(format!(
                "unsupported projection config version {}",
                self.version,
            )));
        }

        if self.units.trim().is_empty() {
            return Err(io::Error::other("projection units cannot be empty"));
        }

        validate_axis_vector("axis_vectors.x", self.axis_vectors.x)?;
        validate_axis_vector("axis_vectors.y", self.axis_vectors.y)?;
        validate_axis_vector("axis_vectors.z", self.axis_vectors.z)?;

        Ok(())
    }
}

fn validate_axis_vector(name: &str, value: [f32; 2]) -> io::Result<()> {
    if value.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{name} must contain finite values"
        )))
    }
}

pub fn load_projection_config(path: impl AsRef<Path>) -> io::Result<ProjectionConfig> {
    let path = path.as_ref();

    let text = fs::read_to_string(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to read projection config {}: {}",
                path.display(),
                error,
            ),
        )
    })?;

    let config: ProjectionConfig = serde_json::from_str(&text).map_err(|error| {
        io::Error::other(format!(
            "failed to parse projection config {}: {}",
            path.display(),
            error,
        ))
    })?;

    config.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::ProjectionConfig;

    #[test]
    fn embedded_default_projection_is_valid() {
        let config: ProjectionConfig =
            serde_json::from_str(include_str!("../assets/projection.default.json"))
                .expect("default projection JSON should parse");

        config
            .validate()
            .expect("default projection should validate");

        assert_eq!(config.axis_vectors.x, [8.0, 0.0]);
        assert_eq!(config.axis_vectors.y, [0.0, -3.0]);
        assert_eq!(config.axis_vectors.z, [-2.0, 2.0]);
    }
}
