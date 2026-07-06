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


def patch_debug_console_close_method(text: str) -> str:
    if "fn close_debug_console" in text:
        return text

    marker = "    fn toggle_debug_console(&mut self) {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find toggle_debug_console method")

    method = '''    fn close_debug_console(&mut self) {
        if self.show_debug_console {
            self.show_debug_console = false;
        }
    }

'''

    return text[:index] + method + text[index:]


def patch_handle_key_press(text: str) -> str:
    marker = "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    old = '''    if state.show_debug_console {
        match key_code {
            KeyCode::PageUp => {
                state.scroll_debug_console_up(6);
                return KeyHandling::Handled;
            }
            KeyCode::PageDown => {
                state.scroll_debug_console_down(6);
                return KeyHandling::Handled;
            }
            KeyCode::Left => {
                state.scroll_debug_console_left(8);
                return KeyHandling::Handled;
            }
            KeyCode::Right => {
                state.scroll_debug_console_right(8);
                return KeyHandling::Handled;
            }
            _ => {}
        }
    }

'''

    new = '''    if state.show_debug_console {
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {
                state.close_debug_console();
                return KeyHandling::Handled;
            }
            KeyCode::PageUp => {
                state.scroll_debug_console_up(6);
                return KeyHandling::Handled;
            }
            KeyCode::PageDown => {
                state.scroll_debug_console_down(6);
                return KeyHandling::Handled;
            }
            KeyCode::Left => {
                state.scroll_debug_console_left(8);
                return KeyHandling::Handled;
            }
            KeyCode::Right => {
                state.scroll_debug_console_right(8);
                return KeyHandling::Handled;
            }
            _ => {}
        }
    }

'''

    if old in block:
        block = block.replace(old, new, 1)
    elif "KeyCode::Esc | KeyCode::Enter" not in block:
        raise SystemExit("Could not find debug console key handling block to patch")

    # Old loaded-a3d auto-hide popup should not steal Enter/Esc while the new console is open.
    block = block.replace(
        "    if is_loaded_a3d_debug_popup_visible(state) {",
        "    if !state.show_debug_console && is_loaded_a3d_debug_popup_visible(state) {",
        1,
    )

    return text[:start] + block + text[end:]


def patch_popup_source(text: str) -> str:
    old = "    let debug_popup_lines = debug_console_popup_lines(state).or_else(|| loaded_a3d_debug_popup_lines(state));\n"
    new = "    let debug_popup_lines = debug_console_popup_lines(state);\n"

    if old in text:
        return text.replace(old, new, 1)

    # Fallback for earlier version.
    old2 = "    let debug_popup_lines = loaded_a3d_debug_popup_lines(state);\n"
    if old2 in text:
        return text.replace(old2, new, 1)

    if "let debug_popup_lines = debug_console_popup_lines(state);" in text:
        return text

    raise SystemExit("Could not find debug_popup_lines assignment")


def main() -> None:
    text = APP.read_text()

    text = patch_debug_console_close_method(text)
    text = patch_handle_key_press(text)
    text = patch_popup_source(text)

    APP.write_text(text)

    print("Debug console now closes with Esc/Enter.")
    print("Old loaded-a3d auto-hide popup no longer renders as the debug popup source.")


if __name__ == "__main__":
    main()
