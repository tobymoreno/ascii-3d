use std::ops::Mul;

use super::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub const fn new(m: [[f32; 4]; 4]) -> Self {
        Self { m }
    }

    pub const fn identity() -> Self {
        Self::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub const fn translation(x: f32, y: f32, z: f32) -> Self {
        Self::new([
            [1.0, 0.0, 0.0, x],
            [0.0, 1.0, 0.0, y],
            [0.0, 0.0, 1.0, z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub const fn translation_vec3(translation: Vec3) -> Self {
        Self::translation(translation.x, translation.y, translation.z)
    }

    pub const fn scale(x: f32, y: f32, z: f32) -> Self {
        Self::new([
            [x, 0.0, 0.0, 0.0],
            [0.0, y, 0.0, 0.0],
            [0.0, 0.0, z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub const fn uniform_scale(scale: f32) -> Self {
        Self::scale(scale, scale, scale)
    }

    pub fn rotation_x(angle_radians: f32) -> Self {
        let cosine = angle_radians.cos();
        let sine = angle_radians.sin();

        Self::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, cosine, -sine, 0.0],
            [0.0, sine, cosine, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_y(angle_radians: f32) -> Self {
        let cosine = angle_radians.cos();
        let sine = angle_radians.sin();

        Self::new([
            [cosine, 0.0, sine, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-sine, 0.0, cosine, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_z(angle_radians: f32) -> Self {
        let cosine = angle_radians.cos();
        let sine = angle_radians.sin();

        Self::new([
            [cosine, -sine, 0.0, 0.0],
            [sine, cosine, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn transform_point(self, point: Vec3) -> Vec3 {
        let x =
            self.m[0][0] * point.x + self.m[0][1] * point.y + self.m[0][2] * point.z + self.m[0][3];

        let y =
            self.m[1][0] * point.x + self.m[1][1] * point.y + self.m[1][2] * point.z + self.m[1][3];

        let z =
            self.m[2][0] * point.x + self.m[2][1] * point.y + self.m[2][2] * point.z + self.m[2][3];

        let w =
            self.m[3][0] * point.x + self.m[3][1] * point.y + self.m[3][2] * point.z + self.m[3][3];

        if w.abs() > f32::EPSILON && (w - 1.0).abs() > f32::EPSILON {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }

    pub fn transform_vector(self, vector: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * vector.x + self.m[0][1] * vector.y + self.m[0][2] * vector.z,
            self.m[1][0] * vector.x + self.m[1][1] * vector.y + self.m[1][2] * vector.z,
            self.m[2][0] * vector.x + self.m[2][1] * vector.y + self.m[2][2] * vector.z,
        )
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = [[0.0; 4]; 4];

        for (row, result_row) in result.iter_mut().enumerate() {
            for (column, result_cell) in result_row.iter_mut().enumerate() {
                *result_cell = self.m[row][0] * rhs.m[0][column]
                    + self.m[row][1] * rhs.m[1][column]
                    + self.m[row][2] * rhs.m[2][column]
                    + self.m[row][3] * rhs.m[3][column];
            }
        }

        Self::new(result)
    }
}

#[cfg(test)]
mod tests {
    use super::Mat4;
    use crate::math::Vec3;

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
    fn identity_leaves_point_unchanged() {
        let point = Vec3::new(1.0, 2.0, 3.0);

        assert_vec3_close(Mat4::identity().transform_point(point), point);
    }

    #[test]
    fn translation_moves_points() {
        let matrix = Mat4::translation(10.0, 20.0, 30.0);

        assert_vec3_close(
            matrix.transform_point(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(11.0, 22.0, 33.0),
        );
    }

    #[test]
    fn translation_does_not_move_direction_vectors() {
        let matrix = Mat4::translation(10.0, 20.0, 30.0);

        assert_vec3_close(
            matrix.transform_vector(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, 3.0),
        );
    }

    #[test]
    fn uniform_scale_scales_point() {
        let matrix = Mat4::uniform_scale(2.0);

        assert_vec3_close(
            matrix.transform_point(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(2.0, 4.0, 6.0),
        );
    }

    #[test]
    fn rotation_x_matches_legacy_vec3_rotation() {
        let vector = Vec3::new(2.0, 1.0, 3.0);
        let angle = 45.0_f32.to_radians();

        assert_vec3_close(
            Mat4::rotation_x(angle).transform_vector(vector),
            vector.rotate_x(angle),
        );
    }

    #[test]
    fn rotation_y_matches_legacy_vec3_rotation() {
        let vector = Vec3::new(2.0, 1.0, 3.0);
        let angle = 45.0_f32.to_radians();

        assert_vec3_close(
            Mat4::rotation_y(angle).transform_vector(vector),
            vector.rotate_y(angle),
        );
    }

    #[test]
    fn rotation_z_matches_legacy_vec3_rotation() {
        let vector = Vec3::new(2.0, 1.0, 3.0);
        let angle = 45.0_f32.to_radians();

        assert_vec3_close(
            Mat4::rotation_z(angle).transform_vector(vector),
            vector.rotate_z(angle),
        );
    }

    #[test]
    fn matrix_order_is_scale_then_rotate_then_translate() {
        let scale = Mat4::uniform_scale(2.0);
        let rotation = Mat4::rotation_z(90.0_f32.to_radians());
        let translation = Mat4::translation(10.0, 0.0, 0.0);

        let model = translation * rotation * scale;

        let transformed = model.transform_point(Vec3::new(1.0, 0.0, 0.0));

        assert_vec3_close(transformed, Vec3::new(10.0, 2.0, 0.0));
    }
}
