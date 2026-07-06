#!/usr/bin/env python3
from pathlib import Path
import re

APP = Path("src/app.rs")
COMMAND = Path("src/input/command.rs")
MENU = Path("src/menu/model.rs")


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
    old = '''    RotateWorldLeft,
    RotateWorldRight,
    RotateWorldUp,
    RotateWorldDown,
    ResetWorldObject,
'''
    new = '''    RotateWorldPositiveX,
    RotateWorldNegativeX,
    RotateWorldPositiveY,
    RotateWorldNegativeY,
    RotateWorldPositiveZ,
    RotateWorldNegativeZ,
    MoveWorldOriginLeft,
    MoveWorldOriginRight,
    MoveWorldOriginUp,
    MoveWorldOriginDown,
    ResetWorldAxes,
'''
    text = text.replace(old, new, 1)

    text = text.replace(
        '''
    DebugRotateLoadedA3dObjectZPositive,
    DebugRotateLoadedA3dObjectZNegative,''',
        "",
        1,
    )

    return text


def patch_scene_mode_keymap(text: str) -> str:
    start, end = find_brace_span(text, "pub fn scene_mode_command_for_key")
    replacement = '''pub fn scene_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control)),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),

        KeyCode::Char('m') | KeyCode::Char('M') => Some(AppCommand::OpenMenu(MenuKind::Scenes)),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(AppCommand::OpenMenu(MenuKind::Glyphs)),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(AppCommand::OpenMenu(MenuKind::Physics)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::OpenMenu(MenuKind::Debug)),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('x') => Some(AppCommand::RotateWorldPositiveX),
        KeyCode::Char('X') => Some(AppCommand::RotateWorldNegativeX),
        KeyCode::Char('y') => Some(AppCommand::RotateWorldPositiveY),
        KeyCode::Char('Y') => Some(AppCommand::RotateWorldNegativeY),
        KeyCode::Char('z') => Some(AppCommand::RotateWorldPositiveZ),
        KeyCode::Char('Z') => Some(AppCommand::RotateWorldNegativeZ),

        _ => None,
    }
}
'''
    return text[:start] + replacement + text[end:]


def remove_debug_z_from_other_modes(text: str) -> str:
    text = text.replace(
        "        KeyCode::Char('z') => Some(AppCommand::DebugRotateLoadedA3dObjectZPositive),\n",
        "",
    )
    text = text.replace(
        "        KeyCode::Char('Z') => Some(AppCommand::DebugRotateLoadedA3dObjectZNegative),\n",
        "",
    )
    return text


def patch_menu(text: str) -> str:
    old = '''const CONTROL_ITEMS: &[MenuItem] = &[
    MenuItem::real("World mode", AppCommand::SetControlModeScene),
    MenuItem::real("Camera mode", AppCommand::SetControlModeCamera),
    MenuItem::real("Light mode", AppCommand::SetControlModeLight),
];'''
    new = '''const CONTROL_ITEMS: &[MenuItem] = &[
    MenuItem::real("World mode", AppCommand::SetControlModeScene),
    MenuItem::real("Camera mode", AppCommand::SetControlModeCamera),
    MenuItem::real("Light mode", AppCommand::SetControlModeLight),
    MenuItem::real("Rotate world +X  [x]", AppCommand::RotateWorldPositiveX),
    MenuItem::real("Rotate world -X  [X]", AppCommand::RotateWorldNegativeX),
    MenuItem::real("Rotate world +Y  [y]", AppCommand::RotateWorldPositiveY),
    MenuItem::real("Rotate world -Y  [Y]", AppCommand::RotateWorldNegativeY),
    MenuItem::real("Rotate world +Z  [z]", AppCommand::RotateWorldPositiveZ),
    MenuItem::real("Rotate world -Z  [Z]", AppCommand::RotateWorldNegativeZ),
    MenuItem::real("Move origin -X  [Ctrl+Left]", AppCommand::MoveWorldOriginLeft),
    MenuItem::real("Move origin +X  [Ctrl+Right]", AppCommand::MoveWorldOriginRight),
    MenuItem::real("Move origin +Y  [Ctrl+Up]", AppCommand::MoveWorldOriginUp),
    MenuItem::real("Move origin -Y  [Ctrl+Down]", AppCommand::MoveWorldOriginDown),
    MenuItem::real("Reset world axes/origin", AppCommand::ResetWorldAxes),
];'''
    if old not in text:
        raise SystemExit("Could not find CONTROL_ITEMS block")
    text = text.replace(old, new, 1)

    text = text.replace(
        'MenuItem::placeholder("Control menu: C, choose World/Camera/Light", AppCommand::CloseMenu,)',
        'MenuItem::placeholder("World axes: x/X y/Y z/Z rotate; Ctrl+arrows move origin", AppCommand::CloseMenu,)',
    )
    text = text.replace(
        '    MenuItem::placeholder(\n        "Control menu: C, choose World/Camera/Light",\n        AppCommand::CloseMenu,\n    ),',
        '    MenuItem::placeholder(\n        "World axes: x/X y/Y z/Z rotate; Ctrl+arrows move origin",\n        AppCommand::CloseMenu,\n    ),',
    )

    return text


def patch_app_imports(text: str) -> str:
    if "KeyEvent" in text and "KeyModifiers" in text:
        return text

    text = text.replace(
        "KeyCode, KeyEventKind",
        "KeyCode, KeyEvent, KeyEventKind, KeyModifiers",
        1,
    )
    return text


def patch_app_state_world_origin(text: str) -> str:
    if "world_origin:" not in text:
        text = text.replace(
            "    world_camera_pitch_degrees: f32,\n",
            "    world_camera_pitch_degrees: f32,\n    world_origin: Vec3,\n",
            1,
        )

    if "world_origin: Vec3::zero()" not in text:
        text = text.replace(
            "            world_camera_pitch_degrees,\n",
            "            world_camera_pitch_degrees,\n            world_origin: Vec3::zero(),\n",
            1,
        )

    return text


def patch_app_methods(text: str) -> str:
    if "fn move_world_origin" not in text:
        marker = "    fn rotate_world_camera(&mut self, yaw_delta_degrees: f32, pitch_delta_degrees: f32) {"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find rotate_world_camera method")

        methods = '''    fn move_world_origin(&mut self, delta: Vec3) {
        self.world_origin = Vec3::new(
            self.world_origin.x + delta.x,
            self.world_origin.y + delta.y,
            self.world_origin.z + delta.z,
        );

        self.push_debug_console_line(format!(
            "world origin: [{:.2}, {:.2}, {:.2}]",
            self.world_origin.x, self.world_origin.y, self.world_origin.z
        ));
    }

    fn reset_world_axes(&mut self) -> bool {
        self.world_origin = Vec3::zero();
        let rotated = self.reset_loaded_a3d_world_object();
        self.push_debug_console_line("world axes: reset origin and rotation".to_string());
        rotated
    }

'''
        text = text[:index] + methods + text[index:]

    text = text.replace(
        "            ControlMode::Scene => self.reset_loaded_a3d_world_object(),",
        "            ControlMode::Scene => self.reset_world_axes(),",
        1,
    )

    return text


def remove_debug_rotate_method(text: str) -> str:
    marker = "    fn debug_rotate_first_loaded_a3d_object_z"
    if marker not in text:
        return text
    start, end = find_brace_span(text, marker)
    return text[:start] + text[end:]


def patch_workspace_origin(text: str) -> str:
    text = text.replace(
        "    let origin = Vec3::zero();\n    let positive_x = Vec3::new(4.0, 0.0, 0.0);\n    let positive_y = Vec3::new(0.0, 3.0, 0.0);\n    let negative_z = Vec3::new(0.0, 0.0, -4.0);\n",
        "    let origin = state.world_origin;\n    let positive_x = Vec3::new(origin.x + 4.0, origin.y, origin.z);\n    let positive_y = Vec3::new(origin.x, origin.y + 3.0, origin.z);\n    let negative_z = Vec3::new(origin.x, origin.y, origin.z - 4.0);\n",
        1,
    )
    return text


def patch_apply_app_command(text: str) -> str:
    start, end = find_brace_span(text, "fn apply_app_command")
    block = text[start:end]

    # Remove old debug z branches.
    block = re.sub(
        r"\n        AppCommand::DebugRotateLoadedA3dObjectZPositive => \{.*?\n        \}\n\n        AppCommand::DebugRotateLoadedA3dObjectZNegative => \{.*?\n        \}\n",
        "\n",
        block,
        flags=re.S,
    )

    old = '''        AppCommand::RotateWorldLeft => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, -5.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldRight => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 5.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldUp => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(-5.0, 0.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldDown => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(5.0, 0.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::ResetWorldObject => {
            if state.reset_loaded_a3d_world_object() {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }
'''

    new = '''        AppCommand::RotateWorldPositiveX => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(5.0, 0.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldNegativeX => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(-5.0, 0.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldPositiveY => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 5.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldNegativeY => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, -5.0, 0.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldPositiveZ => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 0.0, 5.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::RotateWorldNegativeZ => {
            if state.rotate_loaded_a3d_world_object(Vec3::new(0.0, 0.0, -5.0)) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveWorldOriginLeft => {
            state.move_world_origin(Vec3::new(-0.25, 0.0, 0.0));
            KeyHandling::Handled
        }

        AppCommand::MoveWorldOriginRight => {
            state.move_world_origin(Vec3::new(0.25, 0.0, 0.0));
            KeyHandling::Handled
        }

        AppCommand::MoveWorldOriginUp => {
            state.move_world_origin(Vec3::new(0.0, 0.25, 0.0));
            KeyHandling::Handled
        }

        AppCommand::MoveWorldOriginDown => {
            state.move_world_origin(Vec3::new(0.0, -0.25, 0.0));
            KeyHandling::Handled
        }

        AppCommand::ResetWorldAxes => {
            if state.reset_world_axes() {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }
'''

    if old not in block:
        raise SystemExit("Could not find old RotateWorld command block")

    block = block.replace(old, new, 1)

    return text[:start] + block + text[end:]


def patch_handle_key_press_signature_and_ctrl_arrows(text: str) -> str:
    text = text.replace(
        "fn handle_key_press(state: &mut AppState, key_code: KeyCode) -> KeyHandling {",
        "fn handle_key_press(state: &mut AppState, key: KeyEvent) -> KeyHandling {\n    let key_code = key.code;",
        1,
    )

    marker = "    if state.a3d_file_picker.is_some() {"
    insert = '''    if state.control_mode == ControlMode::Scene && key.modifiers.contains(KeyModifiers::CONTROL) {
        let command = match key_code {
            KeyCode::Left => Some(AppCommand::MoveWorldOriginLeft),
            KeyCode::Right => Some(AppCommand::MoveWorldOriginRight),
            KeyCode::Up => Some(AppCommand::MoveWorldOriginUp),
            KeyCode::Down => Some(AppCommand::MoveWorldOriginDown),
            _ => None,
        };

        if let Some(command) = command {
            return apply_app_command(state, command);
        }
    }

'''

    if insert not in text:
        text = text.replace(marker, insert + marker, 1)

    text = text.replace(
        "        match handle_key_press(&mut state, key.code) {",
        "        match handle_key_press(&mut state, key) {",
        1,
    )

    return text


def patch_help_debug_text(text: str) -> str:
    text = text.replace(
        '"keys/menu/scene routing will be logged here".to_string(),',
        '"world/object debug print statements appear here".to_string(),',
        1,
    )
    text = text.replace(
        '"PageUp/PageDown scroll this debug console".to_string(),',
        '"x/X y/Y z/Z rotate; Ctrl+arrows move origin".to_string(),',
        1,
    )
    return text


def main() -> None:
    command = COMMAND.read_text()
    command = patch_command_enum(command)
    command = patch_scene_mode_keymap(command)
    command = remove_debug_z_from_other_modes(command)
    COMMAND.write_text(command)

    menu = MENU.read_text()
    menu = patch_menu(menu)
    MENU.write_text(menu)

    app = APP.read_text()
    app = patch_app_imports(app)
    app = patch_app_state_world_origin(app)
    app = patch_help_debug_text(app)
    app = patch_app_methods(app)
    app = remove_debug_rotate_method(app)
    app = patch_workspace_origin(app)
    app = patch_apply_app_command(app)
    app = patch_handle_key_press_signature_and_ctrl_arrows(app)
    APP.write_text(app)

    print("Standardized world controls:")
    print("  x/X y/Y z/Z rotate world axes")
    print("  Ctrl+arrows move world origin")
    print("Removed temporary z/Z object debug hotkey and old WASD/arrow world navigation.")


if __name__ == "__main__":
    main()
