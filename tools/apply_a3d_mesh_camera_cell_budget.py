#!/usr/bin/env python3
from pathlib import Path
import json

APP = Path("src/app.rs")
TEAPOT_SCENE = Path("assets/a3d/external_teapot/scene.a3d")

def replace_once(text: str, old: str, new: str) -> str:
    if old not in text:
        raise SystemExit(f"Could not find expected text:\n{old}")
    return text.replace(old, new, 1)

def insert_after_function(text: str, name: str, addition: str) -> str:
    if addition.strip().splitlines()[0] in text:
        return text

    start_marker = f"fn {name}("
    start = text.find(start_marker)
    if start < 0:
        raise SystemExit(f"Could not find function {name}")

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit(f"Could not find opening brace for {name}")

    depth = 0
    end = None
    for index in range(brace, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                end = index + 1
                break

    if end is None:
        raise SystemExit(f"Could not find closing brace for {name}")

    return text[:end] + "\n\n" + addition.rstrip() + text[end:]

def patch_app() -> None:
    text = APP.read_text()

    # Add helper next to edge_stride helper.
    if "fn loaded_a3d_object_max_camera_cells(" not in text:
        text = replace_once(
            text,
            '''fn loaded_a3d_object_edge_stride(root: &Path, object: &crate::a3d::SceneObject) -> usize {
    loaded_a3d_object_render_usize(root, &object.id, "edge_stride", 1)
}
''',
            '''fn loaded_a3d_object_edge_stride(root: &Path, object: &crate::a3d::SceneObject) -> usize {
    loaded_a3d_object_render_usize(root, &object.id, "edge_stride", 1)
}

fn loaded_a3d_object_max_camera_cells(root: &Path, object: &crate::a3d::SceneObject) -> usize {
    loaded_a3d_object_render_usize(root, &object.id, "max_camera_cells", usize::MAX)
}
''',
        )

    counted_helper = '''fn draw_camera_viewport_depth_line_counted(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    inner: ClipRect,
    from_world: Vec3,
    to_world: Vec3,
    character: char,
) -> usize {
    let Some(from_camera) = world_to_camera_space(state, from_world) else {
        return 0;
    };
    let Some(to_camera) = world_to_camera_space(state, to_world) else {
        return 0;
    };

    let cell_aspect_ratio = camera_viewport_cell_aspect_ratio(state);
    let perspective_scale = camera_viewport_perspective_scale(state);

    let Some((from_screen, from_depth)) = project_camera_space_to_viewport_with_depth(
        from_camera,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    ) else {
        return 0;
    };
    let Some((to_screen, to_depth)) = project_camera_space_to_viewport_with_depth(
        to_camera,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    ) else {
        return 0;
    };

    let mut x0 = from_screen.x;
    let mut y0 = from_screen.y;
    let x1 = to_screen.x;
    let y1 = to_screen.y;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };

    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut error = dx + dy;
    let steps = dx.max(-dy).max(1);
    let mut step_index = 0;
    let mut written_cells = 0;

    loop {
        let t = step_index as f32 / steps as f32;
        let depth = from_depth + (to_depth - from_depth) * t;
        let point = Point2::new(x0, y0);

        if depth_buffer.try_update(point, depth) {
            canvas.set(point, character);
            written_cells += 1;
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let doubled_error = 2 * error;

        if doubled_error >= dy {
            error += dy;
            x0 += sx;
        }

        if doubled_error <= dx {
            error += dx;
            y0 += sy;
        }

        step_index += 1;
    }

    written_cells
}
'''
    text = insert_after_function(text, "draw_camera_viewport_depth_line", counted_helper)

    old = '''    let mesh = load_loaded_a3d_mesh(root, path)?;
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

    new = '''    let mesh = load_loaded_a3d_mesh(root, path)?;
    let object_world = object.transform.matrix();
    let character = object.render.stroke_character.unwrap_or('#');
    let edge_stride = loaded_a3d_object_edge_stride(root, object);
    let max_camera_cells = loaded_a3d_object_max_camera_cells(root, object);
    let mut camera_cells_written = 0;

    for (edge_index, (from_index, to_index)) in mesh.unique_edges().into_iter().enumerate() {
        if edge_index % edge_stride != 0 {
            continue;
        }

        if camera_cells_written >= max_camera_cells {
            break;
        }

        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        camera_cells_written += draw_camera_viewport_depth_line_counted(
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
    if old in text:
        text = text.replace(old, new, 1)
    elif "let max_camera_cells = loaded_a3d_object_max_camera_cells(root, object);" not in text:
        raise SystemExit("Could not patch mesh camera viewport loop with max_camera_cells.")

    APP.write_text(text)

def patch_teapot_scene() -> None:
    if not TEAPOT_SCENE.exists():
        return

    data = json.loads(TEAPOT_SCENE.read_text())
    render = data["objects"][0].setdefault("render", {})
    render.setdefault("visible", True)
    render.setdefault("stroke_character", ".")
    render.setdefault("edge_stride", 3)
    render["max_camera_cells"] = 900

    TEAPOT_SCENE.write_text(json.dumps(data, indent=2) + "\n")

def main() -> None:
    patch_app()
    patch_teapot_scene()
    print("Added A3D mesh render.max_camera_cells support and set teapot max_camera_cells=900.")

if __name__ == "__main__":
    main()
