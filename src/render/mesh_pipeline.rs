use crate::mesh::Mesh;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedMeshVertex {
    pub world: [f32; 3],
    pub camera: Option<[f32; 3]>,
    pub screen: Option<(i32, i32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedFrameMesh {
    pub vertices: Vec<ProjectedMeshVertex>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedMeshTriangle {
    pub indexes: [usize; 3],
    pub world_normal: [f32; 3],
    pub screen: [(i32, i32, f32); 3],
}

pub fn prepare_frame_mesh(
    mesh: &Mesh,
    mut transform_world: impl FnMut([f32; 3]) -> [f32; 3],
    mut world_to_camera: impl FnMut([f32; 3]) -> Option<[f32; 3]>,
    mut project_camera: impl FnMut([f32; 3]) -> Option<(i32, i32, f32)>,
) -> PreparedFrameMesh {
    let vertices = mesh
        .vertices
        .iter()
        .map(|vertex| {
            let world = transform_world([vertex.x, vertex.y, vertex.z]);
            let camera = world_to_camera(world);
            let screen = camera.and_then(&mut project_camera);

            ProjectedMeshVertex {
                world,
                camera,
                screen,
            }
        })
        .collect();

    PreparedFrameMesh { vertices }
}

pub fn visit_prepared_triangles(
    mesh: &Mesh,
    prepared: &PreparedFrameMesh,
    backface_cull: bool,
    mut visitor: impl FnMut(PreparedMeshTriangle),
) {
    for primitive in &mesh.faces {
        if primitive.len() < 3 {
            continue;
        }

        let first = primitive[0];
        for triangle_index in 1..primitive.len() - 1 {
            let indexes = [
                first,
                primitive[triangle_index],
                primitive[triangle_index + 1],
            ];

            if indexes
                .iter()
                .any(|index| *index >= prepared.vertices.len())
            {
                continue;
            }

            let vertices = [
                prepared.vertices[indexes[0]],
                prepared.vertices[indexes[1]],
                prepared.vertices[indexes[2]],
            ];

            let world_normal = cross(
                subtract(vertices[1].world, vertices[0].world),
                subtract(vertices[2].world, vertices[0].world),
            );
            let Some(world_normal) = normalized(world_normal) else {
                continue;
            };

            if backface_cull {
                let [Some(camera0), Some(camera1), Some(camera2)] =
                    [vertices[0].camera, vertices[1].camera, vertices[2].camera]
                else {
                    continue;
                };

                let camera_normal = cross(subtract(camera1, camera0), subtract(camera2, camera0));
                let centroid = scale(add(add(camera0, camera1), camera2), 1.0 / 3.0);
                if dot(camera_normal, centroid) >= 0.0 {
                    continue;
                }
            }

            let [Some(screen0), Some(screen1), Some(screen2)] =
                [vertices[0].screen, vertices[1].screen, vertices[2].screen]
            else {
                continue;
            };

            visitor(PreparedMeshTriangle {
                indexes,
                world_normal,
                screen: [screen0, screen1, screen2],
            });
        }
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(value: [f32; 3], factor: f32) -> [f32; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized(value: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(value, value).sqrt();
    (length > f32::EPSILON).then(|| scale(value, 1.0 / length))
}

#[cfg(test)]
mod tests {
    use crate::{math::Vec3, mesh::Mesh};

    use super::*;

    fn triangle_mesh() -> Mesh {
        Mesh::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![vec![0, 1, 2]],
        )
    }

    #[test]
    fn transforms_and_projects_each_unique_vertex_once() {
        let mesh = triangle_mesh();
        let mut transform_calls = 0;
        let mut camera_calls = 0;
        let mut projection_calls = 0;

        let prepared = prepare_frame_mesh(
            &mesh,
            |position| {
                transform_calls += 1;
                [position[0] + 2.0, position[1] + 3.0, position[2] + 4.0]
            },
            |world| {
                camera_calls += 1;
                Some(world)
            },
            |camera| {
                projection_calls += 1;
                Some((camera[0] as i32, camera[1] as i32, camera[2]))
            },
        );

        assert_eq!(prepared.vertices.len(), 3);
        assert_eq!(transform_calls, 3);
        assert_eq!(camera_calls, 3);
        assert_eq!(projection_calls, 3);
    }

    #[test]
    fn visits_one_triangle_for_triangle_face() {
        let mesh = triangle_mesh();
        let prepared = prepare_frame_mesh(
            &mesh,
            |position| position,
            Some,
            |camera| Some((camera[0] as i32, camera[1] as i32, camera[2] + 1.0)),
        );

        let mut count = 0;
        visit_prepared_triangles(&mesh, &prepared, false, |_| count += 1);
        assert_eq!(count, 1);
    }
}
