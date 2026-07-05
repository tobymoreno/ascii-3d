#!/usr/bin/env python3
from pathlib import Path
import re

def find_toggle_return(text: str) -> str:
    match = re.search(
        r"AppCommand::ToggleFrameTiming\s*=>\s*\{(?P<body>.*?)\n\s*\}",
        text,
        re.DOTALL,
    )
    if not match:
        raise SystemExit("Could not find AppCommand::ToggleFrameTiming arm.")

    body = match.group("body")

    key_match = re.findall(r"KeyHandling::[A-Za-z0-9_]+", body)
    if not key_match:
        raise SystemExit("Could not find KeyHandling return in ToggleFrameTiming arm.")

    return key_match[-1]

def main() -> None:
    path = Path("src/app.rs")
    text = path.read_text()

    key_return = find_toggle_return(text)

    pattern = re.compile(
        r"(?P<indent>\s*)AppCommand::ShowOsGraphicsOverlay\s*=>\s*\{\n"
        r"(?P<body>\s*crate::graphics::raylib_overlay::spawn_raylib_overlay_demo\(\);\n)"
        r"\s*\}",
        re.DOTALL,
    )

    def replace(match: re.Match) -> str:
        indent = match.group("indent")
        body = match.group("body")

        if key_return in match.group(0):
            return match.group(0)

        return (
            f"{indent}AppCommand::ShowOsGraphicsOverlay => {{\n"
            f"{body}"
            f"{indent}    {key_return}\n"
            f"{indent}}}"
        )

    text, count = pattern.subn(replace, text, count=1)

    if count == 0:
        raise SystemExit("Could not patch AppCommand::ShowOsGraphicsOverlay arm.")

    path.write_text(text)

    print(f"Patched ShowOsGraphicsOverlay arm to return {key_return}.")

if __name__ == "__main__":
    main()
