use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

pub fn spawn_raylib_overlay_demo() {
    match spawn_overlay_process() {
        Ok(()) => {}
        Err(error) => eprintln!("failed to launch raylib overlay demo: {error}"),
    }
}

fn spawn_overlay_process() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = overlay_command();

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command.spawn()?;

    Ok(())
}

fn overlay_command() -> Command {
    if let Some(binary_path) = sibling_overlay_binary() {
        if binary_path.exists() {
            return Command::new(binary_path);
        }
    }

    let mut command = Command::new("cargo");
    command.args(["run", "--bin", "raylib_overlay_demo"]);

    command
}

fn sibling_overlay_binary() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    Some(current_exe.with_file_name(binary_name()))
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "raylib_overlay_demo.exe"
    } else {
        "raylib_overlay_demo"
    }
}

#[cfg(test)]
mod tests {
    use super::binary_name;

    #[test]
    fn overlay_binary_name_is_not_empty() {
        assert!(!binary_name().is_empty());
    }
}
