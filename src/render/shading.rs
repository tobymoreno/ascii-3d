use super::Vec3;

pub const DEFAULT_LIGHT_RAY_DIRECTION: [f32; 3] = [-1.0, -1.0, -1.0];
pub const DEFAULT_ASCII_SHADE_RAMP: &[u8] = b" .,-~:;=!*#$@";

pub fn normalized_direction(direction: [f32; 3]) -> Option<Vec3> {
    let direction = Vec3::from_array(direction);
    let length =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();

    if length <= f32::EPSILON {
        None
    } else {
        Some(Vec3::new(
            direction.x / length,
            direction.y / length,
            direction.z / length,
        ))
    }
}

/// Convert a directional-light ray direction into the vector from a surface
/// point toward the light source.
pub fn surface_to_light_from_ray_direction(direction: [f32; 3]) -> Vec3 {
    let ray_direction = normalized_direction(direction)
        .unwrap_or_else(|| Vec3::from_array(DEFAULT_LIGHT_RAY_DIRECTION).normalized());

    Vec3::new(-ray_direction.x, -ray_direction.y, -ray_direction.z)
}

pub fn lambert_brightness(
    normal: Vec3,
    surface_to_light: Vec3,
    ambient: f32,
    diffuse_strength: f32,
) -> f32 {
    let normal = normal.normalized();
    let surface_to_light = surface_to_light.normalized();
    let diffuse = normal.dot(surface_to_light).max(0.0);

    (ambient + diffuse * diffuse_strength).clamp(0.0, 1.0)
}

pub fn shade_ascii_lambert(
    normal: Vec3,
    surface_to_light: Vec3,
    ambient: f32,
    diffuse_strength: f32,
) -> char {
    shade_ascii_brightness(
        lambert_brightness(normal, surface_to_light, ambient, diffuse_strength),
        DEFAULT_ASCII_SHADE_RAMP,
    )
}

pub fn shade_ascii_brightness(brightness: f32, ramp: &[u8]) -> char {
    if ramp.is_empty() {
        return ' ';
    }

    let brightness = brightness.clamp(0.0, 1.0);
    let index = (brightness * (ramp.len().saturating_sub(1)) as f32).round() as usize;

    ramp[index.min(ramp.len().saturating_sub(1))] as char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_direction_is_inverted_to_surface_to_light() {
        let direction = surface_to_light_from_ray_direction([0.0, 0.0, -2.0]);
        assert!((direction.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn aligned_normal_is_brighter_than_opposed_normal() {
        let light = Vec3::new(0.0, 0.0, 1.0);
        let aligned = lambert_brightness(Vec3::new(0.0, 0.0, 1.0), light, 0.18, 0.82);
        let opposed = lambert_brightness(Vec3::new(0.0, 0.0, -1.0), light, 0.18, 0.82);

        assert!(aligned > opposed);
        assert!((opposed - 0.18).abs() < 0.0001);
    }
}
