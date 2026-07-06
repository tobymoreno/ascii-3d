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


def patch_appstate_field(text: str) -> str:
    if "    confirm_exit: bool,\n" not in text:
        marker = "    show_debug_console: bool,\n"
        if marker not in text:
            raise SystemExit("Could not find show_debug_console field")
        text = text.replace(marker, marker + "    confirm_exit: bool,\n", 1)

    if "            confirm_exit: false,\n" not in text:
        marker = "            show_debug_console: false,\n"
        if marker not in text:
            raise SystemExit("Could not find show_debug_console initializer")
        text = text.replace(marker, marker + "            confirm_exit: false,\n", 1)

    return text


def patch_appstate_methods(text: str) -> str:
    if "fn open_exit_confirm" in text:
        return text

    marker = "    fn close_debug_console(&mut self) {"
    if marker not in text:
        raise SystemExit("Could not find close_debug_console insertion point")

    methods = '''    fn open_exit_confirm(&mut self) {
        self.confirm_exit = true;
        self.close_menu();
        self.close_a3d_file_picker();
    }

    fn close_exit_confirm(&mut self) {
        self.confirm_exit = false;
    }

'''
    return text.replace(marker, methods + marker, 1)


def patch_apply_quit(text: str) -> str:
    old = "        AppCommand::Quit => KeyHandling::Quit,\n"
    new = '''        AppCommand::Quit => {
            state.open_exit_confirm();
            KeyHandling::Handled
        }
'''
    if old in text:
        text = text.replace(old, new, 1)
    return text


def insert_exit_popup_lines(text: str) -> str:
    if "fn exit_confirm_popup_lines" in text:
        return text

    marker = "fn debug_console_popup_lines(state: &AppState) -> Option<Vec<String>> {"
    if marker not in text:
        raise SystemExit("Could not find debug_console_popup_lines insertion point")

    function = '''fn exit_confirm_popup_lines(state: &AppState) -> Option<Vec<String>> {
    state.confirm_exit.then(|| {
        vec![
            "Exit ascii-3d".to_string(),
            String::new(),
            "Do you really want to exit?".to_string(),
            String::new(),
            "Enter / y  = Yes, exit".to_string(),
            "Esc / n / c = Cancel".to_string(),
        ]
    })
}

'''
    return text.replace(marker, function + marker, 1)


def patch_render_popup_source(text: str) -> str:
    text = text.replace(
        "    let debug_popup_lines = debug_console_popup_lines(state);\n",
        "    let debug_popup_lines = exit_confirm_popup_lines(state).or_else(|| debug_console_popup_lines(state));\n",
        1,
    )
    return text


def patch_handle_key_press(text: str) -> str:
    start, end = find_brace_span(text, "fn handle_key_press")
    block = text[start:end]

    if "if state.confirm_exit {" in block:
        return text

    marker = "    let key_code = key.code;\n"
    if marker not in block:
        raise SystemExit("Could not find key_code line in handle_key_press")

    insert = '''
    if state.confirm_exit {
        match key_code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                return KeyHandling::Quit;
            }
            KeyCode::Esc
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Char('c')
            | KeyCode::Char('C') => {
                state.close_exit_confirm();
                return KeyHandling::Handled;
            }
            _ => {
                return KeyHandling::Handled;
            }
        }
    }

'''
    block = block.replace(marker, marker + insert, 1)
    return text[:start] + block + text[end:]


def main() -> None:
    text = APP.read_text()
    text = patch_appstate_field(text)
    text = patch_appstate_methods(text)
    text = patch_apply_quit(text)
    text = insert_exit_popup_lines(text)
    text = patch_render_popup_source(text)
    text = patch_handle_key_press(text)
    APP.write_text(text)

    print("Added exit confirmation popup.")
    print("Esc now opens confirm from normal workspace instead of immediately quitting.")
    print("Enter/y confirms exit; Esc/n/c cancels.")


if __name__ == "__main__":
    main()
