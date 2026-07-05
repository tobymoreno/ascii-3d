#!/usr/bin/env python3
from pathlib import Path

def main() -> None:
    path = Path("src/app.rs")
    text = path.read_text()

    text = text.replace(
        'cartesian_axes_metadata.geometry_asset != "cartesian_axes.obj"',
        'cartesian_axes_metadata.geometry_asset != "models/cartesian_axes.obj"',
    )

    text = text.replace(
        'cartesian_axes.json references unexpected geometry asset \'{}\'',
        'cartesian_axes.json references unexpected geometry asset \'{}\'',
    )

    path.write_text(text)

    print("Fixed cartesian axes metadata validation for models/cartesian_axes.obj.")

if __name__ == "__main__":
    main()
