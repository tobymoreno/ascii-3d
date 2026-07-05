use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{A3dManifest, LoadedWorld};

#[derive(Debug, Clone)]
pub struct LoadedA3dProject {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: A3dManifest,
}

impl LoadedA3dProject {
    pub fn into_world(self) -> Result<LoadedWorld, String> {
        LoadedWorld::from_manifest(self.manifest)
    }

    pub fn resolve_asset_path(&self, relative_path: &str) -> PathBuf {
        self.root.join(relative_path)
    }
}

pub fn load_a3d_project(path: impl AsRef<Path>) -> io::Result<LoadedA3dProject> {
    let path = path.as_ref();

    let manifest_path = if path.is_dir() {
        path.join("scene.a3d")
    } else {
        path.to_path_buf()
    };

    let root = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let text = fs::read_to_string(&manifest_path)?;
    let manifest: A3dManifest = serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {}", manifest_path.display(), error),
        )
    })?;

    manifest
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

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

    #[test]
    fn loads_project_from_folder_scene_a3d() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "ascii_3d_a3d_loader_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("time should work")
                .as_nanos()
        ));

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
}
