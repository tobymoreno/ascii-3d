use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::{self, Write},
    path::Path,
};

use crate::math::Vec3;

pub type Edge = (usize, usize);

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,

    /// Polygon faces and explicit two-point line segments.
    ///
    /// Polygon faces contain three or more indexes. OBJ `l` records are
    /// converted by the loader into consecutive two-index entries so the
    /// existing wireframe renderer can draw both kinds of geometry through
    /// the same `unique_edges()` interface.
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

    /// Returns a lower-detail mesh by clustering nearby vertices into a
    /// regular 3D grid and rebuilding primitives with the remapped indexes.
    ///
    /// This is intended for low-resolution ASCII/TUI rendering, where a
    /// coherent simplified wireframe usually looks better than randomly
    /// skipping edges from the original mesh.
    pub fn simplify_by_vertex_grid(&self, grid_size: f32) -> Self {
        if grid_size <= 0.0 || self.vertices.is_empty() {
            return self.clone();
        }

        let Some(bounds) = self.bounds() else {
            return self.clone();
        };

        let origin = bounds.min;
        let mut clusters: HashMap<(i32, i32, i32), usize> = HashMap::new();
        let mut sums = Vec::new();
        let mut counts = Vec::new();
        let mut old_to_new = Vec::with_capacity(self.vertices.len());

        for vertex in &self.vertices {
            let local = *vertex - origin;
            let key = (
                (local.x / grid_size).floor() as i32,
                (local.y / grid_size).floor() as i32,
                (local.z / grid_size).floor() as i32,
            );

            let new_index = match clusters.get(&key).copied() {
                Some(index) => index,
                None => {
                    let index = sums.len();
                    clusters.insert(key, index);
                    sums.push(Vec3::new(0.0, 0.0, 0.0));
                    counts.push(0usize);
                    index
                }
            };

            sums[new_index] = sums[new_index] + *vertex;
            counts[new_index] += 1;
            old_to_new.push(new_index);
        }

        let vertices = sums
            .into_iter()
            .zip(counts)
            .map(|(sum, count)| sum * (1.0 / count as f32))
            .collect();

        let mut faces = Vec::new();

        for primitive in &self.faces {
            match primitive.as_slice() {
                [a, b] => {
                    let a = old_to_new[*a];
                    let b = old_to_new[*b];

                    if a != b {
                        faces.push(vec![a, b]);
                    }
                }

                indexes if indexes.len() >= 3 => {
                    let mut remapped = Vec::new();

                    for index in indexes {
                        let new_index = old_to_new[*index];

                        if remapped.last().copied() != Some(new_index) {
                            remapped.push(new_index);
                        }
                    }

                    if remapped.len() >= 2 && remapped.first().copied() == remapped.last().copied()
                    {
                        remapped.pop();
                    }

                    let unique_indexes: BTreeSet<_> = remapped.iter().copied().collect();

                    if unique_indexes.len() >= 3 {
                        faces.push(remapped);
                    }
                }

                _ => {}
            }
        }

        Self::new(vertices, faces)
    }

    /// Simplifies toward a requested vertex budget by searching for a
    /// vertex-grid size that produces no more than the target.
    pub fn simplify_to_target_vertices(&self, target_vertices: usize) -> Self {
        if target_vertices == 0 || self.vertices.len() <= target_vertices {
            return self.clone();
        }

        let Some(bounds) = self.bounds() else {
            return self.clone();
        };
        let max_dimension = bounds.largest_dimension();
        if max_dimension <= f32::EPSILON {
            return self.clone();
        }

        let mut low = max_dimension / 4096.0;
        let mut high = max_dimension;
        let mut best = self.clone();

        for _ in 0..24 {
            let grid = (low + high) * 0.5;
            let candidate = self.simplify_by_vertex_grid(grid);

            if candidate.vertices.len() > target_vertices {
                low = grid;
            } else {
                high = grid;
                if candidate.vertices.len() > best.vertices.len()
                    || best.vertices.len() > target_vertices
                {
                    best = candidate;
                }
            }
        }

        if best.vertices.len() > target_vertices {
            self.simplify_by_vertex_grid(high)
        } else {
            best
        }
    }

    /// Writes the mesh as a simple OBJ containing positions and faces/lines.
    pub fn write_obj(&self, path: &Path) -> io::Result<()> {
        let mut file = File::create(path)?;

        for vertex in &self.vertices {
            writeln!(file, "v {} {} {}", vertex.x, vertex.y, vertex.z)?;
        }

        for primitive in &self.faces {
            match primitive.as_slice() {
                [a, b] => {
                    writeln!(file, "l {} {}", a + 1, b + 1)?;
                }
                indexes if indexes.len() >= 3 => {
                    write!(file, "f")?;
                    for index in indexes {
                        write!(file, " {}", index + 1)?;
                    }
                    writeln!(file)?;
                }
                _ => {}
            }
        }

        file.flush()
    }

    /// Derives every unique drawable edge.
    ///
    /// Entries with two indexes represent explicit line segments.
    /// Entries with three or more indexes represent closed polygon faces.
    ///
    /// An edge is stored in canonical order as `(min_index, max_index)`.
    ///
    /// Therefore `(2, 5)` and `(5, 2)` are treated as the same edge.
    pub fn unique_edges(&self) -> Vec<Edge> {
        let mut edges = BTreeSet::new();

        for primitive in &self.faces {
            match primitive.as_slice() {
                [a, b] => {
                    insert_edge(&mut edges, *a, *b);
                }

                indexes if indexes.len() >= 3 => {
                    for index in 0..indexes.len() {
                        let a = indexes[index];
                        let b = indexes[(index + 1) % indexes.len()];

                        insert_edge(&mut edges, a, b);
                    }
                }

                _ => {}
            }
        }

        edges.into_iter().collect()
    }
}

fn insert_edge(edges: &mut BTreeSet<Edge>, a: usize, b: usize) {
    if a == b {
        return;
    }

    let edge = if a < b { (a, b) } else { (b, a) };

    edges.insert(edge);
}

#[cfg(test)]
mod tests {
    use super::Mesh;
    use crate::math::Vec3;

    #[test]
    fn unit_box_has_expected_geometry() {
        let mesh = Mesh::unit_box();

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 6);
        assert_eq!(mesh.unique_edges().len(), 12);
    }

    #[test]
    fn simplify_by_vertex_grid_preserves_unit_box_with_tiny_grid() {
        let mesh = Mesh::unit_box();
        let simplified = mesh.simplify_by_vertex_grid(0.01);

        assert_eq!(simplified.vertices.len(), mesh.vertices.len());
        assert_eq!(simplified.faces.len(), mesh.faces.len());
        assert_eq!(simplified.unique_edges().len(), mesh.unique_edges().len());
    }

    #[test]
    fn simplify_by_vertex_grid_drops_degenerate_collapsed_faces() {
        let mesh = Mesh::unit_box();
        let simplified = mesh.simplify_by_vertex_grid(10.0);

        assert_eq!(simplified.vertices.len(), 1);
        assert!(simplified.faces.is_empty());
        assert!(simplified.unique_edges().is_empty());
    }

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
            vec![Vec3::new(10.0, 20.0, 30.0), Vec3::new(14.0, 22.0, 32.0)],
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

    #[test]
    fn explicit_two_point_segments_are_not_closed_again() {
        let mesh = Mesh::new(
            vec![
                Vec3::zero(),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ],
            vec![vec![0, 1], vec![1, 2]],
        );

        assert_eq!(mesh.unique_edges(), vec![(0, 1), (1, 2)],);
    }
}
