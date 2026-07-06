#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")
TUI = Path("src/tui/mod.rs")


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


def patch_app_state_horizontal_scroll(text: str) -> str:
    if "debug_console_horizontal_scroll:" not in text:
        text = text.replace(
            "    debug_console_scroll: usize,\n",
            "    debug_console_scroll: usize,\n    debug_console_horizontal_scroll: usize,\n",
            1,
        )

    if "debug_console_horizontal_scroll: 0" not in text:
        text = text.replace(
            "            debug_console_scroll: 0,\n",
            "            debug_console_scroll: 0,\n            debug_console_horizontal_scroll: 0,\n",
            1,
        )

    return text


def patch_app_debug_methods(text: str) -> str:
    start, end = find_brace_span(text, "impl AppState")
    block = text[start:end]

    if "fn scroll_debug_console_left" not in block:
        marker = '''    fn scroll_debug_console_down(&mut self, amount: usize) {
        self.debug_console_scroll = self.debug_console_scroll.saturating_sub(amount);
    }

'''
        insert = marker + '''    fn scroll_debug_console_left(&mut self, amount: usize) {
        self.debug_console_horizontal_scroll =
            self.debug_console_horizontal_scroll.saturating_sub(amount);
    }

    fn scroll_debug_console_right(&mut self, amount: usize) {
        self.debug_console_horizontal_scroll =
            self.debug_console_horizontal_scroll.saturating_add(amount);
    }

'''
        if marker not in block:
            raise SystemExit("Could not find scroll_debug_console_down method")
        block = block.replace(marker, insert, 1)

    return text[:start] + block + text[end:]


def patch_toggle_debug_console(text: str) -> str:
    marker = "    fn toggle_debug_console(&mut self) {"
    if marker not in text:
        return text

    start, end = find_brace_span(text, marker)
    block = text[start:end]

    if "self.push_world_debug_lines();" not in block:
        block = block.replace(
            "        self.show_debug_console = !self.show_debug_console;\n",
            "        self.show_debug_console = !self.show_debug_console;\n        if self.show_debug_console {\n            self.push_world_debug_lines();\n        }\n",
            1,
        )

    return text[:start] + block + text[end:]


def patch_world_debug_method(text: str) -> str:
    if "fn push_world_debug_lines" in text:
        return text

    marker = "    fn toggle_debug_console(&mut self) {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find toggle_debug_console insertion point")

    method = '''    fn push_world_debug_lines(&mut self) {
        let scene_title = self.current_scene().title().to_string();
        self.push_debug_console_line(format!("world debug: scene={scene_title}"));
        self.push_debug_console_line(format!(
            "world debug: camera pos [{:.2}, {:.2}, {:.2}] yaw {:.1} pitch {:.1}",
            self.world_camera_position.x,
            self.world_camera_position.y,
            self.world_camera_position.z,
            self.world_camera_yaw_degrees,
            self.world_camera_pitch_degrees,
        ));
        self.push_debug_console_line(format!(
            "world debug: control_mode={} menu={}",
            self.control_mode.label(),
            self.active_menu
                .as_ref()
                .map(|menu| menu.kind().title())
                .unwrap_or("closed"),
        ));

        if let Some(world) = self.loaded_a3d_world.as_ref() {
            self.push_debug_console_line(format!(
                "world debug: loaded_a3d title='{}' objects={} lights={}",
                world.title,
                world.objects.len(),
                world.lights.len(),
            ));

            let object_lines = world
                .objects
                .iter()
                .map(|object| {
                    format!(
                        "world debug: object={} pos=[{:.2},{:.2},{:.2}] rot=[{:.1},{:.1},{:.1}] scale=[{:.2},{:.2},{:.2}]",
                        object.id,
                        object.transform.position[0],
                        object.transform.position[1],
                        object.transform.position[2],
                        object.transform.rotation_degrees[0],
                        object.transform.rotation_degrees[1],
                        object.transform.rotation_degrees[2],
                        object.transform.scale[0],
                        object.transform.scale[1],
                        object.transform.scale[2],
                    )
                })
                .collect::<Vec<_>>();

            for line in object_lines {
                self.push_debug_console_line(line);
            }
        } else {
            self.push_debug_console_line("world debug: no loaded_a3d world".to_string());
        }
    }

'''

    return text[:index] + method + text[index:]


def patch_debug_popup_lines(text: str) -> str:
    marker = "fn debug_console_popup_lines(state: &AppState) -> Option<Vec<String>> {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    new_block = '''fn debug_console_popup_lines(state: &AppState) -> Option<Vec<String>> {
    if !state.show_debug_console {
        return None;
    }

    let visible_rows = 24usize;
    let max_scroll = state.debug_console_max_scroll();
    let start = state
        .debug_console_lines
        .len()
        .saturating_sub(visible_rows)
        .saturating_sub(state.debug_console_scroll);
    let end = (start + visible_rows).min(state.debug_console_lines.len());

    let mut lines = vec![
        format!(
            "Debug Console v[{}/{}] h[{}] PageUp/PageDown Left/Right",
            max_scroll.saturating_sub(state.debug_console_scroll),
            max_scroll,
            state.debug_console_horizontal_scroll
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
            .map(|line| {
                line.chars()
                    .skip(state.debug_console_horizontal_scroll)
                    .collect::<String>()
            }),
    );

    Some(lines)
}

'''

    return text[:start] + new_block + text[end:]


def patch_handle_key_press(text: str) -> str:
    marker = "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    block = block.replace('    push_key_debug_trace(state, "raw input", key_code);\n', "")
    block = block.replace('    trace_key_event(state, "raw input", key_code);\n', "")

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
            _ => {}
        }
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
    elif "if state.show_debug_console" not in block:
        # Insert after function opening if missing.
        block = block.replace(
            marker + "\n",
            marker + "\n" + new,
            1,
        )

    block = block.replace('    push_key_debug_trace(state, "active scene key", key_code);\n\n', "")

    return text[:start] + block + text[end:]


def patch_apply_command_spam(text: str) -> str:
    marker = "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    block = block.replace('    push_command_debug_trace(state, "app dispatch", command);\n', "")
    block = block.replace('    trace_command_event(state, "app dispatch", command);\n', "")

    return text[:start] + block + text[end:]


def patch_world_debug_on_reload(text: str) -> str:
    # If reload_a3d exists, add explicit world debug after successful reload where possible.
    if "world debug: reloaded" in text:
        return text

    marker = "    fn reload_a3d(&mut self)"
    if marker not in text:
        return text

    start, end = find_brace_span(text, marker)
    block = text[start:end]

    # Conservative: add line near end before final brace if there is no early certainty.
    block = block[:-1] + '''        if self.show_debug_console {
            self.push_debug_console_line("world debug: reloaded active .a3d world".to_string());
            self.push_world_debug_lines();
        }
'''
    block += "}"

    return text[:start] + block + text[end:]


def patch_tui_popup_size(text: str) -> str:
    text = text.replace(
        "top_right_rect(50, lines.len() as u16 + 6, area)",
        "centered_rect(86, (lines.len() as u16 + 4).min(area.height.saturating_sub(4)), area)",
        1,
    )
    return text


def patch_tui_scrollbars(text: str) -> str:
    if "Scrollbar" not in text:
        text = text.replace(
            "widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},",
            "widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},",
            1,
        )

    marker = "fn draw_debug_popup"
    if marker not in text:
        return text

    start, end = find_brace_span(text, marker)
    block = text[start:end]

    if "ScrollbarOrientation::VerticalRight" in block:
        return text

    # Add lightweight scrollbars after rendering paragraph.
    block = block.replace(
        "    frame.render_widget(paragraph, area);\n",
        '''    frame.render_widget(paragraph, area);

    let vertical_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let mut vertical_state = ScrollbarState::new(lines.len().saturating_sub(1)).position(lines.len().saturating_sub(1));
    frame.render_stateful_widget(vertical_scrollbar, area, &mut vertical_state);

    let horizontal_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom);
    let max_line_width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    let mut horizontal_state = ScrollbarState::new(max_line_width.saturating_sub(1)).position(0);
    frame.render_stateful_widget(horizontal_scrollbar, area, &mut horizontal_state);
''',
        1,
    )

    return text[:start] + block + text[end:]


def main() -> None:
    app = APP.read_text()
    app = patch_app_state_horizontal_scroll(app)
    app = patch_app_debug_methods(app)
    app = patch_world_debug_method(app)
    app = patch_toggle_debug_console(app)
    app = patch_debug_popup_lines(app)
    app = patch_handle_key_press(app)
    app = patch_apply_command_spam(app)
    app = patch_world_debug_on_reload(app)
    APP.write_text(app)

    tui = TUI.read_text()
    tui = patch_tui_popup_size(tui)
    tui = patch_tui_scrollbars(tui)
    TUI.write_text(tui)

    print("Updated debug console popup:")
    print("- explicit debug lines only")
    print("- initial world debug prints")
    print("- larger popup")
    print("- vertical/horizontal scrollbars")
    print("- PageUp/PageDown and Left/Right scrolling while visible")


if __name__ == "__main__":
    main()
