fn main() {
    if let Err(error) = ascii_3d::native_editor::run() {
        eprintln!("native editor failed: {error}");
        std::process::exit(1);
    }
}
