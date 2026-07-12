use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use crate::math::Mat4;

use super::{A3dManifest, A3dObject, AssetRef, LoadedWorld, SceneObject};

#[derive(Debug, Clone)]
pub struct LoadedA3dProject {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: A3dManifest,
}

impl LoadedA3dProject {
    pub fn into_world(self) -> Result<LoadedWorld, String> {
        let mut objects = Vec::new();
        let mut loading = HashSet::new();

        expand_objects(
            &self.root,
            self.manifest.objects,
            Mat4::identity(),
            "",
            false,
            &mut loading,
            &mut objects,
        )
        .map_err(|error| error.to_string())?;

        LoadedWorld::from_expanded(self.manifest.title, self.manifest.world.physics, objects)
    }

    pub fn resolve_asset_path(&self, relative_path: &str) -> PathBuf {
        self.root.join(relative_path)
    }
}

fn load_manifest_file(path: &Path) -> io::Result<(PathBuf, A3dManifest)> {
    let text = fs::read_to_string(path)?;
    let manifest: A3dManifest = serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {}", path.display(), error),
        )
    })?;

    manifest
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    let root = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    Ok((root, manifest))
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[allow(clippy::too_many_arguments)]
fn expand_objects(
    root: &Path,
    objects: Vec<A3dObject>,
    parent_matrix: Mat4,
    id_prefix: &str,
    hidden_by_composite: bool,
    loading: &mut HashSet<PathBuf>,
    output: &mut Vec<SceneObject>,
) -> io::Result<()> {
    for mut object in objects {
        let qualified_id = if id_prefix.is_empty() {
            object.id.clone()
        } else {
            format!("{id_prefix}/{}", object.id)
        };

        match &object.asset {
            AssetRef::Group { path } => {
                let group_path = resolve_path(root, path);
                let canonical =
                    fs::canonicalize(&group_path).unwrap_or_else(|_| group_path.clone());

                if !loading.insert(canonical.clone()) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "recursive .a3d group cycle detected at {}",
                            group_path.display()
                        ),
                    ));
                }

                let (group_root, group_manifest) = load_manifest_file(&group_path)?;
                let group_matrix = parent_matrix * object.transform.matrix();
                let hide_children = hidden_by_composite || object.editor_composite;

                object.asset.resolve_paths(root);
                output.push(SceneObject {
                    id: qualified_id.clone(),
                    asset: object.asset,
                    transform: object.transform,
                    render: object.render,
                    behaviors: object.behaviors,
                    physics: object.physics,
                    parent_matrix,
                    editor_composite: object.editor_composite,
                    editor_hidden: hidden_by_composite,
                    source_root: root.to_path_buf(),
                });

                expand_objects(
                    &group_root,
                    group_manifest.objects,
                    group_matrix,
                    &qualified_id,
                    hide_children,
                    loading,
                    output,
                )?;

                loading.remove(&canonical);
            }
            _ => {
                object.asset.resolve_paths(root);
                output.push(SceneObject {
                    id: qualified_id,
                    asset: object.asset,
                    transform: object.transform,
                    render: object.render,
                    behaviors: object.behaviors,
                    physics: object.physics,
                    parent_matrix,
                    editor_composite: object.editor_composite,
                    editor_hidden: hidden_by_composite,
                    source_root: root.to_path_buf(),
                });
            }
        }
    }

    Ok(())
}

pub fn load_a3d_project(path: impl AsRef<Path>) -> io::Result<LoadedA3dProject> {
    let path = path.as_ref();

    let manifest_path = if path.is_dir() {
        path.join("scene.a3d")
    } else {
        path.to_path_buf()
    };

    let (root, manifest) = load_manifest_file(&manifest_path)?;

    Ok(LoadedA3dProject {
        root,
        manifest_path,
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::load_a3d_project;
    use std::{fs, time::SystemTime};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "ascii_3d_{name}_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("time should work")
                .as_nanos()
        ));
        dir
    }

    #[test]
    fn loads_project_from_folder_scene_a3d() {
        let dir = temp_dir("a3d_loader_test");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        fs::write(
            dir.join("scene.a3d"),
            r#"{
              "version": 1,
              "title": "Loader test",
              "objects": []
            }"#,
        )
        .expect("manifest should be written");

        let project = load_a3d_project(&dir).expect("project should load");

        assert_eq!(project.manifest.title, "Loader test");
        assert_eq!(project.manifest_path, dir.join("scene.a3d"));

        fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn expands_reusable_composite_group_and_hides_children_from_editor() {
        let dir = temp_dir("a3d_group_test");
        fs::create_dir_all(dir.join("group")).unwrap();

        fs::write(
            dir.join("group/group.a3d"),
            r#"{
              "version": 1,
              "title": "Group",
              "objects": [{
                "id": "child",
                "asset": {"type": "mesh", "path": "child.obj"}
              }]
            }"#,
        )
        .unwrap();

        fs::write(
            dir.join("scene.a3d"),
            r#"{
              "version": 1,
              "title": "Scene",
              "objects": [{
                "id": "logo",
                "asset": {"type": "group", "path": "group/group.a3d"},
                "editor_composite": true
              }]
            }"#,
        )
        .unwrap();

        let world = load_a3d_project(&dir).unwrap().into_world().unwrap();

        assert_eq!(world.objects.len(), 2);
        assert_eq!(world.objects[0].id, "logo");
        assert!(!world.objects[0].editor_hidden);
        assert_eq!(world.objects[1].id, "logo/child");
        assert!(world.objects[1].editor_hidden);

        fs::remove_dir_all(&dir).unwrap();
    }
}
