use super::{SceneWorkspace, WorkspaceEvent, WorkspaceResponse};
use ascii_3d::editor_core::{EditorCommand, EditorEntry, EditorSession, EditorTransformCommand};

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

pub type WorldEditorEntry = EditorEntry<WorldEditorTarget>;

fn camera_entry() -> WorldEditorEntry {
    EditorEntry::new(WorldEditorTarget::Camera, None)
}

fn scene_origin_entry() -> WorldEditorEntry {
    EditorEntry::new(WorldEditorTarget::SceneOrigin, None)
}

fn object_entry(id: impl Into<String>, visible: bool) -> WorldEditorEntry {
    EditorEntry::new(WorldEditorTarget::Object(id.into()), Some(visible))
}

/// Scene-local adapter for the LoadedA3d world-space editor.
///
/// Selection, activation, panel state, and per-target gizmo state live in the
/// renderer-independent `EditorSession`. This workspace keeps the existing
/// A3DWS API while translating UI actions into shared editor commands.
#[derive(Debug, Clone)]
pub struct LoadedA3dWorkspace {
    session: EditorSession<WorldEditorTarget>,
}

impl Default for LoadedA3dWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadedA3dWorkspace {
    pub fn new() -> Self {
        Self {
            session: EditorSession::new(
                vec![camera_entry(), scene_origin_entry()],
                WorldEditorTarget::Camera,
            ),
        }
    }

    pub fn entries(&self) -> &[WorldEditorEntry] {
        self.session.entries()
    }

    pub fn inspected_target(&self) -> Option<&WorldEditorTarget> {
        self.session.inspected_target()
    }

    pub fn active_xyz_target(&self) -> &WorldEditorTarget {
        self.session.active_target()
    }

    pub const fn selected_entry(&self) -> usize {
        self.session.selected_entry()
    }

    pub const fn objects_panel_open(&self) -> bool {
        self.session.objects_panel_open()
    }

    pub fn open_objects_panel(&mut self) {
        self.session.apply(EditorCommand::OpenObjectsPanel);
    }

    pub fn close_objects_panel(&mut self) {
        self.session.apply(EditorCommand::CloseObjectsPanel);
    }

    pub fn set_selected_entry(&mut self, index: usize) {
        self.session.apply(EditorCommand::SelectIndex(index));
    }

    pub fn move_selection_up(&mut self) {
        self.session.apply(EditorCommand::MoveSelectionUp);
    }

    pub fn move_selection_down(&mut self) {
        self.session.apply(EditorCommand::MoveSelectionDown);
    }

    pub fn inspect_selected(&mut self) -> Option<&WorldEditorTarget> {
        self.session.apply(EditorCommand::InspectSelected);
        self.session.inspected_target()
    }

    pub fn inspect_target(&mut self, target: WorldEditorTarget) -> bool {
        self.session.apply(EditorCommand::Inspect(target))
    }

    pub fn activate_target(&mut self, target: WorldEditorTarget) -> bool {
        self.session.apply(EditorCommand::Activate(target))
    }

    pub fn activate_inspected_xyz_target(&mut self) -> bool {
        self.session.apply(EditorCommand::ActivateInspected)
    }

    pub fn sync_objects<I, S>(&mut self, objects: I)
    where
        I: IntoIterator<Item = (S, bool)>,
        S: Into<String>,
    {
        let mut entries = vec![camera_entry(), scene_origin_entry()];
        entries.extend(
            objects
                .into_iter()
                .map(|(id, visible)| object_entry(id, visible)),
        );
        self.session
            .replace_entries(entries, WorldEditorTarget::Camera);
    }

    pub fn is_xyz_active(&self, target: &WorldEditorTarget) -> bool {
        self.session.is_active(target)
    }

    pub fn visibility(&self, target: &WorldEditorTarget) -> Option<bool> {
        self.session.visibility(target)
    }

    pub fn set_visibility(&mut self, target: &WorldEditorTarget, visible: bool) -> bool {
        self.session.apply(EditorCommand::SetVisibility {
            target: target.clone(),
            visible,
        })
    }

    pub fn toggle_visibility(&mut self, target: &WorldEditorTarget) -> Option<bool> {
        if !self
            .session
            .apply(EditorCommand::ToggleVisibility(target.clone()))
        {
            return None;
        }
        self.session.visibility(target)
    }

    pub fn gizmo_visible(&self, target: &WorldEditorTarget) -> bool {
        self.session.gizmo_visible(target)
    }

    pub fn toggle_gizmo(&mut self, target: &WorldEditorTarget) -> Option<bool> {
        if !self
            .session
            .apply(EditorCommand::ToggleGizmo(target.clone()))
        {
            return None;
        }
        Some(self.session.gizmo_visible(target))
    }

    pub fn request_transform(
        &self,
        command: EditorTransformCommand<WorldEditorTarget>,
    ) -> Option<EditorTransformCommand<WorldEditorTarget>> {
        self.session.request_transform(command)
    }

    pub fn request_active_translate(
        &self,
        delta: [f32; 3],
    ) -> Option<EditorTransformCommand<WorldEditorTarget>> {
        self.request_transform(EditorTransformCommand::Translate {
            target: self.active_xyz_target().clone(),
            delta,
        })
    }

    pub fn request_active_rotate(
        &self,
        delta_degrees: [f32; 3],
    ) -> Option<EditorTransformCommand<WorldEditorTarget>> {
        self.request_transform(EditorTransformCommand::Rotate {
            target: self.active_xyz_target().clone(),
            delta_degrees,
        })
    }

    pub fn request_active_scale(
        &self,
        factor: f32,
    ) -> Option<EditorTransformCommand<WorldEditorTarget>> {
        self.request_transform(EditorTransformCommand::ScaleUniform {
            target: self.active_xyz_target().clone(),
            factor,
        })
    }

    pub fn request_reset(
        &self,
        target: &WorldEditorTarget,
    ) -> Option<EditorTransformCommand<WorldEditorTarget>> {
        self.request_transform(EditorTransformCommand::Reset {
            target: target.clone(),
        })
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
    #[test]
    fn object_visibility_is_updated_through_editor_commands() {
        let mut workspace = LoadedA3dWorkspace::new();
        let earth = WorldEditorTarget::Object("earth".to_string());
        workspace.sync_objects([("earth", true)]);

        assert_eq!(workspace.toggle_visibility(&earth), Some(false));
        assert_eq!(workspace.visibility(&earth), Some(false));

        assert!(workspace.set_visibility(&earth, true));
        assert_eq!(workspace.visibility(&earth), Some(true));
        assert_eq!(
            workspace.toggle_visibility(&WorldEditorTarget::Camera),
            None
        );
    }

    #[test]
    fn transform_requests_are_shared_and_target_aware() {
        let mut workspace = LoadedA3dWorkspace::new();
        workspace.sync_objects([("earth", true)]);
        assert!(workspace.activate_target(WorldEditorTarget::Object("earth".to_string())));

        assert_eq!(
            workspace.request_active_translate([1.0, 0.0, 0.0]),
            Some(EditorTransformCommand::Translate {
                target: WorldEditorTarget::Object("earth".to_string()),
                delta: [1.0, 0.0, 0.0],
            })
        );
        assert!(workspace.request_active_scale(0.0).is_none());
    }
}
