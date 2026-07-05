#!/usr/bin/env python3
from pathlib import Path
import re

RAYLIB_OVERLAY_MODULE = r'''use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

pub fn spawn_raylib_overlay_demo() {
    match spawn_overlay_process() {
        Ok(()) => {}
        Err(error) => eprintln!("failed to launch raylib overlay demo: {error}"),
    }
}

fn spawn_overlay_process() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = overlay_command();

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command.spawn()?;

    Ok(())
}

fn overlay_command() -> Command {
    if let Some(binary_path) = sibling_overlay_binary() {
        if binary_path.exists() {
            return Command::new(binary_path);
        }
    }

    let mut command = Command::new("cargo");
    command.args(["run", "--bin", "raylib_overlay_demo"]);

    command
}

fn sibling_overlay_binary() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    Some(current_exe.with_file_name(binary_name()))
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "raylib_overlay_demo.exe"
    } else {
        "raylib_overlay_demo"
    }
}

#[cfg(test)]
mod tests {
    use super::binary_name;

    #[test]
    fn overlay_binary_name_is_not_empty() {
        assert!(!binary_name().is_empty());
    }
}
'''

FINAL_RAYLIB_BIN = r'''use std::time::{Duration, Instant};

use raylib::consts::ConfigFlags;
use raylib::prelude::*;

const WIDTH: i32 = 960;
const HEIGHT: i32 = 540;
const DEMO_SECONDS: u64 = 20;

fn main() {
    let flags = (ConfigFlags::FLAG_WINDOW_TRANSPARENT as u32)
        | (ConfigFlags::FLAG_WINDOW_UNDECORATED as u32)
        | (ConfigFlags::FLAG_WINDOW_TOPMOST as u32)
        | (ConfigFlags::FLAG_WINDOW_ALWAYS_RUN as u32)
        | (ConfigFlags::FLAG_WINDOW_MOUSE_PASSTHROUGH as u32);

    unsafe {
        raylib::ffi::SetConfigFlags(flags);
    }

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("ascii-3d raylib transparent primitive overlay")
        .build();

    rl.set_target_fps(60);

    let start = Instant::now();

    while !rl.window_should_close() && start.elapsed() < Duration::from_secs(DEMO_SECONDS) {
        let elapsed = start.elapsed().as_secs_f32();

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLANK);

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
        "ascii-3d raylib overlay placeholder",
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

def ensure_dependency() -> None:
    path = Path("Cargo.toml")
    text = path.read_text()

    lines = [line for line in text.splitlines() if not line.strip().startswith("raylib = ")]
    text = "\n".join(lines) + "\n"

    marker = "[dependencies]\n"
    if marker not in text:
        raise SystemExit("Could not find [dependencies] in Cargo.toml")

    text = text.replace(marker, marker + 'raylib = "6.0.0"\n', 1)
    path.write_text(text)

def ensure_module() -> None:
    graphics_dir = Path("src/graphics")
    graphics_dir.mkdir(parents=True, exist_ok=True)

    Path("src/graphics/raylib_overlay.rs").write_text(RAYLIB_OVERLAY_MODULE)

    mod_path = Path("src/graphics/mod.rs")
    text = mod_path.read_text() if mod_path.exists() else ""

    if "pub mod raylib_overlay;" not in text:
        text = text.rstrip() + "\npub mod raylib_overlay;\n"

    mod_path.write_text(text)

def ensure_bin() -> None:
    bin_path = Path("src/bin/raylib_overlay_demo.rs")
    bin_path.parent.mkdir(parents=True, exist_ok=True)
    bin_path.write_text(FINAL_RAYLIB_BIN)

def patch_command_enum() -> None:
    path = Path("src/input/command.rs")
    text = path.read_text()

    if "ShowOsGraphicsOverlay" in text:
        return

    if "ToggleFrameTiming," not in text:
        raise SystemExit("Could not find ToggleFrameTiming variant in src/input/command.rs")

    text = text.replace(
        "ToggleFrameTiming,",
        "ToggleFrameTiming,\n    ShowOsGraphicsOverlay,",
        1,
    )

    path.write_text(text)

def make_overlay_menu_line_from(line: str) -> str:
    new_line = line.replace("ToggleFrameTiming", "ShowOsGraphicsOverlay")
    new_line = re.sub(r'"[^"]*"', '"Show OS graphics overlay"', new_line, count=1)
    return new_line

def patch_menu_model() -> None:
    path = Path("src/menu/model.rs")
    text = path.read_text()

    if "ShowOsGraphicsOverlay" in text:
        return

    lines = text.splitlines()
    output = []
    inserted = False

    for line in lines:
        output.append(line)

        if "ToggleFrameTiming" in line and not inserted:
            output.append(make_overlay_menu_line_from(line))
            inserted = True

    if not inserted:
        raise SystemExit("Could not find ToggleFrameTiming menu item in src/menu/model.rs")

    path.write_text("\n".join(output) + "\n")

def patch_app_handler() -> None:
    path = Path("src/app.rs")
    text = path.read_text()

    if "ShowOsGraphicsOverlay" in text:
        return

    match = re.search(r"(?P<indent>\s*)(?P<prefix>[A-Za-z0-9_:]+)::ToggleFrameTiming\s*=>", text)
    if not match:
        raise SystemExit("Could not find ToggleFrameTiming match arm in src/app.rs")

    indent = match.group("indent")
    prefix = match.group("prefix")

    new_arm = (
        f"{indent}{prefix}::ShowOsGraphicsOverlay => {{\n"
        f"{indent}    crate::graphics::raylib_overlay::spawn_raylib_overlay_demo();\n"
        f"{indent}}},\n"
    )

    text = text[:match.start()] + new_arm + text[match.start():]
    path.write_text(text)

def main() -> None:
    ensure_dependency()
    ensure_module()
    ensure_bin()
    patch_command_enum()
    patch_menu_model()
    patch_app_handler()

    print("Applied Ratatui menu trigger for raylib overlay placeholder.")

if __name__ == "__main__":
    main()
