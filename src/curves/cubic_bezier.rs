use crate::math::Vec3;

/// A sampled 3D curve represented as ordered points.
///
/// This is the bridge between a mathematical curve and renderers that
/// ultimately need discrete points, lines, pixels, or terminal cells.
#[derive(Debug, Clone, PartialEq)]
pub struct SampledCurve3 {
    pub points: Vec<Vec3>,
}

impl SampledCurve3 {
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns line segments connecting adjacent sampled points.
    pub fn line_segments(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        self.points.windows(2).map(|pair| (pair[0], pair[1]))
    }
}

/// A cubic Bézier curve in 3D.
///
/// B(t) = (1-t)^3 p0
///      + 3(1-t)^2 t p1
///      + 3(1-t)t^2 p2
///      + t^3 p3
///
/// where t is in [0, 1].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier3 {
    pub p0: Vec3,
    pub p1: Vec3,
    pub p2: Vec3,
    pub p3: Vec3,
}

impl CubicBezier3 {
    pub const fn new(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Self {
        Self { p0, p1, p2, p3 }
    }

    pub fn point_at(self, t: f32) -> Vec3 {
        let t = t.clamp(0.0, 1.0);
        let one_minus_t = 1.0 - t;

        let a = one_minus_t * one_minus_t * one_minus_t;
        let b = 3.0 * one_minus_t * one_minus_t * t;
        let c = 3.0 * one_minus_t * t * t;
        let d = t * t * t;

        self.p0 * a + self.p1 * b + self.p2 * c + self.p3 * d
    }

    pub fn tangent_at(self, t: f32) -> Vec3 {
        let t = t.clamp(0.0, 1.0);
        let one_minus_t = 1.0 - t;

        let a = 3.0 * one_minus_t * one_minus_t;
        let b = 6.0 * one_minus_t * t;
        let c = 3.0 * t * t;

        (self.p1 - self.p0) * a + (self.p2 - self.p1) * b + (self.p3 - self.p2) * c
    }

    pub fn direction_at(self, t: f32) -> Option<Vec3> {
        let tangent = self.tangent_at(t);

        if tangent.length_squared() <= f32::EPSILON {
            None
        } else {
            Some(tangent.normalized())
        }
    }

    pub fn sample(self, segments: usize) -> SampledCurve3 {
        let segments = segments.max(1);
        let mut points = Vec::with_capacity(segments + 1);

        for index in 0..=segments {
            let t = index as f32 / segments as f32;
            points.push(self.point_at(t));
        }

        SampledCurve3 { points }
    }
}

#[cfg(test)]
mod tests {
    use super::CubicBezier3;
    use crate::math::Vec3;

    const EPSILON: f32 = 0.000_01;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual.x - expected.x).abs() <= EPSILON,
            "x actual={} expected={}",
            actual.x,
            expected.x,
        );

        assert!(
            (actual.y - expected.y).abs() <= EPSILON,
            "y actual={} expected={}",
            actual.y,
            expected.y,
        );

        assert!(
            (actual.z - expected.z).abs() <= EPSILON,
            "z actual={} expected={}",
            actual.z,
            expected.z,
        );
    }

    fn curve() -> CubicBezier3 {
        CubicBezier3::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(3.0, 2.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
        )
    }

    #[test]
    fn point_at_zero_returns_start_point() {
        let curve = curve();

        assert_vec3_close(curve.point_at(0.0), curve.p0);
    }

    #[test]
    fn point_at_one_returns_end_point() {
        let curve = curve();

        assert_vec3_close(curve.point_at(1.0), curve.p3);
    }

    #[test]
    fn point_at_half_matches_cubic_bezier_equation() {
        let curve = curve();

        assert_vec3_close(curve.point_at(0.5), Vec3::new(2.0, 1.5, 0.0));
    }

    #[test]
    fn tangent_at_start_points_toward_first_handle() {
        let curve = curve();

        assert_vec3_close(curve.tangent_at(0.0), Vec3::new(3.0, 6.0, 0.0));
    }

    #[test]
    fn tangent_at_end_points_from_second_handle_to_end() {
        let curve = curve();

        assert_vec3_close(curve.tangent_at(1.0), Vec3::new(3.0, -6.0, 0.0));
    }

    #[test]
    fn direction_at_returns_normalized_tangent() {
        let curve = CubicBezier3::new(
            Vec3::zero(),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
        );

        assert_vec3_close(
            curve.direction_at(0.5).expect("direction should exist"),
            Vec3::new(1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn direction_at_returns_none_for_degenerate_curve() {
        let curve = CubicBezier3::new(Vec3::zero(), Vec3::zero(), Vec3::zero(), Vec3::zero());

        assert_eq!(curve.direction_at(0.5), None);
    }

    #[test]
    fn sample_returns_segments_plus_one_points() {
        let sampled = curve().sample(8);

        assert_eq!(sampled.len(), 9);
    }

    #[test]
    fn sample_connects_adjacent_points_as_segments() {
        let sampled = curve().sample(4);
        let segments: Vec<_> = sampled.line_segments().collect();

        assert_eq!(segments.len(), 4);
        assert_vec3_close(segments[0].0, curve().p0);
        assert_vec3_close(segments[3].1, curve().p3);
    }
}
