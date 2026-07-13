use super::{GeoJsonMapAsset, lerp_angle_degrees, lon_lat_to_sphere, segment_steps};

/// Visits every sampled point in a longitude/latitude path.
///
/// Sampling density is shared by every renderer host so map outlines have the
/// same geometry regardless of the camera or viewport implementation.
pub fn visit_lon_lat_samples(
    points_lon_lat: &[(f32, f32)],
    radius: f32,
    mut visit: impl FnMut([f32; 3]),
) {
    for pair in points_lon_lat.windows(2) {
        let (lon_a, lat_a) = pair[0];
        let (lon_b, lat_b) = pair[1];
        let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);

        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let lon = lerp_angle_degrees(lon_a, lon_b, t);
            let lat = lat_a + (lat_b - lat_a) * t;
            let point = lon_lat_to_sphere(lon, lat, radius);
            visit([point.x, point.y, point.z]);
        }
    }
}

/// Visits visible line segments from a GeoJSON sphere map.
///
/// The shared stage owns spherical subdivision, continuity breaks across
/// hidden/clipped samples, and marker selection. Hosts supply world
/// transformation and visibility rules because camera conventions differ.
pub fn visit_geojson_segments(
    map: &GeoJsonMapAsset,
    radius: f32,
    mut transform: impl FnMut([f32; 3]) -> [f32; 3],
    mut is_visible: impl FnMut([f32; 3]) -> bool,
    mut visit_segment: impl FnMut(char, [f32; 3], [f32; 3]),
) {
    for line in &map.lines {
        let mut previous = None;

        visit_lon_lat_samples(&line.points_lon_lat, radius, |local| {
            let world = transform(local);

            if !is_visible(world) {
                previous = None;
                return;
            }

            if let Some(from) = previous {
                visit_segment(line.marker, from, world);
            }

            previous = Some(world);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_path_visits_subdivided_points() {
        let mut points = Vec::new();
        visit_lon_lat_samples(&[(0.0, 0.0), (10.0, 0.0)], 1.0, |point| points.push(point));

        assert!(points.len() >= 2);
        assert!(
            points
                .iter()
                .all(|point| point.iter().all(|value| value.is_finite()))
        );
    }
}
