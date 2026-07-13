use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Tabs},
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

    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(1)])
        .split(area);

    frame.render_widget(tabs, header[0]);
    frame.render_widget(Paragraph::new(status), header[1]);
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
