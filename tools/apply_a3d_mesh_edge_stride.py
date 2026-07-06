#!/usr/bin/env python3
from pathlib import Path
import json

APP = Path("src/app.rs")
TEAPOT_SCENE = Path("assets/a3d/external_teapot/scene.a3d")

def replace_once(text: str, old: str, new: str) -> str:
    if old not in text:
        raise SystemExit(f"Could not find expected text:\n{old}")
    return text.replace(old, new, 1)

def patch_app() -> None:
    text = APP.read_text()

    marker = "\nfn load_loaded_a3d_mesh(root: &Path, relative_path: &str) -> io::Result<Mesh> {\n"
    helper = '''
fn loaded_a3d_object_render_usize(
    root: &Path,
    object_id: &str,
    key: &str,
    default_value: usize,
) -> usize {
    let scene_path = root.join("scene.a3d");
    let Ok(source) = std::fs::read_to_string(&scene_path) else {
        return default_value;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&source) else {
        return default_value;
    };

    let Some(objects) = json.get("objects").and_then(serde_json::Value::as_array) else {
        return default_value;
    };

    let Some(value) = objects
        .iter()
        .find(|entry| {
            entry
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| id == object_id)
        })
        .and_then(|entry| entry.get("render"))
        .and_then(|render| render.get(key))
        .and_then(serde_json::Value::as_u64)
    else {
        return default_value;
    };

    usize::try_from(value).ok().filter(|value| *value > 0).unwrap_or(default_value)
}

fn loaded_a3d_object_edge_stride(root: &Path, object: &crate::a3d::SceneObject) -> usize {
    loaded_a3d_object_render_usize(root, &object.id, "edge_stride", 1)
}

'''
    if "fn loaded_a3d_object_edge_stride(" not in text:
        text = replace_once(text, marker, "\n" + helper + "fn load_loaded_a3d_mesh(root: &Path, relative_path: &str) -> io::Result<Mesh> {\n")

    # Patch world-space mesh loop.
    old_ws = '''    let mesh = load_loaded_a3d_mesh(root, path)?;
    let object_world = object.transform.matrix();

    for (from_index, to_index) in mesh.unique_edges() {
        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        canvas.draw_line(projector.project(from_world), projector.project(to_world), '#');
    }
'''
    new_ws = '''    let mesh = load_loaded_a3d_mesh(root, path)?;
    let object_world = object.transform.matrix();
    let edge_stride = loaded_a3d_object_edge_stride(root, object);

    for (edge_index, (from_index, to_index)) in mesh.unique_edges().into_iter().enumerate() {
        if edge_index % edge_stride != 0 {
            continue;
        }

        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        canvas.draw_line(projector.project(from_world), projector.project(to_world), '#');
    }
'''
    if old_ws in text:
        text = text.replace(old_ws, new_ws, 1)

    # Patch camera viewport mesh loop. This handles either the original line draw call or the dedup helper version.
    old_camera_plain = '''    let mesh = load_loaded_a3d_mesh(root, path)?;
    let object_world = object.transform.matrix();
    let character = object.render.stroke_character.unwrap_or('#');

    for (from_index, to_index) in mesh.unique_edges() {
        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        draw_camera_viewport_depth_line(
            canvas,
            depth_buffer,
            state,
            inner,
            from_world,
            to_world,
            character,
        );
    }
'''
    new_camera_plain = '''    let mesh = load_loaded_a3d_mesh(root, path)?;
    let object_world = object.transform.matrix();
    let character = object.render.stroke_character.unwrap_or('#');
    let edge_stride = loaded_a3d_object_edge_stride(root, object);

    for (edge_index, (from_index, to_index)) in mesh.unique_edges().into_iter().enumerate() {
        if edge_index % edge_stride != 0 {
            continue;
        }

        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        draw_camera_viewport_depth_line(
            canvas,
            depth_buffer,
            state,
            inner,
            from_world,
            to_world,
            character,
        );
    }
'''
    if old_camera_plain in text:
        text = text.replace(old_camera_plain, new_camera_plain, 1)
    elif "let edge_stride = loaded_a3d_object_edge_stride(root, object);" not in text:
        raise SystemExit("Could not patch camera mesh edge loop.")

    APP.write_text(text)

def patch_teapot_scene() -> None:
    if not TEAPOT_SCENE.exists():
        return

    data = json.loads(TEAPOT_SCENE.read_text())
    obj = data["objects"][0]
    render = obj.setdefault("render", {})
    render.setdefault("visible", True)
    render.setdefault("stroke_character", ".")
    render["edge_stride"] = 3

    TEAPOT_SCENE.write_text(json.dumps(data, indent=2) + "\n")

def main() -> None:
    patch_app()
    patch_teapot_scene()
    print("Added A3D mesh render.edge_stride support and set teapot edge_stride=3.")

if __name__ == "__main__":
    main()
