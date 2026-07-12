use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SceneDocument {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mesh_asset: String,
    pub display: DisplayDocument,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<LightingDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_overlay: Option<MapOverlayDocument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quads: Vec<QuadDocument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GroupDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DisplayDocument {
    pub world_scale: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_y_degrees_per_turn: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LightingDocument {
    pub primary_light_direction: [f32; 3],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapOverlayDocument {
    pub asset: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_map_radius_scale")]
    pub radius_scale: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupDocument {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub transform: TransformDocument,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub editor_composite: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<BehaviorDocument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "node", rename_all = "snake_case")]
pub enum NodeDocument {
    Group(GroupDocument),
    Object(ObjectDocument),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectDocument {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub transform: TransformDocument,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<BehaviorDocument>,
    #[serde(flatten)]
    pub object: ObjectKindDocument,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObjectKindDocument {
    Mesh {
        asset: String,
        #[serde(default)]
        transform: TransformDocument,
    },
    GeoJsonMap {
        asset: String,
        #[serde(default = "default_true")]
        visible: bool,
        #[serde(default = "default_map_radius_scale")]
        radius_scale: f32,
    },
    SphereGuide {
        guide: SphereGuideDocument,
        marker: char,
        #[serde(default = "default_true")]
        visible: bool,
        #[serde(default = "default_guide_radius_scale")]
        radius_scale: f32,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SphereGuideDocument {
    Equator,
    MeridianX,
    MeridianZ,
    Latitude { degrees: f32 },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorDocument {
    Spin {
        axis: AxisDocument,
        degrees_per_second: f32,
        #[serde(default = "default_true")]
        enabled: bool,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisDocument {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct TransformDocument {
    #[serde(default = "default_position")]
    pub position: [f32; 3],
    #[serde(default = "default_rotation")]
    pub rotation_degrees: [f32; 3],
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
}

impl Default for TransformDocument {
    fn default() -> Self {
        Self {
            position: default_position(),
            rotation_degrees: default_rotation(),
            scale: default_scale(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuadDocument {
    pub id: String,
    pub position: [f32; 3],
    pub size: [f32; 2],
    pub rotation_z_degrees: f32,
    pub marker: String,
    pub color: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_map_radius_scale() -> f32 {
    1.018
}
fn default_guide_radius_scale() -> f32 {
    1.01
}
fn default_position() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}
fn default_rotation() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}
fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

pub fn set_scene_document_visibility(
    document: &mut SceneDocument,
    path: &str,
    visible: bool,
) -> bool {
    for group in &mut document.groups {
        if set_group_document_visibility(group, "", path, visible) {
            return true;
        }
    }

    false
}

fn set_group_document_visibility(
    group: &mut GroupDocument,
    parent_path: &str,
    requested_path: &str,
    visible: bool,
) -> bool {
    let path = join_document_path(parent_path, &group.id);

    if path == requested_path {
        group.visible = visible;
        return true;
    }

    for child in &mut group.children {
        match child {
            NodeDocument::Group(child_group) => {
                if set_group_document_visibility(child_group, &path, requested_path, visible) {
                    return true;
                }
            }
            NodeDocument::Object(object) => {
                if join_document_path(&path, &object.id) == requested_path {
                    object.visible = visible;
                    return true;
                }
            }
        }
    }

    false
}

fn join_document_path(parent: &str, id: &str) -> String {
    if parent.is_empty() {
        id.to_string()
    } else {
        format!("{parent}/{id}")
    }
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

pub fn save_scene_document(path: impl AsRef<Path>, document: &SceneDocument) -> io::Result<()> {
    let path = path.as_ref();
    let text = serde_json::to_string_pretty(document).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize scene {}: {error}", path.display()),
        )
    })?;

    fs::write(path, format!("{text}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_scene_graph_round_trips() {
        let document = SceneDocument {
            name: "test".to_string(),
            mesh_asset: String::new(),
            display: DisplayDocument {
                world_scale: 2.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: Vec::new(),
            groups: vec![GroupDocument {
                id: "root".to_string(),
                name: "Root".to_string(),
                transform: TransformDocument::default(),
                visible: true,
                editor_composite: false,
                behaviors: Vec::new(),
                children: vec![NodeDocument::Group(GroupDocument {
                    id: "graticule".to_string(),
                    name: "Graticule".to_string(),
                    transform: TransformDocument::default(),
                    visible: true,
                    editor_composite: true,
                    behaviors: Vec::new(),
                    children: Vec::new(),
                })],
            }],
        };

        let json = serde_json::to_string_pretty(&document).unwrap();
        let decoded: SceneDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.groups.len(), 1);
        let NodeDocument::Group(graticule) = &decoded.groups[0].children[0] else {
            panic!("expected nested group");
        };
        assert!(graticule.editor_composite);
    }
}
