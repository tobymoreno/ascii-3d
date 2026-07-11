#[derive(Clone, Copy, Debug)]
pub struct ViewerState {
    pub rotation_x_degrees: f32,
    pub rotation_y_degrees: f32,
    pub rotation_z_degrees: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub origin_z: f32,
    pub zoom: f32,
    pub show_axes: bool,
    pub fps: f32,
    pub frame_time_ms: f32,
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
