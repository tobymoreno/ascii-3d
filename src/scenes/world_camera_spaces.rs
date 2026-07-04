use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    axis_metadata::{CartesianAxesMetadata, load_cartesian_axes_metadata},
    camera3d::{Camera3D, CameraProjection},
    canvas::Canvas,
    geometry2d::Point2,
    math::{Mat4, Vec3},
    mesh::Mesh,
    mesh_renderer::MeshTransform,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::load_projection_config,
    world_space::WorldSpace3D,
};

use super::render_asset_axes;

const PROJECTION_ASSET: &str = "assets/projections/plan_xy.projection.json";
const AXES_ASSET: &str = "assets/cartesian_axes.obj";
const AXES_METADATA_ASSET: &str = "assets/cartesian_axes.json";

const WORLD_AXES_SCALE: f32 = 1.0;
const CAMERA_GIZMO_LEG: f32 = 0.35;

fn asset_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn load_mesh(relative_path: &str) -> io::Result<Mesh> {
    let path = asset_path(relative_path);

    load_obj(&path).map_err(|error| {
        io::Error::other(format!("failed to load OBJ {}: {}", path.display(), error))
    })
}

fn load_axes_metadata(relative_path: &str) -> io::Result<CartesianAxesMetadata> {
    load_cartesian_axes_metadata(asset_path(relative_path))
}

fn load_projector() -> io::Result<ObliqueProjector> {
    let projection = load_projection_config(asset_path(PROJECTION_ASSET))?;

    Ok(ObliqueProjector::from_axis_vectors(
        Point2::new(projection.screen_origin[0], projection.screen_origin[1]),
        projection.axis_vectors.x,
        projection.axis_vectors.y,
        projection.axis_vectors.z,
    ))
}

fn camera_for_debug() -> Camera3D {
    Camera3D::new(
        Vec3::new(0.65, 0.55, 0.35),
        Vec3::new(0.65, 0.55, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        CameraProjection::Perspective,
        0.1,
        100.0,
    )
}

fn camera_world_matrix(camera: Camera3D) -> Mat4 {
    Mat4::translation_vec3(camera.position)
}

fn project_matrix_point(projector: &ObliqueProjector, transform: Mat4, point: Vec3) -> Point2 {
    projector.project(transform.transform_point(point))
}

fn draw_matrix_line(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    transform: Mat4,
    from: Vec3,
    to: Vec3,
    character: char,
) {
    canvas.draw_line(
        project_matrix_point(projector, transform, from),
        project_matrix_point(projector, transform, to),
        character,
    );
}

fn draw_world_axes(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    axes_mesh: &Mesh,
    axes_metadata: &CartesianAxesMetadata,
) -> io::Result<()> {
    let transform = MeshTransform {
        scale: WORLD_AXES_SCALE,
        ..MeshTransform::default()
    };

    render_asset_axes(canvas, projector, axes_mesh, axes_metadata, transform)
}

fn draw_camera_gizmo(canvas: &mut Canvas, projector: &ObliqueProjector, camera: Camera3D) {
    let camera_world = camera_world_matrix(camera);

    let origin = Vec3::zero();

    // Small camera-local axes.
    // x follows +X, y follows +Y, z is one ray along +Z in this world projection
    // so it appears southwest like the world Z direction.
    let x = Vec3::new(CAMERA_GIZMO_LEG, 0.0, 0.0);
    let y = Vec3::new(0.0, CAMERA_GIZMO_LEG, 0.0);
    let z = Vec3::new(0.0, 0.0, CAMERA_GIZMO_LEG);

    draw_matrix_line(canvas, projector, camera_world, origin, x, '-');
    draw_matrix_line(canvas, projector, camera_world, origin, y, '|');
    draw_matrix_line(canvas, projector, camera_world, origin, z, '/');

    let origin_2d = project_matrix_point(projector, camera_world, origin);
    let x_2d = project_matrix_point(projector, camera_world, x);
    let y_2d = project_matrix_point(projector, camera_world, y);
    let z_2d = project_matrix_point(projector, camera_world, z);

    canvas.set(origin_2d, '*');
    canvas.set(x_2d, 'x');
    canvas.set(y_2d, 'y');
    canvas.set(z_2d, 'z');
}

fn draw_metadata(canvas: &mut Canvas, world: WorldSpace3D, camera: Camera3D) {
    canvas.draw_text(
        Point2::new(2, 1),
        "Scene: WorldSpace3D axes with small Camera3D xyz gizmo",
    );
    canvas.draw_text(
        Point2::new(2, 2),
        "Big XYZ = world universe axes. Small */x/y/z = movable camera gizmo.",
    );
    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "world axis_length {:.1} | camera pos [{:.2}, {:.2}, {:.2}]",
            world.axis_length, camera.position.x, camera.position.y, camera.position.z
        ),
    );
    canvas.draw_text(
        Point2::new(2, 26),
        "Camera gizmo is visual/orientation helper only; world axes remain the main 3D universe.",
    );
}

pub fn render(canvas: &mut Canvas) -> io::Result<()> {
    let world = WorldSpace3D::default_world();
    let camera = camera_for_debug();

    let projector = load_projector()?;
    let axes_mesh = load_mesh(AXES_ASSET)?;
    let axes_metadata = load_axes_metadata(AXES_METADATA_ASSET)?;

    draw_metadata(canvas, world, camera);
    draw_world_axes(canvas, &projector, &axes_mesh, &axes_metadata)?;
    draw_camera_gizmo(canvas, &projector, camera);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CAMERA_GIZMO_LEG, camera_for_debug, camera_world_matrix};
    use crate::math::Vec3;

    #[test]
    fn camera_gizmo_origin_maps_to_camera_position() {
        let camera = camera_for_debug();
        let world = camera_world_matrix(camera);

        let origin = world.transform_point(Vec3::zero());

        assert_eq!(origin.x, camera.position.x);
        assert_eq!(origin.y, camera.position.y);
        assert_eq!(origin.z, camera.position.z);
    }

    #[test]
    fn camera_gizmo_legs_are_small() {
        assert!(CAMERA_GIZMO_LEG > 0.0);
        assert!(CAMERA_GIZMO_LEG < 1.0);
    }
}
