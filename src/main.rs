mod app;
mod axis_metadata;
mod canvas;
mod curves;
mod geometry2d;
mod math;
mod mesh;
mod mesh_renderer;
mod obj;
mod projection;
mod projection_config;
mod scene_config;
mod scenes;

fn main() -> std::io::Result<()> {
    app::run()
}
