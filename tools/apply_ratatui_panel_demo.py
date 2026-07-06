#!/usr/bin/env python3
from pathlib import Path

CARGO = Path("Cargo.toml")
BIN = Path("src/bin/ratatui_panel_demo.rs")


def ensure_dependency(text: str, name: str, version: str) -> str:
    if f"{name} =" in text:
        return text

    marker = "[dependencies]"
    index = text.find(marker)
    if index < 0:
        raise SystemExit("Could not find [dependencies] in Cargo.toml")

    insert_at = text.find("\n", index)
    if insert_at < 0:
        raise SystemExit("Could not find end of [dependencies] line in Cargo.toml")

    return text[:insert_at + 1] + f'{name} = "{version}"\n' + text[insert_at + 1:]


def patch_cargo() -> None:
    text = CARGO.read_text()
    text = ensure_dependency(text, "ratatui", "0.29")
    CARGO.write_text(text)


def write_demo_bin() -> None:
    BIN.parent.mkdir(parents=True, exist_ok=True)

    BIN.write_text(r'''use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

fn main() -> io::Result<()> {
    let mut terminal = start_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(&mut terminal)?;

    result
}

fn start_terminal() -> io::Result<AppTerminal> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut AppTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn run(terminal: &mut AppTerminal) -> io::Result<()> {
    let debug_lines = vec![
        "debug console booted",
        "raw key events will eventually show here",
        "menu events will eventually show here",
        "active scene events will eventually show here",
        "world-space events will eventually show here",
        "camera events will eventually show here",
        "light events will eventually show here",
        "object events will eventually show here",
    ];

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(8),
                    Constraint::Length(10),
                ])
                .split(frame.area());

            let main_panel = Paragraph::new(vec![
                Line::from(Span::styled(
                    "ascii-3d Ratatui layout test",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Top panel: placeholder for world/editor/camera rendering."),
                Line::from(""),
                Line::from("Press q or Esc to quit."),
            ])
            .block(Block::default().title("Main Area").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

            let debug_text: Vec<Line> = debug_lines
                .iter()
                .map(|line| Line::from(*line))
                .collect();

            let debug_panel = Paragraph::new(debug_text)
                .block(Block::default().title("Debug Console").borders(Borders::ALL))
                .wrap(Wrap { trim: false });

            frame.render_widget(main_panel, chunks[0]);
            frame.render_widget(debug_panel, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}
''')


def main() -> None:
    patch_cargo()
    write_demo_bin()

    print("Added ratatui_panel_demo binary.")
    print("Run with: cargo run --bin ratatui_panel_demo")


if __name__ == "__main__":
    main()
