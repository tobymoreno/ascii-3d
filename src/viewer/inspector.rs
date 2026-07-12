use crate::render::{RenderGroup, RenderNode, RenderObject, RenderScene};

pub const VIEWER_MENU_TITLES: &[&str] = &["File", "Objects", "View", "Help"];
pub const OBJECTS_MENU_INDEX: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneObjectKind {
    Group,
    Mesh,
    QuadGroup,
    GeoJsonMap,
    SphereGuide,
}

impl SceneObjectKind {
    pub const fn label(self) -> &'static str {
        match self {
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
    pub objects_open: bool,
    pub selected_object: usize,
    pub active_object_path: Option<String>,
}

impl ViewerInspectorState {
    pub fn focus_menu(&mut self) {
        self.menu_focused = true;
    }

    pub fn close_popup(&mut self) {
        self.objects_open = false;
        self.menu_focused = false;
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
        if self.selected_menu != OBJECTS_MENU_INDEX {
            return;
        }
        self.objects_open = true;
        self.selected_object = self.selected_object.min(object_count.saturating_sub(1));
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
        self.close_popup();
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
    use crate::render::{RenderDisplay, RenderMeshObject, RenderObjectNode, RenderTransform};

    #[test]
    fn scene_objects_are_flattened_in_tree_order() {
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
                }),
            )));

        root.children.push(RenderNode::Group(earth));
        scene.groups.push(root);

        let entries = collect_scene_objects(&scene);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "root");
        assert_eq!(entries[1].path, "root/earth");
        assert_eq!(entries[2].path, "root/earth/mesh");
        assert_eq!(entries[2].depth, 2);
        assert_eq!(entries[2].kind, SceneObjectKind::Mesh);
    }

    #[test]
    fn object_selection_wraps_and_tracks_path() {
        let entries = vec![
            SceneObjectEntry {
                path: "root".to_string(),
                id: "root".to_string(),
                name: "Root".to_string(),
                depth: 0,
                kind: SceneObjectKind::Group,
                visible: true,
            },
            SceneObjectEntry {
                path: "root/mesh".to_string(),
                id: "mesh".to_string(),
                name: "Mesh".to_string(),
                depth: 1,
                kind: SceneObjectKind::Mesh,
                visible: true,
            },
        ];

        let mut state = ViewerInspectorState::default();
        state.move_object_up(entries.len());
        assert_eq!(state.selected_object, 1);

        state.activate_selected(&entries);
        assert_eq!(state.active_object_path.as_deref(), Some("root/mesh"));
        assert_eq!(state.active_label(&entries), Some("Mesh"));
    }
}
