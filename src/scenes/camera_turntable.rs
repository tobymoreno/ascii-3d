use std::io;

use crate::{
    canvas::Canvas,
    geometry2d::Point2,
    math::{Mat4, Vec3},
    projection::ObliqueProjector,
};

use super::draw_axes;

const BASE_EYE: Vec3 = Vec3 {
    x: -2.5,
    y: 2.0,
    z: 3.5,
};

const BASE_TARGET: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
};

const WORLD_UP: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};

const BASIS_LENGTH: f32 = 1.5;

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    angle_degrees: f32,
) -> io::Result<()> {
    draw_axes(canvas, projector, true);

    let eye = BASE_EYE;
    let target = BASE_TARGET;
    let approximate_up = WORLD_UP;

    let forward = (target - eye).normalized();
    let right = forward.cross(approximate_up).normalized();
    let true_up = right.cross(forward).normalized();

    let forward_end = eye + forward * BASIS_LENGTH;
    let right_end = eye + right * BASIS_LENGTH;
    let up_end = eye + true_up * BASIS_LENGTH;

    let display_rotation = Mat4::rotation_y(angle_degrees.to_radians());

    let displayed_eye = display_rotation.transform_point(eye);

    let displayed_target = display_rotation.transform_point(target);

    let displayed_forward_end = display_rotation.transform_point(forward_end);

    let displayed_right_end = display_rotation.transform_point(right_end);

    let displayed_up_end = display_rotation.transform_point(up_end);

    let eye_2d = projector.project(displayed_eye);
    let target_2d = projector.project(displayed_target);
    let forward_2d = projector.project(displayed_forward_end);
    let right_2d = projector.project(displayed_right_end);
    let up_2d = projector.project(displayed_up_end);

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
        .ok_or_else(|| io::Error::other("invalid fixed look_at camera configuration"))?;

    let eye_in_view = view.transform_point(eye);
    let target_in_view = view.transform_point(target);

    canvas.draw_text(
        Point2::new(2, 1),
        &format!(
            "Scene: camera Y-turntable inspection  angle={:06.1}",
            angle_degrees.rem_euclid(360.0),
        ),
    );

    canvas.draw_text(
        Point2::new(2, 22),
        "Display rotates around world Y; camera math stays fixed.",
    );

    canvas.draw_text(
        Point2::new(2, 23),
        &format!("eye    = ({:.2}, {:.2}, {:.2})", eye.x, eye.y, eye.z,),
    );

    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "forward= ({:.2}, {:.2}, {:.2})",
            forward.x, forward.y, forward.z,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "view(eye) = ({:.2}, {:.2}, {:.2})",
            eye_in_view.x, eye_in_view.y, eye_in_view.z,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 26),
        &format!(
            "view(target) = ({:.2}, {:.2}, {:.2})",
            target_in_view.x, target_in_view.y, target_in_view.z,
        ),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BASE_EYE, BASE_TARGET, WORLD_UP};
    use crate::math::{Mat4, Vec3};

    const EPSILON: f32 = 0.000_01;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual.x - expected.x).abs() <= EPSILON,
            "x: actual={} expected={}",
            actual.x,
            expected.x,
        );

        assert!(
            (actual.y - expected.y).abs() <= EPSILON,
            "y: actual={} expected={}",
            actual.y,
            expected.y,
        );

        assert!(
            (actual.z - expected.z).abs() <= EPSILON,
            "z: actual={} expected={}",
            actual.z,
            expected.z,
        );
    }

    #[test]
    fn zero_degree_turntable_leaves_eye_unchanged() {
        let rotated = Mat4::rotation_y(0.0).transform_point(BASE_EYE);

        assert_vec3_close(rotated, BASE_EYE);
    }

    #[test]
    fn full_turntable_rotation_wraps_to_start() {
        let rotated = Mat4::rotation_y(360.0_f32.to_radians()).transform_point(BASE_EYE);

        assert_vec3_close(rotated, BASE_EYE);
    }

    #[test]
    fn look_at_target_stays_on_negative_z() {
        let view =
            Mat4::look_at(BASE_EYE, BASE_TARGET, WORLD_UP).expect("fixed camera must be valid");

        let target_in_view = view.transform_point(BASE_TARGET);

        assert!(target_in_view.x.abs() <= EPSILON);
        assert!(target_in_view.y.abs() <= EPSILON);
        assert!(target_in_view.z < 0.0);
    }
}
