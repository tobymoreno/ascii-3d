mod app;
mod axis_metadata;
mod canvas;
mod geometry2d;
mod math;
mod mesh;
mod mesh_renderer;
mod obj;
mod projection;
mod scenes;

fn main() -> std::io::Result<()> {
    app::run()
}
