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

    constants_marker = 'const P_WORD_WORLD_SCALE: f32 = 1.35;\n'
    constants_add = '''const DEFAULT_CAMERA_VIEWPORT_CELL_ASPECT_RATIO: f32 = 0.5;
const DEFAULT_CAMERA_VIEWPORT_PERSPECTIVE_SCALE: f32 = 22.0;

'''
    if "DEFAULT_CAMERA_VIEWPORT_CELL_ASPECT_RATIO" not in text:
        text = replace_once(text, constants_marker, constants_marker + "\n" + constants_add)

    old_projection = '''fn project_camera_space_to_viewport(camera_space: Vec3, inner: ClipRect) -> Option<Point2> {
    // Mat4::look_at uses the conventional right-handed camera space:
    // +X = camera right, +Y = camera up, and camera forward points along -Z.
    if camera_space.z >= -0.01 {
        return None;
    }

    let center_x = inner.x + inner.width as i32 / 2;
    let center_y = inner.y + inner.height as i32 / 2;
    let depth = -camera_space.z;

    let perspective = 22.0 / depth;
    let screen_x = center_x + (camera_space.x * perspective).round() as i32;
    let screen_y = center_y - (camera_space.y * perspective).round() as i32;

    Some(Point2::new(screen_x, screen_y))
}

fn project_camera_space_to_viewport_with_depth(
    camera_space: Vec3,
    inner: ClipRect,
) -> Option<(Point2, f32)> {
    let point = project_camera_space_to_viewport(camera_space, inner)?;
    let depth = -camera_space.z;

    Some((point, depth))
}
'''

    new_projection = '''fn project_camera_space_to_viewport(
    camera_space: Vec3,
    inner: ClipRect,
    cell_aspect_ratio: f32,
    perspective_scale: f32,
) -> Option<Point2> {
    // Mat4::look_at uses the conventional right-handed camera space:
    // +X = camera right, +Y = camera up, and camera forward points along -Z.
    if camera_space.z >= -0.01 {
        return None;
    }

    let center_x = inner.x + inner.width as i32 / 2;
    let center_y = inner.y + inner.height as i32 / 2;
    let depth = -camera_space.z;

    let perspective = perspective_scale / depth;
    let screen_x = center_x + (camera_space.x * perspective).round() as i32;
    let screen_y =
        center_y - (camera_space.y * perspective * cell_aspect_ratio).round() as i32;

    Some(Point2::new(screen_x, screen_y))
}

fn project_camera_space_to_viewport_with_depth(
    camera_space: Vec3,
    inner: ClipRect,
    cell_aspect_ratio: f32,
    perspective_scale: f32,
) -> Option<(Point2, f32)> {
    let point = project_camera_space_to_viewport(
        camera_space,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    )?;
    let depth = -camera_space.z;

    Some((point, depth))
}
'''

    if old_projection in text:
        text = text.replace(old_projection, new_projection, 1)
    elif "fn project_camera_space_to_viewport(\n    camera_space: Vec3,\n    inner: ClipRect,\n    cell_aspect_ratio: f32," not in text:
        raise SystemExit("Projection function did not match expected text and was not already patched.")

    helpers_marker = '\nstruct CameraViewportDepthBuffer {\n'
    helpers = '''
fn loaded_a3d_camera_view_number(state: &AppState, key: &str, default_value: f32) -> f32 {
    let Some(root) = state.loaded_a3d_root.as_deref() else {
        return default_value;
    };

    let scene_path = root.join("scene.a3d");
    let Ok(source) = std::fs::read_to_string(&scene_path) else {
        return default_value;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&source) else {
        return default_value;
    };

    let Some(value) = json
        .get("viewport")
        .and_then(|viewport| viewport.get("camera_view"))
        .and_then(|camera_view| camera_view.get(key))
        .and_then(serde_json::Value::as_f64)
    else {
        return default_value;
    };

    if value.is_finite() && value > 0.0 {
        value as f32
    } else {
        default_value
    }
}

fn camera_viewport_cell_aspect_ratio(state: &AppState) -> f32 {
    loaded_a3d_camera_view_number(
        state,
        "cell_aspect_ratio",
        DEFAULT_CAMERA_VIEWPORT_CELL_ASPECT_RATIO,
    )
}

fn camera_viewport_perspective_scale(state: &AppState) -> f32 {
    loaded_a3d_camera_view_number(
        state,
        "perspective_scale",
        DEFAULT_CAMERA_VIEWPORT_PERSPECTIVE_SCALE,
    )
}

'''
    if "fn camera_viewport_cell_aspect_ratio(" not in text:
        text = replace_once(text, helpers_marker, helpers + helpers_marker)

    old_calls = '''    let Some((from_screen, from_depth)) =
        project_camera_space_to_viewport_with_depth(from_camera, inner)
    else {
        return;
    };
    let Some((to_screen, to_depth)) = project_camera_space_to_viewport_with_depth(to_camera, inner)
    else {
        return;
    };
'''

    new_calls = '''    let cell_aspect_ratio = camera_viewport_cell_aspect_ratio(state);
    let perspective_scale = camera_viewport_perspective_scale(state);

    let Some((from_screen, from_depth)) = project_camera_space_to_viewport_with_depth(
        from_camera,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    ) else {
        return;
    };
    let Some((to_screen, to_depth)) = project_camera_space_to_viewport_with_depth(
        to_camera,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    ) else {
        return;
    };
'''
    if old_calls in text:
        text = text.replace(old_calls, new_calls, 1)
    elif "let cell_aspect_ratio = camera_viewport_cell_aspect_ratio(state);" not in text:
        raise SystemExit("Depth projection call site did not match expected text and was not already patched.")

    old_status = '''            "{} | pos [{:.2},{:.2},{:.2}] yaw {:.1} pitch {:.1}",
            world.title,
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
            state.world_camera_yaw_degrees,
            state.world_camera_pitch_degrees,
'''
    new_status = '''            "{} | pos [{:.2},{:.2},{:.2}] yaw {:.1} pitch {:.1} | cell {:.2} persp {:.1}",
            world.title,
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
            state.world_camera_yaw_degrees,
            state.world_camera_pitch_degrees,
            camera_viewport_cell_aspect_ratio(state),
            camera_viewport_perspective_scale(state),
'''
    if old_status in text and "cell {:.2} persp {:.1}" not in text:
        text = text.replace(old_status, new_status, 1)

    APP.write_text(text)

def patch_teapot_scene() -> None:
    if not TEAPOT_SCENE.exists():
        return

    data = json.loads(TEAPOT_SCENE.read_text())
    viewport = data.setdefault("viewport", {})
    viewport.setdefault("show_world_debug", True)
    viewport.setdefault("show_camera3d", True)

    camera_view = viewport.setdefault("camera_view", {})
    camera_view.setdefault("cell_aspect_ratio", 0.5)
    camera_view.setdefault("perspective_scale", 22.0)

    TEAPOT_SCENE.write_text(json.dumps(data, indent=2) + "\n")

def main() -> None:
    patch_app()
    patch_teapot_scene()
    print("Added .a3d camera_view projection config.")

if __name__ == "__main__":
    main()
