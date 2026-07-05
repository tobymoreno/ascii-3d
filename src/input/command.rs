use crossterm::event::KeyCode;

use crate::menu::MenuKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    Quit,

    ToggleControlMode,

    NextScene,
    PreviousScene,
    ResetWorldCamera,

    NextGlyphStroke,
    PreviousGlyphStroke,

    OpenMenu(MenuKind),
    CloseMenu,
    MenuUp,
    MenuDown,
    MenuSelect,

    MoveCameraForward,
    MoveCameraBackward,
    MoveCameraLeft,
    MoveCameraRight,
    MoveCameraDown,
    MoveCameraUp,
    RotateCameraLeft,
    RotateCameraRight,
    RotateCameraUp,
    RotateCameraDown,

    ToggleCameraDebug,
    ResetCamera,
    ToggleNearPlaneDebug,

    ToggleWorldAxes,
    ToggleWorldGrid,

    NextGlyph,
    PreviousGlyph,
    SelectGlyph,

    ToggleSimulationPause,
    StepSimulation,

    ToggleDepthView,
    ToggleProjectionDebug,
}

pub fn menu_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc => Some(AppCommand::CloseMenu),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(AppCommand::MenuUp),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(AppCommand::MenuDown),
        KeyCode::Enter => Some(AppCommand::MenuSelect),
        _ => None,
    }
}

pub fn scene_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Right | KeyCode::Enter => Some(AppCommand::NextScene),
        KeyCode::Left => Some(AppCommand::PreviousScene),
        KeyCode::Char(' ') => Some(AppCommand::NextGlyphStroke),
        KeyCode::Backspace => Some(AppCommand::PreviousGlyphStroke),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetWorldCamera),

        KeyCode::Char('m') | KeyCode::Char('M') => Some(AppCommand::OpenMenu(MenuKind::Scenes)),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Camera)),
        KeyCode::Char('w') | KeyCode::Char('W') => Some(AppCommand::OpenMenu(MenuKind::World)),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(AppCommand::OpenMenu(MenuKind::Glyphs)),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(AppCommand::OpenMenu(MenuKind::Physics)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::OpenMenu(MenuKind::Debug)),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        _ => None,
    }
}

pub fn camera_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetWorldCamera),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('w') | KeyCode::Char('W') => Some(AppCommand::MoveCameraForward),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(AppCommand::MoveCameraBackward),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(AppCommand::MoveCameraLeft),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::MoveCameraRight),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::MoveCameraDown),
        KeyCode::Char('e') | KeyCode::Char('E') => Some(AppCommand::MoveCameraUp),

        KeyCode::Left => Some(AppCommand::RotateCameraLeft),
        KeyCode::Right => Some(AppCommand::RotateCameraRight),
        KeyCode::Up => Some(AppCommand::RotateCameraUp),
        KeyCode::Down => Some(AppCommand::RotateCameraDown),

        _ => None,
    }
}
