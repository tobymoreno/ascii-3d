use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Tabs, Widget},
};

use crate::{
    canvas::Canvas,
    menu::{MenuKind, MenuState},
};

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
