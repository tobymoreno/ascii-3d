#!/usr/bin/env python3
from pathlib import Path

APP = Path("src/app.rs")

BROKEN_1 = '''fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {
    // The old LoadedA3d auto-hide popup no longer uses the debug popup layer.
 else {
        false
    }
}
'''

BROKEN_2 = '''fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {
    // The old LoadedA3d auto-hide popup no longer uses the debug popup layer.
else {
        false
    }
}
'''

BROKEN_3 = '''fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {
    // The old LoadedA3d auto-hide popup no longer uses the debug popup layer.
    else {
        false
    }
}
'''

FIXED = '''fn dismiss_loaded_a3d_debug_popup(state: &mut AppState) -> bool {
    if is_loaded_a3d_debug_popup_visible(state) {
        state.loaded_a3d_debug_popup_until = None;
        true
    } else {
        false
    }
}
'''


def main() -> None:
    text = APP.read_text()

    replaced = False
    for broken in (BROKEN_1, BROKEN_2, BROKEN_3):
        if broken in text:
            text = text.replace(broken, FIXED, 1)
            replaced = True
            break

    if not replaced:
        start = text.find("fn dismiss_loaded_a3d_debug_popup")
        if start < 0:
            raise SystemExit("Could not find dismiss_loaded_a3d_debug_popup")

        next_fn = text.find("\nfn debug_console_popup_lines", start)
        if next_fn < 0:
            raise SystemExit("Could not find debug_console_popup_lines after dismiss_loaded_a3d_debug_popup")

        text = text[:start] + FIXED + text[next_fn + 1:]
        replaced = True

    # Ensure old popup does not render through debug popup source.
    text = text.replace(
        "    let debug_popup_lines =\n        debug_console_popup_lines(state).or_else(|| loaded_a3d_debug_popup_lines(state));\n",
        "    let debug_popup_lines = debug_console_popup_lines(state);\n",
    )
    text = text.replace(
        "    let debug_popup_lines = debug_console_popup_lines(state).or_else(|| loaded_a3d_debug_popup_lines(state));\n",
        "    let debug_popup_lines = debug_console_popup_lines(state);\n",
    )

    # Ensure close method exists.
    if "fn close_debug_console" not in text:
        marker = "    fn toggle_debug_console(&mut self) {"
        index = text.find(marker)
        if index < 0:
            raise SystemExit("Could not find toggle_debug_console method")
        close_method = '''    fn close_debug_console(&mut self) {
        self.show_debug_console = false;
    }

'''
        text = text[:index] + close_method + text[index:]

    APP.write_text(text)

    print("Hard-fixed dismiss_loaded_a3d_debug_popup dangling else.")
    print("Verify with:")
    print("  sed -n '2953,2968p' src/app.rs")


if __name__ == "__main__":
    main()
