#!/usr/bin/env python3
from pathlib import Path

NEW_FILE = r'''#![allow(unexpected_cfgs)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    error::Error,
    thread,
    time::Duration,
};

use cocoa::{
    appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSBackingStoreBuffered,
        NSScreen, NSTextField,
    },
    base::{id, nil, NO, YES},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString},
};
use objc::{class, msg_send, sel, sel_impl};

const DEMO_SECONDS: u64 = 20;

const NONACTIVATING_PANEL_STYLE_MASK: u64 = 1 << 7;

const CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
const FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
const PANEL_COLLECTION_BEHAVIOR: u64 = CAN_JOIN_ALL_SPACES | FULL_SCREEN_AUXILIARY;

const OVERLAY_WINDOW_LEVEL: i64 = 1_000;

const PANEL_WIDTH: f64 = 620.0;
const PANEL_HEIGHT: f64 = 110.0;
const TOP_MARGIN: f64 = 72.0;
const LEFT_MARGIN: f64 = 72.0;

pub fn run_transparent_overlay_demo() -> Result<(), Box<dyn Error>> {
    show_text_overlay("ASCII-3D OVERLAY TEST");

    Ok(())
}

fn nsstring(value: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(value) }
}

unsafe fn main_screen_frame() -> NSRect {
    let screen = NSScreen::mainScreen(nil);

    if screen == nil {
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0))
    } else {
        screen.visibleFrame()
    }
}

unsafe fn appkit_colors() -> (id, id) {
    let color_class = class!(NSColor);

    let clear: id = msg_send![color_class, clearColor];
    let green: id = msg_send![color_class, systemGreenColor];

    (clear, green)
}

unsafe fn overlay_font() -> id {
    let font_class = class!(NSFont);
    let font: id = msg_send![font_class, userFixedPitchFontOfSize: 34.0f64];

    if font == nil {
        msg_send![font_class, boldSystemFontOfSize: 34.0f64]
    } else {
        font
    }
}

unsafe fn create_text_label(message: &str, font: id, text_color: id, clear_color: id) -> id {
    let label: id = msg_send![class!(NSTextField), alloc];
    let frame = NSRect::new(
        NSPoint::new(20.0, 18.0),
        NSSize::new(PANEL_WIDTH - 40.0, PANEL_HEIGHT - 36.0),
    );
    let label: id = msg_send![label, initWithFrame: frame];

    let ns_message = nsstring(message);

    let _: () = msg_send![label, setStringValue: ns_message];
    let _: () = msg_send![label, setFont: font];
    let _: () = msg_send![label, setTextColor: text_color];
    let _: () = msg_send![label, setBackgroundColor: clear_color];
    let _: () = msg_send![label, setDrawsBackground: NO];
    let _: () = msg_send![label, setBordered: NO];
    let _: () = msg_send![label, setEditable: NO];
    let _: () = msg_send![label, setSelectable: NO];
    let _: () = msg_send![label, setAlignment: 0u64];

    label
}

unsafe fn create_panel(window_frame: NSRect, label: id, clear_color: id) -> id {
    let panel_class = class!(NSPanel);
    let panel: id = msg_send![panel_class, alloc];

    let panel: id = msg_send![
        panel,
        initWithContentRect: window_frame
        styleMask: NONACTIVATING_PANEL_STYLE_MASK
        backing: NSBackingStoreBuffered
        defer: NO
    ];

    let _: () = msg_send![panel, setOpaque: NO];
    let _: () = msg_send![panel, setBackgroundColor: clear_color];
    let _: () = msg_send![panel, setIgnoresMouseEvents: YES];
    let _: () = msg_send![panel, setHidesOnDeactivate: NO];

    let _: () = msg_send![panel, setLevel: OVERLAY_WINDOW_LEVEL];
    let _: () = msg_send![panel, setCollectionBehavior: PANEL_COLLECTION_BEHAVIOR];

    let content_view: id = msg_send![panel, contentView];
    let _: () = msg_send![content_view, addSubview: label];
    let _: () = msg_send![panel, setAlphaValue: 1.0f64];

    panel
}

fn schedule_exit() {
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(DEMO_SECONDS));
        std::process::exit(0);
    });
}

fn show_text_overlay(message: &str) {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        let screen_frame = main_screen_frame();
        let panel_origin_y =
            screen_frame.origin.y + screen_frame.size.height - PANEL_HEIGHT - TOP_MARGIN;

        let panel_frame = NSRect::new(
            NSPoint::new(screen_frame.origin.x + LEFT_MARGIN, panel_origin_y),
            NSSize::new(PANEL_WIDTH, PANEL_HEIGHT),
        );

        let (clear_color, text_color) = appkit_colors();
        let label = create_text_label(message, overlay_font(), text_color, clear_color);
        let panel = create_panel(panel_frame, label, clear_color);

        let _: () = msg_send![panel, orderFrontRegardless];

        schedule_exit();

        app.run();
    }
}
'''

def patch() -> None:
    Path("src/graphics/macos_overlay.rs").write_text(NEW_FILE)

def main() -> None:
    patch()
    print("Applied macOS transparent text overlay proof patch.")

if __name__ == "__main__":
    main()
