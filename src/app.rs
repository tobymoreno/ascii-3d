use std::{
    io::{self, Write, stdout},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

use crate::{
    canvas::{Canvas, ClipRect},
    geometry2d::Point2,
    math::Vec3,
    mesh::Mesh,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::{ProjectionConfig, load_projection_config},
    scene_config::{Quad4SceneConfig, load_quad4_scene_config},
    scenes::{
        RotationAxis, Scene, render_arbitrary_vector, render_asset_axes_rotation, render_axes,
        render_bezier_axes, render_camera, render_camera_motion, render_camera_turntable,
        render_crew, render_cross_negative_z, render_cross_positive_z, render_obj_box, render_pitt,
        render_pitt_crew, render_quad4, render_rotation, render_single_c, render_single_e,
        render_single_i, render_single_p, render_single_r, render_single_t, render_single_w,
        render_world_camera_spaces,
    },
};

const CANVAS_WIDTH: usize = 80;
const CANVAS_HEIGHT: usize = 34;

#[derive(Debug, Clone, Copy)]
struct ViewportRect {
    x: i32,
    y: i32,
    width: usize,
    height: usize,
}

const HEADER_ROW: i32 = 1;

const WORLD_DEBUG_VIEWPORT: ClipRect = ClipRect {
    x: 0,
    y: 2,
    width: CANVAS_WIDTH,
    height: 22,
};

const CAMERA_VIEWPORT: ViewportRect = ViewportRect {
    x: 0,
    y: 24,
    width: CANVAS_WIDTH,
    height: 7,
};

const FOOTER_ROW: i32 = 32;

const ROTATION_SPEED_DEGREES_PER_SECOND: f32 = 30.0;
const FULL_ROTATION_DEGREES: f32 = 360.0;

const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / 60);

const GLYPH_STROKE_CHARACTERS: &[char] = &[
    '*', '+', '#', '@', '%', '&', '=', '-', '~', '.', ':', ';', 'o', 'O', '0', '·', '•', '○', '●',
    '─', '│', '┌', '┐', '└', '┘', '┼', '═', '║', '╔', '╗', '╚', '╝', '╬', '█', '▓', '▒', '░',
];

const DEFAULT_GLYPH_STROKE_INDEX: usize = 0;

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;

        execute!(stdout(), EnterAlternateScreen, Hide, Clear(ClearType::All),)?;

        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

const CAMERA_MOVE_STEP: f32 = 0.10;

fn vec3_cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn vec3_length(value: Vec3) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

fn vec3_normalize(value: Vec3) -> Vec3 {
    let length = vec3_length(value);

    if length <= f32::EPSILON {
        Vec3::zero()
    } else {
        Vec3::new(value.x / length, value.y / length, value.z / length)
    }
}

fn vec3_scale(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn camera_forward_from_yaw_pitch(yaw_degrees: f32, pitch_degrees: f32) -> Vec3 {
    let yaw = yaw_degrees.to_radians();
    let pitch = pitch_degrees.to_radians();
    let horizontal = pitch.cos();

    vec3_normalize(Vec3::new(
        yaw.sin() * horizontal,
        pitch.sin(),
        yaw.cos() * horizontal,
    ))
}

fn camera_right_from_forward(forward: Vec3) -> Vec3 {
    vec3_normalize(vec3_cross(Vec3::new(0.0, 1.0, 0.0), forward))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlMode {
    Scene,
    Camera,
}

impl ControlMode {
    fn label(self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Camera => "Camera",
        }
    }
}

#[derive(Debug)]
struct AppState {
    scene_position: usize,
    animation_angle_degrees: f32,
    box_angle_degrees: f32,
    glyph_stroke_index: usize,
    control_mode: ControlMode,
    world_camera_position: Vec3,
    world_camera_yaw_degrees: f32,
    world_camera_pitch_degrees: f32,
}

impl AppState {
    fn new() -> Self {
        Self {
            scene_position: 0,
            animation_angle_degrees: 0.0,
            box_angle_degrees: 0.0,
            glyph_stroke_index: DEFAULT_GLYPH_STROKE_INDEX,
            control_mode: ControlMode::Scene,
            world_camera_position: Vec3::new(0.65, 0.55, 0.35),
            world_camera_yaw_degrees: 0.0,
            world_camera_pitch_degrees: 0.0,
        }
    }

    fn current_scene(&self) -> Scene {
        Scene::ALL[self.scene_position]
    }

    fn next_scene(&mut self) {
        self.scene_position = (self.scene_position + 1) % Scene::ALL.len();
        self.reset_animation();
    }

    fn previous_scene(&mut self) {
        self.scene_position = if self.scene_position == 0 {
            Scene::ALL.len() - 1
        } else {
            self.scene_position - 1
        };

        self.reset_animation();
    }

    fn reset_animation(&mut self) {
        self.animation_angle_degrees = 0.0;
        self.box_angle_degrees = 0.0;
    }

    fn glyph_stroke_character(&self) -> char {
        GLYPH_STROKE_CHARACTERS[self.glyph_stroke_index]
    }

    fn glyph_stroke_position(&self) -> usize {
        self.glyph_stroke_index + 1
    }

    fn glyph_stroke_character_count(&self) -> usize {
        GLYPH_STROKE_CHARACTERS.len()
    }

    fn next_glyph_stroke_character(&mut self) {
        self.glyph_stroke_index = (self.glyph_stroke_index + 1) % GLYPH_STROKE_CHARACTERS.len();
    }

    fn previous_glyph_stroke_character(&mut self) {
        self.glyph_stroke_index = if self.glyph_stroke_index == 0 {
            GLYPH_STROKE_CHARACTERS.len() - 1
        } else {
            self.glyph_stroke_index - 1
        };
    }

    fn toggle_control_mode(&mut self) {
        self.control_mode = match self.control_mode {
            ControlMode::Scene => ControlMode::Camera,
            ControlMode::Camera => ControlMode::Scene,
        };
    }

    fn move_world_camera(&mut self, delta: Vec3) {
        self.world_camera_position = Vec3::new(
            self.world_camera_position.x + delta.x,
            self.world_camera_position.y + delta.y,
            self.world_camera_position.z + delta.z,
        );
    }

    fn camera_forward(&self) -> Vec3 {
        camera_forward_from_yaw_pitch(
            self.world_camera_yaw_degrees,
            self.world_camera_pitch_degrees,
        )
    }

    fn camera_right(&self) -> Vec3 {
        camera_right_from_forward(self.camera_forward())
    }

    fn move_world_camera_forward(&mut self, amount: f32) {
        self.move_world_camera(vec3_scale(self.camera_forward(), amount));
    }

    fn move_world_camera_right(&mut self, amount: f32) {
        self.move_world_camera(vec3_scale(self.camera_right(), amount));
    }

    fn move_world_camera_up(&mut self, amount: f32) {
        self.move_world_camera(Vec3::new(0.0, amount, 0.0));
    }

    fn rotate_world_camera(&mut self, yaw_delta_degrees: f32, pitch_delta_degrees: f32) {
        self.world_camera_yaw_degrees += yaw_delta_degrees;
        self.world_camera_yaw_degrees %= FULL_ROTATION_DEGREES;

        self.world_camera_pitch_degrees =
            (self.world_camera_pitch_degrees + pitch_delta_degrees).clamp(-80.0, 80.0);
    }

    fn update(&mut self, elapsed: Duration) -> bool {
        let delta_degrees = elapsed.as_secs_f32() * ROTATION_SPEED_DEGREES_PER_SECOND;

        match self.current_scene() {
            Scene::AssetAxesRotateX
            | Scene::AssetAxesRotateY
            | Scene::AssetAxesRotateZ
            | Scene::Quad4
            | Scene::CameraMotion
            | Scene::CameraTurntable
            | Scene::RotateAxesX
            | Scene::RotateAxesY
            | Scene::RotateAxesZ => {
                self.animation_angle_degrees += delta_degrees;
                self.animation_angle_degrees %= FULL_ROTATION_DEGREES;
                true
            }

            Scene::ObjBox => {
                self.box_angle_degrees += delta_degrees;
                self.box_angle_degrees %= FULL_ROTATION_DEGREES;
                true
            }

            _ => false,
        }
    }
}

struct SceneAssets {
    box_mesh: Mesh,
    quad4_mesh: Mesh,
    quad4_scene_config: Quad4SceneConfig,
    projection_config: ProjectionConfig,
    cartesian_axes_mesh: Mesh,
    cartesian_axes_metadata: crate::axis_metadata::CartesianAxesMetadata,
}

fn asset_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(filename)
}

fn load_mesh_asset(filename: &str) -> io::Result<Mesh> {
    let path = asset_path(filename);

    load_obj(&path).map_err(|error| {
        io::Error::other(format!(
            "failed to load OBJ asset {}: {}",
            path.display(),
            error,
        ))
    })
}

fn load_scene_assets() -> io::Result<SceneAssets> {
    let projection_config = load_projection_config(asset_path("projection.default.json"))?;

    let mut box_mesh = load_mesh_asset("box.obj")?;

    if !box_mesh.normalize_to_size(1.0) {
        return Err(io::Error::other("could not normalize assets/box.obj"));
    }

    let quad4_scene_config = load_quad4_scene_config(asset_path("quad4.scene.json"))?;

    if quad4_scene_config.mesh_asset != "quad4.obj" {
        return Err(io::Error::other(format!(
            "quad4.scene.json references unexpected mesh asset '{}'",
            quad4_scene_config.mesh_asset,
        )));
    }

    let quad4_mesh = load_mesh_asset(&quad4_scene_config.mesh_asset)?;

    if quad4_mesh.vertices.len() != 4 {
        return Err(io::Error::other(format!(
            "assets/quad4.obj expected 4 vertices, but loaded {}",
            quad4_mesh.vertices.len(),
        )));
    }

    if quad4_mesh.faces.len() != 1 {
        return Err(io::Error::other(format!(
            "assets/quad4.obj expected 1 face, but loaded {}",
            quad4_mesh.faces.len(),
        )));
    }

    let cartesian_axes_metadata =
        crate::axis_metadata::load_cartesian_axes_metadata(asset_path("cartesian_axes.json"))?;

    if cartesian_axes_metadata.geometry_asset != "cartesian_axes.obj" {
        return Err(io::Error::other(format!(
            "cartesian_axes.json references unexpected geometry asset '{}'",
            cartesian_axes_metadata.geometry_asset,
        )));
    }

    let cartesian_axes_mesh = load_mesh_asset(&cartesian_axes_metadata.geometry_asset)?;

    if cartesian_axes_mesh.vertices.is_empty() {
        return Err(io::Error::other(
            "assets/cartesian_axes.obj contains no vertices",
        ));
    }

    if cartesian_axes_mesh.faces.is_empty() {
        return Err(io::Error::other(
            "assets/cartesian_axes.obj contains no faces",
        ));
    }

    Ok(SceneAssets {
        box_mesh,
        quad4_mesh,
        quad4_scene_config,
        projection_config,
        cartesian_axes_mesh,
        cartesian_axes_metadata,
    })
}

fn projector_from_config(config: &ProjectionConfig) -> ObliqueProjector {
    ObliqueProjector::from_axis_vectors(
        Point2::new(config.screen_origin[0], config.screen_origin[1]),
        config.axis_vectors.x,
        config.axis_vectors.y,
        config.axis_vectors.z,
    )
}

fn write_full_frame(output: &mut impl Write, frame: &str) -> io::Result<()> {
    execute!(output, MoveTo(0, 0))?;
    write!(output, "{frame}")?;
    output.flush()
}

fn write_diff_frame(
    output: &mut impl Write,
    previous_frame: Option<&str>,
    current_frame: &str,
) -> io::Result<()> {
    let Some(previous_frame) = previous_frame else {
        return write_full_frame(output, current_frame);
    };

    for (row_index, (previous_row, current_row)) in previous_frame
        .split("\r\n")
        .zip(current_frame.split("\r\n"))
        .enumerate()
    {
        if previous_row == current_row {
            continue;
        }

        execute!(output, MoveTo(0, row_index as u16))?;
        write!(output, "{current_row}")?;
    }

    output.flush()
}

fn draw_horizontal_span(canvas: &mut Canvas, y: i32, character: char) {
    canvas.draw_line(
        Point2::new(CAMERA_VIEWPORT.x, y),
        Point2::new(CAMERA_VIEWPORT.x + CAMERA_VIEWPORT.width as i32 - 1, y),
        character,
    );
}

fn draw_camera_viewport_placeholder(canvas: &mut Canvas, state: &AppState) {
    let left = CAMERA_VIEWPORT.x;
    let right = CAMERA_VIEWPORT.x + CAMERA_VIEWPORT.width as i32 - 1;
    let top = CAMERA_VIEWPORT.y;
    let bottom = CAMERA_VIEWPORT.y + CAMERA_VIEWPORT.height as i32 - 1;

    draw_horizontal_span(canvas, top, '=');
    draw_horizontal_span(canvas, bottom, '=');

    canvas.draw_line(Point2::new(left, top), Point2::new(left, bottom), '|');
    canvas.draw_line(Point2::new(right, top), Point2::new(right, bottom), '|');

    canvas.set(Point2::new(left, top), '+');
    canvas.set(Point2::new(right, top), '+');
    canvas.set(Point2::new(left, bottom), '+');
    canvas.set(Point2::new(right, bottom), '+');

    canvas.draw_text(Point2::new(left + 2, top + 1), "Camera3D viewport");
    canvas.draw_text(
        Point2::new(left + 2, top + 4),
        &format!(
            "placeholder | pos [{:.2}, {:.2}, {:.2}] | yaw {:.1} pitch {:.1}",
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
            state.world_camera_yaw_degrees,
            state.world_camera_pitch_degrees,
        ),
    );
    canvas.draw_text(
        Point2::new(left + 2, top + 3),
        "next: render actual Camera3D projection into this clipped region",
    );
}

fn render_scene_frame(state: &AppState, assets: &SceneAssets) -> io::Result<String> {
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let projector = projector_from_config(&assets.projection_config);

    match state.current_scene() {
        Scene::WorldCameraSpaces => {
            canvas.with_viewport(WORLD_DEBUG_VIEWPORT, |canvas| {
                render_world_camera_spaces(
                    canvas,
                    state.world_camera_position,
                    state.world_camera_yaw_degrees,
                    state.world_camera_pitch_degrees,
                    Some(state.glyph_stroke_character()),
                )
            })?;
        }

        Scene::PittCrew => {
            render_pitt_crew(&mut canvas, Some(state.glyph_stroke_character()))?;
        }

        Scene::Crew => {
            render_crew(&mut canvas, Some(state.glyph_stroke_character()))?;
        }

        Scene::Pitt => {
            render_pitt(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleE => {
            render_single_e(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleW => {
            render_single_w(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleC => {
            render_single_c(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleR => {
            render_single_r(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleT => {
            render_single_t(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleI => {
            render_single_i(&mut canvas, Some(state.glyph_stroke_character()))?;
        }
        Scene::SingleP => {
            render_single_p(&mut canvas, Some(state.glyph_stroke_character()))?;
        }

        Scene::BezierAxes => {
            render_bezier_axes(&mut canvas)?;
        }

        Scene::AssetAxesRotateX => {
            render_asset_axes_rotation(
                &mut canvas,
                &projector,
                &assets.cartesian_axes_mesh,
                &assets.cartesian_axes_metadata,
                RotationAxis::X,
                state.animation_angle_degrees,
            )?;
        }

        Scene::AssetAxesRotateY => {
            render_asset_axes_rotation(
                &mut canvas,
                &projector,
                &assets.cartesian_axes_mesh,
                &assets.cartesian_axes_metadata,
                RotationAxis::Y,
                state.animation_angle_degrees,
            )?;
        }

        Scene::AssetAxesRotateZ => {
            render_asset_axes_rotation(
                &mut canvas,
                &projector,
                &assets.cartesian_axes_mesh,
                &assets.cartesian_axes_metadata,
                RotationAxis::Z,
                state.animation_angle_degrees,
            )?;
        }

        Scene::Quad4 => {
            render_quad4(
                &mut canvas,
                &projector,
                &assets.quad4_mesh,
                &assets.cartesian_axes_mesh,
                &assets.cartesian_axes_metadata,
                &assets.quad4_scene_config,
                state.animation_angle_degrees,
            )?;
        }

        Scene::CameraMotion => {
            render_camera_motion(&mut canvas, &projector, state.animation_angle_degrees)?;
        }

        Scene::CameraTurntable => {
            render_camera_turntable(&mut canvas, &projector, state.animation_angle_degrees)?;
        }

        Scene::CameraLookAt => {
            render_camera(&mut canvas, &projector)?;
        }

        Scene::ObjBox => {
            render_obj_box(
                &mut canvas,
                &projector,
                &assets.box_mesh,
                state.box_angle_degrees,
            )?;
        }

        Scene::RotateAxesZ => {
            render_rotation(
                &mut canvas,
                &projector,
                RotationAxis::Z,
                state.animation_angle_degrees,
            );
        }

        Scene::RotateAxesY => {
            render_rotation(
                &mut canvas,
                &projector,
                RotationAxis::Y,
                state.animation_angle_degrees,
            );
        }

        Scene::RotateAxesX => {
            render_rotation(
                &mut canvas,
                &projector,
                RotationAxis::X,
                state.animation_angle_degrees,
            );
        }

        Scene::CrossNegativeZ => {
            render_cross_negative_z(&mut canvas, &projector);
        }

        Scene::CrossPositiveZ => {
            render_cross_positive_z(&mut canvas, &projector);
        }

        Scene::ArbitraryVector => {
            render_arbitrary_vector(&mut canvas, &projector);
        }

        Scene::Axes => {
            render_axes(&mut canvas, &projector);
        }
    }

    canvas.draw_text(Point2::new(2, HEADER_ROW), "Scene: WorldSpace3D + Camera3D");

    draw_camera_viewport_placeholder(&mut canvas, state);

    canvas.draw_text(
        Point2::new(2, FOOTER_ROW),
        &format!(
            "[Scene {}/{}] {} | Mode: {} | Glyph '{}' [{}/{}] | Tab: mode | Scene: Space/Backspace/Left/Right | Camera: W/S z, A/D x, Q/E world-Y, Arrows rotate | Esc: quit",
            state.scene_position + 1,
            Scene::ALL.len(),
            state.current_scene().title(),
            state.control_mode.label(),
            state.glyph_stroke_character(),
            state.glyph_stroke_position(),
            state.glyph_stroke_character_count(),
        ),
    );

    Ok(canvas.render())
}

fn render_scene(
    state: &AppState,
    assets: &SceneAssets,
    previous_frame: &mut Option<String>,
) -> io::Result<()> {
    let frame = render_scene_frame(state, assets)?;

    let mut output = stdout();
    write_diff_frame(&mut output, previous_frame.as_deref(), &frame)?;

    *previous_frame = Some(frame);

    Ok(())
}

pub fn run() -> io::Result<()> {
    let assets = load_scene_assets()?;
    let _terminal = TerminalGuard::enter()?;

    let mut state = AppState::new();
    let mut previous_time = Instant::now();
    let mut previous_frame: Option<String> = None;

    render_scene(&state, &assets, &mut previous_frame)?;

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(previous_time);
        previous_time = now;

        if state.update(elapsed) {
            render_scene(&state, &assets, &mut previous_frame)?;
        }

        if !event::poll(FRAME_DURATION)? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if key.code == KeyCode::Esc {
            break;
        }

        match state.control_mode {
            ControlMode::Scene => match key.code {
                KeyCode::Tab => {
                    state.toggle_control_mode();
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char(' ') => {
                    state.next_glyph_stroke_character();
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Backspace => {
                    state.previous_glyph_stroke_character();
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Right | KeyCode::Enter => {
                    state.next_scene();
                    previous_time = Instant::now();
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Left => {
                    state.previous_scene();
                    previous_time = Instant::now();
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    break;
                }

                _ => {}
            },

            ControlMode::Camera => match key.code {
                KeyCode::Tab => {
                    state.toggle_control_mode();
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('w') | KeyCode::Char('W') => {
                    state.move_world_camera_forward(CAMERA_MOVE_STEP);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('s') | KeyCode::Char('S') => {
                    state.move_world_camera_forward(-CAMERA_MOVE_STEP);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('a') | KeyCode::Char('A') => {
                    state.move_world_camera_right(-CAMERA_MOVE_STEP);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('d') | KeyCode::Char('D') => {
                    state.move_world_camera_right(CAMERA_MOVE_STEP);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    state.move_world_camera_up(-CAMERA_MOVE_STEP);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Char('e') | KeyCode::Char('E') => {
                    state.move_world_camera_up(CAMERA_MOVE_STEP);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Left => {
                    state.rotate_world_camera(-5.0, 0.0);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Right => {
                    state.rotate_world_camera(5.0, 0.0);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Up => {
                    state.rotate_world_camera(0.0, 5.0);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                KeyCode::Down => {
                    state.rotate_world_camera(0.0, -5.0);
                    render_scene(&state, &assets, &mut previous_frame)?;
                }

                _ => {}
            },
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppState, asset_path, load_mesh_asset};
    use crate::scenes::Scene;

    #[test]
    fn application_starts_on_single_p_scene() {
        let state = AppState::new();

        assert_eq!(state.current_scene(), Scene::WorldCameraSpaces);
    }

    #[test]
    fn next_scene_moves_to_pittcrew_scene() {
        let mut state = AppState::new();

        state.next_scene();

        assert_eq!(state.current_scene(), Scene::PittCrew);
    }

    #[test]
    fn previous_scene_wraps_to_oldest_scene() {
        let mut state = AppState::new();

        state.previous_scene();

        assert_eq!(state.current_scene(), Scene::Axes);
    }

    #[test]
    fn glyph_stroke_character_defaults_to_star() {
        let state = AppState::new();

        assert_eq!(state.glyph_stroke_character(), '*');
    }

    #[test]
    fn glyph_stroke_character_cycles_forward_and_backward() {
        let mut state = AppState::new();

        state.next_glyph_stroke_character();
        assert_eq!(state.glyph_stroke_character(), '+');

        state.previous_glyph_stroke_character();
        assert_eq!(state.glyph_stroke_character(), '*');
    }

    #[test]
    fn glyph_stroke_character_wraps_backward_to_last_curated_character() {
        let mut state = AppState::new();

        state.previous_glyph_stroke_character();

        assert_eq!(state.glyph_stroke_character(), '░');
    }

    #[test]
    fn changing_scenes_resets_animation_angles() {
        let mut state = AppState::new();

        state.animation_angle_degrees = 45.0;
        state.box_angle_degrees = 90.0;

        state.next_scene();

        assert_eq!(state.animation_angle_degrees, 0.0);
        assert_eq!(state.box_angle_degrees, 0.0);
    }

    #[test]
    fn quad4_asset_exists() {
        assert!(asset_path("quad4.obj").is_file());
    }

    #[test]
    fn quad4_scene_config_exists() {
        assert!(asset_path("quad4.scene.json").is_file());
    }

    #[test]
    fn projection_config_exists() {
        assert!(asset_path("projection.default.json").is_file());
    }

    #[test]
    fn quad4_asset_loads_four_vertices() {
        let mesh = load_mesh_asset("quad4.obj").expect("quad4.obj should load");

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces.len(), 1);
    }
}
