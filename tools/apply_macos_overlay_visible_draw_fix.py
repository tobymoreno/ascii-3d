#!/usr/bin/env python3
from pathlib import Path

def patch_macos_overlay() -> None:
    path = Path("src/graphics/macos_overlay.rs")
    text = path.read_text()

    if "const OVERLAY_WINDOW_LEVEL: i64 = 1_000;" not in text:
        text = text.replace(
            "const DEMO_SECONDS: u64 = 20;\n",
            """const DEMO_SECONDS: u64 = 20;
const CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
const FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
const PANEL_COLLECTION_BEHAVIOR: u64 = CAN_JOIN_ALL_SPACES | FULL_SCREEN_AUXILIARY;
const OVERLAY_WINDOW_LEVEL: i64 = 1_000;
""",
            1,
        )

    text = text.replace(
        "        let _: () = msg_send![window, setLevel: 3i64];\n",
        """        let _: () = msg_send![window, setLevel: OVERLAY_WINDOW_LEVEL];
        let _: () = msg_send![window, setCollectionBehavior: PANEL_COLLECTION_BEHAVIOR];
""",
        1,
    )

    text = text.replace(
        """        while start.elapsed() < Duration::from_secs(DEMO_SECONDS) {
            let _: () = msg_send![view, setNeedsDisplay: YES];
            let _: () = msg_send![view, displayIfNeeded];
            let _: () = msg_send![window, displayIfNeeded];

            thread::sleep(Duration::from_millis(16));
        }
""",
        """        while start.elapsed() < Duration::from_secs(DEMO_SECONDS) {
            let _: () = msg_send![view, setNeedsDisplay: YES];
            let _: () = msg_send![view, display];
            let _: () = msg_send![window, display];

            thread::sleep(Duration::from_millis(16));
        }
""",
        1,
    )

    path.write_text(text)

def main() -> None:
    patch_macos_overlay()
    print("Applied macOS overlay visible draw fix.")

if __name__ == "__main__":
    main()
