#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")
CANVAS = Path("src/canvas.rs")


def find_brace_span(text: str, marker: str) -> tuple[int, int]:
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"Could not find marker: {marker}")

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit(f"Could not find opening brace after: {marker}")

    depth = 0
    for index in range(brace, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return start, index + 1

    raise SystemExit(f"Could not find closing brace for: {marker}")


def find_inner_brace_span(text: str, open_brace_index: int) -> tuple[int, int]:
    depth = 0
    for index in range(open_brace_index, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return open_brace_index, index + 1

    raise SystemExit("Could not find matching brace")


def patch_app_imports(text: str) -> str:
    if "VecDeque" in text:
        return text

    if "collections::HashMap" in text:
        return text.replace(
            "collections::HashMap",
            "collections::{HashMap, VecDeque}",
            1,
        )

    if "use std::{" in text:
        return text.replace("use std::{", "use std::{\n    collections::VecDeque,", 1)

    raise SystemExit("Could not patch std imports with VecDeque")


def patch_app_state_struct(text: str) -> str:
    if "debug_console_lines:" in text:
        return text

    start, end = find_brace_span(text, "struct AppState")
    block = text[start:end]

    insert = '''    debug_console_lines: VecDeque<String>,
    debug_console_scroll: usize,
'''

    block = block[:-1] + insert + "}"
    return text[:start] + block + text[end:]


def patch_app_state_new(text: str) -> str:
    if "debug_console_lines: VecDeque::from" in text:
        return text

    new_start = text.find("    fn new() -> Self")
    if new_start < 0:
        raise SystemExit("Could not find AppState::new")

    self_start = text.find("Self {", new_start)
    if self_start < 0:
        raise SystemExit("Could not find Self initializer in AppState::new")

    brace = text.find("{", self_start)
    _, self_end = find_inner_brace_span(text, brace)

    block = text[self_start:self_end]
    insert = '''            debug_console_lines: VecDeque::from([
                "debug console attached to main workspace".to_string(),
                "keys/menu/scene routing will be logged here".to_string(),
                "PageUp/PageDown scroll this debug console".to_string(),
            ]),
            debug_console_scroll: 0,
'''

    block = block[:-1] + insert + "}"
    return text[:self_start] + block + text[self_end:]


def patch_canvas_methods(text: str) -> str:
    if "fn width(&self)" in text and "fn height(&self)" in text:
        return text

    impl_marker = "impl Canvas"
    start = text.find(impl_marker)
    if start < 0:
        raise SystemExit("Could not find impl Canvas in src/canvas.rs")

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit("Could not find impl Canvas opening brace")

    insert_at = brace + 1

    methods = '''

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }
'''

    return text[:insert_at] + methods + text[insert_at:]


def main() -> None:
    app = APP.read_text()
    app = patch_app_imports(app)
    app = patch_app_state_struct(app)
    app = patch_app_state_new(app)
    APP.write_text(app)

    canvas = CANVAS.read_text()
    canvas = patch_canvas_methods(canvas)
    CANVAS.write_text(canvas)

    print("Repaired main debug console fields and Canvas width/height accessors.")


if __name__ == "__main__":
    main()
