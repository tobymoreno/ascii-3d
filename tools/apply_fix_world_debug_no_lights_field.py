#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")


def main() -> None:
    text = APP.read_text()

    old = '''            self.push_debug_console_line(format!(
                "world debug: loaded_a3d title='{}' objects={} lights={}",
                world.title,
                world.objects.len(),
                world.lights.len(),
            ));'''

    new = '''            self.push_debug_console_line(format!(
                "world debug: loaded_a3d title='{}' objects={}",
                world.title,
                world.objects.len(),
            ));'''

    if old not in text:
        raise SystemExit("Could not find world debug line that references world.lights.len().")

    text = text.replace(old, new, 1)
    APP.write_text(text)

    print("Removed invalid world.lights debug reference.")


if __name__ == "__main__":
    main()
