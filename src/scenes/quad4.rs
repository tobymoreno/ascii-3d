use std::io;

use crate::{
    axis_metadata::CartesianAxesMetadata,
    canvas::Canvas,
    geometry2d::Point2,
    math::Vec3,
    mesh::Mesh,
    mesh_renderer::{MeshTransform, draw_wireframe},
    projection::ObliqueProjector,
};

use super::render_asset_axes;

const SCENE_ROTATION_X_RADIANS: f32 = 0.0;
const SCENE_ROTATION_Y_RADIANS: f32 = 0.0;
const SCENE_ROTATION_Z_RADIANS: f32 = 0.0;
const SCENE_SCALE: f32 = 3.0;

fn scene_transform() -> MeshTransform {
    MeshTransform {
        rotation_x: SCENE_ROTATION_X_RADIANS,
        rotation_y: SCENE_ROTATION_Y_RADIANS,
        rotation_z: SCENE_ROTATION_Z_RADIANS,
        scale: SCENE_SCALE,
        translation: Vec3::zero(),
    }
}

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    quad_mesh: &Mesh,
    axes_mesh: &Mesh,
    axes_metadata: &CartesianAxesMetadata,
) -> io::Result<()> {
    if quad_mesh.vertices.len() != 4 {
        return Err(io::Error::other(format!(
            "quad4 scene expected 4 vertices, but loaded {}",
            quad_mesh.vertices.len(),
        )));
    }

    if quad_mesh.faces.len() != 1 {
        return Err(io::Error::other(format!(
            "quad4 scene expected 1 face, but loaded {}",
            quad_mesh.faces.len(),
        )));
    }

    let transform = scene_transform();

    render_asset_axes(canvas, projector, axes_mesh, axes_metadata, transform)?;

    draw_wireframe(canvas, projector, quad_mesh, transform).map_err(io::Error::other)?;

    for (index, vertex) in quad_mesh.vertices.iter().enumerate() {
        let displayed_vertex = transform.transform_vertex(*vertex);

        let projected = projector.project(displayed_vertex);

        canvas.set(
            projected,
            char::from_digit((index + 1) as u32, 10).unwrap_or('*'),
        );

        canvas.draw_text(
            Point2::new(projected.x + 2, projected.y),
            &format!("P{}", index + 1),
        );
    }

    canvas.draw_text(
        Point2::new(2, 1),
        "Scene: loaded quad4.obj with asset-driven Cartesian axes",
    );

    canvas.draw_text(Point2::new(2, 21), "Geometry:");

    canvas.draw_text(Point2::new(2, 22), "  assets/quad4.obj");

    canvas.draw_text(Point2::new(2, 23), "  assets/cartesian_axes.obj");

    canvas.draw_text(Point2::new(2, 24), "Metadata:");

    canvas.draw_text(Point2::new(2, 25), "  assets/cartesian_axes.json");

    canvas.draw_text(
        Point2::new(2, 26),
        "Quad and axes use the same scene transform.",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::{
        axis_metadata::{AxisMetadata, CartesianAxesMetadata, DisplayMetadata, OriginMetadata},
        canvas::Canvas,
        geometry2d::Point2,
        math::Vec3,
        mesh::Mesh,
        projection::ObliqueProjector,
    };

    fn quad_mesh() -> Mesh {
        Mesh {
            vertices: vec![
                Vec3::new(-0.5, -0.5, 0.0),
                Vec3::new(0.5, -0.5, 0.0),
                Vec3::new(0.5, 0.5, 0.0),
                Vec3::new(-0.5, 0.5, 0.0),
            ],
            faces: vec![vec![0, 1, 2, 3]],
        }
    }

    fn axes_mesh() -> Mesh {
        Mesh {
            vertices: vec![
                Vec3::zero(),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            faces: vec![vec![0, 1, 2], vec![0, 2, 3]],
        }
    }

    fn axis(id: &str, label: &str, label_position: [f32; 3]) -> AxisMetadata {
        AxisMetadata {
            id: id.to_string(),
            group_shaft: format!("{id}_axis_shaft"),
            group_arrow: format!("{id}_axis_arrow"),
            positive_direction: [0.0, 0.0, 0.0],
            negative_direction: [0.0, 0.0, 0.0],
            length: 1.0,
            positive_endpoint: label_position,
            positive_label: label.to_string(),
            negative_label: format!("-{}", id.to_uppercase()),
            positive_label_position: label_position,
            negative_label_position: [0.0, 0.0, 0.0],
        }
    }

    fn axes_metadata() -> CartesianAxesMetadata {
        CartesianAxesMetadata {
            name: "cartesian_axes".to_string(),
            version: 1,
            units: "world".to_string(),
            geometry_asset: "cartesian_axes.obj".to_string(),
            origin: OriginMetadata {
                position: [0.0, 0.0, 0.0],
                label: "O".to_string(),
                group: "origin_marker".to_string(),
            },
            axes: vec![
                axis("x", "+X", [1.2, 0.0, 0.0]),
                axis("y", "+Y", [0.0, 1.2, 0.0]),
                axis("z", "+Z", [0.0, 0.0, 1.2]),
            ],
            display: DisplayMetadata {
                show_origin: true,
                show_positive_labels: true,
                show_negative_labels: false,
                default_axis_length: 1.0,
                arrowhead_length: 0.2,
                label_strategy: "sidecar_metadata".to_string(),
                notes: Vec::new(),
            },
        }
    }

    #[test]
    fn renders_loaded_quad_with_loaded_axes() {
        let quad = quad_mesh();
        let axes = axes_mesh();
        let metadata = axes_metadata();

        let mut canvas = Canvas::new(80, 28);

        let projector = ObliqueProjector::new(Point2::new(34, 14));

        render(&mut canvas, &projector, &quad, &axes, &metadata)
            .expect("quad and asset axes should render");

        let output = canvas.render();

        assert!(output.contains("+X"));
        assert!(output.contains("+Y"));
        assert!(output.contains("+Z"));
        assert!(output.contains("P1"));
    }

    #[test]
    fn rejects_wrong_quad_vertex_count() {
        let quad = Mesh {
            vertices: vec![
                Vec3::zero(),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            faces: vec![vec![0, 1, 2]],
        };

        let axes = axes_mesh();
        let metadata = axes_metadata();

        let mut canvas = Canvas::new(80, 28);

        let projector = ObliqueProjector::new(Point2::new(34, 14));

        assert!(render(&mut canvas, &projector, &quad, &axes, &metadata,).is_err());
    }
}
