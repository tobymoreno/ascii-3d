use serde::Deserialize;
use std::{fs, io, path::Path};

#[derive(Debug, Deserialize)]
pub struct SceneDocument {
    pub name: String,
    pub mesh_asset: String,
    pub display: DisplayDocument,

    #[serde(default)]
    pub lighting: Option<LightingDocument>,

    #[serde(default)]
    pub map_overlay: Option<MapOverlayDocument>,

    #[serde(default)]
    pub quads: Vec<QuadDocument>,
}

#[derive(Debug, Deserialize)]
pub struct DisplayDocument {
    pub world_scale: f32,

    #[serde(default)]
    pub rotation_y_degrees_per_turn: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct LightingDocument {
    pub primary_light_direction: [f32; 3],
}

#[derive(Debug, Deserialize)]
pub struct MapOverlayDocument {
    pub asset: String,

    #[serde(default = "default_map_overlay_visible")]
    pub visible: bool,

    #[serde(default = "default_map_radius_scale")]
    pub radius_scale: f32,
}

fn default_map_overlay_visible() -> bool {
    true
}

fn default_map_radius_scale() -> f32 {
    1.018
}

#[derive(Debug, Deserialize)]
pub struct QuadDocument {
    pub id: String,
    pub position: [f32; 3],
    pub size: [f32; 2],
    pub rotation_z_degrees: f32,
    pub marker: String,
    pub color: Option<String>,
}

pub fn load_scene_document(path: impl AsRef<Path>) -> io::Result<SceneDocument> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)?;

    serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse scene {}: {error}", path.display()),
        )
    })
}
