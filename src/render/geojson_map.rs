use serde_json::Value;
use std::{fs, io, path::Path};

#[derive(Debug)]
pub struct GeoJsonMapAsset {
    pub lines: Vec<MapLine>,
}

#[derive(Debug)]
pub struct MapLine {
    pub name: String,
    pub marker: char,
    pub points_lon_lat: Vec<(f32, f32)>,
}

#[derive(Clone, Copy, Debug)]
pub struct GeoPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GeoPoint {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

pub fn load_geojson_map_asset(path: &Path) -> io::Result<GeoJsonMapAsset> {
    let text = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&text).map_err(invalid_data)?;

    let mut lines = Vec::new();

    let Some(features) = value.get("features").and_then(|value| value.as_array()) else {
        return Ok(GeoJsonMapAsset { lines });
    };

    for feature in features {
        let name = feature
            .get("properties")
            .and_then(|properties| properties.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("unnamed")
            .to_string();

        let marker = feature
            .get("properties")
            .and_then(|properties| properties.get("marker"))
            .and_then(|marker| marker.as_str())
            .and_then(|marker| marker.chars().next())
            .unwrap_or('*');

        let Some(geometry) = feature.get("geometry") else {
            continue;
        };

        let geometry_type = geometry
            .get("type")
            .and_then(|geometry_type| geometry_type.as_str())
            .unwrap_or("");

        match geometry_type {
            "Polygon" => {
                if let Some(rings) = geometry.get("coordinates").and_then(|coords| coords.as_array()) {
                    for ring in rings {
                        if let Some(points) = parse_lon_lat_ring(ring) {
                            lines.push(MapLine {
                                name: name.clone(),
                                marker,
                                points_lon_lat: points,
                            });
                        }
                    }
                }
            }
            "MultiPolygon" => {
                if let Some(polygons) = geometry.get("coordinates").and_then(|coords| coords.as_array()) {
                    for polygon in polygons {
                        let Some(rings) = polygon.as_array() else {
                            continue;
                        };

                        for ring in rings {
                            if let Some(points) = parse_lon_lat_ring(ring) {
                                lines.push(MapLine {
                                    name: name.clone(),
                                    marker,
                                    points_lon_lat: points,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(GeoJsonMapAsset { lines })
}

fn parse_lon_lat_ring(value: &Value) -> Option<Vec<(f32, f32)>> {
    let coordinates = value.as_array()?;
    let mut points = Vec::new();

    for coordinate in coordinates {
        let pair = coordinate.as_array()?;

        if pair.len() < 2 {
            continue;
        }

        let lon = pair[0].as_f64()? as f32;
        let lat = pair[1].as_f64()? as f32;

        points.push((lon, lat));
    }

    if points.len() >= 2 {
        Some(points)
    } else {
        None
    }
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

pub fn lon_lat_to_sphere(lon_degrees: f32, lat_degrees: f32, radius: f32) -> GeoPoint {
    let lon = lon_degrees.to_radians();
    let lat = lat_degrees.to_radians();

    GeoPoint::new(
        radius * lat.cos() * lon.cos(),
        radius * lat.sin(),
        radius * lat.cos() * lon.sin(),
    )
}

pub fn segment_steps(lon_a: f32, lat_a: f32, lon_b: f32, lat_b: f32) -> usize {
    let lon_delta = (lon_b - lon_a).abs();
    let lat_delta = (lat_b - lat_a).abs();
    let degrees = lon_delta.max(lat_delta);
    ((degrees / 4.0).ceil() as usize).clamp(2, 16)
}

pub fn lerp_angle_degrees(a: f32, b: f32, t: f32) -> f32 {
    let mut delta = b - a;

    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }

    a + delta * t
}

pub fn point_in_polygon(px: f32, py: f32, polygon: &[(i32, i32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;

    for current in 0..polygon.len() {
        let xi = polygon[current].0 as f32;
        let yi = polygon[current].1 as f32;
        let xj = polygon[previous].0 as f32;
        let yj = polygon[previous].1 as f32;

        let crosses = (yi > py) != (yj > py);
        if crosses {
            let x_at_y = (xj - xi) * (py - yi) / ((yj - yi).abs().max(0.0001)) + xi;
            if px < x_at_y {
                inside = !inside;
            }
        }

        previous = current;
    }

    inside
}

pub fn land_fill_char(x: i32, y: i32) -> Option<char> {
    let n = (x * 17 + y * 31 + x * y * 3).rem_euclid(11);

    match n {
        0 | 1 | 2 => Some('+'),
        3 => Some(':'),
        _ => None,
    }
}
