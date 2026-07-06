#!/usr/bin/env python3
from pathlib import Path
import json

APP = Path("src/app.rs")
SOURCE_SCENE = Path("assets/a3d/external_teapot_raster/scene.a3d")
FALLBACK_SCENE = Path("assets/a3d/external_teapot/scene.a3d")
STATIC_DIR = Path("assets/a3d/external_teapot_static")
STATIC_SCENE = STATIC_DIR / "scene.a3d"


def find_brace_span(text: str, marker: str) -> tuple[int, int]:
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"Could not find marker: {marker}")

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit(f"Could not find opening brace after: {marker}")

    depth = 0
    for index in range(brace, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return start, index + 1

    raise SystemExit(f"Could not find closing brace for: {marker}")


def insert_before_method(text: str, method_marker: str, addition: str) -> str:
    if addition.strip().splitlines()[0] in text:
        return text

    index = text.find(method_marker)
    if index < 0:
        raise SystemExit(f"Could not find insertion marker: {method_marker}")

    return text[:index] + addition.rstrip() + "\n\n" + text[index:]


def patch_app() -> None:
    text = APP.read_text()

    helper = '''    fn loaded_a3d_has_enabled_rotation_behavior(&self) -> bool {
        let Some(manifest_path) = self
            .loaded_a3d_manifest_path
            .clone()
            .or_else(|| self.loaded_a3d_root.as_ref().map(|root| root.join("scene.a3d")))
        else {
            return true;
        };

        let Ok(source) = std::fs::read_to_string(manifest_path) else {
            return true;
        };

        let Ok(json) = serde_json::from_str::<serde_json::Value>(&source) else {
            return true;
        };

        let Some(objects) = json.get("objects").and_then(serde_json::Value::as_array) else {
            return true;
        };

        let mut saw_rotation_behavior = false;

        for object in objects {
            let enabled = object
                .get("behavior")
                .and_then(|behavior| behavior.get("rotation"))
                .and_then(|rotation| rotation.get("enabled"))
                .and_then(serde_json::Value::as_bool);

            let Some(enabled) = enabled else {
                continue;
            };

            saw_rotation_behavior = true;

            if enabled {
                return true;
            }
        }

        // Backward-compatible default:
        // old scenes without behavior.rotation metadata keep their existing update behavior.
        !saw_rotation_behavior
    }
'''

    text = insert_before_method(text, "    fn update(&mut self, elapsed: Duration) -> bool {", helper)

    old = '''            Scene::LoadedA3d => {
                if let Some(world) = &mut self.loaded_a3d_world {
                    world.update(elapsed.as_secs_f32());
                }
                true
            }'''

    new = '''            Scene::LoadedA3d => {
                if self.loaded_a3d_has_enabled_rotation_behavior() {
                    if let Some(world) = &mut self.loaded_a3d_world {
                        world.update(elapsed.as_secs_f32());
                    }
                }
                true
            }'''

    if old not in text:
        raise SystemExit("Could not find Scene::LoadedA3d update block in src/app.rs")

    text = text.replace(old, new, 1)
    APP.write_text(text)


def ensure_transform_static(obj: dict) -> None:
    transform = obj.setdefault("transform", {})
    if not isinstance(transform, dict):
        transform = {}
        obj["transform"] = transform

    transform["rotation"] = [0.0, 0.0, 0.0]


def ensure_behavior_rotation(obj: dict, enabled: bool) -> None:
    behavior = obj.setdefault("behavior", {})
    if not isinstance(behavior, dict):
        behavior = {}
        obj["behavior"] = behavior

    rotation = behavior.setdefault("rotation", {})
    if not isinstance(rotation, dict):
        rotation = {}
        behavior["rotation"] = rotation

    rotation["enabled"] = enabled
    rotation.setdefault("axis", [0.0, 1.0, 0.0])
    rotation.setdefault("degrees_per_second", 25.0)


def ensure_raster_render(obj: dict) -> None:
    render = obj.setdefault("render", {})
    if not isinstance(render, dict):
        render = {}
        obj["render"] = render

    render["visible"] = True
    render["mode"] = "ascii_raster"
    render.setdefault("stroke_character", ".")
    render.setdefault("edge_stride", 1)
    render.setdefault("ascii_simplify", {"enabled": True, "grid_size": 0.15})
    render.pop("max_camera_cells", None)
    render.pop("shrink_edge_details", None)


def ensure_light(data: dict) -> None:
    lights = data.setdefault("lights", [])
    if not isinstance(lights, list):
        lights = []
        data["lights"] = lights

    if not lights:
        lights.append({})

    light = lights[0]
    if not isinstance(light, dict):
        light = {}
        lights[0] = light

    light["id"] = light.get("id", "key-light")
    light["type"] = "directional"
    light["position"] = light.get("position", [5.0, 2.0, -2.5])
    light["direction"] = light.get("direction", [-1.0, -0.08, 0.0])
    light["intensity"] = light.get("intensity", 1.0)

    gizmo = light.setdefault("gizmo", {})
    if not isinstance(gizmo, dict):
        gizmo = {}
        light["gizmo"] = gizmo

    gizmo["visible"] = True
    gizmo.setdefault("length", 1.0)
    gizmo.setdefault("source_character", "L")
    gizmo.setdefault("ray_character", "-")


def create_static_scene() -> None:
    source = SOURCE_SCENE if SOURCE_SCENE.exists() else FALLBACK_SCENE
    if not source.exists():
        raise SystemExit(
            "Could not find a source scene. Expected one of: "
            f"{SOURCE_SCENE} or {FALLBACK_SCENE}"
        )

    data = json.loads(source.read_text())
    data["title"] = "Static external OBJ teapot raster inspection"

    objects = data.setdefault("objects", [])
    if not isinstance(objects, list) or not objects:
        raise SystemExit(f"Source scene has no objects: {source}")

    for obj in objects:
        if not isinstance(obj, dict):
            continue

        ensure_transform_static(obj)
        ensure_behavior_rotation(obj, enabled=False)
        ensure_raster_render(obj)

    ensure_light(data)

    STATIC_DIR.mkdir(parents=True, exist_ok=True)
    STATIC_SCENE.write_text(json.dumps(data, indent=2) + "\n")
    json.loads(STATIC_SCENE.read_text())


def add_behavior_to_raster_scene() -> None:
    if not SOURCE_SCENE.exists():
        return

    data = json.loads(SOURCE_SCENE.read_text())
    changed = False

    for obj in data.get("objects", []):
        if not isinstance(obj, dict):
            continue

        behavior = obj.setdefault("behavior", {})
        if not isinstance(behavior, dict):
            behavior = {}
            obj["behavior"] = behavior

        rotation = behavior.setdefault("rotation", {})
        if not isinstance(rotation, dict):
            rotation = {}
            behavior["rotation"] = rotation

        rotation.setdefault("enabled", True)
        rotation.setdefault("axis", [0.0, 1.0, 0.0])
        rotation.setdefault("degrees_per_second", 25.0)
        changed = True

    if changed:
        SOURCE_SCENE.write_text(json.dumps(data, indent=2) + "\n")
        json.loads(SOURCE_SCENE.read_text())


def main() -> None:
    patch_app()
    add_behavior_to_raster_scene()
    create_static_scene()

    print("Added per-object behavior.rotation.enabled support for loaded A3D update gating.")
    print(f"Created {STATIC_SCENE} with behavior.rotation.enabled=false.")


if __name__ == "__main__":
    main()
