#!/usr/bin/env python3
from pathlib import Path

def patch_cargo() -> None:
    path = Path("Cargo.toml")
    text = path.read_text()

    if 'minifb = ' not in text:
        text = text.rstrip() + '\nminifb = "0.28"\n'

    path.write_text(text)

def patch_main() -> None:
    path = Path("src/main.rs")
    text = path.read_text()

    if "mod graphics;" not in text:
        text = text.replace("mod glyphs;\n", "mod glyphs;\nmod graphics;\n", 1)

    path.write_text(text)

def write_graphics_module() -> None:
    root = Path("src/graphics")
    root.mkdir(parents=True, exist_ok=True)

    (root / "mod.rs").write_text('''pub mod primitives;
pub mod window;

pub use window::run_primitives_demo;
''')

    (root / "primitives.rs").write_text(r'''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const BLACK: Self = Self(0, 0, 0);
    pub const GREEN: Self = Self(80, 255, 120);
    pub const WHITE: Self = Self(230, 230, 230);
    pub const BLUE: Self = Self(90, 160, 255);
    pub const RED: Self = Self(255, 90, 90);
    pub const YELLOW: Self = Self(255, 220, 80);

    pub fn to_u32(self) -> u32 {
        let Self(red, green, blue) = self;

        ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
    }
}

#[derive(Debug)]
pub struct PixelSurface {
    width: usize,
    height: usize,
    buffer: Vec<u32>,
}

impl PixelSurface {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![Rgb::BLACK.to_u32(); width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn buffer(&self) -> &[u32] {
        &self.buffer
    }

    pub fn clear(&mut self, color: Rgb) {
        self.buffer.fill(color.to_u32());
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgb) {
        if x < 0 || y < 0 {
            return;
        }

        let x = x as usize;
        let y = y as usize;

        if x >= self.width || y >= self.height {
            return;
        }

        self.buffer[(y * self.width) + x] = color.to_u32();
    }

    pub fn draw_line(&mut self, start: (i32, i32), end: (i32, i32), color: Rgb) {
        let (mut x0, mut y0) = start;
        let (x1, y1) = end;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.set_pixel(x0, y0, color);

            if x0 == x1 && y0 == y1 {
                break;
            }

            let twice_error = error * 2;

            if twice_error >= dy {
                error += dy;
                x0 += sx;
            }

            if twice_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgb) {
        if width <= 0 || height <= 0 {
            return;
        }

        let right = x + width - 1;
        let bottom = y + height - 1;

        self.draw_line((x, y), (right, y), color);
        self.draw_line((right, y), (right, bottom), color);
        self.draw_line((right, bottom), (x, bottom), color);
        self.draw_line((x, bottom), (x, y), color);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgb) {
        if width <= 0 || height <= 0 {
            return;
        }

        for yy in y..(y + height) {
            for xx in x..(x + width) {
                self.set_pixel(xx, yy, color);
            }
        }
    }

    pub fn draw_circle(&mut self, center_x: i32, center_y: i32, radius: i32, color: Rgb) {
        if radius <= 0 {
            return;
        }

        let mut x = radius;
        let mut y = 0;
        let mut error = 0;

        while x >= y {
            self.set_pixel(center_x + x, center_y + y, color);
            self.set_pixel(center_x + y, center_y + x, color);
            self.set_pixel(center_x - y, center_y + x, color);
            self.set_pixel(center_x - x, center_y + y, color);
            self.set_pixel(center_x - x, center_y - y, color);
            self.set_pixel(center_x - y, center_y - x, color);
            self.set_pixel(center_x + y, center_y - x, color);
            self.set_pixel(center_x + x, center_y - y, color);

            y += 1;

            if error <= 0 {
                error += (2 * y) + 1;
            }

            if error > 0 {
                x -= 1;
                error -= (2 * x) + 1;
            }
        }
    }

    pub fn draw_polyline(&mut self, points: &[(i32, i32)], color: Rgb) {
        for pair in points.windows(2) {
            self.draw_line(pair[0], pair[1], color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PixelSurface, Rgb};

    #[test]
    fn set_pixel_writes_expected_buffer_cell() {
        let mut surface = PixelSurface::new(4, 3);

        surface.set_pixel(2, 1, Rgb::GREEN);

        assert_eq!(surface.buffer()[6], Rgb::GREEN.to_u32());
    }

    #[test]
    fn set_pixel_clips_out_of_bounds_points() {
        let mut surface = PixelSurface::new(4, 3);

        surface.set_pixel(-1, 0, Rgb::GREEN);
        surface.set_pixel(4, 0, Rgb::GREEN);
        surface.set_pixel(0, 3, Rgb::GREEN);

        assert!(surface.buffer().iter().all(|pixel| *pixel == Rgb::BLACK.to_u32()));
    }
}
''')

    (root / "window.rs").write_text(r'''use std::error::Error;
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
''')

def write_demo_bin() -> None:
    bin_dir = Path("src/bin")
    bin_dir.mkdir(parents=True, exist_ok=True)

    (bin_dir / "os_graphics_demo.rs").write_text(r'''#[path = "../graphics/mod.rs"]
mod graphics;

fn main() {
    if let Err(error) = graphics::run_primitives_demo() {
        eprintln!("os graphics demo error: {error}");
        std::process::exit(1);
    }
}
''')

def main() -> None:
    patch_cargo()
    patch_main()
    write_graphics_module()
    write_demo_bin()
    print("Applied OS graphics primitives demo patch.")

if __name__ == "__main__":
    main()
