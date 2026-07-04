use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    axis_metadata::{CartesianAxesMetadata, load_cartesian_axes_metadata},
    camera3d::{Camera3D, CameraProjection},
    canvas::Canvas,
    geometry2d::Point2,
    glyphs::{WordAsset, WordMetadata, read_json, render_word_with_stroke_character},
    math::{Mat4, Vec3},
    mesh::Mesh,
    mesh_renderer::MeshTransform,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::load_projection_config,
    world_space::WorldSpace3D,
};

const PROJECTION_ASSET: &str = "assets/projections/plan_xy.projection.json";
const AXES_ASSET: &str = "assets/cartesian_axes.obj";
const AXES_METADATA_ASSET: &str = "assets/cartesian_axes.json";
const SINGLE_P_WORD_ASSET: &str = "assets/words/single_p.word.json";
const SINGLE_P_WORD_METADATA_ASSET: &str = "assets/words/single_p.metadata.json";

const WORLD_AXES_SCALE: f32 = 2.8;
const CAMERA_GIZMO_SCREEN_LEG: f32 = 3.0;

// Screen-only framing for the debug/world view.
// This does not change 3D world coordinates. It only moves the projected
// universe on the terminal so +X has more visible room.
const WORLD_DEBUG_SCREEN_OFFSET_X: i32 = -18;
const WORLD_DEBUG_SCREEN_OFFSET_Y: i32 = 6;

// World placement for the actual P object.
const P_WORD_WORLD_X: f32 = 0.35;
const P_WORD_WORLD_Y: f32 = 0.10;
const P_WORD_WORLD_Z: f32 = -1.80;

const P2_WORD_WORLD_X: f32 = 0.55;
const P2_WORD_WORLD_Y: f32 = 0.10;
const P2_WORD_WORLD_Z: f32 = -3.20;

const P_WORD_WORLD_SCALE: f32 = 1.35;

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

fn load_single_p_word_assets() -> io::Result<(WordAsset, WordMetadata)> {
    let word: WordAsset = read_json(SINGLE_P_WORD_ASSET)?;
    let metadata: WordMetadata = read_json(SINGLE_P_WORD_METADATA_ASSET)?;

    Ok((word, metadata))
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

fn vec3_from_array(values: [f32; 3]) -> Vec3 {
    Vec3::new(values[0], values[1], values[2])
}

fn draw_axis_line(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    transform: MeshTransform,
    from: Vec3,
    to: Vec3,
    character: char,
) -> Point2 {
    let from_screen = projector.project(transform.transform_vertex(from));
    let to_screen = projector.project(transform.transform_vertex(to));

    canvas.draw_line(from_screen, to_screen, character);

    to_screen
}

fn world_axis_label_position(axis_id: &str, endpoint_screen: Point2, negative: bool) -> Point2 {
    match (axis_id, negative) {
        ("x", false) => Point2::new(endpoint_screen.x + 1, endpoint_screen.y),
        ("y", false) => Point2::new(endpoint_screen.x - 1, endpoint_screen.y - 1),
        ("z", false) => Point2::new(endpoint_screen.x - 2, endpoint_screen.y),
        ("z", true) => Point2::new(endpoint_screen.x + 1, endpoint_screen.y),
        _ => Point2::new(endpoint_screen.x + 1, endpoint_screen.y),
    }
}

fn draw_world_axes(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    _axes_mesh: &Mesh,
    axes_metadata: &CartesianAxesMetadata,
) -> io::Result<()> {
    let transform = MeshTransform {
        scale: WORLD_AXES_SCALE,
        ..MeshTransform::default()
    };

    let origin = vec3_from_array(axes_metadata.origin.position);

    for axis in &axes_metadata.axes {
        let positive_endpoint = vec3_from_array(axis.positive_endpoint);

        let character = match axis.id.as_str() {
            "x" => '-',
            "y" => '|',
            "z" => '/',
            _ => '.',
        };

        let positive_endpoint_screen = draw_axis_line(
            canvas,
            projector,
            transform,
            origin,
            positive_endpoint,
            character,
        );

        if axes_metadata.display.show_positive_labels {
            let label_screen =
                world_axis_label_position(axis.id.as_str(), positive_endpoint_screen, false);
            canvas.draw_text(label_screen, &axis.positive_label);
        }

        if axes_metadata.display.show_negative_labels && !axis.negative_label.trim().is_empty() {
            let negative_endpoint = Vec3::new(
                origin.x - positive_endpoint.x,
                origin.y - positive_endpoint.y,
                origin.z - positive_endpoint.z,
            );

            let negative_endpoint_screen = draw_axis_line(
                canvas,
                projector,
                transform,
                origin,
                negative_endpoint,
                character,
            );

            let label_screen =
                world_axis_label_position(axis.id.as_str(), negative_endpoint_screen, true);
            canvas.draw_text(label_screen, &axis.negative_label);
        }
    }

    if axes_metadata.display.show_origin {
        let origin_screen = projector.project(transform.transform_vertex(origin));
        canvas.set(origin_screen, 'O');
    }

    Ok(())
}

fn draw_camera_gizmo(canvas: &mut Canvas, projector: &ObliqueProjector, camera: Camera3D) {
    let origin_world = camera.position;

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

fn draw_single_p_at_world_position(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    position: Vec3,
    stroke_character: Option<char>,
) -> io::Result<()> {
    let (word, metadata) = load_single_p_word_assets()?;

    let word_world = Mat4::translation(position.x, position.y, position.z)
        * Mat4::uniform_scale(P_WORD_WORLD_SCALE);

    render_word_with_stroke_character(
        canvas,
        projector,
        &word,
        &metadata,
        word_world,
        stroke_character,
    )?;

    Ok(())
}

fn draw_metadata(
    _canvas: &mut Canvas,
    _world: WorldSpace3D,
    _camera: Camera3D,
    _yaw_degrees: f32,
    _pitch_degrees: f32,
    _stroke_character: Option<char>,
) {
}

pub fn render(
    canvas: &mut Canvas,
    camera_position: Vec3,
    camera_yaw_degrees: f32,
    camera_pitch_degrees: f32,
    stroke_character: Option<char>,
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
        stroke_character,
    );
    draw_world_axes(canvas, &projector, &axes_mesh, &axes_metadata)?;
    draw_single_p_at_world_position(
        canvas,
        &projector,
        Vec3::new(P2_WORD_WORLD_X, P2_WORD_WORLD_Y, P2_WORD_WORLD_Z),
        stroke_character,
    )?;
    draw_single_p_at_world_position(
        canvas,
        &projector,
        Vec3::new(P_WORD_WORLD_X, P_WORD_WORLD_Y, P_WORD_WORLD_Z),
        stroke_character,
    )?;
    draw_camera_gizmo(canvas, &projector, camera);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CAMERA_GIZMO_SCREEN_LEG, camera_for_debug, vec3_cross, vec3_length, vec3_normalize,
        vec3_subtract,
    };
    use crate::{
        glyphs::{WordAsset, read_json},
        math::Vec3,
    };

    fn vec3_dot(a: Vec3, b: Vec3) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

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

    #[test]
    fn single_p_word_asset_loads() {
        let word: WordAsset =
            read_json("assets/words/single_p.word.json").expect("single_p word asset should load");

        assert_eq!(word.children.len(), 1);
    }
}
