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


def patch_close_method(text: str) -> str:
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


def patch_handle_key_press(text: str) -> str:
    marker = "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    old_block_start = block.find("    if state.show_debug_console {")
    if old_block_start < 0:
        raise SystemExit("Could not find show_debug_console key block")

    # Find the first full if-block after the marker.
    absolute_if_start = start + old_block_start
    _, absolute_if_end = find_brace_span(text, "    if state.show_debug_console {")

    # The generic find above finds first occurrence in full file, which should be in handle_key_press
    # but verify it is inside this function.
    if not (start <= absolute_if_start < absolute_if_end <= end):
        # fallback: locate braces manually within block
        brace = block.find("{", old_block_start)
        depth = 0
        local_end = None
        for idx in range(brace, len(block)):
            if block[idx] == "{":
                depth += 1
            elif block[idx] == "}":
                depth -= 1
                if depth == 0:
                    local_end = idx + 1
                    break
        if local_end is None:
            raise SystemExit("Could not parse show_debug_console key block")
        old = block[old_block_start:local_end]
        new = '''    if state.show_debug_console {
        match key_code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
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
            _ => {
                return KeyHandling::Handled;
            }
        }
    }'''
        block = block[:old_block_start] + new + block[local_end:]
    else:
        old = text[absolute_if_start:absolute_if_end]
        new = '''    if state.show_debug_console {
        match key_code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
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
            _ => {
                return KeyHandling::Handled;
            }
        }
    }'''
        text = text[:absolute_if_start] + new + text[absolute_if_end:]
        return text

    return text[:start] + block + text[end:]


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
    text = text.replace(
        "    let debug_popup_lines = loaded_a3d_debug_popup_lines(state);\n",
        "    let debug_popup_lines = debug_console_popup_lines(state);\n",
        1,
    )
    return text


def patch_old_popup_handler(text: str) -> str:
    # Since old LoadedA3d popup no longer renders, it also should not consume Enter/Esc.
    marker = "    if is_loaded_a3d_debug_popup_visible(state) {"
    if marker not in text:
        return text

    start = text.find(marker)
    brace = text.find("{", start)
    depth = 0
    end = None
    for idx in range(brace, len(text)):
        if text[idx] == "{":
            depth += 1
        elif text[idx] == "}":
            depth -= 1
            if depth == 0:
                end = idx + 1
                break
    if end is None:
        raise SystemExit("Could not parse old LoadedA3d popup key handler")

    replacement = '''    // The old LoadedA3d auto-hide popup no longer uses the debug popup layer.
'''
    return text[:start] + replacement + text[end:]


def patch_tui_imports(text: str) -> str:
    if "Scrollbar" in text and "ScrollbarState" in text:
        return text

    old = "widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},"
    new = "widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},"
    if old not in text:
        raise SystemExit("Could not find ratatui widgets import line")
    return text.replace(old, new, 1)


def patch_tui_draw_debug_popup(text: str) -> str:
    marker = "fn draw_debug_popup(frame: &mut Frame<'_>, lines: &[String], area: Rect) {"
    start, end = find_brace_span(text, marker)

    new_function = '''fn draw_debug_popup(frame: &mut Frame<'_>, lines: &[String], area: Rect) {
    let popup_lines = lines
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect::<Vec<_>>();

    let popup = Paragraph::new(Text::from(popup_lines)).block(
        Block::default()
            .title("Debug Console")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);

    let vertical_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let mut vertical_state =
        ScrollbarState::new(lines.len().saturating_sub(1)).position(lines.len().saturating_sub(1));
    frame.render_stateful_widget(vertical_scrollbar, area, &mut vertical_state);

    let max_line_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let horizontal_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom);
    let mut horizontal_state = ScrollbarState::new(max_line_width.saturating_sub(1)).position(0);
    frame.render_stateful_widget(horizontal_scrollbar, area, &mut horizontal_state);
}

'''

    return text[:start] + new_function + text[end:]


def patch_tui_popup_size(text: str) -> str:
    old = '''            centered_rect(
                86,
                (lines.len() as u16 + 4).min(area.height.saturating_sub(4)),
                area,
            ),'''
    new = '''            centered_rect(
                area.width.saturating_sub(8).max(40),
                area.height.saturating_sub(6).max(12),
                area,
            ),'''
    if old in text:
        return text.replace(old, new, 1)

    return text


def main() -> None:
    app = APP.read_text()
    app = patch_close_method(app)
    app = patch_handle_key_press(app)
    app = patch_popup_source(app)
    app = patch_old_popup_handler(app)
    APP.write_text(app)

    tui = TUI.read_text()
    tui = patch_tui_imports(tui)
    tui = patch_tui_draw_debug_popup(tui)
    tui = patch_tui_popup_size(tui)
    TUI.write_text(tui)

    print("Fixed debug console close behavior.")
    print("Removed old LoadedA3d popup fallback from debug popup layer.")
    print("Updated debug popup title/help and scrollbars.")


if __name__ == "__main__":
    main()
