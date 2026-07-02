use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalized(self) -> Self {
        let length = self.length();

        if length <= f32::EPSILON {
            Self::zero()
        } else {
            self / length
        }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn rotate_x(self, angle_radians: f32) -> Self {
        let cosine = angle_radians.cos();
        let sine = angle_radians.sin();

        Self::new(
            self.x,
            self.y * cosine - self.z * sine,
            self.y * sine + self.z * cosine,
        )
    }

    pub fn rotate_y(self, angle_radians: f32) -> Self {
        let cosine = angle_radians.cos();
        let sine = angle_radians.sin();

        Self::new(
            self.x * cosine + self.z * sine,
            self.y,
            -self.x * sine + self.z * cosine,
        )
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self::Output {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, scalar: f32) -> Self::Output {
        Self::new(self.x / scalar, self.y / scalar, self.z / scalar)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::Vec3;

    const EPSILON: f32 = 0.0001;

    fn approximately_equal(left: f32, right: f32) -> bool {
        (left - right).abs() < EPSILON
    }

    #[test]
    fn calculates_length() {
        let vector = Vec3::new(3.0, 4.0, 0.0);

        assert!(approximately_equal(vector.length(), 5.0));
    }

    #[test]
    fn normalizes_vector() {
        let vector = Vec3::new(3.0, 4.0, 0.0).normalized();

        assert!(approximately_equal(vector.length(), 1.0));
        assert!(approximately_equal(vector.x, 0.6));
        assert!(approximately_equal(vector.y, 0.8));
    }

    #[test]
    fn calculates_dot_product() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);

        assert!(approximately_equal(x.dot(y), 0.0));
    }

    #[test]
    fn calculates_cross_product() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(x.cross(y), Vec3::new(0.0, 0.0, 1.0));
    }
}
