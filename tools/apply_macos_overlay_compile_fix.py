#!/usr/bin/env python3
from pathlib import Path

def patch_macos_overlay() -> None:
    path = Path("src/graphics/macos_overlay.rs")
    text = path.read_text()

    text = text.replace(
        """        NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSBackingStoreBuffered,
        NSBorderlessWindowMask, NSScreen,
""",
        """        NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSBackingStoreBuffered,
        NSScreen,
""",
    )

    text = text.replace(
        "class, msg_send, sel,\n",
        "class, msg_send, sel, sel_impl,\n",
    )

    text = text.replace(
        "            styleMask: NSBorderlessWindowMask\n",
        "            styleMask: 0u64\n",
    )

    path.write_text(text)

def main() -> None:
    patch_macos_overlay()
    print("Applied macOS overlay compile fix.")

if __name__ == "__main__":
    main()
