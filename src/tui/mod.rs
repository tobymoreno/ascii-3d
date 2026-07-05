use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Widget},
};

use crate::{
    canvas::Canvas,
    menu::{MenuKind, MenuState},
};

pub struct FilePickerView<'a> {
    pub title: &'a str,
    pub current_dir: &'a str,
    pub entries: &'a [String],
    pub selected: usize,
    pub error: Option<&'a str>,
}

const MENU_KINDS: &[MenuKind] = &[
    MenuKind::File,
    MenuKind::Scenes,
    MenuKind::Camera,
    MenuKind::World,
    MenuKind::Glyphs,
    MenuKind::Physics,
    MenuKind::Debug,
    MenuKind::Help,
];

pub fn draw(
    frame: &mut Frame<'_>,
    scene_canvas: &Canvas,
    camera_viewport_canvas: Option<&Canvas>,
    active_menu: Option<&MenuState>,
    debug_popup_lines: Option<&[String]>,
    frame_timing_lines: Option<&[String]>,
    file_picker_view: Option<FilePickerView<'_>>,
) {
    let area = frame.area();

    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    draw_menu_bar(frame, shell[0], active_menu.map(MenuState::kind));

    if let Some(camera_viewport_canvas) = camera_viewport_canvas {
        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(18)])
            .split(shell[1]);

        draw_scene(frame, scene_canvas, content[0]);
        draw_camera_viewport_block(frame, camera_viewport_canvas, content[1]);
    } else {
        draw_scene(frame, scene_canvas, shell[1]);
    }

    if let Some(lines) = debug_popup_lines {
        draw_debug_popup(
            frame,
            lines,
            top_right_rect(50, lines.len() as u16 + 6, area),
        );
    }

    if let Some(lines) = frame_timing_lines {
        draw_frame_timing(
            frame,
            lines,
            top_left_rect(44, lines.len() as u16 + 2, area),
        );
    }

    if let Some(file_picker_view) = file_picker_view {
        draw_file_picker(frame, file_picker_view, centered_rect(72, 22, area));
    }

    if let Some(menu) = active_menu {
        draw_menu_popup(
            frame,
            menu,
            centered_rect(54, menu.kind().items().len() as u16 + 4, area),
        );
    }
}

fn draw_menu_bar(frame: &mut Frame<'_>, area: Rect, active_menu_kind: Option<MenuKind>) {
    let titles = MENU_KINDS
        .iter()
        .map(|menu| {
            let label = if Some(*menu) == active_menu_kind {
                format!(" ▶ {}:{} ◀ ", menu.title(), menu.hotkey())
            } else {
                format!(" {}:{} ", menu.title(), menu.hotkey())
            };

            Line::from(vec![Span::raw(label)])
        })
        .collect::<Vec<_>>();

    let selected = active_menu_kind
        .and_then(|active| MENU_KINDS.iter().position(|menu| *menu == active))
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .divider(" ")
        .select(selected)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn draw_scene(frame: &mut Frame<'_>, scene_canvas: &Canvas, area: Rect) {
    frame.render_widget(
        CanvasWidget {
            canvas: scene_canvas,
        },
        area,
    );
}

struct CanvasWidget<'a> {
    canvas: &'a Canvas,
}

impl Widget for CanvasWidget<'_> {
    fn render(self, area: Rect, buffer: &mut ratatui::buffer::Buffer) {
        self.canvas.render_to_ratatui_buffer(area, buffer);
    }
}

fn draw_camera_viewport_block(frame: &mut Frame<'_>, canvas: &Canvas, area: Rect) {
    let block = Block::default()
        .title("Camera3D viewport")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(CanvasWidget { canvas }, inner);
}

fn draw_frame_timing(frame: &mut Frame<'_>, lines: &[String], area: Rect) {
    let text = lines.join("\n");
    let panel = Paragraph::new(text).block(Block::default().title("FPS").borders(Borders::ALL));

    frame.render_widget(Clear, area);
    frame.render_widget(panel, area);
}

fn top_left_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x,
        y: area.y.saturating_add(1),
        width: width.min(area.width),
        height: height.min(area.height.saturating_sub(1)),
    }
}

fn draw_debug_popup(frame: &mut Frame<'_>, lines: &[String], area: Rect) {
    let mut popup_lines = lines
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect::<Vec<_>>();

    popup_lines.push(Line::from(""));
    popup_lines.push(Line::from(vec![
        Span::raw("Press "),
        Span::styled("[ OK ]", Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw("  Enter/o/Esc"),
    ]));

    let popup = Paragraph::new(Text::from(popup_lines)).block(
        Block::default()
            .title("LoadedA3d debug")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn top_right_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect {
        x: area.x + area.width.saturating_sub(width + 2),
        y: area.y + 2,
        width,
        height,
    }
}

fn draw_file_picker(frame: &mut Frame<'_>, view: FilePickerView<'_>, area: Rect) {
    let items = view
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let selector = if index == view.selected { ">" } else { " " };
            ListItem::new(Line::from(format!("{selector} {entry}")))
        })
        .collect::<Vec<_>>();

    let help = match view.error {
        Some(error) => format!("Enter=open/load  Backspace=parent  Esc=cancel  ERROR: {error}"),
        None => "Enter=open/load  Backspace=parent  Esc=cancel".to_string(),
    };

    let list = List::new(items).block(
        Block::default()
            .title(Line::from(vec![
                Span::styled(view.title, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("  {}", view.current_dir)),
            ]))
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(list, area);

    let help_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(4),
        height: 1,
    };
    frame.render_widget(Paragraph::new(help), help_area);
}

fn draw_menu_popup(frame: &mut Frame<'_>, menu: &MenuState, area: Rect) {
    let items = menu
        .kind()
        .items()
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let selector = if index == menu.selected_index() {
                ">"
            } else {
                " "
            };
            let placeholder = if item.placeholder {
                " (placeholder)"
            } else {
                ""
            };

            ListItem::new(Line::from(format!(
                "{selector} {}{placeholder}",
                item.label
            )))
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .title(Line::from(vec![
                Span::styled(
                    menu.kind().title(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" menu [{}]", menu.kind().hotkey())),
            ]))
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(list, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;

    Rect {
        x,
        y,
        width,
        height,
    }
}
