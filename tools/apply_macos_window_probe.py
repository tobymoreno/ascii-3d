#!/usr/bin/env python3
from pathlib import Path

MACOS_WINDOW_PROBE = r"""use std::{error::Error, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowBounds {
    app_name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("macos_window_probe error: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
fn run() -> Result<(), Box<dyn Error>> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(
            r#"tell application "System Events"
    set frontApp to first application process whose frontmost is true
    set appName to name of frontApp
    set frontWindow to front window of frontApp
    set windowPosition to position of frontWindow
    set windowSize to size of frontWindow
    set outputText to appName & "|" & (item 1 of windowPosition as text) & "|" & (item 2 of windowPosition as text) & "|" & (item 1 of windowSize as text) & "|" & (item 2 of windowSize as text)
    return outputText
end tell"#,
        )
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "osascript failed. macOS may require Accessibility permission for your terminal app. stderr: {stderr}"
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let bounds = parse_window_bounds(stdout.trim())?;

    println!("front_app={}", bounds.app_name);
    println!("x={}", bounds.x);
    println!("y={}", bounds.y);
    println!("width={}", bounds.width);
    println!("height={}", bounds.height);

    println!();
    println!(
        "overlay_args=--x {} --y {} --width {} --height {}",
        bounds.x, bounds.y, bounds.width, bounds.height
    );

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run() -> Result<(), Box<dyn Error>> {
    println!("macos_window_probe is only implemented on macOS.");
    Ok(())
}

#[cfg(target_os = "macos")]
fn parse_window_bounds(value: &str) -> Result<WindowBounds, Box<dyn Error>> {
    let parts: Vec<&str> = value.split('|').collect();

    if parts.len() != 5 {
        return Err(format!("unexpected osascript output: {value:?}").into());
    }

    Ok(WindowBounds {
        app_name: parts[0].to_string(),
        x: parts[1].parse()?,
        y: parts[2].parse()?,
        width: parts[3].parse()?,
        height: parts[4].parse()?,
    })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::{parse_window_bounds, WindowBounds};

    #[test]
    fn parses_window_bounds_output() {
        let parsed = parse_window_bounds("Ghostty|40|80|1320|760").unwrap();

        assert_eq!(
            parsed,
            WindowBounds {
                app_name: "Ghostty".to_string(),
                x: 40,
                y: 80,
                width: 1320,
                height: 760,
            }
        );
    }
}
"""

def write_probe() -> None:
    path = Path("src/bin/macos_window_probe.rs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(MACOS_WINDOW_PROBE)

def main() -> None:
    write_probe()
    print("Applied macOS window probe binary.")

if __name__ == "__main__":
    main()
