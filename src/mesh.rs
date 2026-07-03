use std::collections::BTreeSet;

use crate::math::Vec3;

pub type Edge = (usize, usize);

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy)]
pub struct MeshBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl MeshBounds {
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    pub fn largest_dimension(self) -> f32 {
        let size = self.size();

        size.x.max(size.y).max(size.z)
    }
}

impl Mesh {
    pub fn new(vertices: Vec<Vec3>, faces: Vec<Vec<usize>>) -> Self {
        Self { vertices, faces }
    }

    /// Creates a box centered at local origin.
    ///
    /// The largest dimension is 1.0:
    ///
    /// x: -0.5 .. +0.5
    /// y: -0.5 .. +0.5
    /// z: -0.5 .. +0.5
    pub fn unit_box() -> Self {
        let vertices = vec![
            // z = -0.5 face
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(0.5, -0.5, -0.5),  // 1
            Vec3::new(0.5, 0.5, -0.5),   // 2
            Vec3::new(-0.5, 0.5, -0.5),  // 3
            // z = +0.5 face
            Vec3::new(-0.5, -0.5, 0.5), // 4
            Vec3::new(0.5, -0.5, 0.5),  // 5
            Vec3::new(0.5, 0.5, 0.5),   // 6
            Vec3::new(-0.5, 0.5, 0.5),  // 7
        ];

        let faces = vec![
            vec![0, 1, 2, 3], // back
            vec![4, 5, 6, 7], // front
            vec![0, 1, 5, 4], // bottom
            vec![3, 2, 6, 7], // top
            vec![0, 3, 7, 4], // left
            vec![1, 2, 6, 5], // right
        ];

        Self::new(vertices, faces)
    }

    pub fn bounds(&self) -> Option<MeshBounds> {
        let first = *self.vertices.first()?;

        let mut min = first;
        let mut max = first;

        for vertex in &self.vertices {
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);

            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }

        Some(MeshBounds { min, max })
    }

    /// Centers the mesh at `(0,0,0)` and scales it uniformly so
    /// its largest bounding-box dimension equals `target_size`.
    ///
    /// Returns false for an empty or completely degenerate mesh.
    pub fn normalize_to_size(&mut self, target_size: f32) -> bool {
        if target_size <= 0.0 {
            return false;
        }

        let Some(bounds) = self.bounds() else {
            return false;
        };

        let largest_dimension = bounds.largest_dimension();

        if largest_dimension <= f32::EPSILON {
            return false;
        }

        let center = bounds.center();
        let scale = target_size / largest_dimension;

        for vertex in &mut self.vertices {
            *vertex = (*vertex - center) * scale;
        }

        true
    }

    /// Derives all unique polygon boundary edges.
    ///
    /// An edge is stored in canonical order:
    ///
    ///     (min_index, max_index)
    ///
    /// Therefore `(2, 5)` and `(5, 2)` are treated as the same edge.
    pub fn unique_edges(&self) -> Vec<Edge> {
        let mut edges = BTreeSet::new();

        for face in &self.faces {
            if face.len() < 2 {
                continue;
            }

            for index in 0..face.len() {
                let a = face[index];
                let b = face[(index + 1) % face.len()];

                if a == b {
                    continue;
                }

                let edge = if a < b { (a, b) } else { (b, a) };

                edges.insert(edge);
            }
        }

        edges.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Mesh;

    #[test]
    fn unit_box_has_expected_geometry() {
        let mesh = Mesh::unit_box();

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 6);
        assert_eq!(mesh.unique_edges().len(), 12);
    }

    #[test]
    fn unit_box_is_centered_and_normalized() {
        let mesh = Mesh::unit_box();
        let bounds = mesh.bounds().expect("unit box must have bounds");

        assert_eq!(bounds.center().x, 0.0);
        assert_eq!(bounds.center().y, 0.0);
        assert_eq!(bounds.center().z, 0.0);

        assert_eq!(bounds.largest_dimension(), 1.0);
    }

    #[test]
    fn normalization_centers_and_scales_mesh() {
        let mut mesh = Mesh::new(
            vec![
                crate::math::Vec3::new(10.0, 20.0, 30.0),
                crate::math::Vec3::new(14.0, 22.0, 32.0),
            ],
            Vec::new(),
        );

        assert!(mesh.normalize_to_size(1.0));

        let bounds = mesh.bounds().expect("normalized mesh must have bounds");
        let center = bounds.center();

        assert!(center.x.abs() <= f32::EPSILON);
        assert!(center.y.abs() <= f32::EPSILON);
        assert!(center.z.abs() <= f32::EPSILON);

        assert!((bounds.largest_dimension() - 1.0).abs() <= f32::EPSILON);
    }
}
