use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem},
};

use super::{EditorEvent, EditorItem, EditorTargetKey, EventSource};

#[derive(Clone, Debug, Default)]
pub struct ObjectHierarchyState {
    open: bool,
    selected: Option<EditorTargetKey>,
    selected_index: usize,
    scroll: usize,
}

impl ObjectHierarchyState {
    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn open(&mut self, items: &[EditorItem]) {
        self.open = true;
        self.reconcile(items);
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn selected_item<'a>(&self, items: &'a [EditorItem]) -> Option<&'a EditorItem> {
        items.get(self.selected_index)
    }

    pub fn replace_items(&mut self, items: &[EditorItem]) {
        self.reconcile(items);
    }

    pub fn handle_key(&mut self, code: KeyCode, items: &[EditorItem]) -> Option<EditorEvent> {
        match code {
            KeyCode::Esc => {
                self.close();
                Some(EditorEvent::CloseRequested)
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.move_up(items);
                self.selection_event(items)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.move_down(items);
                self.selection_event(items)
            }
            KeyCode::Enter => self
                .selected_item(items)
                .map(|item| EditorEvent::InspectRequested {
                    target: item.target.clone(),
                    source: EventSource::ObjectHierarchy,
                }),
            _ => None,
        }
    }

    fn reconcile(&mut self, items: &[EditorItem]) {
        if items.is_empty() {
            self.selected = None;
            self.selected_index = 0;
            self.scroll = 0;
            return;
        }

        if let Some(selected) = self.selected.as_ref()
            && let Some(index) = items.iter().position(|item| &item.target.key == selected)
        {
            self.selected_index = index;
            return;
        }

        self.selected_index = self.selected_index.min(items.len() - 1);
        self.selected = Some(items[self.selected_index].target.key.clone());
    }

    fn move_up(&mut self, items: &[EditorItem]) {
        if items.is_empty() {
            self.reconcile(items);
            return;
        }
        self.selected_index = if self.selected_index == 0 {
            items.len() - 1
        } else {
            self.selected_index - 1
        };
        self.selected = Some(items[self.selected_index].target.key.clone());
    }

    fn move_down(&mut self, items: &[EditorItem]) {
        if items.is_empty() {
            self.reconcile(items);
            return;
        }
        self.selected_index = (self.selected_index + 1) % items.len();
        self.selected = Some(items[self.selected_index].target.key.clone());
    }

    fn selection_event(&self, items: &[EditorItem]) -> Option<EditorEvent> {
        self.selected_item(items)
            .map(|item| EditorEvent::SelectionChanged {
                target: item.target.clone(),
                source: EventSource::ObjectHierarchy,
            })
    }
}

pub fn draw_object_hierarchy(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    items: &[EditorItem],
    state: &ObjectHierarchyState,
    title: &str,
) {
    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let selector = if index == state.selected_index() {
                ">"
            } else {
                " "
            };
            let row = ListItem::new(Line::from(format!("{selector} {}", item.display_label())));
            if index == state.selected_index() {
                row.style(Style::default().add_modifier(Modifier::REVERSED))
            } else {
                row
            }
        })
        .collect::<Vec<_>>();

    let list = List::new(rows).block(
        Block::default()
            .title(format!(" {title}  Enter=inspect  Esc=close "))
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(list, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_ui::{EditorCapabilities, EditorTarget, EditorTargetKind};

    fn item(key: &str) -> EditorItem {
        EditorItem {
            target: EditorTarget::new(key, key, key, EditorTargetKind::Group),
            label: key.to_string(),
            depth: 0,
            visible: Some(true),
            has_children: false,
            capabilities: EditorCapabilities::VISIBILITY,
        }
    }

    #[test]
    fn refresh_preserves_selection_by_stable_key() {
        let mut state = ObjectHierarchyState::default();
        let items = vec![item("earth"), item("logo")];
        state.open(&items);
        state.handle_key(KeyCode::Down, &items);

        let reordered = vec![item("logo"), item("earth")];
        state.replace_items(&reordered);

        assert_eq!(state.selected_index(), 0);
        assert_eq!(
            state.selected_item(&reordered).unwrap().target.key,
            EditorTargetKey::new("logo")
        );
    }

    #[test]
    fn enter_emits_target_context() {
        let mut state = ObjectHierarchyState::default();
        let items = vec![item("earth")];
        state.open(&items);
        let event = state.handle_key(KeyCode::Enter, &items);
        assert!(matches!(
            event,
            Some(EditorEvent::InspectRequested { target, .. }) if target.path == "earth"
        ));
    }
}
