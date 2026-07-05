#!/usr/bin/env python3
from pathlib import Path

PROBE = 'use std::{env, process};\n\n#[path = "../math/mod.rs"]\nmod math;\n\n#[path = "../mesh.rs"]\nmod mesh;\n\n#[path = "../obj.rs"]\nmod obj;\n\nfn main() {\n    let path = env::args()\n        .nth(1)\n        .unwrap_or_else(|| "assets/models/cube.obj".to_string());\n\n    match obj::load_obj(&path) {\n        Ok(mesh) => {\n            println!("path={path}");\n            println!("vertices={}", mesh.vertices.len());\n            println!("faces={}", mesh.faces.len());\n            println!("unique_edges={}", mesh.unique_edges().len());\n\n            println!();\n            println!("first_vertices:");\n            for (index, vertex) in mesh.vertices.iter().take(5).enumerate() {\n                println!("{index}: {}, {}, {}", vertex.x, vertex.y, vertex.z);\n            }\n\n            println!();\n            println!("first_edges:");\n            for (a, b) in mesh.unique_edges().iter().take(12) {\n                println!("{a}-{b}");\n            }\n        }\n        Err(error) => {\n            eprintln!("standard_obj_probe error: {error}");\n            process::exit(1);\n        }\n    }\n}\n'

def main() -> None:
    path = Path("src/bin/standard_obj_probe.rs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(PROBE)

    print("Fixed standard_obj_probe math module path.")

if __name__ == "__main__":
    main()
