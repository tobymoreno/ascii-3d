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


def patch_dismiss_old_popup(text: str) -> str:
    marker = "fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {"
    start, end = find_brace_span(text, marker)

    replacement = '''fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {
    if is_loaded_a3d_debug_popup_visible(state) {
        state.loaded_a3d_debug_popup_until = None;
        true
    } else {
        false
    }
}
'''

    return text[:start] + replacement + text[end:]


def patch_close_debug_console_method(text: str) -> str:
    if "fn close_debug_console" in text:
        return text

    marker = "    fn toggle_debug_console(&mut self) {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find toggle_debug_console method")

    method = '''    fn close_debug_console(&mut self) {
        self.show_debug_console = false;
    }

'''

    return text[:index] + method + text[index:]


def patch_disable_old_popup_key_handler(text: str) -> str:
    marker = "    if is_loaded_a3d_debug_popup_visible(state) {"
    start = text.find(marker)
    if start < 0:
        return text

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit("Could not find old popup handler opening brace")

    depth = 0
    end = None
    for index in range(brace, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                end = index + 1
                break

    if end is None:
        raise SystemExit("Could not parse old popup key handler")

    replacement = '''    // The old LoadedA3d auto-hide popup no longer consumes Enter/Esc.
'''
    return text[:start] + replacement + text[end:]


def patch_popup_source(text: str) -> str:
    text = text.replace(
        "    let debug_popup_lines =\n        debug_console_popup_lines(state).or_else(|| loaded_a3d_debug_popup_lines(state));\n",
        "    let debug_popup_lines = debug_console_popup_lines(state);\n",
        1,
    )
    text = text.replace(
        "    let debug_popup_lines = debug_console_popup_lines(state).or_else(|| loaded_a3d_debug_popup_lines(state));\n",
        "    let debug_popup_lines = debug_console_popup_lines(state);\n",
        1,
    )
    return text


def main() -> None:
    text = APP.read_text()

    text = patch_dismiss_old_popup(text)
    text = patch_close_debug_console_method(text)
    text = patch_disable_old_popup_key_handler(text)
    text = patch_popup_source(text)

    APP.write_text(text)

    print("Fixed dangling old popup else.")
    print("Ensured close_debug_console exists.")
    print("Disabled old LoadedA3d popup key consumption and fallback rendering.")


if __name__ == "__main__":
    main()
