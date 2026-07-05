ascii-3d Ratatui overlay KeyHandling fix

Fixes:

expected KeyHandling, found ()

The patch updates AppCommand::ShowOsGraphicsOverlay so it launches the raylib
overlay helper and then returns the same KeyHandling variant used by the
existing ToggleFrameTiming command arm.

Apply from project root:

unzip -l ~/Archive/ascii-3d-ratatui-overlay-keyhandling-fix-update.zip
unzip -o ~/Archive/ascii-3d-ratatui-overlay-keyhandling-fix-update.zip
python3 tools/apply_ratatui_overlay_keyhandling_fix.py
cargo fmt
cargo test
cargo run --bin ascii-3d

Manual helper test:

cargo run --bin raylib_overlay_demo
