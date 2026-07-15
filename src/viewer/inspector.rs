use crate::render::{
    RenderAxis, RenderBehavior, RenderGroup, RenderNode, RenderObject, RenderScene,
    RenderSphereGuideKind, RenderTransform,
};
use crossterm::event::KeyCode;

pub const VIEWER_MENU_TITLES: &[&str] = &["File", "Objects", "View", "Help"];
pub const FILE_MENU_INDEX: usize = 0;
pub const OBJECTS_MENU_INDEX: usize = 1;

pub const SCENE_HELPER_ROOT_PATH: &str = "@scene";
pub const CAMERA_HELPER_PATH: &str = "@scene/camera";
pub const LIGHT_HELPER_PATH: &str = "@scene/light";
pub const WORLD_AXES_HELPER_PATH: &str = "@scene/world-axes";
pub const SCENE_ORIGIN_HELPER_PATH: &str = "@scene/scene-origin";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneObjectKind {
    SceneHelpers,
    Camera,
    Light,
    WorldAxes,
    SceneOrigin,
    Group,
    Mesh,
    QuadGroup,
    GeoJsonMap,
    SphereGuide,
}

impl SceneObjectKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SceneHelpers => "runtime",
            Self::Camera => "camera",
            Self::Light => "light",
            Self::WorldAxes => "settings",
            Self::SceneOrigin => "origin",
            Self::Group => "group",
            Self::Mesh => "mesh",
            Self::QuadGroup => "quads",
            Self::GeoJsonMap => "map",
            Self::SphereGuide => "guide",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneObjectEntry {
    pub path: String,
    pub id: String,
    pub name: String,
    pub depth: usize,
    pub kind: SceneObjectKind,
    pub visible: bool,
}

impl SceneObjectEntry {
    pub fn display_label(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let visibility = if self.visible { "[on] " } else { "[off]" };
        format!("{indent}{visibility} {} ({})", self.name, self.kind.label())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ViewerInspectorState {
    pub menu_focused: bool,
    pub selected_menu: usize,
    pub file_open: bool,
    pub selected_file_item: usize,
    pub save_as_open: bool,
    pub save_as_path: String,
    pub objects_open: bool,
    pub properties_open: bool,
    pub selected_object: usize,
    pub active_object_path: Option<String>,
    pub active_xyz_target_path: String,
    pub selected_property_item: usize,
}

impl ViewerInspectorState {
    pub fn focus_menu(&mut self) {
        self.menu_focused = true;
    }

    pub fn close_popup(&mut self) {
        self.file_open = false;
        self.save_as_open = false;
        self.objects_open = false;
        self.properties_open = false;
        self.menu_focused = false;
    }

    pub fn close_properties(&mut self) {
        self.properties_open = false;
        self.objects_open = true;
        self.menu_focused = true;
    }

    pub fn move_menu_left(&mut self) {
        let count = VIEWER_MENU_TITLES.len();
        self.selected_menu = if self.selected_menu == 0 {
            count - 1
        } else {
            self.selected_menu - 1
        };
    }

    pub fn move_menu_right(&mut self) {
        self.selected_menu = (self.selected_menu + 1) % VIEWER_MENU_TITLES.len();
    }

    pub fn open_selected_menu(&mut self, object_count: usize) {
        self.file_open = self.selected_menu == FILE_MENU_INDEX;
        self.objects_open = self.selected_menu == OBJECTS_MENU_INDEX;
        self.properties_open = false;

        if self.file_open {
            self.selected_file_item = 0;
        }

        if self.objects_open {
            self.selected_object = self.selected_object.min(object_count.saturating_sub(1));
        }
    }

    pub fn move_file_up(&mut self) {
        self.selected_file_item = if self.selected_file_item == 0 { 1 } else { 0 };
    }

    pub fn move_file_down(&mut self) {
        self.selected_file_item = (self.selected_file_item + 1) % 2;
    }

    pub fn open_save_as(&mut self, path: impl Into<String>) {
        self.file_open = false;
        self.save_as_open = true;
        self.save_as_path = path.into();
    }

    pub fn close_save_as(&mut self) {
        self.save_as_open = false;
        self.menu_focused = false;
    }

    pub fn move_object_up(&mut self, object_count: usize) {
        if object_count == 0 {
            self.selected_object = 0;
            return;
        }
        self.selected_object = if self.selected_object == 0 {
            object_count - 1
        } else {
            self.selected_object - 1
        };
    }

    pub fn move_object_down(&mut self, object_count: usize) {
        if object_count == 0 {
            self.selected_object = 0;
            return;
        }
        self.selected_object = (self.selected_object + 1) % object_count;
    }

    pub fn activate_selected(&mut self, entries: &[SceneObjectEntry]) {
        let Some(entry) = entries.get(self.selected_object) else {
            return;
        };

        self.active_object_path = Some(entry.path.clone());
        self.selected_property_item = 0;
        self.objects_open = false;
        self.properties_open = true;
        self.menu_focused = true;
    }

    pub fn move_property_up(&mut self, item_count: usize) {
        if item_count == 0 {
            self.selected_property_item = 0;
        } else if self.selected_property_item == 0 {
            self.selected_property_item = item_count - 1;
        } else {
            self.selected_property_item -= 1;
        }
    }

    pub fn move_property_down(&mut self, item_count: usize) {
        if item_count == 0 {
            self.selected_property_item = 0;
        } else {
            self.selected_property_item = (self.selected_property_item + 1) % item_count;
        }
    }

    pub fn active_label<'a>(&self, entries: &'a [SceneObjectEntry]) -> Option<&'a str> {
        let active_path = self.active_object_path.as_deref()?;
        entries
            .iter()
            .find(|entry| entry.path == active_path)
            .map(|entry| entry.name.as_str())
    }
}

pub fn collect_scene_objects(scene: &RenderScene) -> Vec<SceneObjectEntry> {
    let mut entries = Vec::new();

    for group in &scene.groups {
        collect_group(group, "", 0, &mut entries);
    }

    entries
}

pub fn collect_scene_objects_with_helpers(
    scene: &RenderScene,
    _world_axes_visible: bool,
) -> Vec<SceneObjectEntry> {
    let mut entries = vec![
        SceneObjectEntry {
            path: CAMERA_HELPER_PATH.to_string(),
            id: "camera".to_string(),
            name: "Camera".to_string(),
            depth: 0,
            kind: SceneObjectKind::Camera,
            visible: true,
        },
        SceneObjectEntry {
            path: SCENE_ORIGIN_HELPER_PATH.to_string(),
            id: "scene-origin".to_string(),
            name: "Scene Origin".to_string(),
            depth: 0,
            kind: SceneObjectKind::SceneOrigin,
            visible: true,
        },
    ];

    for group in &scene.groups {
        collect_group(group, "", 0, &mut entries);
    }

    entries
}

pub fn scene_helper_property_lines(
    path: &str,
    _world_axes_visible: bool,
    active_path: Option<&str>,
) -> Option<Vec<String>> {
    let xyz = if active_path == Some(path) {
        "xyz control: active"
    } else {
        "xyz control: inactive"
    };

    let lines = match path {
        CAMERA_HELPER_PATH => vec![
            xyz.to_string(),
            "kind: viewport camera".to_string(),
            "saved: no".to_string(),
            "mode: runtime only".to_string(),
        ],
        SCENE_ORIGIN_HELPER_PATH => vec![
            xyz.to_string(),
            "kind: runtime scene origin".to_string(),
            "saved: no".to_string(),
            "mode: runtime only".to_string(),
        ],
        _ => return None,
    };

    Some(lines)
}

pub fn set_scene_object_visibility(scene: &mut RenderScene, path: &str, visible: bool) -> bool {
    for group in &mut scene.groups {
        if set_group_visibility(group, "", path, visible) {
            return true;
        }
    }

    false
}

pub fn toggle_scene_object_visibility(scene: &mut RenderScene, path: &str) -> Option<bool> {
    let current = scene_object_visibility(scene, path)?;
    let next = !current;

    if set_scene_object_visibility(scene, path, next) {
        Some(next)
    } else {
        None
    }
}

pub fn scene_object_visibility(scene: &RenderScene, path: &str) -> Option<bool> {
    for group in &scene.groups {
        if let Some(visible) = group_visibility(group, "", path) {
            return Some(visible);
        }
    }

    None
}

pub fn scene_object_property_lines(
    scene: &RenderScene,
    path: &str,
    active_path: Option<&str>,
) -> Option<Vec<String>> {
    for group in &scene.groups {
        if let Some(mut lines) = group_property_lines(group, "", path) {
            lines.insert(
                0,
                if active_path == Some(path) {
                    "xyz control: active".to_string()
                } else {
                    "xyz control: inactive".to_string()
                },
            );
            lines.insert(1, "mode: runtime only".to_string());
            lines.insert(2, "saved: no".to_string());
            return Some(lines);
        }
    }
    None
}

pub fn reset_scene_object_transform(scene: &mut RenderScene, path: &str) -> bool {
    for group in &mut scene.groups {
        if reset_group_transform(group, "", path) {
            return true;
        }
    }
    false
}

fn reset_group_transform(group: &mut RenderGroup, parent_path: &str, requested_path: &str) -> bool {
    let path = join_path(parent_path, &group.id);

    if path == requested_path {
        group.transform.position = [0.0, 0.0, 0.0];
        group.transform.rotation_degrees = [0.0, 0.0, 0.0];
        group.transform.scale = [1.0, 1.0, 1.0];
        return true;
    }

    for node in &mut group.children {
        match node {
            RenderNode::Group(child_group) => {
                if reset_group_transform(child_group, &path, requested_path) {
                    return true;
                }
            }
            RenderNode::Object(object_node) => {
                if join_path(&path, &object_node.id) == requested_path {
                    object_node.transform.position = [0.0, 0.0, 0.0];
                    object_node.transform.rotation_degrees = [0.0, 0.0, 0.0];
                    object_node.transform.scale = [1.0, 1.0, 1.0];
                    return true;
                }
            }
        }
    }

    false
}

pub fn handle_scene_object_transform_key(
    scene: &mut RenderScene,
    path: &str,
    code: KeyCode,
) -> bool {
    for group in &mut scene.groups {
        if handle_group_transform_key(group, "", path, code) {
            return true;
        }
    }
    false
}

fn handle_group_transform_key(
    group: &mut RenderGroup,
    parent_path: &str,
    requested_path: &str,
    code: KeyCode,
) -> bool {
    let path = join_path(parent_path, &group.id);

    if path == requested_path {
        apply_transform_key(&mut group.transform, code);
        return true;
    }

    for node in &mut group.children {
        match node {
            RenderNode::Group(child_group) => {
                if handle_group_transform_key(child_group, &path, requested_path, code) {
                    return true;
                }
            }
            RenderNode::Object(object_node) => {
                if join_path(&path, &object_node.id) == requested_path {
                    apply_transform_key(&mut object_node.transform, code);
                    return true;
                }
            }
        }
    }

    false
}

const MIN_OBJECT_SCALE: f32 = 0.01;
const MAX_OBJECT_SCALE: f32 = 100.0;
const OBJECT_SCALE_FACTOR: f32 = 1.1;

fn scaled_component(value: f32, factor: f32) -> f32 {
    if !value.is_finite() {
        return 1.0;
    }

    (value * factor).clamp(MIN_OBJECT_SCALE, MAX_OBJECT_SCALE)
}

fn apply_transform_key(transform: &mut RenderTransform, code: KeyCode) {
    match code {
        KeyCode::Left => transform.position[0] -= 0.5,
        KeyCode::Right => transform.position[0] += 0.5,
        KeyCode::Up => transform.position[1] += 0.5,
        KeyCode::Down => transform.position[1] -= 0.5,
        KeyCode::PageUp => transform.position[2] += 0.5,
        KeyCode::PageDown => transform.position[2] -= 0.5,
        KeyCode::Char('x') => transform.rotation_degrees[0] += 2.0,
        KeyCode::Char('X') => transform.rotation_degrees[0] -= 2.0,
        KeyCode::Char('y') => transform.rotation_degrees[1] += 2.0,
        KeyCode::Char('Y') => transform.rotation_degrees[1] -= 2.0,
        KeyCode::Char('z') => transform.rotation_degrees[2] += 2.0,
        KeyCode::Char('Z') => transform.rotation_degrees[2] -= 2.0,
        KeyCode::Char('+') | KeyCode::Char('=') => {
            for component in &mut transform.scale {
                *component = scaled_component(*component, OBJECT_SCALE_FACTOR);
            }
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            for component in &mut transform.scale {
                *component = scaled_component(*component, 1.0 / OBJECT_SCALE_FACTOR);
            }
        }
        KeyCode::Char('0') => {
            transform.position = [0.0, 0.0, 0.0];
            transform.rotation_degrees = [0.0, 0.0, 0.0];
            transform.scale = [1.0, 1.0, 1.0];
        }
        _ => {}
    }
}

fn collect_group(
    group: &RenderGroup,
    parent_path: &str,
    depth: usize,
    entries: &mut Vec<SceneObjectEntry>,
) {
    let path = join_path(parent_path, &group.id);
    entries.push(SceneObjectEntry {
        path: path.clone(),
        id: group.id.clone(),
        name: group.name.clone(),
        depth,
        kind: SceneObjectKind::Group,
        visible: group.visible,
    });

    if group.editor_composite {
        return;
    }

    for node in &group.children {
        match node {
            RenderNode::Group(child_group) => collect_group(child_group, &path, depth + 1, entries),
            RenderNode::Object(object_node) => entries.push(SceneObjectEntry {
                path: join_path(&path, &object_node.id),
                id: object_node.id.clone(),
                name: object_node.name.clone(),
                depth: depth + 1,
                kind: object_kind(&object_node.object),
                visible: object_node.visible && object_visible(&object_node.object),
            }),
        }
    }
}

fn set_group_visibility(
    group: &mut RenderGroup,
    parent_path: &str,
    requested_path: &str,
    visible: bool,
) -> bool {
    let path = join_path(parent_path, &group.id);

    if path == requested_path {
        group.visible = visible;
        return true;
    }

    for node in &mut group.children {
        match node {
            RenderNode::Group(child_group) => {
                if set_group_visibility(child_group, &path, requested_path, visible) {
                    return true;
                }
            }
            RenderNode::Object(object_node) => {
                let object_path = join_path(&path, &object_node.id);

                if object_path == requested_path {
                    object_node.visible = visible;
                    return true;
                }
            }
        }
    }

    false
}

fn group_visibility(group: &RenderGroup, parent_path: &str, requested_path: &str) -> Option<bool> {
    let path = join_path(parent_path, &group.id);

    if path == requested_path {
        return Some(group.visible);
    }

    for node in &group.children {
        match node {
            RenderNode::Group(child_group) => {
                if let Some(visible) = group_visibility(child_group, &path, requested_path) {
                    return Some(visible);
                }
            }
            RenderNode::Object(object_node) => {
                let object_path = join_path(&path, &object_node.id);

                if object_path == requested_path {
                    return Some(object_node.visible && object_visible(&object_node.object));
                }
            }
        }
    }

    None
}

fn group_property_lines(
    group: &RenderGroup,
    parent_path: &str,
    requested_path: &str,
) -> Option<Vec<String>> {
    let path = join_path(parent_path, &group.id);
    if path == requested_path {
        let mut lines = common_property_lines(
            &group.id,
            &group.name,
            &path,
            SceneObjectKind::Group,
            group.visible,
            group.transform,
        );
        lines.push(format!("editor composite: {}", group.editor_composite));
        lines.push(format!("internal children: {}", group.children.len()));
        append_behavior_lines(&mut lines, &group.behaviors);
        return Some(lines);
    }

    for node in &group.children {
        match node {
            RenderNode::Group(child_group) => {
                if let Some(lines) = group_property_lines(child_group, &path, requested_path) {
                    return Some(lines);
                }
            }
            RenderNode::Object(object_node) => {
                let object_path = join_path(&path, &object_node.id);
                if object_path == requested_path {
                    return Some(object_property_lines(object_node, &object_path));
                }
            }
        }
    }
    None
}

fn object_property_lines(object_node: &crate::render::RenderObjectNode, path: &str) -> Vec<String> {
    let visible = object_node.visible && object_visible(&object_node.object);
    let mut lines = common_property_lines(
        &object_node.id,
        &object_node.name,
        path,
        object_kind(&object_node.object),
        visible,
        object_node.transform,
    );

    match &object_node.object {
        RenderObject::Mesh(mesh) => {
            lines.push(format!("mesh asset: {}", mesh.mesh_asset));
            append_transform_lines(&mut lines, "mesh", mesh.transform);
        }
        RenderObject::QuadGroup(group) => {
            lines.push(format!("quad count: {}", group.quads.len()));
            append_transform_lines(&mut lines, "quads", group.transform);
        }
        RenderObject::GeoJsonMap(map) => {
            lines.push(format!("map asset: {}", map.asset));
            lines.push(format!("map visible: {}", map.visible));
            lines.push(format!("radius scale: {:.3}", map.radius_scale));
        }
        RenderObject::SphereGuide(guide) => {
            lines.push(format!("guide: {}", guide_kind_label(&guide.kind)));
            lines.push(format!("marker: {}", guide.marker));
            lines.push(format!("guide visible: {}", guide.visible));
            lines.push(format!("radius scale: {:.3}", guide.radius_scale));
        }
    }

    append_behavior_lines(&mut lines, &object_node.behaviors);
    lines
}

fn common_property_lines(
    id: &str,
    name: &str,
    path: &str,
    kind: SceneObjectKind,
    visible: bool,
    transform: RenderTransform,
) -> Vec<String> {
    let mut lines = vec![
        format!("name: {name}"),
        format!("id: {id}"),
        format!("path: {path}"),
        format!("type: {}", kind.label()),
        format!("visible: {visible}"),
    ];
    append_transform_lines(&mut lines, "node", transform);
    lines
}

fn append_transform_lines(lines: &mut Vec<String>, label: &str, transform: RenderTransform) {
    lines.push(format!(
        "{label} position: {:.2}, {:.2}, {:.2}",
        transform.position[0], transform.position[1], transform.position[2]
    ));
    lines.push(format!(
        "{label} rotation: {:.2}, {:.2}, {:.2}",
        transform.rotation_degrees[0], transform.rotation_degrees[1], transform.rotation_degrees[2]
    ));
    lines.push(format!(
        "{label} scale: {:.2}, {:.2}, {:.2}",
        transform.scale[0], transform.scale[1], transform.scale[2]
    ));
}

fn append_behavior_lines(lines: &mut Vec<String>, behaviors: &[RenderBehavior]) {
    if behaviors.is_empty() {
        lines.push("behaviors: none".to_string());
        return;
    }
    lines.push(format!("behaviors: {}", behaviors.len()));
    for behavior in behaviors {
        match behavior {
            RenderBehavior::Spin(spin) => lines.push(format!(
                "  spin axis={} speed={:.2} enabled={}",
                axis_label(spin.axis),
                spin.degrees_per_second,
                spin.enabled
            )),
        }
    }
}

fn axis_label(axis: RenderAxis) -> &'static str {
    match axis {
        RenderAxis::X => "x",
        RenderAxis::Y => "y",
        RenderAxis::Z => "z",
    }
}

fn guide_kind_label(kind: &RenderSphereGuideKind) -> String {
    match kind {
        RenderSphereGuideKind::GreatCircle(circle) => format!("{circle:?}"),
        RenderSphereGuideKind::Latitude(degrees) => format!("latitude {degrees:.1}"),
    }
}

fn join_path(parent: &str, id: &str) -> String {
    if parent.is_empty() {
        id.to_string()
    } else {
        format!("{parent}/{id}")
    }
}

fn object_kind(object: &RenderObject) -> SceneObjectKind {
    match object {
        RenderObject::Mesh(_) => SceneObjectKind::Mesh,
        RenderObject::QuadGroup(_) => SceneObjectKind::QuadGroup,
        RenderObject::GeoJsonMap(_) => SceneObjectKind::GeoJsonMap,
        RenderObject::SphereGuide(_) => SceneObjectKind::SphereGuide,
    }
}

fn object_visible(object: &RenderObject) -> bool {
    match object {
        RenderObject::GeoJsonMap(map) => map.visible,
        RenderObject::SphereGuide(guide) => guide.visible,
        RenderObject::Mesh(_) | RenderObject::QuadGroup(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{MeshPrepareOptions, RenderDisplay, RenderMeshObject, RenderObjectNode};

    fn test_scene() -> RenderScene {
        let mut scene = RenderScene::new("test", RenderDisplay { world_scale: 1.0 });
        let mut root = RenderGroup::new("root", "Root");
        let mut earth = RenderGroup::new("earth", "Earth");
        earth
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "mesh",
                "Mesh",
                RenderObject::Mesh(RenderMeshObject {
                    mesh_asset: "earth.obj".to_string(),
                    transform: RenderTransform::default(),
                    backface_cull: false,
                    prepare: MeshPrepareOptions::default(),
                }),
            )));
        root.children.push(RenderNode::Group(earth));
        scene.groups.push(root);
        scene
    }

    #[test]
    fn scene_objects_are_flattened_in_tree_order() {
        let entries = collect_scene_objects(&test_scene());
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "root");
        assert_eq!(entries[1].path, "root/earth");
        assert_eq!(entries[2].path, "root/earth/mesh");
        assert_eq!(entries[2].depth, 2);
        assert_eq!(entries[2].kind, SceneObjectKind::Mesh);
    }

    #[test]
    fn object_selection_opens_properties() {
        let entries = collect_scene_objects(&test_scene());
        let mut state = ViewerInspectorState::default();
        state.selected_object = 2;
        state.activate_selected(&entries);
        assert_eq!(state.active_object_path.as_deref(), Some("root/earth/mesh"));
        assert!(state.properties_open);
        assert!(!state.objects_open);
    }

    #[test]
    fn editor_composite_group_hides_internal_children() {
        let mut scene = RenderScene::new("test", RenderDisplay { world_scale: 1.0 });
        let mut root = RenderGroup::new("root", "Root");
        let mut composite = RenderGroup::new("graticule", "Graticule");
        composite.editor_composite = true;
        composite
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "guide",
                "Guide",
                RenderObject::SphereGuide(crate::render::RenderSphereGuide {
                    kind: RenderSphereGuideKind::Latitude(30.0),
                    marker: 'n',
                    visible: true,
                    radius_scale: 1.0,
                }),
            )));
        root.children.push(RenderNode::Group(composite));
        scene.groups.push(root);

        let entries = collect_scene_objects(&scene);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].path, "root/graticule");
        assert_eq!(entries[1].name, "Graticule");

        let lines = scene_object_property_lines(&scene, "root/graticule", None).unwrap();
        assert!(lines.iter().any(|line| line == "editor composite: true"));
        assert!(lines.iter().any(|line| line == "internal children: 1"));
    }

    #[test]
    fn toggling_group_visibility_updates_scene() {
        let mut scene = test_scene();

        assert_eq!(scene_object_visibility(&scene, "root/earth"), Some(true));
        assert_eq!(
            toggle_scene_object_visibility(&mut scene, "root/earth"),
            Some(false)
        );
        assert_eq!(scene_object_visibility(&scene, "root/earth"), Some(false));
    }

    #[test]
    fn toggling_object_visibility_updates_scene() {
        let mut scene = test_scene();

        assert_eq!(
            scene_object_visibility(&scene, "root/earth/mesh"),
            Some(true)
        );
        assert_eq!(
            toggle_scene_object_visibility(&mut scene, "root/earth/mesh"),
            Some(false)
        );
        assert_eq!(
            scene_object_visibility(&scene, "root/earth/mesh"),
            Some(false)
        );
    }

    #[test]
    fn mesh_property_lines_include_asset_and_transform() {
        let lines = scene_object_property_lines(&test_scene(), "root/earth/mesh", None).unwrap();
        assert!(lines.iter().any(|line| line == "mesh asset: earth.obj"));
        assert!(lines.iter().any(|line| line.starts_with("node position:")));
        assert!(lines.iter().any(|line| line == "visible: true"));
    }

    #[test]
    fn repeated_scale_up_stops_at_maximum() {
        let mut transform = RenderTransform::default();

        for _ in 0..1_000 {
            apply_transform_key(&mut transform, KeyCode::Char('+'));
        }

        assert_eq!(transform.scale, [MAX_OBJECT_SCALE; 3]);
    }

    #[test]
    fn repeated_scale_down_stops_at_minimum() {
        let mut transform = RenderTransform::default();

        for _ in 0..1_000 {
            apply_transform_key(&mut transform, KeyCode::Char('-'));
        }

        assert_eq!(transform.scale, [MIN_OBJECT_SCALE; 3]);
    }

    #[test]
    fn non_finite_scale_recovers_to_default() {
        let mut transform = RenderTransform {
            scale: [f32::INFINITY, f32::NAN, f32::NEG_INFINITY],
            ..RenderTransform::default()
        };

        apply_transform_key(&mut transform, KeyCode::Char('+'));

        assert_eq!(transform.scale, [1.0; 3]);
    }
}
