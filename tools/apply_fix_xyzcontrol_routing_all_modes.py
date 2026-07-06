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


def patch_xyz_pending_returns(text: str) -> str:
    text = text.replace(
        '''            ControlMode::Camera => {
                self.push_debug_console_line(format!(
                    "xyzcontrol/camera: {} binding pending",
                    event.label()
                ));
                false
            }
''',
        '''            ControlMode::Camera => {
                self.push_debug_console_line(format!(
                    "xyzcontrol/camera: {} binding pending",
                    event.label()
                ));
                true
            }
''',
        1,
    )

    text = text.replace(
        '''            ControlMode::Light => {
                self.push_debug_console_line(format!(
                    "xyzcontrol/light: {} binding pending",
                    event.label()
                ));
                false
            }
''',
        '''            ControlMode::Light => {
                self.push_debug_console_line(format!(
                    "xyzcontrol/light: {} binding pending",
                    event.label()
                ));
                true
            }
''',
        1,
    )

    return text


def patch_set_control_mode_trace(text: str) -> str:
    old = '''    fn set_control_mode(&mut self, control_mode: ControlMode) {
        self.control_mode = control_mode;
    }
'''
    new = '''    fn set_control_mode(&mut self, control_mode: ControlMode) {
        self.control_mode = control_mode;
        self.push_debug_console_line(format!("control mode: {}", self.control_mode.label()));
    }
'''
    if old in text:
        text = text.replace(old, new, 1)
    return text


def patch_toggle_control_mode_trace(text: str) -> str:
    old = '''    fn toggle_control_mode(&mut self) {
        self.control_mode = match self.control_mode {
            ControlMode::Scene => ControlMode::Camera,
            ControlMode::Camera => ControlMode::Light,
            ControlMode::Light => ControlMode::Scene,
        };
    }
'''
    new = '''    fn toggle_control_mode(&mut self) {
        self.control_mode = match self.control_mode {
            ControlMode::Scene => ControlMode::Camera,
            ControlMode::Camera => ControlMode::Light,
            ControlMode::Light => ControlMode::Scene,
        };

        self.push_debug_console_line(format!("control mode: {}", self.control_mode.label()));
    }
'''
    if old in text:
        text = text.replace(old, new, 1)
    return text


def patch_handle_key_press(text: str) -> str:
    start, end = find_brace_span(text, "fn handle_key_press")
    replacement = '''fn handle_key_press(state: &mut AppState, key: KeyEvent) -> KeyHandling {
    let key_code = key.code;

    if state.a3d_file_picker.is_some() {
        match key_code {
            KeyCode::Esc => {
                state.close_a3d_file_picker();
                return KeyHandling::Handled;
            }
            KeyCode::Up => {
                state.move_a3d_file_picker_up();
                return KeyHandling::Handled;
            }
            KeyCode::Down => {
                state.move_a3d_file_picker_down();
                return KeyHandling::Handled;
            }
            KeyCode::Backspace => {
                state.a3d_file_picker_parent();
                return KeyHandling::Handled;
            }
            KeyCode::Enter => {
                state.select_a3d_file_picker_entry();
                return KeyHandling::Handled;
            }
            _ => return KeyHandling::Ignored,
        }
    }

    // Menus are modal and must keep priority over the floating debug console.
    if state.active_menu.is_some() {
        return menu_command_for_key(key_code)
            .map(|command| apply_app_command(state, command))
            .unwrap_or(KeyHandling::Ignored);
    }

    // XyzControl is the primitive axis/origin input layer. It routes through
    // the currently active control target inside apply_xyz_control_event().
    if let Some(event) = state.xyz_control.event_for_key(key) {
        return apply_app_command(state, AppCommand::XyzControl(event));
    }

    if state.show_debug_console {
        match key_code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
                state.close_debug_console();
                return KeyHandling::Handled;
            }
            KeyCode::Tab => {
                return apply_app_command(state, AppCommand::ToggleControlMode);
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                return apply_app_command(state, AppCommand::OpenMenu(crate::menu::MenuKind::Control));
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
    }

    if is_loaded_a3d_debug_popup_visible(state) {
        match key_code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('O') => {
                dismiss_loaded_a3d_debug_popup(state);
                return KeyHandling::Handled;
            }
            _ => {}
        }
    }

    trace_key_event(state, "active scene key", key_code);

    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };

    command
        .map(|command| apply_app_command(state, command))
        .unwrap_or(KeyHandling::Ignored)
}
'''
    return text[:start] + replacement + text[end:]


def main() -> None:
    text = APP.read_text()
    text = patch_xyz_pending_returns(text)
    text = patch_set_control_mode_trace(text)
    text = patch_toggle_control_mode_trace(text)
    text = patch_handle_key_press(text)
    APP.write_text(text)

    print("Fixed XyzControl routing for all modes.")
    print("Active menu now has priority over the debug console.")
    print("Debug console now allows Tab and Control menu while open.")
    print("Camera/Light pending XyzControl logs now render.")


if __name__ == "__main__":
    main()
