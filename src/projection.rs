use crate::{geometry2d::Point2, math::Vec3};

pub struct ObliqueProjector {
    screen_origin: Point2,

    x_scale: f32,
    y_scale: f32,

    z_x_scale: f32,
    z_y_scale: f32,
}

impl ObliqueProjector {
    pub fn new(screen_origin: Point2) -> Self {
        Self {
            screen_origin,

            // Terminal characters are taller than they are wide,
            // so X uses more columns per world-space unit.
            x_scale: 8.0,
            y_scale: 3.0,

            // Positive Z moves down and right on the terminal.
            z_x_scale: 2.0,
            z_y_scale: 2.0,
        }
    }

    pub fn project(&self, point: Vec3) -> Point2 {
        let screen_x =
            self.screen_origin.x as f32 + point.x * self.x_scale + point.z * self.z_x_scale;

        let screen_y =
            self.screen_origin.y as f32 - point.y * self.y_scale + point.z * self.z_y_scale;

        Point2::new(screen_x.round() as i32, screen_y.round() as i32)
    }
}
