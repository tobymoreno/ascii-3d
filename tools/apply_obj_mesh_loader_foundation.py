#!/usr/bin/env python3
from pathlib import Path

OBJ_MODULE = 'use std::{\n    collections::BTreeSet,\n    error::Error,\n    fs,\n    path::Path,\n};\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct ObjVertex {\n    pub x: f32,\n    pub y: f32,\n    pub z: f32,\n}\n\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ObjFace {\n    pub indices: Vec<usize>,\n}\n\n#[derive(Debug, Clone, PartialEq)]\npub struct ObjMesh {\n    pub vertices: Vec<ObjVertex>,\n    pub faces: Vec<ObjFace>,\n    pub edges: Vec<(usize, usize)>,\n}\n\npub fn load_obj_mesh(path: impl AsRef<Path>) -> Result<ObjMesh, Box<dyn Error>> {\n    let text = fs::read_to_string(path)?;\n    parse_obj_mesh(&text)\n}\n\npub fn parse_obj_mesh(text: &str) -> Result<ObjMesh, Box<dyn Error>> {\n    let mut vertices = Vec::new();\n    let mut faces = Vec::new();\n\n    for (line_number, raw_line) in text.lines().enumerate() {\n        let line = strip_comment(raw_line).trim();\n\n        if line.is_empty() {\n            continue;\n        }\n\n        let mut parts = line.split_whitespace();\n        let Some(kind) = parts.next() else {\n            continue;\n        };\n\n        match kind {\n            "v" => {\n                let x = parse_f32(parts.next(), line_number, "x")?;\n                let y = parse_f32(parts.next(), line_number, "y")?;\n                let z = parse_f32(parts.next(), line_number, "z")?;\n\n                vertices.push(ObjVertex { x, y, z });\n            }\n            "f" => {\n                let indices = parts\n                    .map(|token| parse_face_index(token, vertices.len(), line_number))\n                    .collect::<Result<Vec<_>, _>>()?;\n\n                if indices.len() < 2 {\n                    return Err(format!(\n                        "line {}: face must contain at least 2 vertices",\n                        line_number + 1\n                    )\n                    .into());\n                }\n\n                faces.push(ObjFace { indices });\n            }\n            _ => {\n                // Ignore standard OBJ records we do not need yet:\n                // o, g, s, vt, vn, mtllib, usemtl, l, etc.\n            }\n        }\n    }\n\n    let edges = derive_edges(&faces);\n\n    Ok(ObjMesh {\n        vertices,\n        faces,\n        edges,\n    })\n}\n\nfn strip_comment(line: &str) -> &str {\n    line.split_once(\'#\')\n        .map(|(before_comment, _)| before_comment)\n        .unwrap_or(line)\n}\n\nfn parse_f32(\n    value: Option<&str>,\n    line_number: usize,\n    axis_name: &str,\n) -> Result<f32, Box<dyn Error>> {\n    let value =\n        value.ok_or_else(|| format!("line {}: missing vertex {axis_name}", line_number + 1))?;\n\n    Ok(value.parse()?)\n}\n\nfn parse_face_index(\n    token: &str,\n    vertex_count: usize,\n    line_number: usize,\n) -> Result<usize, Box<dyn Error>> {\n    let raw_index = token\n        .split(\'/\')\n        .next()\n        .ok_or_else(|| format!("line {}: empty face token", line_number + 1))?;\n\n    let obj_index: isize = raw_index.parse()?;\n\n    let zero_based = if obj_index > 0 {\n        obj_index - 1\n    } else if obj_index < 0 {\n        vertex_count as isize + obj_index\n    } else {\n        return Err(format!("line {}: OBJ indices are 1-based; got 0", line_number + 1).into());\n    };\n\n    if zero_based < 0 || zero_based as usize >= vertex_count {\n        return Err(format!(\n            "line {}: face index {} is out of range for {} vertices",\n            line_number + 1,\n            obj_index,\n            vertex_count\n        )\n        .into());\n    }\n\n    Ok(zero_based as usize)\n}\n\nfn derive_edges(faces: &[ObjFace]) -> Vec<(usize, usize)> {\n    let mut edges = BTreeSet::new();\n\n    for face in faces {\n        if face.indices.len() < 2 {\n            continue;\n        }\n\n        for pair in face.indices.windows(2) {\n            insert_edge(&mut edges, pair[0], pair[1]);\n        }\n\n        let first = face.indices[0];\n        let last = face.indices[face.indices.len() - 1];\n\n        insert_edge(&mut edges, last, first);\n    }\n\n    edges.into_iter().collect()\n}\n\nfn insert_edge(edges: &mut BTreeSet<(usize, usize)>, a: usize, b: usize) {\n    if a == b {\n        return;\n    }\n\n    if a < b {\n        edges.insert((a, b));\n    } else {\n        edges.insert((b, a));\n    }\n}\n\n#[cfg(test)]\nmod tests {\n    use super::{ObjFace, ObjVertex, parse_obj_mesh};\n\n    const CUBE_OBJ: &str = r#"\n# cube\nv -1 -1 -1\nv  1 -1 -1\nv  1  1 -1\nv -1  1 -1\nv -1 -1  1\nv  1 -1  1\nv  1  1  1\nv -1  1  1\n\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n"#;\n\n    #[test]\n    fn parses_vertices_and_faces() {\n        let mesh = parse_obj_mesh(CUBE_OBJ).unwrap();\n\n        assert_eq!(mesh.vertices.len(), 8);\n        assert_eq!(mesh.faces.len(), 6);\n\n        assert_eq!(\n            mesh.vertices[0],\n            ObjVertex {\n                x: -1.0,\n                y: -1.0,\n                z: -1.0\n            }\n        );\n\n        assert_eq!(\n            mesh.faces[0],\n            ObjFace {\n                indices: vec![0, 1, 2, 3]\n            }\n        );\n    }\n\n    #[test]\n    fn derives_unique_wireframe_edges_from_faces() {\n        let mesh = parse_obj_mesh(CUBE_OBJ).unwrap();\n\n        assert_eq!(mesh.edges.len(), 12);\n        assert!(mesh.edges.contains(&(0, 1)));\n        assert!(mesh.edges.contains(&(0, 3)));\n        assert!(mesh.edges.contains(&(0, 4)));\n        assert!(mesh.edges.contains(&(6, 7)));\n    }\n\n    #[test]\n    fn supports_obj_face_tokens_with_texture_and_normal_parts() {\n        let mesh = parse_obj_mesh(\n            r#"\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1/1 2/2/1 3/3/1\n"#,\n        )\n        .unwrap();\n\n        assert_eq!(mesh.faces[0].indices, vec![0, 1, 2]);\n        assert_eq!(mesh.edges, vec![(0, 1), (0, 2), (1, 2)]);\n    }\n\n    #[test]\n    fn supports_negative_obj_indices() {\n        let mesh = parse_obj_mesh(\n            r#"\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n"#,\n        )\n        .unwrap();\n\n        assert_eq!(mesh.faces[0].indices, vec![0, 1, 2]);\n    }\n}\n'
CUBE_OBJ = '# ascii-3d standard OBJ cube\n#\n# OBJ face indices are 1-based.\n# This file intentionally stores standard geometry only.\n# ascii-3d metadata/style should live in a sidecar manifest later.\n\nv -1 -1 -1\nv  1 -1 -1\nv  1  1 -1\nv -1  1 -1\nv -1 -1  1\nv  1 -1  1\nv  1  1  1\nv -1  1  1\n\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n'
PYRAMID_OBJ = '# ascii-3d standard OBJ pyramid\n#\n# 5 vertices, 5 faces.\n# square base plus four triangular sides.\n\nv -1 -1 -1\nv  1 -1 -1\nv  1 -1  1\nv -1 -1  1\nv  0  1  0\n\nf 1 4 3 2\nf 1 2 5\nf 2 3 5\nf 3 4 5\nf 4 1 5\n'
OBJ_PROBE = 'use std::{env, process};\n\n#[path = "../mesh/mod.rs"]\nmod mesh;\n\nfn main() {\n    let path = env::args()\n        .nth(1)\n        .unwrap_or_else(|| "assets/models/cube.obj".to_string());\n\n    match mesh::obj::load_obj_mesh(&path) {\n        Ok(mesh) => {\n            println!("path={path}");\n            println!("vertices={}", mesh.vertices.len());\n            println!("faces={}", mesh.faces.len());\n            println!("edges={}", mesh.edges.len());\n\n            println!();\n            println!("first_vertices:");\n            for (index, vertex) in mesh.vertices.iter().take(5).enumerate() {\n                println!("{index}: {}, {}, {}", vertex.x, vertex.y, vertex.z);\n            }\n\n            println!();\n            println!("first_edges:");\n            for (a, b) in mesh.edges.iter().take(12) {\n                println!("{a}-{b}");\n            }\n        }\n        Err(error) => {\n            eprintln!("obj_mesh_probe error: {error}");\n            process::exit(1);\n        }\n    }\n}\n'

def ensure_mesh_module() -> None:
    mesh_dir = Path("src/mesh")
    mesh_dir.mkdir(parents=True, exist_ok=True)

    Path("src/mesh/obj.rs").write_text(OBJ_MODULE)

    mod_path = Path("src/mesh/mod.rs")
    text = mod_path.read_text() if mod_path.exists() else ""

    if "pub mod obj;" not in text:
        text = text.rstrip() + "\npub mod obj;\n"

    mod_path.write_text(text)

def ensure_main_mod() -> None:
    path = Path("src/main.rs")
    text = path.read_text()

    if "mod mesh;" not in text:
        if "mod graphics;" in text:
            text = text.replace("mod graphics;", "mod graphics;\nmod mesh;", 1)
        else:
            text = "mod mesh;\n" + text

    path.write_text(text)

def ensure_assets() -> None:
    assets = Path("assets/models")
    assets.mkdir(parents=True, exist_ok=True)

    Path("assets/models/cube.obj").write_text(CUBE_OBJ)
    Path("assets/models/pyramid.obj").write_text(PYRAMID_OBJ)

def ensure_probe_bin() -> None:
    path = Path("src/bin/obj_mesh_probe.rs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(OBJ_PROBE)

def main() -> None:
    ensure_mesh_module()
    ensure_main_mod()
    ensure_assets()
    ensure_probe_bin()

    print("Applied OBJ mesh loader foundation.")

if __name__ == "__main__":
    main()
