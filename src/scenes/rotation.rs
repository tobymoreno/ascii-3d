use crate::{canvas::Canvas, geometry2d::Point2, math::Vec3, projection::ObliqueProjector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationAxis {
    X,
    Y,
    Z,
}
impl RotationAxis {
    pub const fn name(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }
    fn rotate(self, vector: Vec3, angle_radians: f32) -> Vec3 {
        match self {
            Self::X => vector.rotate_x(angle_radians),
            Self::Y => vector.rotate_y(angle_radians),
            Self::Z => vector.rotate_z(angle_radians),
        }
    }
}

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    axis: RotationAxis,
    angle_degrees: f32,
) {
    let origin = Vec3::zero();
    let base_x = Vec3::new(4.0, 0.0, 0.0);
    let base_y = Vec3::new(0.0, 3.0, 0.0);
    let base_z = Vec3::new(0.0, 0.0, 4.0);
    let angle_radians = angle_degrees.to_radians();
    let rotated_x = axis.rotate(base_x, angle_radians);
    let rotated_y = axis.rotate(base_y, angle_radians);
    let rotated_z = axis.rotate(base_z, angle_radians);
    let origin_2d = projector.project(origin);
    let x_2d = projector.project(rotated_x);
    let y_2d = projector.project(rotated_y);
    let z_2d = projector.project(rotated_z);
    canvas.draw_arrow_auto(origin_2d, x_2d, '>');
    canvas.draw_arrow_auto(origin_2d, y_2d, '^');
    canvas.draw_arrow_auto(origin_2d, z_2d, 'v');
    canvas.set(origin_2d, 'O');
    canvas.draw_text(Point2::new(x_2d.x + 2, x_2d.y), "+X");
    canvas.draw_text(Point2::new(y_2d.x + 2, y_2d.y), "+Y");
    canvas.draw_text(Point2::new(z_2d.x + 2, z_2d.y), "+Z");
    canvas.draw_text(
        Point2::new(2, 1),
        &format!(
            "Rotate Cartesian axes around {}: {:06.1} / 360.0 degrees",
            axis.name(),
            angle_degrees
        ),
    );
    canvas.draw_text(Point2::new(2, 24), "Origin O = (0, 0, 0)");
    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "Rotating around {} leaves the {} axis unchanged.",
            axis.name(),
            axis.name()
        ),
    );
    canvas.draw_text(Point2::new(2, 26), "The other two axes sweep around it.");
}
