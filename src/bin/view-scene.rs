use ascii_3d::render::Frame;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, terminal,
};
use serde::Deserialize;
use std::{
    env, fs, io,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

const WIDTH: usize = 96;
const HEIGHT: usize = 34;

#[derive(Debug, Deserialize)]
struct MultiQuadScene {
    name: String,
    mesh_asset: String,
    display: Display,
    quads: Vec<Quad>,
}

#[derive(Debug, Deserialize)]
struct Display {
    world_scale: f32,
    rotation_y_degrees_per_turn: f32,
}

#[derive(Debug, Deserialize)]
struct Quad {
    id: String,
    position: [f32; 3],
    size: [f32; 2],
    rotation_z_degrees: f32,
    marker: String,
    color: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy)]
struct Mat4 {
    m: [[f32; 4]; 4],
}

impl Mat4 {
    const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn translation(v: Vec3) -> Self {
        let mut result = Self::identity();
        result.m[0][3] = v.x;
        result.m[1][3] = v.y;
        result.m[2][3] = v.z;
        result
    }

    fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn rotation_x(radians: f32) -> Self {
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

    fn rotation_y(radians: f32) -> Self {
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

    fn rotation_z(radians: f32) -> Self {
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

    fn transform_point(self, p: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * p.x + self.m[0][1] * p.y + self.m[0][2] * p.z + self.m[0][3],
            self.m[1][0] * p.x + self.m[1][1] * p.y + self.m[1][2] * p.z + self.m[1][3],
            self.m[2][0] * p.x + self.m[2][1] * p.y + self.m[2][2] * p.z + self.m[2][3],
        )
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = [[0.0; 4]; 4];

        for row in 0..4 {
            for col in 0..4 {
                out[row][col] = self.m[row][0] * rhs.m[0][col]
                    + self.m[row][1] * rhs.m[1][col]
                    + self.m[row][2] * rhs.m[2][col]
                    + self.m[row][3] * rhs.m[3][col];
            }
        }

        Self { m: out }
    }
}

struct ViewerState {
    rotation_x_degrees: f32,
    rotation_y_degrees: f32,
    rotation_z_degrees: f32,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    zoom: f32,
    show_axes: bool,
    fps: f32,
    frame_time_ms: f32,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            rotation_x_degrees: 0.0,
            rotation_y_degrees: 0.0,
            rotation_z_degrees: 0.0,
            origin_x: 0.0,
            origin_y: 0.0,
            origin_z: 0.0,
            zoom: 1.0,
            show_axes: false,
            fps: 0.0,
            frame_time_ms: 0.0,
        }
    }
}

fn validate_scene(scene: &MultiQuadScene) -> io::Result<()> {
    if scene.name.trim().is_empty() {
        return Err(io::Error::other("scene name cannot be empty"));
    }

    if scene.mesh_asset.trim().is_empty() {
        return Err(io::Error::other("mesh_asset cannot be empty"));
    }

    if !scene.display.world_scale.is_finite() || scene.display.world_scale <= 0.0 {
        return Err(io::Error::other(
            "display.world_scale must be finite and greater than zero",
        ));
    }

    if !scene.display.rotation_y_degrees_per_turn.is_finite() {
        return Err(io::Error::other(
            "display.rotation_y_degrees_per_turn must be finite",
        ));
    }

    if scene.quads.is_empty() {
        return Err(io::Error::other("scene must contain at least one quad"));
    }

    for quad in &scene.quads {
        if quad.id.trim().is_empty() {
            return Err(io::Error::other("quad id cannot be empty"));
        }

        if !quad.position.into_iter().all(f32::is_finite) {
            return Err(io::Error::other("quad.position must contain finite values"));
        }

        if !quad.size.into_iter().all(|value| value.is_finite() && value > 0.0) {
            return Err(io::Error::other(
                "quad.size values must be finite and greater than zero",
            ));
        }

        if !quad.rotation_z_degrees.is_finite() {
            return Err(io::Error::other(
                "quad.rotation_z_degrees must be finite",
            ));
        }

        if quad.marker.is_empty() {
            return Err(io::Error::other("quad.marker cannot be empty"));
        }
    }

    Ok(())
}

fn read_scene(path: impl AsRef<Path>) -> io::Result<MultiQuadScene> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read scene {}: {}", path.display(), error),
        )
    })?;

    let scene: MultiQuadScene = serde_json::from_str(&text).map_err(|error| {
        io::Error::other(format!(
            "failed to parse scene {}: {}",
            path.display(),
            error,
        ))
    })?;

    validate_scene(&scene)?;

    Ok(scene)
}

fn marker_char(marker: &str) -> char {
    marker.chars().next().unwrap_or('#')
}

fn shade_char(color: Option<&str>, marker: char) -> char {
    match color.unwrap_or_default().to_ascii_lowercase().as_str() {
        "#e56a2d" => '@',
        "#e0b23a" => '#',
        "#76a9f7" => '*',
        _ => marker,
    }
}

fn screen_project(point: Vec3) -> Option<(i32, i32, f32)> {
    let camera_distance = 8.0;
    let near_clip = 0.25;
    let depth = camera_distance + point.z;

    if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() || depth <= near_clip {
        return None;
    }

    let perspective = camera_distance / depth;

    if !perspective.is_finite() {
        return None;
    }

    let aspect_correction = 2.0;
    let x = point.x * perspective * aspect_correction + WIDTH as f32 * 0.5;
    let y = HEIGHT as f32 * 0.52 - point.y * perspective;

    if !x.is_finite() || !y.is_finite() {
        return None;
    }

    Some((x.round() as i32, y.round() as i32, point.z))
}

fn edge(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (p.0 - a.0) * (b.1 - a.1) - (p.1 - a.1) * (b.0 - a.0)
}

fn fill_triangle(
    frame: &mut Frame,
    a: (i32, i32, f32),
    b: (i32, i32, f32),
    c: (i32, i32, f32),
    ch: char,
) {
    let min_x = a.0.min(b.0).min(c.0).max(0);
    let max_x = a.0.max(b.0).max(c.0).min(WIDTH as i32 - 1);
    let min_y = a.1.min(b.1).min(c.1).max(0);
    let max_y = a.1.max(b.1).max(c.1).min(HEIGHT as i32 - 1);

    let af = (a.0 as f32, a.1 as f32);
    let bf = (b.0 as f32, b.1 as f32);
    let cf = (c.0 as f32, c.1 as f32);

    let area = edge(af, bf, cf);

    if area.abs() < f32::EPSILON {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f32 + 0.5, y as f32 + 0.5);

            let w0 = edge(bf, cf, p) / area;
            let w1 = edge(cf, af, p) / area;
            let w2 = edge(af, bf, p) / area;

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                frame.set(x, y, z, ch);
            }
        }
    }
}

fn draw_line(frame: &mut Frame, a: (i32, i32, f32), b: (i32, i32, f32), ch: char) {
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = a.0;
    let mut y = a.1;
    let steps = dx.max(-dy).max(1) as f32;
    let mut step = 0.0;

    loop {
        let t = step / steps;
        let z = a.2 * (1.0 - t) + b.2 * t;
        frame.set(x, y, z - 0.001, ch);

        if x == b.0 && y == b.1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            err += dx;
            y += sy;
        }

        step += 1.0;
    }
}

fn draw_line_overlay(frame: &mut Frame, a: (i32, i32, f32), b: (i32, i32, f32), ch: char) {
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = a.0;
    let mut y = a.1;

    loop {
        frame.set_overlay(x, y, ch);

        if x == b.0 && y == b.1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}


fn draw_axes(frame: &mut Frame, world: Mat4) {
    let Some(origin) = screen_project(world.transform_point(Vec3::new(0.0, 0.0, 0.0))) else {
        return;
    };

    if let Some(x) = screen_project(world.transform_point(Vec3::new(2.0, 0.0, 0.0))) {
        draw_line_overlay(frame, origin, x, 'x');
    }

    if let Some(y) = screen_project(world.transform_point(Vec3::new(0.0, 2.0, 0.0))) {
        draw_line_overlay(frame, origin, y, 'y');
    }

    if let Some(z) = screen_project(world.transform_point(Vec3::new(0.0, 0.0, 2.0))) {
        draw_line_overlay(frame, origin, z, 'z');
    }
}

fn quad_matrix(scene: &MultiQuadScene, quad: &Quad, state: &ViewerState) -> Mat4 {
    let root = Mat4::translation(Vec3::new(state.origin_x, state.origin_y, state.origin_z))
        * Mat4::rotation_x(state.rotation_x_degrees.to_radians())
        * Mat4::rotation_y(state.rotation_y_degrees.to_radians())
        * Mat4::rotation_z(state.rotation_z_degrees.to_radians())
        * Mat4::scale(
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
        );

    root * Mat4::translation(Vec3::new(
        quad.position[0],
        quad.position[1],
        quad.position[2],
    )) * Mat4::rotation_z(quad.rotation_z_degrees.to_radians())
        * Mat4::scale(quad.size[0], quad.size[1], 1.0)
}

fn draw_quad_scene(frame: &mut Frame, scene: &MultiQuadScene, state: &ViewerState) {
    frame.clear();

    let root = Mat4::translation(Vec3::new(state.origin_x, state.origin_y, state.origin_z))
        * Mat4::rotation_x(state.rotation_x_degrees.to_radians())
        * Mat4::rotation_y(state.rotation_y_degrees.to_radians())
        * Mat4::rotation_z(state.rotation_z_degrees.to_radians())
        * Mat4::scale(
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
        );

    let local_corners = [
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::new(0.5, -0.5, 0.0),
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(-0.5, 0.5, 0.0),
    ];

    for quad in &scene.quads {
        let world = quad_matrix(scene, quad, state);
        let projected = local_corners.map(|corner| screen_project(world.transform_point(corner)));

        let Some(p0) = projected[0] else { continue };
        let Some(p1) = projected[1] else { continue };
        let Some(p2) = projected[2] else { continue };
        let Some(p3) = projected[3] else { continue };

        let fill = shade_char(quad.color.as_deref(), marker_char(&quad.marker));

        fill_triangle(frame, p0, p1, p2, fill);
        fill_triangle(frame, p0, p2, p3, fill);

        draw_line(frame, p0, p1, '+');
        draw_line(frame, p1, p2, '+');
        draw_line(frame, p2, p3, '+');
        draw_line(frame, p3, p0, '+');
    }

    if state.show_axes {
        draw_axes(frame, root);
    }

    frame.draw_text(
        2,
        1,
        &format!(
            "view-scene: {} | quads={} | mesh={}",
            scene.name,
            scene.quads.len(),
            scene.mesh_asset
        ),
    );
    frame.draw_text(
        2,
        2,
        &format!(
            "rot x/y/z = {:+.1}/{:+.1}/{:+.1} | zoom {:.2}",
            state.rotation_x_degrees, state.rotation_y_degrees, state.rotation_z_degrees, state.zoom
        ),
    );
    frame.draw_text(
        2,
        3,
        &format!(
            "origin x/y/z = {:+.1}/{:+.1}/{:+.1} | axes {} | fps {:>5.1} | frame {:>5.2} ms",
            state.origin_x,
            state.origin_y,
            state.origin_z,
            if state.show_axes { "on" } else { "off" },
            state.fps,
            state.frame_time_ms
        ),
    );
    frame.draw_text(
        2,
        HEIGHT - 2,
        "controls: a axes on | A axes off | arrows origin | PgUp/PgDn z | +/- zoom | x/y/z rotate | 0 origin | r reset | q quit",
    );
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn run_viewer(scene: MultiQuadScene) -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    let mut state = ViewerState::default();
    let mut frame = Frame::new(WIDTH, HEIGHT);
    let target_frame = Duration::from_millis(33);
    let mut previous_frame_start = Instant::now();

    // Clear once when entering the viewer. After this, every frame overwrites
    // the same fixed-size buffer without clearing the terminal, which prevents flicker.
    execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;

    loop {
        let frame_start = Instant::now();
        let delta = frame_start.duration_since(previous_frame_start);
        previous_frame_start = frame_start;

        state.frame_time_ms = delta.as_secs_f32() * 1000.0;
        state.fps = if delta.as_secs_f32() > 0.0 {
            1.0 / delta.as_secs_f32()
        } else {
            0.0
        };

        draw_quad_scene(&mut frame, &scene, &state);

        let rendered = frame.render();

        execute!(stdout, cursor::MoveTo(0, 0))?;
        write!(stdout, "{}", rendered)?;
        stdout.flush()?;

        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('x') => state.rotation_x_degrees += 2.0,
                    KeyCode::Char('X') => state.rotation_x_degrees -= 2.0,
                    KeyCode::Char('y') => state.rotation_y_degrees += 2.0,
                    KeyCode::Char('Y') => state.rotation_y_degrees -= 2.0,
                    KeyCode::Char('z') => state.rotation_z_degrees += 2.0,
                    KeyCode::Char('Z') => state.rotation_z_degrees -= 2.0,
                    KeyCode::Char('a') => state.show_axes = true,
                    KeyCode::Char('A') => state.show_axes = false,
                    KeyCode::Char('+') | KeyCode::Char('=') => state.zoom *= 1.1,
                    KeyCode::Char('-') | KeyCode::Char('_') => state.zoom /= 1.1,
                    KeyCode::Left => state.origin_x -= 0.5,
                    KeyCode::Right => state.origin_x += 0.5,
                    KeyCode::Up => state.origin_y += 0.5,
                    KeyCode::Down => state.origin_y -= 0.5,
                    KeyCode::PageUp => state.origin_z += 0.5,
                    KeyCode::PageDown => state.origin_z -= 0.5,
                    KeyCode::Char('0') => {
                        state.origin_x = 0.0;
                        state.origin_y = 0.0;
                        state.origin_z = 0.0;
                    }
                    KeyCode::Char('r') => state = ViewerState::default(),
                    _ => {}
                }
            }
        }

        let elapsed = frame_start.elapsed();

        if elapsed < target_frame {
            std::thread::sleep(target_frame - elapsed);
        }
    }
}

fn main() -> io::Result<()> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/scenes/km_logo_quads.scene.json".to_string());

    let scene = read_scene(&path)?;
    run_viewer(scene)
}
