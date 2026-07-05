#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

#[cfg(target_os = "macos")]
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
