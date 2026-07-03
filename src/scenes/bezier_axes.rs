use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    axis_metadata::{CartesianAxesMetadata, load_cartesian_axes_metadata},
    canvas::Canvas,
    curves::CubicBezier3,
    geometry2d::Point2,
    math::{Mat4, Vec3},
    mesh::Mesh,
    mesh_renderer::MeshTransform,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::load_projection_config,
};

use super::render_asset_axes;

const SCENE_ASSET: &str = "assets/scenes/bezier_axes_xy.scene.json";

#[derive(Debug, Deserialize)]
struct BezierSceneAsset {
    name: String,
    version: u32,
    projection_preset: String,
    nodes: Vec<SceneNode>,
}

#[derive(Debug, Clone, Deserialize)]
struct SceneNode {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    geometry_asset: Option<String>,
    metadata_asset: Option<String>,
    curve_asset: Option<String>,
    parent: Option<String>,
    local_transform: TransformConfig,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct TransformConfig {
    translation: [f32; 3],
    rotation_degrees: [f32; 3],
    scale: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct CurveAsset {
    name: String,
    version: u32,
    #[serde(rename = "type")]
    curve_type: String,
    control_points: ControlPoints,
    sampling: SamplingConfig,
}

#[derive(Debug, Deserialize)]
struct ControlPoints {
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct SamplingConfig {
    default_segments: usize,
}

#[derive(Debug, Deserialize)]
struct CurveMetadata {
    display: CurveDisplay,
}

#[derive(Debug, Deserialize)]
struct CurveDisplay {
    show_curve: bool,
    show_control_points: bool,
    show_control_polygon: bool,
    curve_character: char,
    control_polygon_character: char,
    control_point_labels: ControlPointLabels,
}

#[derive(Debug, Deserialize)]
struct ControlPointLabels {
    p0: String,
    p1: String,
    p2: String,
    p3: String,
}

fn asset_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn read_json<T>(relative_path: &str) -> io::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let path = asset_path(relative_path);

    let text = fs::read_to_string(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read {}: {}", path.display(), error),
        )
    })?;

    serde_json::from_str(&text)
        .map_err(|error| io::Error::other(format!("failed to parse {}: {}", path.display(), error)))
}

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn transform_matrix(config: TransformConfig) -> Mat4 {
    Mat4::translation_vec3(vec3(config.translation))
        * Mat4::rotation_z(config.rotation_degrees[2].to_radians())
        * Mat4::rotation_y(config.rotation_degrees[1].to_radians())
        * Mat4::rotation_x(config.rotation_degrees[0].to_radians())
        * Mat4::scale(config.scale[0], config.scale[1], config.scale[2])
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

fn load_scene_assets() -> io::Result<(
    BezierSceneAsset,
    SceneNode,
    SceneNode,
    Mesh,
    CartesianAxesMetadata,
    CurveAsset,
    CurveMetadata,
    ObliqueProjector,
)> {
    let scene: BezierSceneAsset = read_json(SCENE_ASSET)?;

    if scene.version != 1 {
        return Err(io::Error::other(format!(
            "unsupported Bezier scene version {}",
            scene.version,
        )));
    }

    let axes_node = scene
        .nodes
        .iter()
        .find(|node| node.node_type == "cartesian_axes")
        .ok_or_else(|| io::Error::other("Bezier scene is missing cartesian_axes node"))?
        .clone();

    let curve_node = scene
        .nodes
        .iter()
        .find(|node| node.node_type == "cubic_bezier_curve")
        .ok_or_else(|| io::Error::other("Bezier scene is missing cubic_bezier_curve node"))?
        .clone();

    if curve_node.parent.as_deref() != Some(&axes_node.id) {
        return Err(io::Error::other(
            "Bezier curve node must be parented to the Cartesian axes node",
        ));
    }

    let axes_mesh = load_mesh(
        axes_node
            .geometry_asset
            .as_deref()
            .ok_or_else(|| io::Error::other("axes node missing geometry_asset"))?,
    )?;

    let axes_metadata = load_axes_metadata(
        axes_node
            .metadata_asset
            .as_deref()
            .ok_or_else(|| io::Error::other("axes node missing metadata_asset"))?,
    )?;

    let curve_asset: CurveAsset = read_json(
        curve_node
            .curve_asset
            .as_deref()
            .ok_or_else(|| io::Error::other("curve node missing curve_asset"))?,
    )?;

    if curve_asset.version != 1 || curve_asset.curve_type != "cubic_bezier_3d" {
        return Err(io::Error::other(format!(
            "unsupported curve asset '{}' version {} type {}",
            curve_asset.name, curve_asset.version, curve_asset.curve_type,
        )));
    }

    let curve_metadata: CurveMetadata = read_json(
        curve_node
            .metadata_asset
            .as_deref()
            .ok_or_else(|| io::Error::other("curve node missing metadata_asset"))?,
    )?;

    let projection = load_projection_config(asset_path(&scene.projection_preset))?;

    let projector = ObliqueProjector::from_axis_vectors(
        Point2::new(projection.screen_origin[0], projection.screen_origin[1]),
        projection.axis_vectors.x,
        projection.axis_vectors.y,
        projection.axis_vectors.z,
    );

    Ok((
        scene,
        axes_node,
        curve_node,
        axes_mesh,
        axes_metadata,
        curve_asset,
        curve_metadata,
        projector,
    ))
}

fn draw_control_point(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    point: Vec3,
    label: &str,
    marker: char,
) {
    let projected = projector.project(point);

    canvas.set(projected, marker);
    canvas.draw_text(Point2::new(projected.x + 2, projected.y), label);
}

pub fn render(canvas: &mut Canvas) -> io::Result<()> {
    let (
        scene,
        axes_node,
        curve_node,
        axes_mesh,
        axes_metadata,
        curve_asset,
        curve_metadata,
        projector,
    ) = load_scene_assets()?;

    let axes_world = transform_matrix(axes_node.local_transform);
    let curve_world = axes_world * transform_matrix(curve_node.local_transform);

    let axes_transform = MeshTransform {
        rotation_x: axes_node.local_transform.rotation_degrees[0].to_radians(),
        rotation_y: axes_node.local_transform.rotation_degrees[1].to_radians(),
        rotation_z: axes_node.local_transform.rotation_degrees[2].to_radians(),
        scale: axes_node.local_transform.scale[0],
        translation: vec3(axes_node.local_transform.translation),
    };

    render_asset_axes(
        canvas,
        &projector,
        &axes_mesh,
        &axes_metadata,
        axes_transform,
    )?;

    let curve = CubicBezier3::new(
        vec3(curve_asset.control_points.p0),
        vec3(curve_asset.control_points.p1),
        vec3(curve_asset.control_points.p2),
        vec3(curve_asset.control_points.p3),
    );

    let display = &curve_metadata.display;

    let p0 = curve_world.transform_point(curve.p0);
    let p1 = curve_world.transform_point(curve.p1);
    let p2 = curve_world.transform_point(curve.p2);
    let p3 = curve_world.transform_point(curve.p3);

    if display.show_control_polygon {
        let c = display.control_polygon_character;

        canvas.draw_line(projector.project(p0), projector.project(p1), c);
        canvas.draw_line(projector.project(p1), projector.project(p2), c);
        canvas.draw_line(projector.project(p2), projector.project(p3), c);
    }

    if display.show_curve {
        let sampled = curve.sample(curve_asset.sampling.default_segments);

        for (start, end) in sampled.line_segments() {
            let start = curve_world.transform_point(start);
            let end = curve_world.transform_point(end);

            canvas.draw_line(
                projector.project(start),
                projector.project(end),
                display.curve_character,
            );
        }
    }

    if display.show_control_points {
        draw_control_point(
            canvas,
            &projector,
            p0,
            &display.control_point_labels.p0,
            '0',
        );
        draw_control_point(
            canvas,
            &projector,
            p1,
            &display.control_point_labels.p1,
            '1',
        );
        draw_control_point(
            canvas,
            &projector,
            p2,
            &display.control_point_labels.p2,
            '2',
        );
        draw_control_point(
            canvas,
            &projector,
            p3,
            &display.control_point_labels.p3,
            '3',
        );
    }

    canvas.draw_text(Point2::new(2, 1), "Scene: Bezier child of Cartesian axes");
    canvas.draw_text(Point2::new(2, 2), &format!("Asset: {}", scene.name));
    canvas.draw_text(
        Point2::new(2, 24),
        "XY editor projection: +X east, +Y north",
    );
    canvas.draw_text(
        Point2::new(2, 25),
        "Bezier is parented to axes_root; Z is depth only",
    );
    canvas.draw_text(
        Point2::new(2, 26),
        "Curve '*' is sampled from a mathematical cubic Bezier",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BezierSceneAsset, CurveAsset, CurveMetadata, read_json};

    #[test]
    fn bezier_scene_asset_loads() {
        let scene: BezierSceneAsset =
            read_json("assets/scenes/bezier_axes_xy.scene.json").expect("scene should load");

        assert_eq!(scene.version, 1);
        assert_eq!(scene.nodes.len(), 2);
    }

    #[test]
    fn bezier_curve_asset_loads() {
        let curve: CurveAsset =
            read_json("assets/curves/bezier_demo.curve.json").expect("curve should load");

        assert_eq!(curve.curve_type, "cubic_bezier_3d");
        assert_eq!(curve.sampling.default_segments, 32);
    }

    #[test]
    fn bezier_metadata_loads() {
        let metadata: CurveMetadata =
            read_json("assets/curves/bezier_demo.metadata.json").expect("metadata should load");

        assert_eq!(metadata.display.curve_character, '*');
        assert!(metadata.display.show_control_points);
    }
}
