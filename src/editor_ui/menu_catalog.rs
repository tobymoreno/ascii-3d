use super::{MenuDefinition, MenuEntry, MenuId};

pub const FILE_MENU_ID: &str = "file";
pub const OBJECTS_MENU_ID: &str = "objects";
pub const VIEW_MENU_ID: &str = "view";
pub const DEBUG_MENU_ID: &str = "debug";
pub const HELP_MENU_ID: &str = "help";

pub const FILE_OPEN_ID: &str = "open";
pub const FILE_RELOAD_ID: &str = "reload";
pub const FILE_SAVE_ID: &str = "save";
pub const FILE_SAVE_AS_ID: &str = "save-as";
pub const FILE_BROWSE_SCENES_ID: &str = "browse-scenes";
pub const FILE_EXIT_ID: &str = "exit";

pub const DEBUG_TOGGLE_LOG_ID: &str = "toggle-log";
pub const DEBUG_OPEN_RAYLIB_ID: &str = "open-raylib-gui";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuCapabilities {
    pub can_open: bool,
    pub can_reload: bool,
    pub can_save: bool,
    pub can_save_as: bool,
    pub can_browse_scenes: bool,
    pub can_exit: bool,
    pub can_toggle_log: bool,
    pub can_open_raylib_gui: bool,
}

pub fn shared_menu_definitions(capabilities: MenuCapabilities) -> Vec<MenuDefinition> {
    vec![
        MenuDefinition {
            id: MenuId::new(FILE_MENU_ID),
            label: "File".to_string(),
            hotkey: Some('f'),
            entries: vec![
                action(FILE_OPEN_ID, "Open...", capabilities.can_open),
                action(FILE_RELOAD_ID, "Reload", capabilities.can_reload),
                MenuEntry::Separator,
                action(FILE_SAVE_ID, "Save", capabilities.can_save),
                action(FILE_SAVE_AS_ID, "Save As...", capabilities.can_save_as),
                MenuEntry::Separator,
                action(
                    FILE_BROWSE_SCENES_ID,
                    "Browse built-in scenes...",
                    capabilities.can_browse_scenes,
                ),
                MenuEntry::Separator,
                action(FILE_EXIT_ID, "Exit", capabilities.can_exit),
            ],
        },
        MenuDefinition {
            id: MenuId::new(OBJECTS_MENU_ID),
            label: "Objects".to_string(),
            hotkey: Some('o'),
            entries: Vec::new(),
        },
        MenuDefinition {
            id: MenuId::new(VIEW_MENU_ID),
            label: "View".to_string(),
            hotkey: Some('v'),
            entries: Vec::new(),
        },
        MenuDefinition {
            id: MenuId::new(DEBUG_MENU_ID),
            label: "Debug".to_string(),
            hotkey: Some('d'),
            entries: vec![
                action(
                    DEBUG_TOGGLE_LOG_ID,
                    "Toggle log",
                    capabilities.can_toggle_log,
                ),
                action(
                    DEBUG_OPEN_RAYLIB_ID,
                    "Open Raylib GUI",
                    capabilities.can_open_raylib_gui,
                ),
            ],
        },
        MenuDefinition {
            id: MenuId::new(HELP_MENU_ID),
            label: "Help".to_string(),
            hotkey: Some('h'),
            entries: Vec::new(),
        },
    ]
}

fn action(id: &str, label: &str, enabled: bool) -> MenuEntry {
    MenuEntry::Action {
        id: id.to_string(),
        label: label.to_string(),
        enabled,
        shortcut: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_catalog_has_stable_top_level_order() {
        let definitions = shared_menu_definitions(MenuCapabilities::default());
        let ids = definitions
            .iter()
            .map(|definition| definition.id.0.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                FILE_MENU_ID,
                OBJECTS_MENU_ID,
                VIEW_MENU_ID,
                DEBUG_MENU_ID,
                HELP_MENU_ID,
            ]
        );
    }

    #[test]
    fn unsupported_file_actions_remain_visible_but_disabled() {
        let definitions = shared_menu_definitions(MenuCapabilities::default());
        let file = &definitions[0];
        assert!(matches!(
            &file.entries[0],
            MenuEntry::Action { enabled: false, .. }
        ));
        assert!(file.entries.iter().any(|entry| matches!(
            entry,
            MenuEntry::Action { id, enabled: false, .. } if id == FILE_BROWSE_SCENES_ID
        )));
    }
}
