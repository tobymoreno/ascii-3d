use std::{error::Error, fmt};

use crate::{
    canvas::Canvas,
    math::{Mat4, Vec3},
    mesh::Mesh,
    projection::ObliqueProjector,
};

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
    pub fn model_matrix(self) -> Mat4 {
        Mat4::translation_vec3(self.translation)
            * Mat4::rotation_z(self.rotation_z)
            * Mat4::rotation_y(self.rotation_y)
            * Mat4::rotation_x(self.rotation_x)
            * Mat4::uniform_scale(self.scale)
    }

    pub fn transform_vertex(self, vertex: Vec3) -> Vec3 {
        self.model_matrix().transform_point(vertex)
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

/// Draws every unique edge exposed by the mesh using a conventional
/// scale/rotation/translation transform.
pub fn draw_wireframe(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    mesh: &Mesh,
    transform: MeshTransform,
) -> Result<(), MeshRenderError> {
    draw_wireframe_matrix(canvas, projector, mesh, transform.model_matrix())
}

/// Draws every unique edge exposed by the mesh using an arbitrary model matrix.
///
/// This is useful for parent-child transforms such as:
///
/// `camera_world * near_plane_local`
pub fn draw_wireframe_matrix(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    mesh: &Mesh,
    model_matrix: Mat4,
) -> Result<(), MeshRenderError> {
    let transformed_vertices: Vec<Vec3> = mesh
        .vertices
        .iter()
        .copied()
        .map(|vertex| model_matrix.transform_point(vertex))
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
    use crate::math::{Mat4, Vec3};

    const EPSILON: f32 = 0.000_01;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!((actual.x - expected.x).abs() <= EPSILON);
        assert!((actual.y - expected.y).abs() <= EPSILON);
        assert!((actual.z - expected.z).abs() <= EPSILON);
    }

    #[test]
    fn default_transform_leaves_vertex_unchanged() {
        let vertex = Vec3::new(1.0, 2.0, 3.0);
        let transformed = MeshTransform::default().transform_vertex(vertex);

        assert_vec3_close(transformed, vertex);
    }

    #[test]
    fn transform_applies_scale_and_translation() {
        let transform = MeshTransform {
            scale: 2.0,
            translation: Vec3::new(10.0, 20.0, 30.0),
            ..MeshTransform::default()
        };

        let transformed = transform.transform_vertex(Vec3::new(1.0, 2.0, 3.0));

        assert_vec3_close(transformed, Vec3::new(12.0, 24.0, 36.0));
    }

    #[test]
    fn parent_child_matrix_rotates_local_translation_with_parent() {
        let parent = Mat4::rotation_x(90.0_f32.to_radians());
        let child = Mat4::translation(0.0, 0.0, -0.25);

        let center = (parent * child).transform_point(Vec3::zero());

        assert_vec3_close(center, Vec3::new(0.0, 0.25, 0.0));
    }
}
