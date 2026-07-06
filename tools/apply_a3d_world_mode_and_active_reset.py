#!/usr/bin/env python3
from pathlib import Path

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


def patch_command() -> None:
    text = COMMAND.read_text()

    if "ResetActiveControl" not in text:
        text = text.replace(
            "    ResetWorldCamera,\n",
            "    ResetWorldCamera,\n    ResetActiveControl,\n",
            1,
        )

    if "RotateWorldLeft" not in text:
        text = text.replace(
            "    RotateCameraDown,\n",
            "    RotateCameraDown,\n\n"
            "    RotateWorldLeft,\n"
            "    RotateWorldRight,\n"
            "    RotateWorldUp,\n"
            "    RotateWorldDown,\n"
            "    ResetWorldObject,\n",
            1,
        )

    text = text.replace(
        "KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetWorldCamera)",
        "KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl)",
    )

    world_mode = '''pub fn scene_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control)),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),

        KeyCode::Char('x') | KeyCode::Char('X') => Some(AppCommand::OpenMenu(MenuKind::File)),
        KeyCode::Char('m') | KeyCode::Char('M') => Some(AppCommand::OpenMenu(MenuKind::Scenes)),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(AppCommand::OpenMenu(MenuKind::Glyphs)),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(AppCommand::OpenMenu(MenuKind::Physics)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::OpenMenu(MenuKind::Debug)),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
            Some(AppCommand::RotateWorldLeft)
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
            Some(AppCommand::RotateWorldRight)
        }
        KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => Some(AppCommand::RotateWorldUp),
        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
            Some(AppCommand::RotateWorldDown)
        }

        _ => None,
    }
}'''

    start, end = find_brace_span(text, "pub fn scene_mode_command_for_key")
    text = text[:start] + world_mode + text[end:]

    if "pub fn light_mode_command_for_key" in text:
        start, end = find_brace_span(text, "pub fn light_mode_command_for_key")
        light = text[start:end]
        if "ResetActiveControl" not in light:
            light = light.replace(
                "        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {\n",
                "        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),\n        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {\n",
                1,
            )
        text = text[:start] + light + text[end:]

    COMMAND.write_text(text)


def patch_app() -> None:
    text = APP.read_text()

    helpers = '''    fn reset_active_control(&mut self) -> bool {
        match self.control_mode {
            ControlMode::Scene => self.reset_loaded_a3d_world_object(),
            ControlMode::Camera => {
                self.reset_world_camera();
                true
            }
            ControlMode::Light => self.reset_loaded_a3d_light(),
        }
    }

    fn loaded_a3d_manifest_path_for_edit(&self) -> Option<PathBuf> {
        self.loaded_a3d_manifest_path
            .clone()
            .or_else(|| self.loaded_a3d_root.as_ref().map(|root| root.join("scene.a3d")))
    }

    fn edit_loaded_a3d_manifest<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce(&mut serde_json::Value) -> bool,
    {
        let Some(manifest_path) = self.loaded_a3d_manifest_path_for_edit() else {
            return false;
        };

        let Ok(source) = std::fs::read_to_string(&manifest_path) else {
            return false;
        };

        let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&source) else {
            return false;
        };

        if !edit(&mut json) {
            return false;
        }

        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| source.clone()) + "\\n",
        )
        .is_ok()
    }

    fn edit_first_loaded_a3d_light_position<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce([f32; 3]) -> [f32; 3],
    {
        self.edit_loaded_a3d_manifest(|json| {
            let Some(lights) = json.get_mut("lights").and_then(serde_json::Value::as_array_mut)
            else {
                return false;
            };

            let Some(light) = lights.first_mut() else {
                return false;
            };

            let Some(position) = light
                .get_mut("position")
                .and_then(serde_json::Value::as_array_mut)
            else {
                return false;
            };

            if position.len() != 3 {
                return false;
            }

            let current = [
                position[0].as_f64().unwrap_or(0.0) as f32,
                position[1].as_f64().unwrap_or(0.0) as f32,
                position[2].as_f64().unwrap_or(0.0) as f32,
            ];

            let next = edit(current);

            position[0] = serde_json::json!(next[0]);
            position[1] = serde_json::json!(next[1]);
            position[2] = serde_json::json!(next[2]);

            true
        })
    }

    fn move_loaded_a3d_light(&mut self, delta: Vec3) -> bool {
        self.edit_first_loaded_a3d_light_position(|current| {
            [
                current[0] + delta.x,
                current[1] + delta.y,
                current[2] + delta.z,
            ]
        })
    }

    fn reset_loaded_a3d_light(&mut self) -> bool {
        self.edit_first_loaded_a3d_light_position(|_| [5.0, 2.0, -2.5])
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

    fn edit_first_loaded_a3d_object_rotation<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce([f32; 3]) -> [f32; 3],
    {
        self.edit_loaded_a3d_manifest(|json| {
            let Some(objects) = json
                .get_mut("objects")
                .and_then(serde_json::Value::as_array_mut)
            else {
                return false;
            };

            let Some(object) = objects.first_mut() else {
                return false;
            };

            if object.get("transform").is_none() {
                object["transform"] = serde_json::json!({});
            }

            let Some(transform) = object
                .get_mut("transform")
                .and_then(serde_json::Value::as_object_mut)
            else {
                return false;
            };

            if !transform.contains_key("rotation") {
                transform.insert("rotation".to_string(), serde_json::json!([0.0, 0.0, 0.0]));
            }

            let Some(rotation) = transform
                .get_mut("rotation")
                .and_then(serde_json::Value::as_array_mut)
            else {
                return false;
            };

            if rotation.len() != 3 {
                *rotation = vec![
                    serde_json::json!(0.0),
                    serde_json::json!(0.0),
                    serde_json::json!(0.0),
                ];
            }

            let current = [
                rotation[0].as_f64().unwrap_or(0.0) as f32,
                rotation[1].as_f64().unwrap_or(0.0) as f32,
                rotation[2].as_f64().unwrap_or(0.0) as f32,
            ];

            let next = edit(current);

            rotation[0] = serde_json::json!(next[0]);
            rotation[1] = serde_json::json!(next[1]);
            rotation[2] = serde_json::json!(next[2]);

            true
        })
    }

    fn rotate_loaded_a3d_world_object(&mut self, delta: Vec3) -> bool {
        self.edit_first_loaded_a3d_object_rotation(|current| {
            [
                current[0] + delta.x,
                current[1] + delta.y,
                current[2] + delta.z,
            ]
        })
    }

    fn reset_loaded_a3d_world_object(&mut self) -> bool {
        self.edit_first_loaded_a3d_object_rotation(|_| [0.0, 0.0, 0.0])
    }

'''

    old_start = text.find("    fn loaded_a3d_manifest_path_for_edit(")
    if old_start >= 0:
        marker = "    fn toggle_frame_timing(&mut self)"
        old_end = text.find(marker, old_start)
        if old_end < 0:
            raise SystemExit("Could not find end of existing loaded A3D edit helpers")
        text = text[:old_start] + helpers + text[old_end:]
    else:
        marker = "    fn toggle_frame_timing(&mut self)"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find insertion point for A3D edit helpers")
        text = text[:index] + helpers + text[index:]

    if "AppCommand::ResetActiveControl" not in text:
        text = text.replace(
            '''        AppCommand::ResetWorldCamera | AppCommand::ResetCamera => {
            state.reset_world_camera();
            KeyHandling::Handled
        }
''',
            '''        AppCommand::ResetActiveControl => {
            if state.reset_active_control() {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::ResetWorldCamera | AppCommand::ResetCamera => {
            state.reset_world_camera();
            KeyHandling::Handled
        }
''',
            1,
        )

    if "AppCommand::RotateWorldLeft =>" not in text:
        world_branches = '''        AppCommand::RotateWorldLeft => {
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
        marker = "        AppCommand::MoveCameraForward =>"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find camera command branch insertion point")
        text = text[:index] + world_branches + text[index:]

    APP.write_text(text)


def main() -> None:
    patch_command()
    patch_app()
    print("Added real World mode rotation and active-target reset.")
    print("R now resets active target: World object, Camera, or Light.")


if __name__ == "__main__":
    main()
