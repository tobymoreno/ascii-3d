use std::{error::Error, fs, path::Path};

use serde_json::Value;

use crate::math::Vec3;

#[derive(Clone, Debug, Default)]
pub struct GeoJsonPolygon {
    pub points: Vec<Vec3>,
    pub projected: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Default)]
pub struct GeoJsonMap {
    pub segments: Vec<(Vec3, Vec3)>,
    pub polygons: Vec<GeoJsonPolygon>,
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
                for ring in rings {
                    add_line_string(Some(ring), radius_scale, segments);
                }
                if let Some(exterior) = rings.first() {
                    add_polygon_ring(exterior, radius_scale, polygons);
                }
            }
        }
        Some("MultiPolygon") => {
            if let Some(multi_polygons) = value.get("coordinates").and_then(Value::as_array) {
                for polygon in multi_polygons {
                    if let Some(rings) = polygon.as_array() {
                        for ring in rings {
                            add_line_string(Some(ring), radius_scale, segments);
                        }
                        if let Some(exterior) = rings.first() {
                            add_polygon_ring(exterior, radius_scale, polygons);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn add_polygon_ring(value: &Value, radius_scale: f32, polygons: &mut Vec<GeoJsonPolygon>) {
    let Some(values) = value.as_array() else {
        return;
    };
    let mut coordinates = values
        .iter()
        .filter_map(coordinate_lon_lat)
        .collect::<Vec<_>>();
    if coordinates.len() >= 2 && coordinates[0] == *coordinates.last().unwrap() {
        coordinates.pop();
    }
    if coordinates.len() < 3 {
        return;
    }

    let points = coordinates
        .iter()
        .map(|coordinate| lon_lat_to_sphere(*coordinate, radius_scale))
        .collect::<Vec<_>>();
    let projected = project_ring_for_triangulation(&coordinates);
    polygons.push(GeoJsonPolygon { points, projected });
}

fn project_ring_for_triangulation(coordinates: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mean_latitude =
        coordinates.iter().map(|point| point[1]).sum::<f32>() / coordinates.len() as f32;
    let unwrapped_longitudes = unwrap_longitudes(coordinates);
    let longitude_span = unwrapped_longitudes
        .iter()
        .copied()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
            (min.min(value), max.max(value))
        });

    if mean_latitude.abs() > 70.0 && longitude_span.1 - longitude_span.0 > 180.0 {
        let north = mean_latitude >= 0.0;
        coordinates
            .iter()
            .map(|point| {
                let angle = point[0].to_radians();
                let radius = if north {
                    90.0 - point[1]
                } else {
                    90.0 + point[1]
                };
                [radius * angle.sin(), radius * angle.cos()]
            })
            .collect()
    } else {
        let longitude_scale = mean_latitude.to_radians().cos().abs().max(0.15);
        unwrapped_longitudes
            .into_iter()
            .zip(coordinates.iter())
            .map(|(longitude, point)| [longitude * longitude_scale, point[1]])
            .collect()
    }
}

fn unwrap_longitudes(coordinates: &[[f32; 2]]) -> Vec<f32> {
    let mut result = Vec::with_capacity(coordinates.len());
    let mut previous = coordinates[0][0];
    result.push(previous);
    for coordinate in &coordinates[1..] {
        let mut longitude = coordinate[0];
        while longitude - previous > 180.0 {
            longitude -= 360.0;
        }
        while longitude - previous < -180.0 {
            longitude += 360.0;
        }
        result.push(longitude);
        previous = longitude;
    }
    result
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
    fn seam_crossing_ring_projects_continuously() {
        let coordinates = [
            [170.0, 10.0],
            [-170.0, 10.0],
            [-170.0, -10.0],
            [170.0, -10.0],
        ];
        let projected = project_ring_for_triangulation(&coordinates);
        let xs = projected.iter().map(|point| point[0]).collect::<Vec<_>>();
        let span = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - xs.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(span < 60.0);
    }
}
