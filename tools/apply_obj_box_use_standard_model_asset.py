#!/usr/bin/env python3
from pathlib import Path

CUBE_OBJ = '# ascii-3d standard OBJ cube\n#\n# OBJ face indices are 1-based.\n# This file stores standard geometry only.\n# ascii-3d metadata/style should live in a sidecar manifest later.\n\nv -1 -1 -1\nv  1 -1 -1\nv  1  1 -1\nv -1  1 -1\nv -1 -1  1\nv  1 -1  1\nv  1  1  1\nv -1  1  1\n\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n'
TEST_TO_ADD = '\n    #[test]\n    fn standard_cube_obj_asset_exists() {\n        assert!(asset_path(STANDARD_BOX_ASSET).is_file());\n    }\n\n    #[test]\n    fn standard_cube_obj_asset_loads_as_wireframe_cube() {\n        let mesh = load_mesh_asset(STANDARD_BOX_ASSET).expect("models/cube.obj should load");\n\n        assert_eq!(mesh.vertices.len(), 8);\n        assert_eq!(mesh.faces.len(), 6);\n        assert_eq!(mesh.unique_edges().len(), 12);\n    }\n'

def ensure_standard_cube_asset() -> None:
    path = Path("assets/models/cube.obj")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(CUBE_OBJ)

def update_app_asset_loading() -> None:
    path = Path("src/app.rs")
    text = path.read_text()

    text = text.replace(
        'let mut box_mesh = load_mesh_asset("box.obj")?;',
        'let mut box_mesh = load_mesh_asset(STANDARD_BOX_ASSET)?;',
        1,
    )

    text = text.replace(
        'io::Error::other("could not normalize assets/box.obj")',
        'io::Error::other(format!("could not normalize assets/{STANDARD_BOX_ASSET}"))',
        1,
    )

    if 'const STANDARD_BOX_ASSET: &str = "models/cube.obj";' not in text:
        marker = 'const DEFAULT_GLYPH_STROKE_INDEX: usize = 0;'
        replacement = marker + '\n\nconst STANDARD_BOX_ASSET: &str = "models/cube.obj";'
        if marker in text:
            text = text.replace(marker, replacement, 1)
        else:
            text = 'const STANDARD_BOX_ASSET: &str = "models/cube.obj";\n' + text

    if 'fn standard_cube_obj_asset_exists()' not in text and '#[cfg(test)]' in text:
        last = text.rfind('\n}\n')
        if last != -1:
            text = text[:last] + TEST_TO_ADD + text[last:]

    path.write_text(text)

def update_obj_box_scene_label() -> None:
    path = Path("src/scenes/obj_box.rs")
    text = path.read_text()

    text = text.replace(
        '"Scene: rotating OBJ wireframe box  angle={:06.1}"',
        '"Scene: rotating standard OBJ cube  angle={:06.1}"',
        1,
    )

    text = text.replace(
        '"Source: assets/box.obj"',
        '"Source: assets/models/cube.obj"',
        1,
    )

    path.write_text(text)

def main() -> None:
    ensure_standard_cube_asset()
    update_app_asset_loading()
    update_obj_box_scene_label()

    print("Updated ObjBox scene to load standard OBJ model assets/models/cube.obj.")

if __name__ == "__main__":
    main()
