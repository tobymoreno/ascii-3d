#!/usr/bin/env python3
from pathlib import Path
import json
import shutil

SOURCE_SCENE = Path("assets/a3d/external_teapot_raster/scene.a3d")
FALLBACK_SCENE = Path("assets/a3d/external_teapot/scene.a3d")
STATIC_DIR = Path("assets/a3d/external_teapot_static")
STATIC_SCENE = STATIC_DIR / "scene.a3d"


ANIMATION_KEYS = {
    "animation",
    "animations",
    "animate",
    "animated",
    "spin",
    "orbit",
    "velocity",
    "angular_velocity",
    "rotation_velocity",
    "rotation_speed",
}


def strip_animation_fields(value):
    if isinstance(value, dict):
        return {
            key: strip_animation_fields(child)
            for key, child in value.items()
            if key not in ANIMATION_KEYS
        }

    if isinstance(value, list):
        return [strip_animation_fields(child) for child in value]

    return value


def ensure_transform_static(obj: dict) -> None:
    transform = obj.setdefault("transform", {})
    if not isinstance(transform, dict):
        transform = {}
        obj["transform"] = transform

    transform["rotation"] = [0.0, 0.0, 0.0]


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


def main() -> None:
    source = SOURCE_SCENE if SOURCE_SCENE.exists() else FALLBACK_SCENE
    if not source.exists():
        raise SystemExit(
            "Could not find a source scene. Expected one of: "
            f"{SOURCE_SCENE} or {FALLBACK_SCENE}"
        )

    data = json.loads(source.read_text())
    data = strip_animation_fields(data)

    data["title"] = "Static external OBJ teapot raster inspection"

    objects = data.setdefault("objects", [])
    if not isinstance(objects, list) or not objects:
        raise SystemExit(f"Source scene has no objects: {source}")

    for obj in objects:
        if not isinstance(obj, dict):
            continue

        ensure_transform_static(obj)
        ensure_raster_render(obj)

    ensure_light(data)

    STATIC_DIR.mkdir(parents=True, exist_ok=True)
    STATIC_SCENE.write_text(json.dumps(data, indent=2) + "\n")
    json.loads(STATIC_SCENE.read_text())

    print(f"Created {STATIC_SCENE}")
    print("Static teapot scene is rasterized, visible, and rotation is reset to [0, 0, 0].")


if __name__ == "__main__":
    main()
