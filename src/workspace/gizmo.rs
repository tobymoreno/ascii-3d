use std::{fs, io, path::Path};

use crate::math::Vec3;

/// Visual configuration for a workspace light-direction gizmo loaded from an
/// A3D scene manifest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadedA3dLightGizmo {
    pub visible: bool,
    pub length: f32,
    pub source_character: char,
    pub ray_character: char,
}

/// Light data needed by workspace gizmos and simple A3D shading.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedA3dLight {
    pub id: String,
    pub position: Vec3,
    pub direction: Vec3,
    pub intensity: f32,
    pub gizmo: LoadedA3dLightGizmo,
}

fn read_json_vec3(value: &serde_json::Value) -> Option<Vec3> {
    let values = value.as_array()?;

    if values.len() != 3 {
        return None;
    }

    Some(Vec3::new(
        values[0].as_f64()? as f32,
        values[1].as_f64()? as f32,
        values[2].as_f64()? as f32,
    ))
}

fn read_json_char(value: Option<&serde_json::Value>, default_value: char) -> char {
    value
        .and_then(serde_json::Value::as_str)
        .and_then(|text| text.chars().next())
        .unwrap_or(default_value)
}

/// Read A3D lights and their workspace gizmo metadata from `scene.a3d`.
pub fn loaded_a3d_lights(root: &Path) -> io::Result<Vec<LoadedA3dLight>> {
    let scene_path = root.join("scene.a3d");
    let source = fs::read_to_string(&scene_path)?;
    let json = serde_json::from_str::<serde_json::Value>(&source).map_err(|error| {
        io::Error::other(format!(
            "failed to parse A3D lights from {}: {}",
            scene_path.display(),
            error
        ))
    })?;

    let Some(lights) = json.get("lights").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };

    let mut parsed_lights = Vec::new();

    for light in lights {
        let id = light
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("light")
            .to_string();

        let position = light
            .get("position")
            .and_then(read_json_vec3)
            .unwrap_or_else(Vec3::zero);

        let direction = light
            .get("direction")
            .and_then(read_json_vec3)
            .unwrap_or_else(|| Vec3::new(-1.0, -1.0, -1.0));

        let intensity = light
            .get("intensity")
            .and_then(serde_json::Value::as_f64)
            .map(|value| value as f32)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(1.0);

        let gizmo = light.get("gizmo").unwrap_or(&serde_json::Value::Null);
        let visible = gizmo
            .get("visible")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let length = gizmo
            .get("length")
            .and_then(serde_json::Value::as_f64)
            .map(|value| value as f32)
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(1.5);

        let source_character = read_json_char(gizmo.get("source_character"), 'L');
        let ray_character = read_json_char(gizmo.get("ray_character"), '-');

        parsed_lights.push(LoadedA3dLight {
            id,
            position,
            direction,
            intensity,
            gizmo: LoadedA3dLightGizmo {
                visible,
                length,
                source_character,
                ray_character,
            },
        });
    }

    Ok(parsed_lights)
}

pub fn normalized_light_direction(direction: Vec3) -> Option<Vec3> {
    let length =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();

    if length <= f32::EPSILON {
        return None;
    }

    Some(direction * (1.0 / length))
}

/// Return the first enabled light direction, or a stable default.
pub fn loaded_a3d_primary_light_direction(root: &Path) -> io::Result<Vec3> {
    for light in loaded_a3d_lights(root)? {
        if light.intensity <= 0.0 {
            continue;
        }

        if let Some(direction) = normalized_light_direction(light.direction) {
            return Ok(direction);
        }
    }

    Ok(Vec3::new(-1.0, -1.0, -1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_light_direction_rejects_zero_vector() {
        assert_eq!(normalized_light_direction(Vec3::zero()), None);
    }

    #[test]
    fn normalized_light_direction_returns_unit_vector() {
        let direction = normalized_light_direction(Vec3::new(0.0, 3.0, 4.0)).unwrap();

        assert!((direction.x - 0.0).abs() < 0.0001);
        assert!((direction.y - 0.6).abs() < 0.0001);
        assert!((direction.z - 0.8).abs() < 0.0001);
    }
}
