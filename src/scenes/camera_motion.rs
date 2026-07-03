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
const TARGET_VERTICAL_RANGE: f32 = 1.5;
const DOLLY_DISTANCE: f32 = 2.5;

#[derive(Debug, Clone, Copy)]
enum MotionPhase {
    TiltUp,
    TiltDown,
    DollyBackward,
    DollyForward,
}

impl MotionPhase {
    const fn name(self) -> &'static str {
        match self {
            Self::TiltUp => "tilt up",
            Self::TiltDown => "tilt down",
            Self::DollyBackward => "dolly backward",
            Self::DollyForward => "dolly forward",
        }
    }
}

fn camera_pose(cycle_degrees: f32) -> (Vec3, Vec3, MotionPhase, f32) {
    let normalized = cycle_degrees.rem_euclid(360.0) / 360.0;

    let phase_position = normalized * 4.0;
    let phase_index = phase_position.floor() as usize;
    let phase_progress = phase_position.fract();

    match phase_index {
        0 => {
            let target = BASE_TARGET + Vec3::new(0.0, TARGET_VERTICAL_RANGE * phase_progress, 0.0);

            (BASE_EYE, target, MotionPhase::TiltUp, phase_progress)
        }

        1 => {
            let target = BASE_TARGET
                + Vec3::new(
                    0.0,
                    TARGET_VERTICAL_RANGE * (1.0 - 2.0 * phase_progress),
                    0.0,
                );

            (BASE_EYE, target, MotionPhase::TiltDown, phase_progress)
        }

        2 => {
            let backward = (BASE_EYE - BASE_TARGET).normalized();

            let eye = BASE_EYE + backward * (DOLLY_DISTANCE * phase_progress);

            (eye, BASE_TARGET, MotionPhase::DollyBackward, phase_progress)
        }

        _ => {
            let backward = (BASE_EYE - BASE_TARGET).normalized();

            let eye = BASE_EYE + backward * (DOLLY_DISTANCE * (1.0 - phase_progress));

            (eye, BASE_TARGET, MotionPhase::DollyForward, phase_progress)
        }
    }
}

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    cycle_degrees: f32,
) -> io::Result<()> {
    draw_axes(canvas, projector, true);

    let (eye, target, phase, phase_progress) = camera_pose(cycle_degrees);

    let forward = (target - eye).normalized();
    let right = forward.cross(WORLD_UP).normalized();
    let true_up = right.cross(forward).normalized();

    let forward_end = eye + forward * BASIS_LENGTH;

    let right_end = eye + right * BASIS_LENGTH;

    let up_end = eye + true_up * BASIS_LENGTH;

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

    let view = Mat4::look_at(eye, target, WORLD_UP)
        .ok_or_else(|| io::Error::other("invalid animated look_at camera configuration"))?;

    let target_in_view = view.transform_point(target);

    canvas.draw_text(
        Point2::new(2, 1),
        &format!("Scene: animated camera motion — {}", phase.name(),),
    );

    canvas.draw_text(
        Point2::new(2, 22),
        &format!(
            "cycle={:06.1} degrees  phase={:.0}%",
            cycle_degrees.rem_euclid(360.0),
            phase_progress * 100.0,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 23),
        &format!("eye    = ({:.2}, {:.2}, {:.2})", eye.x, eye.y, eye.z,),
    );

    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "target = ({:.2}, {:.2}, {:.2})",
            target.x, target.y, target.z,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "forward= ({:.2}, {:.2}, {:.2})",
            forward.x, forward.y, forward.z,
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
    use super::{MotionPhase, camera_pose};

    #[test]
    fn cycle_starts_with_tilt_up() {
        let (_, _, phase, progress) = camera_pose(0.0);

        assert!(matches!(phase, MotionPhase::TiltUp));
        assert!(progress.abs() <= f32::EPSILON);
    }

    #[test]
    fn third_quarter_is_dolly_backward() {
        let (_, _, phase, _) = camera_pose(180.0);

        assert!(matches!(phase, MotionPhase::DollyBackward));
    }

    #[test]
    fn full_cycle_wraps_to_start() {
        let (eye_a, target_a, _, _) = camera_pose(0.0);

        let (eye_b, target_b, _, _) = camera_pose(360.0);

        assert_eq!(eye_a, eye_b);
        assert_eq!(target_a, target_b);
    }
}
