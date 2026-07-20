use crate::math::Vec3;

const CLOUD_SHELL_RADIUS: f32 = 1.018;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CloudSettings {
    pub(crate) show: bool,
    pub(crate) opacity: f32,
    pub(crate) coverage: f32,
    pub(crate) seed: u32,
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self {
            show: true,
            opacity: 0.50,
            coverage: 0.82,
            seed: 7,
        }
    }
}

impl CloudSettings {
    pub(crate) fn clamped(self) -> Self {
        Self {
            show: self.show,
            opacity: self.opacity.clamp(0.0, 1.0),
            coverage: self.coverage.clamp(0.0, 1.0),
            seed: self.seed,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CloudPuff {
    pub(crate) center: Vec3,
    pub(crate) tangent: Vec3,
    pub(crate) bitangent: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) size: [f32; 2],
    pub(crate) opacity: f32,
    pub(crate) seed: f32,
}

pub(crate) fn generate_cloud_puffs(settings: CloudSettings) -> Vec<CloudPuff> {
    let settings = settings.clamped();
    if !settings.show || settings.opacity <= 0.001 || settings.coverage <= 0.001 {
        return Vec::new();
    }

    let mut rng = Rng::new(0x8f3d_d1b4_7a9c_25e1_u64 ^ settings.seed as u64);
    let mut puffs = Vec::new();

    let wispy_sheet_count = (9.0 + settings.coverage * 8.0).round() as usize;
    let flow_band_count = (12.0 + settings.coverage * 12.0).round() as usize;
    let linked_system_count = (14.0 + settings.coverage * 12.0).round() as usize;
    let small_cluster_count = (28.0 + settings.coverage * 18.0).round() as usize;
    let spiral_sample_count = (24.0 + settings.coverage * 24.0).round() as usize;

    for _ in 0..wispy_sheet_count {
        append_wispy_sheet(&mut puffs, &mut rng, settings);
    }
    for _ in 0..flow_band_count {
        append_flow_band(&mut puffs, &mut rng, settings);
    }
    for _ in 0..linked_system_count {
        append_linked_system(&mut puffs, &mut rng, settings);
    }
    for _ in 0..small_cluster_count {
        append_small_cluster(&mut puffs, &mut rng, settings);
    }
    append_spiral_system(&mut puffs, &mut rng, settings, spiral_sample_count.max(14));

    puffs
}

fn append_wispy_sheet(puffs: &mut Vec<CloudPuff>, rng: &mut Rng, settings: CloudSettings) {
    let center = random_cloud_direction(rng);
    let wind = prevailing_wind_tangent(center, rng);
    let cross = center.cross(wind).normalized();
    let count = rng.range_usize(6, 10);
    let spacing = rng.range(0.055, 0.095);
    let sweep = (count.saturating_sub(1)) as f32 * 0.5;
    let curvature = rng.range(-0.030, 0.030);

    for index in 0..count {
        let t = index as f32 - sweep;
        let along = t * spacing + rng.range(-0.018, 0.018);
        let across = curvature * t * t + rng.range(-0.030, 0.030);
        let normal = (center + wind * along + cross * across).normalized();
        let size = [
            rng.range(0.15, 0.26) * (0.92 + settings.coverage * 0.48),
            rng.range(0.040, 0.090) * (0.92 + settings.coverage * 0.24),
        ];
        puffs.push(make_puff_aligned(
            normal,
            wind,
            size,
            rng.range(0.12, 0.21),
            rng.f32(),
        ));
    }
}

fn append_flow_band(puffs: &mut Vec<CloudPuff>, rng: &mut Rng, settings: CloudSettings) {
    let center = random_cloud_direction(rng);
    let wind = prevailing_wind_tangent(center, rng);
    let cross = center.cross(wind).normalized();
    let count = rng.range_usize(9, 15);
    let spacing = rng.range(0.035, 0.060);
    let sweep = (count.saturating_sub(1)) as f32 * 0.5;
    let curvature = rng.range(-0.038, 0.038);
    let band_offset = rng.range(-0.050, 0.050);

    for index in 0..count {
        let t = index as f32 - sweep;
        let along = t * spacing + rng.range(-0.014, 0.014);
        let across = band_offset + curvature * t * t + rng.range(-0.018, 0.018);
        let normal = (center + wind * along + cross * across).normalized();
        let size = [
            rng.range(0.13, 0.24) * (0.96 + settings.coverage * 0.38),
            rng.range(0.032, 0.070) * (0.96 + settings.coverage * 0.20),
        ];
        puffs.push(make_puff_aligned(
            normal,
            wind,
            size,
            rng.range(0.13, 0.23),
            rng.f32(),
        ));
    }
}

fn append_linked_system(puffs: &mut Vec<CloudPuff>, rng: &mut Rng, settings: CloudSettings) {
    let center = random_cloud_direction(rng);
    let mut normal = center;
    let mut wind = prevailing_wind_tangent(normal, rng);
    let step_count = rng.range_usize(6, 11);
    let step_size = rng.range(0.030, 0.050);

    for index in 0..step_count {
        let cross = normal.cross(wind).normalized();
        let drift = rng.range(-0.020, 0.020);
        let forward = step_size * (0.82 + rng.range(0.0, 0.28));
        normal = (normal + wind * forward + cross * drift).normalized();
        wind = prevailing_wind_tangent(normal, rng);

        let overlap_count = if index == 0 || index + 1 == step_count {
            2
        } else {
            3
        };
        for overlap in 0..overlap_count {
            let along_jitter = rng.range(-0.018, 0.018);
            let across_jitter = rng.range(-0.020, 0.020);
            let local = (normal + wind * along_jitter + cross * across_jitter).normalized();
            let size = [
                rng.range(0.12, 0.22) * (0.98 + settings.coverage * 0.34),
                rng.range(0.038, 0.082) * (0.98 + settings.coverage * 0.20),
            ];
            puffs.push(make_puff_aligned(
                local,
                wind,
                size,
                rng.range(0.15, 0.25),
                rng.f32(),
            ));
        }
    }
}

fn append_small_cluster(puffs: &mut Vec<CloudPuff>, rng: &mut Rng, settings: CloudSettings) {
    let center = random_cloud_direction(rng);
    let (east, north) = tangent_basis(center);
    let count = rng.range_usize(4, 7);
    let spread = rng.range(0.028, 0.070) * (0.88 + settings.coverage * 0.28);

    for _ in 0..count {
        let normal =
            (center + east * rng.range(-spread, spread) + north * rng.range(-spread, spread))
                .normalized();
        let wind = prevailing_wind_tangent(normal, rng);
        let size = [
            rng.range(0.060, 0.13) * (1.02 + settings.coverage * 0.34),
            rng.range(0.040, 0.10) * (1.00 + settings.coverage * 0.24),
        ];
        puffs.push(make_puff_aligned(
            normal,
            wind,
            size,
            rng.range(0.18, 0.32),
            rng.f32(),
        ));
    }
}

fn append_spiral_system(
    puffs: &mut Vec<CloudPuff>,
    rng: &mut Rng,
    settings: CloudSettings,
    sample_count: usize,
) {
    let center = random_cloud_direction(rng);
    let (east, north) = tangent_basis(center);
    let base_angle = rng.range(0.0, std::f32::consts::TAU);
    let turns = rng.range(1.15, 1.55);

    for index in 0..sample_count {
        let t = index as f32 / (sample_count.saturating_sub(1).max(1)) as f32;
        if rng.f32() < 0.18 {
            continue;
        }
        let angle = base_angle + t * turns * std::f32::consts::TAU;
        let radial = 0.035 + t * 0.19;
        let x = angle.cos() * radial * 1.18;
        let y = angle.sin() * radial * 0.72;
        let normal = (center + east * x + north * y).normalized();
        let flow = (east * angle.cos() + north * angle.sin()).normalized();
        let size = [
            rng.range(0.09, 0.18) * (0.92 + settings.coverage * 0.34),
            rng.range(0.035, 0.075) * (0.90 + settings.coverage * 0.18),
        ];
        puffs.push(make_puff_aligned(
            normal,
            flow,
            size,
            rng.range(0.11, 0.20),
            rng.f32(),
        ));
    }
}

fn make_puff(normal: Vec3, rotation: f32, size: [f32; 2], opacity: f32, seed: f32) -> CloudPuff {
    let (east, north) = tangent_basis(normal);
    let tangent = (east * rotation.cos() + north * rotation.sin()).normalized();
    let bitangent = (north * rotation.cos() - east * rotation.sin()).normalized();
    CloudPuff {
        center: normal * CLOUD_SHELL_RADIUS,
        tangent,
        bitangent,
        normal,
        size,
        opacity,
        seed,
    }
}

fn make_puff_aligned(
    normal: Vec3,
    tangent_hint: Vec3,
    size: [f32; 2],
    opacity: f32,
    seed: f32,
) -> CloudPuff {
    let tangent = (tangent_hint - normal * tangent_hint.dot(normal)).normalized();
    let tangent = if tangent.length_squared() <= f32::EPSILON {
        tangent_basis(normal).0
    } else {
        tangent
    };
    let bitangent = normal.cross(tangent).normalized();
    CloudPuff {
        center: normal * CLOUD_SHELL_RADIUS,
        tangent,
        bitangent,
        normal,
        size,
        opacity,
        seed,
    }
}

fn prevailing_wind_tangent(normal: Vec3, rng: &mut Rng) -> Vec3 {
    let latitude = normal.y.asin().to_degrees().abs();
    let (east, north) = tangent_basis(normal);
    let zonal = if latitude < 28.0 {
        -east
    } else if latitude < 60.0 {
        east
    } else {
        -east
    };
    let meridional = if normal.y >= 0.0 { north } else { -north };
    let meridional_push = if latitude < 18.0 {
        0.08
    } else if latitude < 35.0 {
        0.16
    } else {
        0.22
    };
    let curl = rng.range(-0.10, 0.10);
    (zonal + meridional * (meridional_push + curl)).normalized()
}

fn random_cloud_direction(rng: &mut Rng) -> Vec3 {
    let latitude = rng.range(-58.0, 58.0).to_radians();
    let longitude = rng.range(-180.0, 180.0).to_radians();
    let horizontal = latitude.cos();
    Vec3::new(
        horizontal * longitude.sin(),
        latitude.sin(),
        horizontal * longitude.cos(),
    )
    .normalized()
}

fn tangent_basis(normal: Vec3) -> (Vec3, Vec3) {
    let mut east = Vec3::new(0.0, 1.0, 0.0).cross(normal).normalized();
    if east.length_squared() <= f32::EPSILON {
        east = Vec3::new(1.0, 0.0, 0.0);
    }
    let north = normal.cross(east).normalized();
    (east, north)
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }

    fn range_usize(&mut self, min: usize, max_inclusive: usize) -> usize {
        if max_inclusive <= min {
            min
        } else {
            min + (self.next_u32() as usize % (max_inclusive - min + 1))
        }
    }
}
