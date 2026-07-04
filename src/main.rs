mod app;
mod axis_metadata;
mod camera3d;
mod canvas;
mod curves;
mod geometry2d;
mod glyphs;
mod math;
mod mesh;
mod mesh_renderer;
mod obj;
mod projection;
mod projection_config;
mod scene_config;
mod scenes;
mod world_space;

fn main() -> std::io::Result<()> {
    app::run()
}
