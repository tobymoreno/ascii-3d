#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")

def replace_once(text: str, old: str, new: str) -> str:
    if old not in text:
        raise SystemExit(f"Could not find expected text:\n{old}")
    return text.replace(old, new, 1)

def main() -> None:
    text = APP.read_text()

    text = replace_once(
        text,
        '"Rendering Word assets from assets/a3d/p_depth_demo/scene.a3d"',
        '"Rendering Word and Mesh assets from the loaded .a3d scene"',
    )

    text = replace_once(
        text,
        "        for object in &world.objects {\n"
        "            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;\n"
        "        }\n",
        "        for object in &world.objects {\n"
        "            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;\n"
        "            draw_loaded_a3d_mesh_object(canvas, &mut depth_buffer, state, root, inner, object)?;\n"
        "        }\n",
    )

    text = replace_once(
        text,
        "    for object in &world.objects {\n"
        "        draw_loaded_a3d_word_object_in_ws(canvas, projector, state, root, object)?;\n"
        "    }\n",
        "    for object in &world.objects {\n"
        "        draw_loaded_a3d_word_object_in_ws(canvas, projector, state, root, object)?;\n"
        "        draw_loaded_a3d_mesh_object_in_ws(canvas, projector, root, object)?;\n"
        "    }\n",
    )

    text = replace_once(
        text,
        "        for object in &world.objects {\n"
        "            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;\n"
        "        }\n",
        "        for object in &world.objects {\n"
        "            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;\n"
        "            draw_loaded_a3d_mesh_object(canvas, &mut depth_buffer, state, root, inner, object)?;\n"
        "        }\n",
    )

    marker = "\nfn draw_loaded_a3d_word_object(\n"
    helpers = r"""
fn load_loaded_a3d_mesh(root: &Path, relative_path: &str) -> io::Result<Mesh> {
    let mesh_path = resolve_a3d_asset_path(root, relative_path)?;
    let mut mesh = load_obj(Path::new(&mesh_path)).map_err(|error| {
        io::Error::other(format!("failed to load A3D mesh {}: {}", mesh_path, error))
    })?;

    // External OBJ files often arrive in arbitrary units and offsets.
    // Normalize first, then let the .a3d object transform place/scale it.
    if !mesh.normalize_to_size(1.0) {
        return Err(io::Error::other(format!(
            "could not normalize A3D mesh {}",
            mesh_path
        )));
    }

    Ok(mesh)
}

fn draw_loaded_a3d_mesh_object_in_ws(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    root: &Path,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::Mesh { path } = &object.asset else {
        return Ok(());
    };

    let mesh = load_loaded_a3d_mesh(root, path)?;
    let object_world = object.transform.matrix();

    for (from_index, to_index) in mesh.unique_edges() {
        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        canvas.draw_line(projector.project(from_world), projector.project(to_world), '#');
    }

    Ok(())
}

fn draw_loaded_a3d_mesh_object(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    root: &Path,
    inner: ClipRect,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::Mesh { path } = &object.asset else {
        return Ok(());
    };

    let mesh = load_loaded_a3d_mesh(root, path)?;
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

    Ok(())
}

"""
    if "fn draw_loaded_a3d_mesh_object(" not in text:
        text = text.replace(marker, helpers + marker, 1)

    APP.write_text(text)
    print("Added LoadedA3d mesh asset rendering.")

if __name__ == "__main__":
    main()
