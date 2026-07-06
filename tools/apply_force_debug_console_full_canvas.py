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


def patch_canvas_reset_method(text: str) -> str:
    if "pub fn reset_clip_and_origin" in text:
        return text

    marker = "impl Canvas"
    start = text.find(marker)
    if start < 0:
        raise SystemExit("Could not find impl Canvas")

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit("Could not find impl Canvas opening brace")

    insert_at = brace + 1
    method = '''

    pub fn reset_clip_and_origin(&mut self) {
        self.clip_rect = None;
        self.origin_offset = Point2::new(0, 0);
    }
'''
    return text[:insert_at] + method + text[insert_at:]


def remove_debug_calls_outside_main_render(text: str) -> str:
    call = "    draw_debug_console_panel(&mut canvas, state);\n\n"

    main_start, main_end = find_brace_span(text, "fn render_scene_frame")
    before = text[:main_start].replace(call, "")
    main = text[main_start:main_end]
    after = text[main_end:].replace(call, "")

    if call not in main:
        final_return = "    Ok(canvas)\n}"
        if final_return not in main:
            raise SystemExit("Could not find final Ok(canvas) in render_scene_frame")
        main = main.replace(
            final_return,
            call + final_return,
            1,
        )

    return before + main + after


def patch_debug_draw_reset(text: str) -> str:
    marker = "fn draw_debug_console_panel(canvas: &mut Canvas, state: &AppState) {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    if "canvas.reset_clip_and_origin();" not in block:
        block = block.replace(
            marker + "\n",
            marker + "\n    canvas.reset_clip_and_origin();\n",
            1,
        )

    return text[:start] + block + text[end:]


def main() -> None:
    canvas = CANVAS.read_text()
    canvas = patch_canvas_reset_method(canvas)
    CANVAS.write_text(canvas)

    app = APP.read_text()
    app = remove_debug_calls_outside_main_render(app)
    app = patch_debug_draw_reset(app)
    APP.write_text(app)

    print("Forced debug console to draw on unclipped full canvas.")
    print("Removed debug console calls outside render_scene_frame.")


if __name__ == "__main__":
    main()
