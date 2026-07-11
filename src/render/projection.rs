use crossterm::terminal;
use std::sync::OnceLock;

const DEFAULT_CELL_ASPECT_RATIO: f32 = 0.5;

#[derive(Clone, Copy, Debug)]
pub struct Projection {
    width: usize,
    height: usize,
    camera_distance: f32,
    near_clip: f32,
    cell_aspect_ratio: f32,
    vertical_center_ratio: f32,
}

impl Projection {
    pub fn terminal_with_camera(
        width: usize,
        height: usize,
        camera_distance: f32,
        near_clip: f32,
        vertical_center_ratio: f32,
    ) -> Self {
        Self {
            width,
            height,
            camera_distance,
            near_clip,
            cell_aspect_ratio: terminal_cell_aspect_ratio(),
            vertical_center_ratio,
        }
    }

    pub fn terminal_cell_aspect_ratio() -> f32 {
        terminal_cell_aspect_ratio()
    }

    pub fn project_xyz(self, x: f32, y: f32, z: f32) -> Option<(i32, i32, f32)> {
        let depth = self.camera_distance + z;

        if !x.is_finite() || !y.is_finite() || !z.is_finite() || depth <= self.near_clip {
            return None;
        }

        let perspective = self.camera_distance / depth;

        if !perspective.is_finite() {
            return None;
        }

        let aspect_correction = 1.0 / self.cell_aspect_ratio;
        let screen_x = x * perspective * aspect_correction + self.width as f32 * 0.5;
        let screen_y = self.height as f32 * self.vertical_center_ratio - y * perspective;

        if !screen_x.is_finite() || !screen_y.is_finite() {
            return None;
        }

        Some((screen_x.round() as i32, screen_y.round() as i32, depth))
    }
}

fn terminal_cell_aspect_ratio() -> f32 {
    static CELL_ASPECT_RATIO: OnceLock<f32> = OnceLock::new();

    *CELL_ASPECT_RATIO.get_or_init(|| match terminal::window_size() {
        Ok(size) if size.width > 0 && size.height > 0 && size.columns > 0 && size.rows > 0 => {
            let cell_width = size.width as f32 / size.columns as f32;
            let cell_height = size.height as f32 / size.rows as f32;

            (cell_width / cell_height).clamp(0.25, 2.0)
        }
        _ => DEFAULT_CELL_ASPECT_RATIO,
    })
}
