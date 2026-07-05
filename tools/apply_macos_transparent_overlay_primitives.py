#!/usr/bin/env python3
from pathlib import Path

def patch_cargo() -> None:
    path = Path("Cargo.toml")
    text = path.read_text()

    target_header = "[target.'cfg(target_os = \"macos\")'.dependencies]"
    if target_header not in text:
        text = text.rstrip() + f"\n\n{target_header}\n"

    if 'cocoa = ' not in text:
        text = text.replace(target_header + "\n", target_header + "\ncocoa = \"0.26\"\n", 1)

    if 'objc = ' not in text:
        text = text.replace(target_header + "\n", target_header + "\nobjc = \"0.2\"\n", 1)

    path.write_text(text)

def patch_graphics_mod() -> None:
    path = Path("src/graphics/mod.rs")
    text = path.read_text()

    if "pub mod macos_overlay;" not in text:
        text = text.rstrip() + '\n\n#[cfg(target_os = "macos")]\npub mod macos_overlay;\n'

    path.write_text(text)

def write_macos_overlay() -> None:
    path = Path("src/graphics/macos_overlay.rs")
    path.write_text(r'''use std::{
    error::Error,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use cocoa::{
    appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSBackingStoreBuffered,
        NSBorderlessWindowMask, NSScreen,
    },
    base::{id, nil, NO, YES},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize},
};
use objc::{
    class, msg_send, sel,
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};

const DEMO_SECONDS: u64 = 20;

pub fn run_transparent_overlay_demo() -> Result<(), Box<dyn Error>> {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        let screen = NSScreen::mainScreen(nil);
        if screen == nil {
            return Err("could not find main screen".into());
        }

        let frame = screen.visibleFrame();

        let window: id = msg_send![class!(NSWindow), alloc];
        let window: id = msg_send![
            window,
            initWithContentRect: frame
            styleMask: NSBorderlessWindowMask
            backing: NSBackingStoreBuffered
            defer: NO
        ];

        let clear_color: id = msg_send![class!(NSColor), clearColor];

        let _: () = msg_send![window, setOpaque: NO];
        let _: () = msg_send![window, setBackgroundColor: clear_color];
        let _: () = msg_send![window, setIgnoresMouseEvents: YES];
        let _: () = msg_send![window, setLevel: 3i64];

        let view_class = primitive_overlay_view_class();
        let view: id = msg_send![view_class, alloc];
        let view: id = msg_send![view, initWithFrame: frame];

        let _: () = msg_send![window, setContentView: view];
        let _: () = msg_send![window, orderFrontRegardless];

        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(DEMO_SECONDS) {
            let _: () = msg_send![view, setNeedsDisplay: YES];
            let _: () = msg_send![view, displayIfNeeded];
            let _: () = msg_send![window, displayIfNeeded];

            thread::sleep(Duration::from_millis(16));
        }

        let _: () = msg_send![window, close];
        let _: () = msg_send![pool, drain];
    }

    Ok(())
}

fn primitive_overlay_view_class() -> *const Class {
    static CLASS: OnceLock<usize> = OnceLock::new();

    let class_ptr = CLASS.get_or_init(|| {
        let superclass = class!(NSView);
        let mut declaration = ClassDecl::new("Ascii3dPrimitiveOverlayView", superclass)
            .expect("Ascii3dPrimitiveOverlayView should register once");

        unsafe {
            declaration.add_method(
                sel!(drawRect:),
                draw_rect as extern "C" fn(&Object, Sel, NSRect),
            );
        }

        declaration.register() as *const Class as usize
    });

    *class_ptr as *const Class
}

extern "C" fn draw_rect(_this: &Object, _selector: Sel, rect: NSRect) {
    static START: OnceLock<Instant> = OnceLock::new();

    let elapsed = START.get_or_init(Instant::now).elapsed().as_secs_f64();
    let width = rect.size.width;
    let height = rect.size.height;
    let center_x = width / 2.0;
    let center_y = height / 2.0;

    unsafe {
        draw_rect_outline(80.0, 80.0, width - 160.0, height - 160.0, 0.0, 1.0, 0.35, 0.75);
        draw_line(96.0, center_y, width - 96.0, center_y, 0.35, 0.65, 1.0, 0.55, 1.0);
        draw_line(center_x, 96.0, center_x, height - 96.0, 0.35, 0.65, 1.0, 0.55, 1.0);

        draw_circle(center_x, center_y, 92.0, 0.9, 0.9, 0.9, 0.7);

        let orbit_radius = 150.0;
        let orbit_x = center_x + elapsed.cos() * orbit_radius;
        let orbit_y = center_y + elapsed.sin() * orbit_radius;

        draw_line(center_x, center_y, orbit_x, orbit_y, 1.0, 0.85, 0.1, 0.85, 1.0);
        draw_circle(orbit_x, orbit_y, 28.0, 1.0, 0.85, 0.1, 0.9);

        let box_points = rotated_box_points(center_x, center_y, 190.0, 96.0, elapsed * 1.4);
        draw_polyline(&box_points, 1.0, 0.2, 0.15, 0.85, 1.0);

        fill_rect(104.0, height - 140.0, 14.0, 14.0, 0.0, 1.0, 0.35, 0.85);
        draw_rect_outline(128.0, height - 154.0, 150.0, 48.0, 0.9, 0.9, 0.9, 0.75);
        draw_line(140.0, height - 130.0, 266.0, height - 130.0, 0.0, 1.0, 0.35, 0.85, 1.0);
    }
}

unsafe fn set_stroke_color(red: f64, green: f64, blue: f64, alpha: f64) {
    let color: id = msg_send![
        class!(NSColor),
        colorWithCalibratedRed: red
        green: green
        blue: blue
        alpha: alpha
    ];
    let _: () = msg_send![color, setStroke];
}

unsafe fn set_fill_color(red: f64, green: f64, blue: f64, alpha: f64) {
    let color: id = msg_send![
        class!(NSColor),
        colorWithCalibratedRed: red
        green: green
        blue: blue
        alpha: alpha
    ];
    let _: () = msg_send![color, setFill];
}

unsafe fn draw_line(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    width: f64,
) {
    set_stroke_color(red, green, blue, alpha);

    let path: id = msg_send![class!(NSBezierPath), bezierPath];
    let _: () = msg_send![path, setLineWidth: width];
    let _: () = msg_send![path, moveToPoint: NSPoint::new(x0, y0)];
    let _: () = msg_send![path, lineToPoint: NSPoint::new(x1, y1)];
    let _: () = msg_send![path, stroke];
}

unsafe fn draw_rect_outline(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    set_stroke_color(red, green, blue, alpha);

    let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithRect: rect];
    let _: () = msg_send![path, setLineWidth: 1.0f64];
    let _: () = msg_send![path, stroke];
}

unsafe fn fill_rect(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    set_fill_color(red, green, blue, alpha);

    let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithRect: rect];
    let _: () = msg_send![path, fill];
}

unsafe fn draw_circle(
    center_x: f64,
    center_y: f64,
    radius: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    set_stroke_color(red, green, blue, alpha);

    let rect = NSRect::new(
        NSPoint::new(center_x - radius, center_y - radius),
        NSSize::new(radius * 2.0, radius * 2.0),
    );
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithOvalInRect: rect];
    let _: () = msg_send![path, setLineWidth: 1.0f64];
    let _: () = msg_send![path, stroke];
}

unsafe fn draw_polyline(
    points: &[(f64, f64)],
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    width: f64,
) {
    if points.len() < 2 {
        return;
    }

    set_stroke_color(red, green, blue, alpha);

    let path: id = msg_send![class!(NSBezierPath), bezierPath];
    let _: () = msg_send![path, setLineWidth: width];

    let first = points[0];
    let _: () = msg_send![path, moveToPoint: NSPoint::new(first.0, first.1)];

    for point in &points[1..] {
        let _: () = msg_send![path, lineToPoint: NSPoint::new(point.0, point.1)];
    }

    let _: () = msg_send![path, stroke];
}

fn rotated_box_points(
    center_x: f64,
    center_y: f64,
    width: f64,
    height: f64,
    angle: f64,
) -> Vec<(f64, f64)> {
    let half_width = width / 2.0;
    let half_height = height / 2.0;

    let corners = [
        (-half_width, -half_height),
        (half_width, -half_height),
        (half_width, half_height),
        (-half_width, half_height),
        (-half_width, -half_height),
    ];

    let cos_angle = angle.cos();
    let sin_angle = angle.sin();

    corners
        .iter()
        .map(|(x, y)| {
            let rotated_x = (x * cos_angle) - (y * sin_angle);
            let rotated_y = (x * sin_angle) + (y * cos_angle);

            (center_x + rotated_x, center_y + rotated_y)
        })
        .collect()
}
''')

def write_overlay_bin() -> None:
    path = Path("src/bin/os_graphics_overlay_demo.rs")
    path.parent.mkdir(parents=True, exist_ok=True)

    path.write_text(r'''#[cfg(target_os = "macos")]
#[path = "../graphics/mod.rs"]
mod graphics;

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = graphics::macos_overlay::run_transparent_overlay_demo() {
        eprintln!("macOS transparent overlay demo error: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("os_graphics_overlay_demo is currently implemented only for macOS.");
}
''')

def main() -> None:
    patch_cargo()
    patch_graphics_mod()
    write_macos_overlay()
    write_overlay_bin()
    print("Applied macOS transparent overlay primitives patch.")

if __name__ == "__main__":
    main()
