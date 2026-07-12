use ascii_3d::render::{
    draw_line_overlay, land_fill_char, lerp_angle_degrees, load_geojson_map_asset,
    load_obj_mesh, lon_lat_to_sphere, point_in_polygon, segment_steps, Frame, GeoJsonMapAsset,
    MeshAsset, MeshVertex, Projection,
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const WIDTH: usize = 100;
const HEIGHT: usize = 38;
const EARTH_CAMERA_DISTANCE: f32 = 34.0;
const EARTH_NEAR_CLIP: f32 = 1.0;
const EARTH_VERTICAL_CENTER_RATIO: f32 = 0.54;
const SHADE_RAMP: &[u8] = b" .:-=+*#%@";

#[derive(Debug, Deserialize)]
struct EarthScene {
    name: String,
    mesh_asset: String,
    display: DisplayConfig,
    lighting: LightingConfig,
    map_overlay: Option<MapOverlayConfig>,
}

#[derive(Debug, Deserialize)]
struct MapOverlayConfig {
    asset: String,
    #[serde(default = "default_map_overlay_visible")]
    visible: bool,
    #[serde(default = "default_map_radius_scale")]
    radius_scale: f32,
}

fn default_map_overlay_visible() -> bool {
    true
}

fn default_map_radius_scale() -> f32 {
    1.018
}

#[derive(Debug, Deserialize)]
struct DisplayConfig {
    world_scale: f32,
    #[allow(dead_code)]
    rotation_y_degrees_per_turn: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct LightingConfig {
    primary_light_direction: [f32; 3],
}

#[derive(Clone, Copy, Debug, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let length = self.length();

        if length <= f32::EPSILON {
            return Self::new(0.0, 1.0, 0.0);
        }

        Self::new(self.x / length, self.y / length, self.z / length)
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x * (1.0 - t) + other.x * t,
            self.y * (1.0 - t) + other.y * t,
            self.z * (1.0 - t) + other.z * t,
        )
    }

    fn translated(self, offset: Self) -> Self {
        Self::new(self.x + offset.x, self.y + offset.y, self.z + offset.z)
    }

    fn from_array(values: [f32; 3]) -> Self {
        Self::new(values[0], values[1], values[2])
    }

    fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Debug)]
struct MapOverlay {
    asset: GeoJsonMapAsset,
    radius_scale: f32,
    visible: bool,
}

#[derive(Clone, Copy)]
struct Mat3 {
    m: [[f32; 3]; 3],
}

impl Mat3 {
    fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    fn rotation_x(degrees: f32) -> Self {
        let r = degrees.to_radians();
        let (s, c) = r.sin_cos();
        Self {
            m: [
                [1.0, 0.0, 0.0],
                [0.0, c, -s],
                [0.0, s, c],
            ],
        }
    }

    fn rotation_y(degrees: f32) -> Self {
        let r = degrees.to_radians();
        let (s, c) = r.sin_cos();
        Self {
            m: [
                [c, 0.0, s],
                [0.0, 1.0, 0.0],
                [-s, 0.0, c],
            ],
        }
    }

    fn rotation_z(degrees: f32) -> Self {
        let r = degrees.to_radians();
        let (s, c) = r.sin_cos();
        Self {
            m: [
                [c, -s, 0.0],
                [s, c, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    fn transform(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        )
    }
}

impl std::ops::Mul for Mat3 {
    type Output = Mat3;

    fn mul(self, rhs: Mat3) -> Self::Output {
        let mut out = Mat3::identity();

        for row in 0..3 {
            for col in 0..3 {
                out.m[row][col] = self.m[row][0] * rhs.m[0][col]
                    + self.m[row][1] * rhs.m[1][col]
                    + self.m[row][2] * rhs.m[2][col];
            }
        }

        out
    }
}


#[derive(Debug)]
struct ViewerState {
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    zoom: f32,
    show_axes: bool,
    show_guides: bool,
    spin: bool,
    spin_axis: char,
    fps: f32,
    frame_ms: f32,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            rotation_x: -12.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
            origin_x: 0.0,
            origin_y: 0.0,
            origin_z: 0.0,
            zoom: 1.0,
            show_axes: false,
            show_guides: true,
            spin: false,
            spin_axis: 'y',
            fps: 0.0,
            frame_ms: 0.0,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/scenes/earth_sphere.scene.json".to_string());

    let scene_path = PathBuf::from(scene_path);
    let scene = load_scene(&scene_path)?;
    let mesh_path = resolve_mesh_path(&scene_path, &scene.mesh_asset);
    let mesh = load_obj_mesh(&mesh_path)?;
    let map_overlay = load_map_overlay(&scene_path, &scene)?;

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let result = run_viewer(&mut stdout, &scene, &mesh, map_overlay.as_ref());

    execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn run_viewer(stdout: &mut io::Stdout, scene: &EarthScene, mesh: &MeshAsset, map_overlay: Option<&MapOverlay>) -> Result<(), Box<dyn Error>> {
    let target_frame = Duration::from_millis(33);
    let mut frame = Frame::new(WIDTH, HEIGHT);
    let mut state = ViewerState::default();

    let light = Vec3::new(
        scene.lighting.primary_light_direction[0],
        scene.lighting.primary_light_direction[1],
        scene.lighting.primary_light_direction[2],
    )
    .normalized();

    let mut previous_frame_start = Instant::now();

    execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;

    loop {
        let frame_start = Instant::now();
        let delta = frame_start.duration_since(previous_frame_start);
        previous_frame_start = frame_start;

        state.frame_ms = delta.as_secs_f32() * 1000.0;
        state.fps = if delta.as_secs_f32() > 0.0 {
            1.0 / delta.as_secs_f32()
        } else {
            0.0
        };

        if state.spin {
            match state.spin_axis {
                'x' => state.rotation_x += 0.75,
                'y' => state.rotation_y += 0.75,
                'z' => state.rotation_z += 0.75,
                _ => state.rotation_y += 0.75,
            }
        }

        frame.clear();
        draw_earth(&mut frame, scene, mesh, &state, light, map_overlay);

        execute!(stdout, cursor::MoveTo(0, 0))?;
        write!(stdout, "{}", frame.render())?;
        stdout.flush()?;

        while event::poll(Duration::from_millis(0))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('1') => {
                    // North America roughly front-facing.
                    state.rotation_x = 18.0;
                    state.rotation_y = -10.0;
                    state.rotation_z = 0.0;
                }
                KeyCode::Char('2') => {
                    // Europe roughly front-facing.
                    state.rotation_x = 18.0;
                    state.rotation_y = 100.0;
                    state.rotation_z = 0.0;
                }
                KeyCode::Char('3') => {
                    // Asia roughly front-facing.
                    state.rotation_x = 18.0;
                    state.rotation_y = 180.0;
                    state.rotation_z = 0.0;
                }
                KeyCode::Char('4') => {
                    // Northern hemisphere overview: North America, Europe, and Asia spread across the view.
                    state.rotation_x = 18.0;
                    state.rotation_y = 100.0;
                    state.rotation_z = 38.0;
                }
                KeyCode::Char('s') => state.spin = true,
                KeyCode::Char('S') => state.spin = false,
                KeyCode::Char('a') => state.show_axes = true,
                KeyCode::Char('A') => state.show_axes = false,
                KeyCode::Char('g') => state.show_guides = !state.show_guides,
                KeyCode::Char('G') => state.show_guides = false,
                KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => state.origin_x -= 0.5,
                KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => state.origin_x += 0.5,
                KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => state.origin_y += 0.5,
                KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => state.origin_y -= 0.5,
                KeyCode::PageUp => state.origin_z += 0.5,
                KeyCode::PageDown => state.origin_z -= 0.5,
                KeyCode::Char('0') => {
                    state.origin_x = 0.0;
                    state.origin_y = 0.0;
                    state.origin_z = 0.0;
                }

                KeyCode::Char('+') | KeyCode::Char('=') => state.zoom *= 1.1,
                KeyCode::Char('-') | KeyCode::Char('_') => state.zoom /= 1.1,
                KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.rotation_x -= 2.0;
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.rotation_x += 2.0;
                }
                KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.rotation_y -= 2.0;
                }
                KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.rotation_y += 2.0;
                }

                // Fallback: some terminals intercept Ctrl+Arrow.
                KeyCode::Up => {
                    state.rotation_x -= 2.0;
                }
                KeyCode::Down => {
                    state.rotation_x += 2.0;
                }
                KeyCode::Left => {
                    state.rotation_y -= 2.0;
                }
                KeyCode::Right => {
                    state.rotation_y += 2.0;
                }

                KeyCode::Char('i') => {
                    state.rotation_x -= 2.0;
                }
                KeyCode::Char('k') => {
                    state.rotation_x += 2.0;
                }
                KeyCode::Char('j') => {
                    state.rotation_y -= 2.0;
                }
                KeyCode::Char('l') => {
                    state.rotation_y += 2.0;
                }

                KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.spin_axis = 'x';
                    state.spin = true;
                }
                KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.spin_axis = 'y';
                    state.spin = true;
                }
                KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.spin_axis = 'z';
                    state.spin = true;
                }
                // Some terminals report Ctrl+letter as raw control characters.
                KeyCode::Char('\u{18}') => {
                    state.spin_axis = 'x';
                    state.spin = true;
                }
                KeyCode::Char('\u{19}') => {
                    state.spin_axis = 'y';
                    state.spin = true;
                }
                KeyCode::Char('\u{1a}') => {
                    state.spin_axis = 'z';
                    state.spin = true;
                }

                KeyCode::Char('x') => {
                    state.rotation_x += 2.0;
                }
                KeyCode::Char('X') => {
                    state.rotation_x -= 2.0;
                }
                KeyCode::Char('y') => {
                    state.rotation_y += 2.0;
                }
                KeyCode::Char('Y') => {
                    state.rotation_y -= 2.0;
                }
                KeyCode::Char('z') => {
                    state.rotation_z += 2.0;
                }
                KeyCode::Char('Z') => {
                    state.rotation_z -= 2.0;
                }
                KeyCode::Char('r') => state = ViewerState::default(),
                _ => {}
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed < target_frame {
            std::thread::sleep(target_frame - elapsed);
        }
    }
}

fn draw_earth(frame: &mut Frame, scene: &EarthScene, mesh: &MeshAsset, state: &ViewerState, light: Vec3, map_overlay: Option<&MapOverlay>) {
    let rotation = Mat3::rotation_z(state.rotation_z)
        * Mat3::rotation_y(state.rotation_y)
        * Mat3::rotation_x(state.rotation_x);

    let scale = scene.display.world_scale * state.zoom;
    let origin = Vec3::new(state.origin_x, state.origin_y, state.origin_z);

    for triangle in &mesh.triangles {
        let a = transform_vertex(triangle.a, rotation, scale, origin);
        let b = transform_vertex(triangle.b, rotation, scale, origin);
        let c = transform_vertex(triangle.c, rotation, scale, origin);

        let Some(pa) = project(Vec3::from_array(a.position)) else { continue };
        let Some(pb) = project(Vec3::from_array(b.position)) else { continue };
        let Some(pc) = project(Vec3::from_array(c.position)) else { continue };

        fill_triangle(frame, pa, pb, pc, Vec3::from_array(a.normal), Vec3::from_array(b.normal), Vec3::from_array(c.normal), light);
    }

    if let Some(map_overlay) = map_overlay {
        if map_overlay.visible {
            draw_map_overlay(frame, map_overlay, rotation, scale, origin);
        }
    }

    if state.show_guides {
        draw_great_circle(frame, rotation, scale, origin, GreatCircle::EquatorY0, 'e');
        draw_great_circle(frame, rotation, scale, origin, GreatCircle::MeridianX0, 'm');
        draw_great_circle(frame, rotation, scale, origin, GreatCircle::MeridianZ0, 'p');
        // Extra latitude guide rings. These are generated overlays, not OBJ geometry.
        draw_latitude_circle(frame, rotation, scale, origin, 60.0, 'N');
        draw_latitude_circle(frame, rotation, scale, origin, 30.0, 'n');
        draw_latitude_circle(frame, rotation, scale, origin, 15.0, '.');
        draw_latitude_circle(frame, rotation, scale, origin, -30.0, 's');

    }

    if state.show_axes {
        draw_axes(frame, rotation, scale, origin);
    }

    frame.draw_text(
        1,
        0,
        &format!(
            "view-earth: {} | mesh triangles={} vertices={} normals={}",
            scene.name,
            mesh.triangles.len(),
            mesh.vertex_count,
            mesh.normal_count
        ),
    );

    frame.draw_text(
        1,
        1,
        &format!(
            "rot x/y/z={:+.1}/{:+.1}/{:+.1} | origin {:+.1}/{:+.1}/{:+.1} | zoom {:.2} | cell {:.2} | axes {} | guides {} | spin {}:{} | fps {:>5.1}",
            state.rotation_x,
            state.rotation_y,
            state.rotation_z,
            state.origin_x,
            state.origin_y,
            state.origin_z,
            state.zoom,
            Projection::terminal_cell_aspect_ratio(),
            if state.show_axes { "on" } else { "off" },
            if state.show_guides { "on" } else { "off" },
            if state.spin { "on" } else { "off" },
            state.spin_axis,
            state.fps
        ),
    );

    frame.draw_text(
        1,
        HEIGHT - 1,
        "controls: Ctrl+X/Y/Z spin on axis | S stop spin | s resume | x/X y/Y z/Z manual rotate | 1-4 presets | g guides | q quit",
    );
}

fn transform_vertex(vertex: MeshVertex, rotation: Mat3, scale: f32, origin: Vec3) -> MeshVertex {
    let position = Vec3::from_array(vertex.position);
    let normal = Vec3::from_array(vertex.normal);

    MeshVertex {
        position: rotation
            .transform(position)
            .scaled(scale)
            .translated(origin)
            .to_array(),
        normal: rotation.transform(normal).normalized().to_array(),
    }
}

trait Scaled {
    fn scaled(self, scale: f32) -> Self;
}

impl Scaled for Vec3 {
    fn scaled(self, scale: f32) -> Self {
        Self::new(self.x * scale, self.y * scale, self.z * scale)
    }
}

fn project(point: Vec3) -> Option<(i32, i32, f32)> {
    Projection::terminal_with_camera(
        WIDTH,
        HEIGHT,
        EARTH_CAMERA_DISTANCE,
        EARTH_NEAR_CLIP,
        EARTH_VERTICAL_CENTER_RATIO,
    )
    .project_xyz(point.x, point.y, point.z)
}

fn fill_triangle(
    frame: &mut Frame,
    a: (i32, i32, f32),
    b: (i32, i32, f32),
    c: (i32, i32, f32),
    na: Vec3,
    nb: Vec3,
    nc: Vec3,
    light: Vec3,
) {
    let min_x = a.0.min(b.0).min(c.0).max(0);
    let max_x = a.0.max(b.0).max(c.0).min(WIDTH as i32 - 1);
    let min_y = a.1.min(b.1).min(c.1).max(0);
    let max_y = a.1.max(b.1).max(c.1).min(HEIGHT as i32 - 1);

    let area = edge(a, b, c.0 as f32, c.1 as f32);

    if area.abs() <= f32::EPSILON {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;

            if w0 >= -0.001 && w1 >= -0.001 && w2 >= -0.001 {
                let z = a.2 * w0 + b.2 * w1 + c.2 * w2;
                let normal = Vec3::new(
                    na.x * w0 + nb.x * w1 + nc.x * w2,
                    na.y * w0 + nb.y * w1 + nc.y * w2,
                    na.z * w0 + nb.z * w1 + nc.z * w2,
                )
                .normalized();

                let diffuse = normal.dot(light).max(0.0);
                let brightness = (0.12 + diffuse * 0.63).clamp(0.0, 0.75);
                let ch = shade_char(brightness);

                frame.set(x, y, z, ch);
            }
        }
    }
}

fn edge(a: (i32, i32, f32), b: (i32, i32, f32), x: f32, y: f32) -> f32 {
    (x - a.0 as f32) * (b.1 as f32 - a.1 as f32)
        - (y - a.1 as f32) * (b.0 as f32 - a.0 as f32)
}

fn shade_char(brightness: f32) -> char {
    let index = (brightness * (SHADE_RAMP.len() - 1) as f32).round() as usize;
    SHADE_RAMP[index.min(SHADE_RAMP.len() - 1)] as char
}

enum GreatCircle {
    EquatorY0,
    MeridianX0,
    MeridianZ0,
}

fn draw_great_circle(frame: &mut Frame, rotation: Mat3, scale: f32, origin: Vec3, circle: GreatCircle, ch: char) {
    let steps = 96;
    let mut previous = None;

    for i in 0..=steps {
        let theta = i as f32 / steps as f32 * std::f32::consts::TAU;
        let (s, c) = theta.sin_cos();

        let local = match circle {
            GreatCircle::EquatorY0 => Vec3::new(c, 0.0, s),
            GreatCircle::MeridianX0 => Vec3::new(0.0, c, s),
            GreatCircle::MeridianZ0 => Vec3::new(c, s, 0.0),
        };

        let world = rotation.transform(local).scaled(scale * 1.01).translated(origin);

        if let Some(current) = project(world) {
            if let Some(prev) = previous {
                draw_line_overlay(frame, prev, current, ch);
            }
            previous = Some(current);
        } else {
            previous = None;
        }
    }
}


fn draw_latitude_circle(
    frame: &mut Frame,
    rotation: Mat3,
    scale: f32,
    origin: Vec3,
    latitude_degrees: f32,
    ch: char,
) {
    let steps = 96;
    let lat = latitude_degrees.to_radians();
    let y = lat.sin();
    let ring_radius = lat.cos();
    let mut previous = None;

    for i in 0..=steps {
        let theta = i as f32 / steps as f32 * std::f32::consts::TAU;
        let (s, c) = theta.sin_cos();

        let local = Vec3::new(ring_radius * c, y, ring_radius * s);
        let world = rotation
            .transform(local)
            .scaled(scale * 1.012)
            .translated(origin);

        if let Some(current) = project(world) {
            if let Some(prev) = previous {
                draw_line_overlay(frame, prev, current, ch);
            }
            previous = Some(current);
        } else {
            previous = None;
        }
    }
}


fn draw_map_overlay(
    frame: &mut Frame,
    map_overlay: &MapOverlay,
    rotation: Mat3,
    scale: f32,
    origin: Vec3,
) {
    let map_scale = scale * map_overlay.radius_scale;

    // Decorative land fill first. This is intentionally not light-based;
    // it is a sparse graphic texture inside each projected GeoJSON contour.
    for line in &map_overlay.asset.lines {
        draw_lon_lat_fill(
            frame,
            &line.points_lon_lat,
            rotation,
            map_scale * 0.999,
            origin,
        );
    }

    // Draw the contour on top so the land edge stays crisp.
    for line in &map_overlay.asset.lines {
        draw_lon_lat_line(
            frame,
            &line.points_lon_lat,
            line.marker,
            rotation,
            map_scale,
            origin,
        );
    }
}

fn draw_lon_lat_fill(
    frame: &mut Frame,
    points_lon_lat: &[(f32, f32)],
    rotation: Mat3,
    scale: f32,
    origin: Vec3,
) {
    let polygon = projected_lon_lat_polygon(points_lon_lat, rotation, scale, origin);

    if polygon.len() < 3 {
        return;
    }

    let min_x = polygon
        .iter()
        .map(|point| point.0)
        .min()
        .unwrap_or(0)
        .max(0);
    let max_x = polygon
        .iter()
        .map(|point| point.0)
        .max()
        .unwrap_or(0)
        .min(WIDTH as i32 - 1);
    let min_y = polygon
        .iter()
        .map(|point| point.1)
        .min()
        .unwrap_or(0)
        .max(0);
    let max_y = polygon
        .iter()
        .map(|point| point.1)
        .max()
        .unwrap_or(0)
        .min(HEIGHT as i32 - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let fill_depth = polygon
        .iter()
        .map(|point| point.2)
        .fold(f32::INFINITY, f32::min);

    if !fill_depth.is_finite() {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if !point_in_polygon(x as f32 + 0.5, y as f32 + 0.5, &polygon) {
                continue;
            }

            if let Some(ch) = land_fill_char(x, y) {
                frame.set(x, y, fill_depth + 0.03, ch);
            }
        }
    }
}

fn projected_lon_lat_polygon(
    points_lon_lat: &[(f32, f32)],
    rotation: Mat3,
    scale: f32,
    origin: Vec3,
) -> Vec<(i32, i32, f32)> {
    let mut polygon = Vec::new();

    if points_lon_lat.len() < 3 {
        return polygon;
    }

    for pair in points_lon_lat.windows(2) {
        let (lon_a, lat_a) = pair[0];
        let (lon_b, lat_b) = pair[1];
        let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);

        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let lon = lerp_angle_degrees(lon_a, lon_b, t);
            let lat = lat_a * (1.0 - t) + lat_b * t;

            let local = lon_lat_to_sphere(lon, lat, 1.0);
            let rotated = rotation.transform(Vec3::new(local.x, local.y, local.z));

            // Match the outline behavior: skip the far hemisphere so land
            // texture does not bleed through the back of the globe.
            if rotated.z > 0.10 {
                continue;
            }

            let world = rotated.scaled(scale).translated(origin);

            if let Some(projected) = project(world) {
                if polygon
                    .last()
                    .map(|last: &(i32, i32, f32)| last.0 != projected.0 || last.1 != projected.1)
                    .unwrap_or(true)
                {
                    polygon.push(projected);
                }
            }
        }
    }

    polygon
}



fn draw_lon_lat_line(
    frame: &mut Frame,
    points_lon_lat: &[(f32, f32)],
    marker: char,
    rotation: Mat3,
    scale: f32,
    origin: Vec3,
) {
    if points_lon_lat.len() < 2 {
        return;
    }

    let mut previous = None;

    for pair in points_lon_lat.windows(2) {
        let (lon_a, lat_a) = pair[0];
        let (lon_b, lat_b) = pair[1];

        let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);

        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let lon = lerp_angle_degrees(lon_a, lon_b, t);
            let lat = lat_a * (1.0 - t) + lat_b * t;

            let local = lon_lat_to_sphere(lon, lat, 1.0);
            let rotated = rotation.transform(Vec3::new(local.x, local.y, local.z));

            // Only draw the camera-facing hemisphere, otherwise the back-side map
            // would show through the globe as an overlay.
            if rotated.z > 0.10 {
                previous = None;
                continue;
            }

            let world = rotated.scaled(scale).translated(origin);

            if let Some(current) = project(world) {
                if let Some(prev) = previous {
                    draw_line_overlay(frame, prev, current, marker);
                }
                previous = Some(current);
            } else {
                previous = None;
            }
        }
    }
}





fn draw_axes(frame: &mut Frame, rotation: Mat3, scale: f32, origin_offset: Vec3) {
    let axis_len = scale * 1.35;
    let origin = project(origin_offset);

    let Some(origin) = origin else {
        return;
    };

    let x = project(rotation.transform(Vec3::new(1.0, 0.0, 0.0)).scaled(axis_len).translated(origin_offset));
    let y = project(rotation.transform(Vec3::new(0.0, 1.0, 0.0)).scaled(axis_len).translated(origin_offset));
    let z = project(rotation.transform(Vec3::new(0.0, 0.0, 1.0)).scaled(axis_len).translated(origin_offset));

    if let Some(x) = x {
        draw_line_overlay(frame, origin, x, 'x');
    }

    if let Some(y) = y {
        draw_line_overlay(frame, origin, y, 'y');
    }

    if let Some(z) = z {
        draw_line_overlay(frame, origin, z, 'z');
    }
}

fn load_scene(path: &Path) -> Result<EarthScene, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn resolve_mesh_path(scene_path: &Path, mesh_asset: &str) -> PathBuf {
    let mesh_path = Path::new(mesh_asset);

    if mesh_path.is_absolute() {
        return mesh_path.to_path_buf();
    }

    let assets_root = scene_path
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("assets"));

    assets_root.join(mesh_asset)
}


fn load_map_overlay(scene_path: &Path, scene: &EarthScene) -> Result<Option<MapOverlay>, Box<dyn Error>> {
    let Some(config) = &scene.map_overlay else {
        return Ok(None);
    };

    let map_path = resolve_mesh_path(scene_path, &config.asset);
    let asset = load_geojson_map_asset(&map_path)?;

    Ok(Some(MapOverlay {
        asset,
        radius_scale: config.radius_scale,
        visible: config.visible,
    }))
}

