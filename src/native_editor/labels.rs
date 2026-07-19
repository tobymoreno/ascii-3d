use std::{collections::HashSet, fs, path::Path};

use serde_json::Value;

use crate::math::Vec3;

const COUNTRY_LABELS_FILE: &str = "assets/maps/labels/ne_10m_admin_0_label_points.geojson";
const POPULATED_PLACES_FILE: &str = "assets/maps/labels/ne_10m_populated_places_simple.geojson";
const MARINE_LABELS_FILE: &str = "assets/maps/labels/ne_10m_geography_marine_label_points.geojson";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelKind {
    Place,
    Country,
    Marine,
}

#[derive(Clone, Debug)]
pub(crate) struct GlobeLabel {
    pub(crate) text: String,
    pub(crate) direction: Vec3,
    pub(crate) priority: i32,
    pub(crate) font_size: f32,
    pub(crate) altitude: f32,
    pub(crate) kind: LabelKind,
    pub(crate) morph: f32,
}

pub(crate) fn load_builtin_labels(project_root: &Path) -> Vec<GlobeLabel> {
    let mut labels = Vec::new();
    let mut seen = HashSet::new();

    load_country_labels(
        &project_root.join(COUNTRY_LABELS_FILE),
        &mut labels,
        &mut seen,
    );
    load_populated_place_labels(
        &project_root.join(POPULATED_PLACES_FILE),
        &mut labels,
        &mut seen,
    );
    load_marine_labels(
        &project_root.join(MARINE_LABELS_FILE),
        &mut labels,
        &mut seen,
    );

    labels.sort_by(|left, right| left.priority.cmp(&right.priority));
    labels
}

fn load_country_labels(path: &Path, labels: &mut Vec<GlobeLabel>, seen: &mut HashSet<String>) {
    let Some(document) = read_geojson(path) else {
        return;
    };
    let Some(features) = document.get("features").and_then(Value::as_array) else {
        return;
    };

    for feature in features {
        let properties = feature.get("properties").unwrap_or(&Value::Null);
        let labelrank = prop_f32(properties, "labelrank").unwrap_or(99.0);
        if labelrank > 4.0 {
            continue;
        }

        let Some(text) = prop_string(properties, &["name", "name_en", "nameascii"]) else {
            continue;
        };
        let Some([longitude, latitude]) = feature_lon_lat(feature) else {
            continue;
        };

        let key = format!("country:{}", text.to_lowercase());
        if !seen.insert(key) {
            continue;
        }

        labels.push(GlobeLabel {
            text,
            direction: lon_lat_to_unit(longitude, latitude),
            priority: (labelrank.round() as i32) * 10,
            font_size: (18.0 - labelrank * 1.2).clamp(12.0, 18.0),
            altitude: 0.09,
            kind: LabelKind::Country,
            morph: 0.0,
        });
    }
}

fn to_initial_caps(text: &str) -> String {
    const SMALL_WORDS: &[&str] = &[
        "of", "the", "and", "in", "on", "at", "to", "for", "by", "de", "del", "la",
    ];
    text.split_whitespace()
        .enumerate()
        .map(|(index, word)| {
            let lower = word.to_ascii_lowercase();
            if index > 0 && SMALL_WORDS.contains(&lower.as_str()) {
                return lower;
            }
            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_marine_labels(path: &Path, labels: &mut Vec<GlobeLabel>, seen: &mut HashSet<String>) {
    let Some(document) = read_geojson(path) else {
        return;
    };
    let Some(features) = document.get("features").and_then(Value::as_array) else {
        return;
    };

    for feature in features {
        let properties = feature.get("properties").unwrap_or(&Value::Null);
        let scalerank = prop_f32(properties, "scalerank").unwrap_or(99.0);
        if scalerank > 2.0 {
            continue;
        }
        let Some(text) = prop_string(properties, &["name_en", "name", "nameascii"]) else {
            continue;
        };
        let text = to_initial_caps(&text);
        let Some([longitude, latitude]) = feature_lon_lat(feature) else {
            continue;
        };

        let key = format!(
            "marine:{}:{longitude:.3}:{latitude:.3}",
            text.to_lowercase()
        );
        if !seen.insert(key) {
            continue;
        }

        let feature_class = prop_string(properties, &["featurecla"])
            .unwrap_or_default()
            .to_ascii_lowercase();
        let (priority, font_size, altitude, morph) = match feature_class.as_str() {
            "ocean" => (5 + scalerank.round() as i32, 14.2, 0.014, 0.0),
            "sea" => (18 + scalerank.round() as i32, 13.0, 0.013, 0.0),
            "bay" | "gulf" => (26 + scalerank.round() as i32, 12.0, 0.012, 0.0),
            _ => (34 + scalerank.round() as i32, 11.2, 0.011, 0.0),
        };

        labels.push(GlobeLabel {
            text,
            direction: lon_lat_to_unit(longitude, latitude),
            priority,
            font_size,
            altitude,
            kind: LabelKind::Marine,
            morph,
        });
    }
}
fn load_populated_place_labels(
    path: &Path,
    labels: &mut Vec<GlobeLabel>,
    seen: &mut HashSet<String>,
) {
    let Some(document) = read_geojson(path) else {
        return;
    };
    let Some(features) = document.get("features").and_then(Value::as_array) else {
        return;
    };

    for feature in features {
        let properties = feature.get("properties").unwrap_or(&Value::Null);

        let feature_class = prop_string(properties, &["featurecla"])
            .unwrap_or_default()
            .to_ascii_lowercase();
        let is_capital = prop_boolish(properties, "adm0cap")
            || prop_boolish(properties, "capin")
            || feature_class.contains("capital");
        let is_world_city = prop_boolish(properties, "worldcity");
        let is_megacity = prop_boolish(properties, "megacity");
        let labelrank = prop_f32(properties, "labelrank").unwrap_or(99.0);

        if !(is_capital || is_world_city || is_megacity || labelrank <= 3.0) {
            continue;
        }

        let Some(text) = prop_string(properties, &["nameascii", "name"]) else {
            continue;
        };
        let Some([longitude, latitude]) = feature_lon_lat(feature) else {
            continue;
        };

        let key = format!("place:{}", text.to_lowercase());
        if !seen.insert(key) {
            continue;
        }

        let (priority, font_size, altitude) = if is_capital {
            (40 + labelrank.round() as i32, 14.5, 0.075)
        } else if is_megacity {
            (52, 13.5, 0.065)
        } else if is_world_city {
            (60, 12.5, 0.06)
        } else {
            (70 + labelrank.round() as i32, 11.5, 0.055)
        };

        labels.push(GlobeLabel {
            text,
            direction: lon_lat_to_unit(longitude, latitude),
            priority,
            font_size,
            altitude,
            kind: LabelKind::Place,
            morph: 0.0,
        });
    }
}

fn read_geojson(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn feature_lon_lat(feature: &Value) -> Option<[f32; 2]> {
    let geometry = feature.get("geometry")?;
    let geometry_type = geometry.get("type")?.as_str()?;
    if geometry_type == "Point" {
        let coordinates = geometry.get("coordinates")?.as_array()?;
        return Some([
            coordinates.first()?.as_f64()? as f32,
            coordinates.get(1)?.as_f64()? as f32,
        ]);
    }

    let properties = feature.get("properties").unwrap_or(&Value::Null);
    Some([
        prop_f32(properties, "longitude")?,
        prop_f32(properties, "latitude")?,
    ])
}

fn prop_string(properties: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = properties.get(*key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }
    None
}

fn prop_f32(properties: &Value, key: &str) -> Option<f32> {
    let value = properties.get(key)?;
    if let Some(number) = value.as_f64() {
        return Some(number as f32);
    }
    if let Some(text) = value.as_str() {
        return text.parse::<f32>().ok();
    }
    None
}

fn prop_boolish(properties: &Value, key: &str) -> bool {
    let Some(value) = properties.get(key) else {
        return false;
    };
    if let Some(boolean) = value.as_bool() {
        return boolean;
    }
    if let Some(number) = value.as_i64() {
        return number != 0;
    }
    if let Some(text) = value.as_str() {
        let lowered = text.trim().to_ascii_lowercase();
        return matches!(lowered.as_str(), "1" | "true" | "yes");
    }
    false
}

fn lon_lat_to_unit(longitude: f32, latitude: f32) -> Vec3 {
    let lon = longitude.to_radians();
    let lat = latitude.to_radians();
    let cos_lat = lat.cos();
    Vec3::new(cos_lat * lon.sin(), lat.sin(), cos_lat * lon.cos()).normalized()
}

pub(crate) fn rasterize_marine_label(text: &str) -> egui::ColorImage {
    const SCALE: usize = 6;
    const GLYPH_W: usize = 5;
    const GLYPH_H: usize = 7;
    const TRACKING: usize = 1;
    const PAD: usize = 1;

    let text = text.to_owned();
    let glyph_count = text.chars().count().max(1);
    let cell_width = GLYPH_W * SCALE + TRACKING * SCALE;
    let width = PAD * 2 + glyph_count * cell_width - TRACKING * SCALE;
    let height = PAD * 2 + GLYPH_H * SCALE;
    let mut image = egui::ColorImage::filled([width, height], egui::Color32::TRANSPARENT);

    // Marine labels are intentionally a single dark navy tone. No pale edge,
    // highlight, shadow, or background is baked into the glyph texture.
    let fill = egui::Color32::from_rgba_unmultiplied(28, 72, 138, 172);

    for (glyph_index, ch) in text.chars().enumerate() {
        let pattern = marine_glyph(ch);
        let origin_x = PAD + glyph_index * cell_width;
        let origin_y = PAD;
        for (row, bits) in pattern.into_iter().enumerate() {
            for col in 0..GLYPH_W {
                if bits & (1 << (GLYPH_W - 1 - col)) == 0 {
                    continue;
                }
                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        let x = origin_x + col * SCALE + sx;
                        let y = origin_y + row * SCALE + sy;
                        image.pixels[y * width + x] = fill;
                    }
                }
            }
        }
    }
    image
}

fn marine_glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        'a' => [0, 0b01110, 0b00001, 0b01111, 0b10001, 0b10011, 0b01101],
        'b' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110,
        ],
        'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110],
        'd' => [
            0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111,
        ],
        'e' => [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
        'f' => [
            0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000,
        ],
        'g' => [0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'h' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001,
        ],
        'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        'j' => [0b00010, 0, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100],
        'k' => [
            0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010,
        ],
        'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101],
        'n' => [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        'o' => [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        'p' => [0, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000],
        'q' => [0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001],
        'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        's' => [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        't' => [
            0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110,
        ],
        'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'w' => [0, 0, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010],
        'x' => [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        'y' => [0, 0, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'z' => [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '\'' => [0b00100, 0b00100, 0b00010, 0, 0, 0, 0],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod marine_label_tests {
    use super::*;

    #[test]
    fn marine_initial_caps_preserves_lowercase_word_bodies() {
        assert_eq!(to_initial_caps("ATLANTIC OCEAN"), "Atlantic Ocean");
        assert_eq!(to_initial_caps("GULF OF GUINEA"), "Gulf of Guinea");
        assert_eq!(to_initial_caps("SOUTHERN OCEAN"), "Southern Ocean");
    }

    #[test]
    fn lowercase_marine_glyphs_are_distinct_from_uppercase() {
        assert_ne!(marine_glyph('t'), marine_glyph('T'));
        assert_ne!(marine_glyph('c'), marine_glyph('C'));
        assert_ne!(marine_glyph('e'), marine_glyph('E'));
    }

    #[test]
    fn rasterizer_accepts_mixed_case_without_forcing_uppercase() {
        let mixed = rasterize_marine_label("Atlantic Ocean");
        let upper = rasterize_marine_label("ATLANTIC OCEAN");
        assert_ne!(mixed.pixels, upper.pixels);
    }
}
