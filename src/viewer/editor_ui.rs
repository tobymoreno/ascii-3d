use crate::{
    editor_ui::{
        EditorAction, EditorCapabilities, EditorItem, EditorTarget, EditorTargetKind,
        MenuCapabilities, MenuDefinition, PropertyRow, shared_menu_definitions,
    },
    render::RenderScene,
};

use super::{
    SceneObjectEntry, SceneObjectKind, scene_helper_property_lines, scene_object_property_lines,
};

pub use crate::editor_ui::{DEBUG_MENU_ID, FILE_MENU_ID, OBJECTS_MENU_ID};

pub fn viewer_menu_definitions(_save_enabled: bool) -> Vec<MenuDefinition> {
    let mut definitions = shared_menu_definitions(MenuCapabilities {
        can_open: true,
        can_reload: true,
        can_save: false,
        can_save_as: false,
        can_browse_scenes: true,
        can_exit: true,
        can_toggle_log: false,
        can_open_raylib_gui: false,
    });

    if let Some(file) = definitions
        .iter_mut()
        .find(|definition| definition.id.0 == crate::editor_ui::FILE_MENU_ID)
    {
        file.entries.retain(|entry| {
            !matches!(
                entry,
                crate::editor_ui::MenuEntry::Action { id, .. }
                    if id == crate::editor_ui::FILE_SAVE_ID
                        || id == crate::editor_ui::FILE_SAVE_AS_ID
            )
        });

        let mut compact = Vec::with_capacity(file.entries.len());
        for entry in file.entries.drain(..) {
            if matches!(entry, crate::editor_ui::MenuEntry::Separator)
                && (compact.is_empty()
                    || matches!(compact.last(), Some(crate::editor_ui::MenuEntry::Separator)))
            {
                continue;
            }
            compact.push(entry);
        }
        while matches!(compact.last(), Some(crate::editor_ui::MenuEntry::Separator)) {
            compact.pop();
        }
        file.entries = compact;
    }

    definitions
}

pub fn editor_items(entries: &[SceneObjectEntry]) -> Vec<EditorItem> {
    entries.iter().map(editor_item).collect()
}

fn editor_item(entry: &SceneObjectEntry) -> EditorItem {
    let kind = editor_target_kind(entry.kind);
    let capabilities = match kind {
        EditorTargetKind::Camera => EditorCapabilities::TRANSLATE
            .union(EditorCapabilities::ROTATE)
            .union(EditorCapabilities::DOLLY)
            .union(EditorCapabilities::RESET),
        EditorTargetKind::SceneOrigin => EditorCapabilities::TRANSLATE
            .union(EditorCapabilities::ROTATE)
            .union(EditorCapabilities::SCALE)
            .union(EditorCapabilities::RESET),
        EditorTargetKind::WorldAxes => EditorCapabilities::VISIBILITY,
        _ => EditorCapabilities::VISIBILITY
            .union(EditorCapabilities::TRANSLATE)
            .union(EditorCapabilities::ROTATE)
            .union(EditorCapabilities::SCALE)
            .union(EditorCapabilities::RESET),
    };

    EditorItem {
        target: EditorTarget::new(
            entry.path.clone(),
            entry.id.clone(),
            entry.path.clone(),
            kind,
        ),
        label: entry.name.clone(),
        depth: entry.depth,
        visible: (!matches!(
            kind,
            EditorTargetKind::Camera | EditorTargetKind::SceneOrigin
        ))
        .then_some(entry.visible),
        has_children: matches!(kind, EditorTargetKind::Group),
        capabilities,
    }
}

pub fn property_rows(
    scene: &RenderScene,
    target: &EditorTarget,
    world_axes_visible: bool,
    active_path: Option<&str>,
    gizmo_visible: bool,
) -> Vec<PropertyRow> {
    let mut rows = vec![PropertyRow::Action {
        id: "activate-control-target".to_string(),
        label: if active_path == Some(target.path.as_str()) {
            "XYZ control: active".to_string()
        } else {
            "Activate XYZ control".to_string()
        },
        hint: Some("Enter/Space".to_string()),
        enabled: true,
        action: EditorAction::ActivateControlTarget,
    }];

    if !target.path.starts_with("@scene/") {
        rows.push(PropertyRow::Action {
            id: "toggle-visibility".to_string(),
            label: "Toggle visibility".to_string(),
            hint: Some("Enter/Space".to_string()),
            enabled: true,
            action: EditorAction::ToggleVisibility,
        });
    }

    rows.push(PropertyRow::Action {
        id: "toggle-transform-gizmo".to_string(),
        label: format!(
            "Transform gizmo: {}",
            if gizmo_visible { "On" } else { "Off" }
        ),
        hint: Some("Enter/Space".to_string()),
        enabled: true,
        action: EditorAction::ToggleTransformGizmo,
    });

    rows.push(PropertyRow::Action {
        id: "reset-transform".to_string(),
        label: "Reset transform".to_string(),
        hint: Some("Enter/Space".to_string()),
        enabled: true,
        action: EditorAction::ResetTransform,
    });

    rows.push(PropertyRow::Separator);

    let lines = scene_helper_property_lines(&target.path, world_axes_visible, active_path)
        .or_else(|| scene_object_property_lines(scene, &target.path, active_path))
        .unwrap_or_else(|| vec!["status: object not found".to_string()]);

    rows.extend(lines.into_iter().filter_map(|line| {
        if line.starts_with("xyz control:") || line.starts_with("visible:") {
            return None;
        }
        let (label, value) = line
            .split_once(':')
            .map(|(label, value)| (label.trim().to_string(), value.trim().to_string()))
            .unwrap_or_else(|| ("info".to_string(), line));
        Some(PropertyRow::Value { label, value })
    }));

    rows
}

fn editor_target_kind(kind: SceneObjectKind) -> EditorTargetKind {
    match kind {
        SceneObjectKind::SceneHelpers => EditorTargetKind::Other,
        SceneObjectKind::Camera => EditorTargetKind::Camera,
        SceneObjectKind::Light => EditorTargetKind::Light,
        SceneObjectKind::WorldAxes => EditorTargetKind::WorldAxes,
        SceneObjectKind::SceneOrigin => EditorTargetKind::SceneOrigin,
        SceneObjectKind::Group => EditorTargetKind::Group,
        SceneObjectKind::Mesh => EditorTargetKind::Mesh,
        SceneObjectKind::QuadGroup => EditorTargetKind::QuadGroup,
        SceneObjectKind::GeoJsonMap => EditorTargetKind::GeoJsonMap,
        SceneObjectKind::SphereGuide => EditorTargetKind::SphereGuide,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::{CAMERA_HELPER_PATH, SCENE_ORIGIN_HELPER_PATH};

    #[test]
    fn camera_item_exposes_dolly_but_not_scale() {
        let item = editor_item(&SceneObjectEntry {
            path: CAMERA_HELPER_PATH.to_string(),
            id: "camera".to_string(),
            name: "Camera".to_string(),
            depth: 0,
            kind: SceneObjectKind::Camera,
            visible: true,
        });
        assert!(item.capabilities.contains(EditorCapabilities::DOLLY));
        assert!(!item.capabilities.contains(EditorCapabilities::SCALE));
    }

    #[test]
    fn scene_origin_item_exposes_scale_but_not_dolly() {
        let item = editor_item(&SceneObjectEntry {
            path: SCENE_ORIGIN_HELPER_PATH.to_string(),
            id: "scene-origin".to_string(),
            name: "Scene Origin".to_string(),
            depth: 0,
            kind: SceneObjectKind::SceneOrigin,
            visible: true,
        });
        assert!(item.capabilities.contains(EditorCapabilities::SCALE));
        assert!(!item.capabilities.contains(EditorCapabilities::DOLLY));
    }
}
