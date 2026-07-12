#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn normalized(self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();

        if length == 0.0 {
            return Self::new(0.0, 1.0, 0.0);
        }

        Self::new(self.x / length, self.y / length, self.z / length)
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn from_array(value: [f32; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }

    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn translation(v: Vec3) -> Self {
        let mut out = Self::identity();
        out.m[0][3] = v.x;
        out.m[1][3] = v.y;
        out.m[2][3] = v.z;
        out
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_x(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();

        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, c, -s, 0.0],
                [0.0, s, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_y(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();

        Self {
            m: [
                [c, 0.0, s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [-s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_z(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();

        Self {
            m: [
                [c, -s, 0.0, 0.0],
                [s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn transform_point(self, point: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * point.x + self.m[0][1] * point.y + self.m[0][2] * point.z + self.m[0][3],
            self.m[1][0] * point.x + self.m[1][1] * point.y + self.m[1][2] * point.z + self.m[1][3],
            self.m[2][0] * point.x + self.m[2][1] * point.y + self.m[2][2] * point.z + self.m[2][3],
        )
    }

    pub fn transform_vector(self, vector: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * vector.x + self.m[0][1] * vector.y + self.m[0][2] * vector.z,
            self.m[1][0] * vector.x + self.m[1][1] * vector.y + self.m[1][2] * vector.z,
            self.m[2][0] * vector.x + self.m[2][1] * vector.y + self.m[2][2] * vector.z,
        )
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Mat4;

    fn mul(self, other: Mat4) -> Self::Output {
        let mut out = Mat4 { m: [[0.0; 4]; 4] };

        for row in 0..4 {
            for col in 0..4 {
                out.m[row][col] = self.m[row][0] * other.m[0][col]
                    + self.m[row][1] * other.m[1][col]
                    + self.m[row][2] * other.m[2][col]
                    + self.m[row][3] * other.m[3][col];
            }
        }

        out
    }
}
