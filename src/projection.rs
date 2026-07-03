use crate::{geometry2d::Point2, math::Vec3};

#[derive(Debug, Clone, Copy)]
pub struct ObliqueProjector {
    screen_origin: Point2,
    x_axis_vector: [f32; 2],
    y_axis_vector: [f32; 2],
    z_axis_vector: [f32; 2],
}

impl ObliqueProjector {
    /// Creates the default project-wide ASCII projection.
    ///
    /// Runtime scenes should normally load `assets/projection.default.json`
    /// and call `from_axis_vectors`. This constructor remains useful for
    /// unit tests and simple examples.
    pub fn new(screen_origin: Point2) -> Self {
        Self::from_axis_vectors(screen_origin, [8.0, 0.0], [0.0, -3.0], [2.0, -2.0])
    }

    /// Creates a projection from 2D terminal axis vectors.
    ///
    /// Each vector means "how many terminal cells one positive world-space
    /// unit contributes to the screen position."
    ///
    /// Terminal coordinates use:
    ///
    /// - +screen X = right/east
    /// - +screen Y = down/south
    ///
    /// Therefore an upward/north vector has a negative Y component.
    pub const fn from_axis_vectors(
        screen_origin: Point2,
        x_axis_vector: [f32; 2],
        y_axis_vector: [f32; 2],
        z_axis_vector: [f32; 2],
    ) -> Self {
        Self {
            screen_origin,
            x_axis_vector,
            y_axis_vector,
            z_axis_vector,
        }
    }

    pub fn project(&self, point: Vec3) -> Point2 {
        let screen_x = self.screen_origin.x as f32
            + point.x * self.x_axis_vector[0]
            + point.y * self.y_axis_vector[0]
            + point.z * self.z_axis_vector[0];

        let screen_y = self.screen_origin.y as f32
            + point.x * self.x_axis_vector[1]
            + point.y * self.y_axis_vector[1]
            + point.z * self.z_axis_vector[1];

        Point2::new(screen_x.round() as i32, screen_y.round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::ObliqueProjector;
    use crate::{geometry2d::Point2, math::Vec3};

    #[test]
    fn positive_x_projects_to_the_right() {
        let projector = ObliqueProjector::new(Point2::new(34, 14));

        let origin = projector.project(Vec3::zero());
        let positive_x = projector.project(Vec3::new(1.0, 0.0, 0.0));

        assert!(positive_x.x > origin.x);
        assert_eq!(positive_x.y, origin.y);
    }

    #[test]
    fn positive_y_projects_up() {
        let projector = ObliqueProjector::new(Point2::new(34, 14));

        let origin = projector.project(Vec3::zero());
        let positive_y = projector.project(Vec3::new(0.0, 1.0, 0.0));

        assert_eq!(positive_y.x, origin.x);
        assert!(positive_y.y < origin.y);
    }

    #[test]
    fn positive_z_projects_up_and_right() {
        let projector = ObliqueProjector::new(Point2::new(34, 14));

        let origin = projector.project(Vec3::zero());
        let positive_z = projector.project(Vec3::new(0.0, 0.0, 1.0));

        assert!(positive_z.x > origin.x);
        assert!(positive_z.y < origin.y);
    }

    #[test]
    fn custom_projection_vectors_are_used() {
        let projector = ObliqueProjector::from_axis_vectors(
            Point2::new(10, 10),
            [1.0, 0.0],
            [0.0, -1.0],
            [5.0, -7.0],
        );

        assert_eq!(
            projector.project(Vec3::new(0.0, 0.0, 1.0)),
            Point2::new(15, 3),
        );
    }
}
