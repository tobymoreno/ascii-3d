use super::sphere_guides::GreatCircle;

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
    GeoJsonMap(RenderGeoJsonMapOverlay),
    SphereGuide(RenderSphereGuide),
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

pub fn apply_render_behaviors_to_group(group: &mut RenderGroup, elapsed_seconds: f32) {
    if !elapsed_seconds.is_finite() {
        return;
    }

    for behavior in &group.behaviors {
        match behavior {
            RenderBehavior::Spin(spin) => {
                if !spin.enabled || !spin.degrees_per_second.is_finite() {
                    continue;
                }

                let delta_degrees = spin.degrees_per_second * elapsed_seconds;

                match spin.axis {
                    RenderAxis::X => group.transform.rotation_degrees[0] += delta_degrees,
                    RenderAxis::Y => group.transform.rotation_degrees[1] += delta_degrees,
                    RenderAxis::Z => group.transform.rotation_degrees[2] += delta_degrees,
                }
            }
        }
    }
}

pub fn apply_render_behaviors_to_scene(scene: &mut RenderScene, elapsed_seconds: f32) {
    for group in &mut scene.groups {
        apply_render_behaviors_to_group_tree(group, elapsed_seconds);
    }
}

pub fn apply_render_behaviors_to_group_tree(group: &mut RenderGroup, elapsed_seconds: f32) {
    apply_render_behaviors_to_group(group, elapsed_seconds);

    for child in &mut group.children {
        match child {
            RenderNode::Group(child_group) => {
                apply_render_behaviors_to_group_tree(child_group, elapsed_seconds);
            }
            RenderNode::Object(object_node) => {
                apply_render_behaviors_to_object_node(object_node, elapsed_seconds);
            }
        }
    }
}

pub fn apply_render_behaviors_to_object_node(
    object_node: &mut RenderObjectNode,
    elapsed_seconds: f32,
) {
    if !elapsed_seconds.is_finite() {
        return;
    }

    for behavior in &object_node.behaviors {
        match behavior {
            RenderBehavior::Spin(spin) => {
                if !spin.enabled || !spin.degrees_per_second.is_finite() {
                    continue;
                }

                let delta_degrees = spin.degrees_per_second * elapsed_seconds;

                match spin.axis {
                    RenderAxis::X => object_node.transform.rotation_degrees[0] += delta_degrees,
                    RenderAxis::Y => object_node.transform.rotation_degrees[1] += delta_degrees,
                    RenderAxis::Z => object_node.transform.rotation_degrees[2] += delta_degrees,
                }
            }
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
pub enum RenderSphereGuideKind {
    GreatCircle(GreatCircle),
    Latitude(f32),
}

#[derive(Clone, Debug)]
pub struct RenderSphereGuide {
    pub kind: RenderSphereGuideKind,
    pub marker: char,
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

#[cfg(test)]
mod render_behavior_apply_tests {
    use super::{
        apply_render_behaviors_to_group, RenderAxis, RenderBehavior, RenderGroup,
        RenderSpinBehavior,
    };

    #[test]
    fn spin_behavior_rotates_group_around_y_axis() {
        let mut earth = RenderGroup::new("earth", "Earth");

        earth
            .behaviors
            .push(RenderBehavior::Spin(RenderSpinBehavior::new(
                RenderAxis::Y,
                15.0,
            )));

        apply_render_behaviors_to_group(&mut earth, 2.0);

        assert_eq!(earth.transform.rotation_degrees, [0.0, 30.0, 0.0]);
    }

    #[test]
    fn disabled_spin_behavior_does_not_change_transform() {
        let mut earth = RenderGroup::new("earth", "Earth");

        let mut spin = RenderSpinBehavior::new(RenderAxis::Y, 15.0);
        spin.enabled = false;

        earth.behaviors.push(RenderBehavior::Spin(spin));

        apply_render_behaviors_to_group(&mut earth, 2.0);

        assert_eq!(earth.transform.rotation_degrees, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn non_finite_elapsed_time_is_ignored() {
        let mut earth = RenderGroup::new("earth", "Earth");

        earth
            .behaviors
            .push(RenderBehavior::Spin(RenderSpinBehavior::new(
                RenderAxis::Y,
                15.0,
            )));

        apply_render_behaviors_to_group(&mut earth, f32::NAN);

        assert_eq!(earth.transform.rotation_degrees, [0.0, 0.0, 0.0]);
    }
}

#[cfg(test)]
mod recursive_render_behavior_apply_tests {
    use super::{
        apply_render_behaviors_to_scene, RenderAxis, RenderBehavior, RenderDisplay, RenderGroup,
        RenderNode, RenderScene, RenderSpinBehavior,
    };

    #[test]
    fn scene_behavior_application_reaches_nested_earth_group() {
        let mut earth = RenderGroup::new("earth", "Earth");
        earth
            .behaviors
            .push(RenderBehavior::Spin(RenderSpinBehavior::new(
                RenderAxis::Y,
                15.0,
            )));

        let mut root = RenderGroup::new("root", "Root");
        root.children.push(RenderNode::Group(earth));

        let mut scene = RenderScene::new("test", RenderDisplay { world_scale: 1.0 });
        scene.groups.push(root);

        apply_render_behaviors_to_scene(&mut scene, 2.0);

        let RenderNode::Group(earth) = &scene.groups[0].children[0] else {
            panic!("expected earth group");
        };

        assert_eq!(earth.transform.rotation_degrees, [0.0, 30.0, 0.0]);
    }
}

