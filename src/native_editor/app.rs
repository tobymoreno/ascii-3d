use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    time::Instant,
};

use super::{
    geojson_lines::{GeoJsonLineStyle, append_geojson_lines},
    gpu_renderer::{GpuVertex, UploadStats},
    labels::{GlobeLabel, LabelKind, load_builtin_labels, rasterize_marine_label},
};

use super::starfield::{Starfield, ViewportStar};

use crate::{
    a3d::{AssetRef, LoadedWorld, ToonMaterialConfig, Transform, load_a3d_project},
    editor_core::{EditorCommand, EditorEntry, EditorSession},
    geojson::{GeoJsonElevationPoint, GeoJsonMap, load_geojson_elevation_points, load_geojson_map},
    math::{Mat4, Vec3},
    mesh::Mesh,
    obj::load_obj,
};

const STARTUP_SCENE: &str = "assets/a3d/earth_km/scene.a3d";
const ELEVATION_POINTS_ASSET: &str =
    "assets/maps/ne_10m_geography_regions_elevation_points.geojson";
const DEFAULT_CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const DEFAULT_CAMERA_DISTANCE: f32 = 8.0;
const MIN_CAMERA_DISTANCE: f32 = 0.25;
const MAX_CAMERA_DISTANCE: f32 = 250.0;
const ORBIT_RADIANS_PER_PIXEL: f32 = 0.008;
const ZOOM_EXPONENT_PER_SCROLL_POINT: f32 = 0.0015;
const CAMERA_FOV_DEGREES: f32 = 45.0;
const REFERENCE_VIEWPORT_HEIGHT: f32 = 720.0;
const CAMERA_NEAR: f32 = 0.1;
const CAMERA_FIT_PADDING: f32 = 1.35;
const MIN_FIT_RADIUS: f32 = 0.25;
// Blend rectilinear perspective toward a cylindrical projection near the
// horizontal edges. The center remains ordinary perspective.
const EDGE_CORRECTION_STRENGTH: f32 = 0.65;
const EDGE_CORRECTION_START_RADIANS: f32 = 15.0_f32.to_radians();
const EDGE_CORRECTION_FULL_RADIANS: f32 = 60.0_f32.to_radians();
const CREASE_NORMAL_DOT_THRESHOLD: f32 = 0.70;
const DEFAULT_OUTLINE_PIXEL_WIDTH: f32 = 1.8;
const MIN_OUTLINE_PIXEL_WIDTH: f32 = 0.5;
const MAX_OUTLINE_PIXEL_WIDTH: f32 = 4.0;
const DEFAULT_VIEWPORT_BACKGROUND_RGB: [u8; 3] = [2, 18, 36];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativeEditorTarget {
    Scene,
    Camera,
    Object(String),
}

impl NativeEditorTarget {
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Scene => "Scene",
            Self::Camera => "Camera",
            Self::Object(id) => id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NativeCamera {
    pub(super) position: Vec3,
    pub(super) yaw_radians: f32,
    pub(super) pitch_radians: f32,
    pub(super) roll_radians: f32,
    pub(super) focus_distance: f32,
}

impl Default for NativeCamera {
    fn default() -> Self {
        Self {
            position: DEFAULT_CAMERA_TARGET + Vec3::new(0.0, 0.0, DEFAULT_CAMERA_DISTANCE),
            yaw_radians: 0.0,
            pitch_radians: 0.0,
            roll_radians: 0.0,
            focus_distance: DEFAULT_CAMERA_DISTANCE,
        }
    }
}

impl NativeCamera {
    pub(super) fn forward(self) -> Vec3 {
        let horizontal = self.pitch_radians.cos();
        Vec3::new(
            horizontal * self.yaw_radians.sin(),
            self.pitch_radians.sin(),
            -horizontal * self.yaw_radians.cos(),
        )
        .normalized()
    }

    pub(super) fn target(self) -> Vec3 {
        self.position + self.forward() * self.focus_distance
    }

    pub(super) fn view_axes(self) -> (Vec3, Vec3) {
        let forward = self.forward();
        let base_right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
        let base_up = base_right.cross(forward).normalized();
        let cosine = self.roll_radians.cos();
        let sine = self.roll_radians.sin();
        let right = base_right * cosine + base_up * sine;
        let up = base_up * cosine - base_right * sine;
        (right, up)
    }

    pub(super) fn up(self) -> Vec3 {
        self.view_axes().1
    }

    fn look_at(&mut self, target: Vec3) {
        let direction = target - self.position;
        let distance = direction.length();
        if distance <= f32::EPSILON {
            return;
        }
        let forward = direction * (1.0 / distance);
        self.yaw_radians = forward.x.atan2(-forward.z);
        self.pitch_radians = forward.y.clamp(-1.0, 1.0).asin();
        self.focus_distance = distance;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CameraBounds {
    min: Vec3,
    max: Vec3,
}

impl CameraBounds {
    fn from_point(point: Vec3) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    fn include(&mut self, point: Vec3) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn radius(self) -> f32 {
        (self.max - self.center()).length().max(MIN_FIT_RADIUS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewportLine {
    pub(crate) start: [f32; 2],
    pub(crate) end: [f32; 2],
    pub(crate) selected: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ViewportLabel {
    pub(crate) text: String,
    pub(crate) position: [f32; 2],
    pub(crate) font_size: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct MarineGlyphQuad {
    pub(crate) glyph: char,
    pub(crate) corners: [[f32; 2]; 4],
}

#[derive(Clone, Copy, Debug)]
struct CachedGeoTriangle {
    points: [Vec3; 3],
    colors: [[f32; 4]; 3],
}

#[derive(Clone, Debug)]
struct CachedGeoGpuGeometry {
    key: String,
    revision: u64,
    vertices: Vec<GpuVertex>,
}

pub(crate) struct NativeEditorApp {
    pub(crate) session: EditorSession<NativeEditorTarget>,
    pub(crate) status: String,
    pub(super) world: Option<LoadedWorld>,
    meshes: HashMap<String, Mesh>,
    wire_meshes: HashMap<String, Mesh>,
    geojson_maps: HashMap<String, GeoJsonMap>,
    geojson_render_meshes: HashMap<String, Vec<CachedGeoTriangle>>,
    globe_labels: Vec<GlobeLabel>,
    starfield: Starfield,
    marine_label_textures: HashMap<String, egui::TextureHandle>,
    scene_path: PathBuf,
    pub(super) camera: NativeCamera,
    pub(super) scene_transform: Transform,
    gpu_target_format: Option<egui_wgpu::wgpu::TextureFormat>,
    show_wireframe: bool,
    show_labels: bool,
    outline_pixel_width: f32,
    viewport_background_rgb: [u8; 3],
    fps_last_frame: Instant,
    fps_smoothed: f32,
    geometry_ms_smoothed: f32,
    geo_gpu_geometry_cache: RefCell<Option<CachedGeoGpuGeometry>>,
    upload_stats: UploadStats,
}

impl Default for NativeEditorApp {
    fn default() -> Self {
        Self::load_startup_scene()
    }
}

impl NativeEditorApp {
    fn load_startup_scene() -> Self {
        let scene_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(STARTUP_SCENE);

        match build_scene_state(&scene_path) {
            Ok(state) => state,
            Err(error) => Self::failed_scene_state(scene_path, error),
        }
    }

    fn failed_scene_state(scene_path: PathBuf, error: impl std::fmt::Display) -> Self {
        Self {
            session: EditorSession::new(
                vec![
                    EditorEntry::new(NativeEditorTarget::Scene, None),
                    EditorEntry::new(NativeEditorTarget::Camera, None),
                ],
                NativeEditorTarget::Camera,
            ),
            status: format!("Failed to load {}: {error}", scene_path.display()),
            world: None,
            meshes: HashMap::new(),
            wire_meshes: HashMap::new(),
            geojson_maps: HashMap::new(),
            geojson_render_meshes: HashMap::new(),
            globe_labels: load_builtin_labels(Path::new(env!("CARGO_MANIFEST_DIR"))),
            starfield: Starfield::load(Path::new(env!("CARGO_MANIFEST_DIR"))),
            marine_label_textures: HashMap::new(),
            scene_path,
            camera: NativeCamera::default(),
            scene_transform: Transform::default(),
            gpu_target_format: None,
            show_wireframe: false,
            show_labels: true,
            outline_pixel_width: DEFAULT_OUTLINE_PIXEL_WIDTH,
            viewport_background_rgb: DEFAULT_VIEWPORT_BACKGROUND_RGB,
            fps_last_frame: Instant::now(),
            fps_smoothed: 0.0,
            geometry_ms_smoothed: 0.0,
            geo_gpu_geometry_cache: RefCell::new(None),
            upload_stats: UploadStats::default(),
        }
    }

    pub(crate) fn update_frame_timing(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.fps_last_frame).as_secs_f32();
        self.fps_last_frame = now;

        if !(0.000_001..=0.5).contains(&elapsed) {
            return;
        }

        let instantaneous = 1.0 / elapsed;
        self.fps_smoothed = if self.fps_smoothed <= f32::EPSILON {
            instantaneous
        } else {
            self.fps_smoothed * 0.90 + instantaneous * 0.10
        };
    }

    pub(crate) fn fps(&self) -> Option<f32> {
        (self.fps_smoothed > f32::EPSILON).then_some(self.fps_smoothed)
    }

    pub(crate) fn record_geometry_time(&mut self, elapsed_ms: f32) {
        self.geometry_ms_smoothed = if self.geometry_ms_smoothed <= f32::EPSILON {
            elapsed_ms
        } else {
            self.geometry_ms_smoothed * 0.90 + elapsed_ms * 0.10
        };
    }

    pub(crate) fn geometry_ms(&self) -> Option<f32> {
        (self.geometry_ms_smoothed > f32::EPSILON).then_some(self.geometry_ms_smoothed)
    }

    pub(crate) fn upload_stats(&self) -> UploadStats {
        self.upload_stats.clone()
    }

    pub(crate) fn load_scene(&mut self, scene_path: PathBuf) -> bool {
        match build_scene_state(&scene_path) {
            Ok(mut state) => {
                state.gpu_target_format = self.gpu_target_format;
                state.show_wireframe = self.show_wireframe;
                state.show_labels = self.show_labels;
                state.marine_label_textures = std::mem::take(&mut self.marine_label_textures);
                state.outline_pixel_width = self.outline_pixel_width;
                state.viewport_background_rgb = self.viewport_background_rgb;
                *self = state;
                true
            }
            Err(error) => {
                self.status = format!("Failed to load {}: {error}", scene_path.display());
                false
            }
        }
    }

    pub(crate) fn apply_editor_command(
        &mut self,
        command: EditorCommand<NativeEditorTarget>,
    ) -> bool {
        match command {
            EditorCommand::SetVisibility { target, visible } => {
                self.set_visibility(target, visible)
            }
            other => self.session.apply(other),
        }
    }

    fn set_visibility(&mut self, target: NativeEditorTarget, visible: bool) -> bool {
        let NativeEditorTarget::Object(id) = &target else {
            return false;
        };

        let previous = self.session.visibility(&target);
        if !self.session.apply(EditorCommand::SetVisibility {
            target: target.clone(),
            visible,
        }) {
            return false;
        }

        let updated = self
            .world
            .as_mut()
            .and_then(|world| world.object_mut(id))
            .map(|object| {
                object.render.visible = visible;
            })
            .is_some();

        if !updated {
            if let Some(previous) = previous {
                self.session.apply(EditorCommand::SetVisibility {
                    target,
                    visible: previous,
                });
            }
            return false;
        }

        self.status = format!("{} {}", if visible { "Showing" } else { "Hiding" }, id);
        true
    }

    pub(crate) fn scene_title(&self) -> &str {
        self.world
            .as_ref()
            .map(|world| world.title.as_str())
            .unwrap_or("Scene unavailable")
    }

    pub(crate) fn scene_path(&self) -> &std::path::Path {
        &self.scene_path
    }

    pub(crate) fn object_transform(&self, target: &NativeEditorTarget) -> Option<Transform> {
        let NativeEditorTarget::Object(id) = target else {
            return None;
        };
        self.world
            .as_ref()?
            .object(id)
            .map(|object| object.transform)
    }

    pub(crate) fn editor_object_count(&self) -> usize {
        self.session.entries().len().saturating_sub(2)
    }

    pub(crate) fn mesh_asset_count(&self) -> usize {
        self.meshes.len()
    }

    pub(crate) fn reset_camera(&mut self) {
        self.camera = NativeCamera::default();
        if let Some(bounds) = self.scene_bounds() {
            self.fit_camera_to_bounds(bounds);
        }
        self.status = "Camera reset to scene bounds".to_owned();
    }

    pub(crate) fn look_camera(&mut self, delta_pixels: [f32; 2]) {
        self.camera.yaw_radians -= delta_pixels[0] * ORBIT_RADIANS_PER_PIXEL;
        self.camera.pitch_radians = (self.camera.pitch_radians
            - delta_pixels[1] * ORBIT_RADIANS_PER_PIXEL)
            .clamp(-1.553_343, 1.553_343);
        self.status = "Looking from camera".to_owned();
    }

    pub(crate) fn orbit_focus(&mut self, delta_pixels: [f32; 2]) {
        let target = self.camera.target();
        self.camera.yaw_radians -= delta_pixels[0] * ORBIT_RADIANS_PER_PIXEL;
        self.camera.pitch_radians = (self.camera.pitch_radians
            - delta_pixels[1] * ORBIT_RADIANS_PER_PIXEL)
            .clamp(-1.553_343, 1.553_343);
        self.camera.position = target - self.camera.forward() * self.camera.focus_distance;
        self.status = "Orbiting focus target".to_owned();
    }

    pub(crate) fn pan_camera(&mut self, delta_pixels: [f32; 2]) {
        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let world_units_per_pixel = self.camera.focus_distance / focal_length;
        let (right, up) = self.camera.view_axes();
        self.camera.position = self.camera.position
            - right * (delta_pixels[0] * world_units_per_pixel)
            + up * (delta_pixels[1] * world_units_per_pixel);
        self.status = "Panning camera".to_owned();
    }

    pub(crate) fn zoom_camera(&mut self, scroll_delta: f32) {
        let target = self.camera.target();
        let factor = (-scroll_delta * ZOOM_EXPONENT_PER_SCROLL_POINT).exp();
        self.camera.focus_distance =
            (self.camera.focus_distance * factor).clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
        self.camera.position = target - self.camera.forward() * self.camera.focus_distance;
        self.status = format!("Camera distance {:.2}", self.camera.focus_distance);
    }

    pub(crate) fn frame_selected(&mut self) -> bool {
        let Some(NativeEditorTarget::Object(id)) = self.session.inspected_target() else {
            self.status = "Select an object to frame".to_owned();
            return false;
        };
        let id = id.clone();

        let Some(bounds) = self.object_bounds(&id) else {
            self.status = format!("Selected object {id} has no renderable bounds");
            return false;
        };

        self.fit_camera_to_bounds(bounds);
        self.status = format!("Framed {id} to its bounds");
        true
    }

    fn fit_camera_to_bounds(&mut self, bounds: CameraBounds) {
        let target = bounds.center();
        let radius = bounds.radius();
        let half_fov = 0.5 * CAMERA_FOV_DEGREES.to_radians();
        let distance = (radius / half_fov.sin() * CAMERA_FIT_PADDING)
            .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
        let forward = self.camera.forward();

        self.camera.position = target - forward * distance;
        self.camera.focus_distance = distance;
    }

    fn scene_bounds(&self) -> Option<CameraBounds> {
        self.bounds_for_object_filter(|_| true)
    }

    fn object_bounds(&self, selected_id: &str) -> Option<CameraBounds> {
        self.bounds_for_object_filter(|object_id| {
            object_id == selected_id || object_id.starts_with(&format!("{selected_id}/"))
        })
    }

    fn bounds_for_object_filter(
        &self,
        include_object: impl Fn(&str) -> bool,
    ) -> Option<CameraBounds> {
        let world = self.world.as_ref()?;
        let mut bounds: Option<CameraBounds> = None;

        for object in &world.objects {
            if !include_object(&object.id) {
                continue;
            }
            let world_matrix = self.scene_transform.matrix() * object.world_matrix();
            match &object.asset {
                AssetRef::Mesh { path } => {
                    let Some(mesh) = self.meshes.get(path) else {
                        continue;
                    };
                    for vertex in &mesh.vertices {
                        let point = world_matrix.transform_point(*vertex);
                        match &mut bounds {
                            Some(bounds) => bounds.include(point),
                            None => bounds = Some(CameraBounds::from_point(point)),
                        }
                    }
                }
                AssetRef::GeoJsonMap { path, radius_scale } => {
                    let key = geojson_cache_key(path, *radius_scale);
                    let Some(map) = self.geojson_maps.get(&key) else {
                        continue;
                    };
                    for (start, end) in &map.segments {
                        for vertex in [start, end] {
                            let point = world_matrix.transform_point(*vertex);
                            match &mut bounds {
                                Some(bounds) => bounds.include(point),
                                None => bounds = Some(CameraBounds::from_point(point)),
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        bounds
    }

    pub(crate) fn camera_position(&self) -> Vec3 {
        self.camera.position
    }

    pub(crate) fn camera_target(&self) -> Vec3 {
        self.camera.target()
    }

    pub(crate) fn camera_distance(&self) -> f32 {
        self.camera.focus_distance
    }

    pub(crate) fn camera_angles_degrees(&self) -> [f32; 2] {
        [
            self.camera.yaw_radians.to_degrees(),
            self.camera.pitch_radians.to_degrees(),
        ]
    }

    pub(crate) fn set_gpu_target_format(&mut self, target_format: egui_wgpu::wgpu::TextureFormat) {
        self.gpu_target_format = Some(target_format);
    }

    pub(crate) fn gpu_target_format(&self) -> Option<egui_wgpu::wgpu::TextureFormat> {
        self.gpu_target_format
    }

    pub(crate) fn show_wireframe(&self) -> bool {
        self.show_wireframe
    }

    pub(crate) fn set_show_wireframe(&mut self, show_wireframe: bool) {
        self.show_wireframe = show_wireframe;
        self.status = if show_wireframe {
            "Debug wireframe enabled".to_owned()
        } else {
            "Silhouette outlines enabled".to_owned()
        };
    }

    pub(crate) fn outline_pixel_width(&self) -> f32 {
        self.outline_pixel_width
    }

    pub(crate) fn set_outline_pixel_width(&mut self, width: f32) {
        self.outline_pixel_width = width.clamp(MIN_OUTLINE_PIXEL_WIDTH, MAX_OUTLINE_PIXEL_WIDTH);
        self.status = format!("Outline width: {:.1} px", self.outline_pixel_width);
    }

    pub(crate) fn viewport_background_rgb(&self) -> [u8; 3] {
        self.viewport_background_rgb
    }

    pub(crate) fn set_viewport_background_rgb(&mut self, rgb: [u8; 3]) {
        self.viewport_background_rgb = rgb;
        self.status = format!(
            "Viewport background: #{:02X}{:02X}{:02X}",
            rgb[0], rgb[1], rgb[2]
        );
    }

    pub(crate) fn reset_viewport_style(&mut self) {
        self.outline_pixel_width = DEFAULT_OUTLINE_PIXEL_WIDTH;
        self.viewport_background_rgb = DEFAULT_VIEWPORT_BACKGROUND_RGB;
        self.status = "Viewport style reset".to_owned();
    }

    pub(crate) fn show_labels(&self) -> bool {
        self.show_labels
    }

    pub(crate) fn set_show_labels(&mut self, show: bool) {
        self.show_labels = show;
        self.status = if show {
            format!("Labels enabled ({})", self.globe_labels.len())
        } else {
            "Labels hidden".to_owned()
        };
    }

    pub(crate) fn viewport_labels(&self, width: f32, height: f32) -> Vec<ViewportLabel> {
        if !self.show_labels || width <= 1.0 || height <= 1.0 {
            return Vec::new();
        }
        let Some(world) = &self.world else {
            return Vec::new();
        };
        let Some(view) =
            Mat4::look_at(self.camera.position, self.camera.target(), self.camera.up())
        else {
            return Vec::new();
        };
        let Some(earth) = world.objects.iter().find(|object| {
            world.object_effectively_visible(&object.id)
                && matches!(&object.asset, AssetRef::Mesh { path } if is_native_ocean_sphere(path))
        }) else {
            return Vec::new();
        };

        let model_view = view * self.scene_transform.matrix() * earth.world_matrix();
        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let mut candidates = Vec::<(i32, [f32; 2], f32, String)>::new();

        for label in self
            .globe_labels
            .iter()
            .filter(|label| label.kind != LabelKind::Marine)
        {
            let local_position = label.direction * (1.0 + label.altitude);
            let view_point = model_view.transform_point(local_position);
            if view_point.z >= -CAMERA_NEAR {
                continue;
            }
            let view_normal = model_view.transform_vector(label.direction).normalized();
            let to_camera = (view_point * -1.0).normalized();
            if view_normal.dot(to_camera) <= 0.20 {
                continue;
            }
            let Some(screen) = project_point(view_point, width, height, focal_length) else {
                continue;
            };
            if screen[0] < 8.0
                || screen[0] > width - 8.0
                || screen[1] < 8.0
                || screen[1] > height - 8.0
            {
                continue;
            }
            candidates.push((label.priority, screen, label.font_size, label.text.clone()));
        }

        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let mut accepted = Vec::<ViewportLabel>::new();
        for (_priority, position, font_size, text) in candidates {
            let half_width = font_size * (text.chars().count() as f32 * 0.30 + 0.8);
            let half_height = font_size * 0.78;
            let overlaps = accepted.iter().any(|existing| {
                let existing_half_width =
                    existing.font_size * (existing.text.chars().count() as f32 * 0.30 + 0.8);
                let existing_half_height = existing.font_size * 0.78;
                (position[0] - existing.position[0]).abs() < half_width + existing_half_width
                    && (position[1] - existing.position[1]).abs()
                        < half_height + existing_half_height
            });
            if overlaps {
                continue;
            }
            accepted.push(ViewportLabel {
                text,
                position,
                font_size,
            });
            if accepted.len() >= 80 {
                break;
            }
        }
        accepted
    }

    pub(crate) fn viewport_marine_glyph_quads(
        &self,
        width: f32,
        height: f32,
    ) -> Vec<MarineGlyphQuad> {
        if !self.show_labels || width <= 1.0 || height <= 1.0 {
            return Vec::new();
        }
        let Some(world) = &self.world else {
            return Vec::new();
        };
        let Some(view) =
            Mat4::look_at(self.camera.position, self.camera.target(), self.camera.up())
        else {
            return Vec::new();
        };
        let Some(earth) = world.objects.iter().find(|object| {
            world.object_effectively_visible(&object.id)
                && matches!(&object.asset, AssetRef::Mesh { path } if is_native_ocean_sphere(path))
        }) else {
            return Vec::new();
        };

        let model_view = view * self.scene_transform.matrix() * earth.world_matrix();
        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let mut candidates = Vec::<(i32, Vec<MarineGlyphQuad>, [f32; 4])>::new();

        for label in self
            .globe_labels
            .iter()
            .filter(|label| label.kind == LabelKind::Marine)
        {
            let anchor = label.direction.normalized();
            let center_view = model_view.transform_point(anchor * (1.0 + label.altitude));
            if center_view.z >= -CAMERA_NEAR {
                continue;
            }
            let view_normal = model_view.transform_vector(anchor).normalized();
            let to_camera = (center_view * -1.0).normalized();
            if view_normal.dot(to_camera) <= 0.30 {
                continue;
            }

            let world_up = Vec3::new(0.0, 1.0, 0.0);
            let mut anchor_east = world_up.cross(anchor).normalized();
            if anchor_east.length_squared() <= f32::EPSILON {
                anchor_east = Vec3::new(1.0, 0.0, 0.0);
            }

            // The label loader already applies Initial Caps. Preserve that
            // exact casing when producing one spherical quad per character.
            let characters = label.text.chars().collect::<Vec<_>>();
            if characters.is_empty() {
                continue;
            }

            // Each character is its own small spherical patch. The baseline advances
            // over the globe in angular units, and all four corners are normalized
            // back to the Earth radius before camera projection.
            let glyph_advance = (label.font_size * 0.001968).clamp(0.020, 0.048);
            let glyph_half_width = glyph_advance * 0.34;
            let glyph_half_height = (label.font_size * 0.0019).clamp(0.016, 0.032);
            let center_index = (characters.len().saturating_sub(1)) as f32 * 0.5;
            let radius = 1.0 + label.altitude;
            let mut glyphs = Vec::new();
            let mut bounds = [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ];
            let mut label_valid = true;

            for (index, glyph) in characters.into_iter().enumerate() {
                let offset = (index as f32 - center_index) * glyph_advance;
                let glyph_center_direction = (anchor + anchor_east * offset).normalized();
                let mut east = world_up.cross(glyph_center_direction).normalized();
                if east.length_squared() <= f32::EPSILON {
                    east = anchor_east;
                }
                let north = glyph_center_direction.cross(east).normalized();

                let spherical_corner = |x: f32, y: f32| {
                    (glyph_center_direction + east * x + north * y).normalized() * radius
                };
                let local_corners = [
                    spherical_corner(-glyph_half_width, glyph_half_height),
                    spherical_corner(glyph_half_width, glyph_half_height),
                    spherical_corner(glyph_half_width, -glyph_half_height),
                    spherical_corner(-glyph_half_width, -glyph_half_height),
                ];

                let mut corners = [[0.0; 2]; 4];
                for (slot, point) in local_corners.into_iter().enumerate() {
                    let point_view = model_view.transform_point(point);
                    if point_view.z >= -CAMERA_NEAR {
                        label_valid = false;
                        break;
                    }
                    let Some(screen) = project_point(point_view, width, height, focal_length)
                    else {
                        label_valid = false;
                        break;
                    };
                    corners[slot] = screen;
                    bounds[0] = bounds[0].min(screen[0]);
                    bounds[1] = bounds[1].min(screen[1]);
                    bounds[2] = bounds[2].max(screen[0]);
                    bounds[3] = bounds[3].max(screen[1]);
                }
                if !label_valid {
                    break;
                }
                glyphs.push(MarineGlyphQuad { glyph, corners });
            }

            if !label_valid || glyphs.is_empty() {
                continue;
            }
            if bounds[2] < 0.0 || bounds[0] > width || bounds[3] < 0.0 || bounds[1] > height {
                continue;
            }
            candidates.push((label.priority, glyphs, bounds));
        }

        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let mut accepted_bounds = Vec::<[f32; 4]>::new();
        let mut accepted = Vec::<MarineGlyphQuad>::new();
        let mut accepted_labels = 0usize;
        for (_priority, glyphs, bounds) in candidates {
            let overlaps = accepted_bounds.iter().any(|other| {
                bounds[0] < other[2]
                    && bounds[2] > other[0]
                    && bounds[1] < other[3]
                    && bounds[3] > other[1]
            });
            if overlaps {
                continue;
            }
            accepted_bounds.push(bounds);
            accepted.extend(glyphs);
            accepted_labels += 1;
            if accepted_labels >= 24 {
                break;
            }
        }
        accepted
    }

    pub(crate) fn marine_glyph_texture(
        &mut self,
        context: &egui::Context,
        glyph: char,
    ) -> egui::TextureHandle {
        let key = glyph.to_string();
        if let Some(texture) = self.marine_label_textures.get(&key) {
            return texture.clone();
        }
        let image = rasterize_marine_label(&key);
        let texture = context.load_texture(
            format!("marine-glyph:{key}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        self.marine_label_textures.insert(key, texture.clone());
        texture
    }

    pub(crate) fn viewport_stars(&self, width: f32, height: f32) -> Vec<ViewportStar> {
        let (right, up) = self.camera.view_axes();
        self.starfield.project(
            self.camera.forward(),
            right,
            up,
            width,
            height,
            CAMERA_FOV_DEGREES,
        )
    }

    pub(crate) fn star_count(&self) -> usize {
        self.starfield.count()
    }

    pub(crate) fn object_toon_material(
        &self,
        target: &NativeEditorTarget,
    ) -> Option<ToonMaterialConfig> {
        let NativeEditorTarget::Object(id) = target else {
            return None;
        };
        Some(
            self.effective_toon_material(id)
                .unwrap_or_else(|| fallback_toon_material(id)),
        )
    }

    pub(crate) fn object_uses_fallback_toon_material(&self, target: &NativeEditorTarget) -> bool {
        let NativeEditorTarget::Object(id) = target else {
            return false;
        };
        self.effective_toon_material(id).is_none()
    }

    pub(crate) fn set_object_toon_material(
        &mut self,
        target: &NativeEditorTarget,
        material: ToonMaterialConfig,
    ) -> bool {
        let NativeEditorTarget::Object(id) = target else {
            return false;
        };
        let Some(world) = &mut self.world else {
            return false;
        };

        let descendant_prefix = format!("{id}/");
        let mut updated = 0usize;
        for object in &mut world.objects {
            if object.id == *id || object.id.starts_with(&descendant_prefix) {
                object.render.toon = Some(material);
                updated += 1;
            }
        }

        if updated == 0 {
            return false;
        }

        self.status = if updated == 1 {
            format!("Updated toon material for {id}")
        } else {
            format!(
                "Updated toon material for {id} and {} descendants",
                updated - 1
            )
        };
        true
    }

    fn effective_toon_material(&self, object_id: &str) -> Option<ToonMaterialConfig> {
        let world = self.world.as_ref()?;
        let mut current = Some(object_id);
        while let Some(id) = current {
            if let Some(material) = world
                .objects
                .iter()
                .find(|object| object.id == id)
                .and_then(|object| object.render.toon)
            {
                return Some(material);
            }
            current = id.rsplit_once('/').map(|(parent, _)| parent);
        }
        None
    }

    pub(crate) fn viewport_gpu_geometry(
        &self,
        width: f32,
        height: f32,
    ) -> (
        Vec<GpuVertex>,
        Vec<GpuVertex>,
        Vec<GpuVertex>,
        Vec<GpuVertex>,
        Vec<GpuVertex>,
        Vec<GpuVertex>,
        u64,
    ) {
        if width <= 1.0 || height <= 1.0 {
            return (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
            );
        }

        let Some(world) = &self.world else {
            return (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
            );
        };
        let Some(view) =
            Mat4::look_at(self.camera.position, self.camera.target(), self.camera.up())
        else {
            return (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
            );
        };

        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let mut fills = Vec::new();
        let mut hulls = Vec::new();
        let mut lines = Vec::new();
        let mut overlay_lines = Vec::new();
        let mut geo_fills = Vec::new();
        let mut atmosphere_vertices = Vec::new();
        let mut geo_revision = 0;

        for object in &world.objects {
            if !world.object_effectively_visible(&object.id) {
                continue;
            }

            let material = self
                .effective_toon_material(&object.id)
                .unwrap_or_else(|| fallback_toon_material(&object.id));
            let outline = [
                material.outline_color[0],
                material.outline_color[1],
                material.outline_color[2],
                1.0,
            ];
            let base_color = material.base_color;
            let line_fill = [base_color[0], base_color[1], base_color[2], 1.0];
            let outline_width = material.outline_width.max(0.0) * self.outline_pixel_width
                / DEFAULT_OUTLINE_PIXEL_WIDTH;
            let inner_line_width = inner_stroke_width(outline_width);
            let outer_line_width = outer_stroke_width(outline_width);
            let model_view = view * self.scene_transform.matrix() * object.world_matrix();

            if let AssetRef::GeoJsonMap { path, radius_scale } = &object.asset {
                let key = geojson_cache_key(path, *radius_scale);

                if material.line_only {
                    let Some(map) = self.geojson_maps.get(&key) else {
                        continue;
                    };
                    append_geojson_lines(
                        &mut overlay_lines,
                        map,
                        model_view,
                        width,
                        height,
                        focal_length,
                        GeoJsonLineStyle {
                            // For line-only GeoJSON overlays, outline_width is the
                            // screen-space stroke width and shade_bands[0] is opacity.
                            // This keeps country, state, and river styles data-driven
                            // from the A3D group without expanding the shared schema.
                            inner_color: [
                                line_fill[0],
                                line_fill[1],
                                line_fill[2],
                                material.shade_bands[0].clamp(0.0, 1.0),
                            ],
                            outer_color: [outline[0], outline[1], outline[2], 0.0],
                            inner_width_pixels: material.outline_width.max(0.08),
                            outer_width_pixels: 0.0,
                            shade_bands: [1.0, 1.0, 1.0],
                            band_thresholds: material.band_thresholds,
                        },
                    );
                    continue;
                }

                // Native Earth land is sampled per fragment from the terrain texture
                // on the ocean sphere. Keep the GeoJSON object for terminal/ASCII use,
                // but do not build a second floating polygon shell in the native view.
                if is_native_shader_land_map(path) {
                    continue;
                }

                let Some(cached_triangles) = self.geojson_render_meshes.get(&key) else {
                    continue;
                };

                // Cache the fully transformed/projected land vertices while the camera,
                // viewport, object transform, scene transform, and material are unchanged.
                // The renderer also uses the revision to avoid re-uploading the same land
                // vertex buffer on continuous egui repaints.
                let cache_key = format!(
                    "{}|{}x{}|{:?}|{:?}|{:?}|{:?}",
                    key,
                    width.to_bits(),
                    height.to_bits(),
                    self.camera,
                    self.scene_transform,
                    object.world_matrix(),
                    material,
                );

                let cached_geometry = self.geo_gpu_geometry_cache.borrow();
                if let Some(cached) = cached_geometry
                    .as_ref()
                    .filter(|cached| cached.key == cache_key)
                {
                    geo_fills.extend_from_slice(&cached.vertices);
                    geo_revision = cached.revision;
                    continue;
                }
                drop(cached_geometry);

                let mut rebuilt = Vec::new();
                for cached in cached_triangles {
                    if geo_triangle_fully_back_facing(cached.points, model_view) {
                        continue;
                    }

                    let mut terrain_vertices = Vec::with_capacity(3);
                    for (local_point, color) in cached.points.into_iter().zip(cached.colors) {
                        let view_point = model_view.transform_point(local_point);
                        let Some(position) =
                            project_point_clip(view_point, width, height, focal_length)
                        else {
                            terrain_vertices.clear();
                            break;
                        };
                        let view_normal = model_view
                            .transform_vector(local_point.normalized())
                            .normalized();
                        terrain_vertices.push(GpuVertex::with_normal(
                            position,
                            color,
                            [view_normal.x, view_normal.y, view_normal.z],
                            material.shade_bands,
                            material.band_thresholds,
                        ));
                    }
                    if terrain_vertices.len() == 3 {
                        rebuilt.extend(terrain_vertices);
                    }
                }

                let revision = self
                    .geo_gpu_geometry_cache
                    .borrow()
                    .as_ref()
                    .map_or(1, |cached| cached.revision.wrapping_add(1).max(1));
                // Interior mutability is limited to this derived render cache; scene data
                // and editor state remain immutable during geometry collection.
                *self.geo_gpu_geometry_cache.borrow_mut() = Some(CachedGeoGpuGeometry {
                    key: cache_key,
                    revision,
                    vertices: rebuilt.clone(),
                });
                geo_fills.extend(rebuilt);
                geo_revision = revision;
                continue;
            }

            let AssetRef::Mesh { path } = &object.asset else {
                continue;
            };
            let Some(mesh) = self.meshes.get(path) else {
                continue;
            };
            let use_analytic_sphere_normals = is_native_ocean_sphere(path);
            let view_vertices = mesh
                .vertices
                .iter()
                .map(|vertex| model_view.transform_point(*vertex))
                .collect::<Vec<_>>();
            let projected = view_vertices
                .iter()
                .map(|point| project_point_clip(*point, width, height, focal_length))
                .collect::<Vec<_>>();

            let mut explicit_lines = Vec::new();
            let mut edge_map: HashMap<(usize, usize), EdgeFaces> = HashMap::new();
            let mut triangles: Vec<([usize; 3], Vec3)> = Vec::new();
            let mut vertex_normals = vec![Vec3::new(0.0, 0.0, 0.0); view_vertices.len()];

            if use_analytic_sphere_normals {
                let center_view = model_view.transform_point(Vec3::new(0.0, 0.0, 0.0));
                if let Some(center_clip) =
                    project_point_clip(center_view, width, height, focal_length)
                {
                    let center_ndc = [
                        center_clip[0] / center_clip[3],
                        center_clip[1] / center_clip[3],
                    ];
                    let radius_ndc = projected
                        .iter()
                        .flatten()
                        .map(|projected_vertex| {
                            let dx = projected_vertex[0] / projected_vertex[3] - center_ndc[0];
                            let dy = projected_vertex[1] / projected_vertex[3] - center_ndc[1];
                            (dx * dx + dy * dy).sqrt()
                        })
                        .fold(0.0_f32, f32::max);
                    if radius_ndc > 0.0 {
                        push_atmosphere_ring(
                            &mut atmosphere_vertices,
                            center_clip,
                            center_ndc,
                            radius_ndc,
                            [0.64, 0.80, 1.00, 0.95],
                        );
                    }
                }
            }

            for primitive in &mesh.faces {
                match primitive.as_slice() {
                    [a, b] => {
                        explicit_lines.push((*a, *b));
                    }
                    indexes if indexes.len() >= 3 => {
                        for triangle in 1..indexes.len() - 1 {
                            let tri = [indexes[0], indexes[triangle], indexes[triangle + 1]];
                            let [a, b, c] = tri.map(|index| view_vertices[index]);
                            let area_normal = (b - a).cross(c - a);
                            if area_normal.length_squared() <= f32::EPSILON {
                                continue;
                            }
                            let normal = area_normal.normalized();
                            let centroid = (a + b + c) * (1.0 / 3.0);
                            let to_camera = (centroid * -1.0).normalized();
                            let front_facing = normal.dot(to_camera) > 0.0;

                            for index in tri {
                                vertex_normals[index] = vertex_normals[index] + area_normal;
                            }
                            for &(ea, eb) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])]
                            {
                                let key = if ea < eb { (ea, eb) } else { (eb, ea) };
                                edge_map.entry(key).or_insert_with(EdgeFaces::default).push(
                                    FaceEdgeData {
                                        front_facing,
                                        normal,
                                    },
                                );
                            }
                            triangles.push((tri, normal));
                        }
                    }
                    _ => {}
                }
            }

            for normal in &mut vertex_normals {
                *normal = normal.normalized();
            }

            // Earth uses a dedicated translucent atmosphere shell. Other objects
            // retain their material-driven inverted-hull outline width.
            let hull_width = if use_analytic_sphere_normals {
                7.0
            } else {
                outline_width
            };
            let outline_x_ndc = (2.0 * hull_width) / width.max(1.0);
            let outline_y_ndc = (2.0 * hull_width) / height.max(1.0);

            for (tri, face_normal) in triangles {
                let mut projected_triangle = [[0.0; 4]; 3];
                let mut projected_hull = [[0.0; 4]; 3];
                let mut normals = [[0.0; 3]; 3];
                let mut valid = true;
                for (slot, index) in tri.into_iter().enumerate() {
                    let Some(projected_vertex) = projected[index] else {
                        valid = false;
                        break;
                    };
                    projected_triangle[slot] = projected_vertex;
                    let normal = if use_analytic_sphere_normals {
                        // The ocean is a mathematically perfect sphere. Derive its
                        // view-space normal from the local vertex direction instead
                        // of imported UV-sphere normals, so latitude rings cannot
                        // restart or crease the lighting gradient.
                        model_view
                            .transform_vector(mesh.vertices[index].normalized())
                            .normalized()
                    } else if material.smooth_shading {
                        vertex_normals[index]
                    } else {
                        face_normal
                    };
                    normals[slot] = [normal.x, normal.y, normal.z];
                    let screen_dir = Vec3::new(normal.x, normal.y, 0.0);
                    let screen_len =
                        (screen_dir.x * screen_dir.x + screen_dir.y * screen_dir.y).sqrt();
                    let (dx, dy) = if screen_len > 1.0e-5 {
                        (
                            screen_dir.x / screen_len * outline_x_ndc,
                            screen_dir.y / screen_len * outline_y_ndc,
                        )
                    } else {
                        (0.0, 0.0)
                    };
                    projected_hull[slot] = [
                        projected_vertex[0] + dx * projected_vertex[3],
                        projected_vertex[1] + dy * projected_vertex[3],
                        projected_vertex[2],
                        projected_vertex[3],
                    ];
                }
                if !valid {
                    continue;
                }

                if !material.line_only {
                    let color = [base_color[0], base_color[1], base_color[2], 1.0];
                    if use_analytic_sphere_normals {
                        // Reuse the exact projected Earth inverted hull as the
                        // atmosphere. Its expansion is measured in screen pixels,
                        // so the glow remains attached to the limb at every zoom.
                        let atmosphere_color = [0.72, 0.88, 1.00, 0.18];
                        for slot in 0..3 {
                            let local = mesh.vertices[tri[slot]].normalized();
                            hulls.push(GpuVertex::with_terrain_surface(
                                lerp_clip(projected_hull[slot], projected_triangle[slot], 0.18),
                                atmosphere_color,
                                normals[slot],
                                [local.x, local.y, local.z],
                                material.shade_bands,
                                material.band_thresholds,
                            ));
                        }
                    } else {
                        for slot in 0..3 {
                            hulls.push(GpuVertex::new(
                                projected_hull[slot],
                                outline,
                                1.0,
                                material.shade_bands,
                                material.band_thresholds,
                            ));
                        }
                    }
                    for slot in 0..3 {
                        if use_analytic_sphere_normals {
                            let local = mesh.vertices[tri[slot]].normalized();
                            fills.push(GpuVertex::with_terrain_surface(
                                projected_triangle[slot],
                                color,
                                normals[slot],
                                [local.x, local.y, local.z],
                                material.shade_bands,
                                material.band_thresholds,
                            ));
                        } else {
                            fills.push(GpuVertex::with_normal(
                                projected_triangle[slot],
                                color,
                                normals[slot],
                                material.shade_bands,
                                material.band_thresholds,
                            ));
                        }
                    }
                }
            }

            for (a, b) in explicit_lines {
                let (Some(start), Some(end)) = (projected[a], projected[b]) else {
                    continue;
                };
                push_bordered_screen_space_stroke(
                    &mut lines,
                    start,
                    end,
                    outer_line_width,
                    inner_line_width,
                    width,
                    height,
                    outline,
                    line_fill,
                    material.shade_bands,
                    material.band_thresholds,
                );
            }

            if self.show_wireframe || material.line_only {
                // Filled rendering uses the native high-resolution mesh, while debug
                // wireframe and line-only rendering use the shared low-resolution
                // mesh as a clean proxy when one is available. Drawing every edge of
                // the 128x64 sphere creates a dense checker pattern and very thick
                // bordered strokes.
                let edge_mesh = self.wire_meshes.get(path).unwrap_or(mesh);
                let edge_projected =
                    project_mesh_vertices(edge_mesh, model_view, width, height, focal_length);
                let stroke_width = if self.show_wireframe {
                    0.48
                } else {
                    inner_line_width.clamp(0.55, 0.90)
                };
                let stroke_color = if material.line_only {
                    line_fill
                } else {
                    outline
                };

                for (a, b) in edge_mesh.unique_edges() {
                    let (Some(start), Some(end)) = (edge_projected[a], edge_projected[b]) else {
                        continue;
                    };
                    push_screen_space_stroke(
                        &mut lines,
                        start,
                        end,
                        stroke_width,
                        width,
                        height,
                        stroke_color,
                        material.shade_bands,
                        material.band_thresholds,
                    );
                }
            } else {
                for ((a, b), faces) in edge_map {
                    // The inverted hull gives curved objects a soft contour,
                    // but very thin or nearly planar meshes (logos/glyphs)
                    // can collapse to almost no visible hull. Draw their
                    // mathematically-derived silhouette edges as a stable
                    // screen-space stroke, along with intentional creases.
                    if !(faces.is_silhouette() || faces.is_major_crease()) {
                        continue;
                    }
                    let (Some(start), Some(end)) = (projected[a], projected[b]) else {
                        continue;
                    };
                    push_screen_space_stroke(
                        &mut lines,
                        start,
                        end,
                        outline_width.max(0.75),
                        width,
                        height,
                        outline,
                        material.shade_bands,
                        material.band_thresholds,
                    );
                }
            }
        }

        (
            fills,
            hulls,
            lines,
            overlay_lines,
            geo_fills,
            atmosphere_vertices,
            geo_revision,
        )
    }

    pub(crate) fn viewport_lines(&self, width: f32, height: f32) -> Vec<ViewportLine> {
        if width <= 1.0 || height <= 1.0 {
            return Vec::new();
        }

        let Some(world) = &self.world else {
            return Vec::new();
        };
        let Some(view) =
            Mat4::look_at(self.camera.position, self.camera.target(), self.camera.up())
        else {
            return Vec::new();
        };

        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let selected = self.session.inspected_target();
        let mut lines = Vec::new();

        for object in &world.objects {
            if !world.object_effectively_visible(&object.id) {
                continue;
            }

            let model_view = view * self.scene_transform.matrix() * object.world_matrix();
            let is_selected = selected
                .and_then(|target| match target {
                    NativeEditorTarget::Object(id) => Some(id.as_str()),
                    _ => None,
                })
                .is_some_and(|id| object.id == id || object.id.starts_with(&format!("{id}/")));

            match &object.asset {
                AssetRef::Mesh { path } => {
                    let Some(mesh) = self.meshes.get(path) else {
                        continue;
                    };
                    let projected = mesh
                        .vertices
                        .iter()
                        .map(|vertex| {
                            project_point(
                                model_view.transform_point(*vertex),
                                width,
                                height,
                                focal_length,
                            )
                        })
                        .collect::<Vec<_>>();

                    for (a, b) in mesh.unique_edges() {
                        let (Some(start), Some(end)) = (projected[a], projected[b]) else {
                            continue;
                        };
                        lines.push(ViewportLine {
                            start,
                            end,
                            selected: is_selected,
                        });
                    }
                }
                AssetRef::GeoJsonMap { path, radius_scale } => {
                    let key = geojson_cache_key(path, *radius_scale);
                    let Some(map) = self.geojson_maps.get(&key) else {
                        continue;
                    };
                    for (start, end) in &map.segments {
                        let (Some(start), Some(end)) = (
                            project_point(
                                model_view.transform_point(*start),
                                width,
                                height,
                                focal_length,
                            ),
                            project_point(
                                model_view.transform_point(*end),
                                width,
                                height,
                                focal_length,
                            ),
                        ) else {
                            continue;
                        };
                        lines.push(ViewportLine {
                            start,
                            end,
                            selected: is_selected,
                        });
                    }
                }
                _ => {}
            }
        }

        lines
    }
}

#[derive(Clone, Copy, Debug)]
struct FaceEdgeData {
    front_facing: bool,
    normal: Vec3,
}

#[derive(Clone, Debug, Default)]
struct EdgeFaces {
    faces: Vec<FaceEdgeData>,
}

impl EdgeFaces {
    fn push(&mut self, face: FaceEdgeData) {
        self.faces.push(face);
    }

    fn is_silhouette(&self) -> bool {
        match self.faces.len() {
            0 => false,
            1 => self.faces[0].front_facing,
            _ => self.faces.iter().filter(|face| face.front_facing).count() == 1,
        }
    }

    fn is_major_crease(&self) -> bool {
        if self.faces.len() < 2 {
            return false;
        }

        let front_faces = self
            .faces
            .iter()
            .copied()
            .filter(|face| face.front_facing)
            .collect::<Vec<_>>();
        if front_faces.len() != 2 {
            return false;
        }

        front_faces[0].normal.dot(front_faces[1].normal) < CREASE_NORMAL_DOT_THRESHOLD
    }
}

fn build_scene_state(scene_path: &Path) -> Result<NativeEditorApp, Box<dyn Error>> {
    let project = load_a3d_project(scene_path)?;
    let world = project
        .into_world()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let entries = editor_entries(&world);
    let title = world.title.clone();
    let object_count = entries.len().saturating_sub(2);
    let (meshes, wire_meshes, mesh_errors) = load_world_meshes(&world);
    let (geojson_maps, geojson_errors) = load_world_geojson_maps(&world);
    let elevation_points_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ELEVATION_POINTS_ASSET);
    let elevation_points =
        load_geojson_elevation_points(&elevation_points_path).unwrap_or_default();
    let elevation_point_count = elevation_points.len();
    let geojson_render_meshes = build_geojson_render_meshes(&geojson_maps, &elevation_points);
    let globe_labels = load_builtin_labels(Path::new(env!("CARGO_MANIFEST_DIR")));
    let starfield = Starfield::load(Path::new(env!("CARGO_MANIFEST_DIR")));
    let mesh_count = meshes.len();
    let geojson_count = geojson_maps.len();

    let mut app = NativeEditorApp {
        session: EditorSession::new(entries, NativeEditorTarget::Camera),
        status: if mesh_errors == 0 && geojson_errors == 0 {
            format!(
                "Loaded {title} ({object_count} editor objects, {mesh_count} mesh assets, {geojson_count} GeoJSON assets, {elevation_point_count} elevation points)"
            )
        } else {
            format!(
                "Loaded {title}; {mesh_errors} mesh and {geojson_errors} GeoJSON asset(s) failed to load"
            )
        },
        world: Some(world),
        meshes,
        wire_meshes,
        geojson_maps,
        geojson_render_meshes,
        globe_labels,
        starfield,
        marine_label_textures: HashMap::new(),
        scene_path: scene_path.to_path_buf(),
        camera: NativeCamera::default(),
        scene_transform: Transform::default(),
        gpu_target_format: None,
        show_wireframe: false,
        show_labels: true,
        outline_pixel_width: DEFAULT_OUTLINE_PIXEL_WIDTH,
        viewport_background_rgb: DEFAULT_VIEWPORT_BACKGROUND_RGB,
        fps_last_frame: Instant::now(),
        fps_smoothed: 0.0,
        geometry_ms_smoothed: 0.0,
        geo_gpu_geometry_cache: RefCell::new(None),
        upload_stats: UploadStats::default(),
    };
    if let Some(bounds) = app.scene_bounds() {
        app.fit_camera_to_bounds(bounds);
    }

    Ok(app)
}

const STROKE_DEPTH_BIAS_NDC: f32 = 1.0e-4;
const LINE_FILL_WIDTH_PIXELS: f32 = 2.0;

fn fallback_toon_material(object_id: &str) -> ToonMaterialConfig {
    let mut material = ToonMaterialConfig::default();
    let id = object_id.to_ascii_lowercase();
    material.base_color = if id.contains("earth") || id.contains("sphere") {
        [0.18, 0.48, 0.92]
    } else if id.contains("teapot") {
        [0.92, 0.28, 0.18]
    } else if id.contains("km") || id.contains("logo") {
        [0.96, 0.76, 0.12]
    } else {
        ToonMaterialConfig::default_base_color()
    };
    material
}

fn inner_stroke_width(_outline_width_pixels: f32) -> f32 {
    LINE_FILL_WIDTH_PIXELS
}

fn outer_stroke_width(outline_width_pixels: f32) -> f32 {
    LINE_FILL_WIDTH_PIXELS + outline_width_pixels.max(0.0) * 2.0
}

fn bias_clip_toward_camera(position: [f32; 4], ndc_bias: f32) -> [f32; 4] {
    [
        position[0],
        position[1],
        (position[2] / position[3] - ndc_bias) * position[3],
        position[3],
    ]
}

fn push_atmosphere_ring(
    _vertices: &mut Vec<GpuVertex>,
    _center_clip: [f32; 4],
    _center_ndc: [f32; 2],
    _radius_ndc: f32,
    _color: [f32; 4],
) {
    // Atmosphere now reuses the Earth inverted hull geometry below.
}

fn lerp_clip(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn push_bordered_screen_space_stroke(
    vertices: &mut Vec<GpuVertex>,
    start: [f32; 4],
    end: [f32; 4],
    outer_width_pixels: f32,
    inner_width_pixels: f32,
    viewport_width: f32,
    viewport_height: f32,
    outer_color: [f32; 4],
    inner_color: [f32; 4],
    shade_bands: [f32; 3],
    band_thresholds: [f32; 2],
) {
    push_screen_space_stroke(
        vertices,
        start,
        end,
        outer_width_pixels,
        viewport_width,
        viewport_height,
        outer_color,
        shade_bands,
        band_thresholds,
    );
    push_screen_space_stroke(
        vertices,
        start,
        end,
        inner_width_pixels,
        viewport_width,
        viewport_height,
        inner_color,
        shade_bands,
        band_thresholds,
    );
}

fn push_screen_space_stroke(
    vertices: &mut Vec<GpuVertex>,
    start: [f32; 4],
    end: [f32; 4],
    width_pixels: f32,
    viewport_width: f32,
    viewport_height: f32,
    color: [f32; 4],
    shade_bands: [f32; 3],
    band_thresholds: [f32; 2],
) {
    if start[3].abs() <= f32::EPSILON || end[3].abs() <= f32::EPSILON {
        return;
    }

    let start = bias_clip_toward_camera(start, STROKE_DEPTH_BIAS_NDC);
    let end = bias_clip_toward_camera(end, STROKE_DEPTH_BIAS_NDC);

    let start_ndc = [start[0] / start[3], start[1] / start[3]];
    let end_ndc = [end[0] / end[3], end[1] / end[3]];
    let dx_pixels = (end_ndc[0] - start_ndc[0]) * viewport_width * 0.5;
    let dy_pixels = (end_ndc[1] - start_ndc[1]) * viewport_height * 0.5;
    let length = (dx_pixels * dx_pixels + dy_pixels * dy_pixels).sqrt();
    if length <= f32::EPSILON {
        return;
    }

    let half_width = width_pixels.max(0.5) * 0.5;
    let offset_x_pixels = -dy_pixels / length * half_width;
    let offset_y_pixels = dx_pixels / length * half_width;
    let offset_ndc = [
        offset_x_pixels * 2.0 / viewport_width.max(1.0),
        offset_y_pixels * 2.0 / viewport_height.max(1.0),
    ];

    let start_a = [
        start[0] + offset_ndc[0] * start[3],
        start[1] + offset_ndc[1] * start[3],
        start[2],
        start[3],
    ];
    let start_b = [
        start[0] - offset_ndc[0] * start[3],
        start[1] - offset_ndc[1] * start[3],
        start[2],
        start[3],
    ];
    let end_a = [
        end[0] + offset_ndc[0] * end[3],
        end[1] + offset_ndc[1] * end[3],
        end[2],
        end[3],
    ];
    let end_b = [
        end[0] - offset_ndc[0] * end[3],
        end[1] - offset_ndc[1] * end[3],
        end[2],
        end[3],
    ];

    for position in [start_a, start_b, end_a, start_b, end_b, end_a] {
        vertices.push(GpuVertex::new(
            position,
            color,
            1.0,
            shade_bands,
            band_thresholds,
        ));
    }
}

fn geo_triangle_fully_back_facing(triangle: [Vec3; 3], model_view: Mat4) -> bool {
    triangle.into_iter().all(|local_point| {
        let view_point = model_view.transform_point(local_point);
        let view_normal = model_view
            .transform_vector(local_point.normalized())
            .normalized();
        let to_camera = (view_point * -1.0).normalized();
        view_normal.dot(to_camera) <= 0.0
    })
}

fn subdivide_spherical_triangle(
    triangle: [Vec3; 3],
    max_angle: f32,
    depth: usize,
    output: &mut Vec<[Vec3; 3]>,
) {
    if depth >= 5 || triangle_max_angle(triangle) <= max_angle {
        output.push(triangle);
        return;
    }
    let radius = (triangle[0].length() + triangle[1].length() + triangle[2].length()) / 3.0;
    let ab = (triangle[0].normalized() + triangle[1].normalized()).normalized() * radius;
    let bc = (triangle[1].normalized() + triangle[2].normalized()).normalized() * radius;
    let ca = (triangle[2].normalized() + triangle[0].normalized()).normalized() * radius;
    for child in [
        [triangle[0], ab, ca],
        [ab, triangle[1], bc],
        [ca, bc, triangle[2]],
        [ab, bc, ca],
    ] {
        subdivide_spherical_triangle(child, max_angle, depth + 1, output);
    }
}

fn triangle_max_angle(triangle: [Vec3; 3]) -> f32 {
    [(0, 1), (1, 2), (2, 0)]
        .into_iter()
        .map(|(a, b)| {
            triangle[a]
                .normalized()
                .dot(triangle[b].normalized())
                .clamp(-1.0, 1.0)
                .acos()
        })
        .fold(0.0, f32::max)
}

fn procedural_terrain_elevation(point: Vec3, elevation_points: &[GeoJsonElevationPoint]) -> f32 {
    let unit = point.normalized();
    let latitude = unit.y.clamp(-1.0, 1.0).asin().to_degrees();
    let longitude = unit.x.atan2(unit.z).to_degrees();

    // Retain a restrained broad field so sparse named peaks do not create
    // isolated circular islands of color between observations.
    let broad_shape = 0.025 * (longitude.to_radians() * 2.1 + latitude.to_radians() * 0.7).sin()
        + 0.018 * (longitude.to_radians() * 4.3 - latitude.to_radians() * 1.9).cos();

    // Natural Earth supplies named elevation points rather than a continuous
    // DEM. Convert each point into a smooth spherical influence. Higher and
    // more important points have a wider influence, while max-combining avoids
    // inflating terrain where several nearby labels overlap.
    let sampled_elevation = elevation_points
        .iter()
        .map(|sample| {
            let angular_distance = unit
                .dot(sample.position)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            let importance = ((10.0 - sample.scale_rank) / 10.0).clamp(0.0, 1.0);
            let normalized_height = (sample.elevation_meters / 5500.0).clamp(0.0, 1.25).sqrt();
            let influence_radius =
                (3.5 + normalized_height * 8.0 + importance * 3.0).clamp(3.5, 14.0);
            let falloff = (-0.5 * (angular_distance / influence_radius).powi(2)).exp();
            normalized_height * falloff
        })
        .fold(0.0_f32, f32::max);

    (0.16 + broad_shape + sampled_elevation * 0.78).clamp(0.0, 1.0)
}

fn procedural_terrain_color(point: Vec3, elevation: f32) -> [f32; 4] {
    let unit = point.normalized();
    let latitude = unit.y.clamp(-1.0, 1.0).asin().to_degrees();
    let longitude = unit.x.atan2(unit.z).to_degrees();
    let subtropical_dryness = ((latitude.abs() - 27.0).abs() / 18.0).clamp(0.0, 1.0);
    let regional_variation =
        0.5 + 0.5 * (longitude.to_radians() * 2.7 + latitude.to_radians() * 1.4).sin();
    let moisture =
        (0.68 - 0.24 * (1.0 - subtropical_dryness) + 0.16 * regional_variation).clamp(0.0, 1.0);

    let lush_lowland = [0.16, 0.48, 0.20];
    let dry_lowland = [0.48, 0.50, 0.20];
    let foothill = [0.57, 0.46, 0.22];
    let mountain = [0.43, 0.27, 0.13];
    let high_peak = [0.66, 0.57, 0.43];

    let lowland = lerp_rgb(dry_lowland, lush_lowland, moisture);
    let color = if elevation < 0.32 {
        lerp_rgb(lowland, foothill, elevation / 0.32)
    } else if elevation < 0.62 {
        lerp_rgb(foothill, mountain, (elevation - 0.32) / 0.30)
    } else {
        lerp_rgb(mountain, high_peak, (elevation - 0.62) / 0.38)
    };
    [color[0], color[1], color[2], 1.0]
}

fn wrapped_longitude_delta(longitude: f32, reference: f32) -> f32 {
    let mut delta = longitude - reference;
    while delta > 180.0 {
        delta -= 360.0;
    }
    while delta < -180.0 {
        delta += 360.0;
    }
    delta
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], amount: f32) -> [f32; 3] {
    let amount = amount.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * amount,
        a[1] + (b[1] - a[1]) * amount,
        a[2] + (b[2] - a[2]) * amount,
    ]
}

fn triangulate_geojson_polygon(polygon: &crate::geojson::GeoJsonPolygon) -> Vec<[usize; 3]> {
    // For land fill, triangulate only the exterior ring.
    // Natural Earth land holes are better restored explicitly from the lakes
    // layer than inferred from very large projected polygons, which can create
    // continent-scale blue cutouts when hole winding or projection gets noisy.
    let exterior_end = polygon
        .hole_indices
        .first()
        .copied()
        .unwrap_or(polygon.projected.len());
    if exterior_end < 3 {
        return Vec::new();
    }

    let exterior = &polygon.projected[..exterior_end];
    let coordinates = exterior
        .iter()
        .flat_map(|point| [point[0] as f64, point[1] as f64])
        .collect::<Vec<_>>();

    let Ok(indices) = earcutr::earcut(&coordinates, &[], 2) else {
        return Vec::new();
    };

    indices
        .chunks_exact(3)
        .filter_map(|triangle| {
            let tri = [triangle[0], triangle[1], triangle[2]];
            let centroid = [
                (exterior[tri[0]][0] + exterior[tri[1]][0] + exterior[tri[2]][0]) / 3.0,
                (exterior[tri[0]][1] + exterior[tri[1]][1] + exterior[tri[2]][1]) / 3.0,
            ];
            point_in_ring(centroid, exterior).then_some(tri)
        })
        .collect()
}

fn point_in_ring(point: [f32; 2], ring: &[[f32; 2]]) -> bool {
    if ring.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = *ring.last().unwrap();
    for &current in ring {
        let intersects = ((current[1] > point[1]) != (previous[1] > point[1]))
            && (point[0]
                < (previous[0] - current[0]) * (point[1] - current[1])
                    / (previous[1] - current[1]).max(1.0e-6)
                    + current[0]);
        if intersects {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn project_mesh_vertices(
    mesh: &Mesh,
    model_view: Mat4,
    width: f32,
    height: f32,
    focal_length: f32,
) -> Vec<Option<[f32; 4]>> {
    mesh.vertices
        .iter()
        .map(|vertex| model_view.transform_point(*vertex))
        .map(|point| project_point_clip(point, width, height, focal_length))
        .collect()
}

fn project_point_clip(point: Vec3, width: f32, height: f32, focal_length: f32) -> Option<[f32; 4]> {
    let screen = project_point(point, width, height, focal_length)?;
    let depth = -point.z;
    let far = (MAX_CAMERA_DISTANCE * 4.0).max(CAMERA_NEAR + 1.0);
    let depth_ndc = (far / (far - CAMERA_NEAR)
        - (far * CAMERA_NEAR) / ((far - CAMERA_NEAR) * depth))
        .clamp(0.0, 1.0);
    let x_ndc = screen[0] / width * 2.0 - 1.0;
    let y_ndc = 1.0 - screen[1] / height * 2.0;

    Some([x_ndc * depth, y_ndc * depth, depth_ndc * depth, depth])
}

fn project_point(point: Vec3, width: f32, height: f32, focal_length: f32) -> Option<[f32; 2]> {
    if point.z >= -CAMERA_NEAR {
        return None;
    }

    let depth = -point.z;
    let rect_x = point.x / depth;
    let rect_y = point.y / depth;

    // Cylindrical coordinates grow by viewing angle rather than tan(angle),
    // reducing wide-angle stretching near the horizontal viewport edges.
    let horizontal_radius = (point.x * point.x + depth * depth).sqrt();
    let cylindrical_x = point.x.atan2(depth);
    let cylindrical_y = point.y / horizontal_radius.max(CAMERA_NEAR);

    // Drive correction from the point's camera-space angle, not from the
    // current viewport width. This keeps the same 3D point at the same
    // projected scale when the window merely gains additional real estate.
    let edge_t = ((cylindrical_x.abs() - EDGE_CORRECTION_START_RADIANS)
        / (EDGE_CORRECTION_FULL_RADIANS - EDGE_CORRECTION_START_RADIANS))
        .clamp(0.0, 1.0);
    let smooth_edge = edge_t * edge_t * (3.0 - 2.0 * edge_t);
    let blend = EDGE_CORRECTION_STRENGTH * smooth_edge;

    let corrected_x = rect_x + (cylindrical_x - rect_x) * blend;
    let corrected_y = rect_y + (cylindrical_y - rect_y) * blend;

    Some([
        width * 0.5 + corrected_x * focal_length,
        height * 0.5 - corrected_y * focal_length,
    ])
}

fn load_world_meshes(world: &LoadedWorld) -> (HashMap<String, Mesh>, HashMap<String, Mesh>, usize) {
    let mut meshes = HashMap::new();
    let mut wire_meshes = HashMap::new();
    let mut errors = 0;

    for object in &world.objects {
        let AssetRef::Mesh { path } = &object.asset else {
            continue;
        };
        if meshes.contains_key(path) {
            continue;
        }

        let native_path = native_mesh_override_path(path);
        match load_obj(&native_path) {
            Ok(mesh) => {
                // Keep the resolved A3D asset path as the cache key. Filled native
                // rendering uses the higher-resolution override.
                meshes.insert(path.clone(), mesh);

                // When an override is active, retain the shared low-resolution mesh
                // as a wireframe/line-only proxy. This keeps debug lines readable.
                if native_path != Path::new(path) {
                    if let Ok(proxy) = load_obj(path) {
                        wire_meshes.insert(path.clone(), proxy);
                    }
                }
            }
            Err(_) => errors += 1,
        }
    }

    (meshes, wire_meshes, errors)
}

fn is_native_ocean_sphere(path: &str) -> bool {
    Path::new(path).file_name().and_then(|name| name.to_str()) == Some("sphere_uv_32x16.obj")
}

fn native_mesh_override_path(path: &str) -> PathBuf {
    Path::new(path).to_path_buf()
}

fn is_native_shader_land_map(path: &str) -> bool {
    path.contains("ne_50m_land") || path.contains("admin_0_countries")
}

fn geojson_cache_key(path: &str, radius_scale: f32) -> String {
    format!("{path}#{radius_scale:.6}")
}

fn build_geojson_render_meshes(
    maps: &HashMap<String, GeoJsonMap>,
    elevation_points: &[GeoJsonElevationPoint],
) -> HashMap<String, Vec<CachedGeoTriangle>> {
    let mut cached_maps = HashMap::new();

    for (key, map) in maps {
        if is_native_shader_land_map(key) {
            cached_maps.insert(key.clone(), Vec::new());
            continue;
        }
        let mut cached_triangles = Vec::new();
        for polygon in &map.polygons {
            for [a, b, c] in triangulate_geojson_polygon(polygon) {
                let mut surface_triangles = Vec::new();
                subdivide_spherical_triangle(
                    [polygon.points[a], polygon.points[b], polygon.points[c]],
                    15.0_f32.to_radians(),
                    0,
                    &mut surface_triangles,
                );
                for points in surface_triangles {
                    let elevations =
                        points.map(|point| procedural_terrain_elevation(point, elevation_points));
                    let colors = [
                        procedural_terrain_color(points[0], elevations[0]),
                        procedural_terrain_color(points[1], elevations[1]),
                        procedural_terrain_color(points[2], elevations[2]),
                    ];
                    cached_triangles.push(CachedGeoTriangle { points, colors });
                }
            }
        }
        cached_maps.insert(key.clone(), cached_triangles);
    }

    cached_maps
}

fn load_world_geojson_maps(world: &LoadedWorld) -> (HashMap<String, GeoJsonMap>, usize) {
    let mut maps = HashMap::new();
    let mut errors = 0;

    for object in &world.objects {
        let AssetRef::GeoJsonMap { path, radius_scale } = &object.asset else {
            continue;
        };
        let key = geojson_cache_key(path, *radius_scale);
        if maps.contains_key(&key) {
            continue;
        }
        match load_geojson_map(path, *radius_scale) {
            Ok(map) => {
                maps.insert(key, map);
            }
            Err(_) => errors += 1,
        }
    }

    (maps, errors)
}

fn editor_entries(world: &LoadedWorld) -> Vec<EditorEntry<NativeEditorTarget>> {
    let mut entries = vec![
        EditorEntry::new(NativeEditorTarget::Scene, None),
        EditorEntry::new(NativeEditorTarget::Camera, None),
    ];

    entries.extend(
        world
            .objects
            .iter()
            .filter(|object| !object.editor_hidden)
            .map(|object| {
                EditorEntry::new(
                    NativeEditorTarget::Object(object.id.clone()),
                    Some(object.render.visible),
                )
            }),
    );

    entries
}

impl eframe::App for NativeEditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        super::gui::draw(self, ui, frame);
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ascii-3d Native Editor")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 520.0]),
        renderer: eframe::Renderer::Wgpu,
        depth_buffer: 24,
        ..Default::default()
    };

    eframe::run_native(
        "ascii-3d Native Editor",
        options,
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
            let mut app = NativeEditorApp::default();
            if let Some(render_state) = &creation_context.wgpu_render_state {
                app.set_gpu_target_format(render_state.target_format);
            }
            Ok(Box::new(app))
        }),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec3_near(left: Vec3, right: Vec3) {
        const EPSILON: f32 = 0.00001;
        assert!(
            (left - right).length() < EPSILON,
            "expected {left:?} to be within {EPSILON} of {right:?}"
        );
    }

    #[test]
    fn edge_correction_leaves_optical_center_unchanged() {
        let focal = 600.0;
        let projected = project_point(Vec3::new(0.0, 1.0, -4.0), 1200.0, 800.0, focal)
            .expect("point should project");

        assert!((projected[0] - 600.0).abs() < 0.001);
        assert!((projected[1] - 250.0).abs() < 0.001);
    }

    #[test]
    fn edge_correction_reduces_rectilinear_horizontal_stretch() {
        let width = 1200.0;
        let height = 800.0;
        let focal = 600.0;
        let point = Vec3::new(3.0, 0.0, -4.0);
        let projected = project_point(point, width, height, focal).expect("point should project");
        let rectilinear_x = width * 0.5 + point.x * focal / -point.z;

        assert!(projected[0] < rectilinear_x);
        assert!(projected[0] > width * 0.5);
    }

    #[test]
    fn startup_scene_populates_real_editor_objects() {
        let app = NativeEditorApp::default();

        assert_eq!(app.scene_title(), "Earth and KM Logo");
        assert!(app.editor_object_count() >= 2);
        assert!(
            app.session
                .entries()
                .iter()
                .any(|entry| { entry.target == NativeEditorTarget::Object("earth".to_owned()) })
        );
    }

    #[test]
    fn startup_scene_loads_mesh_assets_and_projects_lines() {
        let app = NativeEditorApp::default();

        assert!(app.mesh_asset_count() >= 2);
        assert!(!app.viewport_lines(800.0, 600.0).is_empty());
    }

    #[test]
    fn resizing_viewport_preserves_projected_object_scale() {
        let app = NativeEditorApp::default();

        let small = app.viewport_lines(800.0, 600.0);
        let large = app.viewport_lines(1200.0, 900.0);

        let small_line = small.first().expect("startup scene should project lines");
        let large_line = large.first().expect("startup scene should project lines");

        let small_length = ((small_line.end[0] - small_line.start[0]).powi(2)
            + (small_line.end[1] - small_line.start[1]).powi(2))
        .sqrt();
        let large_length = ((large_line.end[0] - large_line.start[0]).powi(2)
            + (large_line.end[1] - large_line.start[1]).powi(2))
        .sqrt();

        assert!((small_length - large_length).abs() < 0.001);
    }

    #[test]
    fn visibility_command_updates_loaded_world_and_viewport() {
        let mut app = NativeEditorApp::default();
        let target = NativeEditorTarget::Object("earth".to_owned());
        let before = app.viewport_lines(800.0, 600.0).len();

        assert!(app.apply_editor_command(EditorCommand::SetVisibility {
            target: target.clone(),
            visible: false,
        }));
        assert_eq!(app.session.visibility(&target), Some(false));
        assert_eq!(
            app.world
                .as_ref()
                .and_then(|world| world.object("earth"))
                .map(|object| object.render.visible),
            Some(false)
        );
        assert!(app.viewport_lines(800.0, 600.0).len() < before);
    }
    #[test]
    fn load_scene_replaces_the_current_world() {
        let mut app = NativeEditorApp::default();
        let scene_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/a3d/earth/scene.a3d");

        assert!(app.load_scene(scene_path.clone()));
        assert_eq!(app.scene_path(), scene_path.as_path());
        assert_eq!(app.scene_title(), "Earth");
        assert!(
            app.session
                .entries()
                .iter()
                .any(|entry| entry.target == NativeEditorTarget::Object("earth".to_owned()))
        );
    }

    #[test]
    fn zoom_changes_camera_distance_without_mutating_scene_objects() {
        let mut app = NativeEditorApp::default();
        let earth_before = app
            .world
            .as_ref()
            .and_then(|world| world.object("earth"))
            .map(|object| object.transform)
            .expect("earth transform");

        let distance_before = app.camera_distance();
        app.zoom_camera(120.0);

        assert!(app.camera_distance() < distance_before);
        assert_eq!(
            app.world
                .as_ref()
                .and_then(|world| world.object("earth"))
                .map(|object| object.transform),
            Some(earth_before)
        );
    }

    #[test]
    fn reset_camera_restores_default_navigation_state() {
        let mut app = NativeEditorApp::default();
        let expected = app.camera;
        app.look_camera([40.0, -20.0]);
        app.pan_camera([25.0, 10.0]);
        app.zoom_camera(-120.0);

        app.reset_camera();

        assert_eq!(app.camera, expected);
    }

    #[test]
    fn right_drag_look_keeps_camera_position_fixed() {
        let mut app = NativeEditorApp::default();
        let position_before = app.camera_position();

        app.look_camera([40.0, -20.0]);

        assert_eq!(app.camera_position(), position_before);
        assert_ne!(app.camera_target(), DEFAULT_CAMERA_TARGET);
    }

    #[test]
    fn focus_orbit_keeps_focus_target_fixed() {
        let mut app = NativeEditorApp::default();
        let target_before = app.camera_target();
        let position_before = app.camera_position();

        app.orbit_focus([40.0, -20.0]);

        assert_vec3_near(app.camera_target(), target_before);
        assert_ne!(app.camera_position(), position_before);
    }

    #[test]
    fn frame_selected_fits_object_bounds_without_mutating_scene() {
        let mut app = NativeEditorApp::default();
        let earth_target = NativeEditorTarget::Object("earth".to_owned());
        let earth_index = app
            .session
            .entries()
            .iter()
            .position(|entry| entry.target == earth_target)
            .expect("earth hierarchy entry");
        let transform_before = app
            .object_transform(&earth_target)
            .expect("earth transform");
        let expected_bounds = app.object_bounds("earth").expect("earth bounds");
        let expected_target = expected_bounds.center();
        let scene_distance = app.camera_distance();

        assert!(app.apply_editor_command(EditorCommand::SelectIndex(earth_index)));
        assert!(app.apply_editor_command(EditorCommand::InspectSelected));
        assert!(app.frame_selected());

        assert_vec3_near(app.camera_target(), expected_target);
        assert!(app.camera_distance() < scene_distance);
        assert_eq!(app.object_transform(&earth_target), Some(transform_before));
    }

    #[test]
    fn startup_camera_distance_is_derived_from_scene_bounds() {
        let app = NativeEditorApp::default();
        let bounds = app.scene_bounds().expect("startup scene bounds");
        let expected = (bounds.radius() / (0.5 * CAMERA_FOV_DEGREES.to_radians()).sin()
            * CAMERA_FIT_PADDING)
            .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);

        assert!((app.camera_distance() - expected).abs() < 0.001);
        assert_vec3_near(app.camera_target(), bounds.center());
    }

    #[test]
    fn failed_load_preserves_the_current_world() {
        let mut app = NativeEditorApp::default();
        let previous_title = app.scene_title().to_owned();
        let previous_path = app.scene_path().to_path_buf();
        let missing = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("missing-scene.a3d");

        assert!(!app.load_scene(missing));
        assert_eq!(app.scene_title(), previous_title);
        assert_eq!(app.scene_path(), previous_path.as_path());
        assert!(app.status.starts_with("Failed to load"));
    }
}
