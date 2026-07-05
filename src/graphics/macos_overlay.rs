use std::error::Error;

pub fn run_transparent_overlay_demo() -> Result<(), Box<dyn Error>> {
    eprintln!(
        "The experimental Cocoa primitive overlay is parked. Use: cargo run --bin raylib_overlay_demo"
    );

    Ok(())
}
