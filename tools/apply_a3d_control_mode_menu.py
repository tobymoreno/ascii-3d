#!/usr/bin/env python3
from pathlib import Path
import re

APP = Path("src/app.rs")
MENU_MODEL = Path("src/menu/model.rs")
INPUT_COMMAND = Path("src/input/command.rs")
INPUT_MOD = Path("src/input/mod.rs")


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


def replace_function(text: str, marker: str, replacement: str) -> str:
    start, end = find_brace_span(text, marker)
    return text[:start] + replacement.rstrip() + "\n" + text[end:]


def patch_menu_model() -> None:
    text = MENU_MODEL.read_text()

    text = re.sub(
        r"pub enum MenuKind \{.*?\n\}",
        '''pub enum MenuKind {
    File,
    Scenes,
    Control,
    Glyphs,
    Physics,
    Debug,
    Help,
}''',
        text,
        flags=re.DOTALL,
        count=1,
    )

    text = replace_function(
        text,
        "    pub const fn title(",
        '''    pub const fn title(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Scenes => "Scenes",
            Self::Control => "Control",
            Self::Glyphs => "Glyphs",
            Self::Physics => "Physics",
            Self::Debug => "Debug",
            Self::Help => "Help",
        }
    }''',
    )

    text = replace_function(
        text,
        "    pub const fn hotkey(",
        '''    pub const fn hotkey(self) -> &'static str {
        match self {
            Self::File => "x",
            Self::Scenes => "m",
            Self::Control => "c",
            Self::Glyphs => "g",
            Self::Physics => "f",
            Self::Debug => "d",
            Self::Help => "h/?",
        }
    }''',
    )

    text = replace_function(
        text,
        "    pub fn items(",
        '''    pub fn items(self) -> &'static [MenuItem] {
        match self {
            Self::File => FILE_ITEMS,
            Self::Scenes => SCENE_ITEMS,
            Self::Control => CONTROL_ITEMS,
            Self::Glyphs => GLYPH_ITEMS,
            Self::Physics => PHYSICS_ITEMS,
            Self::Debug => DEBUG_ITEMS,
            Self::Help => HELP_ITEMS,
        }
    }''',
    )

    if "pub fn with_selected" not in text:
        text = text.replace(
            '''    pub const fn new(kind: MenuKind) -> Self {
        Self {
            kind,
            selected_index: 0,
        }
    }
''',
            '''    pub const fn new(kind: MenuKind) -> Self {
        Self {
            kind,
            selected_index: 0,
        }
    }

    pub fn with_selected(kind: MenuKind, selected_index: usize) -> Self {
        let item_count = kind.items().len();

        Self {
            kind,
            selected_index: if item_count == 0 {
                0
            } else {
                selected_index.min(item_count - 1)
            },
        }
    }
''',
            1,
        )

    text = re.sub(
        r"const CAMERA_ITEMS: &\[MenuItem\] = &\[.*?\];\n\nconst WORLD_ITEMS: &\[MenuItem\] = &\[.*?\];",
        '''const CONTROL_ITEMS: &[MenuItem] = &[
    MenuItem::real("World mode", AppCommand::SetControlModeScene),
    MenuItem::real("Camera mode", AppCommand::SetControlModeCamera),
    MenuItem::real("Light mode", AppCommand::SetControlModeLight),
];''',
        text,
        flags=re.DOTALL,
        count=1,
    )

    text = text.replace(
        '''const HELP_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Scene mode: arrows change scene", AppCommand::CloseMenu),
    MenuItem::placeholder("Camera mode: WASD/QE move camera", AppCommand::CloseMenu),
    MenuItem::placeholder("Menus: j/k or arrows, Enter, Esc", AppCommand::CloseMenu),
];''',
        '''const HELP_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Control menu: C, choose World/Camera/Light", AppCommand::CloseMenu),
    MenuItem::placeholder("Camera mode: WASD/QE move camera", AppCommand::CloseMenu),
    MenuItem::placeholder("Light mode: WASD/QE move light", AppCommand::CloseMenu),
    MenuItem::placeholder("Menus: j/k or arrows, Enter, Esc", AppCommand::CloseMenu),
];''',
        1,
    )

    MENU_MODEL.write_text(text)


def patch_input_command() -> None:
    text = INPUT_COMMAND.read_text()

    # C opens the Control menu from all modes.
    text = text.replace(
        "KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::SetControlModeCamera)",
        "KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control))",
    )

    text = text.replace("MenuKind::Camera", "MenuKind::Control")
    text = text.replace("MenuKind::World", "MenuKind::Control")

    # Remove direct L/W switching in scene mode. Control menu owns explicit mode selection.
    scene_marker = "pub fn scene_mode_command_for_key"
    if scene_marker in text:
        start, end = find_brace_span(text, scene_marker)
        function_text = text[start:end]
        function_text = function_text.replace(
            "        KeyCode::Char('l') | KeyCode::Char('L') => Some(AppCommand::SetControlModeLight),\n",
            "",
        )
        function_text = function_text.replace(
            "        KeyCode::Char('w') | KeyCode::Char('W') => Some(AppCommand::SetControlModeScene),\n",
            "",
        )
        text = text[:start] + function_text + text[end:]

    INPUT_COMMAND.write_text(text)


def patch_input_mod() -> None:
    if not INPUT_MOD.exists():
        return

    text = INPUT_MOD.read_text()

    if "light_mode_command_for_key" in text:
        return

    match = re.search(r"pub use command::\{(?P<body>.*?)\};", text, re.DOTALL)
    if not match:
        return

    items = [item.strip() for item in match.group("body").replace("\n", " ").split(",")]
    items = [item for item in items if item]
    items.append("light_mode_command_for_key")

    preferred = [
        "AppCommand",
        "camera_mode_command_for_key",
        "light_mode_command_for_key",
        "menu_command_for_key",
        "scene_mode_command_for_key",
    ]
    ordered = [item for item in preferred if item in items]
    ordered.extend(item for item in items if item not in ordered)

    replacement = "pub use command::{\n    " + ",\n    ".join(ordered) + ",\n};"
    INPUT_MOD.write_text(text[: match.start()] + replacement + text[match.end() :])


def patch_app() -> None:
    text = APP.read_text()

    text = text.replace(
        "camera_mode_command_for_key, menu_command_for_key, scene_mode_command_for_key",
        "camera_mode_command_for_key, light_mode_command_for_key, menu_command_for_key, scene_mode_command_for_key",
        1,
    )

    replacement = '''    fn control_mode_menu_index(&self) -> usize {
        match self.control_mode {
            ControlMode::Scene => 0,
            ControlMode::Camera => 1,
            ControlMode::Light => 2,
        }
    }

    fn open_menu(&mut self, kind: crate::menu::MenuKind) {
        let selected_index = match kind {
            crate::menu::MenuKind::Control => self.control_mode_menu_index(),
            _ => 0,
        };

        self.active_menu = Some(MenuState::with_selected(kind, selected_index));
    }'''

    text = replace_function(text, "    fn open_menu(", replacement)

    for name, mode in [
        ("SetControlModeScene", "ControlMode::Scene"),
        ("SetControlModeCamera", "ControlMode::Camera"),
        ("SetControlModeLight", "ControlMode::Light"),
    ]:
        old = f'''        AppCommand::{name} => {{
            state.set_control_mode({mode});
            KeyHandling::Handled
        }}'''
        new = f'''        AppCommand::{name} => {{
            state.set_control_mode({mode});
            state.close_menu();
            KeyHandling::Handled
        }}'''
        text = text.replace(old, new)

    APP.write_text(text)


def main() -> None:
    patch_menu_model()
    patch_input_command()
    patch_input_mod()
    patch_app()

    print("Added real Control menu for World/Camera/Light modes.")
    print("C now opens the Control dropdown, selected to the active mode.")


if __name__ == "__main__":
    main()
