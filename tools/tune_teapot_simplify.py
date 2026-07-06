#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

DEFAULT_SCENE = Path("assets/a3d/external_teapot/scene.a3d")

def main() -> None:
    parser = argparse.ArgumentParser(description="Tune teapot ASCII mesh simplification.")
    parser.add_argument("grid_size", type=float, help="Simplification grid size, e.g. 0.04, 0.055, 0.08")
    parser.add_argument("--edge-stride", type=int, default=1, help="Optional edge stride after simplification. Default: 1")
    parser.add_argument("--scene", type=Path, default=DEFAULT_SCENE)
    parser.add_argument("--object-id", default="teapot")
    args = parser.parse_args()

    if args.grid_size <= 0:
        raise SystemExit("grid_size must be > 0")
    if args.edge_stride < 1:
        raise SystemExit("--edge-stride must be >= 1")

    data = json.loads(args.scene.read_text())
    target = next((obj for obj in data.get("objects", []) if obj.get("id") == args.object_id), None)
    if target is None:
        raise SystemExit(f"object not found: {args.object_id}")

    render = target.setdefault("render", {})
    render["visible"] = True
    render.setdefault("stroke_character", ".")
    render["ascii_simplify"] = {"enabled": True, "grid_size": args.grid_size}
    render["edge_stride"] = args.edge_stride
    render.pop("max_camera_cells", None)
    render.pop("shrink_edge_details", None)

    args.scene.write_text(json.dumps(data, indent=2) + "\n")
    print(f"grid_size={args.grid_size}")
    print(f"edge_stride={args.edge_stride}")
    print(f"scene={args.scene}")

if __name__ == "__main__":
    main()
