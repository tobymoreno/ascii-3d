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
    if "    MenuLeft,\n" not in text:
        text = text.replace(
            "    MenuDown,\n",
            "    MenuDown,\n    MenuLeft,\n    MenuRight,\n",
            1,
        )
    return text


def patch_menu_command_for_key(text: str) -> str:
    start, end = find_brace_span(text, "pub fn menu_command_for_key")
    replacement = '''pub fn menu_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc => Some(AppCommand::CloseMenu),
        KeyCode::Left => Some(AppCommand::MenuLeft),
        KeyCode::Right => Some(AppCommand::MenuRight),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(AppCommand::MenuUp),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(AppCommand::MenuDown),
        KeyCode::Enter => Some(AppCommand::MenuSelect),
        _ => None,
    }
}
'''
    return text[:start] + replacement + text[end:]


def patch_file_physics_hotkeys_in_keymaps(text: str) -> str:
    text = text.replace(
        "        KeyCode::Char('x') | KeyCode::Char('X') => Some(AppCommand::OpenMenu(MenuKind::File)),",
        "        KeyCode::Char('f') | KeyCode::Char('F') => Some(AppCommand::OpenMenu(MenuKind::File)),",
    )
    text = text.replace(
        "        KeyCode::Char('f') | KeyCode::Char('F') => Some(AppCommand::OpenMenu(MenuKind::Physics)),",
        "        KeyCode::Char('p') | KeyCode::Char('P') => Some(AppCommand::OpenMenu(MenuKind::Physics)),",
    )
    return text


def patch_menu_model(text: str) -> str:
    text = text.replace('            Self::File => "x",', '            Self::File => "f",')
    text = text.replace('            Self::Physics => "f",', '            Self::Physics => "p",')
    return text


def patch_app_imports(text: str) -> str:
    if "KeyModifiers" not in text:
        text = text.replace(
            "event::{self, Event, KeyCode, KeyEvent, KeyEventKind},",
            "event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},",
            1,
        )
    return text


def insert_menu_kind_helpers(text: str) -> str:
    if "fn next_menu_kind(" in text:
        return text

    marker = "impl ControlMode {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find impl ControlMode insertion point")

    helpers = '''fn next_menu_kind(kind: crate::menu::MenuKind) -> crate::menu::MenuKind {
    match kind {
        crate::menu::MenuKind::File => crate::menu::MenuKind::Scenes,
        crate::menu::MenuKind::Scenes => crate::menu::MenuKind::Control,
        crate::menu::MenuKind::Control => crate::menu::MenuKind::Glyphs,
        crate::menu::MenuKind::Glyphs => crate::menu::MenuKind::Physics,
        crate::menu::MenuKind::Physics => crate::menu::MenuKind::Debug,
        crate::menu::MenuKind::Debug => crate::menu::MenuKind::Help,
        crate::menu::MenuKind::Help => crate::menu::MenuKind::File,
    }
}

fn previous_menu_kind(kind: crate::menu::MenuKind) -> crate::menu::MenuKind {
    match kind {
        crate::menu::MenuKind::File => crate::menu::MenuKind::Help,
        crate::menu::MenuKind::Scenes => crate::menu::MenuKind::File,
        crate::menu::MenuKind::Control => crate::menu::MenuKind::Scenes,
        crate::menu::MenuKind::Glyphs => crate::menu::MenuKind::Control,
        crate::menu::MenuKind::Physics => crate::menu::MenuKind::Glyphs,
        crate::menu::MenuKind::Debug => crate::menu::MenuKind::Physics,
        crate::menu::MenuKind::Help => crate::menu::MenuKind::Debug,
    }
}

fn menu_kind_for_hotkey(key_code: KeyCode) -> Option<crate::menu::MenuKind> {
    match key_code {
        KeyCode::Char('f') | KeyCode::Char('F') => Some(crate::menu::MenuKind::File),
        KeyCode::Char('m') | KeyCode::Char('M') => Some(crate::menu::MenuKind::Scenes),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(crate::menu::MenuKind::Control),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(crate::menu::MenuKind::Glyphs),
        KeyCode::Char('p') | KeyCode::Char('P') => Some(crate::menu::MenuKind::Physics),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(crate::menu::MenuKind::Debug),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => Some(crate::menu::MenuKind::Help),
        _ => None,
    }
}

'''
    return text[:index] + helpers + text[index:]


def insert_appstate_menu_methods(text: str) -> str:
    if "fn open_next_menu" in text:
        return text

    marker = "    fn close_menu(&mut self) {"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find close_menu insertion point")

    methods = '''    fn toggle_menu_bar(&mut self) {
        if self.active_menu.is_some() {
            self.close_menu();
        } else {
            self.open_menu(crate::menu::MenuKind::File);
        }
    }

    fn open_menu_for_hotkey(&mut self, key_code: KeyCode) -> bool {
        let Some(kind) = menu_kind_for_hotkey(key_code) else {
            return false;
        };

        self.open_menu(kind);
        true
    }

    fn open_next_menu(&mut self) {
        let next_kind = self
            .active_menu
            .as_ref()
            .map(|menu| next_menu_kind(menu.kind()))
            .unwrap_or(crate::menu::MenuKind::File);

        self.active_menu = Some(MenuState::with_selected(next_kind, 0));
    }

    fn open_previous_menu(&mut self) {
        let previous_kind = self
            .active_menu
            .as_ref()
            .map(|menu| previous_menu_kind(menu.kind()))
            .unwrap_or(crate::menu::MenuKind::File);

        self.active_menu = Some(MenuState::with_selected(previous_kind, 0));
    }

'''
    return text[:index] + methods + text[index:]


def patch_apply_menu_left_right(text: str) -> str:
    start, end = find_brace_span(text, "fn apply_app_command")
    block = text[start:end]

    if "AppCommand::MenuLeft" not in block:
        marker = '''        AppCommand::MenuDown => {
            state.move_menu_down();
            KeyHandling::Handled
        }
'''
        replacement = marker + '''
        AppCommand::MenuLeft => {
            state.open_previous_menu();
            KeyHandling::Handled
        }

        AppCommand::MenuRight => {
            state.open_next_menu();
            KeyHandling::Handled
        }
'''
        if marker not in block:
            raise SystemExit("Could not find MenuDown branch")
        block = block.replace(marker, replacement, 1)

    return text[:start] + block + text[end:]


def patch_handle_alt_menu_activation(text: str) -> str:
    start, end = find_brace_span(text, "fn handle_key_press")
    block = text[start:end]

    if "key.modifiers.contains(KeyModifiers::ALT)" in block:
        return text

    marker = "    if state.a3d_file_picker.is_some() {"
    insert = '''    if key.modifiers.contains(KeyModifiers::ALT) {
        if state.active_menu.is_some() {
            state.toggle_menu_bar();
            return KeyHandling::Handled;
        }

        if state.open_menu_for_hotkey(key_code) {
            return KeyHandling::Handled;
        }

        state.open_menu(crate::menu::MenuKind::File);
        return KeyHandling::Handled;
    }

'''
    if marker not in block:
        raise SystemExit("Could not find file picker block in handle_key_press")

    block = block.replace(marker, insert + marker, 1)

    return text[:start] + block + text[end:]


def patch_debug_console_menu_hotkeys(text: str) -> str:
    start, end = find_brace_span(text, "fn handle_key_press")
    block = text[start:end]

    old = '''            KeyCode::Char('c') | KeyCode::Char('C') => {
                return apply_app_command(state, AppCommand::OpenMenu(crate::menu::MenuKind::Control));
            }
'''
    new = '''            KeyCode::Char('f') | KeyCode::Char('F') => {
                return apply_app_command(state, AppCommand::OpenMenu(crate::menu::MenuKind::File));
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                return apply_app_command(state, AppCommand::OpenMenu(crate::menu::MenuKind::Control));
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                return apply_app_command(state, AppCommand::OpenMenu(crate::menu::MenuKind::Physics));
            }
'''
    if old in block and "KeyCode::Char('p')" not in block:
        block = block.replace(old, new, 1)

    return text[:start] + block + text[end:]


def main() -> None:
    command = COMMAND.read_text()
    command = patch_command_enum(command)
    command = patch_menu_command_for_key(command)
    command = patch_file_physics_hotkeys_in_keymaps(command)
    COMMAND.write_text(command)

    menu = MENU.read_text()
    menu = patch_menu_model(menu)
    MENU.write_text(menu)

    app = APP.read_text()
    app = patch_app_imports(app)
    app = insert_menu_kind_helpers(app)
    app = insert_appstate_menu_methods(app)
    app = patch_apply_menu_left_right(app)
    app = patch_handle_alt_menu_activation(app)
    app = patch_debug_console_menu_hotkeys(app)
    APP.write_text(app)

    print("Added Alt-style menu bar activation and left/right top-level menu navigation.")
    print("Changed File hotkey to f and Physics hotkey to p.")


if __name__ == "__main__":
    main()
