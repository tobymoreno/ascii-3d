#!/usr/bin/env python3
from pathlib import Path

NEW_BIN = r'''use std::time::{Duration, Instant};

use raylib::consts::ConfigFlags;
use raylib::prelude::*;

const WIDTH: i32 = 960;
const HEIGHT: i32 = 540;
const DEMO_SECONDS: u64 = 20;

fn main() {
    let flags = (ConfigFlags::FLAG_WINDOW_UNDECORATED as u32)
        | (ConfigFlags::FLAG_WINDOW_TOPMOST as u32)
        | (ConfigFlags::FLAG_WINDOW_ALWAYS_RUN as u32)
        | (ConfigFlags::FLAG_MSAA_4X_HINT as u32);

    unsafe {
        raylib::ffi::SetConfigFlags(flags);
    }

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("ascii-3d raylib opaque primitive proof")
        .build();

    rl.set_target_fps(60);

    unsafe {
        raylib::ffi::SetWindowPosition(80, 80);
    }

    let start = Instant::now();

    while !rl.window_should_close() && start.elapsed() < Duration::from_secs(DEMO_SECONDS) {
        let elapsed = start.elapsed().as_secs_f32();

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);

        draw_demo_frame(&mut d, elapsed);
    }
}

fn draw_demo_frame(d: &mut RaylibDrawHandle<'_>, elapsed: f32) {
    let center_x = WIDTH / 2;
    let center_y = HEIGHT / 2;

    d.draw_rectangle_lines(24, 24, WIDTH - 48, HEIGHT - 48, Color::GREEN);

    d.draw_line(40, center_y, WIDTH - 40, center_y, Color::BLUE);
    d.draw_line(center_x, 40, center_x, HEIGHT - 40, Color::BLUE);

    d.draw_circle_lines(center_x, center_y, 92.0, Color::WHITE);

    let orbit_radius = 150.0;
    let orbit_x = center_x + (elapsed.cos() * orbit_radius) as i32;
    let orbit_y = center_y + (elapsed.sin() * orbit_radius) as i32;

    d.draw_circle_lines(orbit_x, orbit_y, 28.0, Color::YELLOW);
    d.draw_line(center_x, center_y, orbit_x, orbit_y, Color::YELLOW);

    let rotating_box = rotated_box_points(center_x, center_y, 190.0, 96.0, elapsed * 1.4);
    draw_polyline(d, &rotating_box, Color::RED);

    d.draw_rectangle(42, 42, 12, 12, Color::GREEN);
    d.draw_rectangle_lines(64, 42, 120, 38, Color::WHITE);
    d.draw_line(72, 61, 176, 61, Color::GREEN);

    d.draw_text(
        "raylib opaque primitive proof",
        42,
        HEIGHT - 44,
        20,
        Color::GREEN,
    );
}

fn draw_polyline(d: &mut RaylibDrawHandle<'_>, points: &[(i32, i32)], color: Color) {
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];

        d.draw_line(x0, y0, x1, y1, color);
    }
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
'''

def main() -> None:
    path = Path("src/bin/raylib_overlay_demo.rs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(NEW_BIN)
    print("Applied raylib opaque proof patch.")

if __name__ == "__main__":
    main()
