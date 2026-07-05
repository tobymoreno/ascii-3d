#!/usr/bin/env python3
from pathlib import Path
import shutil

CUBE_OBJ = '# ascii-3d standard OBJ cube\n#\n# OBJ face indices are 1-based.\n# This file stores standard geometry only.\n# ascii-3d metadata/style should live in a sidecar manifest later.\n\nv -1 -1 -1\nv  1 -1 -1\nv  1  1 -1\nv -1  1 -1\nv -1 -1  1\nv  1 -1  1\nv  1  1  1\nv -1  1  1\n\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n'
PYRAMID_OBJ = '# ascii-3d standard OBJ pyramid\n#\n# 5 vertices, 5 faces.\n# Square base plus four triangular sides.\n\nv -1 -1 -1\nv  1 -1 -1\nv  1 -1  1\nv -1 -1  1\nv  0  1  0\n\nf 1 4 3 2\nf 1 2 5\nf 2 3 5\nf 3 4 5\nf 4 1 5\n'
STANDARD_OBJ_PROBE = 'use std::{env, process};\n\n#[path = "../math.rs"]\nmod math;\n\n#[path = "../mesh.rs"]\nmod mesh;\n\n#[path = "../obj.rs"]\nmod obj;\n\nfn main() {\n    let path = env::args()\n        .nth(1)\n        .unwrap_or_else(|| "assets/models/cube.obj".to_string());\n\n    match obj::load_obj(&path) {\n        Ok(mesh) => {\n            println!("path={path}");\n            println!("vertices={}", mesh.vertices.len());\n            println!("faces={}", mesh.faces.len());\n            println!("unique_edges={}", mesh.unique_edges().len());\n\n            println!();\n            println!("first_vertices:");\n            for (index, vertex) in mesh.vertices.iter().take(5).enumerate() {\n                println!("{index}: {}, {}, {}", vertex.x, vertex.y, vertex.z);\n            }\n\n            println!();\n            println!("first_edges:");\n            for (a, b) in mesh.unique_edges().iter().take(12) {\n                println!("{a}-{b}");\n            }\n        }\n        Err(error) => {\n            eprintln!("standard_obj_probe error: {error}");\n            process::exit(1);\n        }\n    }\n}\n'
README = '# Standard OBJ models\n\nThis directory is for standard external geometry files.\n\nGeometry should be stored in normal `.obj` files:\n\n- `v x y z` vertex records\n- `f ...` face records\n- optional OBJ records may be ignored by ascii-3d until supported\n\nascii-3d-specific metadata should live outside the OBJ file later, as a sidecar\nmanifest. The OBJ file should stay portable and tool-friendly.\n'

def remove_duplicate_experiment() -> None:
    for path in [
        Path("src/obj_mesh.rs"),
        Path("src/bin/obj_mesh_probe.rs"),
    ]:
        if path.exists():
            path.unlink()

    generated_mesh_dir = Path("src/mesh")
    if generated_mesh_dir.exists() and generated_mesh_dir.is_dir():
        shutil.rmtree(generated_mesh_dir)

def ensure_standard_assets() -> None:
    assets = Path("assets/models")
    assets.mkdir(parents=True, exist_ok=True)

    Path("assets/models/cube.obj").write_text(CUBE_OBJ)
    Path("assets/models/pyramid.obj").write_text(PYRAMID_OBJ)
    Path("assets/models/README.md").write_text(README)

def ensure_standard_probe() -> None:
    probe = Path("src/bin/standard_obj_probe.rs")
    probe.parent.mkdir(parents=True, exist_ok=True)
    probe.write_text(STANDARD_OBJ_PROBE)

def ensure_main_uses_existing_mesh_module() -> None:
    main_rs = Path("src/main.rs")
    text = main_rs.read_text()

    # If a previous cleanup accidentally removed mod mesh, restore it.
    if "mod mesh;" not in text and Path("src/mesh.rs").exists():
        if "mod math;" in text:
            text = text.replace("mod math;", "mod math;\nmod mesh;", 1)
        elif "mod graphics;" in text:
            text = text.replace("mod graphics;", "mod graphics;\nmod mesh;", 1)
        else:
            text = "mod mesh;\n" + text

    main_rs.write_text(text)

def main() -> None:
    remove_duplicate_experiment()
    ensure_standard_assets()
    ensure_standard_probe()
    ensure_main_uses_existing_mesh_module()

    print("Cleaned duplicate OBJ experiment and added existing-loader standard OBJ probe.")

if __name__ == "__main__":
    main()
