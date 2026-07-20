use std::{f32::consts::TAU, fs, path::Path, time::Instant};

use serde::Deserialize;

use crate::math::Vec3;

const CONFIG_PATH: &str = "assets/a3d/celestial_sphere/config.json";

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct CelestialSphereConfig {
    enabled: bool,
    seed: u64,
    sphere: SphereConfig,
    stars: StarConfig,
    appearance: AppearanceConfig,
    twinkle: TwinkleConfig,
    clusters: ClusterConfig,
}

impl Default for CelestialSphereConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            seed: 1337,
            sphere: SphereConfig::default(),
            stars: StarConfig::default(),
            appearance: AppearanceConfig::default(),
            twinkle: TwinkleConfig::default(),
            clusters: ClusterConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct SphereConfig {
    rotation_degrees: [f32; 3],
    rotation_speed_degrees_per_second: f32,
}

impl Default for SphereConfig {
    fn default() -> Self {
        Self {
            rotation_degrees: [0.0, 0.0, 0.0],
            rotation_speed_degrees_per_second: 0.0035,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct StarConfig {
    count: usize,
    minimum_size_pixels: f32,
    maximum_size_pixels: f32,
    minimum_brightness: f32,
    maximum_brightness: f32,
    medium_star_fraction: f32,
    hero_star_fraction: f32,
}

impl Default for StarConfig {
    fn default() -> Self {
        Self {
            count: 3200,
            minimum_size_pixels: 0.45,
            maximum_size_pixels: 2.8,
            minimum_brightness: 0.22,
            maximum_brightness: 1.0,
            medium_star_fraction: 0.13,
            hero_star_fraction: 0.026,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct AppearanceConfig {
    minimum_softness: f32,
    maximum_softness: f32,
    neutral: [f32; 3],
    cool: [f32; 3],
    warm: [f32; 3],
    cool_fraction: f32,
    warm_fraction: f32,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            minimum_softness: 0.04,
            maximum_softness: 0.98,
            neutral: [0.94, 0.96, 1.0],
            cool: [0.72, 0.84, 1.0],
            warm: [1.0, 0.86, 0.70],
            cool_fraction: 0.19,
            warm_fraction: 0.08,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct TwinkleConfig {
    enabled: bool,
    minimum_amplitude: f32,
    maximum_amplitude: f32,
    minimum_speed_hz: f32,
    maximum_speed_hz: f32,
    speed_modulation: f32,
    speed_modulation_hz: f32,
}

impl Default for TwinkleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            minimum_amplitude: 0.03,
            maximum_amplitude: 0.28,
            minimum_speed_hz: 0.09,
            maximum_speed_hz: 0.75,
            speed_modulation: 0.42,
            speed_modulation_hz: 0.043,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct ClusterConfig {
    enabled: bool,
    count: usize,
    stars_per_cluster_minimum: usize,
    stars_per_cluster_maximum: usize,
    cluster_radius_degrees_minimum: f32,
    cluster_radius_degrees_maximum: f32,
    haze_puffs_minimum: usize,
    haze_puffs_maximum: usize,
    haze_size_pixels_minimum: f32,
    haze_size_pixels_maximum: f32,
    haze_brightness_minimum: f32,
    haze_brightness_maximum: f32,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            count: 18,
            stars_per_cluster_minimum: 18,
            stars_per_cluster_maximum: 44,
            cluster_radius_degrees_minimum: 2.6,
            cluster_radius_degrees_maximum: 7.8,
            haze_puffs_minimum: 3,
            haze_puffs_maximum: 7,
            haze_size_pixels_minimum: 5.0,
            haze_size_pixels_maximum: 16.0,
            haze_brightness_minimum: 0.04,
            haze_brightness_maximum: 0.16,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct StarInstance {
    direction: Vec3,
    size_pixels: f32,
    brightness: f32,
    softness: f32,
    color: [f32; 3],
    phase: f32,
    twinkle_speed_hz: f32,
    twinkle_amplitude: f32,
    speed_phase: f32,
    core_intensity: f32,
    halo_scale: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ViewportStar {
    pub(crate) position: [f32; 2],
    pub(crate) size_pixels: f32,
    pub(crate) brightness: f32,
    pub(crate) softness: f32,
    pub(crate) color: [f32; 3],
    pub(crate) core_intensity: f32,
    pub(crate) halo_scale: f32,
}

pub(crate) struct Starfield {
    config: CelestialSphereConfig,
    stars: Vec<StarInstance>,
    started_at: Instant,
}

impl Starfield {
    pub(crate) fn load(project_root: &Path) -> Self {
        let config_path = project_root.join(CONFIG_PATH);
        let config = fs::read_to_string(&config_path)
            .ok()
            .and_then(|text| serde_json::from_str::<CelestialSphereConfig>(&text).ok())
            .unwrap_or_default();
        let stars = generate_stars(&config);
        Self {
            config,
            stars,
            started_at: Instant::now(),
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.stars.len()
    }

    pub(crate) fn project(
        &self,
        forward: Vec3,
        right: Vec3,
        up: Vec3,
        width: f32,
        height: f32,
        fov_degrees: f32,
    ) -> Vec<ViewportStar> {
        if !self.config.enabled || width <= 1.0 || height <= 1.0 {
            return Vec::new();
        }

        let time = self.started_at.elapsed().as_secs_f32();
        let global_rotation = self
            .config
            .sphere
            .rotation_speed_degrees_per_second
            .to_radians()
            * time
            + self.config.sphere.rotation_degrees[1].to_radians();
        let cosine = global_rotation.cos();
        let sine = global_rotation.sin();
        let focal_length = 0.5 * height / (0.5 * fov_degrees.to_radians()).tan();

        self.stars
            .iter()
            .filter_map(|star| {
                let direction = Vec3::new(
                    star.direction.x * cosine + star.direction.z * sine,
                    star.direction.y,
                    -star.direction.x * sine + star.direction.z * cosine,
                );
                let depth = direction.dot(forward);
                if depth <= 0.02 {
                    return None;
                }

                let x = direction.dot(right) / depth;
                let y = direction.dot(up) / depth;
                let screen = [
                    width * 0.5 + x * focal_length,
                    height * 0.5 - y * focal_length,
                ];
                let margin = star.size_pixels * (3.0 + star.halo_scale);
                if screen[0] < -margin
                    || screen[0] > width + margin
                    || screen[1] < -margin
                    || screen[1] > height + margin
                {
                    return None;
                }

                let brightness = if self.config.twinkle.enabled {
                    let speed_wave = (time * self.config.twinkle.speed_modulation_hz * TAU
                        + star.speed_phase)
                        .sin();
                    let varying_speed = star.twinkle_speed_hz
                        * (1.0 + speed_wave * self.config.twinkle.speed_modulation);
                    let pulse = (time * varying_speed * TAU + star.phase).sin();
                    (star.brightness * (1.0 + pulse * star.twinkle_amplitude)).clamp(0.0, 1.0)
                } else {
                    star.brightness
                };

                Some(ViewportStar {
                    position: screen,
                    size_pixels: star.size_pixels,
                    brightness,
                    softness: star.softness,
                    color: star.color,
                    core_intensity: star.core_intensity,
                    halo_scale: star.halo_scale,
                })
            })
            .collect()
    }
}

fn generate_stars(config: &CelestialSphereConfig) -> Vec<StarInstance> {
    let mut random = SplitMix64::new(config.seed);
    let mut stars = Vec::with_capacity(config.stars.count + config.clusters.count * 64);

    for _ in 0..config.stars.count {
        stars.push(sample_regular_star(config, &mut random));
    }

    if config.clusters.enabled {
        for _ in 0..config.clusters.count {
            append_cluster(config, &mut random, &mut stars);
        }
    }

    stars
}

fn sample_regular_star(config: &CelestialSphereConfig, random: &mut SplitMix64) -> StarInstance {
    let direction = sample_unit_vector(random);
    let class_roll = random.unit();
    let hero = class_roll < config.stars.hero_star_fraction;
    let medium =
        !hero && class_roll < config.stars.hero_star_fraction + config.stars.medium_star_fraction;

    let size_bias = random.unit().powf(2.8);
    let mut size = lerp(
        config.stars.minimum_size_pixels,
        config.stars.maximum_size_pixels,
        size_bias,
    );
    if medium {
        size = size.max(config.stars.maximum_size_pixels * 0.46);
    }
    if hero {
        size = size.max(config.stars.maximum_size_pixels * 0.78);
    }

    let brightness = lerp(
        config.stars.minimum_brightness,
        config.stars.maximum_brightness,
        random.unit().powf(1.7),
    );
    let softness = lerp(
        config.appearance.minimum_softness,
        config.appearance.maximum_softness,
        random.unit(),
    );

    let color = sample_star_color(config, random);

    StarInstance {
        direction,
        size_pixels: size,
        brightness,
        softness,
        color,
        phase: random.range(0.0, TAU),
        twinkle_speed_hz: random.range(
            config.twinkle.minimum_speed_hz,
            config.twinkle.maximum_speed_hz,
        ),
        twinkle_amplitude: random.range(
            config.twinkle.minimum_amplitude,
            config.twinkle.maximum_amplitude,
        ),
        speed_phase: random.range(0.0, TAU),
        core_intensity: if hero { 1.0 } else { 0.85 },
        halo_scale: if hero {
            2.0
        } else if medium {
            1.65
        } else {
            1.25
        },
    }
}

fn append_cluster(
    config: &CelestialSphereConfig,
    random: &mut SplitMix64,
    stars: &mut Vec<StarInstance>,
) {
    let center = sample_unit_vector(random);
    let radius_degrees = random.range(
        config.clusters.cluster_radius_degrees_minimum,
        config.clusters.cluster_radius_degrees_maximum,
    );
    let radius_radians = radius_degrees.to_radians();
    let (tangent, bitangent) = tangent_basis(center);
    let cluster_color = sample_star_color(config, random);

    let cluster_star_count = random.range_usize(
        config.clusters.stars_per_cluster_minimum,
        config.clusters.stars_per_cluster_maximum,
    );
    for _ in 0..cluster_star_count {
        let offset_radius = radius_radians * random.unit().powf(1.8);
        let angle = random.range(0.0, TAU);
        let local_offset =
            tangent * (offset_radius * angle.cos()) + bitangent * (offset_radius * angle.sin());
        let direction = (center + local_offset).normalized();
        let size = random.range(0.8, 2.6);
        let brightness = random.range(0.35, 0.95);
        stars.push(StarInstance {
            direction,
            size_pixels: size,
            brightness,
            softness: random.range(0.18, 0.75),
            color: mix_color(cluster_color, sample_star_color(config, random), 0.25),
            phase: random.range(0.0, TAU),
            twinkle_speed_hz: random.range(0.14, 0.95),
            twinkle_amplitude: random.range(0.08, 0.32),
            speed_phase: random.range(0.0, TAU),
            core_intensity: 0.92,
            halo_scale: random.range(1.25, 2.1),
        });
    }

    let haze_count = random.range_usize(
        config.clusters.haze_puffs_minimum,
        config.clusters.haze_puffs_maximum,
    );
    for _ in 0..haze_count {
        let offset_radius = radius_radians * random.range(0.15, 1.05);
        let angle = random.range(0.0, TAU);
        let local_offset =
            tangent * (offset_radius * angle.cos()) + bitangent * (offset_radius * angle.sin());
        let direction = (center + local_offset).normalized();
        stars.push(StarInstance {
            direction,
            size_pixels: random.range(
                config.clusters.haze_size_pixels_minimum,
                config.clusters.haze_size_pixels_maximum,
            ),
            brightness: random.range(
                config.clusters.haze_brightness_minimum,
                config.clusters.haze_brightness_maximum,
            ),
            softness: random.range(0.88, 0.99),
            color: mix_color(cluster_color, config.appearance.cool, 0.35),
            phase: random.range(0.0, TAU),
            twinkle_speed_hz: random.range(0.03, 0.12),
            twinkle_amplitude: random.range(0.01, 0.05),
            speed_phase: random.range(0.0, TAU),
            core_intensity: random.range(0.02, 0.11),
            halo_scale: random.range(2.8, 5.8),
        });
    }
}

fn sample_unit_vector(random: &mut SplitMix64) -> Vec3 {
    let z = random.range(-1.0, 1.0);
    let angle = random.range(0.0, TAU);
    let radial = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(radial * angle.cos(), z, radial * angle.sin())
}

fn tangent_basis(direction: Vec3) -> (Vec3, Vec3) {
    let helper = if direction.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let tangent = direction.cross(helper).normalized();
    let bitangent = direction.cross(tangent).normalized();
    (tangent, bitangent)
}

fn sample_star_color(config: &CelestialSphereConfig, random: &mut SplitMix64) -> [f32; 3] {
    let color_roll = random.unit();
    if color_roll < config.appearance.warm_fraction {
        config.appearance.warm
    } else if color_roll < config.appearance.warm_fraction + config.appearance.cool_fraction {
        config.appearance.cool
    } else {
        config.appearance.neutral
    }
}

fn mix_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1_u64 << 24) as f32)
    }

    fn range(&mut self, minimum: f32, maximum: f32) -> f32 {
        lerp(minimum, maximum, self.unit())
    }

    fn range_usize(&mut self, minimum: usize, maximum: usize) -> usize {
        if minimum >= maximum {
            return minimum;
        }
        minimum + (self.next_u64() as usize % (maximum - minimum + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_for_same_seed() {
        let config = CelestialSphereConfig::default();
        let first = generate_stars(&config);
        let second = generate_stars(&config);
        assert_eq!(first.len(), second.len());
        for (left, right) in first.iter().zip(second.iter()).take(24) {
            assert_eq!(left.direction, right.direction);
            assert_eq!(left.size_pixels, right.size_pixels);
            assert_eq!(left.phase, right.phase);
        }
    }

    #[test]
    fn generated_directions_are_unit_length() {
        let stars = generate_stars(&CelestialSphereConfig::default());
        for star in stars.iter().take(128) {
            assert!((star.direction.length() - 1.0).abs() < 1.0e-4);
        }
    }

    #[test]
    fn size_distribution_is_biased_toward_small_stars() {
        let config = CelestialSphereConfig::default();
        let stars = generate_stars(&config);
        let midpoint = (config.stars.minimum_size_pixels + config.stars.maximum_size_pixels) * 0.5;
        let small = stars
            .iter()
            .filter(|star| star.size_pixels < midpoint)
            .count();
        assert!(small > config.stars.count / 2);
    }

    #[test]
    fn clusters_add_extra_instances() {
        let mut config = CelestialSphereConfig::default();
        config.stars.count = 10;
        config.clusters.count = 2;
        let stars = generate_stars(&config);
        assert!(stars.len() > config.stars.count);
    }
}
