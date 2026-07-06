#!/usr/bin/env python3
from pathlib import Path
import re

APP = Path("src/app.rs")
COMMAND = Path("src/input/command.rs")
MAIN = Path("src/main.rs")


def find_brace_span(text: str, marker: str) -> tuple[int, int]:
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"Could not find marker: {marker}")
    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit(f"Could not find opening brace after: {marker}")
    depth = 0
    for index in range(brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return start, index + 1
    raise SystemExit(f"Could not find closing brace for: {marker}")


def patch_main_mod(text: str) -> str:
    if re.search(r"(?m)^mod xyz_control;", text):
        return text
    lines = text.splitlines()
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("mod "):
            insert_at = i + 1
    lines.insert(insert_at, "mod xyz_control;")
    return "\n".join(lines) + "\n"


def patch_command(text: str) -> str:
    if "use crate::xyz_control::XyzControlEvent;" not in text:
        text = text.replace(
            "use crate::menu::MenuKind;\n",
            "use crate::menu::MenuKind;\nuse crate::xyz_control::XyzControlEvent;\n",
            1,
        )

    if "XyzControl(XyzControlEvent)" not in text:
        text = text.replace(
            "    ResetActiveControl,\n",
            "    ResetActiveControl,\n    XyzControl(XyzControlEvent),\n",
            1,
        )

    if "pub fn scene_mode_command_for_key" in text:
        start, end = find_brace_span(text, "pub fn scene_mode_command_for_key")
        block = text[start:end]
        for line in [
            "        KeyCode::Char('x') => Some(AppCommand::RotateWorldPositiveX),\n",
            "        KeyCode::Char('X') => Some(AppCommand::RotateWorldNegativeX),\n",
            "        KeyCode::Char('y') => Some(AppCommand::RotateWorldPositiveY),\n",
            "        KeyCode::Char('Y') => Some(AppCommand::RotateWorldNegativeY),\n",
            "        KeyCode::Char('z') => Some(AppCommand::RotateWorldPositiveZ),\n",
            "        KeyCode::Char('Z') => Some(AppCommand::RotateWorldNegativeZ),\n",
            "        KeyCode::Char('z') => Some(AppCommand::DebugRotateLoadedA3dObjectZPositive),\n",
            "        KeyCode::Char('Z') => Some(AppCommand::DebugRotateLoadedA3dObjectZNegative),\n",
        ]:
            block = block.replace(line, "")
        text = text[:start] + block + text[end:]

    return text


def patch_app_imports(text: str) -> str:
    if "xyz_control::{XyzControl" not in text:
        text = text.replace(
            "use crate::{\n",
            "use crate::{\n    xyz_control::{XyzControl, XyzControlEvent},\n",
            1,
        )
    return text


def patch_app_state(text: str) -> str:
    if "xyz_control:" not in text:
        text = text.replace(
            "    control_mode: ControlMode,\n",
            "    control_mode: ControlMode,\n    xyz_control: XyzControl,\n",
            1,
        )
    if "xyz_control: XyzControl::default()" not in text:
        text = text.replace(
            "            control_mode: ControlMode::Scene,\n",
            "            control_mode: ControlMode::Scene,\n            xyz_control: XyzControl::default(),\n",
            1,
        )
    return text


def patch_apply_xyz_method(text: str) -> str:
    if "fn apply_xyz_control_event" in text:
        return text

    marker = "    fn reset_active_control(&mut self) -> bool {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find reset_active_control insertion point")

    method = '''    fn apply_xyz_control_event(&mut self, event: XyzControlEvent) -> bool {
        match self.control_mode {
            ControlMode::Scene => match event {
                XyzControlEvent::Rotate { axis, direction } => {
                    let delta = self.xyz_control.rotation_delta(axis, direction);
                    let handled = self.rotate_loaded_a3d_world_object(delta);
                    self.push_debug_console_line(format!(
                        "xyzcontrol/world: {} handled={handled}",
                        event.label()
                    ));
                    handled
                }
                XyzControlEvent::MoveOrigin { axis, direction } => {
                    let delta = self.xyz_control.origin_delta(axis, direction);
                    self.move_world_origin(delta);
                    self.push_debug_console_line(format!("xyzcontrol/world: {}", event.label()));
                    true
                }
                XyzControlEvent::Reset => self.reset_world_axes(),
            },
            ControlMode::Camera => {
                self.push_debug_console_line(format!(
                    "xyzcontrol/camera: {} binding pending",
                    event.label()
                ));
                false
            }
            ControlMode::Light => {
                self.push_debug_console_line(format!(
                    "xyzcontrol/light: {} binding pending",
                    event.label()
                ));
                false
            }
        }
    }

'''
    return text[:index] + method + text[index:]


def patch_apply_command(text: str) -> str:
    start, end = find_brace_span(text, "fn apply_app_command")
    block = text[start:end]
    if "AppCommand::XyzControl(event)" not in block:
        block = block.replace(
            "        AppCommand::Quit => KeyHandling::Quit,\n",
            '''        AppCommand::Quit => KeyHandling::Quit,

        AppCommand::XyzControl(event) => {
            if state.apply_xyz_control_event(event) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }
''',
            1,
        )
    return text[:start] + block + text[end:]


def remove_old_ctrl_block(text: str) -> str:
    marker = "    if state.control_mode == ControlMode::Scene && key.modifiers.contains(KeyModifiers::CONTROL) {"
    start = text.find(marker)
    if start < 0:
        return text
    brace = text.find("{", start)
    depth = 0
    end = None
    for i in range(brace, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                break
    if end is None:
        raise SystemExit("Could not parse old Ctrl+arrow block")
    while end < len(text) and text[end] == "\n":
        end += 1
        break
    return text[:start] + text[end:]


def patch_handle_key_press(text: str) -> str:
    start, end = find_brace_span(text, "fn handle_key_press")
    block = text[start:end]
    if "state.xyz_control.event_for_key(key)" in block:
        return text

    if "key: KeyEvent" not in block:
        block = block.replace(
            "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {",
            "fn handle_key_press(state: &mut AppState, key: KeyEvent) -> KeyHandling {\n    let key_code = key.code;",
            1,
        )
        text = text.replace(
            "        match handle_key_press(&mut state, key.code) {",
            "        match handle_key_press(&mut state, key) {",
            1,
        )

    insert_after = "    let key_code = key.code;\n"
    insert = '''
    if state.active_menu.is_none() && state.control_mode == ControlMode::Scene {
        if let Some(event) = state.xyz_control.event_for_key(key) {
            return apply_app_command(state, AppCommand::XyzControl(event));
        }
    }
'''
    if insert_after not in block:
        raise SystemExit("Could not find key_code extraction in handle_key_press")
    block = block.replace(insert_after, insert_after + insert, 1)

    return text[:start] + block + text[end:]


def main() -> None:
    Path("src/xyz_control.rs").write_text(Path("tools/xyz_control.rs").read_text())

    MAIN.write_text(patch_main_mod(MAIN.read_text()))
    COMMAND.write_text(patch_command(COMMAND.read_text()))

    app = APP.read_text()
    app = patch_app_imports(app)
    app = patch_app_state(app)
    app = patch_apply_xyz_method(app)
    app = patch_apply_command(app)
    app = remove_old_ctrl_block(app)
    app = patch_handle_key_press(app)
    APP.write_text(app)

    print("Added XyzControl basic setup.")
    print("World mode now receives XyzControl events.")
    print("Camera/Light bindings are intentionally pending.")


if __name__ == "__main__":
    main()
