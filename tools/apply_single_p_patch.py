#!/usr/bin/env python3
from pathlib import Path

def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text()
    if old not in text:
        raise SystemExit(f"Pattern not found in {path}:\n{old}")
    path.write_text(text.replace(old, new, 1))

# src/main.rs
main_path = Path("src/main.rs")
text = main_path.read_text()
if "mod glyphs;" not in text:
    text = text.replace("mod geometry2d;\n", "mod geometry2d;\nmod glyphs;\n")
main_path.write_text(text)

# src/scenes/mod.rs
scenes_path = Path("src/scenes/mod.rs")
text = scenes_path.read_text()
if "mod single_p;" not in text:
    text = text.replace("mod rotation;\n", "mod rotation;\nmod single_p;\n")
if "pub use single_p::render as render_single_p;" not in text:
    text = text.replace("pub use rotation::{RotationAxis, render as render_rotation};\n",
                        "pub use rotation::{RotationAxis, render as render_rotation};\npub use single_p::render as render_single_p;\n")
if "SingleP," not in text:
    text = text.replace("pub enum Scene {\n    BezierAxes,", "pub enum Scene {\n    SingleP,\n    BezierAxes,")
    text = text.replace("pub const ALL: [Self; 16] = [\n        Self::BezierAxes,",
                        "pub const ALL: [Self; 17] = [\n        Self::SingleP,\n        Self::BezierAxes,")
    text = text.replace("Self::BezierAxes => \"Bezier curve child of Cartesian axes\",",
                        "Self::SingleP => \"single_p word parent with P glyph\",\n            Self::BezierAxes => \"Bezier curve child of Cartesian axes\",")
    text = text.replace("assert_eq!(Scene::ALL.first(), Some(&Scene::BezierAxes));",
                        "assert_eq!(Scene::ALL.first(), Some(&Scene::SingleP));")
    text = text.replace("fn newest_scene_is_bezier_axes()", "fn newest_scene_is_single_p()")
    text = text.replace("assert_eq!(Scene::ALL[1], Scene::AssetAxesRotateX);\n        assert_eq!(Scene::ALL[2], Scene::AssetAxesRotateY);\n        assert_eq!(Scene::ALL[3], Scene::AssetAxesRotateZ);",
                        "assert_eq!(Scene::ALL[1], Scene::BezierAxes);\n        assert_eq!(Scene::ALL[2], Scene::AssetAxesRotateX);\n        assert_eq!(Scene::ALL[3], Scene::AssetAxesRotateY);\n        assert_eq!(Scene::ALL[4], Scene::AssetAxesRotateZ);")
    text = text.replace("fn asset_axes_rotation_scenes_follow_bezier_scene()", "fn bezier_and_asset_axes_scenes_follow_single_p()")
    text = text.replace("fn quad4_is_fifth() {\n        assert_eq!(Scene::ALL[4], Scene::Quad4);\n    }",
                        "fn quad4_is_sixth() {\n        assert_eq!(Scene::ALL[5], Scene::Quad4);\n    }")
    text = text.replace("fn scene_count_is_sixteen() {\n        assert_eq!(Scene::ALL.len(), 16);\n    }",
                        "fn scene_count_is_seventeen() {\n        assert_eq!(Scene::ALL.len(), 17);\n    }")
    text = text.replace("assert!(!Scene::BezierAxes.is_animated());",
                        "assert!(!Scene::SingleP.is_animated());\n        assert!(!Scene::BezierAxes.is_animated());")
scenes_path.write_text(text)

# src/app.rs
app_path = Path("src/app.rs")
text = app_path.read_text()
if "render_single_p" not in text:
    text = text.replace("render_rotation,\n", "render_rotation, render_single_p,\n")
if "Scene::SingleP" not in text:
    text = text.replace("        match self.current_scene() {\n            Scene::BezierAxes => {",
                        "        match self.current_scene() {\n            Scene::SingleP => {\n                render_single_p(&mut canvas)?;\n            }\n\n            Scene::BezierAxes => {")
    text = text.replace("assert_eq!(state.current_scene(), Scene::BezierAxes);",
                        "assert_eq!(state.current_scene(), Scene::SingleP);")
    text = text.replace("fn application_starts_on_bezier_axes_scene()", "fn application_starts_on_single_p_scene()")
    text = text.replace("fn next_scene_moves_to_asset_axes_x_rotation() {\n        let mut state = AppState::new();\n\n        state.next_scene();\n\n        assert_eq!(state.current_scene(), Scene::AssetAxesRotateX);\n    }",
                        "fn next_scene_moves_to_bezier_axes_scene() {\n        let mut state = AppState::new();\n\n        state.next_scene();\n\n        assert_eq!(state.current_scene(), Scene::BezierAxes);\n    }")
app_path.write_text(text)

print("Applied single_p scene patch.")
