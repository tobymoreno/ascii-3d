#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreatCircle {
    EquatorY0,
    MeridianX0,
    MeridianZ0,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphereGuidePoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl SphereGuidePoint {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

pub fn great_circle_points(circle: GreatCircle, steps: usize) -> Vec<SphereGuidePoint> {
    let steps = steps.max(3);
    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();

        let point = match circle {
            GreatCircle::EquatorY0 => SphereGuidePoint::new(c, 0.0, s),
            GreatCircle::MeridianX0 => SphereGuidePoint::new(0.0, c, s),
            GreatCircle::MeridianZ0 => SphereGuidePoint::new(c, s, 0.0),
        };

        points.push(point);
    }

    points
}

pub fn latitude_circle_points(latitude_degrees: f32, steps: usize) -> Vec<SphereGuidePoint> {
    let steps = steps.max(3);
    let latitude = latitude_degrees.to_radians();
    let y = latitude.sin();
    let radius = latitude.cos();

    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();

        points.push(SphereGuidePoint::new(radius * c, y, radius * s));
    }

    points
}
