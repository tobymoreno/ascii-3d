use super::draw_axes;
use crate::{
    canvas::Canvas,
    geometry2d::Point2,
    math::Vec3,
    mesh::Mesh,
    mesh_renderer::{MeshTransform, draw_wireframe},
    projection::ObliqueProjector,
};
use std::io;

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    mesh: &Mesh,
    angle_degrees: f32,
) -> io::Result<()> {
    draw_axes(canvas, projector, true);
    let angle_radians = angle_degrees.to_radians();
    let transform = MeshTransform {
        rotation_x: angle_radians * 0.7,
        rotation_y: angle_radians,
        rotation_z: angle_radians * 0.35,
        scale: 2.5,
        translation: Vec3::zero(),
    };
    draw_wireframe(canvas, projector, mesh, transform).map_err(io::Error::other)?;
    canvas.draw_text(
        Point2::new(2, 1),
        &format!(
            "Scene: rotating OBJ wireframe box  angle={:06.1}",
            angle_degrees
        ),
    );
    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "vertices={}  faces={}  unique edges={}",
            mesh.vertices.len(),
            mesh.faces.len(),
            mesh.unique_edges().len()
        ),
    );
    canvas.draw_text(Point2::new(2, 25), "Source: assets/box.obj");
    canvas.draw_text(
        Point2::new(2, 26),
        "Centered at origin; largest dimension normalized to 1.0",
    );
    Ok(())
}
