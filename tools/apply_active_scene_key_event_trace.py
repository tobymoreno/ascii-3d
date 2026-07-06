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


def patch_app_state_field(text: str) -> str:
    if "last_input_event_trace:" in text:
        return text
    return text.replace(
        "    frame_timings: FrameTimings,\n}",
        "    frame_timings: FrameTimings,\n    last_input_event_trace: Option<String>,\n}",
        1,
    )


def patch_app_state_new(text: str) -> str:
    if "last_input_event_trace: None" in text:
        return text
    return text.replace(
        "            frame_timings: FrameTimings::default(),\n        }",
        "            frame_timings: FrameTimings::default(),\n            last_input_event_trace: None,\n        }",
        1,
    )


def patch_trace_helpers(text: str) -> str:
    if "fn describe_key_code_for_trace" in text:
        return text
    helpers = '''fn describe_key_code_for_trace(key_code: KeyCode) -> String {
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

fn trace_key_event(state: &mut AppState, route: &str, key_code: KeyCode) {
    state.last_input_event_trace = Some(format!(
        "{route}: key {} | scene {} | mode {} | menu {}",
        describe_key_code_for_trace(key_code),
        state.current_scene().title(),
        state.control_mode.label(),
        state
            .active_menu
            .as_ref()
            .map(|menu| menu.kind().title())
            .unwrap_or("closed"),
    ));
}

fn trace_command_event(state: &mut AppState, route: &str, command: AppCommand) {
    state.last_input_event_trace = Some(format!(
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


def patch_apply_app_command_trace(text: str) -> str:
    marker = "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    body = text[start:end]
    if "trace_command_event(state, \"app dispatch\", command);" not in body:
        body = body.replace(
            "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {\n",
            "fn apply_app_command(state: &mut AppState, command: AppCommand) -> KeyHandling {\n    trace_command_event(state, \"app dispatch\", command);\n",
            1,
        )
    return text[:start] + body + text[end:]


def patch_handle_key_press_trace(text: str) -> str:
    marker = "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {"
    start, end = find_brace_span(text, marker)
    body = text[start:end]
    if "trace_key_event(state, \"raw input\", key_code);" not in body:
        body = body.replace(
            "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {\n",
            "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {\n    trace_key_event(state, \"raw input\", key_code);\n",
            1,
        )
    if "trace_key_event(state, \"active scene key\", key_code);" not in body:
        old = '''    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };'''
        new = '''    trace_key_event(state, "active scene key", key_code);

    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };'''
        if old not in body:
            raise SystemExit("Could not find active scene command dispatch block in handle_key_press")
        body = body.replace(old, new, 1)
    return text[:start] + body + text[end:]


def patch_footer(text: str) -> str:
    if "state.last_input_event_trace.as_deref()" in text:
        return text
    old = '''            "[{}/{}] {} | Mode: {} | Glyph '{}' | Menu: {} | h help | Esc quit",
            state.scene_position + 1,
            Scene::ALL.len(),
            state.current_scene().title(),
            state.control_mode.label(),
            state.glyph_stroke_character(),
            state
                .active_menu
                .as_ref()
                .map(|menu| menu.kind().title())
                .unwrap_or("closed"),
        ),'''
    new = '''            "[{}/{}] {} | Mode: {} | Glyph '{}' | Menu: {} | Event: {} | h help | Esc quit",
            state.scene_position + 1,
            Scene::ALL.len(),
            state.current_scene().title(),
            state.control_mode.label(),
            state.glyph_stroke_character(),
            state
                .active_menu
                .as_ref()
                .map(|menu| menu.kind().title())
                .unwrap_or("closed"),
            state.last_input_event_trace.as_deref().unwrap_or("none"),
        ),'''
    if old not in text:
        raise SystemExit("Could not find footer format block")
    return text.replace(old, new, 1)


def main() -> None:
    text = APP.read_text()
    text = patch_app_state_field(text)
    text = patch_app_state_new(text)
    text = patch_trace_helpers(text)
    text = patch_apply_app_command_trace(text)
    text = patch_handle_key_press_trace(text)
    text = patch_footer(text)
    APP.write_text(text)
    print("Added active-scene key event trace in the footer.")
    print("Menu/app command dispatch is also traced.")


if __name__ == "__main__":
    main()
