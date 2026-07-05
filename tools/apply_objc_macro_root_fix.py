#!/usr/bin/env python3
from pathlib import Path

MACRO_IMPORT = """#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

"""

def add_macro_import(path: Path) -> None:
    text = path.read_text()

    if "#[macro_use]\nextern crate objc;" in text:
        return

    path.write_text(MACRO_IMPORT + text)

def main() -> None:
    add_macro_import(Path("src/main.rs"))
    add_macro_import(Path("src/bin/os_graphics_demo.rs"))
    add_macro_import(Path("src/bin/os_graphics_overlay_demo.rs"))

    print("Applied objc macro crate-root fix.")

if __name__ == "__main__":
    main()
