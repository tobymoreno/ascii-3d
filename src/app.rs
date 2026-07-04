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
    canvas::Canvas,
    geometry2d::Point2,
    mesh::Mesh,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::{ProjectionConfig, load_projection_config},
    scene_config::{Quad4SceneConfig, load_quad4_scene_config},
    scenes::{
        RotationAxis, Scene, render_arbitrary_vector, render_asset_axes_rotation, render_axes,
        render_bezier_axes, render_camera, render_camera_motion, render_camera_turntable,
        render_cross_negative_z, render_cross_positive_z, render_obj_box, render_quad4,
        render_rotation, render_single_i, render_single_p,
    },
};

const CANVAS_WIDTH: usize = 80;
const CANVAS_HEIGHT: usize = 28;

const ROTATION_SPEED_DEGREES_PER_SECOND: f32 = 30.0;
const FULL_ROTATION_DEGREES: f32 = 360.0;

const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / 60);

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

#[derive(Debug)]
struct AppState {
    scene_position: usize,
    animation_angle_degrees: f32,
    box_angle_degrees: f32,
}

impl AppState {
    fn new() -> Self {
        Self {
            scene_position: 0,
            animation_angle_degrees: 0.0,
            box_angle_degrees: 0.0,
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

fn render_scene(state: &AppState, assets: &SceneAssets) -> io::Result<()> {
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let projector = projector_from_config(&assets.projection_config);

    match state.current_scene() {
        Scene::SingleI => {
            render_single_i(&mut canvas)?;
        }
        Scene::SingleP => {
            render_single_p(&mut canvas)?;
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

    canvas.draw_text(
        Point2::new(2, 27),
        &format!(
            "[Scene {}/{}] {} | Space/Right: older  Left: newer  Q/Esc: quit",
            state.scene_position + 1,
            Scene::ALL.len(),
            state.current_scene().title(),
        ),
    );

    let mut output = stdout();

    execute!(output, MoveTo(0, 0))?;

    write!(output, "{}", canvas.render())?;
    output.flush()
}

pub fn run() -> io::Result<()> {
    let assets = load_scene_assets()?;
    let _terminal = TerminalGuard::enter()?;

    let mut state = AppState::new();
    let mut previous_time = Instant::now();

    render_scene(&state, &assets)?;

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(previous_time);
        previous_time = now;

        if state.update(elapsed) {
            render_scene(&state, &assets)?;
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

        match key.code {
            KeyCode::Char(' ') | KeyCode::Right | KeyCode::Enter => {
                state.next_scene();
                previous_time = Instant::now();
                render_scene(&state, &assets)?;
            }

            KeyCode::Left => {
                state.previous_scene();
                previous_time = Instant::now();
                render_scene(&state, &assets)?;
            }

            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                break;
            }

            _ => {}
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

        assert_eq!(state.current_scene(), Scene::SingleI);
    }

    #[test]
    fn next_scene_moves_to_single_p_scene() {
        let mut state = AppState::new();

        state.next_scene();

        assert_eq!(state.current_scene(), Scene::SingleP);
    }

    #[test]
    fn previous_scene_wraps_to_oldest_scene() {
        let mut state = AppState::new();

        state.previous_scene();

        assert_eq!(state.current_scene(), Scene::Axes);
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
