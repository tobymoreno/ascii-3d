mod a3d;
mod app;
mod axis_metadata;
mod camera3d;
mod canvas;
mod curves;
mod geometry2d;
mod glyphs;
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

fn main() -> std::io::Result<()> {
    app::run()
}
