#!/usr/bin/env python3
from pathlib import Path
import re

APP = Path("src/app.rs")
COMMAND = Path("src/input/command.rs")


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


def replace_span(text: str, marker: str, replacement: str) -> str:
    start, end = find_brace_span(text, marker)
    return text[:start] + replacement.rstrip() + "\n" + text[end:]


def insert_after_function(text: str, function_name: str, addition: str) -> str:
    if addition.strip().splitlines()[0] in text:
        return text

    _, end = find_brace_span(text, f"pub fn {function_name}(")
    return text[:end] + "\n\n" + addition.rstrip() + text[end:]


def patch_command() -> None:
    text = COMMAND.read_text()

    if "SetControlModeLight" not in text:
        text = text.replace(
            "    ToggleControlMode,\n",
            "    ToggleControlMode,\n"
            "    SetControlModeScene,\n"
            "    SetControlModeCamera,\n"
            "    SetControlModeLight,\n",
            1,
        )

    if "MoveLightForward" not in text:
        text = text.replace(
            "    RotateCameraDown,\n",
            "    RotateCameraDown,\n\n"
            "    MoveLightForward,\n"
            "    MoveLightBackward,\n"
            "    MoveLightLeft,\n"
            "    MoveLightRight,\n"
            "    MoveLightDown,\n"
            "    MoveLightUp,\n",
            1,
        )

    scene_span = text.split("pub fn scene_mode_command_for_key", 1)[1].split("pub fn", 1)[0]
    if "SetControlModeCamera" not in scene_span:
        text = text.replace(
            "        KeyCode::Tab => Some(AppCommand::ToggleControlMode),\n",
            "        KeyCode::Tab => Some(AppCommand::ToggleControlMode),\n"
            "        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::SetControlModeCamera),\n"
            "        KeyCode::Char('l') | KeyCode::Char('L') => Some(AppCommand::SetControlModeLight),\n"
            "        KeyCode::Char('w') | KeyCode::Char('W') => Some(AppCommand::SetControlModeScene),\n",
            1,
        )

    text = re.sub(
        r"\n\s*KeyCode::Char\('c'\) \| KeyCode::Char\('C'\) => Some\(AppCommand::OpenMenu\(MenuKind::Camera\)\),",
        "",
        text,
        count=1,
    )
    text = re.sub(
        r"\n\s*KeyCode::Char\('w'\) \| KeyCode::Char\('W'\) => Some\(AppCommand::OpenMenu\(MenuKind::World\)\),",
        "",
        text,
        count=1,
    )

    if "pub fn camera_mode_command_for_key" in text:
        camera_span = text.split("pub fn camera_mode_command_for_key", 1)[1].split("pub fn", 1)[0]
        if "SetControlModeLight" not in camera_span:
            start, end = find_brace_span(text, "pub fn camera_mode_command_for_key")
            function_text = text[start:end]
            function_text = function_text.replace(
                "        KeyCode::Tab => Some(AppCommand::ToggleControlMode),\n",
                "        KeyCode::Tab => Some(AppCommand::ToggleControlMode),\n"
                "        KeyCode::Char('l') | KeyCode::Char('L') => Some(AppCommand::SetControlModeLight),\n"
                "        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::SetControlModeCamera),\n",
                1,
            )
            text = text[:start] + function_text + text[end:]

    light_mode = '''pub fn light_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::SetControlModeCamera),
        KeyCode::Char('l') | KeyCode::Char('L') => Some(AppCommand::SetControlModeLight),
        KeyCode::Char('W') => Some(AppCommand::SetControlModeScene),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('w') => Some(AppCommand::MoveLightForward),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(AppCommand::MoveLightBackward),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(AppCommand::MoveLightLeft),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::MoveLightRight),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::MoveLightDown),
        KeyCode::Char('e') | KeyCode::Char('E') => Some(AppCommand::MoveLightUp),

        KeyCode::Left => Some(AppCommand::MoveLightLeft),
        KeyCode::Right => Some(AppCommand::MoveLightRight),
        KeyCode::Up => Some(AppCommand::MoveLightUp),
        KeyCode::Down => Some(AppCommand::MoveLightDown),

        _ => None,
    }
}'''

    if "pub fn light_mode_command_for_key" not in text:
        text = insert_after_function(text, "camera_mode_command_for_key", light_mode)

    COMMAND.write_text(text)


def patch_app() -> None:
    text = APP.read_text()

    text = text.replace(
        "camera_mode_command_for_key, menu_command_for_key, scene_mode_command_for_key",
        "camera_mode_command_for_key, light_mode_command_for_key, menu_command_for_key, scene_mode_command_for_key",
        1,
    )

    control_mode = '''enum ControlMode {
    Scene,
    Camera,
    Light,
}

impl ControlMode {
    fn label(self) -> &'static str {
        match self {
            Self::Scene => "World",
            Self::Camera => "Camera",
            Self::Light => "Light",
        }
    }
}'''
    if "Light," not in text.split("enum ControlMode", 1)[1].split("impl ControlMode", 1)[0]:
        text = replace_span(text, "enum ControlMode", control_mode)

    toggle = '''    fn toggle_control_mode(&mut self) {
        self.control_mode = match self.control_mode {
            ControlMode::Scene => ControlMode::Camera,
            ControlMode::Camera => ControlMode::Light,
            ControlMode::Light => ControlMode::Scene,
        };
    }

    fn set_control_mode(&mut self, control_mode: ControlMode) {
        self.control_mode = control_mode;
    }'''
    if "fn set_control_mode" not in text:
        text = replace_span(text, "    fn toggle_control_mode(", toggle)

    light_methods = '''    fn loaded_a3d_manifest_path_for_edit(&self) -> Option<PathBuf> {
        self.loaded_a3d_manifest_path
            .clone()
            .or_else(|| self.loaded_a3d_root.as_ref().map(|root| root.join("scene.a3d")))
    }

    fn move_loaded_a3d_light(&mut self, delta: Vec3) -> bool {
        let Some(manifest_path) = self.loaded_a3d_manifest_path_for_edit() else {
            return false;
        };

        let Ok(source) = std::fs::read_to_string(&manifest_path) else {
            return false;
        };

        let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&source) else {
            return false;
        };

        let Some(lights) = json.get_mut("lights").and_then(serde_json::Value::as_array_mut) else {
            return false;
        };

        let Some(light) = lights.first_mut() else {
            return false;
        };

        let Some(position) = light
            .get_mut("position")
            .and_then(serde_json::Value::as_array_mut)
        else {
            light["position"] = serde_json::json!([delta.x, delta.y, delta.z]);
            return std::fs::write(
                &manifest_path,
                serde_json::to_string_pretty(&json).unwrap_or_else(|_| source.clone()) + "\n",
            )
            .is_ok();
        };

        if position.len() != 3 {
            return false;
        }

        let current = [
            position[0].as_f64().unwrap_or(0.0) as f32,
            position[1].as_f64().unwrap_or(0.0) as f32,
            position[2].as_f64().unwrap_or(0.0) as f32,
        ];

        position[0] = serde_json::json!(current[0] + delta.x);
        position[1] = serde_json::json!(current[1] + delta.y);
        position[2] = serde_json::json!(current[2] + delta.z);

        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| source.clone()) + "\n",
        )
        .is_ok()
    }

    fn move_loaded_a3d_light_forward(&mut self, amount: f32) -> bool {
        self.move_loaded_a3d_light(Vec3::new(0.0, 0.0, -amount))
    }

    fn move_loaded_a3d_light_right(&mut self, amount: f32) -> bool {
        self.move_loaded_a3d_light(Vec3::new(amount, 0.0, 0.0))
    }

    fn move_loaded_a3d_light_up(&mut self, amount: f32) -> bool {
        self.move_loaded_a3d_light(Vec3::new(0.0, amount, 0.0))
    }

'''
    if "fn move_loaded_a3d_light(" not in text:
        marker = "    fn toggle_frame_timing(&mut self)"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find insertion point for light movement methods")
        text = text[:index] + light_methods + text[index:]

    if "AppCommand::SetControlModeLight" not in text:
        text = text.replace(
            '''        AppCommand::ToggleControlMode => {
            state.toggle_control_mode();
            KeyHandling::Handled
        }
''',
            '''        AppCommand::ToggleControlMode => {
            state.toggle_control_mode();
            KeyHandling::Handled
        }

        AppCommand::SetControlModeScene => {
            state.set_control_mode(ControlMode::Scene);
            KeyHandling::Handled
        }

        AppCommand::SetControlModeCamera => {
            state.set_control_mode(ControlMode::Camera);
            KeyHandling::Handled
        }

        AppCommand::SetControlModeLight => {
            state.set_control_mode(ControlMode::Light);
            KeyHandling::Handled
        }
''',
            1,
        )

    light_branches = '''        AppCommand::MoveLightForward => {
            if state.move_loaded_a3d_light_forward(0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightBackward => {
            if state.move_loaded_a3d_light_forward(-0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightLeft => {
            if state.move_loaded_a3d_light_right(-0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightRight => {
            if state.move_loaded_a3d_light_right(0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightDown => {
            if state.move_loaded_a3d_light_up(-0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::MoveLightUp => {
            if state.move_loaded_a3d_light_up(0.25) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

'''
    if "AppCommand::MoveLightForward =>" not in text:
        marker = "        // Cross-term menu placeholders."
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find apply_app_command placeholder marker")
        text = text[:index] + light_branches + text[index:]

    text = text.replace(
        '''    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
    };
''',
        '''    let command = match state.control_mode {
        ControlMode::Scene => scene_mode_command_for_key(key_code),
        ControlMode::Camera => camera_mode_command_for_key(key_code),
        ControlMode::Light => light_mode_command_for_key(key_code),
    };
''',
        1,
    )

    APP.write_text(text)


def main() -> None:
    patch_command()
    patch_app()
    print("Added A3D control modes: World, Camera, Light.")
    print("Light mode moves the first loaded A3D light position with WASD/QE or arrow keys.")


if __name__ == "__main__":
    main()
