use std::{io, path::Path, path::PathBuf};

use serde::Deserialize;

use crate::{
    axis_metadata::{CartesianAxesMetadata, load_cartesian_axes_metadata},
    canvas::Canvas,
    geometry2d::Point2,
    glyphs::{
        TransformConfig, WordAsset, WordMetadata, read_json, render_word_with_stroke_character,
        transform_matrix, vec3,
    },
    mesh::Mesh,
    mesh_renderer::MeshTransform,
    obj::load_obj,
    projection::ObliqueProjector,
    projection_config::load_projection_config,
};

use super::render_asset_axes;

const SCENE_ASSET: &str = "assets/scenes/pitt_crew_axes.scene.json";

#[derive(Debug, Clone, Deserialize)]
struct PittCrewSceneAsset {
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
    word_asset: Option<String>,
    parent: Option<String>,
    local_transform: TransformConfig,
}

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

fn load_scene_assets() -> io::Result<(
    PittCrewSceneAsset,
    SceneNode,
    SceneNode,
    Mesh,
    CartesianAxesMetadata,
    WordAsset,
    WordMetadata,
    ObliqueProjector,
)> {
    let scene: PittCrewSceneAsset = read_json(SCENE_ASSET)?;

    if scene.version != 1 {
        return Err(io::Error::other(format!(
            "unsupported pitt_crew scene version {}",
            scene.version,
        )));
    }

    let axes_node = scene
        .nodes
        .iter()
        .find(|node| node.node_type == "cartesian_axes")
        .ok_or_else(|| io::Error::other("pitt_crew scene is missing cartesian_axes node"))?
        .clone();

    let word_node = scene
        .nodes
        .iter()
        .find(|node| node.node_type == "bezier_word")
        .ok_or_else(|| io::Error::other("pitt_crew scene is missing bezier_word node"))?
        .clone();

    if word_node.parent.as_deref() != Some(&axes_node.id) {
        return Err(io::Error::other(
            "pitt_crew word node must be parented to the Cartesian axes node",
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

    let word: WordAsset = read_json(
        word_node
            .word_asset
            .as_deref()
            .ok_or_else(|| io::Error::other("word node missing word_asset"))?,
    )?;

    let word_metadata: WordMetadata = read_json("assets/words/pitt_crew.metadata.json")?;

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
        word_node,
        axes_mesh,
        axes_metadata,
        word,
        word_metadata,
        projector,
    ))
}

pub fn render(canvas: &mut Canvas, stroke_character: Option<char>) -> io::Result<()> {
    let (scene, axes_node, word_node, axes_mesh, axes_metadata, word, word_metadata, projector) =
        load_scene_assets()?;

    let axes_world = transform_matrix(axes_node.local_transform);
    let word_world = axes_world * transform_matrix(word_node.local_transform);

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

    render_word_with_stroke_character(
        canvas,
        &projector,
        &word,
        &word_metadata,
        word_world,
        stroke_character,
    )?;

    canvas.draw_text(Point2::new(2, 1), "Scene: pitt_crew word parent");
    canvas.draw_text(Point2::new(2, 2), &format!("Asset: {}", scene.name));
    canvas.draw_text(
        Point2::new(2, 24),
        "Hierarchy: axes_root -> word_pitt_crew -> glyph_P_0 -> glyph_I_1 -> glyph_T_2 -> glyph_T_3 -> glyph_SPACE_4 -> glyph_C_5 -> glyph_R_6 -> glyph_E_7 -> glyph_W_8",
    );
    canvas.draw_text(
        Point2::new(2, 25),
        "PITT CREW uses four monospace 0..1 glyph boxes",
    );
    canvas.draw_text(
        Point2::new(2, 26),
        "Runtime stroke character cycling applies to all PITT CREW glyphs",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PittCrewSceneAsset, read_json};

    #[test]
    fn pitt_crew_scene_asset_loads() {
        let scene: PittCrewSceneAsset =
            read_json("assets/scenes/pitt_crew_axes.scene.json").expect("scene should load");

        assert_eq!(scene.version, 1);
        assert_eq!(scene.nodes.len(), 2);
    }
}
