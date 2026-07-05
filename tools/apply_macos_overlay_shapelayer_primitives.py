#!/usr/bin/env python3
from pathlib import Path

NEW_FILE = r'''#![allow(unexpected_cfgs)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    error::Error,
    ffi::c_void,
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

struct DrawTick {
    root_layer: usize,
    width: f64,
    height: f64,
    started_at: Instant,
}

pub fn run_transparent_overlay_demo() -> Result<(), Box<dyn Error>> {
    show_shape_overlay();

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

unsafe fn create_root_view(frame: NSRect) -> (id, id) {
    let view: id = msg_send![class!(NSView), alloc];
    let view: id = msg_send![view, initWithFrame: frame];

    let _: () = msg_send![view, setWantsLayer: YES];

    let root_layer: id = msg_send![class!(CALayer), layer];
    let _: () = msg_send![root_layer, setFrame: frame];
    let _: () = msg_send![root_layer, setMasksToBounds: NO];

    let clear: id = msg_send![class!(NSColor), clearColor];
    let clear_cg: id = msg_send![clear, CGColor];
    let _: () = msg_send![root_layer, setBackgroundColor: clear_cg];

    let _: () = msg_send![view, setLayer: root_layer];

    (view, root_layer)
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

fn schedule_draw_ticks(root_layer: id, width: f64, height: f64) {
    let root_layer = root_layer as usize;
    let started_at = Instant::now();

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(16));

        let tick = Box::new(DrawTick {
            root_layer,
            width,
            height,
            started_at,
        });

        let context = Box::into_raw(tick).cast::<c_void>();
        let main_queue = std::ptr::addr_of_mut!(_dispatch_main_q).cast::<c_void>();

        unsafe {
            dispatch_async_f(main_queue, context, apply_draw_tick);
        }
    });
}

unsafe extern "C" fn apply_draw_tick(context: *mut c_void) {
    if context.is_null() {
        return;
    }

    let tick = Box::from_raw(context.cast::<DrawTick>());
    let root_layer = tick.root_layer as id;
    let elapsed = tick.started_at.elapsed().as_secs_f64();

    clear_sublayers(root_layer);
    draw_demo_layers(root_layer, tick.width, tick.height, elapsed);
}

unsafe fn clear_sublayers(root_layer: id) {
    let sublayers: id = msg_send![root_layer, sublayers];

    if sublayers == nil {
        return;
    }

    let count: usize = msg_send![sublayers, count];

    for _ in 0..count {
        let first: id = msg_send![sublayers, objectAtIndex: 0usize];
        let _: () = msg_send![first, removeFromSuperlayer];
    }
}

unsafe fn draw_demo_layers(root_layer: id, width: f64, height: f64, elapsed: f64) {
    let center_x = width / 2.0;
    let center_y = height / 2.0;

    add_rect_outline(root_layer, 80.0, 80.0, width - 160.0, height - 160.0, 0.0, 1.0, 0.35, 0.75);
    add_line(root_layer, 96.0, center_y, width - 96.0, center_y, 0.35, 0.65, 1.0, 0.55, 1.0);
    add_line(root_layer, center_x, 96.0, center_x, height - 96.0, 0.35, 0.65, 1.0, 0.55, 1.0);

    add_circle(root_layer, center_x, center_y, 92.0, 0.9, 0.9, 0.9, 0.7);

    let orbit_radius = 150.0;
    let orbit_x = center_x + elapsed.cos() * orbit_radius;
    let orbit_y = center_y + elapsed.sin() * orbit_radius;

    add_line(root_layer, center_x, center_y, orbit_x, orbit_y, 1.0, 0.85, 0.1, 0.85, 1.0);
    add_circle(root_layer, orbit_x, orbit_y, 28.0, 1.0, 0.85, 0.1, 0.9);

    let box_points = rotated_box_points(center_x, center_y, 190.0, 96.0, elapsed * 1.4);
    add_polyline(root_layer, &box_points, 1.0, 0.2, 0.15, 0.85, 1.0);

    add_filled_rect(root_layer, 104.0, height - 140.0, 14.0, 14.0, 0.0, 1.0, 0.35, 0.85);
    add_rect_outline(root_layer, 128.0, height - 154.0, 150.0, 48.0, 0.9, 0.9, 0.9, 0.75);
    add_line(root_layer, 140.0, height - 130.0, 266.0, height - 130.0, 0.0, 1.0, 0.35, 0.85, 1.0);
}

unsafe fn add_shape_layer(
    root_layer: id,
    path: id,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    line_width: f64,
    fill: bool,
) {
    let color: id = msg_send![
        class!(NSColor),
        colorWithCalibratedRed: red
        green: green
        blue: blue
        alpha: alpha
    ];
    let cg_color: id = msg_send![color, CGColor];

    let clear: id = msg_send![class!(NSColor), clearColor];
    let clear_cg: id = msg_send![clear, CGColor];

    let layer: id = msg_send![class!(CAShapeLayer), layer];
    let cg_path: id = msg_send![path, CGPath];

    let _: () = msg_send![layer, setPath: cg_path];

    if fill {
        let _: () = msg_send![layer, setFillColor: cg_color];
        let _: () = msg_send![layer, setStrokeColor: clear_cg];
    } else {
        let _: () = msg_send![layer, setFillColor: clear_cg];
        let _: () = msg_send![layer, setStrokeColor: cg_color];
        let _: () = msg_send![layer, setLineWidth: line_width];
    }

    let _: () = msg_send![root_layer, addSublayer: layer];
}

unsafe fn add_line(
    root_layer: id,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    line_width: f64,
) {
    let path: id = msg_send![class!(NSBezierPath), bezierPath];
    let _: () = msg_send![path, moveToPoint: NSPoint::new(x0, y0)];
    let _: () = msg_send![path, lineToPoint: NSPoint::new(x1, y1)];

    add_shape_layer(root_layer, path, red, green, blue, alpha, line_width, false);
}

unsafe fn add_rect_outline(
    root_layer: id,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithRect: rect];

    add_shape_layer(root_layer, path, red, green, blue, alpha, 1.0, false);
}

unsafe fn add_filled_rect(
    root_layer: id,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithRect: rect];

    add_shape_layer(root_layer, path, red, green, blue, alpha, 1.0, true);
}

unsafe fn add_circle(
    root_layer: id,
    center_x: f64,
    center_y: f64,
    radius: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    let rect = NSRect::new(
        NSPoint::new(center_x - radius, center_y - radius),
        NSSize::new(radius * 2.0, radius * 2.0),
    );
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithOvalInRect: rect];

    add_shape_layer(root_layer, path, red, green, blue, alpha, 1.0, false);
}

unsafe fn add_polyline(
    root_layer: id,
    points: &[(f64, f64)],
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    line_width: f64,
) {
    if points.len() < 2 {
        return;
    }

    let path: id = msg_send![class!(NSBezierPath), bezierPath];

    let first = points[0];
    let _: () = msg_send![path, moveToPoint: NSPoint::new(first.0, first.1)];

    for point in &points[1..] {
        let _: () = msg_send![path, lineToPoint: NSPoint::new(point.0, point.1)];
    }

    add_shape_layer(root_layer, path, red, green, blue, alpha, line_width, false);
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

fn show_shape_overlay() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        let screen_frame = main_screen_frame();
        let clear_color = clear_color();

        let (root_view, root_layer) = create_root_view(screen_frame);
        let panel = create_panel(screen_frame, root_view, clear_color);

        draw_demo_layers(root_layer, screen_frame.size.width, screen_frame.size.height, 0.0);

        let _: () = msg_send![panel, orderFrontRegardless];

        schedule_draw_ticks(root_layer, screen_frame.size.width, screen_frame.size.height);
        schedule_exit();

        app.run();
    }
}
'''

def patch() -> None:
    Path("src/graphics/macos_overlay.rs").write_text(NEW_FILE)

def main() -> None:
    patch()
    print("Applied macOS ShapeLayer primitives overlay patch.")

if __name__ == "__main__":
    main()
