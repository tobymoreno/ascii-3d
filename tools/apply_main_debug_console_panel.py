
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


def patch_imports(text: str) -> str:
    if "VecDeque" in text:
        return text

    return text.replace(
        "collections::HashMap",
        "collections::{HashMap, VecDeque}",
        1,
    )


def patch_constants(text: str) -> str:
    if "DEBUG_CONSOLE_HEIGHT" in text:
        return text

    marker = "const FOOTER_ROW:"
    index = text.find(marker)
    if index < 0:
        index = text.find("fn pulsed_rotation_delta_degrees")
        if index < 0:
            raise SystemExit("Could not find constant insertion point")

    consts = '''const DEBUG_CONSOLE_HEIGHT: i32 = 9;
const DEBUG_CONSOLE_MAX_LINES: usize = 500;

'''
    return text[:index] + consts + text[index:]


def patch_app_state(text: str) -> str:
    if "debug_console_lines:" in text:
        return text

    text = text.replace(
        "    frame_timings: FrameTimings,\n}",
        "    frame_timings: FrameTimings,\n    debug_console_lines: VecDeque<String>,\n    debug_console_scroll: usize,\n}",
        1,
    )

    text = text.replace(
        "            frame_timings: FrameTimings::default(),\n        }",
        '''            frame_timings: FrameTimings::default(),
            debug_console_lines: VecDeque::from([
                "debug console attached to main workspace".to_string(),
                "keys/menu/scene routing will be logged here".to_string(),
                "PageUp/PageDown scroll this debug console".to_string(),
            ]),
            debug_console_scroll: 0,
        }''',
        1,
    )

    return text


def patch_app_methods(text: str) -> str:
    if "fn push_debug_console_line" in text:
        return text

    methods = '''    fn push_debug_console_line(&mut self, message: impl Into<String>) {
        self.debug_console_lines.push_back(message.into());

        while self.debug_console_lines.len() > DEBUG_CONSOLE_MAX_LINES {
            self.debug_console_lines.pop_front();
        }

        self.debug_console_scroll = 0;
    }

    fn debug_console_visible_rows(&self) -> usize {
        DEBUG_CONSOLE_HEIGHT.saturating_sub(3) as usize
    }

    fn debug_console_max_scroll(&self) -> usize {
        self.debug_console_lines
            .len()
            .saturating_sub(self.debug_console_visible_rows())
    }

    fn scroll_debug_console_up(&mut self, amount: usize) {
        self.debug_console_scroll =
            (self.debug_console_scroll + amount).min(self.debug_console_max_scroll());
    }

    fn scroll_debug_console_down(&mut self, amount: usize) {
        self.debug_console_scroll = self.debug_console_scroll.saturating_sub(amount);
    }

'''

    marker = "    fn current_scene(&self) -> Scene {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find AppState method insertion point")

    return text[:index] + methods + text[index:]


def patch_trace_helpers(text: str) -> str:
    if "fn describe_key_code_for_debug_console" in text:
        return text

    helpers = '''fn describe_key_code_for_debug_console(key_code: KeyCode) -> String {
    match key_code {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Char(character) => format!("'{character}'"),
        KeyCode::F(number) => format!("F{number}"),
        other => format!("{other:?}"),
    }
}

fn push_key_debug_trace(state: &mut AppState, route: &str, key_code: KeyCode) {
    state.push_debug_console_line(format!(
        "{route}: key {} | scene {} | mode {} | menu {}",
        describe_key_code_for_debug_console(key_code),
        state.current_scene().title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

fn push_command_debug_trace(state: &mut AppState, route: &str, command: AppCommand) {
    state.push_debug_console_line(format!(
        "{route}: command {command:?} | scene {} | mode {} | menu {}",
        state.current_scene().title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

'''

    marker = "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find apply_app_command insertion point")

    return text[:index] + helpers + text[index:]


def patch_apply_command(text: str) -> str:
    marker = "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    body = text[start:end]

    if "push_command_debug_trace(state, \"app dispatch\", command);" not in body:
        body = body.replace(
            "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {\n",
            "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {\n    push_command_debug_trace(state, \"app dispatch\", command);\n",
            1,
        )

    return text[:start] + body + text[end:]


def patch_handle_key(text: str) -> str:
    marker = "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    body = text[start:end]

    if "push_key_debug_trace(state, \"raw input\", key_code);" not in body:
        body = body.replace(
            "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {\n",
            "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {\n    push_key_debug_trace(state, \"raw input\", key_code);\n",
            1,
        )

    if "scroll_debug_console_up(6)" not in body:
        insertion = '''    match key_code {
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
        body = body.replace(
            "    push_key_debug_trace(state, \"raw input\", key_code);\n",
            "    push_key_debug_trace(state, \"raw input\", key_code);\n" + insertion,
            1,
        )

    if "push_key_debug_trace(state, \"active scene key\", key_code);" not in body:
        old = '''    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };'''

        new = '''    push_key_debug_trace(state, "active scene key", key_code);

    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };'''

        if old not in body:
            raise SystemExit("Could not find active scene command dispatch block in handle_key_press")

        body = body.replace(old, new, 1)

    return text[:start] + body + text[end:]


def patch_draw_console_function(text: str) -> str:
    if "fn draw_debug_console_panel" in text:
        return text

    function = '''fn draw_debug_console_panel(canvas: &mut Canvas, state: &AppState) {
    let height = DEBUG_CONSOLE_HEIGHT;
    let left = 0;
    let top = canvas.height() as i32 - height;
    let right = canvas.width() as i32 - 1;
    let bottom = canvas.height() as i32 - 1;

    if top <= 0 || right <= left {
        return;
    }

    canvas.draw_line(Point2::new(left, top), Point2::new(right, top), '=');
    canvas.draw_line(Point2::new(left, bottom), Point2::new(right, bottom), '=');
    canvas.draw_line(Point2::new(left, top), Point2::new(left, bottom), '|');
    canvas.draw_line(Point2::new(right, top), Point2::new(right, bottom), '|');

    canvas.set(Point2::new(left, top), '+');
    canvas.set(Point2::new(right, top), '+');
    canvas.set(Point2::new(left, bottom), '+');
    canvas.set(Point2::new(right, bottom), '+');

    let visible_rows = height.saturating_sub(3) as usize;
    let max_scroll = state.debug_console_max_scroll();
    let start = state
        .debug_console_lines
        .len()
        .saturating_sub(visible_rows)
        .saturating_sub(state.debug_console_scroll);
    let end = (start + visible_rows).min(state.debug_console_lines.len());

    canvas.draw_text(
        Point2::new(left + 2, top),
        &format!(
            " Debug Console [{}/{}] PageUp/PageDown ",
            max_scroll.saturating_sub(state.debug_console_scroll),
            max_scroll
        ),
    );

    for (line_index, line) in state
        .debug_console_lines
        .iter()
        .skip(start)
        .take(end - start)
        .enumerate()
    {
        let mut display = line.clone();
        let max_width = (right - left - 4).max(0) as usize;

        if display.chars().count() > max_width {
            display = display.chars().take(max_width.saturating_sub(1)).collect::<String>();
            display.push('…');
        }

        canvas.draw_text(
            Point2::new(left + 2, top + 2 + line_index as i32),
            &display,
        );
    }
}

'''

    marker = "fn is_loaded_a3d_debug_popup_visible"
    index = text.find(marker)
    if index < 0:
        index = text.find("#[cfg(test)]")
        if index < 0:
            raise SystemExit("Could not find draw_debug_console_panel insertion point")

    return text[:index] + function + text[index:]


def patch_render_call(text: str) -> str:
    if "draw_debug_console_panel(&mut canvas, state);" in text:
        return text

    ok = text.find("    Ok(canvas)")
    if ok < 0:
        raise SystemExit("Could not find Ok(canvas) insertion point")

    return text[:ok] + "    draw_debug_console_panel(&mut canvas, state);\n\n" + text[ok:]


def main() -> None:
    text = APP.read_text()

    text = patch_imports(text)
    text = patch_constants(text)
    text = patch_app_state(text)
    text = patch_app_methods(text)
    text = patch_trace_helpers(text)
    text = patch_apply_command(text)
    text = patch_handle_key(text)
    text = patch_draw_console_function(text)
    text = patch_render_call(text)

    APP.write_text(text)

    print("Added main workspace debug console panel.")
    print("Logs raw input, app/menu dispatch, and active scene key routing.")
    print("Use PageUp/PageDown to scroll the debug console.")


if __name__ == "__main__":
    main()
