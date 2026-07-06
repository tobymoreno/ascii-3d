#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")


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


def remove_misplaced_camera_viewport_call(text: str) -> str:
    marker = "fn render_loaded_a3d_camera_viewport_canvas"
    if marker not in text:
        return text

    start, end = find_brace_span(text, marker)
    block = text[start:end]

    block = block.replace(
        "\n    draw_debug_console_panel(&mut canvas, state);\n",
        "\n",
    )

    return text[:start] + block + text[end:]


def add_main_render_call(text: str) -> str:
    marker = "fn render_scene_frame"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    if "draw_debug_console_panel(&mut canvas, state);" in block:
        return text

    final_return = "    Ok(canvas)\n}"
    if final_return not in block:
        raise SystemExit("Could not find final Ok(canvas) in render_scene_frame")

    block = block.replace(
        final_return,
        "    draw_debug_console_panel(&mut canvas, state);\n\n" + final_return,
        1,
    )

    return text[:start] + block + text[end:]


def main() -> None:
    text = APP.read_text()

    text = remove_misplaced_camera_viewport_call(text)
    text = add_main_render_call(text)

    APP.write_text(text)

    print("Moved debug console panel draw call to render_scene_frame main canvas.")
    print("Removed misplaced draw call from render_loaded_a3d_camera_viewport_canvas.")


if __name__ == "__main__":
    main()
