use crate::a3d::{AssetRef, LoadedWorld};
use ascii_3d::editor_ui::{
    EditorAction, EditorCapabilities, EditorItem, EditorTarget, EditorTargetKind, PropertyRow,
};

use super::{CAMERA_TARGET_ID, SCENE_ORIGIN_TARGET_ID, WorldEditorEntry, WorldEditorTarget};

pub fn editor_items(entries: &[WorldEditorEntry], world: Option<&LoadedWorld>) -> Vec<EditorItem> {
    entries
        .iter()
        .map(|entry| editor_item(entry, world))
        .collect()
}

fn editor_item(entry: &WorldEditorEntry, world: Option<&LoadedWorld>) -> EditorItem {
    let (key, id, path, kind, capabilities) = match &entry.target {
        WorldEditorTarget::Camera => (
            CAMERA_TARGET_ID.to_string(),
            CAMERA_TARGET_ID.to_string(),
            CAMERA_TARGET_ID.to_string(),
            EditorTargetKind::Camera,
            EditorCapabilities::TRANSLATE
                .union(EditorCapabilities::ROTATE)
                .union(EditorCapabilities::DOLLY)
                .union(EditorCapabilities::RESET),
        ),
        WorldEditorTarget::SceneOrigin => (
            SCENE_ORIGIN_TARGET_ID.to_string(),
            SCENE_ORIGIN_TARGET_ID.to_string(),
            SCENE_ORIGIN_TARGET_ID.to_string(),
            EditorTargetKind::SceneOrigin,
            EditorCapabilities::TRANSLATE
                .union(EditorCapabilities::ROTATE)
                .union(EditorCapabilities::SCALE)
                .union(EditorCapabilities::RESET),
        ),
        WorldEditorTarget::Object(id) => {
            let kind = world
                .and_then(|world| world.object(id))
                .map(|object| match object.asset {
                    AssetRef::Group { .. } => EditorTargetKind::Group,
                    AssetRef::Mesh { .. } | AssetRef::Glyph { .. } | AssetRef::Word { .. } => {
                        EditorTargetKind::Mesh
                    }
                    AssetRef::GeoJsonMap { .. } => EditorTargetKind::GeoJsonMap,
                })
                .unwrap_or(EditorTargetKind::Other);
            (
                id.clone(),
                id.clone(),
                id.clone(),
                kind,
                EditorCapabilities::VISIBILITY
                    .union(EditorCapabilities::TRANSLATE)
                    .union(EditorCapabilities::ROTATE)
                    .union(EditorCapabilities::SCALE)
                    .union(EditorCapabilities::RESET),
            )
        }
    };

    EditorItem {
        target: EditorTarget::new(key, id, path, kind),
        label: entry.target.label().to_string(),
        depth: entry.target.id().matches('/').count(),
        visible: entry.visible,
        has_children: kind == EditorTargetKind::Group,
        capabilities,
    }
}

pub fn property_rows(
    target: &EditorTarget,
    world: Option<&LoadedWorld>,
    active_target: &WorldEditorTarget,
    gizmo_visible: bool,
) -> Vec<PropertyRow> {
    let active = target.path == active_target.id();
    let mut rows = vec![PropertyRow::Action {
        id: "activate-control-target".to_string(),
        label: if active {
            "XYZ control: active".to_string()
        } else {
            "Activate XYZ control".to_string()
        },
        hint: Some("Enter/Space".to_string()),
        enabled: true,
        action: EditorAction::ActivateControlTarget,
    }];

    if !target.path.starts_with('@') {
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

    match target.kind {
        EditorTargetKind::Camera => {
            rows.push(value("kind", "world-space camera"));
            rows.push(value("gizmo", "external editor view"));
            rows.push(value("saved", "runtime only"));
        }
        EditorTargetKind::SceneOrigin => {
            rows.push(value("kind", "scene origin"));
            rows.push(value("saved", "runtime only"));
        }
        _ => {
            if let Some(object) = world.and_then(|world| world.object(&target.path)) {
                rows.push(value("position", &format_vec3(object.transform.position)));
                rows.push(value(
                    "rotation",
                    &format_vec3(object.transform.rotation_degrees),
                ));
                rows.push(value("scale", &format_vec3(object.transform.scale)));
                rows.push(value("visible", object.render.visible.to_string()));
            } else {
                rows.push(value("status", "object not found"));
            }
        }
    }

    rows
}

pub fn world_target(target: &EditorTarget) -> WorldEditorTarget {
    match target.path.as_str() {
        CAMERA_TARGET_ID => WorldEditorTarget::Camera,
        SCENE_ORIGIN_TARGET_ID => WorldEditorTarget::SceneOrigin,
        path => WorldEditorTarget::Object(path.to_string()),
    }
}

fn value(label: &str, value: impl Into<String>) -> PropertyRow {
    PropertyRow::Value {
        label: label.to_string(),
        value: value.into(),
    }
}

fn format_vec3(value: [f32; 3]) -> String {
    format!("[{:.2}, {:.2}, {:.2}]", value[0], value[1], value[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_is_not_hideable_but_is_resettable() {
        let item = editor_item(&WorldEditorEntry::camera(), None);
        assert!(!item.capabilities.contains(EditorCapabilities::VISIBILITY));
        assert!(item.capabilities.contains(EditorCapabilities::RESET));
    }

    #[test]
    fn object_is_hideable_and_scalable() {
        let item = editor_item(&WorldEditorEntry::object("earth", true), None);
        assert!(item.capabilities.contains(EditorCapabilities::VISIBILITY));
        assert!(item.capabilities.contains(EditorCapabilities::SCALE));
    }
}
