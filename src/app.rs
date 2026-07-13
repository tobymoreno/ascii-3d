use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{self, Write, stdout},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};

use ascii_3d::{
    editor_ui::{
        EditorAction, EditorEvent, ObjectHierarchyState, PropertiesState, draw_object_hierarchy,
        draw_properties_panel,
    },
    render::{
        GeoJsonMapAsset, MeshPrepareOptions, lerp_angle_degrees, load_geojson_map_asset,
        load_prepared_mesh, lon_lat_to_sphere, prepare_frame_mesh, rasterize_triangle_clipped,
        segment_steps, visit_prepared_triangles,
    },
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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
        AppCommand, camera_mode_command_for_key, light_mode_command_for_key, menu_command_for_key,
        scene_mode_command_for_key,
    },
    math::{Mat4, Vec3},
    menu::MenuState,
    mesh::Mesh,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::{ProjectionConfig, load_projection_config},
    scene_config::{
        MultiQuadSceneConfig, Quad4SceneConfig, load_multi_quad_scene_config,
        load_quad4_scene_config,
    },
    scenes::{
        RotationAxis, Scene, render_arbitrary_vector, render_asset_axes_rotation, render_axes,
        render_bezier_axes, render_camera, render_camera_motion, render_camera_turntable,
        render_crew, render_cross_negative_z, render_cross_positive_z, render_logo_quads,
        render_obj_box, render_pitt, render_pitt_crew, render_quad4, render_rotation,
        render_single_c, render_single_e, render_single_i, render_single_p, render_single_r,
        render_single_t, render_single_w, render_world_camera_spaces,
    },
    tui::FilePickerView,
    workspace::{
        LoadedA3dWorkspace, WorldEditorTarget,
        gizmo::{LoadedA3dLight, loaded_a3d_lights, normalized_light_direction},
        loaded_a3d_editor_items, loaded_a3d_property_rows, loaded_a3d_world_target,
    },
    xyz_control::{XyzControl, XyzControlEvent},
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

const DEBUG_CONSOLE_HEIGHT: i32 = 9;
const DEBUG_CONSOLE_MAX_LINES: usize = 500;

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
const KM_LOGO_QUADS_SCENE_ASSET: &str = "scenes/km_logo_quads.scene.json";

const P_WORD_WORLD_X: f32 = 0.35;
const P_WORD_WORLD_Y: f32 = 0.10;
const P_WORD_WORLD_Z: f32 = -1.80;

const P2_WORD_WORLD_X: f32 = 0.55;
const P2_WORD_WORLD_Y: f32 = 0.10;
const P2_WORD_WORLD_Z: f32 = -3.20;

const P_WORD_WORLD_SCALE: f32 = 1.35;

const DEFAULT_CAMERA_VIEWPORT_CELL_ASPECT_RATIO: f32 = 0.5;
const DEFAULT_CAMERA_VIEWPORT_PERSPECTIVE_SCALE: f32 = 22.0;
const A3D_PROFILE_DIRECTORY: &str = ".a3dprofile";
const A3D_PROFILE_FILENAME: &str = "state.json";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct A3dProfile {
    version: u32,
    last_a3d_manifest: PathBuf,
}

impl A3dProfile {
    const VERSION: u32 = 1;
}

fn a3d_profile_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;

    Some(home.join(A3D_PROFILE_DIRECTORY).join(A3D_PROFILE_FILENAME))
}

fn read_a3d_profile_manifest() -> Option<PathBuf> {
    let profile_path = a3d_profile_path()?;
    let source = fs::read_to_string(profile_path).ok()?;
    let profile: A3dProfile = serde_json::from_str(&source).ok()?;

    (profile.version == A3dProfile::VERSION).then_some(profile.last_a3d_manifest)
}

fn write_a3d_profile_manifest(manifest_path: &Path) -> io::Result<()> {
    let Some(profile_path) = a3d_profile_path() else {
        return Ok(());
    };

    let persisted_path =
        fs::canonicalize(manifest_path).unwrap_or_else(|_| manifest_path.to_path_buf());

    let profile = A3dProfile {
        version: A3dProfile::VERSION,
        last_a3d_manifest: persisted_path,
    };

    let parent = profile_path
        .parent()
        .ok_or_else(|| io::Error::other("A3D profile path has no parent directory"))?;
    fs::create_dir_all(parent)?;

    let source = serde_json::to_string_pretty(&profile).map_err(io::Error::other)?;
    fs::write(
        profile_path,
        format!(
            "{source}
"
        ),
    )
}

fn manifest_path_from_selection(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.join("scene.a3d")
    } else {
        path
    }
}

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
    Light,
}

fn next_menu_kind(kind: crate::menu::MenuKind) -> crate::menu::MenuKind {
    match kind {
        crate::menu::MenuKind::File => crate::menu::MenuKind::Scenes,
        crate::menu::MenuKind::Scenes => crate::menu::MenuKind::Control,
        crate::menu::MenuKind::Control => crate::menu::MenuKind::Glyphs,
        crate::menu::MenuKind::Glyphs => crate::menu::MenuKind::Physics,
        crate::menu::MenuKind::Physics => crate::menu::MenuKind::Debug,
        crate::menu::MenuKind::Debug => crate::menu::MenuKind::Help,
        crate::menu::MenuKind::Help => crate::menu::MenuKind::File,
    }
}

fn previous_menu_kind(kind: crate::menu::MenuKind) -> crate::menu::MenuKind {
    match kind {
        crate::menu::MenuKind::File => crate::menu::MenuKind::Help,
        crate::menu::MenuKind::Scenes => crate::menu::MenuKind::File,
        crate::menu::MenuKind::Control => crate::menu::MenuKind::Scenes,
        crate::menu::MenuKind::Glyphs => crate::menu::MenuKind::Control,
        crate::menu::MenuKind::Physics => crate::menu::MenuKind::Glyphs,
        crate::menu::MenuKind::Debug => crate::menu::MenuKind::Physics,
        crate::menu::MenuKind::Help => crate::menu::MenuKind::Debug,
    }
}

fn menu_kind_for_hotkey(key_code: KeyCode) -> Option<crate::menu::MenuKind> {
    match key_code {
        KeyCode::Char('f') | KeyCode::Char('F') => Some(crate::menu::MenuKind::File),
        KeyCode::Char('m') | KeyCode::Char('M') => Some(crate::menu::MenuKind::Scenes),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(crate::menu::MenuKind::Control),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(crate::menu::MenuKind::Glyphs),
        KeyCode::Char('p') | KeyCode::Char('P') => Some(crate::menu::MenuKind::Physics),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(crate::menu::MenuKind::Debug),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(crate::menu::MenuKind::Help)
        }
        _ => None,
    }
}

impl ControlMode {
    fn label(self) -> &'static str {
        match self {
            Self::Scene => "World",
            Self::Camera => "Camera",
            Self::Light => "Light",
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
    xyz_control: XyzControl,
    active_menu: Option<MenuState>,
    scene_browser_open: bool,
    scene_browser_selected: usize,
    world_camera_position: Vec3,
    world_camera_yaw_degrees: f32,
    world_camera_pitch_degrees: f32,
    world_origin: Vec3,
    loaded_a3d_world: Option<LoadedWorld>,
    loaded_a3d_root: Option<PathBuf>,
    loaded_a3d_manifest_path: Option<PathBuf>,
    loaded_a3d_lights: Vec<LoadedA3dLight>,
    loaded_a3d_camera_cell_aspect_ratio: f32,
    loaded_a3d_camera_perspective_scale: f32,
    loaded_a3d_camera_aspect_ratio: CameraViewportAspectRatio,
    loaded_a3d_debug_popup_until: Option<Instant>,
    loaded_a3d_error: Option<String>,
    a3d_file_picker: Option<A3dFilePicker>,
    loaded_a3d_workspace: LoadedA3dWorkspace,
    loaded_a3d_hierarchy: ObjectHierarchyState,
    loaded_a3d_properties: PropertiesState,
    show_frame_timing: bool,
    show_debug_console: bool,
    confirm_exit: bool,
    frame_timings: FrameTimings,
    last_input_event_trace: Option<String>,
    debug_console_lines: VecDeque<String>,
    debug_console_scroll: usize,
    debug_console_horizontal_scroll: usize,
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
            xyz_control: XyzControl::default(),
            active_menu: None,
            scene_browser_open: false,
            scene_browser_selected: 0,
            world_camera_position,
            world_camera_yaw_degrees,
            world_camera_pitch_degrees,
            world_origin: Vec3::zero(),
            loaded_a3d_world: None,
            loaded_a3d_root: None,
            loaded_a3d_manifest_path: None,
            loaded_a3d_lights: Vec::new(),
            loaded_a3d_camera_cell_aspect_ratio: DEFAULT_CAMERA_VIEWPORT_CELL_ASPECT_RATIO,
            loaded_a3d_camera_perspective_scale: DEFAULT_CAMERA_VIEWPORT_PERSPECTIVE_SCALE,
            loaded_a3d_camera_aspect_ratio: CameraViewportAspectRatio::DEFAULT,
            loaded_a3d_debug_popup_until: None,
            loaded_a3d_error: None,
            a3d_file_picker: None,
            loaded_a3d_workspace: LoadedA3dWorkspace::new(),
            loaded_a3d_hierarchy: ObjectHierarchyState::default(),
            loaded_a3d_properties: PropertiesState::default(),
            show_frame_timing: false,
            show_debug_console: false,
            confirm_exit: false,
            frame_timings: FrameTimings::default(),
            last_input_event_trace: None,
            debug_console_lines: VecDeque::from([
                "debug console attached to main workspace".to_string(),
                "world/object debug print statements appear here".to_string(),
                "x/X y/Y z/Z rotate; Ctrl/Shift+arrows move origin".to_string(),
            ]),
            debug_console_scroll: 0,
            debug_console_horizontal_scroll: 0,
        }
    }

    fn push_world_debug_lines(&mut self) {
        let mut lines = vec![
            format!("world debug: scene={}", self.current_scene_title()),
            format!(
                "world debug: camera pos [{:.2}, {:.2}, {:.2}] yaw {:.1} pitch {:.1}",
                self.world_camera_position.x,
                self.world_camera_position.y,
                self.world_camera_position.z,
                self.world_camera_yaw_degrees,
                self.world_camera_pitch_degrees,
            ),
            format!(
                "world debug: control_mode={} menu={}",
                self.control_mode.label(),
                self.active_menu
                    .as_ref()
                    .map(|menu| menu.kind().title())
                    .unwrap_or("closed"),
            ),
        ];

        if let Some(world) = self.loaded_a3d_world.as_ref() {
            lines.push(format!(
                "world debug: loaded_a3d title='{}' objects={}",
                world.title,
                world.objects.len(),
            ));

            lines.extend(world.objects.iter().map(|object| {
                format!(
                    "world debug: object={} pos=[{:.2},{:.2},{:.2}] rot=[{:.1},{:.1},{:.1}] scale=[{:.2},{:.2},{:.2}]",
                    object.id,
                    object.transform.position[0],
                    object.transform.position[1],
                    object.transform.position[2],
                    object.transform.rotation_degrees[0],
                    object.transform.rotation_degrees[1],
                    object.transform.rotation_degrees[2],
                    object.transform.scale[0],
                    object.transform.scale[1],
                    object.transform.scale[2],
                )
            }));
        } else {
            lines.push("world debug: no loaded_a3d world".to_string());
        }

        for line in lines {
            self.push_debug_console_line(line);
        }
    }

    fn open_exit_confirm(&mut self) {
        self.confirm_exit = true;
        self.close_menu();
        self.close_a3d_file_picker();
    }

    fn close_exit_confirm(&mut self) {
        self.confirm_exit = false;
    }

    fn close_debug_console(&mut self) {
        self.show_debug_console = false;
    }

    fn toggle_debug_console(&mut self) {
        self.show_debug_console = !self.show_debug_console;
        if self.show_debug_console {
            self.push_world_debug_lines();
        }
        self.push_debug_console_line(format!(
            "debug console: {}",
            if self.show_debug_console {
                "shown"
            } else {
                "hidden"
            }
        ));
    }

    fn push_debug_console_line(&mut self, message: impl Into<String>) {
        self.debug_console_lines.push_back(message.into());

        while self.debug_console_lines.len() > DEBUG_CONSOLE_MAX_LINES {
            self.debug_console_lines.pop_front();
        }

        self.debug_console_scroll = 0;
    }

    fn debug_console_visible_rows(&self) -> usize {
        DEBUG_CONSOLE_HEIGHT.saturating_sub(3) as usize
    }

    fn debug_console_max_scroll(&self) -> usize {
        self.debug_console_lines
            .len()
            .saturating_sub(self.debug_console_visible_rows())
    }

    fn scroll_debug_console_up(&mut self, amount: usize) {
        self.debug_console_scroll =
            (self.debug_console_scroll + amount).min(self.debug_console_max_scroll());
    }

    fn scroll_debug_console_down(&mut self, amount: usize) {
        self.debug_console_scroll = self.debug_console_scroll.saturating_sub(amount);
    }

    fn scroll_debug_console_left(&mut self, amount: usize) {
        self.debug_console_horizontal_scroll =
            self.debug_console_horizontal_scroll.saturating_sub(amount);
    }

    fn scroll_debug_console_right(&mut self, amount: usize) {
        self.debug_console_horizontal_scroll =
            self.debug_console_horizontal_scroll.saturating_add(amount);
    }

    fn current_scene_descriptor(&self) -> crate::scenes::SceneDescriptor {
        crate::scenes::scene_descriptor_at(self.scene_position)
    }

    fn current_scene(&self) -> Scene {
        self.current_scene_descriptor().scene
    }

    fn current_scene_title(&self) -> String {
        self.current_scene_descriptor().title
    }

    fn activate_current_scene_assets(&mut self) {
        let descriptor = self.current_scene_descriptor();

        if descriptor.scene != Scene::LoadedA3d {
            return;
        }

        let Some(root) = descriptor.a3d_root else {
            return;
        };

        if self
            .loaded_a3d_root
            .as_ref()
            .is_some_and(|active_root| active_root == &root)
        {
            return;
        }

        self.load_a3d_root(root);
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
            ControlMode::Camera => ControlMode::Light,
            ControlMode::Light => ControlMode::Scene,
        };

        self.push_debug_console_line(format!("control mode: {}", self.control_mode.label()));
    }

    fn set_control_mode(&mut self, control_mode: ControlMode) {
        self.control_mode = control_mode;
        self.push_debug_console_line(format!("control mode: {}", self.control_mode.label()));
    }

    fn control_mode_menu_index(&self) -> usize {
        match self.control_mode {
            ControlMode::Scene => 0,
            ControlMode::Camera => 1,
            ControlMode::Light => 2,
        }
    }

    fn open_menu(&mut self, kind: crate::menu::MenuKind) {
        let selected_index = match kind {
            crate::menu::MenuKind::Control => self.control_mode_menu_index(),
            _ => 0,
        };

        self.active_menu = Some(MenuState::with_selected(kind, selected_index));
    }

    fn toggle_menu_bar(&mut self) {
        if self.active_menu.is_some() {
            self.close_menu();
        } else {
            self.open_menu(crate::menu::MenuKind::File);
        }
    }

    fn open_menu_for_hotkey(&mut self, key_code: KeyCode) -> bool {
        let Some(kind) = menu_kind_for_hotkey(key_code) else {
            return false;
        };

        self.open_menu(kind);
        true
    }

    fn open_next_menu(&mut self) {
        let next_kind = self
            .active_menu
            .as_ref()
            .map(|menu| next_menu_kind(menu.kind()))
            .unwrap_or(crate::menu::MenuKind::File);

        self.active_menu = Some(MenuState::with_selected(next_kind, 0));
    }

    fn open_previous_menu(&mut self) {
        let previous_kind = self
            .active_menu
            .as_ref()
            .map(|menu| previous_menu_kind(menu.kind()))
            .unwrap_or(crate::menu::MenuKind::File);

        self.active_menu = Some(MenuState::with_selected(previous_kind, 0));
    }

    fn close_menu(&mut self) {
        self.active_menu = None;
    }

    fn open_scene_browser(&mut self) {
        self.scene_browser_selected = self.scene_position;
        self.scene_browser_open = true;
        self.close_menu();
    }

    fn close_scene_browser(&mut self) {
        self.scene_browser_open = false;
    }

    fn move_scene_browser_up(&mut self) {
        let count = crate::scenes::scene_count();

        if count == 0 {
            self.scene_browser_selected = 0;
        } else if self.scene_browser_selected == 0 {
            self.scene_browser_selected = count - 1;
        } else {
            self.scene_browser_selected -= 1;
        }
    }

    fn move_scene_browser_down(&mut self) {
        let count = crate::scenes::scene_count();

        if count == 0 {
            self.scene_browser_selected = 0;
        } else {
            self.scene_browser_selected = (self.scene_browser_selected + 1) % count;
        }
    }

    fn select_scene_browser_entry(&mut self) {
        if crate::scenes::scene_count() == 0 {
            self.close_scene_browser();
            return;
        }

        self.scene_position = self.scene_browser_selected;
        self.reset_animation();
        self.activate_current_scene_assets();
        self.close_scene_browser();
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

    fn move_world_origin(&mut self, delta: Vec3) {
        self.world_origin = Vec3::new(
            self.world_origin.x + delta.x,
            self.world_origin.y + delta.y,
            self.world_origin.z + delta.z,
        );

        self.push_debug_console_line(format!(
            "world origin: [{:.2}, {:.2}, {:.2}]",
            self.world_origin.x, self.world_origin.y, self.world_origin.z
        ));
    }

    fn reset_world_axes(&mut self) -> bool {
        self.world_origin = Vec3::zero();
        let rotated = self.reset_loaded_a3d_world_object();
        self.push_debug_console_line("world axes: reset origin and rotation".to_string());
        rotated
    }

    fn rotate_world_camera(&mut self, yaw_delta_degrees: f32, pitch_delta_degrees: f32) {
        self.world_camera_yaw_degrees += yaw_delta_degrees;
        self.world_camera_yaw_degrees %= FULL_ROTATION_DEGREES;

        self.world_camera_pitch_degrees =
            (self.world_camera_pitch_degrees + pitch_delta_degrees).clamp(-80.0, 80.0);
    }

    fn apply_xyz_control_event(&mut self, event: XyzControlEvent) -> bool {
        if self.current_scene() == Scene::LoadedA3d {
            match self.loaded_a3d_workspace.active_xyz_target().clone() {
                WorldEditorTarget::Object(id) => {
                    return self.apply_loaded_a3d_object_xyz_event(&id, event);
                }
                WorldEditorTarget::Camera => self.control_mode = ControlMode::Camera,
                WorldEditorTarget::SceneOrigin => self.control_mode = ControlMode::Scene,
            }
        }

        match self.control_mode {
            ControlMode::Scene => match event {
                XyzControlEvent::Rotate { axis, direction } => {
                    let delta = self.xyz_control.rotation_delta(axis, direction);
                    let handled = self.rotate_loaded_a3d_world_object(delta);
                    self.push_debug_console_line(format!(
                        "xyzcontrol/world: {} handled={handled}",
                        event.label()
                    ));
                    handled
                }
                XyzControlEvent::MoveOrigin { axis, direction } => {
                    let delta = self.xyz_control.origin_delta(axis, direction);
                    self.move_world_origin(delta);
                    self.push_debug_console_line(format!("xyzcontrol/world: {}", event.label()));
                    true
                }
                XyzControlEvent::Reset => self.reset_world_axes(),
            },
            ControlMode::Camera => match event {
                XyzControlEvent::Rotate { axis, direction } => {
                    let delta = self.xyz_control.rotation_delta(axis, direction);
                    let mut handled = false;

                    if delta.y != 0.0 {
                        self.rotate_world_camera(delta.y, 0.0);
                        handled = true;
                    }

                    if delta.x != 0.0 {
                        self.rotate_world_camera(0.0, delta.x);
                        handled = true;
                    }

                    if delta.z != 0.0 {
                        self.push_debug_console_line(format!(
                            "xyzcontrol/camera: {} roll pending",
                            event.label()
                        ));
                        return true;
                    }

                    self.push_debug_console_line(format!(
                        "xyzcontrol/camera: {} handled={handled}",
                        event.label()
                    ));
                    handled
                }
                XyzControlEvent::MoveOrigin { axis, direction } => {
                    let delta = self.xyz_control.origin_delta(axis, direction);

                    if delta.x != 0.0 {
                        self.move_world_camera_right(delta.x);
                    }

                    if delta.y != 0.0 {
                        self.move_world_camera_up(delta.y);
                    }

                    if delta.z != 0.0 {
                        self.move_world_camera_forward(-delta.z);
                    }

                    self.push_debug_console_line(format!(
                        "xyzcontrol/camera: {} pos=[{:.2},{:.2},{:.2}]",
                        event.label(),
                        self.world_camera_position.x,
                        self.world_camera_position.y,
                        self.world_camera_position.z,
                    ));
                    true
                }
                XyzControlEvent::Reset => {
                    self.reset_world_camera();
                    self.push_debug_console_line("xyzcontrol/camera: reset".to_string());
                    true
                }
            },
            ControlMode::Light => match event {
                XyzControlEvent::Rotate { axis, direction } => {
                    let delta = self.xyz_control.rotation_delta(axis, direction);
                    let handled = self.rotate_loaded_a3d_light_direction(delta);

                    self.push_debug_console_line(format!(
                        "xyzcontrol/light: {} direction handled={handled}",
                        event.label()
                    ));
                    handled
                }
                XyzControlEvent::MoveOrigin { axis, direction } => {
                    let delta = self.xyz_control.origin_delta(axis, direction);
                    let mut handled = true;

                    if delta.x != 0.0 {
                        handled &= self.move_loaded_a3d_light_right(delta.x);
                    }

                    if delta.y != 0.0 {
                        handled &= self.move_loaded_a3d_light_up(delta.y);
                    }

                    if delta.z != 0.0 {
                        handled &= self.move_loaded_a3d_light_forward(-delta.z);
                    }

                    self.push_debug_console_line(format!(
                        "xyzcontrol/light: {} handled={handled}",
                        event.label()
                    ));
                    handled
                }
                XyzControlEvent::Reset => {
                    let handled = self.reset_loaded_a3d_light();
                    self.push_debug_console_line(format!(
                        "xyzcontrol/light: reset handled={handled}"
                    ));
                    handled
                }
            },
        }
    }

    fn reset_active_control(&mut self) -> bool {
        match self.control_mode {
            ControlMode::Scene => self.reset_world_axes(),
            ControlMode::Camera => {
                self.reset_world_camera();
                true
            }
            ControlMode::Light => self.reset_loaded_a3d_light(),
        }
    }

    fn loaded_a3d_manifest_path_for_edit(&self) -> Option<PathBuf> {
        self.loaded_a3d_manifest_path.clone().or_else(|| {
            self.loaded_a3d_root
                .as_ref()
                .map(|root| root.join("scene.a3d"))
        })
    }

    fn edit_loaded_a3d_manifest<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce(&mut serde_json::Value) -> bool,
    {
        let Some(manifest_path) = self.loaded_a3d_manifest_path_for_edit() else {
            return false;
        };

        let Ok(source) = std::fs::read_to_string(&manifest_path) else {
            return false;
        };

        let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&source) else {
            return false;
        };

        if !edit(&mut json) {
            return false;
        }

        let serialized =
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| source.clone()) + "\n";

        if std::fs::write(&manifest_path, serialized).is_err() {
            return false;
        }

        // World and light controls edit the active .a3d manifest on disk.
        // Reload immediately so the cached LoadedWorld and rendered objects
        // reflect the new transform/light data on the next draw.
        self.load_a3d_file(manifest_path);

        true
    }

    fn edit_first_loaded_a3d_light_position<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce(Vec3) -> Vec3,
    {
        let Some(light) = self.loaded_a3d_lights.first_mut() else {
            return false;
        };
        light.position = edit(light.position);
        true
    }

    fn edit_first_loaded_a3d_light_direction<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce(Vec3) -> Vec3,
    {
        let Some(light) = self.loaded_a3d_lights.first_mut() else {
            return false;
        };
        light.direction = edit(light.direction);
        true
    }

    fn rotate_loaded_a3d_light_direction(&mut self, delta: Vec3) -> bool {
        self.edit_first_loaded_a3d_light_direction(|current| {
            current
                .rotate_x(delta.x.to_radians())
                .rotate_y(delta.y.to_radians())
                .rotate_z(delta.z.to_radians())
        })
    }

    fn move_loaded_a3d_light(&mut self, delta: Vec3) -> bool {
        self.edit_first_loaded_a3d_light_position(|current| {
            Vec3::new(
                current.x + delta.x,
                current.y + delta.y,
                current.z + delta.z,
            )
        })
    }

    fn reset_loaded_a3d_light(&mut self) -> bool {
        self.edit_first_loaded_a3d_light_position(|_| Vec3::new(5.0, 2.0, -2.5))
    }

    fn move_loaded_a3d_light_forward(&mut self, amount: f32) -> bool {
        self.move_loaded_a3d_light(Vec3::new(0.0, 0.0, -amount))
    }

    fn move_loaded_a3d_light_right(&mut self, amount: f32) -> bool {
        self.move_loaded_a3d_light(Vec3::new(amount, 0.0, 0.0))
    }

    fn move_loaded_a3d_light_up(&mut self, amount: f32) -> bool {
        self.move_loaded_a3d_light(Vec3::new(0.0, amount, 0.0))
    }

    fn edit_first_loaded_a3d_object_rotation<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce([f32; 3]) -> [f32; 3],
    {
        let Some(world) = self.loaded_a3d_world.as_mut() else {
            return false;
        };
        let Some(object) = world
            .objects
            .iter_mut()
            .find(|object| !object.editor_hidden)
        else {
            return false;
        };

        object.transform.rotation_degrees = edit(object.transform.rotation_degrees);
        world.rebuild_parent_matrices();
        true
    }

    fn rotate_loaded_a3d_world_object(&mut self, delta: Vec3) -> bool {
        self.edit_first_loaded_a3d_object_rotation(|current| {
            [
                current[0] + delta.x,
                current[1] + delta.y,
                current[2] + delta.z,
            ]
        })
    }

    fn reset_loaded_a3d_world_object(&mut self) -> bool {
        self.edit_first_loaded_a3d_object_rotation(|_| [0.0, 0.0, 0.0])
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
        let manifest_path = manifest_path_from_selection(manifest_path);
        let (camera_cell_aspect_ratio, camera_perspective_scale, camera_aspect_ratio) =
            read_loaded_a3d_viewport_settings(&manifest_path);

        let Some(root) = manifest_path.parent().map(Path::to_path_buf) else {
            self.loaded_a3d_error = Some("selected .a3d file has no parent folder".to_string());
            self.loaded_a3d_debug_popup_until = Some(Instant::now() + Duration::from_secs(5));
            return;
        };

        match load_a3d_project(&manifest_path).and_then(|project| {
            let camera = project.manifest.camera;

            project
                .into_world()
                .map(|world| (world, camera))
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        }) {
            Ok((world, camera)) => {
                self.loaded_a3d_workspace.sync_objects(
                    world
                        .objects
                        .iter()
                        .filter(|object| !object.editor_hidden)
                        .map(|object| (object.id.clone(), object.render.visible)),
                );
                if let Some(camera) = camera {
                    let position =
                        Vec3::new(camera.position[0], camera.position[1], camera.position[2]);
                    let target = Vec3::new(camera.target[0], camera.target[1], camera.target[2]);
                    let (yaw, pitch) = yaw_pitch_toward(position, target);

                    self.world_camera_position = position;
                    self.world_camera_yaw_degrees = yaw;
                    self.world_camera_pitch_degrees = pitch;
                }

                let registry = crate::scenes::registry();
                let selected_scene = registry
                    .iter()
                    .position(|descriptor| {
                        descriptor.a3d_root.as_ref().is_some_and(|registered_root| {
                            fs::canonicalize(registered_root).ok() == fs::canonicalize(&root).ok()
                        })
                    })
                    .or_else(|| {
                        registry
                            .iter()
                            .position(|descriptor| descriptor.id == "loaded_a3d")
                    });

                if let Some(scene_position) = selected_scene {
                    self.scene_position = scene_position;
                    self.scene_browser_selected = scene_position;
                }

                if let Err(error) = write_a3d_profile_manifest(&manifest_path) {
                    self.push_debug_console_line(format!("A3D profile save failed: {error}"));
                }

                self.loaded_a3d_lights = loaded_a3d_lights(&root).unwrap_or_default();
                self.loaded_a3d_camera_cell_aspect_ratio = camera_cell_aspect_ratio;
                self.loaded_a3d_camera_perspective_scale = camera_perspective_scale;
                self.loaded_a3d_camera_aspect_ratio = camera_aspect_ratio;
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
        if self.show_debug_console {
            self.push_debug_console_line("world debug: reloaded active .a3d world".to_string());
            self.push_world_debug_lines();
        }
    }

    fn scale_active_loaded_a3d_object(&mut self, factor: f32) -> bool {
        let WorldEditorTarget::Object(target_id) =
            self.loaded_a3d_workspace.active_xyz_target().clone()
        else {
            return false;
        };

        let Some(world) = self.loaded_a3d_world.as_mut() else {
            return false;
        };

        if !world.scale_object_uniform(&target_id, factor) {
            return false;
        }

        self.push_debug_console_line(format!("world editor: scaled {target_id} by {factor:.4}"));

        true
    }

    fn loaded_a3d_editor_items(&self) -> Vec<ascii_3d::editor_ui::EditorItem> {
        loaded_a3d_editor_items(
            self.loaded_a3d_workspace.entries(),
            self.loaded_a3d_world.as_ref(),
        )
    }

    fn sync_loaded_a3d_editor_objects(&mut self) {
        let Some(world) = self.loaded_a3d_world.as_ref() else {
            return;
        };
        self.loaded_a3d_workspace.sync_objects(
            world
                .objects
                .iter()
                .filter(|object| !object.editor_hidden)
                .map(|object| (object.id.clone(), object.render.visible)),
        );
        let items = self.loaded_a3d_editor_items();
        self.loaded_a3d_hierarchy.replace_items(&items);
    }

    fn toggle_loaded_a3d_visibility(&mut self, target_id: &str) -> bool {
        let Some(world) = self.loaded_a3d_world.as_mut() else {
            return false;
        };
        if world.toggle_object_visibility(target_id).is_none() {
            return false;
        }
        self.sync_loaded_a3d_editor_objects();
        true
    }

    fn reset_loaded_a3d_editor_target(&mut self, target: &WorldEditorTarget) -> bool {
        match target {
            WorldEditorTarget::Camera => {
                self.reset_world_camera();
                true
            }
            WorldEditorTarget::SceneOrigin => self.reset_world_axes(),
            WorldEditorTarget::Object(id) => self
                .loaded_a3d_world
                .as_mut()
                .is_some_and(|world| world.reset_object_transform(id)),
        }
    }

    fn apply_loaded_a3d_object_xyz_event(
        &mut self,
        target_id: &str,
        event: XyzControlEvent,
    ) -> bool {
        let Some(world) = self.loaded_a3d_world.as_mut() else {
            return false;
        };
        match event {
            XyzControlEvent::Rotate { axis, direction } => {
                let delta = self.xyz_control.rotation_delta(axis, direction);
                world.rotate_object(target_id, [delta.x, delta.y, delta.z])
            }
            XyzControlEvent::MoveOrigin { axis, direction } => {
                let delta = self.xyz_control.origin_delta(axis, direction);
                world.translate_object(target_id, [delta.x, delta.y, delta.z])
            }
            XyzControlEvent::Reset => world.reset_object_transform(target_id),
        }
    }

    fn loaded_a3d_has_enabled_rotation_behavior(&self) -> bool {
        self.loaded_a3d_world.as_ref().is_some_and(|world| {
            world
                .objects
                .iter()
                .any(|object| !object.behaviors.is_empty())
        })
    }

    fn update(&mut self, elapsed: Duration) -> bool {
        let delta_degrees = elapsed.as_secs_f32() * ROTATION_SPEED_DEGREES_PER_SECOND;
        let pulsed_delta_degrees =
            pulsed_rotation_delta_degrees(self.animation_angle_degrees, elapsed);
        let pulsed_box_delta_degrees =
            pulsed_rotation_delta_degrees(self.box_angle_degrees, elapsed);

        match self.current_scene() {
            Scene::LoadedA3d => {
                if self.loaded_a3d_has_enabled_rotation_behavior() {
                    if let Some(world) = &mut self.loaded_a3d_world {
                        world.update(elapsed.as_secs_f32());
                    }
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
    logo_quads_scene_config: MultiQuadSceneConfig,
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

fn initial_a3d_manifest_path() -> PathBuf {
    if let Some(argument) = std::env::args_os().nth(1) {
        return manifest_path_from_selection(PathBuf::from(argument));
    }

    if let Some(profile_manifest) = read_a3d_profile_manifest() {
        if profile_manifest.is_file() {
            return profile_manifest;
        }
    }

    crate::scenes::registry()
        .into_iter()
        .find_map(|descriptor| descriptor.a3d_root)
        .map(|root| root.join("scene.a3d"))
        .unwrap_or_else(|| default_a3d_root_path().join("scene.a3d"))
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
    let logo_quads_scene_config =
        load_multi_quad_scene_config(asset_path(KM_LOGO_QUADS_SCENE_ASSET))?;

    if logo_quads_scene_config.mesh_asset != "models/quad4.obj" {
        return Err(io::Error::other(format!(
            "km_logo_quads.scene.json references unexpected mesh asset '{}'",
            logo_quads_scene_config.mesh_asset,
        )));
    }

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
        logo_quads_scene_config,
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

fn project_camera_space_to_viewport(
    camera_space: Vec3,
    inner: ClipRect,
    cell_aspect_ratio: f32,
    perspective_scale: f32,
) -> Option<Point2> {
    // Mat4::look_at uses the conventional right-handed camera space:
    // +X = camera right, +Y = camera up, and camera forward points along -Z.
    if camera_space.z >= -0.01 {
        return None;
    }

    let center_x = inner.x + inner.width as i32 / 2;
    let center_y = inner.y + inner.height as i32 / 2;
    let depth = -camera_space.z;

    let perspective = perspective_scale / depth;
    let screen_x = center_x + (camera_space.x * perspective).round() as i32;
    let screen_y = center_y - (camera_space.y * perspective / cell_aspect_ratio).round() as i32;

    Some(Point2::new(screen_x, screen_y))
}

fn project_camera_space_to_viewport_with_depth(
    camera_space: Vec3,
    inner: ClipRect,
    cell_aspect_ratio: f32,
    perspective_scale: f32,
) -> Option<(Point2, f32)> {
    let point = project_camera_space_to_viewport(
        camera_space,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    )?;
    let depth = -camera_space.z;

    Some((point, depth))
}

fn camera_viewport_cell_aspect_ratio(state: &AppState) -> f32 {
    state.loaded_a3d_camera_cell_aspect_ratio
}

fn camera_viewport_perspective_scale(state: &AppState) -> f32 {
    state.loaded_a3d_camera_perspective_scale
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CameraViewportAspectRatio {
    width: u16,
    height: u16,
}

impl CameraViewportAspectRatio {
    const DEFAULT: Self = Self {
        width: 16,
        height: 9,
    };

    const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

fn parse_camera_viewport_aspect_ratio(
    value: &serde_json::Value,
) -> Option<CameraViewportAspectRatio> {
    if let Some(text) = value.as_str() {
        let (width, height) = text.split_once(':')?;
        let width = width.trim().parse::<u16>().ok()?;
        let height = height.trim().parse::<u16>().ok()?;

        return (width > 0 && height > 0).then_some(CameraViewportAspectRatio::new(width, height));
    }

    let width = value
        .get("width")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())?;
    let height = value
        .get("height")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())?;

    (width > 0 && height > 0).then_some(CameraViewportAspectRatio::new(width, height))
}

fn loaded_a3d_camera_view_aspect_ratio(state: &AppState) -> CameraViewportAspectRatio {
    state.loaded_a3d_camera_aspect_ratio
}

fn read_loaded_a3d_viewport_settings(
    manifest_path: &Path,
) -> (f32, f32, CameraViewportAspectRatio) {
    let defaults = (
        DEFAULT_CAMERA_VIEWPORT_CELL_ASPECT_RATIO,
        DEFAULT_CAMERA_VIEWPORT_PERSPECTIVE_SCALE,
        CameraViewportAspectRatio::DEFAULT,
    );

    let Ok(source) = std::fs::read_to_string(manifest_path) else {
        return defaults;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&source) else {
        return defaults;
    };

    let camera_view = json
        .get("viewport")
        .and_then(|viewport| viewport.get("camera_view"));

    let cell_aspect_ratio = camera_view
        .and_then(|view| view.get("cell_aspect_ratio"))
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(defaults.0);

    let perspective_scale = camera_view
        .and_then(|view| view.get("perspective_scale"))
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(defaults.1);

    let aspect_ratio = camera_view
        .and_then(|view| view.get("aspect_ratio"))
        .and_then(parse_camera_viewport_aspect_ratio)
        .unwrap_or(defaults.2);

    (cell_aspect_ratio, perspective_scale, aspect_ratio)
}

fn fit_aspect_dimensions(
    available_width: u16,
    available_height: u16,
    aspect: CameraViewportAspectRatio,
    cell_aspect_ratio: f32,
) -> (usize, usize) {
    if available_width == 0 || available_height == 0 || aspect.width == 0 || aspect.height == 0 {
        return (0, 0);
    }

    let cell_aspect_ratio = if cell_aspect_ratio.is_finite() && cell_aspect_ratio > 0.0 {
        cell_aspect_ratio
    } else {
        1.0
    };

    // Terminal cells are not square. To make the viewport look visually like
    // 16:9 or 4:3, the cell width budget must be widened by the cell aspect.
    let visual_width_ratio = aspect.width as f32 * cell_aspect_ratio;
    let visual_height_ratio = aspect.height as f32;

    let width_limited_height =
        (available_width as f32 * visual_height_ratio / visual_width_ratio).floor() as u16;

    if width_limited_height <= available_height {
        return (
            available_width as usize,
            width_limited_height.max(1) as usize,
        );
    }

    let height_limited_width =
        (available_height as f32 * visual_width_ratio / visual_height_ratio).floor() as u16;

    (
        height_limited_width.max(1) as usize,
        available_height as usize,
    )
}

fn camera_viewport_canvas_size(
    state: &AppState,
    terminal_width: u16,
    terminal_height: u16,
) -> (usize, usize) {
    let aspect = loaded_a3d_camera_view_aspect_ratio(state);
    let cell_aspect_ratio = camera_viewport_cell_aspect_ratio(state);

    // The camera viewport lives in the bottom third of the app content area.
    // The world/debug scene keeps the remaining two thirds.
    //
    // terminal_height includes the menu row, and the Ratatui block adds a
    // one-cell border around the camera canvas.
    let app_content_height = terminal_height.saturating_sub(1).max(1);
    let camera_panel_height = (app_content_height / 3).max(8);
    let available_width = terminal_width.saturating_sub(4).max(8);
    let available_height = camera_panel_height.saturating_sub(2).max(6);

    fit_aspect_dimensions(available_width, available_height, aspect, cell_aspect_ratio)
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

    let cell_aspect_ratio = camera_viewport_cell_aspect_ratio(state);
    let perspective_scale = camera_viewport_perspective_scale(state);

    let Some((from_screen, from_depth)) = project_camera_space_to_viewport_with_depth(
        from_camera,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    ) else {
        return;
    };
    let Some((to_screen, to_depth)) = project_camera_space_to_viewport_with_depth(
        to_camera,
        inner,
        cell_aspect_ratio,
        perspective_scale,
    ) else {
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

fn render_camera_viewport_canvas(
    state: &AppState,
    viewport_width: usize,
    viewport_height: usize,
) -> io::Result<Canvas> {
    let viewport_width = viewport_width.max(8);
    let viewport_height = viewport_height.max(6);

    let mut canvas = Canvas::new(viewport_width, viewport_height);

    canvas.draw_text(Point2::new(1, 0), "Camera3D viewport content");

    let inner = ClipRect {
        x: 1,
        y: 2,
        width: viewport_width.saturating_sub(2),
        height: viewport_height.saturating_sub(5),
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
        Point2::new(1, viewport_height as i32 - 2),
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

fn loaded_a3d_object_render_usize(
    root: &Path,
    object_id: &str,
    key: &str,
    default_value: usize,
) -> usize {
    let scene_path = root.join("scene.a3d");
    let Ok(source) = std::fs::read_to_string(&scene_path) else {
        return default_value;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&source) else {
        return default_value;
    };

    let Some(objects) = json.get("objects").and_then(serde_json::Value::as_array) else {
        return default_value;
    };

    let Some(value) = objects
        .iter()
        .find(|entry| {
            entry
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| id == object_id)
        })
        .and_then(|entry| entry.get("render"))
        .and_then(|render| render.get(key))
        .and_then(serde_json::Value::as_u64)
    else {
        return default_value;
    };

    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

fn loaded_a3d_object_edge_stride(_root: &Path, object: &crate::a3d::SceneObject) -> usize {
    object.render.edge_stride.max(1)
}

fn loaded_a3d_object_ascii_simplify(
    _root: &Path,
    object: &crate::a3d::SceneObject,
) -> MeshPrepareOptions {
    object
        .render
        .ascii_simplify
        .as_ref()
        .filter(|config| config.enabled)
        .map(|config| MeshPrepareOptions {
            normalize_to_size: Some(1.0),
            grid_size: (config.grid_size.is_finite() && config.grid_size > 0.0)
                .then_some(config.grid_size),
            target_vertices: config.target_vertices.filter(|value| *value > 0),
            cache: config.cache,
        })
        .unwrap_or(MeshPrepareOptions {
            normalize_to_size: Some(1.0),
            ..MeshPrepareOptions::default()
        })
}

fn load_loaded_a3d_mesh(
    root: &Path,
    relative_path: &str,
    object: &crate::a3d::SceneObject,
) -> io::Result<Arc<ascii_3d::mesh::Mesh>> {
    let mesh_path = resolve_a3d_asset_path(root, relative_path)?;
    load_prepared_mesh(&mesh_path, loaded_a3d_object_ascii_simplify(root, object))
}

fn draw_loaded_a3d_mesh_object_in_ws(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    root: &Path,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::Mesh { path } = &object.asset else {
        return Ok(());
    };

    let mesh = load_loaded_a3d_mesh(root, path, object)?;
    let object_world = object.world_matrix();

    for (from_index, to_index) in mesh.unique_edges() {
        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        canvas.draw_line(
            projector.project(from_world),
            projector.project(to_world),
            '#',
        );
    }

    Ok(())
}

fn draw_loaded_a3d_light_gizmos(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    _root: &Path,
    inner: ClipRect,
) -> io::Result<()> {
    for light in &state.loaded_a3d_lights {
        if !light.gizmo.visible {
            continue;
        }

        let source = light.position;

        let Some(gizmo_direction) = normalized_light_direction(light.direction) else {
            continue;
        };

        let tip = source + gizmo_direction * light.gizmo.length;

        draw_camera_viewport_depth_line(
            canvas,
            depth_buffer,
            state,
            inner,
            source,
            tip,
            light.gizmo.ray_character,
        );

        let Some(source_camera) = world_to_camera_space(state, source) else {
            continue;
        };

        let Some((source_screen, source_depth)) = project_camera_space_to_viewport_with_depth(
            source_camera,
            inner,
            camera_viewport_cell_aspect_ratio(state),
            camera_viewport_perspective_scale(state),
        ) else {
            continue;
        };

        if depth_buffer.try_update(source_screen, source_depth) {
            canvas.set(source_screen, light.gizmo.source_character);
        } else {
            canvas.set(source_screen, light.gizmo.source_character);
        }
    }

    Ok(())
}

fn draw_loaded_a3d_light_gizmos_in_ws(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    state: &AppState,
) -> io::Result<()> {
    let visual_length_cells = 8.0;

    for light in &state.loaded_a3d_lights {
        if !light.gizmo.visible {
            continue;
        }

        // Keep the worldspace light gizmo fixed in screen-cell length, but aim
        // it using the actual A3D light direction so XyzControl light rotation
        // is visible immediately.
        let source = light.position;
        let source_screen = projector.project(source);

        let Some(gizmo_direction) = normalized_light_direction(light.direction) else {
            canvas.set(source_screen, light.gizmo.source_character);
            continue;
        };

        let direction_tip_screen = projector.project(source + gizmo_direction);
        let dx = (direction_tip_screen.x - source_screen.x) as f32;
        let dy = (direction_tip_screen.y - source_screen.y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance <= f32::EPSILON {
            canvas.set(source_screen, light.gizmo.source_character);
            continue;
        }

        let step_x = dx / distance;
        let step_y = dy / distance;

        let tip = Point2::new(
            source_screen.x + (step_x * visual_length_cells).round() as i32,
            source_screen.y + (step_y * visual_length_cells).round() as i32,
        );

        canvas.draw_line(source_screen, tip, light.gizmo.ray_character);

        // Draw source after the ray so the L marker wins visually.
        canvas.set(source_screen, light.gizmo.source_character);
    }

    Ok(())
}

fn loaded_a3d_object_render_mode(_root: &Path, object: &crate::a3d::SceneObject) -> Option<String> {
    object.render.mode.clone()
}

fn loaded_a3d_object_mesh_path(_root: &Path, object: &crate::a3d::SceneObject) -> Option<String> {
    match &object.asset {
        AssetRef::Mesh { path } => Some(path.clone()),
        _ => None,
    }
}

fn dot_vec3(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross_vec3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn normalize_vec3(value: Vec3) -> Option<Vec3> {
    let length = dot_vec3(value, value).sqrt();

    if length <= f32::EPSILON {
        return None;
    }

    Some(value * (1.0 / length))
}

fn shade_character_for_brightness(brightness: f32) -> char {
    const RAMP: &[u8] = b" .,-~:;=!*#$@";

    let brightness = brightness.clamp(0.0, 1.0);
    let index = (brightness * (RAMP.len().saturating_sub(1)) as f32).round() as usize;

    RAMP[index.min(RAMP.len().saturating_sub(1))] as char
}

fn edge_function(a: Point2, b: Point2, point: Point2) -> f32 {
    ((point.x - a.x) as f32 * (b.y - a.y) as f32) - ((point.y - a.y) as f32 * (b.x - a.x) as f32)
}

fn draw_loaded_a3d_mesh_object_raster(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    root: &Path,
    inner: ClipRect,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    let Some(path) = loaded_a3d_object_mesh_path(root, object) else {
        return Ok(());
    };

    let mesh = load_loaded_a3d_mesh(root, &path, object)?;
    let object_world = object.world_matrix();
    let light_direction = state
        .loaded_a3d_lights
        .iter()
        .find(|light| light.intensity > 0.0)
        .and_then(|light| normalized_light_direction(light.direction))
        .unwrap_or_else(|| Vec3::new(-1.0, -1.0, -1.0));
    let light_to_surface = light_direction * -1.0;

    let prepared = prepare_frame_mesh(
        &mesh,
        |position| {
            let world =
                object_world.transform_point(Vec3::new(position[0], position[1], position[2]));
            [world.x, world.y, world.z]
        },
        |world| {
            world_to_camera_space(state, Vec3::new(world[0], world[1], world[2]))
                .map(|camera| [camera.x, camera.y, camera.z])
        },
        |camera| {
            project_camera_space_to_viewport_with_depth(
                Vec3::new(camera[0], camera[1], camera[2]),
                inner,
                camera_viewport_cell_aspect_ratio(state),
                camera_viewport_perspective_scale(state),
            )
            .map(|(point, depth)| (point.x, point.y, depth))
        },
    );

    visit_prepared_triangles(&mesh, &prepared, object.render.backface_cull, |triangle| {
        let normal = Vec3::new(
            triangle.world_normal[0],
            triangle.world_normal[1],
            triangle.world_normal[2],
        );
        let diffuse = dot_vec3(normal, light_to_surface).max(0.0);
        let brightness = (0.18 + diffuse * 0.82).clamp(0.0, 1.0);
        let character = shade_character_for_brightness(brightness);

        rasterize_triangle_clipped(
            inner.width,
            inner.height,
            triangle.screen[0],
            triangle.screen[1],
            triangle.screen[2],
            |x, y, depth| {
                let point = Point2::new(x, y);
                if depth_buffer.try_update(point, depth) {
                    canvas.set(point, character);
                }
            },
        );
    });

    Ok(())
}

fn load_loaded_a3d_map(root: &Path, relative_path: &str) -> io::Result<Arc<GeoJsonMapAsset>> {
    let map_path = resolve_a3d_asset_path(root, relative_path)?;

    if let Some(map) = LOADED_A3D_MAP_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| io::Error::other("A3D map cache lock poisoned"))?
        .get(&map_path)
        .cloned()
    {
        return Ok(map);
    }

    let map = Arc::new(load_geojson_map_asset(Path::new(&map_path))?);
    LOADED_A3D_MAP_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| io::Error::other("A3D map cache lock poisoned"))?
        .insert(map_path, Arc::clone(&map));

    Ok(map)
}

static LOADED_A3D_MAP_CACHE: OnceLock<Mutex<HashMap<String, Arc<GeoJsonMapAsset>>>> =
    OnceLock::new();

fn draw_loaded_a3d_geo_json_map_object(
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

    let AssetRef::GeoJsonMap { path, radius_scale } = &object.asset else {
        return Ok(());
    };

    let map = load_loaded_a3d_map(root, path)?;
    let object_world = object.world_matrix();
    let object_center_world = object_world.transform_point(Vec3::new(0.0, 0.0, 0.0));
    let object_center_camera = world_to_camera_space(state, object_center_world);
    let character = object.render.stroke_character.unwrap_or('*');

    for line in &map.lines {
        for pair in line.points_lon_lat.windows(2) {
            let (lon_a, lat_a) = pair[0];
            let (lon_b, lat_b) = pair[1];
            let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);
            let mut previous_world = None;

            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let lon = lerp_angle_degrees(lon_a, lon_b, t);
                let lat = lat_a + (lat_b - lat_a) * t;
                let point = lon_lat_to_sphere(lon, lat, *radius_scale);
                let world = object_world.transform_point(Vec3::new(point.x, point.y, point.z));

                let front_facing = match (object_center_camera, world_to_camera_space(state, world))
                {
                    (Some(center_camera), Some(point_camera)) => {
                        let outward = point_camera - center_camera;
                        let toward_camera = point_camera * -1.0;
                        dot_vec3(outward, toward_camera) > 0.0
                    }
                    _ => false,
                };

                if !front_facing {
                    previous_world = None;
                    continue;
                }

                if let Some(previous) = previous_world {
                    draw_camera_viewport_depth_line(
                        canvas,
                        depth_buffer,
                        state,
                        inner,
                        previous,
                        world,
                        character,
                    );
                }

                previous_world = Some(world);
            }
        }
    }

    Ok(())
}

fn draw_loaded_a3d_geo_json_map_object_in_ws(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    root: &Path,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::GeoJsonMap { path, radius_scale } = &object.asset else {
        return Ok(());
    };

    let map = load_loaded_a3d_map(root, path)?;
    let object_world = object.world_matrix();
    let character = object.render.stroke_character.unwrap_or('*');

    for line in &map.lines {
        for pair in line.points_lon_lat.windows(2) {
            let (lon_a, lat_a) = pair[0];
            let (lon_b, lat_b) = pair[1];
            let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);
            let mut previous = None;

            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let lon = lerp_angle_degrees(lon_a, lon_b, t);
                let lat = lat_a + (lat_b - lat_a) * t;
                let point = lon_lat_to_sphere(lon, lat, *radius_scale);
                let world = object_world.transform_point(Vec3::new(point.x, point.y, point.z));
                let projected = projector.project(world);

                if let Some(previous) = previous {
                    canvas.draw_line(previous, projected, character);
                }

                previous = Some(projected);
            }
        }
    }

    Ok(())
}

fn draw_loaded_a3d_mesh_object(
    canvas: &mut Canvas,
    depth_buffer: &mut CameraViewportDepthBuffer,
    state: &AppState,
    root: &Path,
    inner: ClipRect,
    object: &crate::a3d::SceneObject,
) -> io::Result<()> {
    if loaded_a3d_object_render_mode(root, object).as_deref() == Some("ascii_raster") {
        return draw_loaded_a3d_mesh_object_raster(
            canvas,
            depth_buffer,
            state,
            root,
            inner,
            object,
        );
    }

    if !object.render.visible {
        return Ok(());
    }

    let AssetRef::Mesh { path } = &object.asset else {
        return Ok(());
    };

    let mesh = load_loaded_a3d_mesh(root, path, object)?;
    let object_world = object.world_matrix();
    let character = object.render.stroke_character.unwrap_or('#');
    let edge_stride = loaded_a3d_object_edge_stride(root, object);

    for (edge_index, (from_index, to_index)) in mesh.unique_edges().into_iter().enumerate() {
        if edge_index % edge_stride != 0 {
            continue;
        }

        let from_world = object_world.transform_point(mesh.vertices[from_index]);
        let to_world = object_world.transform_point(mesh.vertices[to_index]);

        draw_camera_viewport_depth_line(
            canvas,
            depth_buffer,
            state,
            inner,
            from_world,
            to_world,
            character,
        );
    }

    Ok(())
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
    let object_world = object.world_matrix();
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
        "Rendering Word and Mesh assets from the loaded .a3d scene",
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
            if !world.object_effectively_visible(&object.id) {
                continue;
            }
            draw_loaded_a3d_word_object(canvas, &mut depth_buffer, state, root, inner, object)?;
            draw_loaded_a3d_mesh_object(canvas, &mut depth_buffer, state, root, inner, object)?;
            draw_loaded_a3d_geo_json_map_object(
                canvas,
                &mut depth_buffer,
                state,
                root,
                inner,
                object,
            )?;
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
    let object_world = object.world_matrix();
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
        if !world.object_effectively_visible(&object.id) {
            continue;
        }
        draw_loaded_a3d_word_object_in_ws(canvas, projector, state, root, object)?;
        draw_loaded_a3d_mesh_object_in_ws(canvas, projector, root, object)?;
        draw_loaded_a3d_geo_json_map_object_in_ws(canvas, projector, root, object)?;
    }

    draw_loaded_a3d_light_gizmos_in_ws(canvas, projector, state)?;

    Ok(())
}

fn render_loaded_a3d_camera_viewport_canvas(
    state: &AppState,
    viewport_width: usize,
    viewport_height: usize,
) -> io::Result<Canvas> {
    let viewport_width = viewport_width.max(8);
    let viewport_height = viewport_height.max(6);

    let mut canvas = Canvas::new(viewport_width, viewport_height);

    canvas.draw_text(Point2::new(1, 0), "LoadedA3d Camera3D viewport");

    let inner = ClipRect {
        x: 1,
        y: 2,
        width: viewport_width.saturating_sub(2),
        height: viewport_height.saturating_sub(5),
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
            draw_loaded_a3d_mesh_object(canvas, &mut depth_buffer, state, root, inner, object)?;
            draw_loaded_a3d_geo_json_map_object(
                canvas,
                &mut depth_buffer,
                state,
                root,
                inner,
                object,
            )?;
        }

        Ok::<(), io::Error>(())
    })?;

    canvas.draw_text(
        Point2::new(1, viewport_height as i32 - 2),
        &format!(
            "{} | viewport {}x{} aspect {}:{} cell {:.2} | pos [{:.2},{:.2},{:.2}]",
            world.title,
            viewport_width,
            viewport_height,
            loaded_a3d_camera_view_aspect_ratio(state).width,
            loaded_a3d_camera_view_aspect_ratio(state).height,
            camera_viewport_cell_aspect_ratio(state),
            state.world_camera_position.x,
            state.world_camera_position.y,
            state.world_camera_position.z,
        ),
    );

    Ok(canvas)
}

fn render_loaded_a3d_ws_camera_workspace(
    canvas: &mut Canvas,
    state: &AppState,
    projector: &ObliqueProjector,
) {
    let origin = state.world_origin;
    let positive_x = Vec3::new(origin.x + 4.0, origin.y, origin.z);
    let positive_y = Vec3::new(origin.x, origin.y + 3.0, origin.z);
    let negative_z = Vec3::new(origin.x, origin.y, origin.z - 4.0);

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
}

fn render_loaded_a3d_studio_world(
    canvas: &mut Canvas,
    state: &AppState,
    projector: &ObliqueProjector,
    world_debug_viewport: ClipRect,
) -> io::Result<()> {
    canvas.with_viewport(world_debug_viewport, |canvas| {
        render_loaded_a3d_ws_camera_workspace(canvas, state, projector);
        draw_loaded_a3d_objects_in_ws(canvas, state, projector)
    })?;

    Ok(())
}

fn render_scene_frame(
    state: &AppState,
    assets: &SceneAssets,
    viewport_width: usize,
    viewport_height: usize,
) -> io::Result<Canvas> {
    let viewport_width = viewport_width.max(CANVAS_WIDTH);
    let viewport_height = viewport_height.max(CANVAS_HEIGHT);
    let mut canvas = Canvas::new(viewport_width, viewport_height);
    let projector = projector_from_config(&assets.projection_config);
    let world_debug_viewport = ClipRect {
        x: WORLD_DEBUG_VIEWPORT.x,
        y: WORLD_DEBUG_VIEWPORT.y,
        width: viewport_width,
        height: WORLD_DEBUG_VIEWPORT.height,
    };

    match state.current_scene() {
        Scene::LoadedA3d => {
            render_loaded_a3d_studio_world(&mut canvas, state, &projector, world_debug_viewport)?;
        }

        Scene::LogoQuads => {
            render_logo_quads(
                &mut canvas,
                &projector,
                &assets.quad4_mesh,
                &assets.logo_quads_scene_config,
                state.animation_angle_degrees,
            )?;
        }

        Scene::WorldCameraSpaces => {
            canvas.with_viewport(world_debug_viewport, |canvas| {
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
        &format!("Scene: {}", state.current_scene_title()),
    );

    canvas.draw_text(
        Point2::new(2, FOOTER_ROW),
        &format!(
            "[{}/{}] {} | Mode: {} | Glyph '{}' | Menu: {} | Event: {} | h help | Esc quit",
            state.scene_position + 1,
            crate::scenes::scene_count(),
            state.current_scene_title(),
            state.control_mode.label(),
            state.glyph_stroke_character(),
            state
                .active_menu
                .as_ref()
                .map(|menu| menu.kind().title())
                .unwrap_or("closed"),
            state.last_input_event_trace.as_deref().unwrap_or("none"),
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
fn exit_confirm_popup_lines(state: &AppState) -> Option<Vec<String>> {
    state.confirm_exit.then(|| {
        vec![
            "Exit ascii-3d".to_string(),
            String::new(),
            "Do you really want to exit?".to_string(),
            String::new(),
            "Enter / y  = Yes, exit".to_string(),
            "Esc / n / c = Cancel".to_string(),
        ]
    })
}

fn debug_console_popup_lines(state: &AppState) -> Option<Vec<String>> {
    if !state.show_debug_console {
        return None;
    }

    let visible_rows = 24usize;
    let max_scroll = state.debug_console_max_scroll();
    let start = state
        .debug_console_lines
        .len()
        .saturating_sub(visible_rows)
        .saturating_sub(state.debug_console_scroll);
    let end = (start + visible_rows).min(state.debug_console_lines.len());

    let mut lines = vec![
        format!(
            "Debug Console v[{}/{}] h[{}] PageUp/PageDown Left/Right",
            max_scroll.saturating_sub(state.debug_console_scroll),
            max_scroll,
            state.debug_console_horizontal_scroll
        ),
        "Debug menu -> Toggle debug console hides this popup".to_string(),
        String::new(),
    ];

    lines.extend(
        state
            .debug_console_lines
            .iter()
            .skip(start)
            .take(end - start)
            .map(|line| {
                line.chars()
                    .skip(state.debug_console_horizontal_scroll)
                    .collect::<String>()
            }),
    );

    Some(lines)
}

fn scene_browser_popup_lines(state: &AppState) -> Option<Vec<String>> {
    if !state.scene_browser_open {
        return None;
    }

    let registry = crate::scenes::registry();
    let mut lines = vec![
        "Scenes".to_string(),
        "Up/Down select | Enter open | Esc close".to_string(),
        String::new(),
    ];

    for descriptor in registry {
        let selector = if descriptor.index == state.scene_browser_selected {
            ">"
        } else {
            " "
        };
        let current = if descriptor.index == state.scene_position {
            " [current]"
        } else {
            ""
        };

        lines.push(format!("{selector} {}{current}", descriptor.title));
    }

    Some(lines)
}

fn world_objects_popup_lines(state: &AppState) -> Option<Vec<String>> {
    if !state.loaded_a3d_workspace.objects_panel_open() {
        return None;
    }

    let mut lines = vec![
        "Objects".to_string(),
        "Up/Down select | Enter inspect | Esc close".to_string(),
        String::new(),
    ];

    for (index, entry) in state.loaded_a3d_workspace.entries().iter().enumerate() {
        let selector = if index == state.loaded_a3d_workspace.selected_entry() {
            ">"
        } else {
            " "
        };
        let visibility = match entry.visible {
            Some(true) => "visible",
            Some(false) => "hidden",
            None => "runtime",
        };
        let inspected = if state.loaded_a3d_workspace.inspected_target() == Some(&entry.target) {
            " [inspected]"
        } else {
            ""
        };

        lines.push(format!(
            "{selector} {}  ({visibility}){inspected}",
            entry.target.label()
        ));
    }

    Some(lines)
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

    let terminal_area = terminal.size()?;

    let scene_start = Instant::now();
    let scene_canvas = render_scene_frame(
        state,
        assets,
        terminal_area.width as usize,
        terminal_area.height.saturating_sub(1) as usize,
    )?;
    let scene_frame = scene_start.elapsed();

    let camera_start = Instant::now();
    let (camera_viewport_width, camera_viewport_height) =
        camera_viewport_canvas_size(state, terminal_area.width, terminal_area.height);

    let camera_viewport_canvas = match state.current_scene() {
        Scene::LoadedA3d => Some(render_loaded_a3d_camera_viewport_canvas(
            state,
            camera_viewport_width,
            camera_viewport_height,
        )?),
        Scene::WorldCameraSpaces => Some(render_camera_viewport_canvas(
            state,
            camera_viewport_width,
            camera_viewport_height,
        )?),
        _ => None,
    };
    let camera_viewport = camera_start.elapsed();

    let debug_popup_lines = exit_confirm_popup_lines(state)
        .or_else(|| scene_browser_popup_lines(state))
        .or_else(|| debug_console_popup_lines(state));
    let frame_timing_lines = state.frame_timing_lines();
    let file_picker_labels = state.a3d_file_picker.as_ref().map(|picker| picker.labels());
    let file_picker_current_dir = state
        .a3d_file_picker
        .as_ref()
        .map(|picker| picker.current_dir.display().to_string());
    let loaded_editor_items = state.loaded_a3d_editor_items();
    let loaded_property_rows = state
        .loaded_a3d_properties
        .target()
        .map(|target| {
            loaded_a3d_property_rows(
                target,
                state.loaded_a3d_world.as_ref(),
                state.loaded_a3d_workspace.active_xyz_target(),
            )
        })
        .unwrap_or_default();

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

        if state.loaded_a3d_hierarchy.is_open() {
            draw_object_hierarchy(
                frame,
                centered_editor_rect(
                    64,
                    (loaded_editor_items.len() as u16 + 4).clamp(7, 28),
                    frame.area(),
                ),
                &loaded_editor_items,
                &state.loaded_a3d_hierarchy,
                "A3D Objects",
            );
        }
        if state.loaded_a3d_properties.is_open() {
            let object_name = state
                .loaded_a3d_properties
                .target()
                .map(|target| target.id.as_str())
                .unwrap_or("A3D target");
            draw_properties_panel(
                frame,
                centered_editor_rect(
                    76,
                    (loaded_property_rows.len() as u16 + 4).clamp(8, 30),
                    frame.area(),
                ),
                object_name,
                &loaded_property_rows,
                &state.loaded_a3d_properties,
            );
        }
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

fn centered_editor_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyHandling {
    Handled,
    Ignored,
    Quit,
}

fn describe_key_code_for_trace(key_code: KeyCode) -> String {
    match key_code {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Char(character) => format!("'{character}'"),
        KeyCode::F(number) => format!("F{number}"),
        other => format!("{other:?}"),
    }
}

fn trace_key_event(state: &mut AppState, route: &str, key_code: KeyCode) {
    state.last_input_event_trace = Some(format!(
        "{route}: key {} | scene {} | mode {} | menu {}",
        describe_key_code_for_trace(key_code),
        state.current_scene_title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

fn trace_command_event(state: &mut AppState, route: &str, command: AppCommand) {
    state.last_input_event_trace = Some(format!(
        "{route}: command {command:?} | scene {} | mode {} | menu {}",
        state.current_scene_title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

fn describe_key_code_for_debug_console(key_code: KeyCode) -> String {
    match key_code {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Char(character) => format!("'{character}'"),
        KeyCode::F(number) => format!("F{number}"),
        other => format!("{other:?}"),
    }
}

fn push_key_debug_trace(state: &mut AppState, route: &str, key_code: KeyCode) {
    state.push_debug_console_line(format!(
        "{route}: key {} | scene {} | mode {} | menu {}",
        describe_key_code_for_debug_console(key_code),
        state.current_scene_title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

fn push_command_debug_trace(state: &mut AppState, route: &str, command: AppCommand) {
    state.push_debug_console_line(format!(
        "{route}: command {command:?} | scene {} | mode {} | menu {}",
        state.current_scene_title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {
    match command {
        AppCommand::Quit => {
            state.open_exit_confirm();
            KeyHandling::Handled
        }

        AppCommand::XyzControl(event) => {
            if state.apply_xyz_control_event(event) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::OpenA3dFilePicker => {
            state.open_a3d_file_picker();
            KeyHandling::Handled
        }

        AppCommand::ReloadA3d => {
            state.reload_a3d();
            KeyHandling::Handled
        }

        AppCommand::OpenWorldObjects => {
            let items = state.loaded_a3d_editor_items();
            state.loaded_a3d_hierarchy.open(&items);
            state.close_menu();
            KeyHandling::Handled
        }

        AppCommand::ShowOsGraphicsOverlay => {
            crate::graphics::raylib_overlay::spawn_raylib_overlay_demo();

            KeyHandling::Handled
        }

        AppCommand::ToggleDebugConsole => {
            state.toggle_debug_console();
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

        AppCommand::SetControlModeScene => {
            state.set_control_mode(ControlMode::Scene);
            state.close_menu();
            KeyHandling::Handled
        }

        AppCommand::SetControlModeCamera => {
            state.set_control_mode(ControlMode::Camera);
            state.close_menu();
            KeyHandling::Handled
        }

        AppCommand::SetControlModeLight => {
            state.set_control_mode(ControlMode::Light);
            state.close_menu();
            KeyHandling::Handled
        }

        AppCommand::OpenSceneBrowser => {
            state.open_scene_browser();
            KeyHandling::Handled
        }

        AppCommand::ResetActiveControl => {
            if state.reset_active_control() {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
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

        AppCommand::MenuLeft => {
            state.open_previous_menu();
            KeyHandling::Handled
        }

        AppCommand::MenuRight => {
            state.open_next_menu();
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

        AppCommand::RotateWorldPositiveX => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(5.0, 0.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldNegativeX => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(-5.0, 0.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldPositiveY => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 5.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldNegativeY => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, -5.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldPositiveZ => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 0.0, 5.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldNegativeZ => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 0.0, -5.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveWorldOriginLeft => {
            state.move_world_origin(Vec3::new(-0.25, 0.0, 0.0));
            KeyHandling::Handled
        }

        AppCommand::MoveWorldOriginRight => {
            state.move_world_origin(Vec3::new(0.25, 0.0, 0.0));
            KeyHandling::Handled
        }

        AppCommand::MoveWorldOriginUp => {
            state.move_world_origin(Vec3::new(0.0, 0.25, 0.0));
            KeyHandling::Handled
        }

        AppCommand::MoveWorldOriginDown => {
            state.move_world_origin(Vec3::new(0.0, -0.25, 0.0));
            KeyHandling::Handled
        }

        AppCommand::ResetWorldAxes => {
            if state.reset_world_axes() {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
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

        AppCommand::MoveLightForward => {
            if state.move_loaded_a3d_light_forward(0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightBackward => {
            if state.move_loaded_a3d_light_forward(-0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightLeft => {
            if state.move_loaded_a3d_light_right(-0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightRight => {
            if state.move_loaded_a3d_light_right(0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightDown => {
            if state.move_loaded_a3d_light_up(-0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightUp => {
            if state.move_loaded_a3d_light_up(0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
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

fn handle_key_press(state: &mut AppState, key: KeyEvent) -> KeyHandling {
    let key_code = key.code;

    if state.confirm_exit {
        match key_code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                return KeyHandling::Quit;
            }
            KeyCode::Esc
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Char('c')
            | KeyCode::Char('C') => {
                state.close_exit_confirm();
                return KeyHandling::Handled;
            }
            _ => {
                return KeyHandling::Handled;
            }
        }
    }

    if key.modifiers.contains(KeyModifiers::ALT) {
        if state.active_menu.is_some() {
            state.toggle_menu_bar();
            return KeyHandling::Handled;
        }

        if state.open_menu_for_hotkey(key_code) {
            return KeyHandling::Handled;
        }

        state.open_menu(crate::menu::MenuKind::File);
        return KeyHandling::Handled;
    }

    if state.scene_browser_open {
        match key_code {
            KeyCode::Esc => {
                state.close_scene_browser();
                return KeyHandling::Handled;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                state.move_scene_browser_up();
                return KeyHandling::Handled;
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                state.move_scene_browser_down();
                return KeyHandling::Handled;
            }
            KeyCode::Enter => {
                state.select_scene_browser_entry();
                return KeyHandling::Handled;
            }
            _ => return KeyHandling::Handled,
        }
    }

    if state.loaded_a3d_properties.is_open() {
        let rows = state
            .loaded_a3d_properties
            .target()
            .map(|target| {
                loaded_a3d_property_rows(
                    target,
                    state.loaded_a3d_world.as_ref(),
                    state.loaded_a3d_workspace.active_xyz_target(),
                )
            })
            .unwrap_or_default();
        if let Some(editor_event) = state.loaded_a3d_properties.handle_key(key_code, &rows) {
            match editor_event {
                EditorEvent::CloseRequested => {
                    let items = state.loaded_a3d_editor_items();
                    state.loaded_a3d_hierarchy.open(&items);
                }
                EditorEvent::ActionRequested { target, action, .. } => {
                    let world_target = loaded_a3d_world_target(&target);
                    match action {
                        EditorAction::ActivateControlTarget => {
                            state.loaded_a3d_workspace.activate_target(world_target);
                        }
                        EditorAction::ToggleVisibility => {
                            state.toggle_loaded_a3d_visibility(&target.path);
                        }
                        EditorAction::ResetTransform => {
                            state.reset_loaded_a3d_editor_target(&world_target);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        return KeyHandling::Handled;
    }

    if state.loaded_a3d_hierarchy.is_open() {
        let items = state.loaded_a3d_editor_items();
        if let Some(editor_event) = state.loaded_a3d_hierarchy.handle_key(key_code, &items) {
            match editor_event {
                EditorEvent::InspectRequested { target, .. } => {
                    state
                        .loaded_a3d_workspace
                        .inspect_target(loaded_a3d_world_target(&target));
                    state.loaded_a3d_hierarchy.close();
                    state.loaded_a3d_properties.open(target);
                }
                EditorEvent::CloseRequested => {}
                _ => {}
            }
        }
        return KeyHandling::Handled;
    }

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

    // Menus are modal and must keep priority over the floating debug console.
    if state.active_menu.is_some() {
        return menu_command_for_key(key_code)
            .map(|command| apply_app_command(state, command))
            .unwrap_or(KeyHandling::Ignored);
    }

    if state.current_scene() == Scene::LoadedA3d {
        if matches!(
            state.loaded_a3d_workspace.active_xyz_target(),
            WorldEditorTarget::Camera
        ) {
            match key_code {
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    state.move_world_camera_forward(CAMERA_MOVE_STEP);
                    return KeyHandling::Handled;
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    state.move_world_camera_forward(-CAMERA_MOVE_STEP);
                    return KeyHandling::Handled;
                }
                // Retire the old WASD forward/back aliases in the A3D editor.
                // Consume them here so they cannot fall through to legacy
                // camera-mode bindings.
                KeyCode::Char('w')
                | KeyCode::Char('W')
                | KeyCode::Char('s')
                | KeyCode::Char('S') => {
                    return KeyHandling::Handled;
                }
                _ => {}
            }
        }

        let factor = match key_code {
            KeyCode::Char('+') | KeyCode::Char('=') => Some(1.1),
            KeyCode::Char('-') | KeyCode::Char('_') => Some(1.0 / 1.1),
            _ => None,
        };

        if let Some(factor) = factor {
            return if state.scale_active_loaded_a3d_object(factor) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            };
        }
    }

    // XyzControl is the primitive axis/origin input layer. It routes through
    // the currently active control target inside apply_xyz_control_event().
    if let Some(event) = state.xyz_control.event_for_key(key) {
        return apply_app_command(state, AppCommand::XyzControl(event));
    }

    if state.show_debug_console {
        match key_code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
                state.close_debug_console();
                return KeyHandling::Handled;
            }
            KeyCode::Tab => {
                return apply_app_command(state, AppCommand::ToggleControlMode);
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                return apply_app_command(
                    state,
                    AppCommand::OpenMenu(crate::menu::MenuKind::Control),
                );
            }
            KeyCode::PageUp => {
                state.scroll_debug_console_up(6);
                return KeyHandling::Handled;
            }
            KeyCode::PageDown => {
                state.scroll_debug_console_down(6);
                return KeyHandling::Handled;
            }
            KeyCode::Left => {
                state.scroll_debug_console_left(8);
                return KeyHandling::Handled;
            }
            KeyCode::Right => {
                state.scroll_debug_console_right(8);
                return KeyHandling::Handled;
            }
            _ => {
                return KeyHandling::Handled;
            }
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

    trace_key_event(state, "active scene key", key_code);

    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };

    command
        .map(|command| apply_app_command(state, command))
        .unwrap_or(KeyHandling::Ignored)
}

const CONTINUOUS_INPUT_SAMPLE_INTERVAL: Duration = Duration::from_millis(16);
const CONTINUOUS_INPUT_CONTINUITY_WINDOW: Duration = Duration::from_millis(500);

#[derive(Default)]
struct ContinuousInputSampler {
    active: Option<(KeyCode, KeyModifiers)>,
    started_at: Option<Instant>,
    last_seen_at: Option<Instant>,
    last_applied_at: Option<Instant>,
    pending: Option<KeyEvent>,
}

impl ContinuousInputSampler {
    fn observe(&mut self, key: KeyEvent, now: Instant) {
        let identity = (key.code, key.modifiers);
        let continues_hold = self.active.as_ref() == Some(&identity)
            && self.last_seen_at.is_some_and(|last_seen| {
                now.duration_since(last_seen) <= CONTINUOUS_INPUT_CONTINUITY_WINDOW
            });

        if !continues_hold {
            self.started_at = Some(now);
            self.last_applied_at = None;
        }

        self.active = Some(identity);
        self.last_seen_at = Some(now);
        self.pending = Some(key);
    }

    fn clear(&mut self) {
        *self = Self::default();
    }

    fn sample(&mut self, now: Instant) -> Option<(KeyEvent, usize)> {
        let key = self.pending?;

        if self.last_applied_at.is_some_and(|last_applied| {
            now.duration_since(last_applied) < CONTINUOUS_INPUT_SAMPLE_INTERVAL
        }) {
            return None;
        }

        self.pending = None;
        self.last_applied_at = Some(now);

        let held_for = self
            .started_at
            .map(|started| now.duration_since(started))
            .unwrap_or_default();

        let step_count = if held_for < Duration::from_millis(350) {
            1
        } else if held_for < Duration::from_millis(900) {
            2
        } else {
            3
        };

        Some((key, step_count))
    }
}

fn is_continuous_control_key(state: &AppState, key: KeyEvent) -> bool {
    if state.current_scene() != Scene::LoadedA3d
        || state.confirm_exit
        || state.active_menu.is_some()
        || state.scene_browser_open
        || state.a3d_file_picker.is_some()
        || state.loaded_a3d_hierarchy.is_open()
        || state.loaded_a3d_properties.is_open()
        || state.show_debug_console
        || is_loaded_a3d_debug_popup_visible(state)
    {
        return false;
    }

    matches!(
        key.code,
        KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Char('x')
            | KeyCode::Char('X')
            | KeyCode::Char('y')
            | KeyCode::Char('Y')
            | KeyCode::Char('z')
            | KeyCode::Char('Z')
            | KeyCode::Char('+')
            | KeyCode::Char('=')
            | KeyCode::Char('-')
            | KeyCode::Char('_')
    )
}

pub fn run() -> io::Result<()> {
    let assets = load_scene_assets()?;
    let _terminal_guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut state = AppState::new();
    state.load_a3d_file(initial_a3d_manifest_path());
    let mut previous_time = Instant::now();
    let mut previous_frame: Option<String> = None;
    let mut continuous_input = ContinuousInputSampler::default();

    let timings = render_scene(&mut terminal, &state, &assets, &mut previous_frame)?;
    state.record_render_timings(timings);

    loop {
        let frame_start = Instant::now();
        let now = frame_start;
        let elapsed = now.duration_since(previous_time);
        previous_time = now;

        let update_start = Instant::now();
        let mut should_render = state.update(elapsed);
        let update = update_start.elapsed();
        let mut should_quit = false;

        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    if is_continuous_control_key(&state, key) {
                        continuous_input.observe(key, Instant::now());
                    } else if key.kind == KeyEventKind::Press {
                        continuous_input.clear();
                        match handle_key_press(&mut state, key) {
                            KeyHandling::Quit => {
                                should_quit = true;
                                break;
                            }
                            KeyHandling::Handled => {
                                should_render = true;
                            }
                            KeyHandling::Ignored => {}
                        }
                    }
                }
                Event::Resize(_, _) => {
                    continuous_input.clear();
                    should_render = true;
                }
                _ => {}
            }
        }

        if should_quit {
            break;
        }

        if let Some((key, step_count)) = continuous_input.sample(Instant::now()) {
            if is_continuous_control_key(&state, key) {
                for _ in 0..step_count {
                    match handle_key_press(&mut state, key) {
                        KeyHandling::Quit => {
                            should_quit = true;
                            break;
                        }
                        KeyHandling::Handled => {
                            should_render = true;
                        }
                        KeyHandling::Ignored => {}
                    }
                }
            } else {
                continuous_input.clear();
            }
        }

        if should_quit {
            break;
        }

        if should_render {
            let mut timings = render_scene(&mut terminal, &state, &assets, &mut previous_frame)?;
            timings.update = update;
            state.record_render_timings(timings);
        }

        let remaining = FRAME_DURATION.saturating_sub(frame_start.elapsed());
        if !remaining.is_zero() {
            let _ = event::poll(remaining)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppState, asset_path, default_a3d_root_path, load_mesh_asset};
    use crate::scenes::Scene;
    use std::path::Path;

    #[test]
    fn default_a3d_root_points_to_bundled_demo() {
        let path = default_a3d_root_path();

        assert!(path.ends_with(Path::new("assets").join("a3d").join("p_depth_demo")));
    }

    #[test]
    fn application_starts_on_first_scene_index_entry() {
        let state = AppState::new();
        let expected = crate::scenes::scene_descriptor_at(0);

        assert_eq!(state.current_scene_descriptor().id, expected.id);
        assert_eq!(state.current_scene(), expected.scene);
    }

    #[test]
    fn scene_browser_opens_on_current_scene() {
        let mut state = AppState::new();
        state.scene_position = 2;

        state.open_scene_browser();

        assert!(state.scene_browser_open);
        assert_eq!(state.scene_browser_selected, 2);
    }

    #[test]
    fn scene_browser_selects_requested_scene() {
        let mut state = AppState::new();
        state.open_scene_browser();
        state.scene_browser_selected = 1;

        state.select_scene_browser_entry();

        let expected = crate::scenes::scene_descriptor_at(1);
        assert!(!state.scene_browser_open);
        assert_eq!(state.current_scene_descriptor().id, expected.id);
        assert_eq!(state.current_scene(), expected.scene);
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
    fn selecting_scene_from_browser_resets_animation_angles() {
        let mut state = AppState::new();

        state.animation_angle_degrees = 45.0;
        state.box_angle_degrees = 90.0;
        state.open_scene_browser();
        state.scene_browser_selected = 1;

        state.select_scene_browser_entry();

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
    fn fit_aspect_dimensions_uses_full_width_for_16_9_when_height_allows() {
        assert_eq!(
            super::fit_aspect_dimensions(80, 60, super::CameraViewportAspectRatio::new(16, 9), 1.0),
            (80, 45)
        );
    }

    #[test]
    fn fit_aspect_dimensions_limits_by_height_for_16_9() {
        assert_eq!(
            super::fit_aspect_dimensions(
                160,
                40,
                super::CameraViewportAspectRatio::new(16, 9),
                1.0
            ),
            (71, 40)
        );
    }

    #[test]
    fn fit_aspect_dimensions_limits_by_height_for_4_3() {
        assert_eq!(
            super::fit_aspect_dimensions(120, 30, super::CameraViewportAspectRatio::new(4, 3), 1.0),
            (40, 30)
        );
    }

    #[test]
    fn camera_viewport_canvas_size_uses_bottom_third_height_budget_with_default_cell_aspect() {
        let state = AppState::new();

        assert_eq!(
            super::camera_viewport_canvas_size(&state, 180, 60),
            (15, 17)
        );
    }

    #[test]
    fn fit_aspect_dimensions_uses_cell_aspect_ratio_for_visual_16_9() {
        assert_eq!(
            super::fit_aspect_dimensions(
                180,
                17,
                super::CameraViewportAspectRatio::new(16, 9),
                2.0
            ),
            (60, 17)
        );
    }

    #[test]
    fn parse_camera_viewport_aspect_ratio_accepts_string() {
        let value = serde_json::json!("4:3");

        assert_eq!(
            super::parse_camera_viewport_aspect_ratio(&value),
            Some(super::CameraViewportAspectRatio::new(4, 3))
        );
    }

    #[test]
    fn parse_camera_viewport_aspect_ratio_accepts_object() {
        let value = serde_json::json!({
            "width": 16,
            "height": 9
        });

        assert_eq!(
            super::parse_camera_viewport_aspect_ratio(&value),
            Some(super::CameraViewportAspectRatio::new(16, 9))
        );
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
