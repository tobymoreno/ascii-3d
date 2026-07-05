#!/usr/bin/env python3
from pathlib import Path
import shutil

def move_asset(old: str, new: str) -> None:
    old_path = Path(old)
    new_path = Path(new)
    new_path.parent.mkdir(parents=True, exist_ok=True)

    if old_path.exists():
        shutil.move(str(old_path), str(new_path))
    elif new_path.exists():
        pass
    else:
        raise SystemExit(f"Missing asset: expected {old} or {new}")

def replace_in_file(path: str, replacements: list[tuple[str, str]]) -> None:
    file_path = Path(path)
    if not file_path.exists():
        return

    text = file_path.read_text()
    original = text

    for old, new in replacements:
        text = text.replace(old, new)

    if text != original:
        file_path.write_text(text)

def replace_in_many(glob_pattern: str, replacements: list[tuple[str, str]]) -> None:
    for path in Path(".").glob(glob_pattern):
        if path.is_file():
            replace_in_file(str(path), replacements)

def main() -> None:
    move_asset("assets/cartesian_axes.obj", "assets/models/cartesian_axes.obj")
    move_asset("assets/quad4.obj", "assets/models/quad4.obj")

    # Root asset metadata uses paths relative to assets/.
    replace_in_file(
        "assets/cartesian_axes.json",
        [
            ('"geometry_asset": "cartesian_axes.obj"', '"geometry_asset": "models/cartesian_axes.obj"'),
        ],
    )

    # Scene JSON files use paths relative to repo root.
    replace_in_many(
        "assets/scenes/*.scene.json",
        [
            ('"geometry_asset": "assets/cartesian_axes.obj"', '"geometry_asset": "assets/models/cartesian_axes.obj"'),
        ],
    )

    replace_in_file(
        "assets/quad4.scene.json",
        [
            ('"mesh_asset": "quad4.obj"', '"mesh_asset": "models/quad4.obj"'),
        ],
    )

    # Rust code/tests/messages.
    rust_replacements = [
        ('const AXES_ASSET: &str = "assets/cartesian_axes.obj";', 'const AXES_ASSET: &str = "assets/models/cartesian_axes.obj";'),
        ('"cartesian_axes.obj".to_string()', '"models/cartesian_axes.obj".to_string()'),
        ('asset_path("quad4.obj").is_file()', 'asset_path("models/quad4.obj").is_file()'),
        ('load_mesh_asset("quad4.obj").expect("quad4.obj should load")', 'load_mesh_asset("models/quad4.obj").expect("models/quad4.obj should load")'),
        ('quad4_scene_config.mesh_asset != "quad4.obj"', 'quad4_scene_config.mesh_asset != "models/quad4.obj"'),
        ('"assets/quad4.obj expected 4 vertices, but loaded {}"', '"assets/models/quad4.obj expected 4 vertices, but loaded {}"'),
        ('"assets/quad4.obj expected 1 face, but loaded {}"', '"assets/models/quad4.obj expected 1 face, but loaded {}"'),
        ('"Config: assets/quad4.scene.json"', '"Config: assets/quad4.scene.json | Mesh: assets/models/quad4.obj"'),
        ('mesh_asset: "quad4.obj".to_string()', 'mesh_asset: "models/quad4.obj".to_string()'),
        ('assert_eq!(config.mesh_asset, "quad4.obj");', 'assert_eq!(config.mesh_asset, "models/quad4.obj");'),
    ]

    for path in Path("src").rglob("*.rs"):
        replace_in_file(str(path), rust_replacements)

    # Update README if present.
    readme = Path("assets/models/README.md")
    if readme.exists():
        text = readme.read_text()
    else:
        text = "# Standard OBJ models\n\n"

    additions = [
        "- `cube.obj`: standard cube mesh used by the rotating OBJ scene.",
        "- `pyramid.obj`: simple reference pyramid mesh.",
        "- `cartesian_axes.obj`: line-based Cartesian axes geometry.",
        "- `quad4.obj`: four-vertex quad mesh used for camera frustum planes.",
    ]

    for line in additions:
        if line not in text:
            text = text.rstrip() + "\n" + line + "\n"

    readme.write_text(text)

    print("Standardized core OBJ assets under assets/models/.")

if __name__ == "__main__":
    main()
