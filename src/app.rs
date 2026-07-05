use std::{
    io::{self, Write, stdout},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use ratatui::{Terminal, backend::CrosstermBackend};

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
    a3d::{AssetRef, LoadedWorld, load_a3d_project},
    canvas::{Canvas, ClipRect},
    curves::CubicBezier3,
    geometry2d::Point2,
    glyphs::{
        GlyphAsset, GlyphMetadata, GlyphSegment, WordAsset, read_json, transform_matrix, vec3,
    },
    input::{
        AppCommand, camera_mode_command_for_key, menu_command_for_key, scene_mode_command_for_key,
    },
    math::{Mat4, Vec3},
    menu::MenuState,
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
    tui::FilePickerView,
};

const CANVAS_WIDTH: usize = 80;
const CANVAS_HEIGHT: usize = 46;

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
    height: 18,
};

const FOOTER_ROW: i32 = 43;

const ROTATION_SPEED_DEGREES_PER_SECOND: f32 = 30.0;
const PULSED_ROTATION_BASE_SPEED_DEGREES_PER_SECOND: f32 = 95.0;
const PULSED_ROTATION_MIN_MULTIPLIER: f32 = 0.28;
const PULSED_ROTATION_BOOST_MULTIPLIER: f32 = 2.45;
const PULSED_ROTATION_SEGMENT_DEGREES: f32 = 180.0;
const FULL_ROTATION_DEGREES: f32 = 360.0;

const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / 60);

const GLYPH_STROKE_CHARACTERS: &[char] = &[
    '*', '+', '#', '@', '%', '&', '=', '-', '~', '.', ':', ';', 'o', 'O', '0', '·', '•', '○', '●',
    '─', '│', '┌', '┐', '└', '┘', '┼', '═', '║', '╔', '╗', '╚', '╝', '╬', '█', '▓', '▒', '░',
];

const DEFAULT_GLYPH_STROKE_INDEX: usize = 0;

const STANDARD_BOX_ASSET: &str = "models/cube.obj";

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

const SINGLE_P_WORD_ASSET: &str = "assets/words/single_p.word.json";

const P_WORD_WORLD_X: f32 = 0.35;
const P_WORD_WORLD_Y: f32 = 0.10;
const P_WORD_WORLD_Z: f32 = -1.80;

const P2_WORD_WORLD_X: f32 = 0.55;
const P2_WORD_WORLD_Y: f32 = 0.10;
const P2_WORD_WORLD_Z: f32 = -3.20;

const P_WORD_WORLD_SCALE: f32 = 1.35;

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

fn yaw_pitch_toward(from: Vec3, to: Vec3) -> (f32, f32) {
    let direction = vec3_normalize(Vec3::new(to.x - from.x, to.y - from.y, to.z - from.z));

    let yaw_degrees = direction.x.atan2(direction.z).to_degrees();
    let horizontal_length = (direction.x * direction.x + direction.z * direction.z).sqrt();
    let pitch_degrees = direction.y.atan2(horizontal_length).to_degrees();

    (yaw_degrees, pitch_degrees)
}

fn camera_right_from_forward(forward: Vec3) -> Vec3 {
    // Match Mat4::look_at:
    // right = forward.cross(world_up)
    vec3_normalize(vec3_cross(forward, Vec3::new(0.0, 1.0, 0.0)))
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

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);

    t * t * (3.0 - (2.0 * t))
}

fn pulsed_rotation_delta_degrees(current_angle_degrees: f32, elapsed: Duration) -> f32 {
    let segment_phase = current_angle_degrees.rem_euclid(PULSED_ROTATION_SEGMENT_DEGREES)
        / PULSED_ROTATION_SEGMENT_DEGREES;

    let triangular_phase = if segment_phase < 0.5 {
        segment_phase * 2.0
    } else {
        (1.0 - segment_phase) * 2.0
    };

    let eased_phase = smoothstep(triangular_phase);
    let speed_multiplier =
        PULSED_ROTATION_MIN_MULTIPLIER + (eased_phase * PULSED_ROTATION_BOOST_MULTIPLIER);

    elapsed.as_secs_f32() * PULSED_ROTATION_BASE_SPEED_DEGREES_PER_SECOND * speed_multiplier
}

#[derive(Debug, Clone, Copy, Default)]
struct RenderTimings {
    update: Duration,
    scene_frame: Duration,
    camera_viewport: Duration,
    tui_draw: Duration,
    total_render: Duration,
}

#[derive(Debug, Clone, Copy, Default)]
struct FrameTimings {
    update: Duration,
    scene_frame: Duration,
    camera_viewport: Duration,
    tui_draw: Duration,
    total_render: Duration,
    total_frame: Duration,
    fps: f32,
}

impl FrameTimings {
    fn from_render(render: RenderTimings) -> Self {
        let total_frame = render.update + render.total_render;
        let fps = if total_frame.is_zero() {
            0.0
        } else {
            1.0 / total_frame.as_secs_f32()
        };

        Self {
            update: render.update,
            scene_frame: render.scene_frame,
            camera_viewport: render.camera_viewport,
            tui_draw: render.tui_draw,
            total_render: render.total_render,
            total_frame,
            fps,
        }
    }

    fn ms(duration: Duration) -> f32 {
        duration.as_secs_f32() * 1_000.0
    }

    fn lines(self) -> Vec<String> {
        vec![
            format!("fps        {:>7.1}", self.fps),
            format!("frame      {:>7.2} ms", Self::ms(self.total_frame)),
            format!("update     {:>7.2} ms", Self::ms(self.update)),
            format!("render     {:>7.2} ms", Self::ms(self.total_render)),
            format!("  scene    {:>7.2} ms", Self::ms(self.scene_frame)),
            format!("  camera   {:>7.2} ms", Self::ms(self.camera_viewport)),
            format!("  tui      {:>7.2} ms", Self::ms(self.tui_draw)),
        ]
    }
}

#[derive(Debug, Clone)]
struct A3dFilePickerEntry {
    label: String,
    path: PathBuf,
    is_dir: bool,
}

#[derive(Debug, Clone)]
struct A3dFilePicker {
    current_dir: PathBuf,
    entries: Vec<A3dFilePickerEntry>,
    selected: usize,
    error: Option<String>,
}

impl A3dFilePicker {
    fn new(current_dir: PathBuf) -> Self {
        let mut picker = Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            error: None,
        };
        picker.refresh();
        picker
    }

    fn refresh(&mut self) {
        self.entries.clear();

        let read_dir = match std::fs::read_dir(&self.current_dir) {
            Ok(read_dir) => read_dir,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                dirs.push(A3dFilePickerEntry {
                    label: format!("[{file_name}]"),
                    path,
                    is_dir: true,
                });
            } else if path.extension().is_some_and(|extension| extension == "a3d") {
                files.push(A3dFilePickerEntry {
                    label: file_name,
                    path,
                    is_dir: false,
                });
            }
        }

        dirs.sort_by(|a, b| a.label.cmp(&b.label));
        files.sort_by(|a, b| a.label.cmp(&b.label));

        self.entries.push(A3dFilePickerEntry {
            label: "[..]".to_string(),
            path: self
                .current_dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.current_dir.clone()),
            is_dir: true,
        });
        self.entries.extend(dirs);
        self.entries.extend(files);
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        self.error = None;
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
    }

    fn selected_entry(&self) -> Option<&A3dFilePickerEntry> {
        self.entries.get(self.selected)
    }

    fn open_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh();
        }
    }

    fn labels(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.label.clone())
            .collect()
    }
}

#[derive(Debug)]
struct AppState {
    scene_position: usize,
    animation_angle_degrees: f32,
    box_angle_degrees: f32,
    glyph_stroke_index: usize,
    control_mode: ControlMode,
    active_menu: Option<MenuState>,
    world_camera_position: Vec3,
    world_camera_yaw_degrees: f32,
    world_camera_pitch_degrees: f32,
    loaded_a3d_world: Option<LoadedWorld>,
    loaded_a3d_root: Option<PathBuf>,
    loaded_a3d_manifest_path: Option<PathBuf>,
    loaded_a3d_debug_popup_until: Option<Instant>,
    loaded_a3d_error: Option<String>,
    a3d_file_picker: Option<A3dFilePicker>,
    show_frame_timing: bool,
    frame_timings: FrameTimings,
}

impl AppState {
    fn new() -> Self {
        let world_camera_position = Vec3::new(0.65, 0.55, 0.35);
        let p_word_position = Vec3::new(P_WORD_WORLD_X, P_WORD_WORLD_Y, P_WORD_WORLD_Z);
        let (world_camera_yaw_degrees, world_camera_pitch_degrees) =
            yaw_pitch_toward(world_camera_position, p_word_position);

        Self {
            scene_position: 0,
            animation_angle_degrees: 0.0,
            box_angle_degrees: 0.0,
            glyph_stroke_index: DEFAULT_GLYPH_STROKE_INDEX,
            control_mode: ControlMode::Scene,
            active_menu: None,
            world_camera_position,
            world_camera_yaw_degrees,
            world_camera_pitch_degrees,
            loaded_a3d_world: None,
            loaded_a3d_root: None,
            loaded_a3d_manifest_path: None,
            loaded_a3d_debug_popup_until: None,
            loaded_a3d_error: None,
            a3d_file_picker: None,
            show_frame_timing: false,
            frame_timings: FrameTimings::default(),
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

    fn open_menu(&mut self, kind: crate::menu::MenuKind) {
        self.active_menu = Some(MenuState::new(kind));
    }

    fn close_menu(&mut self) {
        self.active_menu = None;
    }

    fn move_menu_up(&mut self) {
        if let Some(menu) = &mut self.active_menu {
            menu.move_up();
        }
    }

    fn move_menu_down(&mut self) {
        if let Some(menu) = &mut self.active_menu {
            menu.move_down();
        }
    }

    fn reset_world_camera(&mut self) {
        let world_camera_position = Vec3::new(0.65, 0.55, 0.35);
        let p_word_position = Vec3::new(P_WORD_WORLD_X, P_WORD_WORLD_Y, P_WORD_WORLD_Z);
        let (world_camera_yaw_degrees, world_camera_pitch_degrees) =
            yaw_pitch_toward(world_camera_position, p_word_position);

        self.world_camera_position = world_camera_position;
        self.world_camera_yaw_degrees = world_camera_yaw_degrees;
        self.world_camera_pitch_degrees = world_camera_pitch_degrees;
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

    fn toggle_frame_timing(&mut self) {
        self.show_frame_timing = !self.show_frame_timing;
        self.close_menu();
    }

    fn record_render_timings(&mut self, timings: RenderTimings) {
        self.frame_timings = FrameTimings::from_render(timings);
    }

    fn frame_timing_lines(&self) -> Option<Vec<String>> {
        self.show_frame_timing.then(|| self.frame_timings.lines())
    }

    fn open_a3d_file_picker(&mut self) {
        let start_dir = self
            .loaded_a3d_root
            .clone()
            .and_then(|root| root.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("assets")
                    .join("a3d")
            });

        self.a3d_file_picker = Some(A3dFilePicker::new(start_dir));
        self.close_menu();
    }

    fn close_a3d_file_picker(&mut self) {
        self.a3d_file_picker = None;
    }

    fn move_a3d_file_picker_up(&mut self) {
        if let Some(picker) = &mut self.a3d_file_picker {
            picker.move_up();
        }
    }

    fn move_a3d_file_picker_down(&mut self) {
        if let Some(picker) = &mut self.a3d_file_picker {
            picker.move_down();
        }
    }

    fn a3d_file_picker_parent(&mut self) {
        if let Some(picker) = &mut self.a3d_file_picker {
            picker.open_parent();
        }
    }

    fn select_a3d_file_picker_entry(&mut self) {
        let Some(picker) = &mut self.a3d_file_picker else {
            return;
        };

        let Some(entry) = picker.selected_entry().cloned() else {
            return;
        };

        if entry.is_dir {
            picker.current_dir = entry.path;
            picker.refresh();
            return;
        }

        self.load_a3d_file(entry.path);
        self.a3d_file_picker = None;
    }

    fn load_a3d_root(&mut self, root: PathBuf) {
        self.load_a3d_file(root.join("scene.a3d"));
    }

    fn load_a3d_file(&mut self, manifest_path: PathBuf) {
        let Some(root) = manifest_path.parent().map(Path::to_path_buf) else {
            self.loaded_a3d_error = Some("selected .a3d file has no parent folder".to_string());
            self.loaded_a3d_debug_popup_until = Some(Instant::now() + Duration::from_secs(5));
            return;
        };

        match load_a3d_project(&manifest_path).and_then(|project| {
            project
                .into_world()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        }) {
            Ok(world) => {
                self.loaded_a3d_root = Some(root);
                self.loaded_a3d_manifest_path = Some(manifest_path);
                self.loaded_a3d_world = Some(world);
                self.loaded_a3d_error = None;
                self.loaded_a3d_debug_popup_until = Some(Instant::now() + Duration::from_secs(5));
            }
            Err(error) => {
                self.loaded_a3d_error = Some(error.to_string());
                self.loaded_a3d_debug_popup_until = Some(Instant::now() + Duration::from_secs(5));
            }
        }
    }

    fn reload_a3d(&mut self) {
        if let Some(manifest_path) = self.loaded_a3d_manifest_path.clone() {
            self.load_a3d_file(manifest_path);
            return;
        }

        let Some(root) = self.loaded_a3d_root.clone() else {
            self.load_a3d_root(default_a3d_root_path());
            return;
        };

        self.load_a3d_root(root);
    }

    fn update(&mut self, elapsed: Duration) -> bool {
        let delta_degrees = elapsed.as_secs_f32() * ROTATION_SPEED_DEGREES_PER_SECOND;
        let pulsed_delta_degrees =
            pulsed_rotation_delta_degrees(self.animation_angle_degrees, elapsed);
        let pulsed_box_delta_degrees =
            pulsed_rotation_delta_degrees(self.box_angle_degrees, elapsed);

        match self.current_scene() {
            Scene::LoadedA3d => {
                if let Some(world) = &mut self.loaded_a3d_world {
                    world.update(elapsed.as_secs_f32());
                }
                true
            }

            Scene::AssetAxesRotateX | Scene::AssetAxesRotateY | Scene::AssetAxesRotateZ => {
                self.animation_angle_degrees += pulsed_delta_degrees;
                self.animation_angle_degrees %= FULL_ROTATION_DEGREES;
                true
            }

            Scene::Quad4
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
                self.box_angle_degrees += pulsed_box_delta_degrees;
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

fn default_a3d_root_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("a3d")
        .join("p_depth_demo")
}

fn load_default_a3d_world() -> io::Result<LoadedWorld> {
    let manifest_path = default_a3d_root_path().join("scene.a3d");

    let project = load_a3d_project(&manifest_path)?;
    project.into_world().map_err(io::Error::other)
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

    let mut box_mesh = load_mesh_asset(STANDARD_BOX_ASSET)?;

    if !box_mesh.normalize_to_size(1.0) {
        return Err(io::Error::other(format!(
            "could not normalize assets/{STANDARD_BOX_ASSET}"
        )));
    }

    let quad4_scene_config = load_quad4_scene_config(asset_path("quad4.scene.json"))?;

    if quad4_scene_config.mesh_asset != "models/quad4.obj" {
        return Err(io::Error::other(format!(
            "quad4.scene.json references unexpected mesh asset '{}'",
            quad4_scene_config.mesh_asset,
        )));
    }

    let quad4_mesh = load_mesh_asset(&quad4_scene_config.mesh_asset)?;

    if quad4_mesh.vertices.len() != 4 {
        return Err(io::Error::other(format!(
            "assets/models/quad4.obj expected 4 vertices, but loaded {}",
            quad4_mesh.vertices.len(),
        )));
    }

    if quad4_mesh.faces.len() != 1 {
        return Err(io::Error::other(format!(
            "assets/models/quad4.obj expected 1 face, but loaded {}",
            quad4_mesh.faces.len(),
        )));
    }

    let cartesian_axes_metadata =
        crate::axis_metadata::load_cartesian_axes_metadata(asset_path("cartesian_axes.json"))?;

    if cartesian_axes_metadata.geometry_asset != "models/cartesian_axes.obj" {
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

fn write_frame(output: &mut impl Write, frame: &str) -> io::Result<()> {
    for (row_index, row) in frame
        .split(
            "
",
        )
        .enumerate()
    {
        execute!(
            output,
            MoveTo(0, row_index as u16),
            Clear(ClearType::CurrentLine)
        )?;
        write!(output, "{row}")?;
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

fn camera_viewport_content_rect() -> ClipRect {
    // Protect the title row and status row inside the viewport.
    //
    // Layout:
    //   top border
    //   title row
    //   camera-render content rows
    //   status row
    //   bottom border
    ClipRect {
        x: CAMERA_VIEWPORT.x + 1,
        y: CAMERA_VIEWPORT.y + 2,
        width: CAMERA_VIEWPORT.width.saturating_sub(2),
        height: CAMERA_VIEWPORT.height.saturating_sub(4),
    }
}

fn project_camera_space_to_viewport(camera_space: Vec3, inner: ClipRect) -> Option<Point2> {
    // Mat4::look_at uses the conventional right-handed camera space:
    // +X = camera right, +Y = camera up, and camera forward points along -Z.
    if camera_space.z >= -0.01 {
        return None;
    }

    let center_x = inner.x + inner.width as i32 / 2;
    let center_y = inner.y + inner.height as i32 / 2;
    let depth = -camera_space.z;

    let perspective = 22.0 / depth;
    let screen_x = center_x + (camera_space.x * perspective).round() as i32;
    let screen_y = center_y - (camera_space.y * perspective).round() as i32;

    Some(Point2::new(screen_x, screen_y))
}

fn project_camera_space_to_viewport_with_depth(
    camera_space: Vec3,
    inner: ClipRect,
) -> Option<(Point2, f32)> {
    let point = project_camera_space_to_viewport(camera_space, inner)?;
    let depth = -camera_space.z;

    Some((point, depth))
}

struct CameraViewportDepthBuffer {
    rect: ClipRect,
    depths: Vec<f32>,
}

impl CameraViewportDepthBuffer {
    fn new(rect: ClipRect) -> Self {
        Self {
            rect,
            depths: vec![f32::INFINITY; rect.width * rect.height],
        }
    }

    fn try_update(&mut self, point: Point2, depth: f32) -> bool {
        if !self.rect.contains(point) {
            return false;
        }

        let x = (point.x - self.rect.x) as usize;
        let y = (point.y - self.rect.y) as usize;
        let index = y * self.rect.width + x;

        if depth >= self.depths[index] {
            return false;
        }

        self.depths[index] = depth;
        true
    }
}

fn world_to_camera_space(state: &AppState, point: Vec3) -> Option<Vec3> {
    let forward = camera_forward_from_yaw_pitch(
        state.world_camera_yaw_degrees,
        state.world_camera_pitch_degrees,
    );

    let target = state.world_camera_position + forward;

    Mat4::look_at(
        state.world_camera_position,
        target,
        Vec3::new(0.0, 1.0, 0.0),
    )
    .map(|view| view.transform_point(point))
}

fn draw_camera_viewport_depth_line(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    inner: ClipRect,
    from_world: Vec3,
    to_world: Vec3,
    character: char,
) {
    let Some(from_camera) = world_to_camera_space(state, from_world) else {
        return;
    };
    let Some(to_camera) = world_to_camera_space(state, to_world) else {
        return;
    };

    let Some((from_screen, from_depth)) =
        project_camera_space_to_viewport_with_depth(from_camera, inner)
    else {
        return;
    };
    let Some((to_screen, to_depth)) = project_camera_space_to_viewport_with_depth(to_camera, inner)
    else {
        return;
    };

    let mut x0 = from_screen.x;
    let mut y0 = from_screen.y;
    let x1 = to_screen.x;
    let y1 = to_screen.y;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };

    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut error = dx + dy;
    let steps = dx.max(-dy).max(1);
    let mut step_index = 0;

    loop {
        let t = step_index as f32 / steps as f32;
        let depth = from_depth + (to_depth - from_depth) * t;
        let point = Point2::new(x0, y0);

        if depth_buffer.try_update(point, depth) {
            canvas.set(point, character);
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let doubled_error = 2 * error;

        if doubled_error >= dy {
            error += dy;
            x0 += sx;
        }

        if doubled_error <= dx {
            error += dx;
            y0 += sy;
        }

        step_index += 1;
    }
}

fn draw_single_p_at_camera_viewport(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    inner: ClipRect,
    position: Vec3,
    stroke_character: char,
) -> io::Result<()> {
    let word: WordAsset = read_json(SINGLE_P_WORD_ASSET)?;
    let word_world = Mat4::translation(position.x, position.y, position.z)
        * Mat4::uniform_scale(P_WORD_WORLD_SCALE);

    for child in &word.children {
        let glyph: GlyphAsset = read_json(&child.glyph_asset)?;
        let glyph_metadata: GlyphMetadata = read_json(&child.metadata_asset)?;
        let child_world = word_world * transform_matrix(child.local_transform);
        let display = &glyph_metadata.display;
        let stroke_character = if display.show_strokes {
            stroke_character
        } else {
            display.stroke_character
        };

        for path in &glyph.paths {
            for segment in &path.segments {
                match segment {
                    GlyphSegment::Line { from, to } => {
                        if !display.show_strokes {
                            continue;
                        }

                        let from_world = child_world.transform_point(vec3(*from));
                        let to_world = child_world.transform_point(vec3(*to));

                        draw_camera_viewport_depth_line(
                            canvas,
                            depth_buffer,
                            state,
                            inner,
                            from_world,
                            to_world,
                            stroke_character,
                        );
                    }

                    GlyphSegment::CubicBezier { p0, p1, p2, p3 } => {
                        if !display.show_strokes {
                            continue;
                        }

                        let curve = CubicBezier3::new(vec3(*p0), vec3(*p1), vec3(*p2), vec3(*p3));
                        let sampled = curve.sample(glyph.sampling.default_segments_per_curve);

                        for (start, end) in sampled.line_segments() {
                            let start_world = child_world.transform_point(start);
                            let end_world = child_world.transform_point(end);

                            draw_camera_viewport_depth_line(
                                canvas,
                                depth_buffer,
                                state,
                                inner,
                                start_world,
                                end_world,
                                stroke_character,
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn draw_camera_viewport(canvas: &mut Canvas, state: &AppState) -> io::Result<()> {
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

    let inner = camera_viewport_content_rect();
    let mut depth_buffer = CameraViewportDepthBuffer::new(inner);

    canvas.with_clip_rect(inner, |canvas| {
        draw_single_p_at_camera_viewport(
            canvas,
            &mut depth_buffer,
            state,
            inner,
            Vec3::new(P2_WORD_WORLD_X, P2_WORD_WORLD_Y, P2_WORD_WORLD_Z),
            state.glyph_stroke_character(),
        )?;

        draw_single_p_at_camera_viewport(
            canvas,
            &mut depth_buffer,
            state,
            inner,
            Vec3::new(P_WORD_WORLD_X, P_WORD_WORLD_Y, P_WORD_WORLD_Z),
            state.glyph_stroke_character(),
        )
    })?;

    canvas.draw_text(
        Point2::new(left + 2, bottom - 1),
        &format!(
            "pos [{:.2},{:.2},{:.2}] yaw {:.1} pitch {:.1} | P2 depth test",
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
            state.world_camera_yaw_degrees,
            state.world_camera_pitch_degrees,
        ),
    );

    Ok(())
}

fn render_camera_viewport_canvas(state: &AppState) -> io::Result<Canvas> {
    const VIEWPORT_CONTENT_WIDTH: usize = 78;
    const VIEWPORT_CONTENT_HEIGHT: usize = 16;

    let mut canvas = Canvas::new(VIEWPORT_CONTENT_WIDTH, VIEWPORT_CONTENT_HEIGHT);

    canvas.draw_text(Point2::new(1, 0), "Camera3D viewport content");

    let inner = ClipRect {
        x: 1,
        y: 2,
        width: VIEWPORT_CONTENT_WIDTH.saturating_sub(2),
        height: VIEWPORT_CONTENT_HEIGHT.saturating_sub(5),
    };

    let mut depth_buffer = CameraViewportDepthBuffer::new(inner);

    canvas.with_clip_rect(inner, |canvas| {
        draw_single_p_at_camera_viewport(
            canvas,
            &mut depth_buffer,
            state,
            inner,
            Vec3::new(P2_WORD_WORLD_X, P2_WORD_WORLD_Y, P2_WORD_WORLD_Z),
            state.glyph_stroke_character(),
        )?;

        draw_single_p_at_camera_viewport(
            canvas,
            &mut depth_buffer,
            state,
            inner,
            Vec3::new(P_WORD_WORLD_X, P_WORD_WORLD_Y, P_WORD_WORLD_Z),
            state.glyph_stroke_character(),
        )
    })?;

    canvas.draw_text(
        Point2::new(1, VIEWPORT_CONTENT_HEIGHT as i32 - 2),
        &format!(
            "pos [{:.2},{:.2},{:.2}] yaw {:.1} pitch {:.1} | P2 depth test",
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
            state.world_camera_yaw_degrees,
            state.world_camera_pitch_degrees,
        ),
    );

    Ok(canvas)
}

fn resolve_a3d_asset_path(root: &Path, relative_path: &str) -> io::Result<String> {
    let path = Path::new(relative_path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else if path.exists() {
        path.to_path_buf()
    } else {
        root.join(path)
    };

    resolved
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| io::Error::other(format!("asset path is not UTF-8: {}", resolved.display())))
}

fn draw_loaded_a3d_word_object(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    root: &Path,
    inner: ClipRect,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::Word { path } = &object.asset else {
        return Ok(());
    };

    let word_path = resolve_a3d_asset_path(root, path)?;
    let word: WordAsset = read_json(&word_path)?;
    let object_world = object.transform.matrix();
    let stroke_character = object
        .render
        .stroke_character
        .unwrap_or_else(|| state.glyph_stroke_character());

    for child in &word.children {
        let glyph_path = resolve_a3d_asset_path(root, &child.glyph_asset)?;
        let metadata_path = resolve_a3d_asset_path(root, &child.metadata_asset)?;
        let glyph: GlyphAsset = read_json(&glyph_path)?;
        let glyph_metadata: GlyphMetadata = read_json(&metadata_path)?;
        let child_world = object_world * transform_matrix(child.local_transform);
        let display = &glyph_metadata.display;
        let stroke_character = if display.show_strokes {
            stroke_character
        } else {
            display.stroke_character
        };

        for path in &glyph.paths {
            for segment in &path.segments {
                match segment {
                    GlyphSegment::Line { from, to } => {
                        if !display.show_strokes {
                            continue;
                        }

                        let from_world = child_world.transform_point(vec3(*from));
                        let to_world = child_world.transform_point(vec3(*to));

                        draw_camera_viewport_depth_line(
                            canvas,
                            depth_buffer,
                            state,
                            inner,
                            from_world,
                            to_world,
                            stroke_character,
                        );
                    }

                    GlyphSegment::CubicBezier { p0, p1, p2, p3 } => {
                        if !display.show_strokes {
                            continue;
                        }

                        let curve = CubicBezier3::new(vec3(*p0), vec3(*p1), vec3(*p2), vec3(*p3));
                        let sampled = curve.sample(glyph.sampling.default_segments_per_curve);

                        for (start, end) in sampled.line_segments() {
                            let start_world = child_world.transform_point(start);
                            let end_world = child_world.transform_point(end);

                            draw_camera_viewport_depth_line(
                                canvas,
                                depth_buffer,
                                state,
                                inner,
                                start_world,
                                end_world,
                                stroke_character,
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn render_loaded_a3d_world(
    canvas: &mut Canvas,
    root: Option<&Path>,
    world: Option<&LoadedWorld>,
    state: &AppState,
) -> io::Result<()> {
    canvas.draw_text(Point2::new(2, 3), "LoadedA3d runtime scene");
    canvas.draw_text(
        Point2::new(2, 4),
        "Rendering Word assets from assets/a3d/p_depth_demo/scene.a3d",
    );

    let Some(root) = root else {
        canvas.draw_text(Point2::new(2, 6), "No .a3d root loaded");
        return Ok(());
    };

    let Some(world) = world else {
        canvas.draw_text(Point2::new(2, 6), "No .a3d world loaded");
        return Ok(());
    };

    let inner = ClipRect {
        x: 1,
        y: 6,
        width: CANVAS_WIDTH.saturating_sub(2),
        height: FOOTER_ROW.saturating_sub(8) as usize,
    };

    let mut depth_buffer = CameraViewportDepthBuffer::new(inner);

    canvas.with_clip_rect(inner, |canvas| {
        for object in &world.objects {
            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;
        }

        Ok::<(), io::Error>(())
    })?;

    let mut status_row = FOOTER_ROW - 5;
    canvas.draw_text(
        Point2::new(2, status_row),
        &format!(
            "World: {} | gravity [{:.1}, {:.1}, {:.1}] damping {:.3}",
            world.title,
            world.physics.gravity[0],
            world.physics.gravity[1],
            world.physics.gravity[2],
            world.physics.damping,
        ),
    );

    status_row += 1;

    for object in &world.objects {
        if status_row >= FOOTER_ROW - 1 {
            break;
        }

        canvas.draw_text(
            Point2::new(2, status_row),
            &format!(
                "{} pos [{:.2}, {:.2}, {:.2}] rot [{:.1}, {:.1}, {:.1}] behaviors {}",
                object.id,
                object.transform.position[0],
                object.transform.position[1],
                object.transform.position[2],
                object.transform.rotation_degrees[0],
                object.transform.rotation_degrees[1],
                object.transform.rotation_degrees[2],
                object.behaviors.len(),
            ),
        );

        status_row += 1;
    }

    Ok(())
}

fn draw_loaded_a3d_word_object_in_ws(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    state: &AppState,
    root: &Path,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::Word { path } = &object.asset else {
        return Ok(());
    };

    let word_path = resolve_a3d_asset_path(root, path)?;
    let word: WordAsset = read_json(&word_path)?;
    let object_world = object.transform.matrix();
    let stroke_character = object
        .render
        .stroke_character
        .unwrap_or_else(|| state.glyph_stroke_character());

    for child in &word.children {
        let glyph_path = resolve_a3d_asset_path(root, &child.glyph_asset)?;
        let metadata_path = resolve_a3d_asset_path(root, &child.metadata_asset)?;
        let glyph: GlyphAsset = read_json(&glyph_path)?;
        let glyph_metadata: GlyphMetadata = read_json(&metadata_path)?;
        let child_world = object_world * transform_matrix(child.local_transform);
        let display = &glyph_metadata.display;
        let stroke_character = if display.show_strokes {
            stroke_character
        } else {
            display.stroke_character
        };

        for path in &glyph.paths {
            for segment in &path.segments {
                match segment {
                    GlyphSegment::Line { from, to } => {
                        if !display.show_strokes {
                            continue;
                        }

                        let from_world = child_world.transform_point(vec3(*from));
                        let to_world = child_world.transform_point(vec3(*to));

                        canvas.draw_line(
                            projector.project(from_world),
                            projector.project(to_world),
                            stroke_character,
                        );
                    }

                    GlyphSegment::CubicBezier { p0, p1, p2, p3 } => {
                        if !display.show_strokes {
                            continue;
                        }

                        let curve = CubicBezier3::new(vec3(*p0), vec3(*p1), vec3(*p2), vec3(*p3));
                        let sampled = curve.sample(glyph.sampling.default_segments_per_curve);

                        for (start, end) in sampled.line_segments() {
                            let start_world = child_world.transform_point(start);
                            let end_world = child_world.transform_point(end);

                            canvas.draw_line(
                                projector.project(start_world),
                                projector.project(end_world),
                                stroke_character,
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn draw_loaded_a3d_objects_in_ws(
    canvas: &mut Canvas,
    state: &AppState,
    projector: &ObliqueProjector,
) -> io::Result<()> {
    let Some(root) = state.loaded_a3d_root.as_deref() else {
        return Ok(());
    };

    let Some(world) = state.loaded_a3d_world.as_ref() else {
        return Ok(());
    };

    for object in &world.objects {
        draw_loaded_a3d_word_object_in_ws(canvas, projector, state, root, object)?;
    }

    Ok(())
}

fn render_loaded_a3d_camera_viewport_canvas(state: &AppState) -> io::Result<Canvas> {
    const VIEWPORT_CONTENT_WIDTH: usize = 78;
    const VIEWPORT_CONTENT_HEIGHT: usize = 16;

    let mut canvas = Canvas::new(VIEWPORT_CONTENT_WIDTH, VIEWPORT_CONTENT_HEIGHT);

    canvas.draw_text(Point2::new(1, 0), "LoadedA3d Camera3D viewport");

    let inner = ClipRect {
        x: 1,
        y: 2,
        width: VIEWPORT_CONTENT_WIDTH.saturating_sub(2),
        height: VIEWPORT_CONTENT_HEIGHT.saturating_sub(5),
    };

    let Some(root) = state.loaded_a3d_root.as_deref() else {
        canvas.draw_text(Point2::new(1, 3), "No .a3d root loaded");
        return Ok(canvas);
    };

    let Some(world) = state.loaded_a3d_world.as_ref() else {
        canvas.draw_text(Point2::new(1, 3), "No .a3d world loaded");
        return Ok(canvas);
    };

    let mut depth_buffer = CameraViewportDepthBuffer::new(inner);

    canvas.with_clip_rect(inner, |canvas| {
        for object in &world.objects {
            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;
        }

        Ok::<(), io::Error>(())
    })?;

    canvas.draw_text(
        Point2::new(1, VIEWPORT_CONTENT_HEIGHT as i32 - 2),
        &format!(
            "{} | pos [{:.2},{:.2},{:.2}] yaw {:.1} pitch {:.1}",
            world.title,
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
            state.world_camera_yaw_degrees,
            state.world_camera_pitch_degrees,
        ),
    );

    Ok(canvas)
}

fn render_loaded_a3d_ws_camera_workspace(
    canvas: &mut Canvas,
    state: &AppState,
    projector: &ObliqueProjector,
) {
    let origin = Vec3::zero();
    let positive_x = Vec3::new(4.0, 0.0, 0.0);
    let positive_y = Vec3::new(0.0, 3.0, 0.0);
    let negative_z = Vec3::new(0.0, 0.0, -4.0);

    canvas.draw_line(
        projector.project(origin),
        projector.project(positive_x),
        '-',
    );
    canvas.draw_line(
        projector.project(origin),
        projector.project(positive_y),
        '|',
    );
    canvas.draw_line(
        projector.project(origin),
        projector.project(negative_z),
        '/',
    );

    canvas.draw_text(projector.project(positive_x), "+X");
    canvas.draw_text(projector.project(positive_y), "+Y");
    canvas.draw_text(projector.project(negative_z), "-Z");
    canvas.set(projector.project(origin), 'O');

    let forward = camera_forward_from_yaw_pitch(
        state.world_camera_yaw_degrees,
        state.world_camera_pitch_degrees,
    );
    let right = state.camera_right();
    let eye = state.world_camera_position;
    let near_center = eye + vec3_scale(forward, 0.60);
    let near_left = near_center + vec3_scale(right, -0.35);
    let near_right = near_center + vec3_scale(right, 0.35);

    let eye_screen = projector.project(eye);
    let near_center_screen = projector.project(near_center);
    let near_left_screen = projector.project(near_left);
    let near_right_screen = projector.project(near_right);

    canvas.set(eye_screen, 'E');
    canvas.set(near_center_screen, 'N');
    canvas.draw_line(eye_screen, near_center_screen, '.');
    canvas.draw_line(near_left_screen, near_right_screen, '=');

    canvas.draw_text(Point2::new(2, 3), "WS: LoadedA3d clean worldspace");
    canvas.draw_text(
        Point2::new(2, 4),
        "Only .a3d objects are drawn here; no hardcoded demo letters",
    );
}

fn render_loaded_a3d_studio_world(
    canvas: &mut Canvas,
    state: &AppState,
    projector: &ObliqueProjector,
) -> io::Result<()> {
    canvas.with_viewport(WORLD_DEBUG_VIEWPORT, |canvas| {
        render_loaded_a3d_ws_camera_workspace(canvas, state, projector);
        draw_loaded_a3d_objects_in_ws(canvas, state, projector)
    })?;

    Ok(())
}

fn render_scene_frame(state: &AppState, assets: &SceneAssets) -> io::Result<Canvas> {
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let projector = projector_from_config(&assets.projection_config);

    match state.current_scene() {
        Scene::LoadedA3d => {
            render_loaded_a3d_studio_world(&mut canvas, state, &projector)?;
        }

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

    canvas.draw_text(
        Point2::new(2, HEADER_ROW),
        &format!("Scene: {}", state.current_scene().title()),
    );

    canvas.draw_text(
        Point2::new(2, FOOTER_ROW),
        &format!(
            "[{}/{}] {} | Mode: {} | Glyph '{}' | Menu: {} | h help | Esc quit",
            state.scene_position + 1,
            Scene::ALL.len(),
            state.current_scene().title(),
            state.control_mode.label(),
            state.glyph_stroke_character(),
            state
                .active_menu
                .as_ref()
                .map(|menu| menu.kind().title())
                .unwrap_or("closed"),
        ),
    );

    Ok(canvas)
}

fn is_loaded_a3d_debug_popup_visible(state: &AppState) -> bool {
    matches!(state.current_scene(), Scene::LoadedA3d)
        && state
            .loaded_a3d_debug_popup_until
            .is_some_and(|until| Instant::now() <= until)
}

fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {
    if is_loaded_a3d_debug_popup_visible(state) {
        state.loaded_a3d_debug_popup_until = None;
        true
    } else {
        false
    }
}

fn loaded_a3d_debug_popup_lines(state: &AppState) -> Option<Vec<String>> {
    if !is_loaded_a3d_debug_popup_visible(state) {
        return None;
    }

    let world = state.loaded_a3d_world.as_ref()?;

    let mut lines = vec![
        world.title.clone(),
        format!("objects: {}", world.objects.len()),
        "source: assets/a3d/p_depth_demo/scene.a3d".to_string(),
        "auto-hide: 5 seconds".to_string(),
    ];

    for object in world.objects.iter().take(5) {
        lines.push(format!(
            "{} x={:.2} z={:.2}",
            object.id, object.transform.position[0], object.transform.position[2],
        ));
    }

    Some(lines)
}

fn render_scene(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &AppState,
    assets: &SceneAssets,
    previous_frame: &mut Option<String>,
) -> io::Result<RenderTimings> {
    let total_start = Instant::now();

    let scene_start = Instant::now();
    let scene_canvas = render_scene_frame(state, assets)?;
    let scene_frame = scene_start.elapsed();

    let camera_start = Instant::now();
    let camera_viewport_canvas = match state.current_scene() {
        Scene::LoadedA3d => Some(render_loaded_a3d_camera_viewport_canvas(state)?),
        Scene::WorldCameraSpaces => Some(render_camera_viewport_canvas(state)?),
        _ => None,
    };
    let camera_viewport = camera_start.elapsed();

    let debug_popup_lines = loaded_a3d_debug_popup_lines(state);
    let frame_timing_lines = state.frame_timing_lines();
    let file_picker_labels = state.a3d_file_picker.as_ref().map(|picker| picker.labels());
    let file_picker_current_dir = state
        .a3d_file_picker
        .as_ref()
        .map(|picker| picker.current_dir.display().to_string());

    let tui_start = Instant::now();
    terminal.draw(|frame| {
        let file_picker_view = match (
            state.a3d_file_picker.as_ref(),
            file_picker_labels.as_deref(),
            file_picker_current_dir.as_deref(),
        ) {
            (Some(picker), Some(entries), Some(current_dir)) => Some(FilePickerView {
                title: "Load .a3d",
                current_dir,
                entries,
                selected: picker.selected,
                error: picker.error.as_deref(),
            }),
            _ => None,
        };

        crate::tui::draw(
            frame,
            &scene_canvas,
            camera_viewport_canvas.as_ref(),
            state.active_menu.as_ref(),
            debug_popup_lines.as_deref(),
            frame_timing_lines.as_deref(),
            file_picker_view,
        );
    })?;
    let tui_draw = tui_start.elapsed();

    *previous_frame = Some(scene_canvas.render());

    Ok(RenderTimings {
        update: Duration::ZERO,
        scene_frame,
        camera_viewport,
        tui_draw,
        total_render: total_start.elapsed(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyHandling {
    Handled,
    Ignored,
    Quit,
}

fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {
    match command {
        AppCommand::Quit => KeyHandling::Quit,

        AppCommand::OpenA3dFilePicker => {
            state.open_a3d_file_picker();
            KeyHandling::Handled
        }

        AppCommand::ReloadA3d => {
            state.reload_a3d();
            KeyHandling::Handled
        }

        AppCommand::ShowOsGraphicsOverlay => {
            crate::graphics::raylib_overlay::spawn_raylib_overlay_demo();

            KeyHandling::Handled
        }

        AppCommand::ToggleFrameTiming => {
            state.toggle_frame_timing();
            KeyHandling::Handled
        }

        AppCommand::ToggleControlMode => {
            state.toggle_control_mode();
            KeyHandling::Handled
        }

        AppCommand::NextScene => {
            state.next_scene();
            KeyHandling::Handled
        }

        AppCommand::PreviousScene => {
            state.previous_scene();
            KeyHandling::Handled
        }

        AppCommand::ResetWorldCamera | AppCommand::ResetCamera => {
            state.reset_world_camera();
            KeyHandling::Handled
        }

        AppCommand::NextGlyphStroke => {
            state.next_glyph_stroke_character();
            KeyHandling::Handled
        }

        AppCommand::PreviousGlyphStroke => {
            state.previous_glyph_stroke_character();
            KeyHandling::Handled
        }

        AppCommand::OpenMenu(kind) => {
            state.open_menu(kind);
            KeyHandling::Handled
        }

        AppCommand::CloseMenu => {
            state.close_menu();
            KeyHandling::Handled
        }

        AppCommand::MenuUp => {
            state.move_menu_up();
            KeyHandling::Handled
        }

        AppCommand::MenuDown => {
            state.move_menu_down();
            KeyHandling::Handled
        }

        AppCommand::MenuSelect => {
            let Some(menu) = &state.active_menu else {
                return KeyHandling::Ignored;
            };

            let selected_command = menu.selected_command();
            state.close_menu();
            apply_app_command(state, selected_command)
        }

        AppCommand::MoveCameraForward => {
            state.move_world_camera_forward(CAMERA_MOVE_STEP);
            KeyHandling::Handled
        }

        AppCommand::MoveCameraBackward => {
            state.move_world_camera_forward(-CAMERA_MOVE_STEP);
            KeyHandling::Handled
        }

        AppCommand::MoveCameraLeft => {
            state.move_world_camera_right(-CAMERA_MOVE_STEP);
            KeyHandling::Handled
        }

        AppCommand::MoveCameraRight => {
            state.move_world_camera_right(CAMERA_MOVE_STEP);
            KeyHandling::Handled
        }

        AppCommand::MoveCameraDown => {
            state.move_world_camera_up(-CAMERA_MOVE_STEP);
            KeyHandling::Handled
        }

        AppCommand::MoveCameraUp => {
            state.move_world_camera_up(CAMERA_MOVE_STEP);
            KeyHandling::Handled
        }

        AppCommand::RotateCameraLeft => {
            state.rotate_world_camera(-5.0, 0.0);
            KeyHandling::Handled
        }

        AppCommand::RotateCameraRight => {
            state.rotate_world_camera(5.0, 0.0);
            KeyHandling::Handled
        }

        AppCommand::RotateCameraUp => {
            state.rotate_world_camera(0.0, 5.0);
            KeyHandling::Handled
        }

        AppCommand::RotateCameraDown => {
            state.rotate_world_camera(0.0, -5.0);
            KeyHandling::Handled
        }

        // Cross-term menu placeholders. They intentionally render as handled
        // so the menu stack, hotkeys, and help text can be wired before the
        // feature-specific behavior exists.
        AppCommand::ToggleCameraDebug
        | AppCommand::ToggleNearPlaneDebug
        | AppCommand::ToggleWorldAxes
        | AppCommand::ToggleWorldGrid
        | AppCommand::NextGlyph
        | AppCommand::PreviousGlyph
        | AppCommand::SelectGlyph
        | AppCommand::ToggleSimulationPause
        | AppCommand::StepSimulation
        | AppCommand::ToggleDepthView
        | AppCommand::ToggleProjectionDebug => KeyHandling::Handled,
    }
}

fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {
    if state.a3d_file_picker.is_some() {
        match key_code {
            KeyCode::Esc => {
                state.close_a3d_file_picker();
                return KeyHandling::Handled;
            }
            KeyCode::Up => {
                state.move_a3d_file_picker_up();
                return KeyHandling::Handled;
            }
            KeyCode::Down => {
                state.move_a3d_file_picker_down();
                return KeyHandling::Handled;
            }
            KeyCode::Backspace => {
                state.a3d_file_picker_parent();
                return KeyHandling::Handled;
            }
            KeyCode::Enter => {
                state.select_a3d_file_picker_entry();
                return KeyHandling::Handled;
            }
            _ => return KeyHandling::Ignored,
        }
    }

    if is_loaded_a3d_debug_popup_visible(state) {
        match key_code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('O') => {
                dismiss_loaded_a3d_debug_popup(state);
                return KeyHandling::Handled;
            }
            _ => {}
        }
    }

    if state.active_menu.is_some() {
        return menu_command_for_key(key_code)
            .map(|command| apply_app_command(state, command))
            .unwrap_or(KeyHandling::Ignored);
    }

    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
    };

    command
        .map(|command| apply_app_command(state, command))
        .unwrap_or(KeyHandling::Ignored)
}

pub fn run() -> io::Result<()> {
    let assets = load_scene_assets()?;
    let _terminal_guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut state = AppState::new();
    state.load_a3d_root(default_a3d_root_path());
    let mut previous_time = Instant::now();
    let mut previous_frame: Option<String> = None;

    let timings = render_scene(&mut terminal, &state, &assets, &mut previous_frame)?;
    state.record_render_timings(timings);

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(previous_time);
        previous_time = now;

        let update_start = Instant::now();
        let should_render = state.update(elapsed);
        let update = update_start.elapsed();

        if should_render {
            let mut timings = render_scene(&mut terminal, &state, &assets, &mut previous_frame)?;
            timings.update = update;
            state.record_render_timings(timings);
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

        match handle_key_press(&mut state, key.code) {
            KeyHandling::Quit => break,
            KeyHandling::Handled => {
                previous_time = Instant::now();

                let timings = render_scene(&mut terminal, &state, &assets, &mut previous_frame)?;
                state.record_render_timings(timings);
            }
            KeyHandling::Ignored => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppState, asset_path, load_mesh_asset};
    use crate::scenes::Scene;

    #[test]
    fn application_starts_on_loaded_a3d_scene() {
        let state = AppState::new();

        assert_eq!(state.current_scene(), Scene::LoadedA3d);
    }

    #[test]
    fn next_scene_moves_to_world_camera_spaces_scene() {
        let mut state = AppState::new();

        state.next_scene();

        assert_eq!(state.current_scene(), Scene::WorldCameraSpaces);
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
        assert!(asset_path("models/quad4.obj").is_file());
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
    fn default_a3d_world_loads() {
        let world = super::load_default_a3d_world().expect("default .a3d world should load");

        assert_eq!(world.title, "PITT CREW depth stack demo");
        assert_eq!(world.objects.len(), 8);
        assert!(world.object("letter_p").is_some());
        assert!(world.object("letter_w").is_some());
    }

    #[test]
    fn quad4_asset_loads_four_vertices() {
        let mesh = load_mesh_asset("models/quad4.obj").expect("models/quad4.obj should load");

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces.len(), 1);
    }
    #[test]
    fn standard_cube_obj_asset_exists() {
        assert!(asset_path("models/cube.obj").is_file());
    }

    #[test]
    fn standard_cube_obj_asset_loads_as_wireframe_cube() {
        let mesh = load_mesh_asset("models/cube.obj").expect("models/cube.obj should load");

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 6);
        assert_eq!(mesh.unique_edges().len(), 12);
    }
}
