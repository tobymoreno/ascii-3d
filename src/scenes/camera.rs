use super::draw_axes;
use crate::{
    canvas::Canvas,
    geometry2d::Point2,
    math::{Mat4, Vec3},
    projection::ObliqueProjector,
};
use std::io;

pub fn render(canvas: &mut Canvas, projector: &ObliqueProjector) -> io::Result<()> {
    draw_axes(canvas, projector, true);
    let eye = Vec3::new(-2.5, 2.0, 3.5);
    let target = Vec3::zero();
    let approximate_up = Vec3::new(0.0, 1.0, 0.0);
    let forward = (target - eye).normalized();
    let right = forward.cross(approximate_up).normalized();
    let true_up = right.cross(forward).normalized();
    let basis_length = 1.5;
    let forward_end = eye + forward * basis_length;
    let right_end = eye + right * basis_length;
    let up_end = eye + true_up * basis_length;
    let eye_2d = projector.project(eye);
    let target_2d = projector.project(target);
    let forward_2d = projector.project(forward_end);
    let right_2d = projector.project(right_end);
    let up_2d = projector.project(up_end);
    canvas.draw_line_auto(eye_2d, target_2d);
    canvas.draw_arrow_auto(eye_2d, forward_2d, 'F');
    canvas.draw_arrow_auto(eye_2d, right_2d, 'R');
    canvas.draw_arrow_auto(eye_2d, up_2d, 'U');
    canvas.set(eye_2d, 'E');
    canvas.set(target_2d, 'T');
    canvas.draw_text(Point2::new(eye_2d.x + 2, eye_2d.y), "eye");
    canvas.draw_text(Point2::new(target_2d.x + 2, target_2d.y), "target");
    canvas.draw_text(Point2::new(forward_2d.x + 2, forward_2d.y), "forward");
    canvas.draw_text(Point2::new(right_2d.x + 2, right_2d.y), "right");
    canvas.draw_text(Point2::new(up_2d.x + 2, up_2d.y), "true up");
    let view = Mat4::look_at(eye, target, approximate_up)
        .ok_or_else(|| io::Error::other("invalid look_at camera configuration"))?;
    let eye_in_view = view.transform_point(eye);
    let target_in_view = view.transform_point(target);
    canvas.draw_text(Point2::new(2, 1), "Scene: look_at camera basis");
    canvas.draw_text(
        Point2::new(2, 23),
        &format!("eye    = ({:.2}, {:.2}, {:.2})", eye.x, eye.y, eye.z),
    );
    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "forward= ({:.2}, {:.2}, {:.2})",
            forward.x, forward.y, forward.z
        ),
    );
    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "view(eye) = ({:.2}, {:.2}, {:.2})",
            eye_in_view.x, eye_in_view.y, eye_in_view.z
        ),
    );
    canvas.draw_text(
        Point2::new(2, 26),
        &format!(
            "view(target) = ({:.2}, {:.2}, {:.2})",
            target_in_view.x, target_in_view.y, target_in_view.z
        ),
    );
    Ok(())
}
