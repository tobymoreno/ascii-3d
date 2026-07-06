#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod a3d;
mod app;
mod axis_metadata;
mod camera3d;
mod canvas;
mod curves;
mod geometry2d;
mod glyphs;
mod graphics;
mod input;
mod math;
mod menu;
mod mesh;
mod mesh_renderer;
mod obj;
mod projection;
mod projection_config;
mod scene_config;
mod scenes;
mod tui;
mod world_space;
mod xyz_control;

fn main() -> std::io::Result<()> {
    app::run()
}
