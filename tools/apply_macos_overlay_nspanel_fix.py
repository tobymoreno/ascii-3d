#!/usr/bin/env python3
from pathlib import Path

NEW_FILE = r'''#![allow(unexpected_cfgs)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    error::Error,
    ffi::c_void,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use cocoa::{
    appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSBackingStoreBuffered,
        NSScreen,
    },
    base::{id, nil, NO, YES},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize},
};
use objc::{class, msg_send, sel, sel_impl};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};

const DEMO_SECONDS: u64 = 20;

const NONACTIVATING_PANEL_STYLE_MASK: u64 = 1 << 7;

const CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
const FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
const PANEL_COLLECTION_BEHAVIOR: u64 = CAN_JOIN_ALL_SPACES | FULL_SCREEN_AUXILIARY;

const OVERLAY_WINDOW_LEVEL: i64 = 1_000;

type DispatchQueue = *mut c_void;

unsafe extern "C" {
    static mut _dispatch_main_q: c_void;

    fn dispatch_async_f(
        queue: DispatchQueue,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

struct DisplayTick {
    view: usize,
    panel: usize,
}

pub fn run_transparent_overlay_demo() -> Result<(), Box<dyn Error>> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        let screen_frame = main_screen_frame();
        let clear_color = clear_color();

        let view_class = primitive_overlay_view_class();
        let view: id = msg_send![view_class, alloc];
        let view: id = msg_send![view, initWithFrame: screen_frame];

        let panel = create_panel(screen_frame, view, clear_color);

        let _: () = msg_send![panel, orderFrontRegardless];

        schedule_overlay_updates(view, panel);
        schedule_exit();

        app.run();
    }

    Ok(())
}

unsafe fn main_screen_frame() -> NSRect {
    let screen = NSScreen::mainScreen(nil);

    if screen == nil {
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0))
    } else {
        screen.visibleFrame()
    }
}

unsafe fn clear_color() -> id {
    msg_send![class!(NSColor), clearColor]
}

unsafe fn create_panel(window_frame: NSRect, overlay_view: id, clear_color: id) -> id {
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
    let _: () = msg_send![content_view, addSubview: overlay_view];
    let _: () = msg_send![panel, setAlphaValue: 1.0f64];

    panel
}

fn schedule_exit() {
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(DEMO_SECONDS));
        std::process::exit(0);
    });
}

fn schedule_overlay_updates(view: id, panel: id) {
    let view = view as usize;
    let panel = panel as usize;

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(16));

        let tick = Box::new(DisplayTick { view, panel });
        let context = Box::into_raw(tick).cast::<c_void>();
        let main_queue = std::ptr::addr_of_mut!(_dispatch_main_q).cast::<c_void>();

        unsafe {
            dispatch_async_f(main_queue, context, apply_display_tick);
        }
    });
}

unsafe extern "C" fn apply_display_tick(context: *mut c_void) {
    if context.is_null() {
        return;
    }

    let tick = Box::from_raw(context.cast::<DisplayTick>());
    let view = tick.view as id;
    let panel = tick.panel as id;

    let _: () = msg_send![view, setNeedsDisplay: YES];
    let _: () = msg_send![view, display];
    let _: () = msg_send![panel, display];
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
'''

def patch() -> None:
    Path("src/graphics/macos_overlay.rs").write_text(NEW_FILE)

def main() -> None:
    patch()
    print("Applied macOS NSPanel overlay fix.")

if __name__ == "__main__":
    main()
