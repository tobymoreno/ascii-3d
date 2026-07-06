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

    marker = "    fn edit_loaded_a3d_manifest<F>(&mut self, edit: F) -> bool"
    start, end = find_brace_span(text, marker)

    replacement = '''    fn edit_loaded_a3d_manifest<F>(&mut self, edit: F) -> bool
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

        let serialized =
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| source.clone()) + "\\n";

        if std::fs::write(&manifest_path, serialized).is_err() {
            return false;
        }

        // World and light controls edit the active .a3d manifest on disk.
        // Reload immediately so the cached LoadedWorld and rendered objects
        // reflect the new transform/light data on the next draw.
        self.load_a3d_file(manifest_path);

        true
    }'''

    text = text[:start] + replacement + text[end:]
    APP.write_text(text)

    print("Updated edit_loaded_a3d_manifest to reload the active .a3d after successful edits.")


if __name__ == "__main__":
    main()
