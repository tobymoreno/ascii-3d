use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    axis_metadata::{CartesianAxesMetadata, load_cartesian_axes_metadata},
    camera3d::{Camera3D, CameraProjection},
    canvas::Canvas,
    geometry2d::Point2,
    math::Vec3,
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

const WORLD_AXES_SCALE: f32 = 2.0;
const CAMERA_GIZMO_SCREEN_LEG: f32 = 3.0;

// Screen-only framing for the debug/world view.
// This does not change 3D world coordinates. It only moves the projected
// universe on the terminal so +X has more visible room.
const WORLD_DEBUG_SCREEN_OFFSET_X: i32 = -18;
const WORLD_DEBUG_SCREEN_OFFSET_Y: i32 = 0;

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
        Point2::new(
            projection.screen_origin[0] + WORLD_DEBUG_SCREEN_OFFSET_X,
            projection.screen_origin[1] + WORLD_DEBUG_SCREEN_OFFSET_Y,
        ),
        projection.axis_vectors.x,
        projection.axis_vectors.y,
        projection.axis_vectors.z,
    ))
}

fn camera_direction_from_yaw_pitch(yaw_degrees: f32, pitch_degrees: f32) -> Vec3 {
    let yaw = yaw_degrees.to_radians();
    let pitch = pitch_degrees.to_radians();
    let horizontal = pitch.cos();

    vec3_normalize(Vec3::new(
        yaw.sin() * horizontal,
        pitch.sin(),
        yaw.cos() * horizontal,
    ))
}

fn camera_for_debug(position: Vec3, yaw_degrees: f32, pitch_degrees: f32) -> Camera3D {
    let direction = camera_direction_from_yaw_pitch(yaw_degrees, pitch_degrees);

    Camera3D::new(
        position,
        Vec3::new(
            position.x + direction.x,
            position.y + direction.y,
            position.z + direction.z,
        ),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        CameraProjection::Perspective,
        0.1,
        100.0,
    )
}

fn vec3_subtract(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn vec3_cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn vec3_length(value: Vec3) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

fn vec3_normalize(value: Vec3) -> Vec3 {
    let length = vec3_length(value);

    if length <= f32::EPSILON {
        Vec3::zero()
    } else {
        Vec3::new(value.x / length, value.y / length, value.z / length)
    }
}

fn fixed_screen_direction(
    projector: &ObliqueProjector,
    origin_world: Vec3,
    direction_world: Vec3,
) -> Point2 {
    let origin_2d = projector.project(origin_world);
    let tip_2d = projector.project(Vec3::new(
        origin_world.x + direction_world.x,
        origin_world.y + direction_world.y,
        origin_world.z + direction_world.z,
    ));

    let dx = (tip_2d.x - origin_2d.x) as f32;
    let dy = (tip_2d.y - origin_2d.y) as f32;
    let length = (dx * dx + dy * dy).sqrt();

    if length <= f32::EPSILON {
        origin_2d
    } else {
        Point2::new(
            origin_2d.x + (dx / length * CAMERA_GIZMO_SCREEN_LEG).round() as i32,
            origin_2d.y + (dy / length * CAMERA_GIZMO_SCREEN_LEG).round() as i32,
        )
    }
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
    let origin_world = camera.position;

    // Orientation comes from Camera3D vectors.
    // The leg length below is fixed in terminal cells, not world units.
    let z_direction = vec3_normalize(vec3_subtract(camera.target, camera.position));
    let x_direction = vec3_normalize(vec3_cross(camera.up, z_direction));
    let y_direction = vec3_normalize(vec3_cross(z_direction, x_direction));

    let origin_2d = projector.project(origin_world);
    let x_2d = fixed_screen_direction(projector, origin_world, x_direction);
    let y_2d = fixed_screen_direction(projector, origin_world, y_direction);
    let z_2d = fixed_screen_direction(projector, origin_world, z_direction);

    canvas.draw_line(origin_2d, x_2d, '-');
    canvas.draw_line(origin_2d, y_2d, '|');
    canvas.draw_line(origin_2d, z_2d, '/');

    canvas.set(origin_2d, '*');
    canvas.set(x_2d, 'x');
    canvas.set(y_2d, 'y');
    canvas.set(z_2d, 'z');
}

fn draw_metadata(
    canvas: &mut Canvas,
    world: WorldSpace3D,
    camera: Camera3D,
    yaw_degrees: f32,
    pitch_degrees: f32,
) {
    canvas.draw_text(
        Point2::new(2, 1),
        "Scene: WorldSpace3D axes with small Camera3D xyz gizmo",
    );
    canvas.draw_text(
        Point2::new(2, 2),
        "Big XYZ = world axes shifted left for +X room. Small */x/y/z = fixed-size camera gizmo.",
    );
    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "world axis_length {:.1} | camera pos [{:.2}, {:.2}, {:.2}] | yaw {:.1} pitch {:.1}",
            world.axis_length,
            camera.position.x,
            camera.position.y,
            camera.position.z,
            yaw_degrees,
            pitch_degrees
        ),
    );
    canvas.draw_text(
        Point2::new(2, 26),
        "Camera gizmo uses an orthogonal x/y/z basis; legs stay fixed screen length.",
    );
}

pub fn render(
    canvas: &mut Canvas,
    camera_position: Vec3,
    camera_yaw_degrees: f32,
    camera_pitch_degrees: f32,
) -> io::Result<()> {
    let world = WorldSpace3D::default_world();
    let camera = camera_for_debug(camera_position, camera_yaw_degrees, camera_pitch_degrees);

    let projector = load_projector()?;
    let axes_mesh = load_mesh(AXES_ASSET)?;
    let axes_metadata = load_axes_metadata(AXES_METADATA_ASSET)?;

    draw_metadata(
        canvas,
        world,
        camera,
        camera_yaw_degrees,
        camera_pitch_degrees,
    );
    draw_world_axes(canvas, &projector, &axes_mesh, &axes_metadata)?;
    draw_camera_gizmo(canvas, &projector, camera);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CAMERA_GIZMO_SCREEN_LEG, camera_for_debug, vec3_length, vec3_normalize, vec3_subtract,
    };
    use crate::math::Vec3;

    #[test]
    fn camera_debug_target_defines_a_direction() {
        let camera = camera_for_debug(Vec3::new(0.65, 0.55, 0.35), 0.0, 0.0);
        let direction = vec3_subtract(camera.target, camera.position);

        assert!(vec3_length(direction) > 0.0);
    }

    #[test]
    fn camera_gizmo_screen_leg_is_fixed_visual_length() {
        assert_eq!(CAMERA_GIZMO_SCREEN_LEG, 3.0);
    }

    #[test]
    fn camera_direction_is_normalized_for_gizmo_orientation() {
        let camera = camera_for_debug(Vec3::new(0.65, 0.55, 0.35), 0.0, 0.0);
        let direction = vec3_normalize(vec3_subtract(camera.target, camera.position));

        assert!((vec3_length(direction) - 1.0).abs() < 0.000_01);
    }

    fn vec3_dot(a: Vec3, b: Vec3) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

    #[test]
    fn yaw_rotates_camera_direction() {
        let camera = camera_for_debug(Vec3::new(0.65, 0.55, 0.35), 90.0, 0.0);
        let direction = vec3_normalize(vec3_subtract(camera.target, camera.position));

        assert!(direction.x > 0.99);
        assert!(direction.z.abs() < 0.000_01);
    }

    #[test]
    fn camera_basis_y_is_orthogonal_to_z_and_x() {
        let camera = camera_for_debug(Vec3::new(0.65, 0.55, 0.35), 25.0, 30.0);

        let z = vec3_normalize(vec3_subtract(camera.target, camera.position));
        let x = vec3_normalize(vec3_cross(camera.up, z));
        let y = vec3_normalize(vec3_cross(z, x));

        assert!((vec3_length(x) - 1.0).abs() < 0.000_01);
        assert!((vec3_length(y) - 1.0).abs() < 0.000_01);
        assert!((vec3_length(z) - 1.0).abs() < 0.000_01);

        assert!(vec3_dot(x, y).abs() < 0.000_01);
        assert!(vec3_dot(y, z).abs() < 0.000_01);
        assert!(vec3_dot(z, x).abs() < 0.000_01);
    }
}
