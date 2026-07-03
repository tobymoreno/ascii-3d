use std::{error::Error, fmt, fs, path::Path};

use crate::{math::Vec3, mesh::Mesh};

#[derive(Debug)]
pub enum ObjError {
    Io(std::io::Error),
    Parse { line: usize, message: String },
}

impl ObjError {
    fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ObjError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => {
                write!(formatter, "failed to read OBJ file: {error}")
            }

            Self::Parse { line, message } => {
                write!(formatter, "OBJ parse error on line {line}: {message}")
            }
        }
    }
}

impl Error for ObjError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Parse { .. } => None,
        }
    }
}

impl From<std::io::Error> for ObjError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn load_obj(path: impl AsRef<Path>) -> Result<Mesh, ObjError> {
    let source = fs::read_to_string(path)?;
    parse_obj(&source)
}

pub fn parse_obj(source: &str) -> Result<Mesh, ObjError> {
    let mut vertices = Vec::new();
    let mut primitives = Vec::new();

    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;

        let line = original_line.split('#').next().unwrap_or("").trim();

        if line.is_empty() {
            continue;
        }

        let mut fields = line.split_whitespace();

        let Some(record_type) = fields.next() else {
            continue;
        };

        match record_type {
            "v" => {
                let x = parse_f32(fields.next(), line_number, "vertex x")?;
                let y = parse_f32(fields.next(), line_number, "vertex y")?;
                let z = parse_f32(fields.next(), line_number, "vertex z")?;

                vertices.push(Vec3::new(x, y, z));
            }

            "f" => {
                let face = parse_vertex_indexes(fields, vertices.len(), line_number)?;

                if face.len() < 3 {
                    return Err(ObjError::parse(
                        line_number,
                        "a face must contain at least 3 vertices",
                    ));
                }

                primitives.push(face);
            }

            "l" => {
                let line_indexes = parse_vertex_indexes(fields, vertices.len(), line_number)?;

                if line_indexes.len() < 2 {
                    return Err(ObjError::parse(
                        line_number,
                        "a line must contain at least 2 vertices",
                    ));
                }

                // Convert an OBJ polyline into consecutive two-point
                // primitives. This lets the existing Mesh and wireframe
                // renderer draw line assets without introducing duplicate
                // skinny face geometry.
                for pair in line_indexes.windows(2) {
                    primitives.push(vec![pair[0], pair[1]]);
                }
            }

            // Supported OBJ records that this simple loader ignores.
            "vn" | "vt" | "o" | "g" | "s" | "usemtl" | "mtllib" => {}

            // Ignore unsupported records so simple downloaded OBJ files
            // can still load when they contain extra metadata.
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err(ObjError::parse(0, "OBJ file contains no vertices"));
    }

    Ok(Mesh::new(vertices, primitives))
}

fn parse_vertex_indexes<'a>(
    fields: impl Iterator<Item = &'a str>,
    vertex_count: usize,
    line_number: usize,
) -> Result<Vec<usize>, ObjError> {
    let mut indexes = Vec::new();

    for token in fields {
        let vertex_index_text = token.split('/').next().unwrap_or("");

        if vertex_index_text.is_empty() {
            return Err(ObjError::parse(
                line_number,
                format!("invalid vertex element '{token}'"),
            ));
        }

        let obj_index = vertex_index_text.parse::<isize>().map_err(|_| {
            ObjError::parse(
                line_number,
                format!("invalid vertex index '{vertex_index_text}'"),
            )
        })?;

        indexes.push(resolve_obj_index(obj_index, vertex_count, line_number)?);
    }

    Ok(indexes)
}

fn parse_f32(value: Option<&str>, line_number: usize, field_name: &str) -> Result<f32, ObjError> {
    let value =
        value.ok_or_else(|| ObjError::parse(line_number, format!("missing {field_name}")))?;

    value
        .parse::<f32>()
        .map_err(|_| ObjError::parse(line_number, format!("invalid {field_name} value '{value}'")))
}

fn resolve_obj_index(
    obj_index: isize,
    vertex_count: usize,
    line_number: usize,
) -> Result<usize, ObjError> {
    if obj_index == 0 {
        return Err(ObjError::parse(line_number, "OBJ index 0 is invalid"));
    }

    let resolved = if obj_index > 0 {
        obj_index - 1
    } else {
        vertex_count as isize + obj_index
    };

    if resolved < 0 || resolved >= vertex_count as isize {
        return Err(ObjError::parse(
            line_number,
            format!(
                "vertex index {obj_index} is out of range; \
                 {vertex_count} vertices are currently defined"
            ),
        ));
    }

    Ok(resolved as usize)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{load_obj, parse_obj};

    #[test]
    fn parses_box_vertices_and_faces() {
        let source = r#"
            v -0.5 -0.5 -0.5
            v  0.5 -0.5 -0.5
            v  0.5  0.5 -0.5
            v -0.5  0.5 -0.5

            v -0.5 -0.5  0.5
            v  0.5 -0.5  0.5
            v  0.5  0.5  0.5
            v -0.5  0.5  0.5

            f 1 2 3 4
            f 5 8 7 6
            f 1 5 6 2
            f 4 3 7 8
            f 1 4 8 5
            f 2 6 7 3
        "#;

        let mesh = parse_obj(source).expect("box OBJ should parse");

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 6);
        assert_eq!(mesh.unique_edges().len(), 12);
    }

    #[test]
    fn parses_face_elements_with_slashes() {
        let source = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0
            v 0.0 1.0 0.0

            f 1/4/7 2/5/7 3/6/7
        "#;

        let mesh = parse_obj(source).expect("slash indexes should parse");

        assert_eq!(mesh.faces, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn parses_negative_relative_indexes() {
        let source = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0
            v 0.0 1.0 0.0

            f -3 -2 -1
        "#;

        let mesh = parse_obj(source).expect("negative indexes should parse");

        assert_eq!(mesh.faces, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn parses_obj_line_as_consecutive_segments() {
        let source = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0
            v 2.0 0.0 0.0

            l 1 2 3
        "#;

        let mesh = parse_obj(source).expect("line indexes should parse");

        assert_eq!(mesh.faces, vec![vec![0, 1], vec![1, 2]],);

        assert_eq!(mesh.unique_edges(), vec![(0, 1), (1, 2)],);
    }

    #[test]
    fn parses_line_elements_with_slashes() {
        let source = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0

            l 1/4 2/5
        "#;

        let mesh = parse_obj(source).expect("line slash indexes should parse");

        assert_eq!(mesh.faces, vec![vec![0, 1]]);
    }

    #[test]
    fn rejects_zero_index() {
        let source = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0
            v 0.0 1.0 0.0

            f 0 2 3
        "#;

        let error = parse_obj(source).expect_err("index zero must fail");

        assert!(error.to_string().contains("index 0 is invalid"));
    }

    #[test]
    fn loads_box_asset_from_disk() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("box.obj");

        let mesh = load_obj(&path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display(),));

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 6);
        assert_eq!(mesh.unique_edges().len(), 12);

        let bounds = mesh.bounds().expect("box must have bounds");

        assert!(bounds.center().x.abs() <= f32::EPSILON);
        assert!(bounds.center().y.abs() <= f32::EPSILON);
        assert!(bounds.center().z.abs() <= f32::EPSILON);

        assert!((bounds.largest_dimension() - 1.0).abs() <= f32::EPSILON);
    }

    #[test]
    fn loads_cartesian_line_asset_from_disk() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("cartesian_axes.obj");

        let mesh = load_obj(&path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display(),));

        assert_eq!(mesh.vertices.len(), 10);
        assert_eq!(mesh.unique_edges().len(), 9);
    }
}
