use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

use crate::{
    render::{
        GeoJsonMapAsset, MeshAsset, RenderNode, RenderObject, RenderScene, load_geojson_map_asset,
        load_obj_mesh,
    },
    scene::{load_scene_document, scene_document_to_render_scene},
};

fn validate_scene(scene: &RenderScene) -> io::Result<()> {
    let mut quad_count = 0;

    for object in &scene.objects {
        let RenderObject::QuadGroup(group) = object else {
            continue;
        };

        quad_count += group.quads.len();

        for quad in &group.quads {
            if !quad.position.iter().all(|value| value.is_finite()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} position must be finite", quad.id),
                ));
            }

            if !quad
                .size
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} size must be positive and finite", quad.id),
                ));
            }

            if !quad.rotation_z_degrees.is_finite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} rotation_z_degrees must be finite", quad.id),
                ));
            }

            if quad.marker.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("quad {} marker must not be empty", quad.id),
                ));
            }
        }
    }

    if scene.objects.is_empty() && scene.groups.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scene must contain at least one object or group",
        ));
    }

    if quad_count == 0 && scene.groups.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scene must contain at least one quad or group",
        ));
    }

    Ok(())
}

pub fn read_scene(path: impl AsRef<Path>) -> io::Result<RenderScene> {
    let document = load_scene_document(path)?;
    let scene = scene_document_to_render_scene(document);

    validate_scene(&scene)?;

    Ok(scene)
}

fn collect_mesh_assets_from_nodes(nodes: &[RenderNode], assets: &mut Vec<String>) {
    for node in nodes {
        match node {
            RenderNode::Group(group) => collect_mesh_assets_from_nodes(&group.children, assets),
            RenderNode::Object(object_node) => {
                if let RenderObject::Mesh(mesh_object) = &object_node.object {
                    assets.push(mesh_object.mesh_asset.clone());
                }
            }
        }
    }
}

fn collect_mesh_assets(scene: &RenderScene) -> Vec<String> {
    let mut assets = Vec::new();

    for group in &scene.groups {
        collect_mesh_assets_from_nodes(&group.children, &mut assets);
    }

    assets.sort();
    assets.dedup();
    assets
}

fn collect_map_assets_from_nodes(nodes: &[RenderNode], assets: &mut Vec<String>) {
    for node in nodes {
        match node {
            RenderNode::Group(group) => collect_map_assets_from_nodes(&group.children, assets),
            RenderNode::Object(object_node) => {
                if let RenderObject::GeoJsonMap(map_object) = &object_node.object {
                    assets.push(map_object.asset.clone());
                }
            }
        }
    }
}

fn collect_map_assets(scene: &RenderScene) -> Vec<String> {
    let mut assets = Vec::new();

    for group in &scene.groups {
        collect_map_assets_from_nodes(&group.children, &mut assets);
    }

    assets.sort();
    assets.dedup();
    assets
}

fn resolve_scene_asset_path(scene_path: &Path, asset: &str) -> PathBuf {
    let direct = PathBuf::from(asset);

    if direct.exists() {
        return direct;
    }

    if let Some(scene_dir) = scene_path.parent() {
        let scene_relative = scene_dir.join(asset);

        if scene_relative.exists() {
            return scene_relative;
        }
    }

    let assets_relative = Path::new("assets").join(asset);

    if assets_relative.exists() {
        return assets_relative;
    }

    if let Some(file_name) = Path::new(asset).file_name() {
        let model_relative = Path::new("assets").join("models").join(file_name);

        if model_relative.exists() {
            return model_relative;
        }

        let map_relative = Path::new("assets").join("maps").join(file_name);

        if map_relative.exists() {
            return map_relative;
        }
    }

    direct
}

pub fn load_scene_meshes(
    scene_path: &Path,
    scene: &RenderScene,
) -> io::Result<HashMap<String, MeshAsset>> {
    let mut meshes = HashMap::new();

    for asset in collect_mesh_assets(scene) {
        let path = resolve_scene_asset_path(scene_path, &asset);

        if !path.exists() {
            continue;
        }

        meshes.insert(asset, load_obj_mesh(path)?);
    }

    Ok(meshes)
}

pub fn load_scene_maps(
    scene_path: &Path,
    scene: &RenderScene,
) -> io::Result<HashMap<String, GeoJsonMapAsset>> {
    let mut maps = HashMap::new();

    for asset in collect_map_assets(scene) {
        let path = resolve_scene_asset_path(scene_path, &asset);

        if !path.exists() {
            continue;
        }

        maps.insert(asset, load_geojson_map_asset(&path)?);
    }

    Ok(maps)
}

#[cfg(test)]
mod tests {
    use super::resolve_scene_asset_path;
    use std::path::Path;

    #[test]
    fn missing_asset_falls_back_to_direct_path() {
        assert_eq!(
            resolve_scene_asset_path(Path::new("assets/scenes/example.scene.json"), "missing.obj"),
            Path::new("missing.obj")
        );
    }
}
