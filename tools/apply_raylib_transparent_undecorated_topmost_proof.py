#!/usr/bin/env python3
from pathlib import Path

path = Path("src/bin/raylib_overlay_demo.rs")
text = path.read_text()

old = """    let flags = (ConfigFlags::FLAG_WINDOW_TRANSPARENT as u32)
        | (ConfigFlags::FLAG_WINDOW_UNDECORATED as u32);
"""

new = """    let flags = (ConfigFlags::FLAG_WINDOW_TRANSPARENT as u32)
        | (ConfigFlags::FLAG_WINDOW_UNDECORATED as u32)
        | (ConfigFlags::FLAG_WINDOW_TOPMOST as u32);
"""

if old not in text:
    raise SystemExit("Could not find transparent + undecorated flag block to patch.")

text = text.replace(old, new, 1)
text = text.replace(
    '        "transparent + undecorated",',
    '        "transparent + undecorated + topmost",',
    1,
)

path.write_text(text)

print("Applied raylib transparent undecorated topmost proof patch.")
