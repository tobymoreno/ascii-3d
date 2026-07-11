#[derive(Clone, Debug)]
pub struct RenderScene {
    pub name: String,
    pub display: RenderDisplay,
    pub lighting: Option<RenderLighting>,
    pub objects: Vec<RenderObject>,
    pub overlays: Vec<RenderOverlay>,
    pub cameras: Vec<RenderCamera>,
    pub active_camera_id: Option<String>,
}

impl RenderScene {
    pub fn new(name: impl Into<String>, display: RenderDisplay) -> Self {
        Self {
            name: name.into(),
            display,
            lighting: None,
            objects: Vec::new(),
            overlays: Vec::new(),
            cameras: Vec::new(),
            active_camera_id: None,
        }
    }

    pub fn active_camera(&self) -> Option<&RenderCamera> {
        let active_camera_id = self.active_camera_id.as_deref()?;

        self.cameras
            .iter()
            .find(|camera| camera.id == active_camera_id)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderDisplay {
    pub world_scale: f32,
}

#[derive(Clone, Debug)]
pub struct RenderCamera {
    pub id: String,
    pub transform: RenderTransform,
    pub projection: RenderProjectionConfig,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderProjectionConfig {
    pub camera_distance: f32,
    pub near_clip: f32,
    pub vertical_center_ratio: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderLighting {
    pub primary_light_direction: [f32; 3],
}

#[derive(Clone, Debug)]
pub enum RenderObject {
    Mesh(RenderMeshObject),
    QuadGroup(RenderQuadGroup),
}

#[derive(Clone, Debug)]
pub struct RenderMeshObject {
    pub mesh_asset: String,
    pub transform: RenderTransform,
}

#[derive(Clone, Debug)]
pub struct RenderQuadGroup {
    pub quads: Vec<RenderQuad>,
    pub transform: RenderTransform,
}

#[derive(Clone, Debug)]
pub struct RenderQuad {
    pub id: String,
    pub position: [f32; 3],
    pub size: [f32; 2],
    pub rotation_z_degrees: f32,
    pub marker: String,
    pub color: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderTransform {
    pub position: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for RenderTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Clone, Debug)]
pub enum RenderOverlay {
    GeoJsonMap(RenderGeoJsonMapOverlay),
    Text(RenderTextOverlay),
}

#[derive(Clone, Debug)]
pub struct RenderGeoJsonMapOverlay {
    pub asset: String,
    pub visible: bool,
    pub radius_scale: f32,
}

#[derive(Clone, Debug)]
pub struct RenderTextOverlay {
    pub x: usize,
    pub y: usize,
    pub text: String,
}
