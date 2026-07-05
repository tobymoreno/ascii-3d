#!/usr/bin/env python3
from pathlib import Path

def main() -> None:
    path = Path("src/app.rs")
    text = path.read_text()

    text = text.replace(
        "asset_path(STANDARD_BOX_ASSET).is_file()",
        'asset_path("models/cube.obj").is_file()',
    )

    text = text.replace(
        'load_mesh_asset(STANDARD_BOX_ASSET).expect("models/cube.obj should load")',
        'load_mesh_asset("models/cube.obj").expect("models/cube.obj should load")',
    )

    path.write_text(text)

    print("Fixed standard cube OBJ tests to use literal asset path in test scope.")

if __name__ == "__main__":
    main()
