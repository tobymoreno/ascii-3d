use std::{error::Error, fmt};

use crate::{canvas::Canvas, math::Vec3, mesh::Mesh, projection::ObliqueProjector};

#[derive(Debug, Clone, Copy)]
pub struct MeshTransform {
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub scale: f32,
    pub translation: Vec3,
}

impl Default for MeshTransform {
    fn default() -> Self {
        Self {
            rotation_x: 0.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            translation: Vec3::zero(),
        }
    }
}

impl MeshTransform {
    pub fn transform_vertex(self, vertex: Vec3) -> Vec3 {
        let rotated = vertex
            .rotate_x(self.rotation_x)
            .rotate_y(self.rotation_y)
            .rotate_z(self.rotation_z);

        rotated * self.scale + self.translation
    }
}

#[derive(Debug)]
pub struct MeshRenderError {
    vertex_index: usize,
    vertex_count: usize,
}

impl fmt::Display for MeshRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "mesh edge references vertex index {}, but the mesh has only {} vertices",
            self.vertex_index, self.vertex_count,
        )
    }
}

impl Error for MeshRenderError {}

/// Draws every unique edge exposed by the mesh.
///
/// Polygon faces contribute their closed boundary edges. Explicit OBJ
/// `l` records are converted by the loader into two-point primitives and
/// therefore pass through this same rendering path.
pub fn draw_wireframe(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    mesh: &Mesh,
    transform: MeshTransform,
) -> Result<(), MeshRenderError> {
    let transformed_vertices: Vec<Vec3> = mesh
        .vertices
        .iter()
        .copied()
        .map(|vertex| transform.transform_vertex(vertex))
        .collect();

    for (start_index, end_index) in mesh.unique_edges() {
        let start = transformed_vertices
            .get(start_index)
            .copied()
            .ok_or(MeshRenderError {
                vertex_index: start_index,
                vertex_count: transformed_vertices.len(),
            })?;

        let end = transformed_vertices
            .get(end_index)
            .copied()
            .ok_or(MeshRenderError {
                vertex_index: end_index,
                vertex_count: transformed_vertices.len(),
            })?;

        canvas.draw_line_auto(projector.project(start), projector.project(end));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::MeshTransform;
    use crate::math::Vec3;

    #[test]
    fn default_transform_leaves_vertex_unchanged() {
        let vertex = Vec3::new(1.0, 2.0, 3.0);
        let transformed = MeshTransform::default().transform_vertex(vertex);

        assert!((transformed.x - vertex.x).abs() <= f32::EPSILON);

        assert!((transformed.y - vertex.y).abs() <= f32::EPSILON);

        assert!((transformed.z - vertex.z).abs() <= f32::EPSILON);
    }

    #[test]
    fn transform_applies_scale_and_translation() {
        let transform = MeshTransform {
            scale: 2.0,
            translation: Vec3::new(10.0, 20.0, 30.0),
            ..MeshTransform::default()
        };

        let transformed = transform.transform_vertex(Vec3::new(1.0, 2.0, 3.0));

        assert!((transformed.x - 12.0).abs() <= f32::EPSILON);
        assert!((transformed.y - 24.0).abs() <= f32::EPSILON);
        assert!((transformed.z - 36.0).abs() <= f32::EPSILON);
    }
}
