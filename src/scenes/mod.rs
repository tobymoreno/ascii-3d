use serde::Deserialize;
use std::{fs, io, path::{Path, PathBuf}};

mod arbitrary_vector;
mod asset_axes;
mod asset_axes_rotation;
mod axes;
mod bezier_axes;
mod camera;
mod camera_motion;
mod camera_turntable;
mod crew;
mod cross_product;
mod obj_box;
mod pitt;
mod pitt_crew;
mod quad4;
mod rotation;
mod single_c;
mod single_e;
mod single_i;
mod single_p;
mod single_r;
mod single_t;
mod single_w;
mod world_camera_spaces;

pub use arbitrary_vector::render as render_arbitrary_vector;
pub use asset_axes::render as render_asset_axes;
pub use asset_axes_rotation::render as render_asset_axes_rotation;
pub use axes::{draw_axes, render as render_axes};
pub use bezier_axes::render as render_bezier_axes;
pub use camera::render as render_camera;
pub use camera_motion::render as render_camera_motion;
pub use camera_turntable::render as render_camera_turntable;
pub use crew::render as render_crew;
pub use cross_product::{
    render_negative_z as render_cross_negative_z, render_positive_z as render_cross_positive_z,
};
pub use obj_box::render as render_obj_box;
pub use pitt::render as render_pitt;
pub use pitt_crew::render as render_pitt_crew;
pub use quad4::render as render_quad4;
pub use rotation::{RotationAxis, render as render_rotation};
pub use single_c::render as render_single_c;
pub use single_e::render as render_single_e;
pub use single_i::render as render_single_i;
pub use single_p::render as render_single_p;
pub use single_r::render as render_single_r;
pub use single_t::render as render_single_t;
pub use single_w::render as render_single_w;
pub use world_camera_spaces::render as render_world_camera_spaces;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDescriptor {
    pub scene: Scene,
    pub id: String,
    pub title: String,
    pub index: usize,
    pub animated: bool,
    pub a3d_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    LoadedA3d,
    WorldCameraSpaces,
    PittCrew,
    Crew,
    Pitt,
    SingleE,
    SingleW,
    SingleC,
    SingleR,
    SingleT,
    SingleI,
    SingleP,
    BezierAxes,
    AssetAxesRotateX,
    AssetAxesRotateY,
    AssetAxesRotateZ,
    Quad4,
    CameraMotion,
    CameraTurntable,
    CameraLookAt,
    ObjBox,
    RotateAxesZ,
    RotateAxesY,
    RotateAxesX,
    CrossNegativeZ,
    CrossPositiveZ,
    ArbitraryVector,
    Axes,
}

impl Scene {
    /// Scenes are ordered newest-first.
    pub const ALL: [Self; 28] = [
        Self::LoadedA3d,
        Self::WorldCameraSpaces,
        Self::PittCrew,
        Self::Crew,
        Self::Pitt,
        Self::SingleE,
        Self::SingleW,
        Self::SingleC,
        Self::SingleR,
        Self::SingleT,
        Self::SingleI,
        Self::SingleP,
        Self::BezierAxes,
        Self::AssetAxesRotateX,
        Self::AssetAxesRotateY,
        Self::AssetAxesRotateZ,
        Self::Quad4,
        Self::CameraMotion,
        Self::CameraTurntable,
        Self::CameraLookAt,
        Self::ObjBox,
        Self::RotateAxesZ,
        Self::RotateAxesY,
        Self::RotateAxesX,
        Self::CrossNegativeZ,
        Self::CrossPositiveZ,
        Self::ArbitraryVector,
        Self::Axes,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::LoadedA3d => "loaded_a3d",
            Self::WorldCameraSpaces => "world_camera_spaces",
            Self::PittCrew => "pitt_crew",
            Self::Crew => "crew",
            Self::Pitt => "pitt",
            Self::SingleE => "single_e",
            Self::SingleW => "single_w",
            Self::SingleC => "single_c",
            Self::SingleR => "single_r",
            Self::SingleT => "single_t",
            Self::SingleI => "single_i",
            Self::SingleP => "single_p",
            Self::BezierAxes => "bezier_axes",
            Self::AssetAxesRotateX => "asset_axes_rotate_x",
            Self::AssetAxesRotateY => "asset_axes_rotate_y",
            Self::AssetAxesRotateZ => "asset_axes_rotate_z",
            Self::Quad4 => "quad4",
            Self::CameraMotion => "camera_motion",
            Self::CameraTurntable => "camera_turntable",
            Self::CameraLookAt => "camera_look_at",
            Self::ObjBox => "obj_box",
            Self::RotateAxesZ => "rotate_axes_z",
            Self::RotateAxesY => "rotate_axes_y",
            Self::RotateAxesX => "rotate_axes_x",
            Self::CrossNegativeZ => "cross_negative_z",
            Self::CrossPositiveZ => "cross_positive_z",
            Self::ArbitraryVector => "arbitrary_vector",
            Self::Axes => "axes",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "loaded_a3d" => Some(Self::LoadedA3d),
            "world_camera_spaces" => Some(Self::WorldCameraSpaces),
            "pitt_crew" => Some(Self::PittCrew),
            "crew" => Some(Self::Crew),
            "pitt" => Some(Self::Pitt),
            "single_e" => Some(Self::SingleE),
            "single_w" => Some(Self::SingleW),
            "single_c" => Some(Self::SingleC),
            "single_r" => Some(Self::SingleR),
            "single_t" => Some(Self::SingleT),
            "single_i" => Some(Self::SingleI),
            "single_p" => Some(Self::SingleP),
            "bezier_axes" => Some(Self::BezierAxes),
            "asset_axes_rotate_x" => Some(Self::AssetAxesRotateX),
            "asset_axes_rotate_y" => Some(Self::AssetAxesRotateY),
            "asset_axes_rotate_z" => Some(Self::AssetAxesRotateZ),
            "quad4" => Some(Self::Quad4),
            "camera_motion" => Some(Self::CameraMotion),
            "camera_turntable" => Some(Self::CameraTurntable),
            "camera_look_at" => Some(Self::CameraLookAt),
            "obj_box" => Some(Self::ObjBox),
            "rotate_axes_z" => Some(Self::RotateAxesZ),
            "rotate_axes_y" => Some(Self::RotateAxesY),
            "rotate_axes_x" => Some(Self::RotateAxesX),
            "cross_negative_z" => Some(Self::CrossNegativeZ),
            "cross_positive_z" => Some(Self::CrossPositiveZ),
            "arbitrary_vector" => Some(Self::ArbitraryVector),
            "axes" => Some(Self::Axes),
            _ => None,
        }
    }

    pub fn descriptor(self, index: usize) -> SceneDescriptor {
        SceneDescriptor {
            scene: self,
            id: self.id().to_string(),
            title: self.title().to_string(),
            index,
            animated: self.is_animated(),
            a3d_root: None,
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::LoadedA3d => "Loaded .a3d data-driven world",
            Self::WorldCameraSpaces => "world space and Camera3D foundation",
            Self::PittCrew => "PITT CREW word parent with P/I/T/T SPACE C/R/E/W glyphs",
            Self::Crew => "CREW word parent with C/R/E/W glyphs",
            Self::Pitt => "PITT word parent with P/I/T/T glyphs",
            Self::SingleE => "single_e word parent with E glyph",
            Self::SingleW => "single_w word parent with W glyph",
            Self::SingleC => "single_c word parent with C glyph",
            Self::SingleR => "single_r word parent with R glyph",
            Self::SingleT => "single_t word parent with T glyph",
            Self::SingleI => "single_i word parent with I glyph",
            Self::SingleP => "single_p word parent with P glyph",
            Self::BezierAxes => "Bezier curve child of Cartesian axes",
            Self::AssetAxesRotateX => "asset Cartesian axes rotating around X",
            Self::AssetAxesRotateY => "asset Cartesian axes rotating around Y",
            Self::AssetAxesRotateZ => "asset Cartesian axes rotating around Z",
            Self::Quad4 => "loaded quad4.obj projected in XYZ space",
            Self::CameraMotion => "animated camera motion",
            Self::CameraTurntable => "camera Y-turntable inspection",
            Self::CameraLookAt => "look_at camera basis",
            Self::ObjBox => "rotating OBJ wireframe box",
            Self::RotateAxesZ => "rotate Cartesian axes around Z",
            Self::RotateAxesY => "rotate Cartesian axes around Y",
            Self::RotateAxesX => "rotate Cartesian axes around X",
            Self::CrossNegativeZ => "B x A points along -Z",
            Self::CrossPositiveZ => "A x B points along +Z",
            Self::ArbitraryVector => "arbitrary Vec3",
            Self::Axes => "3D Cartesian axes",
        }
    }

    pub const fn is_animated(self) -> bool {
        matches!(
            self,
            Self::LoadedA3d
                | Self::AssetAxesRotateX
                | Self::AssetAxesRotateY
                | Self::AssetAxesRotateZ
                | Self::CameraMotion
                | Self::CameraTurntable
                | Self::ObjBox
                | Self::RotateAxesX
                | Self::RotateAxesY
                | Self::RotateAxesZ
        )
    }
}

const SCENE_INDEX_ASSET: &str = "assets/scenes/index.json";

#[derive(Debug, Deserialize)]
struct SceneIndexAsset {
    version: u32,
    #[allow(dead_code)]
    default: Option<String>,
    scenes: Vec<SceneIndexEntry>,
}

#[derive(Debug, Deserialize)]
struct SceneIndexEntry {
    id: String,
    scene: Option<String>,
    title: Option<String>,
    a3d_root: Option<PathBuf>,
}

fn builtin_registry() -> Vec<SceneDescriptor> {
    Scene::ALL
        .iter()
        .enumerate()
        .map(|(index, scene)| scene.descriptor(index))
        .collect()
}

pub fn registry_from_index_path(path: impl AsRef<Path>) -> io::Result<Vec<SceneDescriptor>> {
    let source = fs::read_to_string(path)?;
    let index: SceneIndexAsset = serde_json::from_str(&source)
        .map_err(|error| io::Error::other(format!("failed to parse scene index: {error}")))?;

    if index.version != 1 {
        return Err(io::Error::other(format!(
            "unsupported scene index version {}",
            index.version
        )));
    }

    let mut scenes = Vec::with_capacity(index.scenes.len());

    for entry in index.scenes {
        let scene_id = entry.scene.as_deref().unwrap_or(&entry.id);
        let scene = Scene::from_id(scene_id)
            .ok_or_else(|| io::Error::other(format!("unknown scene id '{}'", scene_id)))?;

        scenes.push(SceneDescriptor {
            scene,
            id: entry.id,
            title: entry.title.unwrap_or_else(|| scene.title().to_string()),
            index: scenes.len(),
            animated: scene.is_animated(),
            a3d_root: entry.a3d_root,
        });
    }

    if scenes.is_empty() {
        return Err(io::Error::other("scene index must include at least one scene"));
    }

    Ok(scenes)
}

pub fn registry() -> Vec<SceneDescriptor> {
    registry_from_index_path(SCENE_INDEX_ASSET).unwrap_or_else(|_| builtin_registry())
}

pub fn scene_count() -> usize {
    registry().len()
}

pub fn scene_descriptor_at(index: usize) -> SceneDescriptor {
    let registry = registry();
    registry[index % registry.len()].clone()
}

pub fn scene_at(index: usize) -> Scene {
    scene_descriptor_at(index).scene
}

#[cfg(test)]
mod tests {
    use super::Scene;

    #[test]
    fn newest_scene_is_loaded_a3d() {
        assert_eq!(Scene::ALL.first(), Some(&Scene::LoadedA3d));
    }

    #[test]
    fn next_scene_after_loaded_a3d_is_world_camera_spaces() {
        assert_eq!(Scene::ALL[1], Scene::WorldCameraSpaces);
        assert_eq!(Scene::ALL[2], Scene::PittCrew);
    }

    #[test]
    fn quad4_follows_asset_axes_rotation_scenes() {
        let quad4_index = Scene::ALL
            .iter()
            .position(|scene| *scene == Scene::Quad4)
            .expect("Quad4 should be present");

        assert_eq!(Scene::ALL[quad4_index - 1], Scene::AssetAxesRotateZ);
    }

    #[test]
    fn oldest_scene_is_last() {
        assert_eq!(Scene::ALL.last(), Some(&Scene::Axes));
    }

    #[test]
    fn scene_count_matches_scene_all() {
        assert_eq!(Scene::ALL.len(), 28);
    }

    #[test]
    fn registry_contains_dynamic_scene_index_order() {
        let registry = super::registry();

        assert!(registry.len() >= Scene::ALL.len());
        assert_eq!(registry[0].id, "glyph_ab");
        assert_eq!(registry[0].scene, Scene::LoadedA3d);
        assert_eq!(registry[0].title, "Glyph A and B");
        assert_eq!(registry[0].index, 0);
        assert!(registry.iter().any(|descriptor| descriptor.id == "world_camera_spaces"));
        assert!(registry.iter().any(|descriptor| descriptor.id == "pitt_crew"));
    }

    #[test]
    fn scene_index_asset_loads_dynamic_scene_order() {
        let registry = super::registry_from_index_path("assets/scenes/index.json")
            .expect("scene index should load");

        assert!(registry.len() >= Scene::ALL.len());
        assert_eq!(registry[0].id, "glyph_ab");
        assert_eq!(registry[0].scene, Scene::LoadedA3d);
        assert_eq!(registry[0].a3d_root.as_deref(), Some(std::path::Path::new("assets/a3d/glyph_ab")));
        assert_eq!(registry.last().map(|descriptor| descriptor.scene), Some(Scene::Axes));
    }

    #[test]
    fn scene_ids_round_trip_from_index() {
        for scene in Scene::ALL {
            assert_eq!(Scene::from_id(scene.id()), Some(scene));
        }
    }

    #[test]
    fn animated_scenes_are_identified() {
        assert!(Scene::LoadedA3d.is_animated());
        assert!(!Scene::WorldCameraSpaces.is_animated());
        assert!(!Scene::PittCrew.is_animated());
        assert!(!Scene::Crew.is_animated());
        assert!(!Scene::Pitt.is_animated());
        assert!(!Scene::SingleE.is_animated());
        assert!(!Scene::SingleW.is_animated());
        assert!(!Scene::SingleC.is_animated());
        assert!(!Scene::SingleR.is_animated());
        assert!(!Scene::SingleT.is_animated());
        assert!(!Scene::SingleI.is_animated());
        assert!(!Scene::SingleP.is_animated());
        assert!(!Scene::BezierAxes.is_animated());

        assert!(Scene::AssetAxesRotateX.is_animated());
        assert!(Scene::AssetAxesRotateY.is_animated());
        assert!(Scene::AssetAxesRotateZ.is_animated());
        assert!(Scene::CameraMotion.is_animated());
        assert!(Scene::CameraTurntable.is_animated());
        assert!(Scene::ObjBox.is_animated());
        assert!(Scene::RotateAxesX.is_animated());
        assert!(Scene::RotateAxesY.is_animated());
        assert!(Scene::RotateAxesZ.is_animated());

        assert!(!Scene::Quad4.is_animated());
        assert!(!Scene::CameraLookAt.is_animated());
        assert!(!Scene::Axes.is_animated());
    }
    #[test]
    fn dynamic_scene_index_asset_exists_and_loads() {
        let registry = super::registry_from_index_path("assets/scenes/index.json")
            .expect("dynamic scene index should load");

        assert!(!registry.is_empty());
        assert_eq!(registry[0].id, "glyph_ab");
        assert_eq!(registry[0].scene, Scene::LoadedA3d);
        assert_eq!(registry[0].title, "Glyph A and B");
        assert_eq!(
            registry[0].a3d_root.as_deref(),
            Some(std::path::Path::new("assets/a3d/glyph_ab"))
        );
    }

    #[test]
    fn dynamic_scene_index_a3d_roots_have_scene_manifests() {
        let registry = super::registry_from_index_path("assets/scenes/index.json")
            .expect("dynamic scene index should load");

        let mut checked_roots = 0;

        for descriptor in registry {
            let Some(root) = descriptor.a3d_root else {
                continue;
            };

            checked_roots += 1;
            assert!(
                root.join("scene.a3d").is_file(),
                "dynamic scene '{}' points to missing manifest {}",
                descriptor.id,
                root.join("scene.a3d").display()
            );
        }

        assert!(checked_roots > 0, "expected at least one dynamic a3d_root scene");
    }

    #[test]
    fn dynamic_scene_index_entries_resolve_to_known_renderers() {
        let source = std::fs::read_to_string("assets/scenes/index.json")
            .expect("dynamic scene index should be readable");
        let json: serde_json::Value =
            serde_json::from_str(&source).expect("dynamic scene index should be valid JSON");

        let scenes = json
            .get("scenes")
            .and_then(serde_json::Value::as_array)
            .expect("scene index should contain scenes array");

        assert!(!scenes.is_empty());

        for entry in scenes {
            let id = entry
                .get("id")
                .and_then(serde_json::Value::as_str)
                .expect("scene index entry should have id");

            assert!(!id.trim().is_empty(), "scene index entry id should not be empty");

            let renderer_id = entry
                .get("scene")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(id);

            assert!(
                Scene::from_id(renderer_id).is_some(),
                "scene index entry '{}' references unknown renderer '{}'",
                id,
                renderer_id
            );
        }
    }

    #[test]
    fn glyph_ab_a3d_scene_references_existing_word_assets() {
        let root = std::path::Path::new("assets/a3d/glyph_ab");
        let manifest_path = root.join("scene.a3d");
        let source = std::fs::read_to_string(&manifest_path)
            .expect("glyph_ab scene manifest should be readable");
        let json: serde_json::Value =
            serde_json::from_str(&source).expect("glyph_ab scene manifest should be valid JSON");

        let objects = json
            .get("objects")
            .and_then(serde_json::Value::as_array)
            .expect("glyph_ab scene should contain objects");

        let mut checked_word_assets = 0;

        for object in objects {
            let asset = object.get("asset").expect("object should have asset");
            let asset_type = asset
                .get("type")
                .and_then(serde_json::Value::as_str)
                .expect("object asset should have type");

            if asset_type != "word" {
                continue;
            }

            let relative_path = asset
                .get("path")
                .and_then(serde_json::Value::as_str)
                .expect("word asset should have path");

            let resolved = root.join(relative_path);
            checked_word_assets += 1;

            assert!(
                resolved.is_file(),
                "glyph_ab references missing word asset {}",
                resolved.display()
            );
        }

        assert_eq!(checked_word_assets, 2, "glyph_ab should reference A and B word assets");
    }

    #[test]
    fn single_a_and_b_word_assets_reference_existing_glyph_assets() {
        for word_asset in [
            "assets/words/single_a.word.json",
            "assets/words/single_b.word.json",
        ] {
            let source = std::fs::read_to_string(word_asset)
                .unwrap_or_else(|error| panic!("failed to read {word_asset}: {error}"));
            let json: serde_json::Value =
                serde_json::from_str(&source).expect("word asset should be valid JSON");

            let children = json
                .get("children")
                .and_then(serde_json::Value::as_array)
                .expect("word asset should contain children");

            assert_eq!(children.len(), 1, "{word_asset} should contain one glyph child");

            for child in children {
                for key in ["glyph_asset", "metadata_asset"] {
                    let asset_path = child
                        .get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_else(|| panic!("{word_asset} child should have {key}"));

                    assert!(
                        std::path::Path::new(asset_path).is_file(),
                        "{word_asset} references missing {key}: {asset_path}"
                    );
                }
            }
        }
    }

}
