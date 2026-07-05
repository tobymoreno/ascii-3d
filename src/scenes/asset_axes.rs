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

fn vec3(values: [f32; 3]) -> Vec3 {
    Vec3::new(values[0], values[1], values[2])
}

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    mesh: &Mesh,
    metadata: &CartesianAxesMetadata,
    transform: MeshTransform,
) -> io::Result<()> {
    draw_wireframe(canvas, projector, mesh, transform).map_err(io::Error::other)?;

    if metadata.display.show_origin {
        let origin_world = transform.transform_vertex(vec3(metadata.origin.position));

        let origin_screen = projector.project(origin_world);

        canvas.set(origin_screen, 'O');

        canvas.draw_text(
            Point2::new(origin_screen.x + 2, origin_screen.y),
            &metadata.origin.label,
        );
    }

    for axis in &metadata.axes {
        if metadata.display.show_positive_labels {
            let label_world = transform.transform_vertex(vec3(axis.positive_label_position));

            let label_screen = projector.project(label_world);

            canvas.draw_text(label_screen, &axis.positive_label);
        }

        if metadata.display.show_negative_labels {
            let label_world = transform.transform_vertex(vec3(axis.negative_label_position));

            let label_screen = projector.project(label_world);

            canvas.draw_text(label_screen, &axis.negative_label);
        }
    }

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
        mesh_renderer::MeshTransform,
        projection::ObliqueProjector,
    };

    fn test_mesh() -> Mesh {
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

    fn axis(id: &str, positive_label: &str, positive_label_position: [f32; 3]) -> AxisMetadata {
        AxisMetadata {
            id: id.to_string(),
            group_shaft: format!("{id}_axis_shaft"),
            group_arrow: format!("{id}_axis_arrow"),
            positive_direction: [0.0, 0.0, 0.0],
            negative_direction: [0.0, 0.0, 0.0],
            length: 1.0,
            positive_endpoint: positive_label_position,
            positive_label: positive_label.to_string(),
            negative_label: format!("-{}", id.to_uppercase()),
            positive_label_position,
            negative_label_position: [0.0, 0.0, 0.0],
        }
    }

    fn test_metadata() -> CartesianAxesMetadata {
        CartesianAxesMetadata {
            name: "cartesian_axes".to_string(),
            version: 1,
            units: "world".to_string(),
            geometry_asset: "models/cartesian_axes.obj".to_string(),
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
    fn renders_asset_geometry_and_metadata_labels() {
        let mesh = test_mesh();
        let metadata = test_metadata();
        let mut canvas = Canvas::new(80, 28);

        let projector = ObliqueProjector::new(Point2::new(34, 14));

        let transform = MeshTransform {
            rotation_x: 0.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
            scale: 3.0,
            translation: Vec3::zero(),
        };

        render(&mut canvas, &projector, &mesh, &metadata, transform)
            .expect("asset-driven axes should render");

        let output = canvas.render();

        assert!(output.contains("+X"));
        assert!(output.contains("+Y"));
        assert!(output.contains("+Z"));
    }
}
