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


def main() -> None:
    text = APP.read_text()

    marker = "    fn push_world_debug_lines(&mut self) {"
    start, end = find_brace_span(text, marker)
    block = text[start:end]

    new_block = '''    fn push_world_debug_lines(&mut self) {
        let mut lines = vec![
            format!("world debug: scene={}", self.current_scene().title()),
            format!(
                "world debug: camera pos [{:.2}, {:.2}, {:.2}] yaw {:.1} pitch {:.1}",
                self.world_camera_position.x,
                self.world_camera_position.y,
                self.world_camera_position.z,
                self.world_camera_yaw_degrees,
                self.world_camera_pitch_degrees,
            ),
            format!(
                "world debug: control_mode={} menu={}",
                self.control_mode.label(),
                self.active_menu
                    .as_ref()
                    .map(|menu| menu.kind().title())
                    .unwrap_or("closed"),
            ),
        ];

        if let Some(world) = self.loaded_a3d_world.as_ref() {
            lines.push(format!(
                "world debug: loaded_a3d title='{}' objects={}",
                world.title,
                world.objects.len(),
            ));

            lines.extend(world.objects.iter().map(|object| {
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
            }));
        } else {
            lines.push("world debug: no loaded_a3d world".to_string());
        }

        for line in lines {
            self.push_debug_console_line(line);
        }
    }
'''

    text = text[:start] + new_block + text[end:]
    APP.write_text(text)

    print("Fixed push_world_debug_lines borrow issue by collecting lines before mutating self.")


if __name__ == "__main__":
    main()
