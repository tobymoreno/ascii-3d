#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

#[path = "../graphics/mod.rs"]
mod graphics;

fn main() {
    if let Err(error) = graphics::run_primitives_demo() {
        eprintln!("os graphics demo error: {error}");
        std::process::exit(1);
    }
}
