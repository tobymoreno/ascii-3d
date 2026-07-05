#!/usr/bin/env python3
from pathlib import Path

def patch_cargo() -> None:
    path = Path("Cargo.toml")
    text = path.read_text()

    text = text.replace('cocoa = "0.26"', 'cocoa = "0.25"')

    lint_block = """[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(feature, values("cargo-clippy"))'] }
"""
    if "[lints.rust]" not in text:
        text = text.rstrip() + "\n\n" + lint_block

    path.write_text(text)

def patch_macos_overlay() -> None:
    path = Path("src/graphics/macos_overlay.rs")
    text = path.read_text()

    if not text.startswith("#![allow(unexpected_cfgs)]"):
        text = "#![allow(unexpected_cfgs)]\n#![allow(unsafe_op_in_unsafe_fn)]\n\n" + text
    elif "#![allow(unsafe_op_in_unsafe_fn)]" not in text[:120]:
        text = text.replace(
            "#![allow(unexpected_cfgs)]\n",
            "#![allow(unexpected_cfgs)]\n#![allow(unsafe_op_in_unsafe_fn)]\n",
            1,
        )

    text = text.replace(
        """use objc::{
    class, msg_send, sel, sel_impl,
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
""",
        """use objc::{class, msg_send, sel, sel_impl};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
""",
    )

    text = text.replace(
        """use objc::{
    class, msg_send, sel,
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
""",
        """use objc::{class, msg_send, sel, sel_impl};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
""",
    )

    path.write_text(text)

def main() -> None:
    patch_cargo()
    patch_macos_overlay()
    print("Applied Secure Password style macOS overlay fix.")

if __name__ == "__main__":
    main()
