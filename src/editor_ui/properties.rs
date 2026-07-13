use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem},
};

use super::{EditorAction, EditorEvent, EditorTarget, EventSource};

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyRow {
    Value {
        label: String,
        value: String,
    },
    Action {
        id: String,
        label: String,
        hint: Option<String>,
        enabled: bool,
        action: EditorAction,
    },
    Separator,
}

impl PropertyRow {
    pub const fn is_selectable(&self) -> bool {
        matches!(self, Self::Action { enabled: true, .. })
    }
}

#[derive(Clone, Debug, Default)]
pub struct PropertiesState {
    open: bool,
    target: Option<EditorTarget>,
    selected_action: usize,
}

impl PropertiesState {
    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn selected_action(&self) -> usize {
        self.selected_action
    }

    pub fn open(&mut self, target: EditorTarget) {
        self.open = true;
        self.target = Some(target);
        self.selected_action = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn target(&self) -> Option<&EditorTarget> {
        self.target.as_ref()
    }

    pub fn handle_key(&mut self, code: KeyCode, rows: &[PropertyRow]) -> Option<EditorEvent> {
        let action_count = rows.iter().filter(|row| row.is_selectable()).count();
        match code {
            KeyCode::Esc => {
                self.close();
                Some(EditorEvent::CloseRequested)
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if action_count > 0 {
                    self.selected_action = if self.selected_action == 0 {
                        action_count - 1
                    } else {
                        self.selected_action - 1
                    };
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if action_count > 0 {
                    self.selected_action = (self.selected_action + 1) % action_count;
                }
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let target = self.target.clone()?;
                let (_, row) = rows
                    .iter()
                    .filter(|row| row.is_selectable())
                    .enumerate()
                    .find(|(index, _)| *index == self.selected_action)?;
                let PropertyRow::Action { action, .. } = row else {
                    return None;
                };
                Some(EditorEvent::ActionRequested {
                    target,
                    action: action.clone(),
                    source: EventSource::PropertiesPanel,
                })
            }
            _ => None,
        }
    }
}

pub fn draw_properties_panel(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    object_name: &str,
    rows: &[PropertyRow],
    state: &PropertiesState,
) {
    let mut action_index = 0usize;
    let items = rows
        .iter()
        .map(|row| match row {
            PropertyRow::Value { label, value } => {
                ListItem::new(Line::from(format!("  {label}: {value}")))
            }
            PropertyRow::Separator => ListItem::new(Line::from("")),
            PropertyRow::Action {
                label,
                hint,
                enabled,
                ..
            } => {
                let selected = *enabled && action_index == state.selected_action();
                let prefix = if selected { "> " } else { "  " };
                let suffix = hint
                    .as_ref()
                    .map(|hint| format!("  [{hint}]"))
                    .unwrap_or_default();
                if *enabled {
                    action_index += 1;
                }
                let item = ListItem::new(Line::from(format!("{prefix}{label}{suffix}")));
                if selected {
                    item.style(Style::default().add_modifier(Modifier::REVERSED))
                } else {
                    item
                }
            }
        })
        .collect::<Vec<_>>();

    let popup = List::new(items).block(
        Block::default()
            .title(format!(
                " Properties: {object_name}  Up/Down select  Enter activate  Esc=back "
            ))
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_ui::EditorTargetKind;

    #[test]
    fn action_event_contains_target_and_action() {
        let target = EditorTarget::new("camera", "camera", "@scene/camera", EditorTargetKind::Camera);
        let rows = vec![PropertyRow::Action {
            id: "activate".to_string(),
            label: "Activate camera".to_string(),
            hint: None,
            enabled: true,
            action: EditorAction::ActivateControlTarget,
        }];
        let mut state = PropertiesState::default();
        state.open(target);
        let event = state.handle_key(KeyCode::Enter, &rows);
        assert!(matches!(
            event,
            Some(EditorEvent::ActionRequested {
                action: EditorAction::ActivateControlTarget,
                ..
            })
        ));
    }
}
