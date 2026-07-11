#[derive(Clone, Debug)]
pub struct RenderScene {
    pub name: String,
    pub display: RenderDisplay,
    pub lighting: Option<RenderLighting>,
    pub objects: Vec<RenderObject>,
    pub overlays: Vec<RenderOverlay>,
    pub groups: Vec<RenderGroup>,
    pub cameras: Vec<RenderCamera>,
    pub active_camera_id: Option<String>,
}

impl RenderScene {
    pub fn new(name: impl Into<String>, display: RenderDisplay) -> Self {
        Self {
            name: name.into(),
            display,
            lighting: None,
            objects: Vec::new(),
            overlays: Vec::new(),
            groups: Vec::new(),
            cameras: Vec::new(),
            active_camera_id: None,
        }
    }

    pub fn active_camera(&self) -> Option<&RenderCamera> {
        let active_camera_id = self.active_camera_id.as_deref()?;

        self.cameras
            .iter()
            .find(|camera| camera.id == active_camera_id)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderDisplay {
    pub world_scale: f32,
}

#[derive(Clone, Debug)]
pub struct RenderCamera {
    pub id: String,
    pub transform: RenderTransform,
    pub projection: RenderProjectionConfig,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderProjectionConfig {
    pub camera_distance: f32,
    pub near_clip: f32,
    pub vertical_center_ratio: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderLighting {
    pub primary_light_direction: [f32; 3],
}

#[derive(Clone, Debug)]
pub enum RenderObject {
    Mesh(RenderMeshObject),
    QuadGroup(RenderQuadGroup),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Debug)]
pub enum RenderBehavior {
    Spin(RenderSpinBehavior),
}

#[derive(Clone, Debug)]
pub struct RenderSpinBehavior {
    pub axis: RenderAxis,
    pub degrees_per_second: f32,
    pub enabled: bool,
}

impl RenderSpinBehavior {
    pub const fn new(axis: RenderAxis, degrees_per_second: f32) -> Self {
        Self {
            axis,
            degrees_per_second,
            enabled: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderGroup {
    pub id: String,
    pub name: String,
    pub transform: RenderTransform,
    pub visible: bool,
    pub behaviors: Vec<RenderBehavior>,
    pub children: Vec<RenderNode>,
}

impl RenderGroup {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            transform: RenderTransform::default(),
            visible: true,
            behaviors: Vec::new(),
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RenderNode {
    Group(RenderGroup),
    Object(RenderObjectNode),
}

#[derive(Clone, Debug)]
pub struct RenderObjectNode {
    pub id: String,
    pub name: String,
    pub transform: RenderTransform,
    pub visible: bool,
    pub behaviors: Vec<RenderBehavior>,
    pub object: RenderObject,
}

impl RenderObjectNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, object: RenderObject) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            transform: RenderTransform::default(),
            visible: true,
            behaviors: Vec::new(),
            object,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderMeshObject {
    pub mesh_asset: String,
    pub transform: RenderTransform,
}

#[derive(Clone, Debug)]
pub struct RenderQuadGroup {
    pub quads: Vec<RenderQuad>,
    pub transform: RenderTransform,
}

#[derive(Clone, Debug)]
pub struct RenderQuad {
    pub id: String,
    pub position: [f32; 3],
    pub size: [f32; 2],
    pub rotation_z_degrees: f32,
    pub marker: String,
    pub color: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderTransform {
    pub position: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for RenderTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Clone, Debug)]
pub enum RenderOverlay {
    GeoJsonMap(RenderGeoJsonMapOverlay),
    Text(RenderTextOverlay),
}

#[derive(Clone, Debug)]
pub struct RenderGeoJsonMapOverlay {
    pub asset: String,
    pub visible: bool,
    pub radius_scale: f32,
}

#[derive(Clone, Debug)]
pub struct RenderTextOverlay {
    pub x: usize,
    pub y: usize,
    pub text: String,
}

#[cfg(test)]
mod scene_graph_behavior_tests {
    use super::{RenderAxis, RenderBehavior, RenderGroup, RenderSpinBehavior};

    #[test]
    fn earth_group_can_carry_spin_behavior() {
        let mut earth = RenderGroup::new("earth", "Earth");

        earth
            .behaviors
            .push(RenderBehavior::Spin(RenderSpinBehavior::new(
                RenderAxis::Y,
                15.0,
            )));

        assert_eq!(earth.id, "earth");
        assert_eq!(earth.name, "Earth");
        assert!(earth.visible);
        assert!(earth.children.is_empty());
        assert_eq!(earth.behaviors.len(), 1);

        let RenderBehavior::Spin(spin) = &earth.behaviors[0];

        assert_eq!(spin.axis, RenderAxis::Y);
        assert_eq!(spin.degrees_per_second, 15.0);
        assert!(spin.enabled);
    }
}

#[cfg(test)]
mod render_scene_group_tests {
    use super::{RenderDisplay, RenderGroup, RenderScene};

    #[test]
    fn render_scene_starts_with_empty_groups() {
        let scene = RenderScene::new("test", RenderDisplay { world_scale: 1.0 });

        assert_eq!(scene.name, "test");
        assert!(scene.objects.is_empty());
        assert!(scene.overlays.is_empty());
        assert!(scene.groups.is_empty());
    }

    #[test]
    fn render_scene_can_store_root_group() {
        let mut scene = RenderScene::new("test", RenderDisplay { world_scale: 1.0 });

        scene.groups.push(RenderGroup::new("root", "Root"));

        assert_eq!(scene.groups.len(), 1);
        assert_eq!(scene.groups[0].id, "root");
        assert_eq!(scene.groups[0].name, "Root");
    }
}
