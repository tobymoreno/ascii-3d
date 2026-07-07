use serde::Deserialize;
use std::{fs, io, path::Path};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneDescriptor {
    pub scene: Scene,
    pub id: &'static str,
    pub title: &'static str,
    pub index: usize,
    pub animated: bool,
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

    pub const fn descriptor(self, index: usize) -> SceneDescriptor {
        SceneDescriptor {
            scene: self,
            id: self.id(),
            title: self.title(),
            index,
            animated: self.is_animated(),
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
    scenes: Vec<SceneIndexEntry>,
}

#[derive(Debug, Deserialize)]
struct SceneIndexEntry {
    id: String,
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
        let scene = Scene::from_id(&entry.id)
            .ok_or_else(|| io::Error::other(format!("unknown scene id '{}'", entry.id)))?;

        scenes.push(scene.descriptor(scenes.len()));
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

pub fn scene_at(index: usize) -> Scene {
    let registry = registry();
    registry[index % registry.len()].scene
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
    fn registry_contains_each_scene_in_all_order() {
        let registry = super::registry();

        assert_eq!(registry.len(), Scene::ALL.len());
        assert_eq!(registry[0].scene, Scene::LoadedA3d);
        assert_eq!(registry[0].id, "loaded_a3d");
        assert_eq!(registry[0].index, 0);
        assert_eq!(registry[1].scene, Scene::WorldCameraSpaces);
        assert_eq!(registry[2].scene, Scene::PittCrew);
    }

    #[test]
    fn scene_index_asset_loads_existing_scene_order() {
        let registry = super::registry_from_index_path("assets/scenes/index.json")
            .expect("scene index should load");

        assert_eq!(registry.len(), Scene::ALL.len());
        assert_eq!(registry[0].scene, Scene::LoadedA3d);
        assert_eq!(registry[1].scene, Scene::WorldCameraSpaces);
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
}
