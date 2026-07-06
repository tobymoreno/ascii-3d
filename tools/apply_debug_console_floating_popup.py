#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")
COMMAND = Path("src/input/command.rs")
MENU = Path("src/menu/model.rs")
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


def patch_command_enum(text: str) -> str:
    if "ToggleDebugConsole" in text:
        return text

    return text.replace(
        "    ToggleFrameTiming,\n    ShowOsGraphicsOverlay,",
        "    ToggleFrameTiming,\n    ToggleDebugConsole,\n    ShowOsGraphicsOverlay,",
        1,
    )


def patch_debug_menu_item(text: str) -> str:
    if "Toggle debug console" in text:
        return text

    return text.replace(
        "const DEBUG_ITEMS: &[MenuItem] = &[\n",
        "const DEBUG_ITEMS: &[MenuItem] = &[\n    MenuItem::real(\"Toggle debug console\", AppCommand::ToggleDebugConsole),\n",
        1,
    )


def patch_app_state_field(text: str) -> str:
    if "show_debug_console:" in text:
        return text

    return text.replace(
        "    show_frame_timing: bool,\n",
        "    show_frame_timing: bool,\n    show_debug_console: bool,\n",
        1,
    )


def patch_app_state_init(text: str) -> str:
    if "show_debug_console: false" in text:
        return text

    return text.replace(
        "            show_frame_timing: false,\n",
        "            show_frame_timing: false,\n            show_debug_console: false,\n",
        1,
    )


def patch_app_methods(text: str) -> str:
    if "fn toggle_debug_console" not in text:
        marker = "    fn push_debug_console_line(&mut self, message: impl Into<String>) {"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find debug console method insertion point")

        method = '''    fn toggle_debug_console(&mut self) {
        self.show_debug_console = !self.show_debug_console;
        self.push_debug_console_line(format!(
            "debug console: {}",
            if self.show_debug_console { "shown" } else { "hidden" }
        ));
    }

'''
        text = text[:index] + method + text[index:]

    if "fn debug_console_popup_lines" not in text:
        marker = "fn loaded_a3d_debug_popup_lines(state: &AppState) -> Option<Vec<String>> {"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find loaded_a3d_debug_popup_lines insertion point")

        function = '''fn debug_console_popup_lines(state: &AppState) -> Option<Vec<String>> {
    if !state.show_debug_console {
        return None;
    }

    let visible_rows = 14usize;
    let max_scroll = state.debug_console_max_scroll();
    let start = state
        .debug_console_lines
        .len()
        .saturating_sub(visible_rows)
        .saturating_sub(state.debug_console_scroll);
    let end = (start + visible_rows).min(state.debug_console_lines.len());

    let mut lines = vec![
        format!(
            "Debug Console [{}/{}] PageUp/PageDown scroll",
            max_scroll.saturating_sub(state.debug_console_scroll),
            max_scroll
        ),
        "Debug menu -> Toggle debug console hides this popup".to_string(),
        String::new(),
    ];

    lines.extend(
        state
            .debug_console_lines
            .iter()
            .skip(start)
            .take(end - start)
            .cloned(),
    );

    Some(lines)
}

'''
        text = text[:index] + function + text[index:]

    return text


def remove_canvas_debug_panel_call(text: str) -> str:
    return text.replace("    draw_debug_console_panel(&mut canvas, state);\n\n", "")


def remove_canvas_debug_panel_function(text: str) -> str:
    marker = "fn draw_debug_console_panel(canvas: &mut Canvas, state: &AppState) {"
    if marker not in text:
        return text

    start, end = find_brace_span(text, marker)
    return text[:start] + text[end:]


def patch_render_scene_popup_source(text: str) -> str:
    if "debug_console_popup_lines(state).or_else" in text:
        return text

    return text.replace(
        "    let debug_popup_lines = loaded_a3d_debug_popup_lines(state);\n",
        "    let debug_popup_lines = debug_console_popup_lines(state).or_else(|| loaded_a3d_debug_popup_lines(state));\n",
        1,
    )


def patch_apply_command(text: str) -> str:
    if "AppCommand::ToggleDebugConsole" in text and "state.toggle_debug_console();" in text:
        return text

    marker = "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    insert = '''        AppCommand::ToggleDebugConsole => {
            state.toggle_debug_console();
            KeyHandling::Handled
        }

'''

    if "AppCommand::ToggleFrameTiming" in block:
        block = block.replace(
            "        AppCommand::ToggleFrameTiming => {\n",
            insert + "        AppCommand::ToggleFrameTiming => {\n",
            1,
        )
    elif "AppCommand::ShowOsGraphicsOverlay" in block:
        block = block.replace(
            "        AppCommand::ShowOsGraphicsOverlay => {\n",
            insert + "        AppCommand::ShowOsGraphicsOverlay => {\n",
            1,
        )
    else:
        raise SystemExit("Could not find ToggleFrameTiming or ShowOsGraphicsOverlay branch")

    return text[:start] + block + text[end:]


def patch_page_scroll(text: str) -> str:
    marker = "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    old = '''    match key_code {
        KeyCode::PageUp => {
            state.scroll_debug_console_up(6);
            state.push_debug_console_line("debug console: scroll up".to_string());
            return KeyHandling::Handled;
        }
        KeyCode::PageDown => {
            state.scroll_debug_console_down(6);
            state.push_debug_console_line("debug console: scroll down".to_string());
            return KeyHandling::Handled;
        }
        _ => {}
    }

'''

    new = '''    if state.show_debug_console {
        match key_code {
            KeyCode::PageUp => {
                state.scroll_debug_console_up(6);
                return KeyHandling::Handled;
            }
            KeyCode::PageDown => {
                state.scroll_debug_console_down(6);
                return KeyHandling::Handled;
            }
            _ => {}
        }
    }

'''

    if old in block:
        block = block.replace(old, new, 1)

    return text[:start] + block + text[end:]


def patch_canvas_cleanup(text: str) -> str:
    method = '''    pub fn reset_clip_and_origin(&mut self) {
        self.clip_rect = None;
        self.origin_offset = Point2::new(0, 0);
    }

'''
    return text.replace(method, "")


def main() -> None:
    command = COMMAND.read_text()
    command = patch_command_enum(command)
    COMMAND.write_text(command)

    menu = MENU.read_text()
    menu = patch_debug_menu_item(menu)
    MENU.write_text(menu)

    app = APP.read_text()
    app = patch_app_state_field(app)
    app = patch_app_state_init(app)
    app = patch_app_methods(app)
    app = remove_canvas_debug_panel_call(app)
    app = remove_canvas_debug_panel_function(app)
    app = patch_render_scene_popup_source(app)
    app = patch_apply_command(app)
    app = patch_page_scroll(app)
    APP.write_text(app)

    canvas = CANVAS.read_text()
    canvas = patch_canvas_cleanup(canvas)
    CANVAS.write_text(canvas)

    print("Switched debug console from canvas-bottom panel to Ratatui floating popup.")
    print("Use Debug menu -> Toggle debug console.")


if __name__ == "__main__":
    main()
