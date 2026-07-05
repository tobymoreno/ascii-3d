#!/usr/bin/env python3
from pathlib import Path

def main() -> None:
    path = Path("src/obj.rs")
    text = path.read_text()

    text = text.replace(
        '.join("cartesian_axes.obj")',
        '.join("models").join("cartesian_axes.obj")',
    )

    path.write_text(text)

    print("Fixed obj.rs cartesian axes asset test path.")

if __name__ == "__main__":
    main()
