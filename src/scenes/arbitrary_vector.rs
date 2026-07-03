use super::draw_axes;
use crate::{canvas::Canvas, geometry2d::Point2, math::Vec3, projection::ObliqueProjector};

pub fn render(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, false);
    let origin = Vec3::zero();
    let vector = Vec3::new(2.0, 1.0, 3.0);
    let origin_2d = projector.project(origin);
    let vector_2d = projector.project(vector);
    canvas.draw_arrow_auto(origin_2d, vector_2d, '*');
    canvas.draw_text(Point2::new(vector_2d.x + 2, vector_2d.y), "V(2,1,3)");
    let normalized = vector.normalized();
    canvas.draw_text(Point2::new(2, 1), "Scene: arbitrary Vec3");
    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "length={:.3} normalized=({:.3}, {:.3}, {:.3})",
            vector.length(),
            normalized.x,
            normalized.y,
            normalized.z
        ),
    );
}
