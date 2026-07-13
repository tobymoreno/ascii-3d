use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

use crate::{math::Vec3, mesh::Mesh, obj::load_obj};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshTriangle {
    pub a: MeshVertex,
    pub b: MeshVertex,
    pub c: MeshVertex,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshAsset {
    pub triangles: Vec<MeshTriangle>,
    pub vertex_count: usize,
    pub normal_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshPrepareOptions {
    pub normalize_to_size: Option<f32>,
    pub grid_size: Option<f32>,
    pub target_vertices: Option<usize>,
    pub cache: bool,
}

static PREPARED_MESH_CACHE: OnceLock<Mutex<HashMap<String, Arc<Mesh>>>> = OnceLock::new();

pub fn load_prepared_mesh(
    path: impl AsRef<Path>,
    options: MeshPrepareOptions,
) -> io::Result<Arc<Mesh>> {
    let path = path.as_ref();
    let cache_key = format!("{}|{options:?}", path.display());

    if let Some(mesh) = PREPARED_MESH_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| io::Error::other("prepared mesh cache lock poisoned"))?
        .get(&cache_key)
        .cloned()
    {
        return Ok(mesh);
    }

    let disk_cache_path = prepared_mesh_disk_cache_path(path, options)?;
    let mesh = if let Some(cache_path) = disk_cache_path.as_ref().filter(|path| path.is_file()) {
        load_obj(cache_path).map_err(|error| {
            io::Error::other(format!(
                "failed to load cached mesh {}: {error}",
                cache_path.display()
            ))
        })?
    } else {
        let mut mesh = load_obj(path).map_err(|error| {
            io::Error::other(format!("failed to load OBJ {}: {error}", path.display()))
        })?;

        if let Some(target_size) = options.normalize_to_size {
            if !mesh.normalize_to_size(target_size) {
                return Err(io::Error::other(format!(
                    "could not normalize mesh {}",
                    path.display()
                )));
            }
        }

        if let Some(target_vertices) = options.target_vertices.filter(|value| *value > 0) {
            mesh = mesh.simplify_to_target_vertices(target_vertices);
        } else if let Some(grid_size) = options
            .grid_size
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            mesh = mesh.simplify_by_vertex_grid(grid_size);
        }

        if let Some(cache_path) = &disk_cache_path {
            let temp_path = cache_path.with_extension("obj.tmp");
            mesh.write_obj(&temp_path)?;
            fs::rename(&temp_path, cache_path)?;
        }

        mesh
    };

    let mesh = Arc::new(mesh);
    PREPARED_MESH_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| io::Error::other("prepared mesh cache lock poisoned"))?
        .insert(cache_key, Arc::clone(&mesh));

    Ok(mesh)
}

pub fn load_obj_mesh(path: impl AsRef<Path>) -> io::Result<MeshAsset> {
    load_obj_mesh_prepared(path, MeshPrepareOptions::default())
}

pub fn load_obj_mesh_prepared(
    path: impl AsRef<Path>,
    options: MeshPrepareOptions,
) -> io::Result<MeshAsset> {
    let mesh = load_prepared_mesh(path, options)?;
    Ok(mesh_asset_from_indexed_mesh(&mesh))
}

pub fn load_obj_mesh_from_str(text: &str) -> io::Result<MeshAsset> {
    let mesh = crate::obj::parse_obj(text)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok(mesh_asset_from_indexed_mesh(&mesh))
}

fn mesh_asset_from_indexed_mesh(mesh: &Mesh) -> MeshAsset {
    let mut triangles = Vec::new();

    for primitive in &mesh.faces {
        if primitive.len() < 3 {
            continue;
        }

        let first = primitive[0];
        for index in 1..primitive.len() - 1 {
            let indexes = [first, primitive[index], primitive[index + 1]];
            if indexes.iter().any(|value| *value >= mesh.vertices.len()) {
                continue;
            }

            let a = mesh.vertices[indexes[0]];
            let b = mesh.vertices[indexes[1]];
            let c = mesh.vertices[indexes[2]];
            let normal = normalized_vec3(cross(b - a, c - a));
            let normal = [normal.x, normal.y, normal.z];

            triangles.push(MeshTriangle {
                a: MeshVertex {
                    position: [a.x, a.y, a.z],
                    normal,
                },
                b: MeshVertex {
                    position: [b.x, b.y, b.z],
                    normal,
                },
                c: MeshVertex {
                    position: [c.x, c.y, c.z],
                    normal,
                },
            });
        }
    }

    MeshAsset {
        triangles,
        vertex_count: mesh.vertices.len(),
        normal_count: 0,
    }
}

fn prepared_mesh_disk_cache_path(
    mesh_path: &Path,
    options: MeshPrepareOptions,
) -> io::Result<Option<PathBuf>> {
    if !options.cache {
        return Ok(None);
    }

    let source = fs::read(mesh_path)?;
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    options
        .normalize_to_size
        .map(f32::to_bits)
        .hash(&mut hasher);
    options.grid_size.map(f32::to_bits).hash(&mut hasher);
    options.target_vertices.hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    let digest = hasher.finish();

    let Some(dir) = mesh_cache_dir() else {
        return Ok(None);
    };
    fs::create_dir_all(&dir)?;

    let stem = mesh_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mesh");

    Ok(Some(dir.join(format!("{stem}-{digest:016x}.obj"))))
}

fn mesh_cache_dir() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("ASCII_3D_CACHE_DIR") {
        return Some(PathBuf::from(value).join("meshes"));
    }

    if cfg!(target_os = "macos") {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library/Caches/ascii-3d/meshes"));
    }

    if let Some(value) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(value).join("ascii-3d/meshes"));
    }

    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".cache/ascii-3d/meshes"))
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn normalized_vec3(value: Vec3) -> Vec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length <= f32::EPSILON {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        value * (1.0 / length)
    }
}

#[cfg(test)]
mod tests {
    use super::load_obj_mesh_from_str;

    #[test]
    fn loads_single_triangle_obj() {
        let mesh = load_obj_mesh_from_str(
            r#"
            v 0 0 0
            v 1 0 0
            v 0 1 0
            vn 0 0 1
            f 1//1 2//1 3//1
            "#,
        )
        .expect("mesh should load");

        assert_eq!(mesh.vertex_count, 3);
        assert_eq!(mesh.triangles.len(), 1);
        assert_eq!(mesh.triangles[0].a.position, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.triangles[0].a.normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn triangulates_quad_faces() {
        let mesh = load_obj_mesh_from_str(
            r#"
            v 0 0 0
            v 1 0 0
            v 1 1 0
            v 0 1 0
            f 1 2 3 4
            "#,
        )
        .expect("mesh should load");

        assert_eq!(mesh.vertex_count, 4);
        assert_eq!(mesh.triangles.len(), 2);
    }
}
