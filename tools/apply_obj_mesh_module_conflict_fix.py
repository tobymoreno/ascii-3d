#!/usr/bin/env python3
from pathlib import Path
import shutil

PROBE = 'use std::{env, process};\n\n#[path = "../obj_mesh.rs"]\nmod obj_mesh;\n\nfn main() {\n    let path = env::args()\n        .nth(1)\n        .unwrap_or_else(|| "assets/models/cube.obj".to_string());\n\n    match obj_mesh::load_obj_mesh(&path) {\n        Ok(mesh) => {\n            println!("path={path}");\n            println!("vertices={}", mesh.vertices.len());\n            println!("faces={}", mesh.faces.len());\n            println!("edges={}", mesh.edges.len());\n\n            println!();\n            println!("first_vertices:");\n            for (index, vertex) in mesh.vertices.iter().take(5).enumerate() {\n                println!("{index}: {}, {}, {}", vertex.x, vertex.y, vertex.z);\n            }\n\n            println!();\n            println!("first_edges:");\n            for (a, b) in mesh.edges.iter().take(12) {\n                println!("{a}-{b}");\n            }\n        }\n        Err(error) => {\n            eprintln!("obj_mesh_probe error: {error}");\n            process::exit(1);\n        }\n    }\n}\n'

def main() -> None:
    old_obj = Path("src/mesh/obj.rs")
    new_obj = Path("src/obj_mesh.rs")

    if old_obj.exists():
        new_obj.write_text(old_obj.read_text())
    elif not new_obj.exists():
        raise SystemExit("Could not find src/mesh/obj.rs or src/obj_mesh.rs")

    mesh_dir = Path("src/mesh")
    if mesh_dir.exists() and mesh_dir.is_dir():
        shutil.rmtree(mesh_dir)

    main_rs = Path("src/main.rs")
    text = main_rs.read_text()
    text = text.replace("mod graphics;\nmod mesh;", "mod graphics;", 1)
    main_rs.write_text(text)

    probe = Path("src/bin/obj_mesh_probe.rs")
    probe.parent.mkdir(parents=True, exist_ok=True)
    probe.write_text(PROBE)

    print("Moved OBJ loader to src/obj_mesh.rs and removed src/mesh module conflict.")

if __name__ == "__main__":
    main()
