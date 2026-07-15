use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceMenu {
    File,
    Objects,
    View,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceKeymap {
    pub file_menu: char,
    pub objects_menu: char,
    pub view_menu: char,
    pub help_menu: char,
}

impl Default for WorkspaceKeymap {
    fn default() -> Self {
        Self {
            file_menu: 'f',
            objects_menu: 'o',
            view_menu: 'v',
            help_menu: 'h',
        }
    }
}

impl WorkspaceKeymap {
    pub fn menu_for_event(self, key: KeyEvent) -> Option<WorkspaceMenu> {
        if !key.modifiers.contains(KeyModifiers::ALT) {
            return None;
        }
        let KeyCode::Char(character) = key.code else {
            return None;
        };
        let character = character.to_ascii_lowercase();
        if character == self.file_menu {
            Some(WorkspaceMenu::File)
        } else if character == self.objects_menu {
            Some(WorkspaceMenu::Objects)
        } else if character == self.view_menu {
            Some(WorkspaceMenu::View)
        } else if character == self.help_menu {
            Some(WorkspaceMenu::Help)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    #[test]
    fn alt_o_opens_objects() {
        let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::ALT);
        assert_eq!(
            WorkspaceKeymap::default().menu_for_event(key),
            Some(WorkspaceMenu::Objects)
        );
    }

    #[test]
    fn plain_o_is_not_a_menu_accelerator() {
        let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
        assert_eq!(WorkspaceKeymap::default().menu_for_event(key), None);
    }
}
