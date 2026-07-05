use std::error::Error;
use std::time::Instant;

use minifb::{Key, Window, WindowOptions};

use super::primitives::{PixelSurface, Rgb};

const WIDTH: usize = 960;
const HEIGHT: usize = 540;

pub fn run_primitives_demo() -> Result<(), Box<dyn Error>> {
    let mut window = Window::new(
        "ascii-3d OS graphics primitives | Esc to close",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;

    let mut surface = PixelSurface::new(WIDTH, HEIGHT);
    let start = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let elapsed = start.elapsed().as_secs_f32();

        draw_demo_frame(&mut surface, elapsed);

        window.update_with_buffer(surface.buffer(), surface.width(), surface.height())?;
    }

    Ok(())
}

fn draw_demo_frame(surface: &mut PixelSurface, elapsed: f32) {
    surface.clear(Rgb::BLACK);

    let width = surface.width() as i32;
    let height = surface.height() as i32;
    let center_x = width / 2;
    let center_y = height / 2;

    surface.draw_rect(24, 24, width - 48, height - 48, Rgb::GREEN);

    surface.draw_line((40, center_y), (width - 40, center_y), Rgb::BLUE);
    surface.draw_line((center_x, 40), (center_x, height - 40), Rgb::BLUE);

    let orbit_radius = 150.0;
    let orbit_x = center_x + (elapsed.cos() * orbit_radius) as i32;
    let orbit_y = center_y + (elapsed.sin() * orbit_radius) as i32;

    surface.draw_circle(center_x, center_y, 92, Rgb::WHITE);
    surface.draw_circle(orbit_x, orbit_y, 28, Rgb::YELLOW);
    surface.draw_line((center_x, center_y), (orbit_x, orbit_y), Rgb::YELLOW);

    let rotating_box = rotated_box_points(center_x, center_y, 190.0, 96.0, elapsed * 1.4);
    surface.draw_polyline(&rotating_box, Rgb::RED);

    surface.fill_rect(42, 42, 12, 12, Rgb::GREEN);
    surface.draw_rect(64, 42, 120, 38, Rgb::WHITE);
    surface.draw_line((72, 61), (176, 61), Rgb::GREEN);
}

fn rotated_box_points(
    center_x: i32,
    center_y: i32,
    width: f32,
    height: f32,
    angle: f32,
) -> Vec<(i32, i32)> {
    let half_width = width / 2.0;
    let half_height = height / 2.0;

    let corners = [
        (-half_width, -half_height),
        (half_width, -half_height),
        (half_width, half_height),
        (-half_width, half_height),
        (-half_width, -half_height),
    ];

    let cos_angle = angle.cos();
    let sin_angle = angle.sin();

    corners
        .iter()
        .map(|(x, y)| {
            let rotated_x = (x * cos_angle) - (y * sin_angle);
            let rotated_y = (x * sin_angle) + (y * cos_angle);

            (
                center_x + rotated_x.round() as i32,
                center_y + rotated_y.round() as i32,
            )
        })
        .collect()
}
