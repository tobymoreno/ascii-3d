#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")

def replace_once(text: str, old: str, new: str) -> str:
    if old not in text:
        raise SystemExit(f"Could not find expected text:\n{old}")
    return text.replace(old, new, 1)

def main() -> None:
    text = APP.read_text()

    helper = '''
fn initial_a3d_root_path_from_args() -> PathBuf {
    let Some(argument) = std::env::args_os().nth(1) else {
        return default_a3d_root_path();
    };

    let path = PathBuf::from(argument);

    if path.is_file() {
        return path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(default_a3d_root_path);
    }

    path
}

'''

    if "fn initial_a3d_root_path_from_args(" not in text:
        text = replace_once(
            text,
            "\nfn load_default_a3d_world() -> io::Result<LoadedWorld> {\n",
            "\n" + helper + "fn load_default_a3d_world() -> io::Result<LoadedWorld> {\n",
        )

    text = replace_once(
        text,
        "    let mut state = AppState::new();\n    state.load_a3d_root(default_a3d_root_path());\n",
        "    let mut state = AppState::new();\n    state.load_a3d_root(initial_a3d_root_path_from_args());\n",
    )

    test = '''
    #[test]
    fn initial_a3d_root_path_defaults_to_bundled_demo_when_no_cli_arg_is_used() {
        let path = default_a3d_root_path();

        assert!(path.ends_with(Path::new("assets").join("a3d").join("p_depth_demo")));
    }

'''
    if "initial_a3d_root_path_defaults_to_bundled_demo_when_no_cli_arg_is_used" not in text:
        text = replace_once(
            text,
            "    #[test]\n    fn application_starts_on_loaded_a3d_scene() {\n",
            test + "    #[test]\n    fn application_starts_on_loaded_a3d_scene() {\n",
        )

    text = text.replace(
        "    use super::{AppState, asset_path, load_mesh_asset};\n",
        "    use super::{AppState, asset_path, default_a3d_root_path, load_mesh_asset};\n    use std::path::Path;\n",
    )

    APP.write_text(text)
    print("Fixed startup to load initial A3D root from the first CLI argument.")

if __name__ == "__main__":
    main()
