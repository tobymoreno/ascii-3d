#!/usr/bin/env python3
from pathlib import Path

BIN = Path("src/bin/ratatui_panel_demo.rs")


def main() -> None:
    if not BIN.exists():
        raise SystemExit("src/bin/ratatui_panel_demo.rs not found. Apply the first Ratatui panel demo patch first.")

    BIN.write_text(r'''use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Terminal,
};

type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

#[derive(Debug)]
struct DemoApp {
    debug_lines: Vec<String>,
    debug_scroll: usize,
    debug_panel_area: Rect,
}

impl DemoApp {
    fn new() -> Self {
        let mut debug_lines = Vec::new();

        debug_lines.push("debug console booted".to_string());
        debug_lines.push("mouse wheel over debug panel scrolls this log".to_string());
        debug_lines.push("arrow up/down also scrolls this log".to_string());
        debug_lines.push("g jumps to newest, G jumps to oldest".to_string());
        debug_lines.push("q / Esc quits".to_string());

        for index in 1..=120 {
            debug_lines.push(format!(
                "debug event #{index:03}: example scene/menu/key/object event payload"
            ));
        }

        Self {
            debug_lines,
            debug_scroll: 0,
            debug_panel_area: Rect::default(),
        }
    }

    fn visible_debug_rows(&self) -> usize {
        self.debug_panel_area.height.saturating_sub(2) as usize
    }

    fn max_debug_scroll(&self) -> usize {
        self.debug_lines
            .len()
            .saturating_sub(self.visible_debug_rows())
    }

    fn scroll_debug_up(&mut self, amount: usize) {
        self.debug_scroll = (self.debug_scroll + amount).min(self.max_debug_scroll());
    }

    fn scroll_debug_down(&mut self, amount: usize) {
        self.debug_scroll = self.debug_scroll.saturating_sub(amount);
    }

    fn jump_debug_to_newest(&mut self) {
        self.debug_scroll = 0;
    }

    fn jump_debug_to_oldest(&mut self) {
        self.debug_scroll = self.max_debug_scroll();
    }

    fn is_in_debug_panel(&self, column: u16, row: u16) -> bool {
        let area = self.debug_panel_area;

        column >= area.x
            && column < area.x.saturating_add(area.width)
            && row >= area.y
            && row < area.y.saturating_add(area.height)
    }

    fn scrollbar_position(&self) -> usize {
        self.max_debug_scroll().saturating_sub(self.debug_scroll)
    }
}

fn main() -> io::Result<()> {
    let mut terminal = start_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(&mut terminal)?;

    result
}

fn start_terminal() -> io::Result<AppTerminal> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        event::EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut AppTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()
}

fn run(terminal: &mut AppTerminal) -> io::Result<()> {
    let mut app = DemoApp::new();

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(8),
                    Constraint::Length(12),
                ])
                .split(frame.area());

            app.debug_panel_area = chunks[1];

            let main_panel = Paragraph::new(vec![
                Line::from(Span::styled(
                    "ascii-3d Ratatui scrollable debug panel test",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Top panel: placeholder for world/editor/camera rendering."),
                Line::from(""),
                Line::from("Debug controls: mouse wheel over debug panel, Up/Down, PageUp/PageDown, g/G."),
                Line::from(""),
                Line::from(format!(
                    "debug_scroll={} max_scroll={} log_lines={}",
                    app.debug_scroll,
                    app.max_debug_scroll(),
                    app.debug_lines.len()
                )),
                Line::from(""),
                Line::from("Press q or Esc to quit."),
            ])
            .block(Block::default().title("Main Area").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

            let visible_rows = app.visible_debug_rows();
            let max_scroll = app.max_debug_scroll();
            let start = app
                .debug_lines
                .len()
                .saturating_sub(visible_rows)
                .saturating_sub(app.debug_scroll);
            let end = (start + visible_rows).min(app.debug_lines.len());

            let debug_text: Vec<Line> = app.debug_lines[start..end]
                .iter()
                .map(|line| Line::from(line.as_str()))
                .collect();

            let debug_panel = Paragraph::new(debug_text)
                .block(
                    Block::default()
                        .title(format!(
                            "Debug Console [{}/{}]",
                            app.scrollbar_position(),
                            max_scroll
                        ))
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false });

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state =
                ScrollbarState::new(max_scroll.max(1)).position(app.scrollbar_position());

            frame.render_widget(main_panel, chunks[0]);
            frame.render_widget(debug_panel, chunks[1]);
            frame.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
        })?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                    KeyCode::Up => app.scroll_debug_up(1),
                    KeyCode::Down => app.scroll_debug_down(1),
                    KeyCode::PageUp => app.scroll_debug_up(8),
                    KeyCode::PageDown => app.scroll_debug_down(8),
                    KeyCode::Char('g') => app.jump_debug_to_newest(),
                    KeyCode::Char('G') => app.jump_debug_to_oldest(),
                    _ => {}
                },
                Event::Mouse(mouse) => {
                    if app.is_in_debug_panel(mouse.column, mouse.row) {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => app.scroll_debug_up(3),
                            MouseEventKind::ScrollDown => app.scroll_debug_down(3),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
''')

    print("Updated ratatui_panel_demo with scrollable debug log and mouse wheel support.")


if __name__ == "__main__":
    main()
