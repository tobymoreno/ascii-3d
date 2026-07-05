use std::io;

use crate::{
    axis_metadata::CartesianAxesMetadata,
    canvas::Canvas,
    geometry2d::Point2,
    math::{Mat4, Vec3},
    mesh::Mesh,
    mesh_renderer::{MeshTransform, draw_wireframe_matrix},
    projection::ObliqueProjector,
    scene_config::{FrustumConfig, Quad4SceneConfig},
};

use super::render_asset_axes;

#[derive(Debug, Clone, Copy)]
struct PlaneDimensions {
    distance: f32,
    width: f32,
    height: f32,
}

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn camera_pitch_radians(animation_angle_degrees: f32, config: &Quad4SceneConfig) -> f32 {
    let phase = animation_angle_degrees.to_radians();

    (phase.sin() * config.camera.pitch_amplitude_degrees).to_radians()
}

fn camera_world_matrix(pitch_radians: f32, config: &Quad4SceneConfig) -> Mat4 {
    Mat4::translation_vec3(vec3(config.camera.position))
        * Mat4::rotation_x(pitch_radians)
        * Mat4::uniform_scale(config.display.world_scale)
}

fn plane_dimensions(distance: f32, frustum: FrustumConfig) -> PlaneDimensions {
    let half_vertical_fov = (frustum.vertical_fov_degrees * 0.5).to_radians();
    let half_height = distance * half_vertical_fov.tan();
    let height = half_height * 2.0;
    let width = height * frustum.aspect_ratio;

    PlaneDimensions {
        distance,
        width,
        height,
    }
}

fn plane_local_matrix(dimensions: PlaneDimensions) -> Mat4 {
    Mat4::translation(0.0, 0.0, -dimensions.distance)
        * Mat4::scale(dimensions.width, dimensions.height, 1.0)
}

fn draw_plane(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    quad_mesh: &Mesh,
    plane_world: Mat4,
    center_marker: char,
) -> io::Result<Vec3> {
    draw_wireframe_matrix(canvas, projector, quad_mesh, plane_world).map_err(io::Error::other)?;

    let center = plane_world.transform_point(Vec3::zero());
    let center_projected = projector.project(center);

    canvas.set(center_projected, center_marker);

    Ok(center)
}

pub fn render(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    quad_mesh: &Mesh,
    axes_mesh: &Mesh,
    axes_metadata: &CartesianAxesMetadata,
    config: &Quad4SceneConfig,
    animation_angle_degrees: f32,
) -> io::Result<()> {
    if quad_mesh.vertices.len() != 4 {
        return Err(io::Error::other(format!(
            "quad4 scene expected 4 vertices, but loaded {}",
            quad_mesh.vertices.len(),
        )));
    }

    if quad_mesh.faces.len() != 1 {
        return Err(io::Error::other(format!(
            "quad4 scene expected 1 face, but loaded {}",
            quad_mesh.faces.len(),
        )));
    }

    let pitch_radians = camera_pitch_radians(animation_angle_degrees, config);

    let camera_transform = MeshTransform {
        rotation_x: pitch_radians,
        scale: config.display.world_scale,
        translation: vec3(config.camera.position),
        ..MeshTransform::default()
    };

    // The axes visualize the camera's local orientation.
    render_asset_axes(
        canvas,
        projector,
        axes_mesh,
        axes_metadata,
        camera_transform,
    )?;

    let near = plane_dimensions(config.frustum.near_distance, config.frustum);
    let far = plane_dimensions(config.frustum.far_distance, config.frustum);

    // The planes are children of the camera:
    //
    // world = camera_world * plane_local
    //
    // Their size is derived from vertical FOV, aspect ratio, and distance.
    let camera_world = camera_world_matrix(pitch_radians, config);
    let near_world = camera_world * plane_local_matrix(near);
    let far_world = camera_world * plane_local_matrix(far);

    // Draw far first so the near plane remains visually readable.
    let far_center = draw_plane(canvas, projector, quad_mesh, far_world, 'F')?;
    let near_center = draw_plane(canvas, projector, quad_mesh, near_world, 'N')?;

    canvas.draw_text(
        Point2::new(2, 1),
        "Scene: FOV-derived camera frustum planes",
    );

    canvas.draw_text(
        Point2::new(2, 2),
        &format!(
            "FOV {:.1} deg vertical | aspect {:.2} | pitch {:+5.1} deg",
            config.frustum.vertical_fov_degrees,
            config.frustum.aspect_ratio,
            pitch_radians.to_degrees(),
        ),
    );

    canvas.draw_text(
        Point2::new(2, 3),
        &format!(
            "Near N: z -{:.2}, {:.2}w x {:.2}h, center ({:+.2}, {:+.2}, {:+.2})",
            near.distance, near.width, near.height, near_center.x, near_center.y, near_center.z,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 4),
        &format!(
            "Far  F: z -{:.2}, {:.2}w x {:.2}h, center ({:+.2}, {:+.2}, {:+.2})",
            far.distance, far.width, far.height, far_center.x, far_center.y, far_center.z,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 23),
        "Plane size = 2 * distance * tan(vertical_fov / 2)",
    );

    canvas.draw_text(
        Point2::new(2, 24),
        "Width = height * aspect_ratio; not fixed manual scale",
    );

    canvas.draw_text(
        Point2::new(2, 25),
        "Config: assets/quad4.scene.json | Mesh: assets/models/quad4.obj",
    );

    canvas.draw_text(
        Point2::new(2, 26),
        "Both planes pitch and move with the camera's local -Z direction.",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{camera_pitch_radians, camera_world_matrix, plane_dimensions, plane_local_matrix};
    use crate::{
        math::Vec3,
        scene_config::{CameraConfig, DisplayConfig, FrustumConfig, Quad4SceneConfig},
    };

    const EPSILON: f32 = 0.000_01;

    fn config() -> Quad4SceneConfig {
        Quad4SceneConfig {
            name: "test".to_string(),
            mesh_asset: "models/quad4.obj".to_string(),
            camera: CameraConfig {
                position: [0.0, 0.0, 0.0],
                pitch_amplitude_degrees: 45.0,
            },
            frustum: FrustumConfig {
                vertical_fov_degrees: 60.0,
                aspect_ratio: 1.0,
                near_distance: 0.25,
                far_distance: 0.75,
            },
            display: DisplayConfig { world_scale: 1.0 },
        }
    }

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!((actual.x - expected.x).abs() <= EPSILON);
        assert!((actual.y - expected.y).abs() <= EPSILON);
        assert!((actual.z - expected.z).abs() <= EPSILON);
    }

    #[test]
    fn fov_computes_near_plane_size_from_distance() {
        let dimensions = plane_dimensions(0.25, config().frustum);

        assert!((dimensions.height - 0.288_675).abs() <= EPSILON);
        assert!((dimensions.width - dimensions.height).abs() <= EPSILON);
    }

    #[test]
    fn fov_computes_far_plane_larger_than_near_plane() {
        let config = config();

        let near = plane_dimensions(config.frustum.near_distance, config.frustum);
        let far = plane_dimensions(config.frustum.far_distance, config.frustum);

        assert!(far.width > near.width);
        assert!(far.height > near.height);
    }

    #[test]
    fn aspect_ratio_affects_width_but_not_height() {
        let mut config = config();
        config.frustum.aspect_ratio = 1.6;

        let dimensions = plane_dimensions(0.25, config.frustum);

        assert!((dimensions.width - dimensions.height * 1.6).abs() <= EPSILON);
    }

    #[test]
    fn zero_pitch_places_near_plane_on_negative_z() {
        let config = config();
        let near = plane_dimensions(config.frustum.near_distance, config.frustum);

        let world = camera_world_matrix(0.0, &config) * plane_local_matrix(near);

        assert_vec3_close(
            world.transform_point(Vec3::zero()),
            Vec3::new(0.0, 0.0, -0.25),
        );
    }

    #[test]
    fn zero_pitch_places_far_plane_farther_on_negative_z() {
        let config = config();
        let far = plane_dimensions(config.frustum.far_distance, config.frustum);

        let world = camera_world_matrix(0.0, &config) * plane_local_matrix(far);

        assert_vec3_close(
            world.transform_point(Vec3::zero()),
            Vec3::new(0.0, 0.0, -0.75),
        );
    }

    #[test]
    fn positive_pitch_moves_both_planes_up_with_camera() {
        let config = config();
        let pitch = 45.0_f32.to_radians();

        let near = plane_dimensions(config.frustum.near_distance, config.frustum);
        let far = plane_dimensions(config.frustum.far_distance, config.frustum);

        let near_world = camera_world_matrix(pitch, &config) * plane_local_matrix(near);
        let far_world = camera_world_matrix(pitch, &config) * plane_local_matrix(far);

        let near_center = near_world.transform_point(Vec3::zero());
        let far_center = far_world.transform_point(Vec3::zero());

        assert!(near_center.y > 0.0);
        assert!(far_center.y > near_center.y);
        assert!(far_center.z < near_center.z);
    }

    #[test]
    fn animation_reaches_configured_pitch_amplitude() {
        let config = config();

        let pitch = camera_pitch_radians(90.0, &config);

        assert!((pitch.to_degrees() - 45.0).abs() <= EPSILON);
    }
}
