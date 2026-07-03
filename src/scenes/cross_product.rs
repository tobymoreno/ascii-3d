use super::draw_axes;
use crate::{canvas::Canvas, geometry2d::Point2, math::Vec3, projection::ObliqueProjector};

pub fn render_positive_z(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, true);
    let origin = Vec3::zero();
    let vector_a = Vec3::new(3.0, 1.0, 0.0);
    let vector_b = Vec3::new(1.0, 2.0, 0.0);
    let cross = vector_a.cross(vector_b);
    let displayed_cross = cross.normalized() * 3.0;
    let origin_2d = projector.project(origin);
    let vector_a_2d = projector.project(vector_a);
    let vector_b_2d = projector.project(vector_b);
    let cross_2d = projector.project(displayed_cross);
    canvas.draw_arrow_auto(origin_2d, vector_a_2d, 'A');
    canvas.draw_arrow_auto(origin_2d, vector_b_2d, 'B');
    canvas.draw_arrow_auto(origin_2d, cross_2d, 'N');
    canvas.draw_text(Point2::new(vector_a_2d.x + 2, vector_a_2d.y), "A");
    canvas.draw_text(Point2::new(vector_b_2d.x + 2, vector_b_2d.y), "B");
    canvas.draw_text(Point2::new(cross_2d.x + 2, cross_2d.y), "A x B");
    canvas.draw_text(Point2::new(2, 1), "Scene: A x B points along +Z");
    canvas.draw_text(
        Point2::new(2, 24),
        &format!("A x B = ({:.1}, {:.1}, {:.1})", cross.x, cross.y, cross.z),
    );
    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "(A x B) dot A = {:.1}    (A x B) dot B = {:.1}",
            cross.dot(vector_a),
            cross.dot(vector_b)
        ),
    );
}

pub fn render_negative_z(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, true);
    let origin = Vec3::zero();
    let vector_a = Vec3::new(3.0, 1.0, 0.0);
    let vector_b = Vec3::new(1.0, 2.0, 0.0);
    let cross = vector_b.cross(vector_a);
    let displayed_cross = cross.normalized() * 3.0;
    let origin_2d = projector.project(origin);
    let vector_a_2d = projector.project(vector_a);
    let vector_b_2d = projector.project(vector_b);
    let cross_2d = projector.project(displayed_cross);
    canvas.draw_arrow_auto(origin_2d, vector_a_2d, 'A');
    canvas.draw_arrow_auto(origin_2d, vector_b_2d, 'B');
    canvas.draw_arrow_auto(origin_2d, cross_2d, 'N');
    canvas.draw_text(Point2::new(vector_a_2d.x + 2, vector_a_2d.y), "A");
    canvas.draw_text(Point2::new(vector_b_2d.x + 2, vector_b_2d.y), "B");
    canvas.draw_text(Point2::new(cross_2d.x - 7, cross_2d.y), "B x A");
    canvas.draw_text(Point2::new(2, 1), "Scene: B x A points along -Z");
    canvas.draw_text(
        Point2::new(2, 24),
        &format!("B x A = ({:.1}, {:.1}, {:.1})", cross.x, cross.y, cross.z),
    );
    canvas.draw_text(
        Point2::new(2, 25),
        "Changing operand order reverses the cross product.",
    );
}
