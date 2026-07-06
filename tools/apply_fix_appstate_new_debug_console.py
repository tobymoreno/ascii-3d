#!/usr/bin/env python3
from pathlib import Path
import re

APP = Path("src/app.rs")


BROKEN_BLOCK = '''            last_input_event_trace: None,
        }
                debug_console_lines: VecDeque::from([
                "debug console attached to main workspace".to_string(),
                "keys/menu/scene routing will be logged here".to_string(),
                "PageUp/PageDown scroll this debug console".to_string(),
            ]),
            debug_console_scroll: 0,
}'''

FIXED_BLOCK = '''            last_input_event_trace: None,
            debug_console_lines: VecDeque::from([
                "debug console attached to main workspace".to_string(),
                "keys/menu/scene routing will be logged here".to_string(),
                "PageUp/PageDown scroll this debug console".to_string(),
            ]),
            debug_console_scroll: 0,
        }
    }'''


def main() -> None:
    text = APP.read_text()

    if BROKEN_BLOCK in text:
        text = text.replace(BROKEN_BLOCK, FIXED_BLOCK, 1)
        APP.write_text(text)
        print("Fixed AppState::new debug console initializer placement.")
        return

    # Fallback for spacing variations.
    pattern = re.compile(
        r'''(?P<prefix>\s+last_input_event_trace:\s+None,\n)\s*}\s*\n\s*debug_console_lines:\s+VecDeque::from\(\[\n(?P<lines>.*?PageUp/PageDown scroll this debug console"\.to_string\(\),\n\s*\]\),\n)\s*debug_console_scroll:\s+0,\n\s*}''',
        re.DOTALL,
    )

    match = pattern.search(text)
    if not match:
        raise SystemExit("Could not find malformed AppState::new debug console initializer.")

    replacement = (
        match.group("prefix")
        + '            debug_console_lines: VecDeque::from([\n'
        + '                "debug console attached to main workspace".to_string(),\n'
        + '                "keys/menu/scene routing will be logged here".to_string(),\n'
        + '                "PageUp/PageDown scroll this debug console".to_string(),\n'
        + '            ]),\n'
        + '            debug_console_scroll: 0,\n'
        + '        }\n    }'
    )

    text = text[: match.start()] + replacement + text[match.end() :]
    APP.write_text(text)
    print("Fixed AppState::new debug console initializer placement via fallback pattern.")


if __name__ == "__main__":
    main()
