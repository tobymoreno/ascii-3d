use std::io;

use crate::{
    canvas::Canvas,
    geometry2d::Point2,
    math::{Mat4, Vec3},
    mesh::Mesh,
    mesh_renderer::draw_wireframe_matrix,
    projection::ObliqueProjector,
    scene_config::MultiQuadSceneConfig,
};

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn marker_char(marker: &str) -> char {
    marker.chars().next().unwrap_or('*')
}

fn quad_world_matrix(
    root_rotation_y_degrees: f32,
    world_scale: f32,
    position: [f32; 3],
    size: [f32; 2],
    rotation_z_degrees: f32,
) -> Mat4 {
    Mat4::rotation_y(root_rotation_y_degrees.to_radians())
        * Mat4::uniform_scale(world_scale)
        * Mat4::translation_vec3(vec3(position))
        * Mat4::rotation_z(rotation_z_degrees.to_radians())
        * Mat4::scale(size[0], size[1], 1.0)
}

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    quad_mesh: &Mesh,
    config: &MultiQuadSceneConfig,
    animation_angle_degrees: f32,
) -> io::Result<()> {
    if quad_mesh.vertices.len() != 4 {
        return Err(io::Error::other(format!(
            "logo quad scene expected 4 vertices, but loaded {}",
            quad_mesh.vertices.len(),
        )));
    }

    if quad_mesh.faces.len() != 1 {
        return Err(io::Error::other(format!(
            "logo quad scene expected 1 face, but loaded {}",
            quad_mesh.faces.len(),
        )));
    }

    let root_rotation_y_degrees =
        animation_angle_degrees * config.display.rotation_y_degrees_per_turn;

    for quad in &config.quads {
        let world = quad_world_matrix(
            root_rotation_y_degrees,
            config.display.world_scale,
            quad.position,
            quad.size,
            quad.rotation_z_degrees,
        );

        draw_wireframe_matrix(canvas, projector, quad_mesh, world).map_err(io::Error::other)?;

        let center = world.transform_point(Vec3::zero());
        canvas.set(projector.project(center), marker_char(&quad.marker));
    }

    canvas.draw_text(
        Point2::new(2, 1),
        "Scene: KM logo from reusable quad4 planes",
    );
    canvas.draw_text(
        Point2::new(2, 2),
        &format!(
            "{} quad4 planes | root Y rotation {:+.1} deg",
            config.quads.len(),
            root_rotation_y_degrees,
        ),
    );
    canvas.draw_text(
        Point2::new(2, 3),
        "Each bar is one transformed quad4 plane; the whole logo rotates as a group.",
    );
    canvas.draw_text(
        Point2::new(2, 25),
        "Config: assets/scenes/km_logo_quads.scene.json | Mesh: assets/models/quad4.obj",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::marker_char;

    #[test]
    fn marker_char_uses_first_character() {
        assert_eq!(marker_char("KM"), 'K');
        assert_eq!(marker_char(""), '*');
    }
}
