#!/usr/bin/env python3
from pathlib import Path

NEW_BIN = r'''use std::time::{Duration, Instant};

use raylib::prelude::*;

const WIDTH: i32 = 960;
const HEIGHT: i32 = 540;
const DEMO_SECONDS: u64 = 20;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("ascii-3d raylib basic visible proof")
        .build();

    rl.set_target_fps(60);

    let start = Instant::now();

    while !rl.window_should_close() && start.elapsed() < Duration::from_secs(DEMO_SECONDS) {
        let elapsed = start.elapsed().as_secs_f32();

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::RAYWHITE);

        d.draw_rectangle(40, 40, 260, 120, Color::RED);
        d.draw_rectangle(340, 40, 260, 120, Color::GREEN);
        d.draw_rectangle(640, 40, 260, 120, Color::BLUE);

        d.draw_text("ASCII-3D RAYLIB DRAW TEST", 44, 210, 36, Color::BLACK);
        d.draw_text("If you can read this, raylib drawing works.", 44, 260, 24, Color::DARKGRAY);

        let center_x = WIDTH / 2;
        let center_y = 390;
        let orbit_radius = 110.0;
        let orbit_x = center_x + (elapsed.cos() * orbit_radius) as i32;
        let orbit_y = center_y + (elapsed.sin() * orbit_radius) as i32;

        d.draw_circle(center_x, center_y, 64.0, Color::YELLOW);
        d.draw_circle_lines(center_x, center_y, 88.0, Color::BLACK);
        d.draw_line(center_x, center_y, orbit_x, orbit_y, Color::BLACK);
        d.draw_circle(orbit_x, orbit_y, 28.0, Color::MAGENTA);
    }
}
'''

def main() -> None:
    path = Path("src/bin/raylib_overlay_demo.rs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(NEW_BIN)
    print("Applied raylib basic visible proof patch.")

if __name__ == "__main__":
    main()
