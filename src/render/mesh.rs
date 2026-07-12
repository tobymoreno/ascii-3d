use std::{fs, io, path::Path};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshTriangle {
    pub a: MeshVertex,
    pub b: MeshVertex,
    pub c: MeshVertex,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshAsset {
    pub triangles: Vec<MeshTriangle>,
    pub vertex_count: usize,
    pub normal_count: usize,
}

pub fn load_obj_mesh(path: impl AsRef<Path>) -> io::Result<MeshAsset> {
    let text = fs::read_to_string(path)?;
    load_obj_mesh_from_str(&text)
}

pub fn load_obj_mesh_from_str(text: &str) -> io::Result<MeshAsset> {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut triangles = Vec::<MeshTriangle>::new();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };

        match kind {
            "v" => {
                let x = parse_f32(parts.next(), "missing vertex x")?;
                let y = parse_f32(parts.next(), "missing vertex y")?;
                let z = parse_f32(parts.next(), "missing vertex z")?;
                positions.push([x, y, z]);
            }
            "vn" => {
                let x = parse_f32(parts.next(), "missing normal x")?;
                let y = parse_f32(parts.next(), "missing normal y")?;
                let z = parse_f32(parts.next(), "missing normal z")?;
                normals.push(normalized([x, y, z]));
            }
            "f" => {
                let refs = parts.map(parse_face_ref).collect::<io::Result<Vec<_>>>()?;

                if refs.len() < 3 {
                    continue;
                }

                for i in 1..refs.len() - 1 {
                    let a = vertex_from_ref(refs[0], &positions, &normals)?;
                    let b = vertex_from_ref(refs[i], &positions, &normals)?;
                    let c = vertex_from_ref(refs[i + 1], &positions, &normals)?;
                    triangles.push(MeshTriangle { a, b, c });
                }
            }
            _ => {}
        }
    }

    Ok(MeshAsset {
        triangles,
        vertex_count: positions.len(),
        normal_count: normals.len(),
    })
}

#[derive(Clone, Copy, Debug)]
struct FaceRef {
    vertex_index: usize,
    normal_index: Option<usize>,
}

fn parse_face_ref(text: &str) -> io::Result<FaceRef> {
    let mut parts = text.split('/');

    let vertex_index = parts
        .next()
        .ok_or_else(|| invalid_data("missing face vertex index"))?
        .parse::<usize>()
        .map_err(|error| invalid_data(format!("invalid face vertex index: {error}")))?
        .checked_sub(1)
        .ok_or_else(|| invalid_data("OBJ indices are 1-based"))?;

    let _uv_index = parts.next();

    let normal_index = parts
        .next()
        .and_then(|part| {
            if part.is_empty() {
                None
            } else {
                Some(part.parse::<usize>())
            }
        })
        .transpose()
        .map_err(|error| invalid_data(format!("invalid normal index: {error}")))?
        .and_then(|index| index.checked_sub(1));

    Ok(FaceRef {
        vertex_index,
        normal_index,
    })
}

fn vertex_from_ref(
    reference: FaceRef,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
) -> io::Result<MeshVertex> {
    let position = *positions
        .get(reference.vertex_index)
        .ok_or_else(|| invalid_data("face vertex index out of bounds"))?;

    let normal = reference
        .normal_index
        .and_then(|index| normals.get(index).copied())
        .unwrap_or_else(|| normalized(position));

    Ok(MeshVertex { position, normal })
}

fn parse_f32(value: Option<&str>, message: &'static str) -> io::Result<f32> {
    value
        .ok_or_else(|| invalid_data(message))?
        .parse::<f32>()
        .map_err(|error| invalid_data(format!("{message}: {error}")))
}

fn normalized(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();

    if length <= f32::EPSILON {
        return [0.0, 1.0, 0.0];
    }

    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::load_obj_mesh_from_str;

    #[test]
    fn loads_single_triangle_obj() {
        let mesh = load_obj_mesh_from_str(
            r#"
            v 0 0 0
            v 1 0 0
            v 0 1 0
            vn 0 0 1
            f 1//1 2//1 3//1
            "#,
        )
        .expect("mesh should load");

        assert_eq!(mesh.vertex_count, 3);
        assert_eq!(mesh.normal_count, 1);
        assert_eq!(mesh.triangles.len(), 1);
        assert_eq!(mesh.triangles[0].a.position, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.triangles[0].b.position, [1.0, 0.0, 0.0]);
        assert_eq!(mesh.triangles[0].c.position, [0.0, 1.0, 0.0]);
        assert_eq!(mesh.triangles[0].a.normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn triangulates_quad_faces() {
        let mesh = load_obj_mesh_from_str(
            r#"
            v 0 0 0
            v 1 0 0
            v 1 1 0
            v 0 1 0
            f 1 2 3 4
            "#,
        )
        .expect("mesh should load");

        assert_eq!(mesh.vertex_count, 4);
        assert_eq!(mesh.triangles.len(), 2);
    }
}
