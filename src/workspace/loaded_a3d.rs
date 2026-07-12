use super::{SceneWorkspace, WorkspaceEvent, WorkspaceResponse};

pub const CAMERA_TARGET_ID: &str = "@camera";
pub const SCENE_ORIGIN_TARGET_ID: &str = "@scene-origin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldEditorTarget {
    Camera,
    SceneOrigin,
    Object(String),
}

impl WorldEditorTarget {
    pub fn id(&self) -> &str {
        match self {
            Self::Camera => CAMERA_TARGET_ID,
            Self::SceneOrigin => SCENE_ORIGIN_TARGET_ID,
            Self::Object(id) => id,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Camera => "Camera",
            Self::SceneOrigin => "Scene Origin",
            Self::Object(id) => id,
        }
    }

    pub const fn runtime_only(&self) -> bool {
        matches!(self, Self::Camera | Self::SceneOrigin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEditorEntry {
    pub target: WorldEditorTarget,
    pub visible: Option<bool>,
}

impl WorldEditorEntry {
    pub fn camera() -> Self {
        Self {
            target: WorldEditorTarget::Camera,
            visible: None,
        }
    }

    pub fn scene_origin() -> Self {
        Self {
            target: WorldEditorTarget::SceneOrigin,
            visible: None,
        }
    }

    pub fn object(id: impl Into<String>, visible: bool) -> Self {
        Self {
            target: WorldEditorTarget::Object(id.into()),
            visible: Some(visible),
        }
    }
}

/// Scene-local state for the LoadedA3d world-space editor.
///
/// This model intentionally separates the object being inspected from the
/// target receiving XYZ input. Camera is the default XYZ target. No transform
/// or visibility mutation is performed here yet.
#[derive(Debug, Clone)]
pub struct LoadedA3dWorkspace {
    entries: Vec<WorldEditorEntry>,
    inspected_target: Option<WorldEditorTarget>,
    active_xyz_target: WorldEditorTarget,
    selected_entry: usize,
    objects_panel_open: bool,
}

impl Default for LoadedA3dWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadedA3dWorkspace {
    pub fn new() -> Self {
        Self {
            entries: vec![WorldEditorEntry::camera(), WorldEditorEntry::scene_origin()],
            inspected_target: None,
            active_xyz_target: WorldEditorTarget::Camera,
            selected_entry: 0,
            objects_panel_open: false,
        }
    }

    pub fn entries(&self) -> &[WorldEditorEntry] {
        &self.entries
    }

    pub fn inspected_target(&self) -> Option<&WorldEditorTarget> {
        self.inspected_target.as_ref()
    }

    pub fn active_xyz_target(&self) -> &WorldEditorTarget {
        &self.active_xyz_target
    }

    pub const fn selected_entry(&self) -> usize {
        self.selected_entry
    }

    pub const fn objects_panel_open(&self) -> bool {
        self.objects_panel_open
    }

    pub fn open_objects_panel(&mut self) {
        self.objects_panel_open = true;
    }

    pub fn close_objects_panel(&mut self) {
        self.objects_panel_open = false;
    }

    pub fn set_selected_entry(&mut self, index: usize) {
        self.selected_entry = index.min(self.entries.len().saturating_sub(1));
    }

    pub fn move_selection_up(&mut self) {
        if self.entries.is_empty() {
            self.selected_entry = 0;
        } else if self.selected_entry == 0 {
            self.selected_entry = self.entries.len() - 1;
        } else {
            self.selected_entry -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.entries.is_empty() {
            self.selected_entry = 0;
        } else {
            self.selected_entry = (self.selected_entry + 1) % self.entries.len();
        }
    }

    pub fn inspect_selected(&mut self) -> Option<&WorldEditorTarget> {
        self.inspected_target = self
            .entries
            .get(self.selected_entry)
            .map(|entry| entry.target.clone());
        self.inspected_target.as_ref()
    }

    pub fn activate_inspected_xyz_target(&mut self) -> bool {
        let Some(target) = self.inspected_target.clone() else {
            return false;
        };

        self.active_xyz_target = target;
        true
    }

    pub fn sync_objects<I, S>(&mut self, objects: I)
    where
        I: IntoIterator<Item = (S, bool)>,
        S: Into<String>,
    {
        let previous_inspected = self.inspected_target.clone();
        let previous_active = self.active_xyz_target.clone();

        self.entries.clear();
        self.entries.push(WorldEditorEntry::camera());
        self.entries.push(WorldEditorEntry::scene_origin());
        self.entries.extend(
            objects
                .into_iter()
                .map(|(id, visible)| WorldEditorEntry::object(id, visible)),
        );

        self.selected_entry = self
            .selected_entry
            .min(self.entries.len().saturating_sub(1));

        self.inspected_target = previous_inspected.filter(|target| self.contains_target(target));

        self.active_xyz_target = if self.contains_target(&previous_active) {
            previous_active
        } else {
            WorldEditorTarget::Camera
        };
    }

    pub fn is_xyz_active(&self, target: &WorldEditorTarget) -> bool {
        &self.active_xyz_target == target
    }

    fn contains_target(&self, target: &WorldEditorTarget) -> bool {
        self.entries.iter().any(|entry| &entry.target == target)
    }
}

impl SceneWorkspace for LoadedA3dWorkspace {
    fn handle_workspace_event(&mut self, _event: WorkspaceEvent) -> WorkspaceResponse {
        WorkspaceResponse::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::AppCommand;

    #[test]
    fn camera_is_default_xyz_target() {
        let workspace = LoadedA3dWorkspace::new();

        assert_eq!(workspace.active_xyz_target(), &WorldEditorTarget::Camera);
        assert_eq!(workspace.entries().len(), 2);
    }

    #[test]
    fn objects_panel_can_open_and_close() {
        let mut workspace = LoadedA3dWorkspace::new();

        assert!(!workspace.objects_panel_open());

        workspace.open_objects_panel();
        assert!(workspace.objects_panel_open());

        workspace.close_objects_panel();
        assert!(!workspace.objects_panel_open());
    }

    #[test]
    fn inspecting_does_not_activate_xyz_target() {
        let mut workspace = LoadedA3dWorkspace::new();
        workspace.sync_objects([("earth", true), ("km-logo", true)]);
        workspace.set_selected_entry(3);

        assert_eq!(
            workspace.inspect_selected(),
            Some(&WorldEditorTarget::Object("km-logo".to_string()))
        );
        assert_eq!(workspace.active_xyz_target(), &WorldEditorTarget::Camera);
    }

    #[test]
    fn inspected_target_can_be_explicitly_activated() {
        let mut workspace = LoadedA3dWorkspace::new();
        workspace.sync_objects([("earth", true)]);
        workspace.set_selected_entry(2);
        workspace.inspect_selected();

        assert!(workspace.activate_inspected_xyz_target());
        assert_eq!(
            workspace.active_xyz_target(),
            &WorldEditorTarget::Object("earth".to_string())
        );
    }

    #[test]
    fn object_refresh_preserves_valid_active_target() {
        let mut workspace = LoadedA3dWorkspace::new();
        workspace.sync_objects([("earth", true), ("km-logo", false)]);
        workspace.set_selected_entry(3);
        workspace.inspect_selected();
        workspace.activate_inspected_xyz_target();

        workspace.sync_objects([("earth", true), ("km-logo", true)]);

        assert_eq!(
            workspace.active_xyz_target(),
            &WorldEditorTarget::Object("km-logo".to_string())
        );
        assert_eq!(workspace.entries()[3].visible, Some(true));
    }

    #[test]
    fn removed_active_object_falls_back_to_camera() {
        let mut workspace = LoadedA3dWorkspace::new();
        workspace.sync_objects([("earth", true)]);
        workspace.set_selected_entry(2);
        workspace.inspect_selected();
        workspace.activate_inspected_xyz_target();

        workspace.sync_objects(std::iter::empty::<(&str, bool)>());

        assert_eq!(workspace.active_xyz_target(), &WorldEditorTarget::Camera);
        assert_eq!(workspace.inspected_target(), None);
    }

    #[test]
    fn loaded_a3d_workspace_still_ignores_parent_commands() {
        let mut workspace = LoadedA3dWorkspace::new();

        assert_eq!(
            workspace.handle_workspace_event(WorkspaceEvent::Command(AppCommand::ReloadA3d)),
            WorkspaceResponse::Ignored
        );
    }
}
