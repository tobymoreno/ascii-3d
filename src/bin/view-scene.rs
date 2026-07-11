use ascii_3d::{
    render::{draw_line_overlay, Frame, Projection, RenderObject, RenderQuad, RenderScene},
    scene::{load_scene_document, scene_document_to_render_scene},
    viewer::{handle_key, ViewerInput, ViewerState},
};
use crossterm::{
    cursor,
    event::{self, Event},
    execute, terminal,
};
use std::{
    env, io,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

const WIDTH: usize = 96;
const HEIGHT: usize = 34;

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

fn validate_scene(scene: &RenderScene) -> io::Result<()> {
    if scene.name.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scene name must not be empty",
        ));
    }

    if !scene.display.world_scale.is_finite() || scene.display.world_scale <= 0.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scene display.world_scale must be positive and finite",
        ));
    }

    for object in &scene.objects {
        let RenderObject::QuadGroup(group) = object else {
            continue;
        };

        for quad in &group.quads {
            if quad.id.trim().is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "quad id must not be empty",
                ));
            }

            if quad.marker.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} marker must not be empty", quad.id),
                ));
            }

            if quad.position.iter().any(|value| !value.is_finite()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} position must be finite", quad.id),
                ));
            }

            if quad.size.iter().any(|value| !value.is_finite() || *value <= 0.0) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} size must be positive and finite", quad.id),
                ));
            }

            if !quad.rotation_z_degrees.is_finite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} rotation_z_degrees must be finite", quad.id),
                ));
            }
        }
    }

    Ok(())
}

fn read_scene(path: impl AsRef<Path>) -> io::Result<RenderScene> {
    let document = load_scene_document(path)?;
    let scene = scene_document_to_render_scene(document);
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

fn screen_project(scene: &RenderScene, point: Vec3) -> Option<(i32, i32, f32)> {
    let camera = scene.active_camera()?;

    Projection::terminal_with_camera(
        WIDTH,
        HEIGHT,
        camera.projection.camera_distance,
        camera.projection.near_clip,
        camera.projection.vertical_center_ratio,
    )
    .project_xyz(point.x, point.y, point.z)
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


fn draw_axes(frame: &mut Frame, scene: &RenderScene, world: Mat4) {
    let Some(origin) = screen_project(scene, world.transform_point(Vec3::new(0.0, 0.0, 0.0))) else {
        return;
    };

    if let Some(x) = screen_project(scene, world.transform_point(Vec3::new(2.0, 0.0, 0.0))) {
        draw_line_overlay(frame, origin, x, 'x');
    }

    if let Some(y) = screen_project(scene, world.transform_point(Vec3::new(0.0, 2.0, 0.0))) {
        draw_line_overlay(frame, origin, y, 'y');
    }

    if let Some(z) = screen_project(scene, world.transform_point(Vec3::new(0.0, 0.0, 2.0))) {
        draw_line_overlay(frame, origin, z, 'z');
    }
}

fn quad_matrix(scene: &RenderScene, quad: &RenderQuad, state: &ViewerState) -> Mat4 {
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

fn draw_quad_scene(frame: &mut Frame, scene: &RenderScene, state: &ViewerState) {
    frame.clear();

    let Some(quad_group) = scene.objects.iter().find_map(|object| match object {
        RenderObject::QuadGroup(group) => Some(group),
        _ => None,
    }) else {
        frame.draw_text(2, 1, &format!("view-scene: {} | no quad group", scene.name));
        return;
    };

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

    for quad in &quad_group.quads {
        let world = quad_matrix(scene, quad, state);
        let projected = local_corners.map(|corner| screen_project(scene, world.transform_point(corner)));

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
        draw_axes(frame, scene, root);
    }

    frame.draw_text(
        2,
        1,
        &format!(
            "view-scene: {} | quads={} | objects={}",
            scene.name,
            quad_group.quads.len(),
            scene.objects.len()
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

fn run_viewer(scene: RenderScene) -> io::Result<()> {
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
                if handle_key(key.code, &mut state) == ViewerInput::Quit {
                    return Ok(());
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
