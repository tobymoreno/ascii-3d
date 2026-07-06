#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_SCENE = Path("assets/a3d/external_teapot/scene.a3d")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Tune the external teapot A3D render settings without changing camera "
            "projection, transform scale, or perspective."
        )
    )
    parser.add_argument(
        "edge_stride",
        type=int,
        help="Draw every Nth mesh edge. Higher is faster/sparser. Example: 8, 10, 12.",
    )
    parser.add_argument(
        "max_camera_cells",
        type=int,
        help=(
            "Maximum depth-accepted camera viewport cells to draw. "
            "Higher keeps more detail. Example: 400, 500, 700."
        ),
    )
    parser.add_argument(
        "--scene",
        type=Path,
        default=DEFAULT_SCENE,
        help=f"Path to the .a3d scene file. Default: {DEFAULT_SCENE}",
    )
    parser.add_argument(
        "--object-id",
        default="teapot",
        help="Object id to tune. Default: teapot",
    )
    parser.add_argument(
        "--stroke",
        default=".",
        help="Single character used to draw the mesh. Default: .",
    )
    parser.add_argument(
        "--remove-shrink-edge-details",
        action=argparse.BooleanOptionalAction,
        default=True,
        help=(
            "Remove shrink_edge_details so edge_stride is the active edge tuning knob. "
            "Default: true."
        ),
    )
    return parser.parse_args()


def validate(args: argparse.Namespace) -> None:
    if args.edge_stride < 1:
        raise SystemExit("edge_stride must be >= 1")

    if args.max_camera_cells < 1:
        raise SystemExit("max_camera_cells must be >= 1")

    if len(args.stroke) != 1:
        raise SystemExit("--stroke must be exactly one character")


def main() -> None:
    args = parse_args()
    validate(args)

    if not args.scene.exists():
        raise SystemExit(f"Scene file not found: {args.scene}")

    data = json.loads(args.scene.read_text())

    objects = data.get("objects", [])
    target = None

    for obj in objects:
        if obj.get("id") == args.object_id:
            target = obj
            break

    if target is None:
        available = ", ".join(str(obj.get("id")) for obj in objects)
        raise SystemExit(
            f"Object id not found: {args.object_id}. Available object ids: {available}"
        )

    render = target.setdefault("render", {})
    render["visible"] = True
    render["stroke_character"] = args.stroke
    render["edge_stride"] = args.edge_stride
    render["max_camera_cells"] = args.max_camera_cells

    if args.remove_shrink_edge_details:
        render.pop("shrink_edge_details", None)

    args.scene.write_text(json.dumps(data, indent=2) + "\n")

    print(f"Updated {args.scene}")
    print(f"object_id={args.object_id}")
    print(f"edge_stride={args.edge_stride}")
    print(f"max_camera_cells={args.max_camera_cells}")
    print(f"stroke_character={args.stroke!r}")

    if args.remove_shrink_edge_details:
        print("shrink_edge_details removed so edge_stride is active")


if __name__ == "__main__":
    main()
