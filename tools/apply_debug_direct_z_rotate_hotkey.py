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

    if "DebugRotateLoadedA3dObjectZPositive" not in text:
        text = text.replace(
            "    ShowOsGraphicsOverlay,\n",
            "    ShowOsGraphicsOverlay,\n\n"
            "    DebugRotateLoadedA3dObjectZPositive,\n"
            "    DebugRotateLoadedA3dObjectZNegative,\n",
            1,
        )

    # Add direct z/Z test hotkeys to every mode command function.
    for marker in [
        "pub fn scene_mode_command_for_key",
        "pub fn camera_mode_command_for_key",
        "pub fn light_mode_command_for_key",
    ]:
        if marker not in text:
            continue

        start, end = find_brace_span(text, marker)
        function_text = text[start:end]

        if "DebugRotateLoadedA3dObjectZPositive" not in function_text:
            function_text = function_text.replace(
                "    match key {\n",
                "    match key {\n"
                "        KeyCode::Char('z') => Some(AppCommand::DebugRotateLoadedA3dObjectZPositive),\n"
                "        KeyCode::Char('Z') => Some(AppCommand::DebugRotateLoadedA3dObjectZNegative),\n",
                1,
            )

        text = text[:start] + function_text + text[end:]

    COMMAND.write_text(text)


def patch_app() -> None:
    text = APP.read_text()

    helper = '''    fn debug_rotate_first_loaded_a3d_object_z(&mut self, delta_degrees: f32) -> bool {
        let Some(world) = &mut self.loaded_a3d_world else {
            self.loaded_a3d_error = Some("debug z rotate: no loaded .a3d world".to_string());
            return false;
        };

        let Some(object) = world.objects.first_mut() else {
            self.loaded_a3d_error = Some("debug z rotate: loaded .a3d world has no objects".to_string());
            return false;
        };

        object.transform.rotation_degrees[2] += delta_degrees;
        object.transform.rotation_degrees[2] =
            object.transform.rotation_degrees[2].rem_euclid(FULL_ROTATION_DEGREES);

        self.loaded_a3d_error = Some(format!(
            "debug z rotate: {} rot_z={:.1}",
            object.id, object.transform.rotation_degrees[2],
        ));

        true
    }

'''

    if "fn debug_rotate_first_loaded_a3d_object_z" not in text:
        marker = "    fn update(&mut self, elapsed: Duration) -> bool {"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find insertion point before AppState::update")
        text = text[:index] + helper + text[index:]

    if "AppCommand::DebugRotateLoadedA3dObjectZPositive" not in text:
        branch = '''        AppCommand::DebugRotateLoadedA3dObjectZPositive => {
            if state.debug_rotate_first_loaded_a3d_object_z(15.0) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

        AppCommand::DebugRotateLoadedA3dObjectZNegative => {
            if state.debug_rotate_first_loaded_a3d_object_z(-15.0) {
                KeyHandling::Handled
            } else {
                KeyHandling::Ignored
            }
        }

'''

        marker = "        AppCommand::Quit => KeyHandling::Quit,"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find AppCommand::Quit branch")
        insert_at = index + len(marker) + 1
        text = text[:insert_at] + "\n" + branch + text[insert_at:]

    APP.write_text(text)


def main() -> None:
    patch_command()
    patch_app()

    print("Added direct debug Z rotation hotkey.")
    print("z rotates first loaded .a3d object +15 degrees around Z.")
    print("Z rotates first loaded .a3d object -15 degrees around Z.")


if __name__ == "__main__":
    main()
