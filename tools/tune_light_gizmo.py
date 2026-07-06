#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

DEFAULT_SCENE = Path("assets/a3d/external_teapot_raster/scene.a3d")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Tune the A3D key-light gizmo position without editing JSON manually."
    )
    parser.add_argument("x", type=float, help="Light gizmo/source X position.")
    parser.add_argument("y", type=float, help="Light gizmo/source Y position.")
    parser.add_argument("z", type=float, help="Light gizmo/source Z position.")
    parser.add_argument(
        "--direction",
        nargs=3,
        type=float,
        metavar=("X", "Y", "Z"),
        default=[-1.0, -0.08, 0.0],
        help="Light direction vector. Default: -1.0 -0.08 0.0",
    )
    parser.add_argument(
        "--length",
        type=float,
        default=4.0,
        help="Gizmo ray length. Default: 4.0",
    )
    parser.add_argument(
        "--scene",
        type=Path,
        default=DEFAULT_SCENE,
        help=f"Scene path. Default: {DEFAULT_SCENE}",
    )
    parser.add_argument(
        "--show-teapot",
        action="store_true",
        help="Make scene objects visible while tuning the light.",
    )
    parser.add_argument(
        "--hide-teapot",
        action="store_true",
        help="Hide scene objects while tuning the light.",
    )

    args = parser.parse_args()

    if args.length <= 0:
        raise SystemExit("--length must be > 0")

    if not args.scene.exists():
        raise SystemExit(f"Scene file does not exist: {args.scene}")

    data = json.loads(args.scene.read_text())

    if args.show_teapot and args.hide_teapot:
        raise SystemExit("Use only one of --show-teapot or --hide-teapot")

    if args.show_teapot or args.hide_teapot:
        for obj in data.get("objects", []):
            obj.setdefault("render", {})["visible"] = args.show_teapot

    lights = data.setdefault("lights", [])
    if not lights:
        lights.append({})

    light = lights[0]
    light["id"] = light.get("id", "key-light")
    light["type"] = "directional"
    light["position"] = [args.x, args.y, args.z]
    light["direction"] = args.direction
    light["intensity"] = light.get("intensity", 1.0)

    gizmo = light.setdefault("gizmo", {})
    gizmo["visible"] = True
    gizmo["length"] = args.length
    gizmo["source_character"] = gizmo.get("source_character", "L")
    gizmo["ray_character"] = gizmo.get("ray_character", "-")

    args.scene.write_text(json.dumps(data, indent=2) + "\n")

    # Validate after writing.
    json.loads(args.scene.read_text())

    print(f"scene={args.scene}")
    print(f"light.position={[args.x, args.y, args.z]}")
    print(f"light.direction={args.direction}")
    print(f"light.gizmo.length={args.length}")
    if args.show_teapot:
        print("objects visible")
    if args.hide_teapot:
        print("objects hidden")


if __name__ == "__main__":
    main()
