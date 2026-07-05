use std::io;

use crate::{
    axis_metadata::CartesianAxesMetadata, canvas::Canvas, geometry2d::Point2, math::Vec3,
    mesh::Mesh, mesh_renderer::MeshTransform, projection::ObliqueProjector,
};

use super::{RotationAxis, render_asset_axes};

const DISPLAY_SCALE: f32 = 3.0;

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    axes_mesh: &Mesh,
    axes_metadata: &CartesianAxesMetadata,
    rotation_axis: RotationAxis,
    angle_degrees: f32,
) -> io::Result<()> {
    let angle_radians = angle_degrees.to_radians();

    let mut transform = MeshTransform {
        scale: DISPLAY_SCALE,
        translation: Vec3::zero(),
        ..MeshTransform::default()
    };

    match rotation_axis {
        RotationAxis::X => {
            transform.rotation_x = angle_radians;
        }

        RotationAxis::Y => {
            transform.rotation_y = angle_radians;
        }

        RotationAxis::Z => {
            transform.rotation_z = angle_radians;
        }
    }

    render_asset_axes(canvas, projector, axes_mesh, axes_metadata, transform)?;

    canvas.draw_text(
        Point2::new(2, 1),
        &format!(
            "Scene: asset Cartesian axes rotating around {}",
            axis_name(rotation_axis),
        ),
    );

    canvas.draw_text(
        Point2::new(2, 24),
        &format!("Rotation axis: {}", axis_name(rotation_axis),),
    );

    canvas.draw_text(
        Point2::new(2, 25),
        &format!("Angle: {:06.1} degrees", angle_degrees.rem_euclid(360.0),),
    );

    canvas.draw_text(
        Point2::new(2, 26),
        "Loaded OBJ geometry and JSON labels share one transform.",
    );

    Ok(())
}

const fn axis_name(axis: RotationAxis) -> &'static str {
    match axis {
        RotationAxis::X => "X",
        RotationAxis::Y => "Y",
        RotationAxis::Z => "Z",
    }
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
        scenes::RotationAxis,
    };

    fn axes_mesh() -> Mesh {
        Mesh {
            vertices: vec![
                Vec3::zero(),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            faces: vec![vec![0, 1], vec![0, 2], vec![0, 3]],
        }
    }

    fn axis(id: &str, label: &str, label_position: [f32; 3]) -> AxisMetadata {
        AxisMetadata {
            id: id.to_string(),
            group_shaft: format!("{id}_axis"),
            group_arrow: format!("{id}_axis"),
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

    fn metadata() -> CartesianAxesMetadata {
        CartesianAxesMetadata {
            name: "cartesian_axes".to_string(),
            version: 1,
            units: "world".to_string(),
            geometry_asset: "models/cartesian_axes.obj".to_string(),
            origin: OriginMetadata {
                position: [0.0, 0.0, 0.0],
                label: String::new(),
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
    fn renders_rotation_around_x() {
        render_axis(RotationAxis::X);
    }

    #[test]
    fn renders_rotation_around_y() {
        render_axis(RotationAxis::Y);
    }

    #[test]
    fn renders_rotation_around_z() {
        render_axis(RotationAxis::Z);
    }

    fn render_axis(axis: RotationAxis) {
        let mesh = axes_mesh();
        let metadata = metadata();
        let mut canvas = Canvas::new(80, 28);

        let projector = ObliqueProjector::new(Point2::new(34, 14));

        render(&mut canvas, &projector, &mesh, &metadata, axis, 45.0)
            .expect("asset-axis rotation should render");

        let output = canvas.render();

        assert!(output.contains("+X"));
        assert!(output.contains("+Z"));
    }
}
