#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")


def main() -> None:
    text = APP.read_text()

    # Remove unused KeyModifiers import from app.rs if present. XyzControl owns modifier handling now.
    text = text.replace(
        "event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},",
        "event::{self, Event, KeyCode, KeyEvent, KeyEventKind},",
        1,
    )

    # Add the actual AppState field. Avoid broad "xyz_control:" checks because that text
    # can appear in debug strings or method calls.
    if "    xyz_control: XyzControl,\n" not in text:
        marker = "    control_mode: ControlMode,\n"
        if marker not in text:
            raise SystemExit("Could not find AppState control_mode field.")
        text = text.replace(
            marker,
            marker + "    xyz_control: XyzControl,\n",
            1,
        )

    # Ensure initializer exists exactly once.
    if "            xyz_control: XyzControl::default(),\n" not in text:
        marker = "            control_mode: ControlMode::Scene,\n"
        if marker not in text:
            raise SystemExit("Could not find AppState control_mode initializer.")
        text = text.replace(
            marker,
            marker + "            xyz_control: XyzControl::default(),\n",
            1,
        )

    APP.write_text(text)

    print("Fixed AppState.xyz_control field.")
    print("Removed unused KeyModifiers import from app.rs.")


if __name__ == "__main__":
    main()
