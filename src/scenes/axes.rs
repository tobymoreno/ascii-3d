use crate::{canvas::Canvas, geometry2d::Point2, math::Vec3, projection::ObliqueProjector};

pub fn draw_axes(canvas: &mut Canvas, projector: &ObliqueProjector, include_negative_z: bool) {
    let origin = projector.project(Vec3::zero());

    let positive_x = projector.project(Vec3::new(4.0, 0.0, 0.0));

    let positive_y = projector.project(Vec3::new(0.0, 3.0, 0.0));

    let positive_z = projector.project(Vec3::new(0.0, 0.0, 4.0));

    canvas.draw_arrow_auto(origin, positive_x, '>');
    canvas.draw_arrow_auto(origin, positive_y, '^');
    canvas.draw_arrow_auto(origin, positive_z, 'v');

    canvas.draw_text(Point2::new(positive_x.x + 2, positive_x.y), "+X");

    canvas.draw_text(Point2::new(positive_y.x + 2, positive_y.y), "+Y");

    canvas.draw_text(Point2::new(positive_z.x + 2, positive_z.y), "+Z");

    if include_negative_z {
        let negative_z = projector.project(Vec3::new(0.0, 0.0, -4.0));

        canvas.draw_arrow_auto(origin, negative_z, '^');

        canvas.draw_text(Point2::new(negative_z.x - 4, negative_z.y), "-Z");
    }

    canvas.set(origin, 'O');
}

pub fn render(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, false);

    canvas.draw_text(Point2::new(2, 1), "Scene: 3D Cartesian axes");

    canvas.draw_text(Point2::new(2, 24), "Origin O = (0, 0, 0)");
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::{canvas::Canvas, geometry2d::Point2, projection::ObliqueProjector};

    #[test]
    fn renders_axis_scene_without_panicking() {
        let mut canvas = Canvas::new(80, 28);
        let projector = ObliqueProjector::new(Point2::new(34, 14));

        render(&mut canvas, &projector);

        let output = canvas.render();

        assert!(output.contains("Scene: 3D Cartesian axes"));
        assert!(output.contains("Origin O = (0, 0, 0)"));
        assert!(output.contains("+X"));
        assert!(output.contains("+Y"));
        assert!(output.contains("+Z"));
    }
}
