use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
};

use super::gpu_renderer::GpuVertex;

use crate::{
    a3d::{AssetRef, LoadedWorld, Transform, load_a3d_project},
    editor_core::{EditorCommand, EditorEntry, EditorSession},
    math::{Mat4, Vec3},
    mesh::Mesh,
    obj::load_obj,
};

const STARTUP_SCENE: &str = "assets/a3d/earth_km/scene.a3d";
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
const DEFAULT_VIEWPORT_BACKGROUND_RGB: [u8; 3] = [52, 56, 64];

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
    position: Vec3,
    yaw_radians: f32,
    pitch_radians: f32,
    focus_distance: f32,
}

impl Default for NativeCamera {
    fn default() -> Self {
        Self {
            position: DEFAULT_CAMERA_TARGET + Vec3::new(0.0, 0.0, DEFAULT_CAMERA_DISTANCE),
            yaw_radians: 0.0,
            pitch_radians: 0.0,
            focus_distance: DEFAULT_CAMERA_DISTANCE,
        }
    }
}

impl NativeCamera {
    fn forward(self) -> Vec3 {
        let horizontal = self.pitch_radians.cos();
        Vec3::new(
            horizontal * self.yaw_radians.sin(),
            self.pitch_radians.sin(),
            -horizontal * self.yaw_radians.cos(),
        )
        .normalized()
    }

    fn target(self) -> Vec3 {
        self.position + self.forward() * self.focus_distance
    }

    fn view_axes(self) -> (Vec3, Vec3) {
        let forward = self.forward();
        let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();
        (right, up)
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

pub(crate) struct NativeEditorApp {
    pub(crate) session: EditorSession<NativeEditorTarget>,
    pub(crate) status: String,
    world: Option<LoadedWorld>,
    meshes: HashMap<String, Mesh>,
    scene_path: PathBuf,
    camera: NativeCamera,
    gpu_target_format: Option<egui_wgpu::wgpu::TextureFormat>,
    show_wireframe: bool,
    outline_pixel_width: f32,
    viewport_background_rgb: [u8; 3],
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
            scene_path,
            camera: NativeCamera::default(),
            gpu_target_format: None,
            show_wireframe: false,
            outline_pixel_width: DEFAULT_OUTLINE_PIXEL_WIDTH,
            viewport_background_rgb: DEFAULT_VIEWPORT_BACKGROUND_RGB,
        }
    }

    pub(crate) fn load_scene(&mut self, scene_path: PathBuf) -> bool {
        match build_scene_state(&scene_path) {
            Ok(mut state) => {
                state.gpu_target_format = self.gpu_target_format;
                state.show_wireframe = self.show_wireframe;
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
            let AssetRef::Mesh { path } = &object.asset else {
                continue;
            };
            let Some(mesh) = self.meshes.get(path) else {
                continue;
            };
            let world_matrix = object.world_matrix();

            for vertex in &mesh.vertices {
                let point = world_matrix.transform_point(*vertex);
                match &mut bounds {
                    Some(bounds) => bounds.include(point),
                    None => bounds = Some(CameraBounds::from_point(point)),
                }
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

    pub(crate) fn viewport_gpu_geometry(
        &self,
        width: f32,
        height: f32,
    ) -> (Vec<GpuVertex>, Vec<GpuVertex>, Vec<GpuVertex>) {
        if width <= 1.0 || height <= 1.0 {
            return (Vec::new(), Vec::new(), Vec::new());
        }

        let Some(world) = &self.world else {
            return (Vec::new(), Vec::new(), Vec::new());
        };
        let Some(view) = Mat4::look_at(
            self.camera.position,
            self.camera.target(),
            Vec3::new(0.0, 1.0, 0.0),
        ) else {
            return (Vec::new(), Vec::new(), Vec::new());
        };

        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let selected = self.session.inspected_target();
        let light_direction = Vec3::new(-0.35, 0.75, 0.55).normalized();
        let mut fills = Vec::new();
        let mut hulls = Vec::new();
        let mut lines = Vec::new();

        for object in &world.objects {
            let AssetRef::Mesh { path } = &object.asset else {
                continue;
            };
            if !world.object_effectively_visible(&object.id) {
                continue;
            }
            let Some(mesh) = self.meshes.get(path) else {
                continue;
            };

            let model_view = view * object.world_matrix();
            let is_selected = selected
                .and_then(|target| match target {
                    NativeEditorTarget::Object(id) => Some(id.as_str()),
                    _ => None,
                })
                .is_some_and(|id| object.id == id || object.id.starts_with(&format!("{id}/")));
            let base_color = toon_base_color(&object.id, is_selected);
            let view_vertices = mesh
                .vertices
                .iter()
                .map(|vertex| model_view.transform_point(*vertex))
                .collect::<Vec<_>>();
            let projected = view_vertices
                .iter()
                .map(|point| project_point_clip(*point, width, height, focal_length))
                .collect::<Vec<_>>();

            let outline = if is_selected {
                [0.45, 0.16, 0.04, 1.0]
            } else {
                [0.08, 0.09, 0.11, 1.0]
            };

            let mut explicit_lines = Vec::new();
            let mut edge_map: HashMap<(usize, usize), EdgeFaces> = HashMap::new();
            let mut triangles = Vec::new();
            let mut vertex_normals = vec![Vec3::new(0.0, 0.0, 0.0); view_vertices.len()];

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
                            triangles.push(tri);
                        }
                    }
                    _ => {}
                }
            }

            for normal in &mut vertex_normals {
                *normal = normal.normalized();
            }

            let outline_x_ndc = (2.0 * self.outline_pixel_width) / width.max(1.0);
            let outline_y_ndc = (2.0 * self.outline_pixel_width) / height.max(1.0);

            for tri in triangles {
                let mut projected_triangle = [[0.0; 4]; 3];
                let mut projected_hull = [[0.0; 4]; 3];
                let mut lights = [0.0; 3];
                let mut valid = true;
                for (slot, index) in tri.into_iter().enumerate() {
                    let Some(projected_vertex) = projected[index] else {
                        valid = false;
                        break;
                    };
                    projected_triangle[slot] = projected_vertex;
                    let normal = vertex_normals[index];
                    lights[slot] = normal.dot(light_direction).abs();
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

                let color = [base_color[0], base_color[1], base_color[2], 1.0];
                for slot in 0..3 {
                    hulls.push(GpuVertex::new(projected_hull[slot], outline, 1.0));
                }
                for slot in 0..3 {
                    fills.push(GpuVertex::new(
                        projected_triangle[slot],
                        color,
                        lights[slot],
                    ));
                }
            }

            for (a, b) in explicit_lines {
                let (Some(start), Some(end)) = (projected[a], projected[b]) else {
                    continue;
                };
                lines.push(GpuVertex::new(start, outline, 1.0));
                lines.push(GpuVertex::new(end, outline, 1.0));
            }

            if self.show_wireframe {
                for (a, b) in mesh.unique_edges() {
                    let (Some(start), Some(end)) = (projected[a], projected[b]) else {
                        continue;
                    };
                    lines.push(GpuVertex::new(start, outline, 1.0));
                    lines.push(GpuVertex::new(end, outline, 1.0));
                }
            } else {
                for ((a, b), faces) in edge_map {
                    if !faces.is_major_crease() {
                        continue;
                    }
                    let (Some(start), Some(end)) = (projected[a], projected[b]) else {
                        continue;
                    };
                    lines.push(GpuVertex::new(start, outline, 1.0));
                    lines.push(GpuVertex::new(end, outline, 1.0));
                }
            }
        }

        (fills, hulls, lines)
    }

    pub(crate) fn viewport_lines(&self, width: f32, height: f32) -> Vec<ViewportLine> {
        if width <= 1.0 || height <= 1.0 {
            return Vec::new();
        }

        let Some(world) = &self.world else {
            return Vec::new();
        };
        let Some(view) = Mat4::look_at(
            self.camera.position,
            self.camera.target(),
            Vec3::new(0.0, 1.0, 0.0),
        ) else {
            return Vec::new();
        };

        let focal_length =
            0.5 * REFERENCE_VIEWPORT_HEIGHT / (0.5 * CAMERA_FOV_DEGREES.to_radians()).tan();
        let selected = self.session.inspected_target();
        let mut lines = Vec::new();

        for object in &world.objects {
            let AssetRef::Mesh { path } = &object.asset else {
                continue;
            };
            if !world.object_effectively_visible(&object.id) {
                continue;
            }
            let Some(mesh) = self.meshes.get(path) else {
                continue;
            };

            let model_view = view * object.world_matrix();
            let is_selected = selected
                .and_then(|target| match target {
                    NativeEditorTarget::Object(id) => Some(id.as_str()),
                    _ => None,
                })
                .is_some_and(|id| object.id == id || object.id.starts_with(&format!("{id}/")));

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
    let (meshes, mesh_errors) = load_world_meshes(&world);
    let mesh_count = meshes.len();

    let mut app = NativeEditorApp {
        session: EditorSession::new(entries, NativeEditorTarget::Camera),
        status: if mesh_errors == 0 {
            format!("Loaded {title} ({object_count} editor objects, {mesh_count} mesh assets)")
        } else {
            format!("Loaded {title}; {mesh_errors} mesh asset(s) failed to load")
        },
        world: Some(world),
        meshes,
        scene_path: scene_path.to_path_buf(),
        camera: NativeCamera::default(),
        gpu_target_format: None,
        show_wireframe: false,
        outline_pixel_width: DEFAULT_OUTLINE_PIXEL_WIDTH,
        viewport_background_rgb: DEFAULT_VIEWPORT_BACKGROUND_RGB,
    };
    if let Some(bounds) = app.scene_bounds() {
        app.fit_camera_to_bounds(bounds);
    }

    Ok(app)
}

fn toon_base_color(object_id: &str, selected: bool) -> [f32; 3] {
    if selected {
        return [1.0, 0.48, 0.10];
    }

    let id = object_id.to_ascii_lowercase();
    if id.contains("earth") || id.contains("sphere") {
        [0.18, 0.48, 0.92]
    } else if id.contains("teapot") {
        [0.92, 0.28, 0.18]
    } else if id.contains("km") || id.contains("logo") {
        [0.96, 0.76, 0.12]
    } else {
        [0.30, 0.72, 0.52]
    }
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

fn load_world_meshes(world: &LoadedWorld) -> (HashMap<String, Mesh>, usize) {
    let mut meshes = HashMap::new();
    let mut errors = 0;

    for object in &world.objects {
        let AssetRef::Mesh { path } = &object.asset else {
            continue;
        };
        if meshes.contains_key(path) {
            continue;
        }

        match load_obj(path) {
            Ok(mesh) => {
                meshes.insert(path.clone(), mesh);
            }
            Err(_) => errors += 1,
        }
    }

    (meshes, errors)
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
