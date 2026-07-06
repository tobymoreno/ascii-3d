#!/usr/bin/env python3
from pathlib import Path

TUI = Path("src/tui/mod.rs")


def main() -> None:
    text = TUI.read_text()

    old = """    MenuKind::Camera,
    MenuKind::World,"""

    new = """    MenuKind::Control,"""

    if old not in text:
        raise SystemExit("Could not find old Camera/World menu entries in src/tui/mod.rs")

    text = text.replace(old, new, 1)
    TUI.write_text(text)

    print("Updated top menu bar from Camera/World entries to Control.")


if __name__ == "__main__":
    main()
