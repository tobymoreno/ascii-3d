use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
};

use super::EditorEvent;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MenuId(pub String);

impl MenuId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Action {
        id: String,
        label: String,
        enabled: bool,
        shortcut: Option<String>,
    },
    Separator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuDefinition {
    pub id: MenuId,
    pub label: String,
    pub hotkey: Option<char>,
    pub entries: Vec<MenuEntry>,
}

#[derive(Clone, Debug, Default)]
pub struct MenuBarState {
    focused: bool,
    selected_menu: usize,
}

impl MenuBarState {
    pub const fn focused(&self) -> bool {
        self.focused
    }

    pub const fn selected_menu(&self) -> usize {
        self.selected_menu
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn with_selected(focused: bool, selected_menu: usize) -> Self {
        Self {
            focused,
            selected_menu,
        }
    }

    pub fn focus_menu(&mut self, id: &str, definitions: &[MenuDefinition]) -> bool {
        let Some(index) = definitions
            .iter()
            .position(|definition| definition.id.0 == id)
        else {
            return false;
        };
        self.selected_menu = index;
        self.focus();
        true
    }

    pub fn blur(&mut self) {
        self.focused = false;
    }

    pub fn handle_key(
        &mut self,
        code: KeyCode,
        definitions: &[MenuDefinition],
    ) -> Option<EditorEvent> {
        if definitions.is_empty() {
            self.selected_menu = 0;
            return None;
        }

        match code {
            KeyCode::Esc | KeyCode::Tab => self.blur(),
            KeyCode::Left => {
                self.selected_menu = if self.selected_menu == 0 {
                    definitions.len() - 1
                } else {
                    self.selected_menu - 1
                };
            }
            KeyCode::Right => {
                self.selected_menu = (self.selected_menu + 1) % definitions.len();
            }
            KeyCode::Enter => {
                let definition = &definitions[self.selected_menu];
                return Some(EditorEvent::MenuOpened {
                    menu_id: definition.id.clone(),
                });
            }
            _ => return None,
        }

        None
    }
}

pub fn draw_menu_bar(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    definitions: &[MenuDefinition],
    state: &MenuBarState,
    status: &str,
) {
    let titles = definitions
        .iter()
        .map(|definition| Line::from(Span::raw(format!(" {} ", definition.label))))
        .collect::<Vec<_>>();

    let selected = state
        .selected_menu()
        .min(definitions.len().saturating_sub(1));
    let tabs = Tabs::new(titles)
        .divider(" ")
        .select(selected)
        .highlight_style(if state.focused() {
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });

    let status_width = header_status_width(area.width, status);
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(status_width)])
        .split(area);

    frame.render_widget(tabs, header[0]);
    frame.render_widget(
        Paragraph::new(status).alignment(Alignment::Right),
        header[1],
    );
}

pub fn menu_action_count(definition: &MenuDefinition) -> usize {
    definition
        .entries
        .iter()
        .filter(|entry| matches!(entry, MenuEntry::Action { .. }))
        .count()
}

pub fn draw_menu_popup(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    definition: &MenuDefinition,
    selected_action: usize,
    detail: Option<&str>,
) {
    let mut action_index = 0usize;
    let mut items = definition
        .entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Separator => ListItem::new(Line::from("  -------------------------")),
            MenuEntry::Action { label, enabled, .. } => {
                let current = action_index;
                action_index += 1;
                let selector = if current == selected_action { ">" } else { " " };
                let suffix = if *enabled { "" } else { " (disabled)" };
                let item = ListItem::new(Line::from(format!("{selector} {label}{suffix}")));
                if current == selected_action {
                    item.style(
                        Style::default()
                            .add_modifier(Modifier::REVERSED)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    item
                }
            }
        })
        .collect::<Vec<_>>();

    if let Some(detail) = detail {
        items.push(ListItem::new(Line::from(format!("  {detail}"))));
    }

    let popup = List::new(items).block(
        Block::default()
            .title(format!(
                " {}  Up/Down select  Enter open  Esc close ",
                definition.label
            ))
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn header_status_width(area_width: u16, status: &str) -> u16 {
    status.chars().count().min(usize::from(area_width)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definitions() -> Vec<MenuDefinition> {
        vec![
            MenuDefinition {
                id: MenuId::new("file"),
                label: "File".to_string(),
                hotkey: Some('f'),
                entries: vec![],
            },
            MenuDefinition {
                id: MenuId::new("objects"),
                label: "Objects".to_string(),
                hotkey: Some('o'),
                entries: vec![],
            },
        ]
    }

    #[test]
    fn status_width_uses_text_width_without_exceeding_header() {
        assert_eq!(header_status_width(80, "fps 30.1"), 8);
        assert_eq!(header_status_width(4, "fps 30.1"), 4);
    }

    #[test]
    fn popup_action_count_ignores_separators() {
        let definition = MenuDefinition {
            id: MenuId::new("file"),
            label: "File".to_string(),
            hotkey: Some('f'),
            entries: vec![
                MenuEntry::Action {
                    id: "open".to_string(),
                    label: "Open".to_string(),
                    enabled: true,
                    shortcut: None,
                },
                MenuEntry::Separator,
                MenuEntry::Action {
                    id: "exit".to_string(),
                    label: "Exit".to_string(),
                    enabled: true,
                    shortcut: None,
                },
            ],
        };
        assert_eq!(menu_action_count(&definition), 2);
    }

    #[test]
    fn menu_navigation_wraps() {
        let definitions = definitions();
        let mut state = MenuBarState::default();
        state.focus();
        state.handle_key(KeyCode::Left, &definitions);
        assert_eq!(state.selected_menu(), 1);
    }

    #[test]
    fn opening_menu_emits_contextual_id() {
        let definitions = definitions();
        let mut state = MenuBarState::default();
        state.focus();
        let event = state.handle_key(KeyCode::Enter, &definitions);
        assert_eq!(
            event,
            Some(EditorEvent::MenuOpened {
                menu_id: MenuId::new("file")
            })
        );
    }
}
