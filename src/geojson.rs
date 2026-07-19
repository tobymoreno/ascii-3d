use std::{error::Error, fs, path::Path};

use serde_json::Value;

use crate::math::Vec3;

#[derive(Clone, Debug, Default)]
pub struct GeoJsonPolygon {
    /// Flattened exterior ring followed by interior rings.
    pub points: Vec<Vec3>,
    /// All rings projected into one shared 2D coordinate system.
    pub projected: Vec<[f32; 2]>,
    /// Start index of each interior ring.
    pub hole_indices: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct GeoJsonMap {
    pub segments: Vec<(Vec3, Vec3)>,
    pub polygons: Vec<GeoJsonPolygon>,
}

#[derive(Clone, Copy, Debug)]
pub struct GeoJsonElevationPoint {
    pub position: Vec3,
    pub elevation_meters: f32,
    pub scale_rank: f32,
}

pub fn load_geojson_elevation_points(
    path: impl AsRef<Path>,
) -> Result<Vec<GeoJsonElevationPoint>, Box<dyn Error>> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let mut points = Vec::new();

    let Some(features) = value.get("features").and_then(Value::as_array) else {
        return Ok(points);
    };

    for feature in features {
        let Some(geometry) = feature.get("geometry") else {
            continue;
        };
        if geometry.get("type").and_then(Value::as_str) != Some("Point") {
            continue;
        }
        let Some(coordinate) = geometry.get("coordinates").and_then(coordinate_lon_lat) else {
            continue;
        };
        let properties = feature.get("properties");
        let Some(elevation_meters) = properties
            .and_then(|value| value.get("elevation"))
            .and_then(value_as_f32)
            .or_else(|| {
                properties
                    .and_then(|value| value.get("elev"))
                    .and_then(value_as_f32)
            })
        else {
            continue;
        };
        if !elevation_meters.is_finite() || elevation_meters <= 0.0 {
            continue;
        }
        let scale_rank = properties
            .and_then(|value| value.get("scalerank"))
            .and_then(value_as_f32)
            .unwrap_or(8.0);
        points.push(GeoJsonElevationPoint {
            position: lon_lat_to_sphere(coordinate, 1.0).normalized(),
            elevation_meters,
            scale_rank,
        });
    }

    Ok(points)
}

fn value_as_f32(value: &Value) -> Option<f32> {
    value
        .as_f64()
        .map(|number| number as f32)
        .or_else(|| value.as_str()?.parse::<f32>().ok())
}

pub fn load_geojson_map(
    path: impl AsRef<Path>,
    radius_scale: f32,
) -> Result<GeoJsonMap, Box<dyn Error>> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let mut map = GeoJsonMap::default();
    collect_value(&value, radius_scale, &mut map.segments, &mut map.polygons);
    Ok(map)
}

fn collect_value(
    value: &Value,
    radius_scale: f32,
    segments: &mut Vec<(Vec3, Vec3)>,
    polygons: &mut Vec<GeoJsonPolygon>,
) {
    match value.get("type").and_then(Value::as_str) {
        Some("FeatureCollection") => {
            if let Some(features) = value.get("features").and_then(Value::as_array) {
                for feature in features {
                    collect_value(feature, radius_scale, segments, polygons);
                }
            }
        }
        Some("Feature") => {
            if let Some(geometry) = value.get("geometry") {
                collect_value(geometry, radius_scale, segments, polygons);
            }
        }
        Some("GeometryCollection") => {
            if let Some(geometries) = value.get("geometries").and_then(Value::as_array) {
                for geometry in geometries {
                    collect_value(geometry, radius_scale, segments, polygons);
                }
            }
        }
        Some("LineString") => add_line_string(value.get("coordinates"), radius_scale, segments),
        Some("MultiLineString") => {
            if let Some(lines) = value.get("coordinates").and_then(Value::as_array) {
                for line in lines {
                    add_line_string(Some(line), radius_scale, segments);
                }
            }
        }
        Some("Polygon") => {
            if let Some(rings) = value.get("coordinates").and_then(Value::as_array) {
                add_polygon(rings, radius_scale, segments, polygons);
            }
        }
        Some("MultiPolygon") => {
            if let Some(multi_polygons) = value.get("coordinates").and_then(Value::as_array) {
                for polygon in multi_polygons {
                    if let Some(rings) = polygon.as_array() {
                        add_polygon(rings, radius_scale, segments, polygons);
                    }
                }
            }
        }
        _ => {}
    }
}

fn add_polygon(
    ring_values: &[Value],
    radius_scale: f32,
    segments: &mut Vec<(Vec3, Vec3)>,
    polygons: &mut Vec<GeoJsonPolygon>,
) {
    let mut rings = Vec::<Vec<[f32; 2]>>::new();
    for ring_value in ring_values {
        add_line_string(Some(ring_value), radius_scale, segments);
        let Some(values) = ring_value.as_array() else {
            continue;
        };
        let mut ring = values
            .iter()
            .filter_map(coordinate_lon_lat)
            .collect::<Vec<_>>();
        if ring.len() >= 2 && ring.first() == ring.last() {
            ring.pop();
        }
        if ring.len() >= 3 {
            rings.push(ring);
        }
    }
    if rings.is_empty() {
        return;
    }

    let mut projected_rings = project_polygon_rings(&rings);
    // Normalize winding without breaking the index correspondence between the
    // spherical and projected copies of each ring.
    if signed_ring_area(&projected_rings[0]) < 0.0 {
        rings[0].reverse();
        projected_rings[0].reverse();
    }
    for ring_index in 1..rings.len() {
        if signed_ring_area(&projected_rings[ring_index]) > 0.0 {
            rings[ring_index].reverse();
            projected_rings[ring_index].reverse();
        }
    }

    let mut points = Vec::new();
    let mut projected = Vec::new();
    let mut hole_indices = Vec::new();
    for (ring_index, (ring, projected_ring)) in rings.iter().zip(projected_rings).enumerate() {
        if ring_index > 0 {
            hole_indices.push(points.len());
        }
        points.extend(
            ring.iter()
                .map(|coordinate| lon_lat_to_sphere(*coordinate, radius_scale)),
        );
        projected.extend(projected_ring);
    }
    polygons.push(GeoJsonPolygon {
        points,
        projected,
        hole_indices,
    });
}

fn project_polygon_rings(rings: &[Vec<[f32; 2]>]) -> Vec<Vec<[f32; 2]>> {
    // Use a local equirectangular tangent plane centered on the exterior ring.
    // This is much more numerically stable than the previous globe-wide
    // gnomonic projection for very large land polygons.
    let exterior = &rings[0];
    let reference_lon = circular_mean_longitude(exterior);
    let reference_lat =
        exterior.iter().map(|coordinate| coordinate[1]).sum::<f32>() / exterior.len().max(1) as f32;
    let cos_lat = reference_lat.to_radians().cos().abs().max(0.25);

    rings
        .iter()
        .map(|ring| {
            let unwrapped = unwrap_longitudes_from_reference(ring, reference_lon);
            ring.iter()
                .zip(unwrapped.into_iter())
                .map(|(coordinate, lon)| {
                    [
                        (lon - reference_lon) * cos_lat,
                        coordinate[1] - reference_lat,
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn circular_mean_longitude(coordinates: &[[f32; 2]]) -> f32 {
    if coordinates.is_empty() {
        return 0.0;
    }

    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    for coordinate in coordinates {
        let radians = coordinate[0].to_radians();
        sum_sin += radians.sin();
        sum_cos += radians.cos();
    }

    sum_sin.atan2(sum_cos).to_degrees()
}

fn signed_ring_area(ring: &[[f32; 2]]) -> f32 {
    if ring.len() < 3 {
        return 0.0;
    }
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f32>()
        * 0.5
}

fn unwrap_longitudes_from_reference(coordinates: &[[f32; 2]], reference: f32) -> Vec<f32> {
    coordinates
        .iter()
        .map(|coordinate| {
            let mut longitude = coordinate[0];
            while longitude - reference > 180.0 {
                longitude -= 360.0;
            }
            while longitude - reference < -180.0 {
                longitude += 360.0;
            }
            longitude
        })
        .collect()
}

fn add_line_string(
    coordinates: Option<&Value>,
    radius_scale: f32,
    segments: &mut Vec<(Vec3, Vec3)>,
) {
    let Some(points) = coordinates.and_then(Value::as_array) else {
        return;
    };

    let mut previous = None;
    for coordinate in points {
        let Some(point) = coordinate_to_sphere(coordinate, radius_scale) else {
            previous = None;
            continue;
        };
        if let Some(start) = previous {
            segments.push((start, point));
        }
        previous = Some(point);
    }
}

fn coordinate_lon_lat(value: &Value) -> Option<[f32; 2]> {
    let coordinate = value.as_array()?;
    Some([
        coordinate.first()?.as_f64()? as f32,
        coordinate.get(1)?.as_f64()? as f32,
    ])
}

fn coordinate_to_sphere(value: &Value, radius: f32) -> Option<Vec3> {
    Some(lon_lat_to_sphere(coordinate_lon_lat(value)?, radius))
}

fn lon_lat_to_sphere(coordinate: [f32; 2], radius: f32) -> Vec3 {
    let lon = coordinate[0].to_radians();
    let lat = coordinate[1].to_radians();
    let cos_lat = lat.cos();
    Vec3::new(
        radius * cos_lat * lon.sin(),
        radius * lat.sin(),
        radius * cos_lat * lon.cos(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_string_becomes_connected_spherical_segments() {
        let value: Value =
            serde_json::from_str(r#"{"type":"LineString","coordinates":[[0,0],[90,0],[90,90]]}"#)
                .unwrap();
        let mut segments = Vec::new();
        let mut polygons = Vec::new();
        collect_value(&value, 2.0, &mut segments, &mut polygons);
        assert_eq!(segments.len(), 2);
        assert!((segments[0].0.length() - 2.0).abs() < 1.0e-5);
        assert!((segments[1].1.length() - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn polygon_preserves_hole_start_indices() {
        let value: Value = serde_json::from_str(
            r#"{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]],[[2,2],[2,4],[4,4],[4,2],[2,2]]]}"#,
        )
        .unwrap();
        let mut segments = Vec::new();
        let mut polygons = Vec::new();
        collect_value(&value, 1.0, &mut segments, &mut polygons);
        assert_eq!(polygons[0].points.len(), 8);
        assert_eq!(polygons[0].hole_indices, vec![4]);
    }

    #[test]
    fn seam_crossing_ring_projects_continuously() {
        let coordinates = [
            [170.0, 10.0],
            [-170.0, 10.0],
            [-170.0, -10.0],
            [170.0, -10.0],
        ];
        let projected = project_polygon_rings(&[coordinates.to_vec()]);
        let xs = projected[0]
            .iter()
            .map(|point| point[0])
            .collect::<Vec<_>>();
        let span = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - xs.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(span < 60.0);
    }
}
